//! The two hooks every twin package exposes (`/home/emoore/CEWP-twin/HOOKS.md`), for the
//! `people` package (spec `FSD/twin/people.md` section 7).
//!
//! 1. Energy counters, counts only. The people model has persons, not devices: each person
//!    acts and receives on ONE device, counted as `HostKind::Phone` (the micro tier's host
//!    kinds); consent-free rows land on serve seats, counted as `HostKind::ServeSeat`. The
//!    model has no bytes, no link class, no storage and no paths, so it counts none of them.
//!    It does have time, so it counts radio wake-ups: a device wakes for an act it signs or
//!    receives unless its radio is still up from an earlier one (held `radio_tail_s` after
//!    each event). Every count is DERIVED from the run's own acts; the tail is a MODEL input.
//! 2. Flat buffers for the globe: locality points, per-locality activity across the week,
//!    cross-locality ties as endpoint pairs, and one flash crowd unfolding. All coordinates
//!    are grid-cell centroids of the page's population grid (generated places, never an
//!    address), all persons generated.

use crate::activity::{Act, ActivityParams, Generator, Process, Reach, SyncEvent};
pub use crate::activity::rhythm;
use crate::micro::HostKind;
use crate::social::{Geography, Placement, Scope, SocialParams, SocialWorld, TieGraph, NO_ROOM};

/// Seconds a phone's radio stays up after its last send or receive (MODEL: the order of an
/// LTE RRC inactivity timer, commonly cited at about 10 s; not measured on CIRIS devices).
pub const RADIO_TAIL_S: u32 = 10;

/// Seats per locality: one per this many persons, as the page (`docs/globe.js` `buildWorld`).
pub const PERSONS_PER_SEAT: u32 = 2000;

/// Counts for one device class, summed over its devices (DERIVED).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClassCounts {
    /// The micro tier's host kind these counts belong to.
    pub class: HostKind,
    /// Devices of the class.
    pub devices: u64,
    /// Acts the class's devices signed.
    pub acts_originated: u64,
    /// Act copies the class's devices received.
    pub acts_received: u64,
    /// Signature operations: one sign per act originated.
    pub signs: u64,
    /// Verify operations: one per act received (assumes each receiver verifies once).
    pub verifies: u64,
}

/// Radio wake-ups over a sample of phones (DERIVED from the run, with `radio_tail_s` MODEL).
#[derive(Debug, Clone, PartialEq)]
pub struct WakeCounts {
    /// Phones sampled.
    pub sampled: u32,
    /// Hours in the horizon.
    pub hours: f64,
    /// Radio tail used.
    pub radio_tail_s: u32,
    /// Seconds in which a sampled phone signed or received at least one act.
    pub busy_seconds: u64,
    /// Sends and receipts on the sampled phones.
    pub events: u64,
    /// Distinct wake-ups (the radio came up from idle).
    pub wakes: u64,
    /// Seconds the radio was up.
    pub awake_s: u64,
    /// Per sampled phone, wake-ups per hour: p50, p90, p99.
    pub wakes_per_hour_p50_p90_p99: [f64; 3],
}

impl WakeCounts {
    /// Mean wake-ups per phone per hour.
    #[must_use]
    pub fn wakes_per_device_hour(&self) -> f64 {
        if self.sampled == 0 || self.hours <= 0.0 { 0.0 } else { self.wakes as f64 / f64::from(self.sampled) / self.hours }
    }
    /// Sends and receipts served per wake-up.
    #[must_use]
    pub fn events_per_wake(&self) -> f64 {
        if self.wakes == 0 { 0.0 } else { self.events as f64 / self.wakes as f64 }
    }
    /// Share of the horizon a phone's radio is up.
    #[must_use]
    pub fn awake_share(&self) -> f64 {
        let t = f64::from(self.sampled) * self.hours * 3600.0;
        if t <= 0.0 { 0.0 } else { self.awake_s as f64 / t }
    }
}

/// Everything the energy pass counts for one run.
#[derive(Debug, Clone, PartialEq)]
pub struct EnergyCounts {
    /// Persons' devices.
    pub phone: ClassCounts,
    /// Serve seats (receive consent-free rows).
    pub seat: ClassCounts,
    /// Wake-ups over the sampled phones.
    pub wakes: WakeCounts,
}

fn quantile(s: &[f64], q: f64) -> f64 {
    if s.is_empty() { 0.0 } else { s[(((s.len() - 1) as f64) * q).round() as usize] }
}

/// Count over one device's busy-second bitmap (bit `t` set = the device sent or received in
/// second `t`): (busy seconds, wake-ups, seconds awake). The radio comes up at a busy second
/// when it is idle and stays up `tail` seconds after the last busy second.
#[must_use]
pub fn count_wakes(bits: &[u64], tail: u64) -> (u64, u64, u64) {
    let (mut busy, mut wakes, mut awake) = (0u64, 0u64, 0u64);
    let mut end = 0u64; // the radio is up while t < end
    for (wi, &word) in bits.iter().enumerate() {
        let mut x = word;
        while x != 0 {
            let t = (wi * 64) as u64 + u64::from(x.trailing_zeros());
            x &= x - 1;
            busy += 1;
            if t >= end {
                wakes += 1;
                awake += tail;
            } else {
                awake += t + tail - end;
            }
            end = t + tail;
        }
    }
    (busy, wakes, awake)
}

/// Run `p` over `w` for `horizon_s` and count. `sample` phones (every k-th person, k set by the
/// sample size) are tracked second by second for wake-ups; the class totals cover everyone.
#[must_use]
pub fn energy(w: &SocialWorld, p: &ActivityParams, horizon_s: f64, seed: u64, forced: &[SyncEvent], sample: usize, radio_tail_s: u32) -> EnergyCounts {
    let n = w.persons.len();
    let secs = horizon_s.ceil().max(1.0) as usize;
    let words = secs.div_ceil(64);
    let stride = (n / sample.clamp(1, n.max(1))).max(1);
    let mut slot = vec![u32::MAX; n];
    let mut sampled = 0u32;
    for i in (0..n).step_by(stride) {
        slot[i] = sampled;
        sampled += 1;
    }
    // per room, the sampled members (CSR)
    let mut s_off = vec![0u32; w.rooms() + 1];
    let mut s_mem: Vec<u32> = Vec::new();
    for r in 0..w.rooms() {
        s_mem.extend(w.members(r as u32).iter().copied().filter(|&m| slot[m as usize] != u32::MAX));
        s_off[r + 1] = s_mem.len() as u32;
    }
    let mut bits = vec![0u64; sampled as usize * words];
    let mut room_acts = vec![0u64; w.rooms()];
    let mut originated = 0u64;
    let mut consent_free = 0u64;
    let mut events = 0u64;
    let mut sink = |a: &Act| {
        originated += 1;
        let sec = (a.t as usize).min(secs - 1);
        let mut mark = |m: u32| {
            let k = slot[m as usize] as usize;
            bits[k * words + sec / 64] |= 1u64 << (sec % 64);
        };
        if slot[a.signer as usize] != u32::MAX {
            mark(a.signer);
            events += 1;
        }
        if a.room != NO_ROOM {
            room_acts[a.room as usize] += 1;
            for &m in &s_mem[s_off[a.room as usize] as usize..s_off[a.room as usize + 1] as usize] {
                if m != a.signer {
                    mark(m);
                    events += 1;
                }
            }
        } else if a.scope == Scope::ConsentFree {
            consent_free += 1;
        }
    };
    Generator::new(w, p, seed).run(horizon_s, seed, forced, &mut sink);
    let received: u64 = (0..w.rooms()).map(|r| room_acts[r] * (w.members(r as u32).len() as u64).saturating_sub(1)).sum();
    let seats: u64 = w.locality_pop.iter().map(|&x| u64::from(x.div_ceil(PERSONS_PER_SEAT))).sum();
    let seat_receipts = consent_free * (1 + u64::from(crate::locality::CONSENT_FREE_REMOTE_SEATS));
    let hours = horizon_s / 3600.0;
    let tail = u64::from(radio_tail_s.max(1));
    let (mut busy, mut wakes, mut awake) = (0u64, 0u64, 0u64);
    let mut per_dev: Vec<f64> = Vec::with_capacity(sampled as usize);
    for k in 0..sampled as usize {
        let (b, dw, up) = count_wakes(&bits[k * words..(k + 1) * words], tail);
        busy += b;
        wakes += dw;
        awake += up;
        per_dev.push(dw as f64 / hours.max(1e-9));
    }
    per_dev.sort_by(|a, b| a.partial_cmp(b).unwrap());
    EnergyCounts {
        phone: ClassCounts { class: HostKind::Phone, devices: n as u64, acts_originated: originated, acts_received: received, signs: originated, verifies: received },
        seat: ClassCounts { class: HostKind::ServeSeat, devices: seats, acts_originated: 0, acts_received: seat_receipts, signs: 0, verifies: seat_receipts },
        wakes: WakeCounts {
            sampled,
            hours,
            radio_tail_s: radio_tail_s.max(1),
            busy_seconds: busy,
            events,
            wakes,
            awake_s: awake,
            wakes_per_hour_p50_p90_p99: [quantile(&per_dev, 0.5), quantile(&per_dev, 0.9), quantile(&per_dev, 0.99)],
        },
    }
}

/// Markdown section comparing Poisson and bursty arrivals at the same mean rate.
#[must_use]
pub fn energy_report(w: &SocialWorld, horizon_s: f64, seed: u64, sample: usize) -> String {
    let rows = [("Poisson (today)", ActivityParams::poisson()), ("bursty (defaults)", ActivityParams::default())];
    let mut out = format!(
        "\n## Energy counters (DERIVED counts, no watts)\n\nOne device per person, counted as `HostKind::Phone`; consent-free rows counted at `HostKind::ServeSeat` (one seat per {PERSONS_PER_SEAT} persons per locality). Horizon {:.0} h. Wake-ups over {} sampled phones, radio held {} s after each send or receipt (MODEL). The model has no bytes, link class, storage or path, so none is counted.\n\n| process | phone acts originated | phone acts received | seat rows received | busy phone-seconds | events | wakes | wakes / phone-hour | p50 / p90 / p99 per phone | events per wake | radio up |\n|---|---|---|---|---|---|---|---|---|---|---|\n",
        horizon_s / 3600.0,
        sample.min(w.persons.len()),
        RADIO_TAIL_S
    );
    for (name, p) in rows {
        let e = energy(w, &p, horizon_s, seed, &[], sample, RADIO_TAIL_S);
        let q = e.wakes.wakes_per_hour_p50_p90_p99;
        out.push_str(&format!(
            "| {name} | {} | {} | {} | {} | {} | {} | {:.2} | {:.2} / {:.2} / {:.2} | {:.2} | {:.1}% |\n",
            e.phone.acts_originated,
            e.phone.acts_received,
            e.seat.acts_received,
            e.wakes.busy_seconds,
            e.wakes.events,
            e.wakes.wakes,
            e.wakes.wakes_per_device_hour(),
            q[0],
            q[1],
            q[2],
            e.wakes.events_per_wake(),
            100.0 * e.wakes.awake_share()
        ));
    }
    out
}

// ------------------------------------------------------------------ globe views

/// Hours in the week buffer.
pub const WEEK_HOURS: usize = 168;

/// The deterministic small scenario the globe draws: the page's grid, `persons` generated
/// persons, default parameters, bursty process.
#[must_use]
pub fn globe_scenario(geo: &Geography, persons: usize, seed: u64) -> SocialWorld {
    SocialWorld::build(geo, &SocialParams::default(), persons, seed, Placement::Placed)
}

/// Locality points. Layout: `[lat_deg, lon_deg, population_share]` per locality, locality
/// index `k` at `3k`. The point is the population-weighted centroid of the locality's grid
/// cells (unit-vector mean, so a locality across the antimeridian stays in place).
#[must_use]
pub fn view_localities(geo: &Geography) -> Vec<f32> {
    let l = geo.localities();
    let mut acc = vec![[0.0f64; 4]; l];
    for c in &geo.cells {
        let (la, lo) = (f64::from(c.lat).to_radians(), f64::from(c.lon).to_radians());
        let a = &mut acc[c.locality as usize];
        a[0] += c.weight * la.cos() * lo.cos();
        a[1] += c.weight * la.cos() * lo.sin();
        a[2] += c.weight * la.sin();
        a[3] += c.weight;
    }
    let total: f64 = acc.iter().map(|a| a[3]).sum::<f64>().max(1e-12);
    let mut out = Vec::with_capacity(3 * l);
    for a in acc {
        let h = (a[0] * a[0] + a[1] * a[1]).sqrt();
        out.push(a[2].atan2(h).to_degrees() as f32);
        out.push(a[1].atan2(a[0]).to_degrees() as f32);
        out.push((a[3] / total) as f32);
    }
    out
}

/// Activity intensity per locality across one week (MODEL output of a one-week run).
/// Layout: `[hour * L + k]` for UTC hour of week `hour` in `0..168` (hour 0 = the run's start,
/// a Monday 00:00 UTC under the default `start_weekday`) and locality `k`; the value is acts
/// signed in that hour by persons of `k`, per person, over the mean hourly rate
/// (`acts_per_person_day / 24`), so 1.0 is an average hour. Localities with no person read 0.
/// Small localities are noisy at small `persons`: the caller may smooth.
#[must_use]
pub fn view_week_intensity(w: &SocialWorld, p: &ActivityParams, seed: u64) -> Vec<f32> {
    let l = w.localities();
    let mut c = vec![0u32; WEEK_HOURS * l];
    let mut sink = |a: &Act| {
        let h = ((a.t / 3600.0) as usize).min(WEEK_HOURS - 1);
        c[h * l + w.persons[a.signer as usize].locality as usize] += 1;
    };
    Generator::new(w, p, seed).run(WEEK_HOURS as f64 * 3600.0, seed, &[], &mut sink);
    let hourly = p.acts_per_person_day / 24.0;
    c.iter()
        .enumerate()
        .map(|(i, &x)| {
            let pop = f64::from(w.locality_pop[i % l]);
            if pop > 0.0 && hourly > 0.0 { (f64::from(x) / pop / hourly) as f32 } else { 0.0 }
        })
        .collect()
}

/// Ties that cross a locality, aggregated to locality pairs (MODEL output of the graph).
/// Layout: `[lat_a, lon_a, lat_b, lon_b, ties]` per pair (five floats), `a < b` by locality
/// index, sorted by `ties` descending then by `(a, b)`, at most `max_pairs` pairs. Endpoints are
/// the `view_localities` centroids. `ties` is the count of person-to-person ties in the pair.
#[must_use]
pub fn view_cross_ties(geo: &Geography, w: &SocialWorld, g: &TieGraph, max_pairs: usize) -> Vec<f32> {
    let pts = view_localities(geo);
    let mut pairs: Vec<u64> = Vec::new();
    for i in 0..w.persons.len() {
        let li = w.persons[i].locality;
        for &j in g.nb(i) {
            let lj = w.persons[j as usize].locality;
            if (j as usize) > i && li != lj {
                let (a, b) = if li < lj { (li, lj) } else { (lj, li) };
                pairs.push((u64::from(a) << 16) | u64::from(b));
            }
        }
    }
    pairs.sort_unstable();
    let mut agg: Vec<(u32, u64)> = Vec::new();
    for x in pairs {
        match agg.last_mut() {
            Some(last) if last.1 == x => last.0 += 1,
            _ => agg.push((1, x)),
        }
    }
    agg.sort_by(|p, q| q.0.cmp(&p.0).then(p.1.cmp(&q.1)));
    agg.truncate(max_pairs);
    let mut out = Vec::with_capacity(5 * agg.len());
    for (n, key) in agg {
        let (a, b) = ((key >> 16) as usize, (key & 0xFFFF) as usize);
        out.extend_from_slice(&[pts[3 * a], pts[3 * a + 1], pts[3 * b], pts[3 * b + 1], n as f32]);
    }
    out
}

/// The flash crowd the globe plays: a region-wide event (30% join, 3 acts each, 300 s decay,
/// FRONTIER stress inputs) in the region of the most populous locality, `lead_s` after the start.
#[must_use]
pub fn flash_scenario(w: &SocialWorld, lead_s: f64) -> SyncEvent {
    let k = (0..w.localities()).max_by_key(|&k| (w.locality_pop[k], std::cmp::Reverse(k))).unwrap_or(0);
    SyncEvent { t0: lead_s, reach: Reach::Region(w.region_of_locality[k]), participation: 0.3, acts_per_participant: 3.0, decay_s: 300.0 }
}

/// One flash crowd unfolding (MODEL output). The run covers `frames * frame_s` seconds from 0
/// with `event` added on top of the background (which carries no scheduled events of its own,
/// so the frame shows this one). Layout: `[(f * L + k) * 2 + 0]` = acts signed by persons of
/// locality `k` in frame `f`, `[(f * L + k) * 2 + 1]` = act copies delivered to persons of `k`
/// in frame `f` (room audiences; consent-free rows to seats are not included). Raw counts.
#[must_use]
pub fn view_flash(w: &SocialWorld, p: &ActivityParams, seed: u64, event: SyncEvent, frames: usize, frame_s: f64) -> Vec<f32> {
    let l = w.localities();
    let mut c = vec![0u32; frames * l * 2];
    let mut q = p.clone();
    q.process = Process::Bursty;
    q.event_share = 0.0; // background bursts and replies stay; only the staged event is synchronized
    let horizon = frames as f64 * frame_s;
    let mut sink = |a: &Act| {
        let f = ((a.t / frame_s) as usize).min(frames - 1);
        c[(f * l + w.persons[a.signer as usize].locality as usize) * 2] += 1;
        if a.room != NO_ROOM {
            for &(k, n) in w.room_localities(a.room) {
                let own = u32::from(k == w.persons[a.signer as usize].locality);
                c[(f * l + k as usize) * 2 + 1] += n - own;
            }
        }
    };
    if frames > 0 {
        Generator::new(w, &q, seed).run(horizon, seed, &[event], &mut sink);
    }
    c.into_iter().map(|x| x as f32).collect()
}

// ------------------------------------------------------------------ the layout `twin-globe` asked for

/// Persons in the globe's scenario world (MODEL size, chosen so a week's run stays small:
/// about persons × 180 acts/day × 7 acts).
pub const GLOBE_PERSONS: usize = 6000;

/// Field classes, as `twin-globe` numbers them.
pub const CLASS_MEASURED: f32 = 0.0;
/// DERIVED.
pub const CLASS_DERIVED: f32 = 1.0;
/// MODEL.
pub const CLASS_MODEL: f32 = 2.0;
/// FRONTIER.
pub const CLASS_FRONTIER: f32 = 4.0;

fn globe_world(grid: &[u8], seed: u64) -> Option<(Geography, SocialWorld)> {
    let geo = Geography::from_pop_grid(grid).ok()?;
    let w = globe_scenario(&geo, GLOBE_PERSONS, seed);
    Some((geo, w))
}

/// Locality points. `grid` is the decoded `POP_GRID_B64` bytes of `docs/globe.js`; empty on a
/// bad grid. Stride 3 per locality `k`: `lat_deg` (DERIVED: population-weighted centroid of the
/// grid cells), `lon_deg` (DERIVED), `population_weight` (DERIVED: share of grid weight, sums
/// to 1). Needs no seed; `seed` is accepted for a uniform signature.
#[must_use]
pub fn people_view_localities(grid: &[u8], _seed: u64) -> Vec<f32> {
    Geography::from_pop_grid(grid).map(|g| view_localities(&g)).unwrap_or_default()
}

/// Activity across the week, row-major `[k * 168 + h]`, locality `k`, UTC hour of week `h`
/// (hour 0 = Monday 00:00 UTC). Value: a MULTIPLIER, acts signed by the locality's persons in
/// that hour per person over the mean hourly rate (1.0 = an average hour); class MODEL (output
/// of a one-week bursty run of `GLOBE_PERSONS` generated persons). The clock is UTC: a
/// locality's own evening peak sits at its longitude's offset. Small localities are noisy.
#[must_use]
pub fn people_view_week(grid: &[u8], seed: u64) -> Vec<f32> {
    let Some((_, w)) = globe_world(grid, seed) else { return Vec::new() };
    let l = w.localities();
    let hk = view_week_intensity(&w, &ActivityParams::default(), seed);
    let mut out = vec![0.0f32; l * WEEK_HOURS];
    for h in 0..WEEK_HOURS {
        for k in 0..l {
            out[k * WEEK_HOURS + h] = hk[h * l + k];
        }
    }
    out
}

/// Cross-locality ties. Stride 3: `locality_a`, `locality_b` (`a < b`), `ties` (count of
/// person-to-person relationships between them in the generated graph; class MODEL). Sorted by
/// ties descending then `(a, b)`. Indices match `people_view_localities`.
#[must_use]
pub fn people_view_ties(grid: &[u8], seed: u64) -> Vec<u32> {
    let Some((_, w)) = globe_world(grid, seed) else { return Vec::new() };
    let g = w.ties(&SocialParams::default(), seed);
    let mut pairs: Vec<u64> = Vec::new();
    for i in 0..w.persons.len() {
        let li = w.persons[i].locality;
        for &j in g.nb(i) {
            let lj = w.persons[j as usize].locality;
            if (j as usize) > i && li != lj {
                let (a, b) = if li < lj { (li, lj) } else { (lj, li) };
                pairs.push((u64::from(a) << 16) | u64::from(b));
            }
        }
    }
    pairs.sort_unstable();
    let mut agg: Vec<(u32, u64)> = Vec::new();
    for x in pairs {
        match agg.last_mut() {
            Some(last) if last.1 == x => last.0 += 1,
            _ => agg.push((1, x)),
        }
    }
    agg.sort_by(|p, q| q.0.cmp(&p.0).then(p.1.cmp(&q.1)));
    agg.into_iter().flat_map(|(n, key)| [(key >> 16) as u32, (key & 0xFFFF) as u32, n]).collect()
}

/// Synchronized events of one week. Stride 6 per event: `lat_deg`, `lon_deg` (centroid of the
/// reached locality or region, population-weighted; NaN for a world-wide event), `start_hour`,
/// `peak_hour`, `end_hour` (fractional UTC hours of week; peak = onset, end = onset + 5 decay
/// times, past which under 1% of its acts remain), `peak_multiplier` (DERIVED from FRONTIER
/// event inputs: 1 + the event's expected acts in its first hour per reached person, cascade
/// replies included, over the mean hourly rate). Holds the staged flash crowd
/// (`flash_scenario` at Wednesday 20:30 UTC) first, then the background schedule's region-
/// and world-wide events in onset order (locality events are too many to draw; omitted).
#[must_use]
pub fn people_view_flash(grid: &[u8], seed: u64) -> Vec<f32> {
    let Some((geo, w)) = globe_world(grid, seed) else { return Vec::new() };
    let p = ActivityParams::default();
    let pts = view_localities(&geo);
    let l = w.localities();
    let mut reg = vec![[0.0f64; 4]; w.region_of_locality.iter().map(|&g| g as usize + 1).max().unwrap_or(1)];
    for k in 0..l {
        let (la, lo, wt) = (f64::from(pts[3 * k]).to_radians(), f64::from(pts[3 * k + 1]).to_radians(), f64::from(pts[3 * k + 2]));
        let a = &mut reg[w.region_of_locality[k] as usize];
        a[0] += wt * la.cos() * lo.cos();
        a[1] += wt * la.cos() * lo.sin();
        a[2] += wt * la.sin();
        a[3] += wt;
    }
    let at = |a: [f64; 4]| -> (f32, f32) { (a[2].atan2((a[0] * a[0] + a[1] * a[1]).sqrt()).to_degrees() as f32, a[1].atan2(a[0]).to_degrees() as f32) };
    let week = WEEK_HOURS as f64 * 3600.0;
    let mut evs = vec![flash_scenario(&w, 2.0 * 86_400.0 + 20.5 * 3600.0)];
    let mut bg: Vec<SyncEvent> = Generator::new(&w, &p, seed).events(week, seed).into_iter().filter(|e| !matches!(e.reach, Reach::Locality(_))).collect();
    bg.sort_by(|a, b| a.t0.partial_cmp(&b.t0).unwrap());
    evs.extend(bg);
    let hourly = p.acts_per_person_day / 24.0;
    let cascade = 1.0 / (1.0 - p.branching.clamp(0.0, 0.99));
    let mut out = Vec::with_capacity(6 * evs.len());
    for e in evs {
        let (lat, lon) = match e.reach {
            Reach::Locality(k) => (pts[3 * k as usize], pts[3 * k as usize + 1]),
            Reach::Region(g) => at(reg[g as usize]),
            Reach::World => (f32::NAN, f32::NAN),
        };
        let first_hour = 1.0 - (-3600.0 / e.decay_s.max(1e-9)).exp();
        let mult = 1.0 + e.participation * e.acts_per_participant * first_hour * cascade / hourly.max(1e-12);
        let h0 = e.t0 / 3600.0;
        out.extend_from_slice(&[lat, lon, h0 as f32, h0 as f32, (h0 + 5.0 * e.decay_s / 3600.0) as f32, mult as f32]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world(n: usize) -> (Geography, SocialWorld) {
        let geo = Geography::stand_in(3, 20, 4, 4);
        let w = SocialWorld::build(&geo, &SocialParams::default(), n, 9, Placement::Placed);
        (geo, w)
    }

    #[test]
    fn energy_is_deterministic_and_conserves_counts() {
        let (_, w) = world(1500);
        let p = ActivityParams::default();
        let a = energy(&w, &p, 86_400.0, 4, &[], 300, RADIO_TAIL_S);
        let b = energy(&w, &p, 86_400.0, 4, &[], 300, RADIO_TAIL_S);
        assert_eq!(a, b);
        assert_eq!(a.phone.signs, a.phone.acts_originated);
        assert!(a.wakes.wakes <= a.wakes.busy_seconds && a.wakes.busy_seconds <= a.wakes.events);
        assert!(a.wakes.awake_s >= a.wakes.wakes * u64::from(RADIO_TAIL_S));
        assert!(a.wakes.awake_s <= a.wakes.busy_seconds * u64::from(RADIO_TAIL_S));
    }

    /// Events inside one tail wake a device once; events further apart than the tail wake it
    /// every time.
    #[test]
    fn a_burst_wakes_once() {
        let mut b = vec![0u64; 4];
        for t in [100u64, 101, 103, 107] {
            b[(t / 64) as usize] |= 1 << (t % 64);
        }
        assert_eq!(count_wakes(&b, 10), (4, 1, 17));
        let mut s = vec![0u64; 4];
        for t in [0u64, 20, 40, 60, 200] {
            s[(t / 64) as usize] |= 1 << (t % 64);
        }
        assert_eq!(count_wakes(&s, 10), (5, 5, 50));
        assert_eq!(count_wakes(&[0, 0], 10), (0, 0, 0));
    }

    #[test]
    fn bursty_wakes_less_per_event_than_poisson() {
        let (_, w) = world(2000);
        let a = energy(&w, &ActivityParams::poisson(), 86_400.0, 5, &[], 400, RADIO_TAIL_S);
        let b = energy(&w, &ActivityParams::default(), 86_400.0, 5, &[], 400, RADIO_TAIL_S);
        assert!(b.wakes.events_per_wake() > a.wakes.events_per_wake(), "poisson {:?} bursty {:?}", a.wakes, b.wakes);
    }

    #[test]
    fn views_have_documented_lengths_and_are_deterministic() {
        let (geo, w) = world(1200);
        let l = w.localities();
        let pts = view_localities(&geo);
        assert_eq!(pts.len(), 3 * l);
        assert!((pts.iter().skip(2).step_by(3).map(|&x| f64::from(x)).sum::<f64>() - 1.0).abs() < 1e-4);
        let p = ActivityParams::default();
        let wk = view_week_intensity(&w, &p, 3);
        assert_eq!(wk.len(), WEEK_HOURS * l);
        assert_eq!(wk, view_week_intensity(&w, &p, 3));
        let g = w.ties(&SocialParams::default(), 3);
        let ct = view_cross_ties(&geo, &w, &g, 50);
        assert_eq!(ct.len() % 5, 0);
        assert!(!ct.is_empty() && ct.len() <= 250);
        assert!(ct.chunks(5).zip(ct.chunks(5).skip(1)).all(|(a, b)| a[4] >= b[4]));
        let ev = flash_scenario(&w, 600.0);
        let fl = view_flash(&w, &p, 3, ev, 30, 60.0);
        assert_eq!(fl.len(), 30 * l * 2);
        assert_eq!(fl, view_flash(&w, &p, 3, ev, 30, 60.0));
    }

    #[test]
    fn the_flash_lights_its_region() {
        let (_, w) = world(3000);
        let base = flash_scenario(&w, 600.0);
        let Reach::Region(g) = base.reach else { panic!("region event") };
        let ev = SyncEvent { participation: 1.0, acts_per_participant: 5.0, ..base };
        let l = w.localities();
        let fl = view_flash(&w, &ActivityParams::default(), 4, ev, 30, 60.0);
        let signed = |f: usize, inside: bool| -> f32 { (0..l).filter(|&k| (w.region_of_locality[k] == g) == inside).map(|k| fl[(f * l + k) * 2]).sum() };
        // before onset (frames 0..10) against the five minutes after (frames 10..15), inside the region
        let before: f32 = (0..10).map(|f| signed(f, true)).sum::<f32>() / 10.0;
        let after: f32 = (10..15).map(|f| signed(f, true)).sum::<f32>() / 5.0;
        assert!(after > 3.0 * before.max(1.0), "before {before} after {after}");
    }

    #[test]
    fn globe_layout_strides_on_the_page_grid() {
        let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/globe.js")).unwrap();
        let key = "POP_GRID_B64=\"";
        let a = src.find(key).unwrap() + key.len();
        let b = a + src[a..].find('"').unwrap();
        let grid = crate::social::base64_decode(&src[a..b]).unwrap();
        let pts = people_view_localities(&grid, 1);
        assert_eq!(pts.len(), 3 * 181);
        let ties = people_view_ties(&grid, 1);
        assert_eq!(ties.len() % 3, 0);
        assert!(ties.chunks(3).all(|t| t[0] < t[1] && t[1] < 181 && t[2] > 0));
        let fl = people_view_flash(&grid, 1);
        assert_eq!(fl.len() % 6, 0);
        assert!(fl.len() >= 6 && fl[2] > 0.0 && fl[5] > 1.0, "{:?}", &fl[..6.min(fl.len())]);
        // world-wide events carry NaN coordinates, so compare bit patterns
        let bits = |v: &[f32]| v.iter().map(|x| x.to_bits()).collect::<Vec<u32>>();
        assert_eq!(bits(&fl), bits(&people_view_flash(&grid, 1)));
        assert!(people_view_week(&[], 1).is_empty());
    }
}
