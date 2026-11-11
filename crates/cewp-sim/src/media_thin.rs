//! One constrained publisher in a five-person call (`FSD/twin/media.md` §13), the design case
//! for edge v45.1: someone on a thin uplink streaming out, where the next hop should relay
//! everything past that point. Member 0 publishes on a thin uplink (swept); member 1 is the
//! natural next hop (fibre, nearest, joins first); members 2–4 are ordinary home links. Every
//! member publishes; the lines report member 0's stream only.
//!
//! Modes:
//! - `a` v45.0.0 as the server ships it: full mesh, every member `relay: false`.
//! - `b` relay on, admission as edge has it (initial fan-out clamp(N−1, 1, 8), probe growth,
//!   nothing refused locally on a datagram leg).
//! - `b975` as `b` with the CIRISEdge#975 delivery report (not in edge v45).
//! - `c` counterfactual: the publisher sends exactly one copy (fan-out fixed at 1, never probes
//!   up), no warm standby taken from it; the next hop relays to the other three.
//! - `c1400` as `c` with the probed MTU 1,400 on every leg.
//!
//! Everything here is MODEL output.

use crate::link::Discipline;
use crate::media::{
    d3, ms, permille, AccessProfile, CallParams, Condition, Engine, NodeSpec, Scenario, CABLE_HOME_WIFI, DSL_HOME, FIBER_HOME,
    PHONE_HOME_WIFI, RNS_BASE_MTU,
};
use crate::media_call2::PROBED_MTU;
use crate::micro::Rng;

/// The swept publisher uplinks (bit/s). 2 Mbit/s is added above the four asked for, to bracket
/// where video fits.
pub const THIN_UPLINKS: [u64; 5] = [128_000, 256_000, 512_000, 1_000_000, 2_000_000];

/// The modes, in order.
pub const THIN_MODES: [&str; 5] = ["a", "b", "b975", "c", "c1400"];

/// Audio is carried when the p95 session misses at most this (permille). MODEL threshold.
pub const AUDIO_OK_P95_PERMILLE: u64 = 50;
/// Video is carried when every session shows video and the p95 session freezes at most this
/// (ms per minute). MODEL threshold, set just above the two-person direct Wi-Fi call's p95
/// (§11, 21.9 s/min): loss with keyframe-only recovery freezes that much on any uplink, so the
/// test asks only that the thin uplink adds no freeze of its own.
pub const VIDEO_OK_P95_FREEZE_MS_MIN: u64 = 25_000;

/// The thin uplink: the DSL line's buffer depth and last mile with the swept uplink. MODEL.
#[must_use]
pub const fn thin_profile(up_bps: u64) -> AccessProfile {
    AccessProfile { name: "thin-uplink", up_bps, relay: false, ..DSL_HOME }
}

/// The scenario for one uplink, mode and medium.
#[must_use]
pub fn scenario_thin(seed: u64, contention: bool, disc: Discipline, up_bps: u64, mode: &str, video: bool) -> Scenario {
    let mut rng = Rng::new(seed ^ 0x7417);
    let mut nodes = vec![NodeSpec::member(thin_profile(up_bps), 200_000), NodeSpec::member(FIBER_HOME, 500_000)];
    for p in [FIBER_HOME, CABLE_HOME_WIFI, PHONE_HOME_WIFI] {
        nodes.push(NodeSpec::member(p, 1_500_000 + rng.next_u64() % 1_500_000));
    }
    if mode == "a" {
        for n in &mut nodes {
            n.relay = false;
        }
    }
    let mut call = CallParams { delivery_report: mode == "b975", ..CallParams::default() };
    if !video {
        call.video_bps = 0;
    }
    if mode.starts_with('c') {
        call.publisher_cap = 1;
        call.standby_spares_publisher = true;
    }
    call.rns_mtu = if mode == "c1400" { PROBED_MTU } else { RNS_BASE_MTU };
    Scenario { name: format!("thin {up_bps} {mode}"), nodes, duration_us: 60_000_000, call, contention, disc, impairments: true, seed }
}

/// One pooled result for member 0's stream.
#[derive(Debug, Clone, Default)]
pub struct ThinResult {
    /// Sessions of stream 0.
    pub sessions: usize,
    /// Sessions of stream 0 that never showed a frame (video runs only).
    pub no_video: usize,
    /// Audio missing per session (permille).
    pub audio_miss: Vec<u64>,
    /// Freeze per session (ms per minute; video runs only).
    pub freeze: Vec<u64>,
    /// Keyframe loss per session (permille).
    pub key_loss: Vec<u64>,
    /// Join per session (ms).
    pub join: Vec<u64>,
    /// The publisher's media bytes on its uplink against its capacity, per run (permille).
    pub up_use: Vec<u64>,
    /// The publisher's direct children at the end, per run.
    pub fanout: Vec<u64>,
    /// Copies the next hop forwarded for others, per run.
    pub next_hop_fwd_kbps: Vec<u64>,
    /// Every run conserved.
    pub conserved: bool,
}

impl ThinResult {
    /// Audio carried by the model's threshold.
    #[must_use]
    pub fn audio_ok(&self) -> bool {
        !self.audio_miss.is_empty() && crate::qoe::Dist::of(&self.audio_miss).p95 <= AUDIO_OK_P95_PERMILLE
    }
    /// Video carried by the model's threshold.
    #[must_use]
    pub fn video_ok(&self) -> bool {
        self.no_video == 0 && !self.freeze.is_empty() && crate::qoe::Dist::of(&self.freeze).p95 <= VIDEO_OK_P95_FREEZE_MS_MIN
    }
}

/// Run `seeds` seeds and pool member 0's stream.
#[must_use]
pub fn thin_pool(up_bps: u64, mode: &str, video: bool, cond: &Condition, seeds: u64) -> ThinResult {
    let mut out = ThinResult { conserved: true, ..ThinResult::default() };
    for seed in 1..=seeds {
        let r = Engine::new(scenario_thin(seed, cond.contention, cond.disc, up_bps, mode, video)).run();
        out.conserved &= r.ledger.balanced() && r.links_conserve;
        let e = &r.energy[0];
        out.up_use.push(e.media_own_tx_bytes * 8 * 1_000_000 / e.present_us.max(1) * 1_000 / up_bps);
        out.fanout.push(u64::from(r.publisher_fanout[0]));
        out.next_hop_fwd_kbps.push(r.energy[1].media_fwd_tx_bytes * 8_000 / r.energy[1].present_us.max(1));
        for q in r.sessions.iter().filter(|q| q.stream == 0 && q.receiver != 0) {
            out.sessions += 1;
            if q.audio_sent > 0 {
                out.audio_miss.push(permille(q.audio_missing, q.audio_sent));
            }
            if video {
                match q.join_us {
                    Some(j) => out.join.push(ms(j)),
                    None => out.no_video += 1,
                }
                if q.span_us > 10_000_000 {
                    out.freeze.push(q.freeze_us * 60_000 / q.span_us.max(1));
                }
                if q.key_sent > 0 {
                    out.key_loss.push(permille(q.key_lost, q.key_sent));
                }
            }
        }
    }
    out
}

/// One line.
#[must_use]
pub fn thin_line(up_bps: u64, mode: &str, video: bool, cond: &Condition, r: &ThinResult) -> String {
    format!(
        "up={:<5} mode={mode:<5} {:<5} {:<10} sessions={} noVideo={} audioMiss_pm={} freeze_ms/min={} keyLoss_pm={} join_ms={} pubUplinkUse_pm={} pubFanout={} nextHopFwd_kbps={} audioOK={} videoOK={} conserved={}",
        up_bps / 1_000,
        if video { "a+v" } else { "audio" },
        cond.label,
        r.sessions,
        if video { r.no_video.to_string() } else { "-".to_owned() },
        d3(&r.audio_miss),
        d3(&r.freeze),
        d3(&r.key_loss),
        d3(&r.join),
        d3(&r.up_use),
        d3(&r.fanout),
        d3(&r.next_hop_fwd_kbps),
        r.audio_ok(),
        if video { r.video_ok().to_string() } else { "-".to_owned() },
        r.conserved
    )
}

/// The whole sweep, then the lowest swept uplink at which each mode carries audio and video.
#[must_use]
pub fn thin_lines(seeds: u64) -> Vec<String> {
    let mut out = vec![format!(
        "thin publisher in a 5-person call (MODEL output; {seeds} seeds; member 0's stream; p50/p95/p99 over sessions; audioOK = p95 missing <= {AUDIO_OK_P95_PERMILLE} pm; videoOK = all sessions show video and p95 freeze <= {VIDEO_OK_P95_FREEZE_MS_MIN} ms/min)"
    )];
    for cond in [
        Condition { label: "busy,fifo", contention: true, disc: Discipline::TailDrop, delivery_report: false },
        Condition { label: "quiet,fifo", contention: false, disc: Discipline::TailDrop, delivery_report: false },
    ] {
        let mut lowest: Vec<(String, Option<u64>)> = Vec::new();
        for video in [false, true] {
            for mode in THIN_MODES {
                let mut low = None;
                for up in THIN_UPLINKS {
                    let r = thin_pool(up, mode, video, &cond, seeds);
                    out.push(thin_line(up, mode, video, &cond, &r));
                    let ok = if video { r.video_ok() && r.audio_ok() } else { r.audio_ok() };
                    if ok && low.is_none() {
                        low = Some(up);
                    }
                }
                lowest.push((format!("{mode} {}", if video { "a+v" } else { "audio" }), low));
            }
        }
        for (k, v) in lowest {
            out.push(format!(
                "lowest {} {k}: {}",
                cond.label,
                v.map_or_else(|| "none of the swept uplinks".to_owned(), |u| format!("{} kbit/s", u / 1_000))
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_capped_publisher_sends_one_copy_and_the_next_hop_relays() {
        let c = Engine::new(scenario_thin(1, false, Discipline::TailDrop, 512_000, "c", false)).run();
        assert!(c.ledger.balanced() && c.links_conserve);
        assert_eq!(c.publisher_fanout[0], 1, "one copy leaves the thin uplink");
        assert!(c.energy[1].media_fwd_tx_copies > 0, "the next hop relays");
        let ok = c.sessions.iter().filter(|q| q.stream == 0 && q.receiver != 0).count();
        assert_eq!(ok, 4, "every other member receives the thin publisher");
        let a = Engine::new(scenario_thin(1, false, Discipline::TailDrop, 512_000, "a", false)).run();
        assert!(a.ledger.balanced() && a.links_conserve);
        assert_eq!(a.publisher_fanout[0], 4, "full mesh: four copies");
        assert!(a.energy.iter().all(|e| e.media_fwd_tx_copies == 0));
    }
}
