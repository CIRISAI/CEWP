//! Physical access links under the constitutional budgets (`FSD/twin/media.md`).
//!
//! The budget classes of [`crate::transport`] say what a link MAY carry. This module says what an
//! access link CAN carry while a household shares it: a discrete-event model of one direction of
//! one access link (a home's uplink, a phone's radio downlink), with
//!
//! - a capacity that is the nominal rate times a Wi-Fi airtime share times, on cellular, a
//!   seeded Markov level that moves over time ([`CapacityModel`]);
//! - a bottleneck queue with a stated discipline: tail-drop FIFO or CoDel (RFC 8289) as the AQM
//!   variant ([`Discipline`]);
//! - radio / last-mile loss as a two-state Gilbert-Elliott chain stepped per packet
//!   ([`GeParams`]);
//! - background household flows that compete for the same queue, each a Reno-style AIMD
//!   sender ([`RenoFlow`]): slow start, halving once per round trip on a loss, the lost bytes
//!   sent again;
//! - a path MTU that turns an RNS packet larger than it into several IP fragments, every one of
//!   which must arrive ([`ip_fragments`]).
//!
//! Time is integer microseconds and sizes are integer bytes; the only randomness is the seeded
//! splitmix64 of [`crate::micro::Rng`]. Every default is a named constant with its FSD §8.2 class
//! in [`crate::media::MEDIA_PARAM_SPECS`].

use std::collections::VecDeque;

use crate::micro::Rng;

/// One million: loss and transition probabilities are parts per million.
pub const PPM: u64 = 1_000_000;

/// Draw a Bernoulli event of `ppm` parts per million.
#[must_use]
pub fn chance_ppm(rng: &mut Rng, ppm: u32) -> bool {
    ppm > 0 && rng.next_u64() % PPM < u64::from(ppm)
}

/// Integer square root (floor).
#[must_use]
pub fn isqrt(n: u64) -> u64 {
    if n < 2 {
        return n;
    }
    let mut x = (n as f64).sqrt() as u64;
    while x * x > n {
        x -= 1;
    }
    while (x + 1) * (x + 1) <= n {
        x += 1;
    }
    x
}

/// Microseconds to put `bytes` on a wire of `bps` bits per second (rounded up, at least 1).
#[must_use]
pub fn tx_us(bytes: u64, bps: u64) -> u64 {
    if bps == 0 {
        return u64::MAX / 4;
    }
    (bytes * 8 * 1_000_000).div_ceil(bps).max(1)
}

/// IP fragments an RNS packet of `rns_bytes` takes on a path of `path_mtu` with `ip_header`
/// bytes of IP+UDP header per fragment (at least one).
#[must_use]
pub fn ip_fragments(rns_bytes: u64, path_mtu: u64, ip_header: u64) -> u64 {
    let per = path_mtu.saturating_sub(ip_header).max(1);
    rns_bytes.div_ceil(per).max(1)
}

// ------------------------------------------------------------------ loss

/// A two-state Gilbert-Elliott loss chain, stepped once per packet: in the good state a packet
/// is lost with `loss_good_ppm`, in the bad state with `loss_bad_ppm`; after each packet the
/// chain moves good→bad with `p_gb_ppm` and bad→good with `p_bg_ppm`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeParams {
    /// Good → bad transition, per packet (ppm).
    pub p_gb_ppm: u32,
    /// Bad → good transition, per packet (ppm).
    pub p_bg_ppm: u32,
    /// Loss in the good state (ppm).
    pub loss_good_ppm: u32,
    /// Loss in the bad state (ppm).
    pub loss_bad_ppm: u32,
}

impl GeParams {
    /// No loss at all.
    pub const NONE: Self = Self { p_gb_ppm: 0, p_bg_ppm: PPM as u32, loss_good_ppm: 0, loss_bad_ppm: 0 };

    /// Independent (Bernoulli) loss of `ppm`: the chain never leaves the good state.
    #[must_use]
    pub const fn bernoulli(ppm: u32) -> Self {
        Self { p_gb_ppm: 0, p_bg_ppm: PPM as u32, loss_good_ppm: ppm, loss_bad_ppm: ppm }
    }

    /// Stationary loss rate (ppm): `pi_bad * loss_bad + pi_good * loss_good`.
    #[must_use]
    pub fn stationary_loss_ppm(&self) -> u64 {
        let gb = u64::from(self.p_gb_ppm);
        let bg = u64::from(self.p_bg_ppm);
        if gb + bg == 0 {
            return u64::from(self.loss_good_ppm);
        }
        (gb * u64::from(self.loss_bad_ppm) + bg * u64::from(self.loss_good_ppm)) / (gb + bg)
    }

    /// Mean length of a bad-state run, in packets (`1 / p_bg`), times 1000.
    #[must_use]
    pub fn mean_bad_run_milli(&self) -> u64 {
        if self.p_bg_ppm == 0 {
            return u64::MAX;
        }
        PPM * 1000 / u64::from(self.p_bg_ppm)
    }
}

/// The chain's state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GeState {
    /// In the bad state.
    pub bad: bool,
}

impl GeState {
    /// Decide one packet: `true` when it is lost. Loss is drawn in the current state, then the
    /// chain steps.
    pub fn step(&mut self, p: &GeParams, rng: &mut Rng) -> bool {
        let lost = chance_ppm(rng, if self.bad { p.loss_bad_ppm } else { p.loss_good_ppm });
        if self.bad {
            if chance_ppm(rng, p.p_bg_ppm) {
                self.bad = false;
            }
        } else if chance_ppm(rng, p.p_gb_ppm) {
            self.bad = true;
        }
        lost
    }
}

// ------------------------------------------------------------------ capacity

/// A cellular capacity process: a Markov chain over `levels_permille` of the nominal rate, which
/// every `period_us` stays put with `stay_ppm` or moves one level up or down (equally likely,
/// reflecting at the ends).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellModel {
    /// Capacity levels as permille of the nominal rate, best first.
    pub levels_permille: [u32; 4],
    /// How often the level may change (us).
    pub period_us: u64,
    /// Probability of staying at the current level each period (ppm).
    pub stay_ppm: u32,
}

/// What one direction of an access link can carry right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapacityModel {
    /// Nominal rate (bits/s): what the subscription says.
    pub nominal_bps: u64,
    /// Share of Wi-Fi airtime this station gets (permille; 1000 on a wire or a phone radio).
    pub airtime_permille: u32,
    /// Cellular variation, if any.
    pub cell: Option<CellModel>,
}

impl CapacityModel {
    /// A wired link at `bps`.
    #[must_use]
    pub const fn wired(bps: u64) -> Self {
        Self { nominal_bps: bps, airtime_permille: 1000, cell: None }
    }

    /// The rate at cellular level `level` (bits/s).
    #[must_use]
    pub fn rate_bps(&self, level: usize) -> u64 {
        let air = self.nominal_bps * u64::from(self.airtime_permille) / 1000;
        match self.cell {
            Some(c) => air * u64::from(c.levels_permille[level.min(3)]) / 1000,
            None => air,
        }
    }
}

// ------------------------------------------------------------------ queue

/// The bottleneck queue's discipline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Discipline {
    /// First in, first out; a packet that does not fit the buffer is dropped on arrival.
    TailDrop,
    /// CoDel (RFC 8289): tail-drop at the same buffer, plus drops at dequeue while the sojourn
    /// time stays above `target_us` for at least `interval_us`, spaced by the control law
    /// `interval / sqrt(count)`.
    CoDel {
        /// Acceptable standing queue delay (us).
        target_us: u64,
        /// Sliding window over which the minimum sojourn must exceed target (us).
        interval_us: u64,
    },
}

#[derive(Debug, Clone, Copy, Default)]
struct CodelState {
    first_above_us: u64,
    drop_next_us: u64,
    count: u64,
    lastcount: u64,
    dropping: bool,
}

/// One packet as the queue holds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Queued {
    /// The engine's packet id.
    pub id: u32,
    /// Wire bytes.
    pub bytes: u32,
    /// When it entered the queue (us).
    pub enq_us: u64,
}

/// Byte and packet accounting of one link direction. Conservation:
/// `offered = tail_drop + aqm_drop + air_loss + delivered + flushed + queued + in_service`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LinkStats {
    /// Bytes handed to the link.
    pub offered_bytes: u64,
    /// Bytes refused on arrival (buffer full).
    pub tail_drop_bytes: u64,
    /// Bytes dropped by the AQM at dequeue.
    pub aqm_drop_bytes: u64,
    /// Bytes transmitted but lost on the medium (Gilbert-Elliott, IP fragments).
    pub air_loss_bytes: u64,
    /// Bytes that made it across.
    pub delivered_bytes: u64,
    /// Bytes discarded because the node left with them still queued.
    pub flushed_bytes: u64,
    /// Packets handed to the link.
    pub offered_pkts: u64,
    /// Packets lost in any way.
    pub lost_pkts: u64,
    /// Sum of queue sojourn times of transmitted packets (us).
    pub sojourn_sum_us: u64,
    /// Worst queue sojourn time (us).
    pub sojourn_max_us: u64,
    /// Packets transmitted (lost on the medium or not).
    pub tx_pkts: u64,
}

/// What `enqueue` did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Enq {
    /// The link was idle: transmission starts now and ends at this time (us).
    Started(u64),
    /// Waiting behind others.
    Queued,
    /// Dropped: the buffer was full.
    Dropped,
}

/// What a finished transmission produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TxOutcome {
    /// The packet that finished.
    pub done: Queued,
    /// It was lost on the medium.
    pub lost: bool,
    /// The next transmission's end time, if one started.
    pub next_done_us: Option<u64>,
}

/// One direction of one access link: capacity, buffer, discipline, loss.
#[derive(Debug, Clone)]
pub struct LinkDir {
    /// Capacity process.
    pub cap: CapacityModel,
    /// Current cellular level.
    pub level: usize,
    /// Queue discipline.
    pub disc: Discipline,
    /// Buffer limit (bytes).
    pub limit_bytes: u64,
    /// Medium loss.
    pub ge: GeParams,
    /// IP fragments per packet are lost independently on top of the chain: path MTU (bytes).
    pub path_mtu: u64,
    /// IP+UDP header bytes per IP fragment.
    pub ip_header: u64,
    ge_state: GeState,
    q: VecDeque<Queued>,
    q_bytes: u64,
    in_service: Option<Queued>,
    codel: CodelState,
    /// Accounting.
    pub stats: LinkStats,
}

impl LinkDir {
    /// A link of `cap`, a buffer of `buffer_ms` at the nominal rate, `disc`, and medium loss `ge`.
    #[must_use]
    pub fn new(cap: CapacityModel, buffer_ms: u64, disc: Discipline, ge: GeParams) -> Self {
        let limit_bytes = (cap.nominal_bps / 8 * buffer_ms / 1000).max(3_000);
        Self {
            cap,
            level: 0,
            disc,
            limit_bytes,
            ge,
            path_mtu: 1_500,
            ip_header: 28,
            ge_state: GeState::default(),
            q: VecDeque::new(),
            q_bytes: 0,
            in_service: None,
            codel: CodelState::default(),
            stats: LinkStats::default(),
        }
    }

    /// The rate now (bits/s).
    #[must_use]
    pub fn rate_bps(&self) -> u64 {
        self.cap.rate_bps(self.level)
    }

    /// Bytes waiting (not counting the packet on the wire).
    #[must_use]
    pub fn queued_bytes(&self) -> u64 {
        self.q_bytes
    }

    /// Bytes waiting plus the packet on the wire.
    #[must_use]
    pub fn backlog_bytes(&self) -> u64 {
        self.q_bytes + self.in_service.map_or(0, |p| u64::from(p.bytes))
    }

    /// Whether a packet is on the wire.
    #[must_use]
    pub fn busy(&self) -> bool {
        self.in_service.is_some()
    }

    /// Step the cellular level once (call every `cell.period_us`).
    pub fn cell_step(&mut self, rng: &mut Rng) {
        let Some(c) = self.cap.cell else { return };
        if chance_ppm(rng, c.stay_ppm) {
            return;
        }
        let up = rng.next_u64() & 1 == 0;
        self.level = match (up, self.level) {
            (true, 0) => 1,
            (true, l) => l - 1,
            (false, 3) => 2,
            (false, l) => l + 1,
        };
    }

    /// Hand a packet to the link at `now`.
    pub fn enqueue(&mut self, id: u32, bytes: u32, now: u64) -> Enq {
        self.stats.offered_bytes += u64::from(bytes);
        self.stats.offered_pkts += 1;
        let p = Queued { id, bytes, enq_us: now };
        if self.in_service.is_none() && self.q.is_empty() {
            self.in_service = Some(p);
            self.note_sojourn(0);
            return Enq::Started(now + tx_us(u64::from(bytes), self.rate_bps()));
        }
        if self.q_bytes + u64::from(bytes) > self.limit_bytes {
            self.stats.tail_drop_bytes += u64::from(bytes);
            self.stats.lost_pkts += 1;
            return Enq::Dropped;
        }
        self.q_bytes += u64::from(bytes);
        self.q.push_back(p);
        Enq::Queued
    }

    fn note_sojourn(&mut self, s: u64) {
        self.stats.sojourn_sum_us += s;
        self.stats.sojourn_max_us = self.stats.sojourn_max_us.max(s);
        self.stats.tx_pkts += 1;
    }

    fn pop(&mut self) -> Option<Queued> {
        let p = self.q.pop_front()?;
        self.q_bytes -= u64::from(p.bytes);
        Some(p)
    }

    /// The packet on the wire finished at `now`. Decides its medium loss, then takes the next
    /// packet (dropping per the AQM into `aqm_dropped`) and starts it.
    ///
    /// # Panics
    /// When nothing was on the wire (an engine bug).
    pub fn tx_done(&mut self, now: u64, rng: &mut Rng, aqm_dropped: &mut Vec<Queued>) -> TxOutcome {
        let done = self.in_service.take().expect("tx_done with nothing on the wire");
        let mut lost = self.ge_state.step(&self.ge, rng);
        // A packet over the path MTU rides as several IP fragments; any one lost loses it.
        let frags = ip_fragments(u64::from(done.bytes), self.path_mtu, self.ip_header);
        for _ in 1..frags {
            lost |= self.ge_state.step(&self.ge, rng);
        }
        if lost {
            self.stats.air_loss_bytes += u64::from(done.bytes);
            self.stats.lost_pkts += 1;
        } else {
            self.stats.delivered_bytes += u64::from(done.bytes);
        }
        let next = self.dequeue(now, aqm_dropped);
        let next_done_us = next.map(|p| {
            self.note_sojourn(now - p.enq_us);
            self.in_service = Some(p);
            now + tx_us(u64::from(p.bytes), self.rate_bps())
        });
        TxOutcome { done, lost, next_done_us }
    }

    fn dequeue(&mut self, now: u64, dropped: &mut Vec<Queued>) -> Option<Queued> {
        match self.disc {
            Discipline::TailDrop => self.pop(),
            Discipline::CoDel { target_us, interval_us } => self.codel_dequeue(now, target_us, interval_us, dropped),
        }
    }

    fn aqm_drop(&mut self, p: Queued, dropped: &mut Vec<Queued>) {
        self.stats.aqm_drop_bytes += u64::from(p.bytes);
        self.stats.lost_pkts += 1;
        dropped.push(p);
    }

    /// RFC 8289 §5.5 `dequeue`, with the state machine of its pseudo-code.
    fn codel_dequeue(&mut self, now: u64, target: u64, interval: u64, dropped: &mut Vec<Queued>) -> Option<Queued> {
        let control_law = |t: u64, count: u64| t + interval * 1_000 / isqrt(count.max(1) * 1_000_000).max(1);
        let mut ok_to_drop;
        let mut p = self.pop();
        (p, ok_to_drop) = self.codel_check(p, now, target, interval);
        if self.codel.dropping {
            if !ok_to_drop {
                self.codel.dropping = false;
            } else {
                while now >= self.codel.drop_next_us && self.codel.dropping {
                    let Some(victim) = p else { break };
                    self.aqm_drop(victim, dropped);
                    self.codel.count += 1;
                    p = self.pop();
                    (p, ok_to_drop) = self.codel_check(p, now, target, interval);
                    if ok_to_drop {
                        self.codel.drop_next_us = control_law(self.codel.drop_next_us, self.codel.count);
                    } else {
                        self.codel.dropping = false;
                    }
                }
            }
        } else if ok_to_drop {
            if let Some(victim) = p {
                self.aqm_drop(victim, dropped);
                p = self.pop();
                (p, _) = self.codel_check(p, now, target, interval);
            }
            self.codel.dropping = true;
            let delta = self.codel.count.saturating_sub(self.codel.lastcount);
            self.codel.count = if delta > 1 && now.saturating_sub(self.codel.drop_next_us) < 16 * interval {
                delta
            } else {
                1
            };
            self.codel.drop_next_us = control_law(now, self.codel.count);
            self.codel.lastcount = self.codel.count;
        }
        p
    }

    /// RFC 8289 `dodequeue`'s verdict for packet `p` at `now`.
    fn codel_check(&mut self, p: Option<Queued>, now: u64, target: u64, interval: u64) -> (Option<Queued>, bool) {
        let Some(pk) = p else {
            self.codel.first_above_us = 0;
            return (None, false);
        };
        let sojourn = now - pk.enq_us;
        // RFC 8289: below target, or less than one MTU left behind it.
        if sojourn < target || self.q_bytes <= 1_500 {
            self.codel.first_above_us = 0;
            return (p, false);
        }
        if self.codel.first_above_us == 0 {
            self.codel.first_above_us = now + interval;
            return (p, false);
        }
        (p, now >= self.codel.first_above_us)
    }

    /// The node left: everything queued and on the wire is discarded and returned.
    pub fn flush(&mut self) -> Vec<Queued> {
        let mut out: Vec<Queued> = self.q.drain(..).collect();
        if let Some(p) = self.in_service.take() {
            out.push(p);
        }
        self.q_bytes = 0;
        for p in &out {
            self.stats.flushed_bytes += u64::from(p.bytes);
            self.stats.lost_pkts += 1;
        }
        out
    }

    /// `offered - (every outcome) - backlog`: zero when bytes are conserved.
    #[must_use]
    pub fn conservation_residual(&self) -> i128 {
        let s = &self.stats;
        i128::from(s.offered_bytes)
            - i128::from(s.tail_drop_bytes)
            - i128::from(s.aqm_drop_bytes)
            - i128::from(s.air_loss_bytes)
            - i128::from(s.delivered_bytes)
            - i128::from(s.flushed_bytes)
            - i128::from(self.backlog_bytes())
    }
}

// ------------------------------------------------------------------ background flows

/// TCP segment payload of a background flow (bytes): a 1500 B Ethernet MTU less 52 B of
/// IPv4 + TCP with timestamps.
pub const BG_MSS: u32 = 1_448;
/// Wire bytes of one full background segment.
pub const BG_SEGMENT_WIRE: u32 = 1_500;
/// Receive-window cap on a background flow's congestion window (segments).
pub const BG_RWND_SEGMENTS: u32 = 1_000;
/// A background flow's initial window (segments; RFC 6928).
pub const BG_INITIAL_WINDOW: u32 = 10;

/// One household transfer competing for an access link: a Reno-style AIMD sender. Slow start
/// until the first loss, then additive increase of one segment per window and a halving at
/// most once per round trip; a lost segment is sent again. Timeouts, SACK and CUBIC's curve are
/// not modelled (`FSD/twin/media.md` §6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenoFlow {
    /// The node whose access link it loads.
    pub node: u16,
    /// Downlink (a download into the home) or uplink.
    pub down: bool,
    /// Payload bytes not yet sent (lost bytes come back here).
    pub to_send: u64,
    /// Segments in flight.
    pub inflight: u32,
    /// Congestion window (segments).
    pub cwnd: u32,
    /// Slow-start threshold (segments).
    pub ssthresh: u32,
    ca_acc: u32,
    recover_until_us: u64,
    /// Payload bytes acknowledged.
    pub acked: u64,
    /// When it started (us).
    pub start_us: u64,
    /// When the last byte was acknowledged.
    pub done_us: Option<u64>,
}

impl RenoFlow {
    /// A flow of `bytes` starting at `now`.
    #[must_use]
    pub fn new(node: u16, down: bool, bytes: u64, now: u64) -> Self {
        Self {
            node,
            down,
            to_send: bytes,
            inflight: 0,
            cwnd: BG_INITIAL_WINDOW,
            ssthresh: BG_RWND_SEGMENTS,
            ca_acc: 0,
            recover_until_us: 0,
            acked: 0,
            start_us: now,
            done_us: None,
        }
    }

    /// Next segment to send, if the window allows: its payload bytes.
    pub fn next_segment(&mut self) -> Option<u32> {
        if self.inflight >= self.cwnd || self.to_send == 0 {
            return None;
        }
        let pay = u32::try_from(self.to_send.min(u64::from(BG_MSS))).unwrap_or(BG_MSS);
        self.to_send -= u64::from(pay);
        self.inflight += 1;
        Some(pay)
    }

    /// A segment of `pay` bytes was acknowledged at `now`.
    pub fn on_ack(&mut self, pay: u32, now: u64) {
        self.inflight = self.inflight.saturating_sub(1);
        self.acked += u64::from(pay);
        if self.cwnd < self.ssthresh {
            self.cwnd += 1;
        } else {
            self.ca_acc += 1;
            if self.ca_acc >= self.cwnd {
                self.ca_acc = 0;
                self.cwnd += 1;
            }
        }
        self.cwnd = self.cwnd.min(BG_RWND_SEGMENTS);
        if self.to_send == 0 && self.inflight == 0 && self.done_us.is_none() {
            self.done_us = Some(now);
        }
    }

    /// A segment of `pay` bytes was found lost at `now`; `rtt_us` is the current round trip.
    pub fn on_loss(&mut self, pay: u32, now: u64, rtt_us: u64) {
        self.inflight = self.inflight.saturating_sub(1);
        self.to_send += u64::from(pay);
        if now >= self.recover_until_us {
            self.ssthresh = (self.cwnd / 2).max(2);
            self.cwnd = self.ssthresh;
            self.ca_acc = 0;
            self.recover_until_us = now + rtt_us;
        }
    }

    /// Finished.
    #[must_use]
    pub fn finished(&self) -> bool {
        self.done_us.is_some()
    }
}

/// A household's background traffic in one direction: transfers arrive as a Poisson process
/// with mean gap `mean_gap_ms`; a transfer is `large_bytes` with probability `large_ppm`, else
/// `small_bytes`. `periodic_bytes` every `period_ms` adds a steady stream (a video player
/// fetching segments) when both are nonzero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackgroundModel {
    /// Mean gap between transfer arrivals (ms); 0 = none.
    pub mean_gap_ms: u64,
    /// Share of transfers that are large (ppm).
    pub large_ppm: u32,
    /// A small transfer (bytes).
    pub small_bytes: u64,
    /// A large transfer (bytes).
    pub large_bytes: u64,
    /// A periodic fetch's size (bytes); 0 = none.
    pub periodic_bytes: u64,
    /// The periodic fetch's period (ms).
    pub period_ms: u64,
}

impl BackgroundModel {
    /// No background traffic.
    pub const NONE: Self = Self { mean_gap_ms: 0, large_ppm: 0, small_bytes: 0, large_bytes: 0, periodic_bytes: 0, period_ms: 0 };

    /// Draw the gap to the next transfer (us) from an exponential with the model's mean.
    #[must_use]
    pub fn next_gap_us(&self, rng: &mut Rng) -> Option<u64> {
        if self.mean_gap_ms == 0 {
            return None;
        }
        let u = rng.f().max(1e-12);
        Some(((-u.ln()) * self.mean_gap_ms as f64 * 1_000.0) as u64 + 1)
    }

    /// Draw a transfer size.
    #[must_use]
    pub fn draw_bytes(&self, rng: &mut Rng) -> u64 {
        if chance_ppm(rng, self.large_ppm) {
            self.large_bytes
        } else {
            self.small_bytes
        }
    }
}

// ------------------------------------------------------------------ tests

#[cfg(test)]
mod tests {
    use super::*;

    fn drive(link: &mut LinkDir, arrivals: &[(u64, u32)], rng: &mut Rng) -> (Vec<(u32, u64, bool)>, Vec<u32>) {
        // A tiny event loop over one link: arrivals at fixed times, departures as they finish.
        let mut out = Vec::new();
        let mut dropped = Vec::new();
        let mut next_done: Option<u64> = None;
        let mut i = 0usize;
        let mut aqm = Vec::new();
        loop {
            let arr = arrivals.get(i).map(|a| a.0);
            match (arr, next_done) {
                (None, None) => break,
                (Some(a), Some(d)) if d <= a => {
                    let o = link.tx_done(d, rng, &mut aqm);
                    out.push((o.done.id, d, o.lost));
                    next_done = o.next_done_us;
                }
                (None, Some(d)) => {
                    let o = link.tx_done(d, rng, &mut aqm);
                    out.push((o.done.id, d, o.lost));
                    next_done = o.next_done_us;
                }
                (Some(a), _) => {
                    match link.enqueue(i as u32, arrivals[i].1, a) {
                        Enq::Started(d) => next_done = Some(d),
                        Enq::Dropped => dropped.push(i as u32),
                        Enq::Queued => {}
                    }
                    i += 1;
                }
            }
            for p in aqm.drain(..) {
                dropped.push(p.id);
            }
        }
        (out, dropped)
    }

    #[test]
    fn serialization_is_bytes_over_rate() {
        assert_eq!(tx_us(1_250, 10_000_000), 1_000);
        assert_eq!(tx_us(1, 8_000_000), 1);
        assert_eq!(ip_fragments(500, 1_500, 28), 1);
        assert_eq!(ip_fragments(1_473, 1_500, 28), 2);
    }

    #[test]
    fn an_idle_link_delivers_in_order_at_its_rate() {
        let mut l = LinkDir::new(CapacityModel::wired(8_000_000), 100, Discipline::TailDrop, GeParams::NONE);
        let mut rng = Rng::new(1);
        let (out, dropped) = drive(&mut l, &[(0, 1_000), (0, 1_000), (5_000, 500)], &mut rng);
        assert!(dropped.is_empty());
        assert_eq!(out, vec![(0, 1_000, false), (1, 2_000, false), (2, 5_500, false)]);
        assert_eq!(l.conservation_residual(), 0);
    }

    #[test]
    fn a_full_buffer_tail_drops_and_bytes_are_conserved() {
        // 1 Mbit/s, 10 ms buffer = 1250 B -> raised to the 3000 B floor.
        let mut l = LinkDir::new(CapacityModel::wired(1_000_000), 10, Discipline::TailDrop, GeParams::bernoulli(100_000));
        let mut rng = Rng::new(7);
        let arrivals: Vec<(u64, u32)> = (0..400).map(|k| (k * 1_000, 400)).collect();
        let (out, dropped) = drive(&mut l, &arrivals, &mut rng);
        assert!(!dropped.is_empty(), "3.2 Mbit/s into 1 Mbit/s must drop");
        assert_eq!(out.len() + dropped.len(), 400);
        assert_eq!(l.conservation_residual(), 0);
        assert!(l.stats.air_loss_bytes > 0);
        // Mid-run conservation, with a backlog standing.
        let mut m = LinkDir::new(CapacityModel::wired(1_000_000), 50, Discipline::TailDrop, GeParams::NONE);
        for k in 0..20 {
            let _ = m.enqueue(k, 400, 0);
        }
        assert!(m.backlog_bytes() > 0);
        assert_eq!(m.conservation_residual(), 0);
        let flushed = m.flush();
        assert!(!flushed.is_empty());
        assert_eq!(m.conservation_residual(), 0);
    }

    #[test]
    fn gilbert_elliott_stationary_loss_matches_its_closed_form() {
        let ge = GeParams { p_gb_ppm: 10_000, p_bg_ppm: 200_000, loss_good_ppm: 1_000, loss_bad_ppm: 400_000 };
        // pi_bad = 0.01 / 0.21; loss = pi_bad * 0.4 + pi_good * 0.001
        assert_eq!(ge.stationary_loss_ppm(), (10_000 * 400_000 + 200_000 * 1_000) / 210_000);
        let mut s = GeState::default();
        let mut rng = Rng::new(99);
        let n = 2_000_000u64;
        let lost = (0..n).filter(|_| s.step(&ge, &mut rng)).count() as u64;
        let ppm = lost * PPM / n;
        let want = ge.stationary_loss_ppm();
        // 2M draws: the standard error of a ~2% rate is under 0.03% even with bursts of 5.
        assert!(ppm.abs_diff(want) * 20 < want, "measured {ppm} ppm vs {want} ppm");
    }

    #[test]
    fn cellular_levels_scale_capacity_and_reflect_at_the_ends() {
        let cap = CapacityModel {
            nominal_bps: 10_000_000,
            airtime_permille: 500,
            cell: Some(CellModel { levels_permille: [1000, 600, 300, 100], period_us: 500_000, stay_ppm: 0 }),
        };
        assert_eq!(cap.rate_bps(0), 5_000_000);
        assert_eq!(cap.rate_bps(3), 500_000);
        let mut l = LinkDir::new(cap, 100, Discipline::TailDrop, GeParams::NONE);
        let mut rng = Rng::new(3);
        for _ in 0..1000 {
            l.cell_step(&mut rng);
            assert!(l.level <= 3);
        }
    }

    #[test]
    fn codel_keeps_the_standing_queue_short_where_tail_drop_bloats() {
        // A non-responsive 1.25 Mbit/s offered load into 1 Mbit/s with a 1 s buffer, for 20 s:
        // tail-drop fills the buffer in 4 s and then stands at 1 s; CoDel's control law reaches
        // the one-in-five drop rate the excess needs within about half a second.
        let arrivals: Vec<(u64, u32)> = (0..3_125).map(|k| (k * 6_400, 1_000)).collect();
        let mut rng = Rng::new(5);
        let mut fifo = LinkDir::new(CapacityModel::wired(1_000_000), 1_000, Discipline::TailDrop, GeParams::NONE);
        let _ = drive(&mut fifo, &arrivals, &mut rng);
        let mut codel = LinkDir::new(
            CapacityModel::wired(1_000_000),
            1_000,
            Discipline::CoDel { target_us: 5_000, interval_us: 100_000 },
            GeParams::NONE,
        );
        let _ = drive(&mut codel, &arrivals, &mut rng);
        let mean = |l: &LinkDir| l.stats.sojourn_sum_us / l.stats.tx_pkts.max(1);
        assert!(mean(&fifo) > 500_000, "tail-drop stands near its 1 s buffer: {}", mean(&fifo));
        assert!(mean(&codel) * 4 < mean(&fifo), "codel {} vs fifo {}", mean(&codel), mean(&fifo));
        assert!(codel.stats.aqm_drop_bytes > 0);
        assert_eq!(codel.conservation_residual(), 0);
        assert_eq!(fifo.conservation_residual(), 0);
    }

    #[test]
    fn reno_halves_once_per_round_trip_and_resends_what_it_lost() {
        let mut f = RenoFlow::new(0, false, 100 * u64::from(BG_MSS), 0);
        let mut sent = 0;
        while f.next_segment().is_some() {
            sent += 1;
        }
        assert_eq!(sent, BG_INITIAL_WINDOW);
        for _ in 0..10 {
            f.on_ack(BG_MSS, 1_000);
        }
        assert_eq!(f.cwnd, 20);
        while f.next_segment().is_some() {}
        f.on_loss(BG_MSS, 2_000, 50_000);
        assert_eq!(f.cwnd, 10);
        f.on_loss(BG_MSS, 3_000, 50_000);
        assert_eq!(f.cwnd, 10, "one halving per round trip");
        f.on_loss(BG_MSS, 60_000, 50_000);
        assert_eq!(f.cwnd, 5);
        assert_eq!(f.to_send, 100 * u64::from(BG_MSS) - 30 * u64::from(BG_MSS) + 3 * u64::from(BG_MSS));
    }

    #[test]
    fn the_same_seed_gives_the_same_link() {
        let ge = GeParams { p_gb_ppm: 20_000, p_bg_ppm: 300_000, loss_good_ppm: 0, loss_bad_ppm: 500_000 };
        let arrivals: Vec<(u64, u32)> = (0..500).map(|k| (k * 900, 600)).collect();
        let run = |seed| {
            let mut l = LinkDir::new(CapacityModel::wired(4_000_000), 50, Discipline::TailDrop, ge);
            drive(&mut l, &arrivals, &mut Rng::new(seed))
        };
        assert_eq!(run(11), run(11));
        assert_ne!(run(11), run(12));
    }
}
