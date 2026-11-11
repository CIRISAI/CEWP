//! A two-person call (`FSD/twin/media.md` §11): one publisher each way, audio + video, no relay
//! tree and no warm standby, on a punched direct path at the probed MTU or on a relayed leg at
//! the base MTU. Asked for by the question whether v45.0.0 ships calls limited to two people.
//!
//! The relayed leg is modelled as a third node on a hosted server that forwards every copy
//! (both members are marked unreachable, so neither can take the other's stream directly). A
//! real RNS transport hop forwards the link's packets without the A/V layer's per-chunk
//! verify; the model's relay verifies (60 us at a 1 Gbit/s server, never the bottleneck), so
//! the difference is a CPU cost the result does not hinge on. Everything here is MODEL output.

use crate::link::{BackgroundModel, Discipline, GeParams};
use crate::media::{
    pool, AccessProfile, Condition, LinkClass, NodeSpec, Pooled, Scenario, CABLE_HOME_WIFI, CallParams, PHONE_HOME_WIFI, PHONE_LTE,
    RNS_BASE_MTU,
};
use crate::micro::{HostKind, Rng};
use crate::media::{d3, ms, permille};

/// The probed RNS MTU of a punched direct path (frame budget 1,142 B). MODEL: a 1,500 B path
/// less IP/UDP leaves room for it.
pub const PROBED_MTU: u32 = 1_400;

/// A hosted server forwarding a relayed leg: 1 Gbit/s each way, a shallow buffer, no medium
/// loss, no household. MODEL.
pub const SERVER_RELAY: AccessProfile = AccessProfile {
    name: "hosted-relay", up_bps: 1_000_000_000, down_bps: 1_000_000_000, up_buffer_ms: 20, down_buffer_ms: 20, last_mile_us: 1_000,
    airtime_permille: 1000, cell: None, ge: GeParams::NONE, verify_us: 60, sign_us: 25, bg_up: BackgroundModel::NONE,
    bg_down: BackgroundModel::NONE, relay: true, host: HostKind::HostedServer, class: LinkClass::Fixed,
};

/// Two members on `a` and `b`; `relayed` puts a hosted relay between them at the base MTU,
/// otherwise the path is direct at [`PROBED_MTU`].
#[must_use]
pub fn scenario_call2(seed: u64, contention: bool, disc: Discipline, a: AccessProfile, b: AccessProfile, relayed: bool) -> Scenario {
    let mut rng = Rng::new(seed ^ 0x2CA1);
    let mut nodes: Vec<NodeSpec> = [a, b]
        .iter()
        .map(|&p| NodeSpec { relay: false, unreachable: relayed, ..NodeSpec::member(p, 500_000 + rng.next_u64() % 2_500_000) })
        .collect();
    if relayed {
        nodes.push(NodeSpec { publishes: false, receives: true, ..NodeSpec::member(SERVER_RELAY, 0) });
    }
    let call = CallParams { standby: false, rns_mtu: if relayed { RNS_BASE_MTU } else { PROBED_MTU }, ..CallParams::default() };
    Scenario { name: "call 2".to_owned(), nodes, duration_us: 60_000_000, call, contention, disc, impairments: true, seed }
}

/// Both on home Wi-Fi (a laptop, a phone), punched direct.
#[must_use]
pub fn wifi_direct(seed: u64, contention: bool, disc: Discipline) -> Scenario {
    scenario_call2(seed, contention, disc, CABLE_HOME_WIFI, PHONE_HOME_WIFI, false)
}
/// Both on home Wi-Fi, relayed.
#[must_use]
pub fn wifi_relayed(seed: u64, contention: bool, disc: Discipline) -> Scenario {
    scenario_call2(seed, contention, disc, CABLE_HOME_WIFI, PHONE_HOME_WIFI, true)
}
/// A laptop on home Wi-Fi and a phone on LTE, punched direct.
#[must_use]
pub fn lte_direct(seed: u64, contention: bool, disc: Discipline) -> Scenario {
    scenario_call2(seed, contention, disc, CABLE_HOME_WIFI, PHONE_LTE, false)
}
/// A laptop on home Wi-Fi and a phone on LTE, relayed.
#[must_use]
pub fn lte_relayed(seed: u64, contention: bool, disc: Discipline) -> Scenario {
    scenario_call2(seed, contention, disc, CABLE_HOME_WIFI, PHONE_LTE, true)
}

/// The four conditions asked for.
pub const CALL2: [(&str, crate::media::ScenarioFn); 4] =
    [("wifi direct mtu1400", wifi_direct), ("wifi relayed mtu500", wifi_relayed), ("lte direct mtu1400", lte_direct), ("lte relayed mtu500", lte_relayed)];

/// One condition's line: sessions between the two members only (the relay's own reception is
/// not a person's experience). Distributions are p50/p95/p99 over sessions.
#[must_use]
pub fn call2_line(name: &str, cond: &Condition, p: &Pooled) -> String {
    let s: Vec<_> = p.sessions.iter().filter(|q| q.receiver < 2 && q.stream < 2).collect();
    let join: Vec<u64> = s.iter().filter_map(|q| q.join_us).map(ms).collect();
    let never = s.iter().filter(|q| q.join_us.is_none()).count();
    let aud: Vec<u64> = s.iter().filter(|q| q.audio_sent > 0).map(|q| permille(q.audio_missing, q.audio_sent)).collect();
    let alat: Vec<u64> = s.iter().filter_map(|q| crate::qoe::percentile(&q.audio_lat_samples, 950)).map(ms).collect();
    let frz: Vec<u64> = s.iter().filter(|q| q.span_us > 10_000_000).map(|q| q.freeze_us * 60_000 / q.span_us.max(1)).collect();
    let key: Vec<u64> = s.iter().filter(|q| q.key_sent > 0).map(|q| permille(q.key_lost, q.key_sent)).collect();
    let g2g: Vec<u64> = s.iter().filter_map(|q| crate::qoe::percentile(&q.g2g_samples, 950)).map(ms).collect();
    let chunk: Vec<u64> = s.iter().map(|q| q.chunk_loss_ppm / 1_000).collect();
    format!(
        "{name:<20} {:<10} sessions={} noVideo={} audioMiss_pm={} audioLat_p95_ms={} freeze_ms/min={} keyLoss_pm={} g2g_p95_ms={} join_ms={} chunkLoss_pm={} conserved={}",
        cond.label,
        s.len(),
        never,
        d3(&aud),
        d3(&alat),
        d3(&frz),
        d3(&key),
        d3(&g2g),
        d3(&join),
        d3(&chunk),
        p.conserved
    )
}

/// The two-person table: household traffic on (asked), then off (for scale).
#[must_use]
pub fn call2_lines(seeds: u64) -> Vec<String> {
    let mut out = vec![format!("two-person call (MODEL output; {seeds} seeds; p50/p95/p99 over sessions; no standby; relayed = hosted relay at base MTU)")];
    for cond in [
        Condition { label: "busy,fifo", contention: true, disc: Discipline::TailDrop, delivery_report: false },
        Condition { label: "quiet,fifo", contention: false, disc: Discipline::TailDrop, delivery_report: false },
    ] {
        for (name, f) in CALL2 {
            out.push(call2_line(name, &cond, &pool(f, cond, seeds)));
        }
    }
    out
}

/// The member mixes of the audio-only room. MODEL: `wifi` alternates a laptop and a phone on
/// home Wi-Fi; `wifi+lte` alternates a laptop on home Wi-Fi and a phone on LTE; `wifi+1dsl` is
/// the Wi-Fi mix with member 0 on the thin DSL uplink.
pub const AUDIO_MIXES: [&str; 3] = ["wifi", "wifi+lte", "wifi+1dsl"];

fn audio_mix_profile(mix: &str, i: usize) -> AccessProfile {
    match (mix, i % 2) {
        ("wifi+1dsl", _) if i == 0 => crate::media::DSL_HOME,
        ("wifi+lte", 1) => PHONE_LTE,
        (_, 0) => CABLE_HOME_WIFI,
        _ => PHONE_HOME_WIFI,
    }
}

/// An audio-only full-mesh room (`FSD/twin/media.md` §12): `n` members, Opus only, no member
/// relays (the server's relay-off full mesh), no standby. Every receiver takes each stream from
/// its publisher, so each member's uplink carries `n - 1` copies of its voice. Base MTU: one
/// 20 ms Opus frame is one datagram at either MTU.
#[must_use]
pub fn scenario_audio_room(seed: u64, contention: bool, disc: Discipline, n: usize, mix: &str) -> Scenario {
    let mut rng = Rng::new(seed ^ 0xA0D1);
    let nodes = (0..n)
        .map(|i| NodeSpec { relay: false, ..NodeSpec::member(audio_mix_profile(mix, i), 500_000 + rng.next_u64() % 2_500_000) })
        .collect();
    let call = CallParams { video_bps: 0, standby: false, ..CallParams::default() };
    Scenario { name: format!("audio room {n} {mix}"), nodes, duration_us: 60_000_000, call, contention, disc, impairments: true, seed }
}

/// One audio-room line. Audio latency is each session's p50 and p95, then p50/p95/p99 over
/// sessions; uplink is each member's own media bytes on the wire over its time present.
#[must_use]
pub fn audio_room_line(n: usize, mix: &str, cond: &Condition, seeds: u64) -> String {
    let (mut sess, mut up, mut conserved, mut fan) = (Vec::new(), Vec::new(), true, 0u32);
    for seed in 1..=seeds {
        let r = crate::media::Engine::new(scenario_audio_room(seed, cond.contention, cond.disc, n, mix)).run();
        conserved &= r.ledger.balanced() && r.links_conserve;
        fan = fan.max(r.publisher_fanout.iter().copied().max().unwrap_or(0));
        up.extend(r.energy.iter().filter(|e| e.present_us > 0).map(|e| e.media_own_tx_bytes * 8_000 / e.present_us));
        sess.extend(r.sessions);
    }
    let aud: Vec<u64> = sess.iter().filter(|q| q.audio_sent > 0).map(|q| permille(q.audio_missing, q.audio_sent)).collect();
    let l50: Vec<u64> = sess.iter().filter_map(|q| crate::qoe::percentile(&q.audio_lat_samples, 500)).map(ms).collect();
    let l95: Vec<u64> = sess.iter().filter_map(|q| crate::qoe::percentile(&q.audio_lat_samples, 950)).map(ms).collect();
    // The worst publisher: the highest median audio-missing over the streams of one publisher.
    let worst_pub = (0..n as u16)
        .map(|p| {
            let v: Vec<u64> = sess.iter().filter(|q| q.stream == p && q.audio_sent > 0).map(|q| permille(q.audio_missing, q.audio_sent)).collect();
            crate::qoe::Dist::of(&v).p50
        })
        .max()
        .unwrap_or(0);
    format!(
        "N={n} {mix:<10} {:<10} sessions={} audioMiss_pm={} worstPublisher_audioMiss_p50_pm={worst_pub} audioLat_p50_ms={} audioLat_p95_ms={} uplink_kbps={} fanoutMax={fan} conserved={conserved}",
        cond.label,
        sess.len(),
        d3(&aud),
        d3(&l50),
        d3(&l95),
        d3(&up),
    )
}

/// The audio-room table at N = 4 and N = 9, household traffic on, then off.
#[must_use]
pub fn audio_room_lines(seeds: u64) -> Vec<String> {
    let mut out = vec![format!("audio-only full-mesh room (MODEL output; {seeds} seeds; p50/p95/p99 over sessions, uplink over members; no relay, no standby)")];
    for cond in [
        Condition { label: "busy,fifo", contention: true, disc: Discipline::TailDrop, delivery_report: false },
        Condition { label: "quiet,fifo", contention: false, disc: Discipline::TailDrop, delivery_report: false },
    ] {
        for n in [4, 9] {
            for mix in AUDIO_MIXES {
                out.push(audio_room_line(n, mix, &cond, seeds));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::{frame_budget, Engine};

    #[test]
    fn two_people_direct_or_through_one_relay() {
        assert_eq!(frame_budget(PROBED_MTU), 1_142);
        let d = Engine::new(wifi_direct(1, false, Discipline::TailDrop)).run();
        assert!(d.ledger.balanced() && d.links_conserve);
        assert_eq!(d.energy.len(), 2);
        assert!(d.energy.iter().all(|e| e.media_fwd_tx_copies == 0 && e.media_own_tx_copies > 0));
        let r = Engine::new(wifi_relayed(1, false, Discipline::TailDrop)).run();
        assert!(r.ledger.balanced() && r.links_conserve);
        // Every copy a member receives came through the relay; the relay publishes nothing.
        assert_eq!(r.energy[2].media_own_tx_copies, 0);
        assert!(r.energy[2].media_fwd_tx_copies > 0);
        let to_members: u64 = r.energy[..2].iter().map(|e| e.media_rx_copies).sum();
        assert!(to_members <= r.energy[2].media_fwd_tx_copies);
    }

    #[test]
    fn an_audio_room_is_a_full_mesh_of_voice() {
        let r = Engine::new(scenario_audio_room(1, false, Discipline::TailDrop, 4, "wifi")).run();
        assert!(r.ledger.balanced() && r.links_conserve);
        // Nobody forwards; every member sends its own voice to the three others.
        assert!(r.energy.iter().all(|e| e.media_fwd_tx_copies == 0 && e.media_own_tx_copies > 0));
        assert_eq!(r.publisher_fanout, vec![3; 4]);
        assert!(r.sessions.iter().all(|q| q.key_sent == 0));
    }
}
