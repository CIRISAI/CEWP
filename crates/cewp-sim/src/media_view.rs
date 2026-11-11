//! The globe's view of one room call (`HOOKS.md` §2, `FSD/twin/media.md` §9).
//!
//! Flat buffers only, so `kernel-wasm` can hand them to WebGL without walking a struct. One
//! deterministic scenario: [`scenario_view`] (the `churn` room with household traffic off: four
//! homes, six phones joining and leaving, two phones behind hard NAT taking every stream through
//! a home relay, that relay leaving with a signed end at 30 s). Every member, place and device is
//! generated.
//!
//! Each `media_view_*` function runs the scenario again (about a second of CPU natively); a
//! caller that wants more than one buffer calls [`media_view`] once and reads its fields.
//!
//! Field classes use the globe's codes: 0 MEASURED, 1 DERIVED, 2 MODEL, 3 PROJECTED,
//! 4 FRONTIER. Everything the engine computes is DERIVED from MODEL inputs (the access profiles
//! and media parameters of [`crate::media`]); the places are MODEL.

use std::collections::BTreeMap;

use crate::link::Discipline;
use crate::media::{scenario_churn, Engine, Scenario, METER_WINDOW_US};
use crate::micro::Rng;

/// Class code: DERIVED.
pub const CLASS_DERIVED: u8 = 1;
/// Class code: MODEL.
pub const CLASS_MODEL: u8 = 2;

/// Members buffer stride.
pub const MEMBERS_STRIDE: usize = 4;
/// Class of each members field: lat, lon, access profile, uplink.
pub const MEMBERS_CLASSES: [u8; MEMBERS_STRIDE] = [CLASS_MODEL, CLASS_MODEL, CLASS_MODEL, CLASS_MODEL];
/// Tree buffer stride.
pub const TREE_STRIDE: usize = 3;
/// Edges buffer stride (per step, per tree edge).
pub const EDGES_STRIDE: usize = 4;
/// Class of each edges field: rate, loss, queue delay, layer.
pub const EDGES_CLASSES: [u8; EDGES_STRIDE] = [CLASS_DERIVED; EDGES_STRIDE];
/// Receiver buffer stride (per step, per (stream, member)).
pub const RX_STRIDE: usize = 3;
/// Class of each receiver field: layer, stall, delay.
pub const RX_CLASSES: [u8; RX_STRIDE] = [CLASS_DERIVED; RX_STRIDE];
/// Events buffer stride.
pub const EVENTS_STRIDE: usize = 5;

/// Event kind: a parent left (its children lose that leg).
pub const EV_PARENT_LEFT: f32 = 0.0;
/// Event kind: the child's audio from that stream plays again.
pub const EV_AUDIO_RESUMED: f32 = 1.0;
/// Event kind: the child's video from that stream shows a frame again.
pub const EV_VIDEO_RESUMED: f32 = 2.0;
/// Event kind: the session ended with that medium never resumed.
pub const EV_NOT_RESUMED: f32 = 3.0;

/// Region anchors (lat, lon): two coarse metro areas; members scatter within about a degree, so
/// no point is an address. MODEL.
const REGION_ANCHORS: [(f32, f32); 2] = [(45.5, -73.6), (48.9, 2.3)];

/// The scenario the globe shows.
#[must_use]
pub fn scenario_view(seed: u64) -> Scenario {
    let mut sc = scenario_churn(seed, false, Discipline::TailDrop);
    sc.name = "globe view: churn room".to_owned();
    sc
}

/// Every buffer of one run.
#[derive(Debug, Clone, PartialEq)]
pub struct MediaView {
    /// Members.
    pub n: u32,
    /// Steps (one per second of the call).
    pub steps: u32,
    /// Tree edges.
    pub edge_count: u32,
    /// See [`media_view_members`].
    pub members: Vec<f32>,
    /// See [`media_view_tree`].
    pub tree: Vec<u32>,
    /// `steps × edge_count × EDGES_STRIDE`; see [`media_view_edges`].
    pub edges: Vec<f32>,
    /// `steps × n × n × RX_STRIDE`; see [`media_view_rx`].
    pub rx: Vec<f32>,
    /// See [`media_view_events`].
    pub events: Vec<f32>,
}

/// The engine's recorder: per tree edge, per step, what was sent and what arrived.
#[derive(Debug, Clone, Default)]
pub(crate) struct ViewRec {
    step_us: u64,
    steps: usize,
    /// (stream, parent, child), in order of first appearance.
    pub(crate) edges: Vec<(u16, u16, u16)>,
    index: BTreeMap<(u16, u16, u16), usize>,
    /// [step][edge] = (bytes sent, copies sent, copies verified at the child), by send step.
    acc: Vec<Vec<[u64; 3]>>,
    /// [step][edge] = (layer, queue ms) at the step's end.
    snaps: Vec<Vec<(u8, f32)>>,
}

impl ViewRec {
    pub(crate) fn new(step_us: u64, duration_us: u64) -> Self {
        let steps = usize::try_from(duration_us / step_us.max(1)).unwrap_or(0).max(1);
        Self { step_us: step_us.max(1), steps, acc: vec![Vec::new(); steps], snaps: vec![Vec::new(); steps], ..Self::default() }
    }

    pub(crate) fn step_of(&self, t_us: u64) -> usize {
        usize::try_from(t_us / self.step_us).unwrap_or(usize::MAX).min(self.steps - 1)
    }

    pub(crate) fn edge(&mut self, s: u16, p: u16, c: u16) -> usize {
        if let Some(&e) = self.index.get(&(s, p, c)) {
            return e;
        }
        let e = self.edges.len();
        self.edges.push((s, p, c));
        self.index.insert((s, p, c), e);
        e
    }

    fn cell(&mut self, step: usize, e: usize) -> &mut [u64; 3] {
        let row = &mut self.acc[step];
        if row.len() <= e {
            row.resize(e + 1, [0; 3]);
        }
        &mut row[e]
    }

    pub(crate) fn sent(&mut self, s: u16, p: u16, c: u16, bytes: u64, t_us: u64) {
        let e = self.edge(s, p, c);
        let st = self.step_of(t_us);
        let x = self.cell(st, e);
        x[0] += bytes;
        x[1] += 1;
    }

    pub(crate) fn recv(&mut self, s: u16, p: u16, c: u16, sent_us: u64) {
        let e = self.edge(s, p, c);
        let st = self.step_of(sent_us);
        self.cell(st, e)[2] += 1;
    }

    pub(crate) fn snap(&mut self, step: usize, row: Vec<(u8, f32)>) {
        self.snaps[step] = row;
    }
}

fn place(rng: &mut Rng, region: u8) -> (f32, f32) {
    let (lat, lon) = REGION_ANCHORS[usize::from(region) % REGION_ANCHORS.len()];
    let j = |rng: &mut Rng| (rng.f() as f32 - 0.5) * 2.0;
    (lat + j(rng), lon + j(rng) * 1.4)
}

fn profile_code(name: &str) -> f32 {
    match name {
        "fibre-home" => 0.0,
        "cable-home-wifi" => 1.0,
        "dsl-home" => 2.0,
        "phone-home-wifi" => 3.0,
        "phone-lte" => 4.0,
        _ => 5.0,
    }
}

/// Run the view scenario once and fill every buffer.
#[must_use]
pub fn media_view(seed: u64) -> MediaView {
    let sc = scenario_view(seed);
    let n = sc.nodes.len();
    let call = sc.call;
    let nodes = sc.nodes.clone();
    let (res, logs, vframes, aframes, rec) = Engine::new(sc).run_view(METER_WINDOW_US);
    let steps = rec.steps;
    let step_us = rec.step_us;

    let mut rng = Rng::new(seed ^ 0x61_0BE);
    let mut members = Vec::with_capacity(n * MEMBERS_STRIDE);
    for nd in &nodes {
        let (lat, lon) = place(&mut rng, nd.region);
        members.extend([lat, lon, profile_code(nd.profile.name), (nd.profile.up_bps / 8) as f32]);
    }

    let tree: Vec<u32> = rec.edges.iter().flat_map(|&(s, p, c)| [u32::from(s), u32::from(p), u32::from(c)]).collect();
    let ec = rec.edges.len();
    let mut edges = Vec::with_capacity(steps * ec * EDGES_STRIDE);
    for st in 0..steps {
        for e in 0..ec {
            let [bytes, sent, got] = rec.acc[st].get(e).copied().unwrap_or([0; 3]);
            let (layer, q) = rec.snaps[st].get(e).copied().unwrap_or((0, 0.0));
            let rate = (bytes * 1_000_000 / step_us) as f32;
            let loss = if sent == 0 { 0.0 } else { (sent - got.min(sent)) as f32 * 1_000.0 / sent as f32 };
            edges.extend([rate, loss, q, f32::from(layer)]);
        }
    }

    let enc = call.capture_encode_us;
    let mut rx = vec![0.0f32; steps * n * n * RX_STRIDE];
    for s in 0..n {
        for m in 0..n {
            let log = &logs[s * n + m];
            let active = |st: usize| {
                let (a, b) = (st as u64 * step_us, (st as u64 + 1) * step_us);
                m != s && log.end_us > log.start_us && log.start_us < b && log.end_us > a
            };
            // Per step: best layer shown, delay sums.
            let mut layer = vec![-1i8; steps];
            let mut dsum = vec![(0u64, 0u64); steps];
            let mut asum = vec![(0u64, 0u64); steps];
            for (f, &t) in log.vid_shown.iter().enumerate() {
                if t == 0 {
                    continue;
                }
                let st = (t / step_us) as usize;
                if st >= steps {
                    continue;
                }
                let meta = vframes[s][f];
                layer[st] = layer[st].max(if meta.t1 { 2 } else { 1 });
                dsum[st].0 += t + enc - meta.capture_us;
                dsum[st].1 += 1;
            }
            for (a, &t) in log.aud_play.iter().enumerate() {
                if t == 0 {
                    continue;
                }
                let st = (t / step_us) as usize;
                if st >= steps {
                    continue;
                }
                layer[st] = layer[st].max(0);
                asum[st].0 += t - aframes[s][a];
                asum[st].1 += 1;
            }
            let has_video = call.video_bps > 0 && !vframes[s].is_empty();
            for st in 0..steps {
                let o = ((st * n + s) * n + m) * RX_STRIDE;
                if !active(st) {
                    rx[o] = -1.0;
                    rx[o + 2] = -1.0;
                    continue;
                }
                rx[o] = f32::from(layer[st]);
                rx[o + 1] = if has_video && layer[st] < 1 { 1.0 } else { 0.0 };
                let (sum, k) = if dsum[st].1 > 0 { dsum[st] } else { asum[st] };
                rx[o + 2] = if k == 0 { 0.0 } else { (sum / k) as f32 / 1_000.0 };
            }
        }
    }

    let mut events = Vec::new();
    for ob in &res.recoveries {
        let r = ob.rec;
        let (s, c) = (usize::from(r.stream), usize::from(r.child));
        let log = &logs[s * n + c];
        let ids = [f32::from(r.stream), f32::from(r.child), f32::from(r.parent)];
        events.extend([r.leave_us as f32 / 1_000.0, EV_PARENT_LEFT]);
        events.extend(ids);
        let after = |ts: &[u64]| ts.iter().copied().filter(|&t| t > r.leave_us && t <= log.end_us).min();
        for (kind, t) in [(EV_AUDIO_RESUMED, after(&log.aud_play)), (EV_VIDEO_RESUMED, after(&log.vid_shown))] {
            match t {
                Some(t) => events.extend([t as f32 / 1_000.0, kind]),
                None => events.extend([log.end_us as f32 / 1_000.0, EV_NOT_RESUMED]),
            }
            events.extend(ids);
        }
    }

    MediaView { n: n as u32, steps: steps as u32, edge_count: ec as u32, members, tree, edges, rx, events }
}

/// Members, stride 4 per member (index = member id, the same ids as the tree):
/// `[lat deg, lon deg, access profile, uplink bytes/s]`. Access profile codes: 0 fibre home
/// (wired desktop), 1 cable home laptop on Wi-Fi, 2 DSL home, 3 phone on home Wi-Fi, 4 phone on
/// LTE. Uplink is the profile's nominal rate. Classes [`MEMBERS_CLASSES`]: all MODEL (generated
/// places, MODEL profiles).
#[must_use]
pub fn media_view_members(seed: u64) -> Vec<f32> {
    media_view(seed).members
}

/// Every tree edge that existed at any step, stride 3: `[stream, parent, child]` (member ids;
/// a stream's root is its publisher, `stream == parent` for a direct leg). Every member
/// publishes, so a room call is one tree per stream. Edge `k` of this buffer is edge `k` of
/// every [`media_view_edges`] step. DERIVED.
#[must_use]
pub fn media_view_tree(seed: u64) -> Vec<u32> {
    media_view(seed).tree
}

/// Number of steps (one per second).
#[must_use]
pub fn media_view_steps(seed: u64) -> u32 {
    media_view(seed).steps
}

/// Per tree edge at `step`, stride 4, aligned to [`media_view_tree`]:
/// `[rate bytes/s, loss permille, queue delay ms, layer]`. Rate: wire bytes the parent put on
/// its uplink toward that child during the step. Loss: of the copies sent during the step, the
/// share never verified at the child (copies still in flight when the run ends count as lost).
/// Queue delay: parent uplink plus child downlink backlog at the step's end, at the current
/// rate. Layer: 0 not attached, 1 audio + base layer (a warm standby, or a receiver's base-layer
/// cap), 2 audio + both video layers. Classes [`EDGES_CLASSES`]: all DERIVED. Empty past the
/// last step.
#[must_use]
pub fn media_view_edges(seed: u64, step: u32) -> Vec<f32> {
    let v = media_view(seed);
    let w = v.edge_count as usize * EDGES_STRIDE;
    v.edges.get(step as usize * w..(step as usize + 1) * w).map(<[f32]>::to_vec).unwrap_or_default()
}

/// Per (stream, member) at `step`, stride 3, index `stream * n + member`:
/// `[layer, stall, delay ms]`. No session this step (not both present, or the member's own
/// stream): `[-1, 0, -1]`. In a session: layer -1 nothing played, 0 audio only (presence
/// floor), 1 base video layer shown, 2 both layers shown; stall 1 when the stream has video
/// and no frame was shown this step; delay = mean glass-to-glass of frames shown this step
/// (capture + encode included), or of audio frames played when no frame was shown, 0 when
/// neither. Classes [`RX_CLASSES`]: all DERIVED. Empty
/// past the last step.
#[must_use]
pub fn media_view_rx(seed: u64, step: u32) -> Vec<f32> {
    let v = media_view(seed);
    let w = (v.n as usize).pow(2) * RX_STRIDE;
    v.rx.get(step as usize * w..(step as usize + 1) * w).map(<[f32]>::to_vec).unwrap_or_default()
}

/// The moments a parent leaves and the tree repairs, stride 5:
/// `[time ms, kind, stream, child, parent that left]`. Per departure, three rows: the leave
/// ([`EV_PARENT_LEFT`]), then audio and video each resumed ([`EV_AUDIO_RESUMED`],
/// [`EV_VIDEO_RESUMED`]) or not by the session's end ([`EV_NOT_RESUMED`], at that end). DERIVED.
#[must_use]
pub fn media_view_events(seed: u64) -> Vec<f32> {
    media_view(seed).events
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_view_is_deterministic_and_well_shaped() {
        let a = media_view(1);
        assert_eq!(a, media_view(1));
        let (n, steps, ec) = (a.n as usize, a.steps as usize, a.edge_count as usize);
        assert_eq!(a.members.len(), n * MEMBERS_STRIDE);
        assert_eq!(a.tree.len(), ec * TREE_STRIDE);
        assert_eq!(a.edges.len(), steps * ec * EDGES_STRIDE);
        assert_eq!(a.rx.len(), steps * n * n * RX_STRIDE);
        assert_eq!(a.events.len() % EVENTS_STRIDE, 0);
        assert!(ec > 0 && steps == 60);
        for t in a.tree.chunks(TREE_STRIDE) {
            assert!(t.iter().all(|&x| (x as usize) < n));
            assert_ne!(t[1], t[2]);
        }
        for e in a.edges.chunks(EDGES_STRIDE) {
            assert!(e[0] >= 0.0 && (0.0..=1_000.0).contains(&e[1]) && e[2] >= 0.0 && (0.0..=2.0).contains(&e[3]));
        }
        // A relay's departure is in the scenario: at least one parent left and the
        // tree's repair (or its absence) is recorded for both media.
        let ev: Vec<&[f32]> = a.events.chunks(EVENTS_STRIDE).collect();
        let left = ev.iter().filter(|e| e[1] == EV_PARENT_LEFT).count();
        assert!(left > 0);
        assert_eq!(ev.len(), 3 * left);
        // Someone sees video at some step.
        assert!(a.rx.chunks(RX_STRIDE).any(|r| r[0] >= 1.0));
        // The per-step accessors slice the same buffers.
        assert_eq!(media_view_edges(1, 5), a.edges[5 * ec * EDGES_STRIDE..6 * ec * EDGES_STRIDE]);
        assert!(media_view_edges(1, a.steps).is_empty());
    }

    #[test]
    #[ignore = "diagnostic dump, run by hand"]
    fn dump_view() {
        let v = media_view(1);
        println!("n {} steps {} edges {} events {}", v.n, v.steps, v.edge_count, v.events.len() / EVENTS_STRIDE);
        for t in v.tree.chunks(TREE_STRIDE) {
            print!("{:?} ", t);
        }
        println!();
        for st in [5usize, 20, 29, 31, 35, 50] {
            let w = v.edge_count as usize * EDGES_STRIDE;
            let row: Vec<String> = v.edges[st * w..(st + 1) * w].chunks(EDGES_STRIDE).map(|e| format!("{:.0}/{:.0}/{:.0}/{}", e[0], e[1], e[2], e[3])).collect();
            println!("step {st} edges {}", row.join(" "));
            let n = v.n as usize;
            let rw = n * n * RX_STRIDE;
            let rx: Vec<String> = v.rx[st * rw..(st + 1) * rw].chunks(RX_STRIDE).map(|r| format!("{}{}", r[0], if r[1] > 0.0 { "s" } else { "" })).collect();
            println!("step {st} rx {}", rx.join(" "));
        }
        for e in v.events.chunks(EVENTS_STRIDE) {
            println!("event {:?}", e);
        }
        for m in v.members.chunks(MEMBERS_STRIDE) {
            println!("member {:?}", m);
        }
    }
}
