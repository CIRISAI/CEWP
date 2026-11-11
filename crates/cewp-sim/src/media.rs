//! A room call over physical access links (`FSD/twin/media.md`).
//!
//! The call traffic and the relay controller are DERIVED from what CIRISEdge v45 ships
//! (`8e46165`, with its leviculum pin `v0.34.0+ciris.3`); each constant below names its file and
//! symbol. The links it rides are [`crate::link`]: asymmetric access links with household
//! traffic, Wi-Fi airtime, cellular variation, Gilbert-Elliott loss, and a stated queue
//! discipline. The engine is discrete-event over integer microseconds and bytes, seeded.
//!
//! What runs, per packet:
//!
//! 1. A publisher captures an Opus audio frame every 20 ms and an H.264 frame at `fps`, on two
//!    temporal layers (class `0x04`, `realtime_av.rs` `CODEC_TEMPORAL_LAYERED`: one spatial
//!    layer; t0 = base, t1 = enhancement). A frame larger than the frame budget is split into
//!    marked fragments (`av_call/frame.rs` `split`), each its own chunk with its own `chunk_seq`;
//!    audio and video share one `chunk_seq` per stream (`frame.rs` module doc).
//! 2. Each chunk is signed once on the publisher's CPU (`av_chunk_auth.rs`, one Ed25519 per
//!    chunk), then copied to every admitted child as an unreliable datagram (leviculum#102;
//!    `send_unordered`: "no window, no proof, no retransmit").
//! 3. A copy crosses the sender's uplink queue, the core (a propagation delay), and the
//!    receiver's downlink queue; the receiver verifies its signature on its CPU (every relay and
//!    every receiver verifies, `av_call.rs` module doc "Source authentication"), drops it if it
//!    waited longer than `MEDIA_STALE_BUDGET`, forwards it to its own children once
//!    (`FORWARD_DEDUPE`), and feeds its reorder window (`av_call.rs` `ReorderWindow`).
//! 4. The window releases in `chunk_seq` order, waiting for a gap up to
//!    `clamp(2 · gap_ewma, 40 ms, 400 ms)` and then skipping it; a frame is shown only whole and
//!    only when its reference chain holds (no keyframe request exists in edge v45).
//!
//! The controller (who sends to whom) follows `av_call/alm.rs` and
//! `realtime_av_alm/controller/{admission,selector}.rs`, simplified as `FSD/twin/media.md` §3
//! states: direct-first parent choice, admission against `floor + probe` with the probe
//! allowance doubling on use, one warm standby (audio + base layer), failover on link close or
//! silence, voluntary moves off a degraded parent under hysteresis and dwell.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap, VecDeque};

use crate::link::{
    chance_ppm, BackgroundModel, CapacityModel, CellModel, Discipline, Enq, GeParams, LinkDir, LinkStats, Queued, RenoFlow, BG_SEGMENT_WIRE,
    BG_MSS,
};
use crate::micro::Rng;
use crate::qoe::{amp_buckets, score_recovery, score_session, AmpBucket, QoeRules, RecoveryObs, RecoveryRec, SessionLog, SessionQoe, VMeta};

// ------------------------------------------------------------------ provenance

/// FSD §8.2 class of a media-model number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MTag {
    /// Read from edge / leviculum source at the pinned commit; the source names the symbol.
    Derived,
    /// A stated modelling choice with no measurement behind it.
    Model,
    /// An open empirical question; the default is a placeholder.
    Frontier,
}

impl MTag {
    /// As written in the FSD.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Derived => "DERIVED",
            Self::Model => "MODEL",
            Self::Frontier => "FRONTIER",
        }
    }
}

/// One named number of the media model.
#[derive(Debug, Clone, Copy)]
pub struct MParam {
    /// Name.
    pub name: &'static str,
    /// Default value.
    pub value: i64,
    /// Unit.
    pub unit: &'static str,
    /// Class.
    pub tag: MTag,
    /// Citation or reasoning.
    pub source: &'static str,
}

// ------------------------------------------------------------------ edge v45 / leviculum, DERIVED

/// leviculum-core `constants.rs` `MTU`: the base Reticulum MTU.
pub const RNS_BASE_MTU: u32 = 500;
/// leviculum-core `HEADER_MINSIZE` = 2 + 1 + 16.
pub const RNS_HEADER_MINSIZE: u32 = 19;
/// leviculum-core `TOKEN_OVERHEAD` = IV 16 + HMAC 32.
pub const RNS_TOKEN_OVERHEAD: u32 = 48;
/// leviculum-core `AES_BLOCK_SIZE`.
pub const RNS_AES_BLOCK: u32 = 16;
/// leviculum-core `IFAC_MIN_SIZE`.
pub const RNS_IFAC_MIN: u32 = 1;
/// leviculum-core `link/datagram.rs` `DATAGRAM_HEADER_LEN` (the sequence number).
pub const DATAGRAM_HEADER_LEN: u32 = 4;
/// edge `link_seal.rs` `SEAL_OVERHEAD` = `SEAL_HEADER_LEN` 9 + `SEAL_TAG_LEN` 16 (E1 per-link seal).
pub const SEAL_OVERHEAD: u32 = 25;
/// edge `realtime_av.rs` `AV_CHUNK_WIRE_OVERHEAD` = header 48 + codec/layer 4 + two AEAD tags 32.
pub const AV_CHUNK_WIRE_OVERHEAD: u32 = 84;
/// edge `av_chunk_auth.rs` `CHUNK_TRAILER_LEN` = source timestamp 8 + Ed25519 signature 64.
pub const CHUNK_TRAILER_LEN: u32 = 72;
/// edge `av_call/frame.rs` `FRAGMENT_MARKER_LEN`.
pub const FRAGMENT_MARKER_LEN: u32 = 8;
/// edge `av_call/frame.rs` `MAX_FRAME_FRAGMENTS`.
pub const MAX_FRAME_FRAGMENTS: u32 = 4_096;
/// edge `av_call/frame.rs` `MAX_FRAME_BYTES`.
pub const MAX_FRAME_BYTES: u32 = 1 << 20;
/// edge `av_call.rs` `REORDER_WINDOW_MIN`.
pub const REORDER_WINDOW_MIN_US: u64 = 40_000;
/// edge `av_call.rs` `REORDER_WINDOW_MAX`.
pub const REORDER_WINDOW_MAX_US: u64 = 400_000;
/// edge `av_call.rs` `REORDER_GAP_PRIOR`.
pub const REORDER_GAP_PRIOR_US: u64 = 50_000;
/// edge `av_call.rs` `MEDIA_STALE_BUDGET`.
pub const MEDIA_STALE_BUDGET_US: u64 = 800_000;
/// edge `av_call.rs` `WATCH_TICK`.
pub const WATCH_TICK_US: u64 = 250_000;
/// edge `av_call/alm.rs` `PUBLISHER_INITIAL_FANOUT`.
pub const PUBLISHER_INITIAL_FANOUT: u32 = 8;
/// edge `controller/admission.rs` `MAX_PROBE_STREAMS`.
pub const MAX_PROBE_STREAMS: u32 = 16;
/// edge `ControllerConfig::interactive` `probe_clean_windows`.
pub const PROBE_CLEAN_WINDOWS: u32 = 2;
/// edge `admission.rs`: the probe grows when `committed >= 0.8 · (floor + probe)`.
pub const PROBE_USED_PERMILLE: u64 = 800;
/// edge `ControllerConfig::interactive` `meter_window_ms`.
pub const METER_WINDOW_US: u64 = 1_000_000;
/// edge `ControllerConfig::interactive` `join_jitter_ms`.
pub const JOIN_JITTER_US: u64 = 1_000_000;
/// edge `ControllerConfig::interactive` `backoff_base_ms`.
pub const BACKOFF_BASE_US: u64 = 500_000;
/// edge `ControllerConfig::interactive` `backoff_max_ms`.
pub const BACKOFF_MAX_US: u64 = 8_000_000;
/// edge `ControllerConfig::interactive` `min_dwell_ms`.
pub const MIN_DWELL_US: u64 = 10_000_000;
/// edge `selector.rs` `FLAP_WINDOW_MS`.
pub const FLAP_WINDOW_US: u64 = 120_000_000;
/// edge `ControllerConfig::interactive` `hysteresis_hold_windows`.
pub const HYSTERESIS_HOLD_WINDOWS: u32 = 3;
/// edge `ControllerConfig::interactive` `degraded_loss` (0.05).
pub const DEGRADED_LOSS_PERMILLE: u64 = 50;
/// edge `ControllerConfig::interactive` `latency_budget_ms`.
pub const LATENCY_BUDGET_US: u64 = 400_000;
/// edge `ControllerConfig::interactive` `max_depth`.
pub const MAX_DEPTH: u8 = 3;
/// edge `heal.rs` `PARENT_SILENCE_HEAL_MS` (= `suspect_silence_ms`).
pub const SUSPECT_SILENCE_US: u64 = 1_500_000;
/// edge `av_call/alm.rs` `MIN_STREAM_BPS`.
pub const MIN_STREAM_BPS: u64 = 16_000;
/// edge `av_call/alm.rs` `MIN_AUDIO_BPS`.
pub const MIN_AUDIO_BPS: u64 = 8_000;
/// edge `path_meter.rs` `UPLINK_SAFETY_MARGIN` (0.75).
pub const UPLINK_SAFETY_MARGIN_PERMILLE: u64 = 750;
/// edge `path_meter.rs` `UPLINK_REFUSAL_WINDOWS`.
pub const UPLINK_REFUSAL_WINDOWS: u32 = 3;
/// edge `admission.rs` `CEILING_STALE_MS`.
pub const CEILING_STALE_US: u64 = 60_000_000;
/// leviculum-core `LINK_KEEPALIVE_SECS` (the keepalive at and above `LINK_KEEPALIVE_MAX_RTT`).
pub const LINK_KEEPALIVE_MAX_US: u64 = 360_000_000;
/// leviculum-core `LINK_KEEPALIVE_MIN_SECS`.
pub const LINK_KEEPALIVE_MIN_US: u64 = 5_000_000;
/// leviculum-core `LINK_KEEPALIVE_MAX_RTT` (1.75 s).
pub const LINK_KEEPALIVE_MAX_RTT_US: u64 = 1_750_000;
/// leviculum-core `LINK_STALE_FACTOR`.
pub const LINK_STALE_FACTOR: u64 = 2;
/// leviculum-core `LINK_STALE_GRACE_SECS`.
pub const LINK_STALE_GRACE_US: u64 = 5_000_000;

/// The encrypted link MDU for an RNS MTU (leviculum-core `link/mod.rs` `compute_link_mdu`).
#[must_use]
pub const fn link_mdu(mtu: u32) -> u32 {
    ((mtu - RNS_IFAC_MIN - RNS_HEADER_MINSIZE - RNS_TOKEN_OVERHEAD) / RNS_AES_BLOCK) * RNS_AES_BLOCK - 1
}

/// The most one media datagram carries (edge `reticulum.rs` `link_datagram_message_limit`:
/// leviculum `link_datagram_mdu` = MDU − sequence number, less the E1 seal).
#[must_use]
pub const fn media_limit(mtu: u32) -> u32 {
    link_mdu(mtu) - DATAGRAM_HEADER_LEN - SEAL_OVERHEAD
}

/// The largest frame plaintext one chunk carries (edge `av_call.rs` `LegInfo::frame_budget`:
/// the media limit less the chunk's wire overhead and its trailer).
#[must_use]
pub const fn frame_budget(mtu: u32) -> u32 {
    media_limit(mtu) - AV_CHUNK_WIRE_OVERHEAD - CHUNK_TRAILER_LEN
}

/// Bytes of one RNS link packet carrying `payload` link-plaintext bytes: header, then the
/// token (IV, PKCS7-padded AES-CBC, HMAC).
#[must_use]
pub const fn rns_packet_bytes(payload: u32) -> u32 {
    RNS_HEADER_MINSIZE + 16 + (payload / RNS_AES_BLOCK + 1) * RNS_AES_BLOCK + 32
}

/// RNS packet bytes of one media chunk with `plaintext` bytes of chunk plaintext.
#[must_use]
pub const fn chunk_rns_bytes(plaintext: u32) -> u32 {
    rns_packet_bytes(plaintext + AV_CHUNK_WIRE_OVERHEAD + CHUNK_TRAILER_LEN + SEAL_OVERHEAD + DATAGRAM_HEADER_LEN)
}

/// Fragments a video frame of `bytes` takes at `budget` (edge `frame.rs` `split`; an empty
/// frame is one fragment). `None` when edge would refuse it.
#[must_use]
pub fn fragments(bytes: u32, budget: u32) -> Option<u32> {
    if bytes > MAX_FRAME_BYTES || budget <= FRAGMENT_MARKER_LEN {
        return None;
    }
    let n = bytes.div_ceil(budget - FRAGMENT_MARKER_LEN).max(1);
    (n <= MAX_FRAME_FRAGMENTS).then_some(n)
}

/// Closed-form loss of a frame of `n` fragments under independent chunk loss `p`.
#[must_use]
pub fn frame_loss_closed_form(p: f64, n: u32) -> f64 {
    1.0 - (1.0 - p).powi(n as i32)
}

// ------------------------------------------------------------------ MODEL defaults

/// IPv4 + UDP header per datagram (bytes).
pub const IP_UDP_HEADER: u32 = 28;
/// Path MTU on a home or cellular path (bytes).
pub const PATH_MTU: u32 = 1_500;
/// Rounds trips from a publisher's dial to its first media on a leg: RNS link request/proof,
/// transit KEX, stream hello, binding parts (~10 Channel messages, a few windows).
pub const SETUP_RTTS: u64 = 4;
/// One-way core delay between two nodes in the same region (us).
pub const CORE_SAME_REGION_US: u64 = 5_000;
/// One-way core delay between regions (us).
pub const CORE_CROSS_REGION_US: u64 = 40_000;
/// One-way core delay from an access link to a household transfer's far end (us).
pub const SERVER_CORE_US: u64 = 15_000;
/// Share of crashes (no signed end) among phone departures (ppm).
pub const PHONE_CRASH_PPM: u32 = 200_000;

/// Every number of the media model, its unit, class and source.
pub const MEDIA_PARAM_SPECS: &[MParam] = &[
    MParam { name: "rns_mtu", value: RNS_BASE_MTU as i64, unit: "B", tag: MTag::Derived, source: "leviculum-core constants.rs MTU (base); a punched path may probe larger (leviculum#104)" },
    MParam { name: "link_mdu", value: link_mdu(RNS_BASE_MTU) as i64, unit: "B", tag: MTag::Derived, source: "leviculum-core link/mod.rs compute_link_mdu(500)" },
    MParam { name: "media_limit", value: media_limit(RNS_BASE_MTU) as i64, unit: "B", tag: MTag::Derived, source: "edge reticulum.rs link_datagram_message_limit: MDU - DATAGRAM_HEADER_LEN 4 - SEAL_OVERHEAD 25" },
    MParam { name: "frame_budget", value: frame_budget(RNS_BASE_MTU) as i64, unit: "B", tag: MTag::Derived, source: "edge av_call.rs LegInfo::frame_budget = media_limit - AV_CHUNK_WIRE_OVERHEAD 84 - CHUNK_TRAILER_LEN 72" },
    MParam { name: "fragment_payload", value: (frame_budget(RNS_BASE_MTU) - FRAGMENT_MARKER_LEN) as i64, unit: "B", tag: MTag::Derived, source: "edge av_call/frame.rs split: budget - FRAGMENT_MARKER_LEN 8" },
    MParam { name: "full_chunk_wire", value: (chunk_rns_bytes(frame_budget(RNS_BASE_MTU)) + IP_UDP_HEADER) as i64, unit: "B", tag: MTag::Derived, source: "rns_packet_bytes(431) = 19 + 16 + 432 + 32, + IPv4/UDP 28" },
    MParam { name: "reorder_window_min", value: REORDER_WINDOW_MIN_US as i64, unit: "us", tag: MTag::Derived, source: "edge av_call.rs REORDER_WINDOW_MIN" },
    MParam { name: "reorder_window_max", value: REORDER_WINDOW_MAX_US as i64, unit: "us", tag: MTag::Derived, source: "edge av_call.rs REORDER_WINDOW_MAX" },
    MParam { name: "reorder_gap_prior", value: REORDER_GAP_PRIOR_US as i64, unit: "us", tag: MTag::Derived, source: "edge av_call.rs REORDER_GAP_PRIOR (window = 2x gap EWMA)" },
    MParam { name: "media_stale_budget", value: MEDIA_STALE_BUDGET_US as i64, unit: "us", tag: MTag::Derived, source: "edge av_call.rs MEDIA_STALE_BUDGET (inbound queue age drop)" },
    MParam { name: "watch_tick", value: WATCH_TICK_US as i64, unit: "us", tag: MTag::Derived, source: "edge av_call.rs WATCH_TICK (leg checks)" },
    MParam { name: "publisher_initial_fanout", value: PUBLISHER_INITIAL_FANOUT as i64, unit: "streams", tag: MTag::Derived, source: "edge av_call/alm.rs PUBLISHER_INITIAL_FANOUT; probe = clamp(members-1, 1, 8)" },
    MParam { name: "max_probe_streams", value: MAX_PROBE_STREAMS as i64, unit: "streams", tag: MTag::Derived, source: "edge controller/admission.rs MAX_PROBE_STREAMS" },
    MParam { name: "probe_clean_windows", value: PROBE_CLEAN_WINDOWS as i64, unit: "windows", tag: MTag::Derived, source: "edge ControllerConfig::interactive" },
    MParam { name: "join_jitter", value: JOIN_JITTER_US as i64, unit: "us", tag: MTag::Derived, source: "edge ControllerConfig::interactive join_jitter_ms" },
    MParam { name: "backoff_base", value: BACKOFF_BASE_US as i64, unit: "us", tag: MTag::Derived, source: "edge ControllerConfig::interactive backoff_base_ms (x2^k x U(0.5,1.5))" },
    MParam { name: "min_dwell", value: MIN_DWELL_US as i64, unit: "us", tag: MTag::Derived, source: "edge ControllerConfig::interactive min_dwell_ms (x2^k, k = voluntary switches in 120 s)" },
    MParam { name: "hysteresis_hold_windows", value: HYSTERESIS_HOLD_WINDOWS as i64, unit: "windows", tag: MTag::Derived, source: "edge ControllerConfig::interactive" },
    MParam { name: "degraded_loss", value: DEGRADED_LOSS_PERMILLE as i64, unit: "permille", tag: MTag::Derived, source: "edge ControllerConfig::interactive degraded_loss 0.05" },
    MParam { name: "latency_budget", value: LATENCY_BUDGET_US as i64, unit: "us", tag: MTag::Derived, source: "edge ControllerConfig::interactive latency_budget_ms; also the audio deadline here (MODEL use)" },
    MParam { name: "max_depth", value: MAX_DEPTH as i64, unit: "hops", tag: MTag::Derived, source: "edge ControllerConfig::interactive max_depth" },
    MParam { name: "suspect_silence", value: SUSPECT_SILENCE_US as i64, unit: "us", tag: MTag::Derived, source: "edge heal.rs PARENT_SILENCE_HEAL_MS; selector scales silence by 2/(2+depth)" },
    MParam { name: "link_keepalive_max", value: LINK_KEEPALIVE_MAX_US as i64, unit: "us", tag: MTag::Derived, source: "leviculum-core LINK_KEEPALIVE_SECS (stale = 2x keepalive + 5 s grace; FRONTIER: stale path not traced)" },
    MParam { name: "ip_udp_header", value: IP_UDP_HEADER as i64, unit: "B", tag: MTag::Model, source: "IPv4 20 + UDP 8; IPv6 would be 48" },
    MParam { name: "path_mtu", value: PATH_MTU as i64, unit: "B", tag: MTag::Model, source: "Ethernet/most broadband and cellular paths" },
    MParam { name: "setup_rtts", value: SETUP_RTTS as i64, unit: "RTT", tag: MTag::Model, source: "link request/proof + transit KEX + hello + binding parts; count not measured" },
    MParam { name: "core_same_region", value: CORE_SAME_REGION_US as i64, unit: "us", tag: MTag::Model, source: "one-way metro core; replaceable by RIPE Atlas latency (approval request)" },
    MParam { name: "core_cross_region", value: CORE_CROSS_REGION_US as i64, unit: "us", tag: MTag::Model, source: "one-way continental core" },
    MParam { name: "server_core", value: SERVER_CORE_US as i64, unit: "us", tag: MTag::Model, source: "household transfer far end, one way" },
    MParam { name: "audio_bitrate", value: 24_000, unit: "bit/s", tag: MTag::Model, source: "Opus VOIP mono; edge is codec-opaque for audio, the host sets it" },
    MParam { name: "audio_frame", value: 20_000, unit: "us", tag: MTag::Model, source: "Opus 20 ms (edge opus_voice.rs tests use 20 ms; host config not in edge)" },
    MParam { name: "video_bitrate", value: 500_000, unit: "bit/s", tag: MTag::Model, source: "phone H.264 camera; edge does not set or adapt the encoder" },
    MParam { name: "video_fps", value: 15, unit: "fps", tag: MTag::Model, source: "phone camera call rate" },
    MParam { name: "temporal_layers", value: 2, unit: "layers", tag: MTag::Model, source: "H.264 under 0x04 is temporal-layered (DERIVED); two layers, t1 non-reference, is the model" },
    MParam { name: "gop", value: 2_000_000, unit: "us", tag: MTag::Model, source: "periodic IDR; edge v45 has no keyframe request, so a lost base frame freezes to the next IDR" },
    MParam { name: "keyframe_ratio", value: 8_000, unit: "milli", tag: MTag::Model, source: "IDR size / t0 P-frame size" },
    MParam { name: "t1_ratio", value: 700, unit: "permille", tag: MTag::Model, source: "t1 (non-reference) frame size / t0 P-frame size" },
    MParam { name: "frame_size_jitter", value: 200, unit: "permille", tag: MTag::Model, source: "uniform +-20% per frame" },
    MParam { name: "capture_encode", value: 50_000, unit: "us", tag: MTag::Model, source: "camera exposure/readout + encoder" },
    MParam { name: "decode_render", value: 30_000, unit: "us", tag: MTag::Model, source: "decoder + compositor + display" },
    MParam { name: "verify_phone", value: 150, unit: "us", tag: MTag::Frontier, source: "Ed25519 verify on a phone core; MEASURED number requested (debug build is 4.8 ms)" },
    MParam { name: "verify_desktop", value: 60, unit: "us", tag: MTag::Frontier, source: "Ed25519 verify on a desktop core; MEASURED number requested" },
    MParam { name: "sign_phone", value: 60, unit: "us", tag: MTag::Frontier, source: "Ed25519 sign on a phone core" },
    MParam { name: "sign_desktop", value: 25, unit: "us", tag: MTag::Frontier, source: "Ed25519 sign on a desktop core" },
    MParam { name: "phone_crash_share", value: PHONE_CRASH_PPM as i64, unit: "ppm", tag: MTag::Model, source: "phone departures with no signed end (app killed, radio lost)" },
];

// ------------------------------------------------------------------ access profiles

/// One participant's access: both link directions, their buffers, loss, CPU, and the household
/// traffic that shares them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccessProfile {
    /// Name.
    pub name: &'static str,
    /// Nominal uplink (bit/s).
    pub up_bps: u64,
    /// Nominal downlink (bit/s).
    pub down_bps: u64,
    /// Uplink buffer, in ms at the nominal rate.
    pub up_buffer_ms: u64,
    /// Downlink buffer, in ms at the nominal rate.
    pub down_buffer_ms: u64,
    /// One-way last-mile delay (us).
    pub last_mile_us: u64,
    /// Wi-Fi airtime share (permille; 1000 = wired or cellular radio).
    pub airtime_permille: u32,
    /// Cellular capacity variation.
    pub cell: Option<CellModel>,
    /// Medium loss.
    pub ge: GeParams,
    /// Ed25519 verify per chunk (us).
    pub verify_us: u64,
    /// Ed25519 sign per chunk (us).
    pub sign_us: u64,
    /// Household uplink traffic.
    pub bg_up: BackgroundModel,
    /// Household downlink traffic.
    pub bg_down: BackgroundModel,
    /// May relay (CC 5.4.6: server and home by default; a phone only while charging and
    /// unmetered).
    pub relay: bool,
}

/// Wired residential loss: rare, short bursts. MODEL.
pub const GE_WIRED: GeParams = GeParams { p_gb_ppm: 100, p_bg_ppm: 500_000, loss_good_ppm: 10, loss_bad_ppm: 300_000 };
/// Home Wi-Fi: bursty loss from collisions and fades. MODEL.
pub const GE_WIFI: GeParams = GeParams { p_gb_ppm: 3_000, p_bg_ppm: 250_000, loss_good_ppm: 500, loss_bad_ppm: 250_000 };
/// LTE after HARQ/RLC: low residual loss, bursts on handover. MODEL.
pub const GE_LTE: GeParams = GeParams { p_gb_ppm: 1_000, p_bg_ppm: 100_000, loss_good_ppm: 1_000, loss_bad_ppm: 300_000 };
/// LTE capacity process: 500 ms steps among 100/60/30/10 % of nominal. MODEL.
pub const CELL_LTE: CellModel = CellModel { levels_permille: [1000, 600, 300, 100], period_us: 500_000, stay_ppm: 900_000 };
/// Household uploads: a transfer every ~45 s; 15% are a 25 MB backup, the rest 300 KB. MODEL.
pub const BG_UP_HOME: BackgroundModel = BackgroundModel { mean_gap_ms: 45_000, large_ppm: 150_000, small_bytes: 300_000, large_bytes: 25_000_000, periodic_bytes: 0, period_ms: 0 };
/// Household downloads: web fetches plus a 4 Mbit/s video player (2 MB every 4 s). MODEL.
pub const BG_DOWN_HOME: BackgroundModel = BackgroundModel { mean_gap_ms: 10_000, large_ppm: 100_000, small_bytes: 1_500_000, large_bytes: 50_000_000, periodic_bytes: 2_000_000, period_ms: 4_000 };
/// A phone on cellular: app uploads and downloads, no video player. MODEL.
pub const BG_UP_PHONE: BackgroundModel = BackgroundModel { mean_gap_ms: 60_000, large_ppm: 50_000, small_bytes: 200_000, large_bytes: 5_000_000, periodic_bytes: 0, period_ms: 0 };
/// Phone downloads. MODEL.
pub const BG_DOWN_PHONE: BackgroundModel = BackgroundModel { mean_gap_ms: 20_000, large_ppm: 50_000, small_bytes: 500_000, large_bytes: 10_000_000, periodic_bytes: 0, period_ms: 0 };

/// Fibre home, wired desktop. MODEL rates.
pub const FIBER_HOME: AccessProfile = AccessProfile {
    name: "fibre-home", up_bps: 50_000_000, down_bps: 300_000_000, up_buffer_ms: 50, down_buffer_ms: 50, last_mile_us: 2_000,
    airtime_permille: 1000, cell: None, ge: GE_WIRED, verify_us: 60, sign_us: 25, bg_up: BG_UP_HOME, bg_down: BG_DOWN_HOME, relay: true,
};
/// Cable home, laptop on Wi-Fi: a bloated cable-modem uplink buffer. MODEL rates.
pub const CABLE_HOME_WIFI: AccessProfile = AccessProfile {
    name: "cable-home-wifi", up_bps: 10_000_000, down_bps: 150_000_000, up_buffer_ms: 300, down_buffer_ms: 100, last_mile_us: 8_000,
    airtime_permille: 600, cell: None, ge: GE_WIFI, verify_us: 60, sign_us: 25, bg_up: BG_UP_HOME, bg_down: BG_DOWN_HOME, relay: true,
};
/// DSL home, wired: a thin uplink with a deep buffer. MODEL rates.
pub const DSL_HOME: AccessProfile = AccessProfile {
    name: "dsl-home", up_bps: 1_000_000, down_bps: 16_000_000, up_buffer_ms: 500, down_buffer_ms: 200, last_mile_us: 12_000,
    airtime_permille: 1000, cell: None, ge: GE_WIRED, verify_us: 60, sign_us: 25, bg_up: BG_UP_HOME, bg_down: BG_DOWN_HOME, relay: true,
};
/// A phone on the household's cable Wi-Fi (metered = no, but not charging: no relay). MODEL.
pub const PHONE_HOME_WIFI: AccessProfile = AccessProfile {
    name: "phone-home-wifi", up_bps: 10_000_000, down_bps: 150_000_000, up_buffer_ms: 300, down_buffer_ms: 100, last_mile_us: 8_000,
    airtime_permille: 500, cell: None, ge: GE_WIFI, verify_us: 150, sign_us: 60, bg_up: BG_UP_HOME, bg_down: BG_DOWN_HOME, relay: false,
};
/// A phone on LTE: varying capacity, deep per-UE buffers, metered (no relay). MODEL.
pub const PHONE_LTE: AccessProfile = AccessProfile {
    name: "phone-lte", up_bps: 6_000_000, down_bps: 25_000_000, up_buffer_ms: 1_000, down_buffer_ms: 500, last_mile_us: 25_000,
    airtime_permille: 1000, cell: Some(CELL_LTE), ge: GE_LTE, verify_us: 150, sign_us: 60, bg_up: BG_UP_PHONE, bg_down: BG_DOWN_PHONE, relay: false,
};

// ------------------------------------------------------------------ call and scenario

/// The call's media (MODEL; see [`MEDIA_PARAM_SPECS`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CallParams {
    /// Opus bitrate (bit/s); 0 = no audio.
    pub audio_bps: u64,
    /// Opus frame (us).
    pub audio_frame_us: u64,
    /// Video bitrate, both layers (bit/s); 0 = no video.
    pub video_bps: u64,
    /// Frames per second (even: t0 and t1 alternate).
    pub fps: u32,
    /// IDR interval (us).
    pub gop_us: u64,
    /// IDR / t0 P-frame size (milli).
    pub key_ratio_milli: u64,
    /// t1 / t0 P-frame size (permille).
    pub t1_ratio_permille: u64,
    /// Per-frame size jitter (+- permille).
    pub size_jitter_permille: u64,
    /// Capture + encode (us).
    pub capture_encode_us: u64,
    /// Decode + render (us).
    pub decode_render_us: u64,
    /// RNS MTU.
    pub rns_mtu: u32,
    /// IP + UDP header per datagram.
    pub ip_header: u32,
    /// Path MTU.
    pub path_mtu: u32,
    /// Audio frames later than this are missing (us).
    pub audio_deadline_us: u64,
    /// Receivers send layer caps (the host's lever, off by default in edge).
    pub layer_requests: bool,
    /// Members hold a warm standby (edge does; off only to test the bound without one).
    pub standby: bool,
    /// Counterfactual CIRISEdge#975: receivers report delivery, so a forwarder on datagram legs
    /// learns a ceiling from loss its children see. Not in edge v45.
    pub delivery_report: bool,
}

impl Default for CallParams {
    fn default() -> Self {
        Self {
            audio_bps: 24_000,
            audio_frame_us: 20_000,
            video_bps: 500_000,
            fps: 15,
            gop_us: 2_000_000,
            key_ratio_milli: 8_000,
            t1_ratio_permille: 700,
            size_jitter_permille: 200,
            capture_encode_us: 50_000,
            decode_render_us: 30_000,
            rns_mtu: RNS_BASE_MTU,
            ip_header: IP_UDP_HEADER,
            path_mtu: PATH_MTU,
            audio_deadline_us: LATENCY_BUDGET_US,
            layer_requests: false,
            standby: true,
            delivery_report: false,
        }
    }
}

impl CallParams {
    /// Frames per IDR interval.
    #[must_use]
    pub fn gop_frames(&self) -> u32 {
        let g = u32::try_from(self.gop_us * u64::from(self.fps) / 1_000_000).unwrap_or(2).max(2);
        g + g % 2
    }

    /// Mean t0 P-frame size (bytes): solves `key + (G/2 - 1)·S0 + (G/2)·r1·S0 = bitrate·gop/8`.
    #[must_use]
    pub fn p0_bytes(&self) -> u64 {
        let g = u64::from(self.gop_frames());
        let per_gop = self.video_bps * self.gop_us / 8_000_000;
        let denom_milli = self.key_ratio_milli + (g / 2 - 1) * 1_000 + g.div_ceil(2) * self.t1_ratio_permille;
        (per_gop * 1_000 / denom_milli.max(1)).max(1)
    }

    /// The standby's share of a full stream (permille): audio + base layer over audio + video
    /// (edge prices a standby at the measured base-or-audio rate, floored at `MIN_AUDIO_BPS`).
    #[must_use]
    pub fn standby_permille(&self) -> u64 {
        let g = u64::from(self.gop_frames());
        let s0 = self.p0_bytes();
        let t0 = self.key_ratio_milli * s0 / 1_000 + (g / 2 - 1) * s0;
        let t1 = g.div_ceil(2) * self.t1_ratio_permille * s0 / 1_000;
        let audio = self.audio_bps * self.gop_us / 8_000_000;
        let full = (t0 + t1 + audio).max(MIN_STREAM_BPS * self.gop_us / 8_000_000);
        let base = (t0 + audio).max(MIN_AUDIO_BPS * self.gop_us / 8_000_000);
        (base * 1_000 / full.max(1)).min(1_000)
    }
}

/// One participant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeSpec {
    /// Access.
    pub profile: AccessProfile,
    /// Publishes a camera stream.
    pub publishes: bool,
    /// Receives the other streams.
    pub receives: bool,
    /// Offers to relay (and the profile allows it).
    pub relay: bool,
    /// Joins at (us).
    pub join_us: u64,
    /// Leaves at (us), if it does.
    pub leave_us: Option<u64>,
    /// Leaves with a signed end (links closed) rather than vanishing.
    pub graceful: bool,
    /// Caps every stream at the base layer (sent only with `layer_requests`).
    pub cap_t0: bool,
    /// Region (core delay).
    pub region: u8,
    /// A publisher cannot reach this node directly (hard NAT both sides; the NAT package owns
    /// the distribution): it must take every stream through a relay.
    pub unreachable: bool,
    /// A household upload of this many bytes runs from the start when contention is on.
    pub forced_upload_bytes: u64,
}

impl NodeSpec {
    /// A participant on `profile` who publishes and receives, joining at `join_us`.
    #[must_use]
    pub const fn member(profile: AccessProfile, join_us: u64) -> Self {
        Self {
            profile,
            publishes: true,
            receives: true,
            relay: profile.relay,
            join_us,
            leave_us: None,
            graceful: true,
            cap_t0: false,
            region: 0,
            unreachable: false,
            forced_upload_bytes: 0,
        }
    }
}

/// A room and how it runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scenario {
    /// Name.
    pub name: String,
    /// Participants.
    pub nodes: Vec<NodeSpec>,
    /// Run length (us).
    pub duration_us: u64,
    /// Media.
    pub call: CallParams,
    /// Household traffic on (the contention switch).
    pub contention: bool,
    /// Access-link queue discipline.
    pub disc: Discipline,
    /// Medium loss and cellular variation on (off = ideal links, for the exact checks).
    pub impairments: bool,
    /// Seed.
    pub seed: u64,
}

// ------------------------------------------------------------------ engine state

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Ev {
    Capture { s: u16, video: bool },
    Send { s: u16, seq: u32 },
    TxDone { n: u16, down: bool },
    Prop { p: u32 },
    Cpu { n: u16, p: u32 },
    Reorder { n: u16, s: u16 },
    Window,
    Join { n: u16 },
    Ready { n: u16, s: u16 },
    Ask { n: u16, s: u16, gen: u32 },
    Granted { n: u16, s: u16, parent: u16, standby: bool, gen: u32 },
    Leave { n: u16 },
    Detect { n: u16, s: u16, parent: u16 },
    Cell { n: u16, down: bool },
    BgArrive { n: u16, down: bool, periodic: bool },
    BgAck { f: u32, pay: u32 },
    BgLoss { f: u32, pay: u32 },
}

#[derive(Debug, Clone, Copy)]
enum Pk {
    Media { s: u16, seq: u32 },
    Bg { f: u32, pay: u32 },
}

#[derive(Debug, Clone, Copy)]
struct Pkt {
    kind: Pk,
    bytes: u32,
    from: u16,
    to: u16,
}

#[derive(Debug, Clone, Copy)]
struct Chunk {
    audio: bool,
    idx: u32,
    t1: bool,
    plain: u32,
    capture_us: u64,
}

#[derive(Debug, Clone, Default)]
struct Stream {
    chunks: Vec<Chunk>,
    vframes: Vec<VMeta>,
    aframes: Vec<u64>,
    start_us: u64,
    ended_us: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Kid {
    n: u16,
    standby: bool,
    order: u64,
}

#[derive(Debug, Clone, Default)]
struct Adm {
    probe: u32,
    floor: u64,
    streak: u32,
    demand: bool,
    kids: Vec<Kid>,
    order: u64,
    ceiling: Option<u64>,
    bad_windows: u32,
    ceiling_set_us: u64,
}

#[derive(Debug, Clone, Default)]
struct Rx {
    primary: Option<u16>,
    standby: Option<u16>,
    pending: bool,
    sb_pending: bool,
    gen: u32,
    failures: u32,
    last_switch_us: u64,
    switches: VecDeque<u64>,
    degraded_streak: u32,
    left_pub_us: Option<u64>,
    win_sent: u64,
    win_recv: u64,
    win_lat_sum: u64,
    win_lat_n: u64,
    ready: bool,
    seen: Vec<u64>,
    sent: Vec<u64>,
    next: Option<u32>,
    started: bool,
    buf: BTreeMap<u32, u64>,
    released_at: VecDeque<(u32, u64)>,
    gap_ewma: u64,
    tick_at: u64,
    cur_frame: Option<(u32, u16)>,
    last_t0_ok: Option<u32>,
    log: SessionLog,
    skipped: u64,
    late: u64,
    dups: u64,
}

fn bit(v: &[u64], seq: u32) -> bool {
    v.get(seq as usize / 64).is_some_and(|w| w >> (seq % 64) & 1 == 1)
}

fn set_bit(v: &mut Vec<u64>, seq: u32) {
    let i = seq as usize / 64;
    if v.len() <= i {
        v.resize(i + 1, 0);
    }
    v[i] |= 1 << (seq % 64);
}

impl Rx {
    fn seen(&self, seq: u32) -> bool {
        bit(&self.seen, seq)
    }
    fn mark_seen(&mut self, seq: u32) {
        set_bit(&mut self.seen, seq);
    }
    fn window_us(&self) -> u64 {
        (self.gap_ewma * 2).clamp(REORDER_WINDOW_MIN_US, REORDER_WINDOW_MAX_US)
    }
    fn observe_gap(&mut self, gap: u64) {
        self.gap_ewma = if gap > self.gap_ewma { (self.gap_ewma + gap * 3) / 4 } else { (self.gap_ewma * 3 + gap) / 4 };
    }
}

fn set_at<T: Clone + Default>(v: &mut Vec<T>, i: usize, x: T) {
    if v.len() <= i {
        v.resize(i + 1, T::default());
    }
    v[i] = x;
}

struct Node {
    spec: NodeSpec,
    up: LinkDir,
    down: LinkDir,
    rng: Rng,
    cpu_free: u64,
    cpu_busy_us: u64,
    present: bool,
    left_us: Option<u64>,
}

/// Where every media/background packet ended (a packet-level conservation ledger).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Ledger {
    /// Packets created.
    pub created: u64,
    /// Media copies verified at a node.
    pub verified: u64,
    /// Background segments acknowledged or found lost.
    pub bg_done: u64,
    /// Lost on a medium.
    pub air: u64,
    /// Tail-dropped.
    pub tail: u64,
    /// Dropped by the AQM.
    pub aqm: u64,
    /// Discarded when their node left.
    pub flushed: u64,
    /// Reached a node that had left.
    pub absent: u64,
    /// Dropped by age before verification.
    pub stale: u64,
    /// Still alive when the run ended.
    pub live: u64,
}

impl Ledger {
    /// Every created packet is accounted once.
    #[must_use]
    pub fn balanced(&self) -> bool {
        self.created == self.verified + self.bg_done + self.air + self.tail + self.aqm + self.flushed + self.absent + self.stale + self.live
    }
}

/// One run's outcome.
#[derive(Debug, Clone)]
pub struct RunResult {
    /// Scenario name.
    pub name: String,
    /// Per-session experience.
    pub sessions: Vec<SessionQoe>,
    /// Relay departures.
    pub recoveries: Vec<RecoveryObs>,
    /// Frame loss by fragment count against the closed form.
    pub amp: Vec<AmpBucket>,
    /// Packet ledger.
    pub ledger: Ledger,
    /// Every link direction conserved bytes.
    pub links_conserve: bool,
    /// Per node: (profile, uplink stats, downlink stats, CPU busy permille).
    pub links: Vec<(&'static str, LinkStats, LinkStats, u64)>,
    /// Voluntary parent switches.
    pub switches: u64,
    /// Final fan-out of each publisher (children of its own stream).
    pub publisher_fanout: Vec<u32>,
    /// Media copies sent per publisher uplink (bytes).
    pub media_up_bytes: Vec<u64>,
}

/// The engine.
pub struct Engine {
    sc: Scenario,
    now: u64,
    seq: u64,
    heap: BinaryHeap<Reverse<(u64, u64, Ev)>>,
    nodes: Vec<Node>,
    streams: Vec<Stream>,
    rx: Vec<Rx>,
    adm: Vec<Adm>,
    pkts: Vec<Option<Pkt>>,
    free: Vec<u32>,
    flows: Vec<RenoFlow>,
    rng: Rng,
    ledger: Ledger,
    recoveries: Vec<RecoveryRec>,
    aqm_buf: Vec<Queued>,
    switches: u64,
    media_up_bytes: Vec<u64>,
}

impl Engine {
    /// Build the engine for `sc`.
    #[must_use]
    pub fn new(sc: Scenario) -> Self {
        let n = sc.nodes.len();
        let rng = Rng::new(sc.seed ^ 0x3E_D1A);
        let nodes = sc
            .nodes
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let p = s.profile;
                let imp = sc.impairments;
                let cap = |bps| CapacityModel { nominal_bps: bps, airtime_permille: if imp { p.airtime_permille } else { 1000 }, cell: if imp { p.cell } else { None } };
                let ge = if imp { p.ge } else { GeParams::NONE };
                let mut up = LinkDir::new(cap(p.up_bps), p.up_buffer_ms, sc.disc, ge);
                let mut down = LinkDir::new(cap(p.down_bps), p.down_buffer_ms, sc.disc, ge);
                for l in [&mut up, &mut down] {
                    l.path_mtu = u64::from(sc.call.path_mtu);
                    l.ip_header = u64::from(sc.call.ip_header);
                }
                Node { spec: *s, up, down, rng: Rng::new(sc.seed ^ (i as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15)), cpu_free: 0, cpu_busy_us: 0, present: false, left_us: None }
            })
            .collect();
        let rx = (0..n * n)
            .map(|k| {
                let mut r = Rx { gap_ewma: REORDER_GAP_PRIOR_US, ..Rx::default() };
                r.log.stream = (k / n) as u16;
                r.log.receiver = (k % n) as u16;
                r
            })
            .collect();
        let adm = (0..n * n)
            .map(|k| {
                let (s, f) = (k / n, k % n);
                let probe = if s == f { (n.saturating_sub(1) as u32).clamp(1, PUBLISHER_INITIAL_FANOUT) } else { 1 };
                Adm { probe, ..Adm::default() }
            })
            .collect();
        Self {
            now: 0,
            seq: 0,
            heap: BinaryHeap::new(),
            nodes,
            streams: vec![Stream::default(); n],
            rx,
            adm,
            pkts: Vec::new(),
            free: Vec::new(),
            flows: Vec::new(),
            rng,
            ledger: Ledger::default(),
            recoveries: Vec::new(),
            aqm_buf: Vec::new(),
            switches: 0,
            media_up_bytes: vec![0; n],
            sc,
        }
    }

    fn n(&self) -> usize {
        self.nodes.len()
    }
    fn ix(&self, s: u16, n: u16) -> usize {
        usize::from(s) * self.n() + usize::from(n)
    }
    fn push(&mut self, t: u64, e: Ev) {
        self.seq += 1;
        self.heap.push(Reverse((t, self.seq, e)));
    }
    fn present(&self, n: u16) -> bool {
        self.nodes[usize::from(n)].present
    }

    /// One-way propagation between two participants (us).
    #[must_use]
    pub fn prop_us(&self, a: u16, b: u16) -> u64 {
        let (x, y) = (&self.nodes[usize::from(a)].spec, &self.nodes[usize::from(b)].spec);
        let core = if x.region == y.region { CORE_SAME_REGION_US } else { CORE_CROSS_REGION_US };
        x.profile.last_mile_us + y.profile.last_mile_us + core
    }
    fn rtt_us(&self, a: u16, b: u16) -> u64 {
        2 * self.prop_us(a, b)
    }
    fn server_rtt_us(&self, n: u16) -> u64 {
        2 * (self.nodes[usize::from(n)].spec.profile.last_mile_us + SERVER_CORE_US)
    }

    // -------------------------------------------------------------- packets

    fn alloc(&mut self, p: Pkt) -> u32 {
        self.ledger.created += 1;
        if let Some(i) = self.free.pop() {
            self.pkts[i as usize] = Some(p);
            i
        } else {
            self.pkts.push(Some(p));
            (self.pkts.len() - 1) as u32
        }
    }
    fn take(&mut self, id: u32) -> Pkt {
        let p = self.pkts[id as usize].take().expect("live packet");
        self.free.push(id);
        p
    }

    fn enqueue_link(&mut self, n: u16, down: bool, id: u32) {
        let bytes = self.pkts[id as usize].expect("live").bytes;
        let now = self.now;
        let node = &mut self.nodes[usize::from(n)];
        let link = if down { &mut node.down } else { &mut node.up };
        match link.enqueue(id, bytes, now) {
            Enq::Started(t) => self.push(t, Ev::TxDone { n, down }),
            Enq::Queued => {}
            Enq::Dropped => {
                self.ledger.tail += 1;
                self.lost_packet(id);
            }
        }
    }

    /// A packet died somewhere other than the medium's own accounting: free it, and tell a
    /// background flow it lost a segment.
    fn lost_packet(&mut self, id: u32) {
        let p = self.take(id);
        if let Pk::Bg { f, pay } = p.kind {
            let t = self.now + self.server_rtt_us(p.to);
            self.push(t, Ev::BgLoss { f, pay });
        }
    }

    // -------------------------------------------------------------- run

    /// Run to the scenario's end and score it.
    #[must_use]
    pub fn run(self) -> RunResult {
        self.run_logged().0
    }

    /// Run, and also return every (stream, receiver) log (indexed `stream * n + receiver`) and
    /// each stream's frames.
    #[must_use]
    pub fn run_logged(mut self) -> (RunResult, Vec<SessionLog>, Vec<Vec<VMeta>>, Vec<Vec<u64>>) {
        for i in 0..self.n() {
            let s = self.nodes[i].spec;
            self.push(s.join_us, Ev::Join { n: i as u16 });
            if let Some(l) = s.leave_us {
                self.push(l, Ev::Leave { n: i as u16 });
            }
        }
        self.push(METER_WINDOW_US, Ev::Window);
        let end = self.sc.duration_us;
        while let Some(Reverse((t, _, e))) = self.heap.pop() {
            if t > end {
                break;
            }
            self.now = t;
            self.handle(e);
        }
        self.ledger.live = self.pkts.iter().filter(|p| p.is_some()).count() as u64;
        self.finish_logged()
    }

    fn handle(&mut self, e: Ev) {
        match e {
            Ev::Join { n } => self.on_join(n),
            Ev::Leave { n } => self.on_leave(n),
            Ev::Ready { n, s } => self.on_ready(n, s),
            Ev::Ask { n, s, gen } => {
                let r = &self.rx[self.ix(s, n)];
                if r.gen == gen && r.primary.is_none() && !r.pending && self.present(n) && self.present(s) {
                    self.attach_primary(n, s);
                }
            }
            Ev::Granted { n, s, parent, standby, gen } => self.on_granted(n, s, parent, standby, gen),
            Ev::Detect { n, s, parent } => self.on_detect(n, s, parent),
            Ev::Window => self.on_window(),
            Ev::Capture { s, video } => self.on_capture(s, video),
            Ev::Send { s, seq } => {
                if self.present(s) {
                    let kids = self.adm[self.ix(s, s)].kids.clone();
                    for k in kids {
                        self.forward(s, seq, s, k);
                    }
                }
            }
            Ev::TxDone { n, down } => self.on_tx_done(n, down),
            Ev::Prop { p } => {
                let to = self.pkts[p as usize].expect("live").to;
                if self.present(to) {
                    self.enqueue_link(to, true, p);
                } else {
                    self.ledger.absent += 1;
                    self.take(p);
                }
            }
            Ev::Cpu { n, p } => self.on_cpu(n, p),
            Ev::Reorder { n, s } => {
                let i = self.ix(s, n);
                if self.rx[i].tick_at <= self.now {
                    self.rx[i].tick_at = 0;
                }
                self.drain(s, n);
            }
            Ev::Cell { n, down } => {
                let node = &mut self.nodes[usize::from(n)];
                let link = if down { &mut node.down } else { &mut node.up };
                link.cell_step(&mut node.rng);
                if let Some(c) = link.cap.cell {
                    self.push(self.now + c.period_us, Ev::Cell { n, down });
                }
            }
            Ev::BgArrive { n, down, periodic } => self.on_bg_arrive(n, down, periodic),
            Ev::BgAck { f, pay } => {
                let fl = &self.flows[f as usize];
                if self.present(fl.node) {
                    self.flows[f as usize].on_ack(pay, self.now);
                    self.bg_pump(f);
                }
            }
            Ev::BgLoss { f, pay } => {
                let fl = &self.flows[f as usize];
                let node = &self.nodes[usize::from(fl.node)];
                if node.present {
                    let link = if fl.down { &node.down } else { &node.up };
                    let qd = link.backlog_bytes() * 8 * 1_000_000 / link.rate_bps().max(1);
                    let rtt = self.server_rtt_us(fl.node) + qd;
                    self.flows[f as usize].on_loss(pay, self.now, rtt);
                    self.bg_pump(f);
                }
            }
        }
    }

    // -------------------------------------------------------------- membership

    fn on_join(&mut self, n: u16) {
        let ni = usize::from(n);
        self.nodes[ni].present = true;
        let spec = self.nodes[ni].spec;
        if spec.publishes {
            self.streams[ni].start_us = self.now;
            self.push(self.now, Ev::Capture { s: n, video: true });
            if self.sc.call.audio_bps > 0 {
                self.push(self.now, Ev::Capture { s: n, video: false });
            }
        }
        for m in 0..self.n() as u16 {
            if m == n || !self.present(m) {
                continue;
            }
            let setup = SETUP_RTTS * self.rtt_us(n, m);
            if spec.publishes && self.nodes[usize::from(m)].spec.receives {
                self.push(self.now + setup, Ev::Ready { n: m, s: n });
            }
            if spec.receives && self.nodes[usize::from(m)].spec.publishes && self.streams[usize::from(m)].ended_us.is_none() {
                self.push(self.now + setup, Ev::Ready { n, s: m });
            }
        }
        if self.sc.contention {
            let p = spec.profile;
            for (down, model) in [(false, p.bg_up), (true, p.bg_down)] {
                let gap = model.next_gap_us(&mut self.nodes[ni].rng);
                if let Some(g) = gap {
                    self.push(self.now + g, Ev::BgArrive { n, down, periodic: false });
                }
                if model.periodic_bytes > 0 && model.period_ms > 0 {
                    let phase = self.nodes[ni].rng.next_u64() % (model.period_ms * 1_000);
                    self.push(self.now + phase, Ev::BgArrive { n, down, periodic: true });
                }
            }
            if spec.forced_upload_bytes > 0 {
                let f = self.flows.len() as u32;
                self.flows.push(RenoFlow::new(n, false, spec.forced_upload_bytes, self.now));
                self.bg_pump(f);
            }
        }
        if self.sc.impairments {
            for down in [false, true] {
                let link = if down { &self.nodes[ni].down } else { &self.nodes[ni].up };
                if let Some(c) = link.cap.cell {
                    self.push(self.now + c.period_us, Ev::Cell { n, down });
                }
            }
        }
    }

    fn on_ready(&mut self, n: u16, s: u16) {
        if !self.present(n) || !self.present(s) {
            return;
        }
        let i = self.ix(s, n);
        let start = self.nodes[usize::from(n)].spec.join_us.max(self.streams[usize::from(s)].start_us);
        let r = &mut self.rx[i];
        r.ready = true;
        r.log.start_us = start;
        r.log.requested_t0 = self.sc.call.layer_requests && self.nodes[usize::from(n)].spec.cap_t0;
        let gen = r.gen;
        let jitter = self.rng.next_u64() % JOIN_JITTER_US;
        self.push(self.now + jitter, Ev::Ask { n, s, gen });
    }

    fn on_leave(&mut self, n: u16) {
        let ni = usize::from(n);
        if !self.nodes[ni].present {
            return;
        }
        let graceful = self.nodes[ni].spec.graceful;
        // Who loses a parent, and when they find out (computed before anything changes).
        let mut detects = Vec::new();
        for s in 0..self.n() as u16 {
            if s == n {
                continue;
            }
            let depth_n = self.depth(s, n);
            for k in self.adm[self.ix(s, n)].kids.clone() {
                let r = &self.rx[self.ix(s, k.n)];
                let other_path = r.primary.is_some_and(|p| p != n) || r.standby.is_some_and(|p| p != n);
                let tick = self.rng.next_u64() % WATCH_TICK_US;
                let delay = if graceful {
                    tick
                } else if other_path {
                    SUSPECT_SILENCE_US * (2 + u64::from(depth_n.min(4))) / 2 + tick
                } else {
                    let rtt = self.rtt_us(n, k.n);
                    let ka = (rtt * (LINK_KEEPALIVE_MAX_US / 1_000_000) * 1_000_000 / LINK_KEEPALIVE_MAX_RTT_US).clamp(LINK_KEEPALIVE_MIN_US, LINK_KEEPALIVE_MAX_US);
                    LINK_STALE_FACTOR * ka + LINK_STALE_GRACE_US + tick
                };
                detects.push((k.n, s, delay));
                if !k.standby && r.primary == Some(n) {
                    self.recoveries.push(RecoveryRec {
                        child: k.n,
                        stream: s,
                        leave_us: self.now,
                        had_standby: r.standby.is_some_and(|p| p != n && self.nodes[usize::from(p)].present),
                        graceful,
                    });
                }
            }
        }
        self.nodes[ni].present = false;
        self.nodes[ni].left_us = Some(self.now);
        for down in [false, true] {
            let link = if down { &mut self.nodes[ni].down } else { &mut self.nodes[ni].up };
            let gone = link.flush();
            for q in gone {
                self.ledger.flushed += 1;
                self.take(q.id);
            }
        }
        if self.nodes[ni].spec.publishes {
            self.streams[ni].ended_us = Some(self.now);
            let i = self.ix(n, n);
            self.adm[i].kids.clear();
        }
        for s in 0..self.n() as u16 {
            // Parents stop sending to it; it stops forwarding.
            for f in 0..self.n() as u16 {
                let i = self.ix(s, f);
                self.adm[i].kids.retain(|k| k.n != n);
            }
            let i = self.ix(s, n);
            self.adm[i].kids.clear();
            let r = &mut self.rx[i];
            r.primary = None;
            r.standby = None;
            r.gen += 1;
        }
        for (k, s, d) in detects {
            self.push(self.now + d, Ev::Detect { n: k, s, parent: n });
        }
    }

    // -------------------------------------------------------------- the tree

    /// Depth of `n` in stream `s`'s tree along primaries (publisher = 0; 255 = not rooted).
    fn depth(&self, s: u16, n: u16) -> u8 {
        let mut d = 0u8;
        let mut cur = n;
        while cur != s {
            match self.rx[self.ix(s, cur)].primary {
                Some(p) => {
                    cur = p;
                    d += 1;
                    if d > MAX_DEPTH + 2 {
                        return 255;
                    }
                }
                None => return 255,
            }
        }
        d
    }

    fn is_ancestor(&self, s: u16, anc: u16, n: u16) -> bool {
        let mut cur = n;
        for _ in 0..=MAX_DEPTH + 2 {
            if cur == s {
                return false;
            }
            match self.rx[self.ix(s, cur)].primary {
                Some(p) if p == anc => return true,
                Some(p) => cur = p,
                None => return false,
            }
        }
        false
    }

    fn committed(&self, s: u16, f: u16) -> u64 {
        let sp = self.sc.call.standby_permille();
        self.adm[self.ix(s, f)].kids.iter().map(|k| if k.standby { sp } else { 1_000 }).sum()
    }

    fn limit(&self, s: u16, f: u16) -> u64 {
        let a = &self.adm[self.ix(s, f)];
        let l = a.ceiling.unwrap_or(a.floor + u64::from(a.probe) * 1_000);
        if s == f {
            l.max(1_000)
        } else {
            l
        }
    }

    /// Ask forwarder `f` to admit `n` for stream `s`. `freeing` is a commitment the request
    /// replaces (a standby promoted to primary).
    fn admit(&mut self, s: u16, f: u16, n: u16, standby: bool, freeing: u64) -> bool {
        let cost = if standby { self.sc.call.standby_permille() } else { 1_000 };
        if self.committed(s, f).saturating_sub(freeing) + cost <= self.limit(s, f) {
            let i = self.ix(s, f);
            let a = &mut self.adm[i];
            a.order += 1;
            let order = a.order;
            a.kids.push(Kid { n, standby, order });
            true
        } else {
            let i = self.ix(s, f);
            self.adm[i].demand = true;
            false
        }
    }

    fn remove_kid(&mut self, s: u16, f: u16, n: u16, standby: bool) {
        let i = self.ix(s, f);
        if let Some(pos) = self.adm[i].kids.iter().position(|k| k.n == n && k.standby == standby) {
            self.adm[i].kids.remove(pos);
        }
    }

    /// Candidate parents of `n` for `s`, best first: the publisher when it can reach `n`
    /// (direct first, `FSD/CIRIS_MESH.md` R1), then rooted member relays by depth, round
    /// trip, id.
    fn candidates(&self, n: u16, s: u16, for_standby: bool) -> Vec<u16> {
        let r = &self.rx[self.ix(s, n)];
        let mut out = Vec::new();
        if !self.nodes[usize::from(n)].spec.unreachable && self.present(s) && r.primary != Some(s) && r.standby != Some(s) {
            out.push(s);
        }
        let mut relays: Vec<(u8, u64, u16)> = Vec::new();
        for c in 0..self.n() as u16 {
            if c == n || c == s || !self.present(c) || !self.nodes[usize::from(c)].spec.relay || !self.nodes[usize::from(c)].spec.receives {
                continue;
            }
            if r.primary == Some(c) || r.standby == Some(c) {
                continue;
            }
            let d = self.depth(s, c);
            if d == 255 || d + 1 > MAX_DEPTH || self.is_ancestor(s, n, c) {
                continue;
            }
            if for_standby {
                if let Some(p) = r.primary {
                    if p != s && (c == p || self.is_ancestor(s, p, c)) {
                        continue;
                    }
                }
            }
            relays.push((d, self.rtt_us(n, c), c));
        }
        relays.sort_unstable();
        out.extend(relays.into_iter().map(|x| x.2));
        out
    }

    fn attach_primary(&mut self, n: u16, s: u16) {
        for c in self.candidates(n, s, false) {
            if self.admit(s, c, n, false, 0) {
                let i = self.ix(s, n);
                self.rx[i].pending = true;
                let gen = self.rx[i].gen;
                self.push(self.now + self.rtt_us(n, c), Ev::Granted { n, s, parent: c, standby: false, gen });
                return;
            }
        }
        let i = self.ix(s, n);
        let r = &mut self.rx[i];
        r.failures += 1;
        let k = r.failures.min(5);
        let gen = r.gen;
        let base = (BACKOFF_BASE_US << k).min(BACKOFF_MAX_US);
        let u = 500 + self.rng.next_u64() % 1_000;
        self.push(self.now + base * u / 1_000, Ev::Ask { n, s, gen });
    }

    fn attach_standby(&mut self, n: u16, s: u16) {
        let i = self.ix(s, n);
        if !self.sc.call.standby || self.rx[i].sb_pending || self.rx[i].standby.is_some() {
            return;
        }
        for c in self.candidates(n, s, true) {
            if self.admit(s, c, n, true, 0) {
                self.rx[i].sb_pending = true;
                let gen = self.rx[i].gen;
                self.push(self.now + self.rtt_us(n, c), Ev::Granted { n, s, parent: c, standby: true, gen });
                return;
            }
        }
    }

    fn on_granted(&mut self, n: u16, s: u16, parent: u16, standby: bool, gen: u32) {
        let i = self.ix(s, n);
        if standby {
            self.rx[i].sb_pending = false;
        }
        if self.rx[i].gen != gen || !self.present(n) || !self.present(parent) {
            self.remove_kid(s, parent, n, standby);
            if !standby {
                self.rx[i].pending = false;
                if self.present(n) && self.rx[i].primary.is_none() {
                    self.attach_primary(n, s);
                }
            }
            return;
        }
        if standby {
            if self.rx[i].standby.is_some() || self.rx[i].primary == Some(parent) {
                self.remove_kid(s, parent, n, true);
            } else {
                self.rx[i].standby = Some(parent);
            }
            return;
        }
        let old = self.rx[i].primary;
        {
            let r = &mut self.rx[i];
            r.pending = false;
            r.primary = Some(parent);
            r.failures = 0;
        }
        if let Some(o) = old {
            if o != parent {
                self.remove_kid(s, o, n, false);
            }
        }
        if self.rx[i].standby == Some(parent) {
            self.remove_kid(s, parent, n, true);
            self.rx[i].standby = None;
        }
        if self.rx[i].standby.is_none() {
            self.attach_standby(n, s);
        }
    }

    fn on_detect(&mut self, n: u16, s: u16, parent: u16) {
        if !self.present(n) {
            return;
        }
        let i = self.ix(s, n);
        if self.rx[i].standby == Some(parent) {
            self.rx[i].standby = None;
            self.attach_standby(n, s);
        }
        if self.rx[i].primary != Some(parent) {
            return;
        }
        self.rx[i].primary = None;
        self.rx[i].gen += 1;
        self.rx[i].pending = false;
        if !self.present(s) {
            return;
        }
        // Failover: the warm standby first (selector.rs step 4).
        if let Some(sb) = self.rx[i].standby {
            if self.present(sb) && self.admit(s, sb, n, false, self.sc.call.standby_permille()) {
                self.rx[i].pending = true;
                let gen = self.rx[i].gen;
                self.push(self.now + self.rtt_us(n, sb), Ev::Granted { n, s, parent: sb, standby: false, gen });
                return;
            }
        }
        self.attach_primary(n, s);
    }

    fn on_window(&mut self) {
        let n = self.n() as u16;
        let now = self.now;
        let dr = self.sc.call.delivery_report;
        for s in 0..n {
            if !self.present(s) || !self.nodes[usize::from(s)].spec.publishes {
                continue;
            }
            for f in 0..n {
                if !self.present(f) {
                    continue;
                }
                let committed = self.committed(s, f);
                let limit = self.limit(s, f);
                // Children's loss on this forwarder this window (what #975 would report).
                let (mut sent, mut recv) = (0u64, 0u64);
                for k in &self.adm[self.ix(s, f)].kids {
                    let r = &self.rx[self.ix(s, k.n)];
                    if r.primary == Some(f) {
                        sent += r.win_sent;
                        recv += r.win_recv.min(r.win_sent);
                    }
                }
                let loss_pm = if sent == 0 { 0 } else { (sent - recv) * 1_000 / sent };
                let i = self.ix(s, f);
                let a = &mut self.adm[i];
                let mut shed_to = None;
                if dr && loss_pm >= DEGRADED_LOSS_PERMILLE {
                    a.bad_windows += 1;
                    a.streak = 0;
                    if a.bad_windows >= UPLINK_REFUSAL_WINDOWS {
                        let c = (committed * (1_000 - loss_pm) / 1_000 * UPLINK_SAFETY_MARGIN_PERMILLE / 1_000).max(if s == f { 1_000 } else { 0 });
                        a.ceiling = Some(c);
                        a.ceiling_set_us = now;
                        a.bad_windows = 0;
                        shed_to = Some(c);
                    }
                } else {
                    a.bad_windows = 0;
                    a.streak += 1;
                    if a.ceiling.is_some() && now.saturating_sub(a.ceiling_set_us) >= CEILING_STALE_US {
                        a.ceiling = None;
                    }
                    if a.ceiling.is_none() {
                        // On a datagram leg over UDP nothing is ever refused locally, so every
                        // window is clean (path_meter.rs "Limit: datagram legs").
                        let used = committed * 1_000 >= limit * PROBE_USED_PERMILLE;
                        if (used || a.demand) && a.streak >= PROBE_CLEAN_WINDOWS {
                            a.probe = (a.probe * 2).min(MAX_PROBE_STREAMS);
                            a.streak = 0;
                            a.demand = false;
                        }
                        a.floor = a.floor.max(committed);
                    }
                }
                if let Some(c) = shed_to {
                    self.shed_lifo(s, f, c);
                }
            }
        }
        // Receivers: degraded parents, voluntary moves, the direct path's return.
        for s in 0..n {
            if !self.present(s) {
                continue;
            }
            for m in 0..n {
                if m == s || !self.present(m) {
                    continue;
                }
                let i = self.ix(s, m);
                let Some(p) = self.rx[i].primary else {
                    self.rx[i].win_sent = 0;
                    self.rx[i].win_recv = 0;
                    self.rx[i].win_lat_sum = 0;
                    self.rx[i].win_lat_n = 0;
                    continue;
                };
                let r = &mut self.rx[i];
                let loss_pm = if r.win_sent == 0 { 0 } else { r.win_sent.saturating_sub(r.win_recv) * 1_000 / r.win_sent };
                let lat = if r.win_lat_n == 0 { 0 } else { r.win_lat_sum / r.win_lat_n };
                let degraded = loss_pm >= DEGRADED_LOSS_PERMILLE || lat > LATENCY_BUDGET_US;
                r.degraded_streak = if degraded { r.degraded_streak + 1 } else { 0 };
                r.win_sent = 0;
                r.win_recv = 0;
                r.win_lat_sum = 0;
                r.win_lat_n = 0;
                while r.switches.front().is_some_and(|&t| now.saturating_sub(t) > FLAP_WINDOW_US) {
                    r.switches.pop_front();
                }
                let dwell = MIN_DWELL_US << (r.switches.len().min(6) as u32);
                let dwell_ok = now.saturating_sub(r.last_switch_us) >= dwell;
                let blocked = r.pending || !dwell_ok;
                if r.standby.is_none() && !r.sb_pending {
                    self.attach_standby(m, s);
                }
                if blocked {
                    continue;
                }
                let r = &self.rx[i];
                let streak = r.degraded_streak;
                let left_pub = r.left_pub_us;

                if streak >= HYSTERESIS_HOLD_WINDOWS {
                    let cands: Vec<u16> = self.candidates(m, s, false).into_iter().filter(|&c| c != p).collect();
                    for c in cands {
                        if self.admit(s, c, m, false, 0) {
                            self.begin_switch(m, s, c, p == s);
                            break;
                        }
                    }
                } else if p != s
                    && !self.nodes[usize::from(m)].spec.unreachable
                    && left_pub.is_none_or(|t| now.saturating_sub(t) >= FLAP_WINDOW_US)
                    && self.admit(s, s, m, false, 0)
                {
                    self.begin_switch(m, s, s, false);
                }
            }
        }
        self.push(now + METER_WINDOW_US, Ev::Window);
    }

    fn begin_switch(&mut self, m: u16, s: u16, to: u16, leaving_publisher: bool) {
        let i = self.ix(s, m);
        let now = self.now;
        let r = &mut self.rx[i];
        r.pending = true;
        r.last_switch_us = now;
        r.switches.push_back(now);
        r.degraded_streak = 0;
        if leaving_publisher {
            r.left_pub_us = Some(now);
        }
        let gen = r.gen;
        self.switches += 1;
        // Make-before-break: the old parent keeps sending until the new one is granted.
        if self.rx[i].standby == Some(to) {
            self.remove_kid(s, to, m, true);
            self.rx[i].standby = None;
        }
        self.push(now + self.rtt_us(m, to), Ev::Granted { n: m, s, parent: to, standby: false, gen });
    }

    fn shed_lifo(&mut self, s: u16, f: u16, ceiling: u64) {
        while self.committed(s, f) > ceiling {
            let i = self.ix(s, f);
            let Some(pos) = self.adm[i].kids.iter().enumerate().max_by_key(|(_, k)| k.order).map(|(p, _)| p) else { break };
            let k = self.adm[i].kids.remove(pos);
            let j = self.ix(s, k.n);
            if k.standby {
                if self.rx[j].standby == Some(f) {
                    self.rx[j].standby = None;
                }
            } else if self.rx[j].primary == Some(f) {
                self.rx[j].primary = None;
                self.rx[j].gen += 1;
                self.rx[j].pending = false;
                self.attach_primary(k.n, s);
            }
        }
    }

    // -------------------------------------------------------------- media plane

    fn frame_bytes(&mut self, key: bool, t1: bool) -> u32 {
        let c = self.sc.call;
        let s0 = c.p0_bytes();
        let base = if key { s0 * c.key_ratio_milli / 1_000 } else if t1 { s0 * c.t1_ratio_permille / 1_000 } else { s0 };
        let j = c.size_jitter_permille;
        let f = if j == 0 { 1_000 } else { 1_000 - j + self.rng.next_u64() % (2 * j + 1) };
        u32::try_from((base * f / 1_000).max(1)).unwrap_or(MAX_FRAME_BYTES).min(MAX_FRAME_BYTES)
    }

    fn on_capture(&mut self, s: u16, video: bool) {
        let si = usize::from(s);
        if !self.present(s) || self.now >= self.sc.duration_us {
            return;
        }
        let c = self.sc.call;
        let budget = frame_budget(c.rns_mtu);
        let start = self.streams[si].start_us;
        let sign = self.nodes[si].spec.profile.sign_us;
        let mut plains: Vec<(u32, Chunk)> = Vec::new();
        let next_t;
        if video {
            if c.video_bps == 0 {
                return;
            }
            let idx = self.streams[si].vframes.len() as u32;
            let key = idx % c.gop_frames() == 0;
            let t1 = idx % 2 == 1 && !key;
            let bytes = self.frame_bytes(key, t1);
            let nfr = fragments(bytes, budget).expect("frame within edge's limits");
            let per = budget - FRAGMENT_MARKER_LEN;
            for k in 0..nfr {
                let body = if k + 1 < nfr { per } else { bytes - per * (nfr - 1) };
                plains.push((k, Chunk { audio: false, idx, t1, plain: body + FRAGMENT_MARKER_LEN, capture_us: self.now }));
            }
            self.streams[si].vframes.push(VMeta { capture_us: self.now, key, t1, frags: nfr as u16, bytes });
            next_t = start + (u64::from(idx) + 1) * 1_000_000 / u64::from(c.fps);
        } else {
            let idx = self.streams[si].aframes.len() as u32;
            let bytes = u32::try_from((c.audio_bps * c.audio_frame_us / 8_000_000).max(1)).unwrap_or(1);
            plains.push((0, Chunk { audio: true, idx, t1: false, plain: bytes, capture_us: self.now }));
            self.streams[si].aframes.push(self.now);
            next_t = start + (u64::from(idx) + 1) * c.audio_frame_us;
        }
        for (_, ch) in plains {
            let seq = self.streams[si].chunks.len() as u32;
            self.streams[si].chunks.push(ch);
            let node = &mut self.nodes[si];
            let done = node.cpu_free.max(self.now) + sign;
            node.cpu_free = done;
            node.cpu_busy_us += sign;
            self.push(done, Ev::Send { s, seq });
        }
        self.push(next_t, Ev::Capture { s, video });
    }

    fn forward(&mut self, s: u16, seq: u32, from: u16, k: Kid) {
        let ch = self.streams[usize::from(s)].chunks[seq as usize];
        if k.standby && ch.t1 {
            return;
        }
        let i = self.ix(s, k.n);
        if !k.standby && ch.t1 && self.rx[i].log.requested_t0 {
            return;
        }
        if !self.present(k.n) {
            return;
        }
        {
            let r = &mut self.rx[i];
            if r.primary == Some(from) {
                r.win_sent += 1;
            }
            if ch.audio {
                set_at(&mut r.log.aud_sent, ch.idx as usize, true);
            } else {
                set_at(&mut r.log.vid_sent, ch.idx as usize, true);
            }
        }
        let bytes = chunk_rns_bytes(ch.plain) + self.sc.call.ip_header;
        let id = self.alloc(Pkt { kind: Pk::Media { s, seq }, bytes, from, to: k.n });
        self.media_up_bytes[usize::from(from)] += u64::from(bytes);
        self.note_sent_unique(s, k.n, seq);
        self.enqueue_link(from, false, id);
    }

    /// Count a chunk as sent toward `n` once, however many parents send it.
    fn note_sent_unique(&mut self, s: u16, n: u16, seq: u32) {
        let i = self.ix(s, n);
        let r = &mut self.rx[i];
        if !bit(&r.sent, seq) {
            set_bit(&mut r.sent, seq);
            r.log.chunks_sent += 1;
        }
    }

    fn on_tx_done(&mut self, n: u16, down: bool) {
        let now = self.now;
        let ni = usize::from(n);
        let outcome = {
            let node = &mut self.nodes[ni];
            let link = if down { &mut node.down } else { &mut node.up };
            if !link.busy() {
                return;
            }
            link.tx_done(now, &mut node.rng, &mut self.aqm_buf)
        };
        let drops: Vec<Queued> = self.aqm_buf.drain(..).collect();
        for q in drops {
            self.ledger.aqm += 1;
            self.lost_packet(q.id);
        }
        if let Some(t) = outcome.next_done_us {
            self.push(t, Ev::TxDone { n, down });
        }
        let id = outcome.done.id;
        let p = self.pkts[id as usize].expect("live");
        if outcome.lost {
            self.ledger.air += 1;
            self.lost_packet(id);
            return;
        }
        match (p.kind, down) {
            (Pk::Media { .. }, false) => {
                let t = now + self.prop_us(p.from, p.to);
                self.push(t, Ev::Prop { p: id });
            }
            (Pk::Media { .. }, true) => self.cpu_enqueue(n, id),
            (Pk::Bg { f, pay }, false) => {
                self.take(id);
                self.ledger.bg_done += 1;
                let t = now + self.server_rtt_us(n);
                self.push(t, Ev::BgAck { f, pay });
            }
            (Pk::Bg { f, pay }, true) => {
                self.take(id);
                self.ledger.bg_done += 1;
                let t = now + self.server_rtt_us(n) / 2;
                self.push(t, Ev::BgAck { f, pay });
            }
        }
    }

    fn cpu_enqueue(&mut self, n: u16, id: u32) {
        let now = self.now;
        let node = &mut self.nodes[usize::from(n)];
        let start = node.cpu_free.max(now);
        if start - now > MEDIA_STALE_BUDGET_US {
            self.ledger.stale += 1;
            self.take(id);
            return;
        }
        let v = node.spec.profile.verify_us;
        node.cpu_free = start + v;
        node.cpu_busy_us += v;
        let t = node.cpu_free;
        self.push(t, Ev::Cpu { n, p: id });
    }

    fn on_cpu(&mut self, n: u16, id: u32) {
        let p = self.take(id);
        self.ledger.verified += 1;
        if !self.present(n) {
            return;
        }
        let Pk::Media { s, seq } = p.kind else { return };
        let ch = self.streams[usize::from(s)].chunks[seq as usize];
        let i = self.ix(s, n);
        let now = self.now;
        {
            let r = &mut self.rx[i];
            if r.primary == Some(p.from) {
                r.win_recv += 1;
                r.win_lat_sum += now - ch.capture_us;
                r.win_lat_n += 1;
            }
            if r.seen(seq) {
                r.dups += 1;
                let first = r.buf.get(&seq).copied().or_else(|| r.released_at.iter().find(|x| x.0 == seq).map(|x| x.1));
                if let Some(t) = first {
                    r.observe_gap(now - t);
                }
                return;
            }
            r.mark_seen(seq);
            r.log.chunks_recv += 1;
        }
        for k in self.adm[i].kids.clone() {
            self.forward(s, seq, n, k);
        }
        if self.nodes[usize::from(n)].spec.receives {
            let r = &mut self.rx[i];
            if r.next.is_some_and(|nx| seq < nx) {
                r.late += 1;
                return;
            }
            r.buf.insert(seq, now);
            self.drain(s, n);
        }
    }

    /// The reorder window (edge `av_call.rs` `ReorderWindow::drain`).
    fn drain(&mut self, s: u16, n: u16) {
        let i = self.ix(s, n);
        let now = self.now;
        let mut released = Vec::new();
        {
            let r = &mut self.rx[i];
            let next = *r.next.get_or_insert(0);
            let mut next = next;
            loop {
                let Some((&head, &first_seen)) = r.buf.iter().next() else { break };
                if head != next {
                    let deadline = first_seen + r.window_us();
                    if now < deadline {
                        if r.tick_at == 0 || deadline < r.tick_at {
                            r.tick_at = deadline;
                            released.push((u32::MAX, deadline));
                        }
                        break;
                    }
                    if r.started {
                        r.skipped += u64::from(head - next);
                    }
                    next = head;
                    continue;
                }
                r.buf.remove(&head);
                r.started = true;
                r.released_at.push_back((head, first_seen));
                if r.released_at.len() > 512 {
                    r.released_at.pop_front();
                }
                released.push((head, now));
                next = head + 1;
            }
            r.next = Some(next);
        }
        for (seq, t) in released {
            if seq == u32::MAX {
                self.push(t, Ev::Reorder { n, s });
            } else {
                self.release(s, n, seq);
            }
        }
    }

    fn release(&mut self, s: u16, n: u16, seq: u32) {
        let ch = self.streams[usize::from(s)].chunks[seq as usize];
        let now = self.now;
        let i = self.ix(s, n);
        if ch.audio {
            set_at(&mut self.rx[i].log.aud_play, ch.idx as usize, now);
            return;
        }
        let meta = self.streams[usize::from(s)].vframes[ch.idx as usize];
        let depth = match self.depth(s, n) {
            255 => self.rx[self.ix(s, n)].standby.map_or(4, |sb| self.depth(s, sb).saturating_add(1)),
            d => d,
        };
        let dr = self.sc.call.decode_render_us;
        let r = &mut self.rx[i];
        let got = match r.cur_frame {
            Some((f, g)) if f == ch.idx => g + 1,
            _ => 1,
        };
        r.cur_frame = Some((ch.idx, got));
        if got != meta.frags {
            return;
        }
        set_at(&mut r.log.vid_complete, ch.idx as usize, true);
        let f = ch.idx;
        let ok = if meta.key {
            true
        } else if meta.t1 {
            r.last_t0_ok == Some(f - 1)
        } else {
            f >= 2 && r.last_t0_ok == Some(f - 2)
        };
        if ok {
            if !meta.t1 {
                r.last_t0_ok = Some(f);
            }
            set_at(&mut r.log.vid_shown, f as usize, now + dr);
            set_at(&mut r.log.vid_depth, f as usize, depth.clamp(1, 4));
        }
    }

    // -------------------------------------------------------------- household traffic

    fn on_bg_arrive(&mut self, n: u16, down: bool, periodic: bool) {
        if !self.present(n) || self.now >= self.sc.duration_us {
            return;
        }
        let ni = usize::from(n);
        let p = self.nodes[ni].spec.profile;
        let model = if down { p.bg_down } else { p.bg_up };
        let bytes = if periodic { model.periodic_bytes } else { model.draw_bytes(&mut self.nodes[ni].rng) };
        let f = self.flows.len() as u32;
        self.flows.push(RenoFlow::new(n, down, bytes, self.now));
        self.bg_pump(f);
        let next = if periodic { Some(model.period_ms * 1_000) } else { model.next_gap_us(&mut self.nodes[ni].rng) };
        if let Some(g) = next {
            self.push(self.now + g, Ev::BgArrive { n, down, periodic });
        }
    }

    fn bg_pump(&mut self, f: u32) {
        let (n, down) = (self.flows[f as usize].node, self.flows[f as usize].down);
        while let Some(pay) = self.flows[f as usize].next_segment() {
            let bytes = pay + (BG_SEGMENT_WIRE - BG_MSS);
            let id = self.alloc(Pkt { kind: Pk::Bg { f, pay }, bytes, from: n, to: n });
            if down {
                let t = self.now + self.server_rtt_us(n) / 2;
                self.push(t, Ev::Prop { p: id });
            } else {
                self.enqueue_link(n, false, id);
            }
        }
    }

    // -------------------------------------------------------------- scoring

    fn finish_logged(mut self) -> (RunResult, Vec<SessionLog>, Vec<Vec<VMeta>>, Vec<Vec<u64>>) {
        let n = self.n();
        let end = self.sc.duration_us;
        let rules = QoeRules { fps: self.sc.call.fps, capture_encode_us: self.sc.call.capture_encode_us, audio_deadline_us: self.sc.call.audio_deadline_us, tail_us: 1_000_000 };
        let mut amp = amp_buckets();
        let mut sessions = Vec::new();
        let mut recoveries = Vec::new();
        for s in 0..n {
            if !self.nodes[s].spec.publishes || self.streams[s].vframes.is_empty() && self.streams[s].aframes.is_empty() {
                continue;
            }
            for m in 0..n {
                if m == s || !self.nodes[m].spec.receives {
                    continue;
                }
                let i = s * n + m;
                if !self.rx[i].ready {
                    continue;
                }
                let stop = [Some(end), self.nodes[m].left_us, self.streams[s].ended_us].into_iter().flatten().min().unwrap_or(end);
                self.rx[i].log.end_us = stop;
                let log = &self.rx[i].log;
                sessions.push(score_session(log, &self.streams[s].vframes, &self.streams[s].aframes, &rules, &mut amp));
            }
        }
        for rec in &self.recoveries {
            let i = usize::from(rec.stream) * n + usize::from(rec.child);
            recoveries.push(score_recovery(*rec, &self.rx[i].log, &self.streams[usize::from(rec.stream)].vframes));
        }
        let links_conserve = self.nodes.iter().all(|nd| nd.up.conservation_residual() == 0 && nd.down.conservation_residual() == 0);
        let links = self
            .nodes
            .iter()
            .map(|nd| {
                let span = end.saturating_sub(nd.spec.join_us).max(1);
                (nd.spec.profile.name, nd.up.stats, nd.down.stats, nd.cpu_busy_us * 1_000 / span)
            })
            .collect();
        let publisher_fanout = (0..n).map(|s| self.adm[s * n + s].kids.iter().filter(|k| !k.standby).count() as u32).collect();
        let result = RunResult {
            name: self.sc.name.clone(),
            sessions,
            recoveries,
            amp,
            ledger: self.ledger,
            links_conserve,
            links,
            switches: self.switches,
            publisher_fanout,
            media_up_bytes: std::mem::take(&mut self.media_up_bytes),
        };
        let logs = self.rx.iter().map(|r| r.log.clone()).collect();
        let vf = self.streams.iter().map(|st| st.vframes.clone()).collect();
        let af = self.streams.iter().map(|st| st.aframes.clone()).collect();
        (result, logs, vf, af)
    }
}


// ------------------------------------------------------------------ scenarios

/// CoDel at RFC 8289's defaults: 5 ms target, 100 ms interval.
pub const CODEL: Discipline = Discipline::CoDel { target_us: 5_000, interval_us: 100_000 };

fn join_spread(rng: &mut Rng, lo_us: u64, hi_us: u64) -> u64 {
    lo_us + rng.next_u64() % (hi_us - lo_us).max(1)
}

fn base(name: &str, nodes: Vec<NodeSpec>, duration_us: u64, seed: u64, contention: bool, disc: Discipline) -> Scenario {
    Scenario { name: name.to_owned(), nodes, duration_us, call: CallParams::default(), contention, disc, impairments: true, seed }
}

/// One-to-many: a fibre desktop publishes to 24 viewers on mixed access; homes relay.
#[must_use]
pub fn scenario_broadcast(seed: u64, contention: bool, disc: Discipline) -> Scenario {
    let mut rng = Rng::new(seed ^ 0xB0AD);
    let mut nodes = vec![NodeSpec { receives: false, ..NodeSpec::member(FIBER_HOME, 0) }];
    let mix = [CABLE_HOME_WIFI, FIBER_HOME, PHONE_LTE, CABLE_HOME_WIFI, DSL_HOME, PHONE_HOME_WIFI];
    for k in 0..24 {
        let p = mix[k % mix.len()];
        nodes.push(NodeSpec { publishes: false, region: u8::from(k % 4 == 3), ..NodeSpec::member(p, join_spread(&mut rng, 500_000, 5_000_000)) });
    }
    base("broadcast 1->24", nodes, 60_000_000, seed, contention, disc)
}

/// A four-person call: laptop on cable Wi-Fi, fibre desktop, phone on LTE, phone on home Wi-Fi.
#[must_use]
pub fn scenario_call4(seed: u64, contention: bool, disc: Discipline) -> Scenario {
    let mut rng = Rng::new(seed ^ 0xCA11);
    let nodes = [CABLE_HOME_WIFI, FIBER_HOME, PHONE_LTE, PHONE_HOME_WIFI]
        .iter()
        .map(|&p| NodeSpec::member(p, join_spread(&mut rng, 0, 3_000_000)))
        .collect();
    base("call 4", nodes, 60_000_000, seed, contention, disc)
}

/// A twenty-person interactive room in gallery view: every receiver caps every stream at the
/// base layer (the host enables layer requests).
#[must_use]
pub fn scenario_room20(seed: u64, contention: bool, disc: Discipline) -> Scenario {
    let mut rng = Rng::new(seed ^ 0x2020);
    let mix = [CABLE_HOME_WIFI, CABLE_HOME_WIFI, FIBER_HOME, PHONE_HOME_WIFI, PHONE_LTE];
    let nodes = (0..20)
        .map(|k| NodeSpec { cap_t0: true, region: u8::from(k % 5 == 4), ..NodeSpec::member(mix[k % mix.len()], join_spread(&mut rng, 0, 5_000_000)) })
        .collect();
    let mut sc = base("room 20", nodes, 40_000_000, seed, contention, disc);
    sc.call.layer_requests = true;
    sc
}

/// Phones joining and leaving a room held by four homes; two phones sit behind hard NAT and
/// take every stream through a home relay; one relaying home leaves with a signed end at 30 s.
#[must_use]
pub fn scenario_churn(seed: u64, contention: bool, disc: Discipline) -> Scenario {
    let mut rng = Rng::new(seed ^ 0xC4A2);
    let mut nodes: Vec<NodeSpec> = [CABLE_HOME_WIFI, FIBER_HOME, CABLE_HOME_WIFI, DSL_HOME].iter().map(|&p| NodeSpec::member(p, join_spread(&mut rng, 0, 2_000_000))).collect();
    nodes[2].leave_us = Some(30_000_000);
    for k in 0..6 {
        let p = if k % 2 == 0 { PHONE_LTE } else { PHONE_HOME_WIFI };
        let join = join_spread(&mut rng, 2_000_000, 30_000_000);
        let u = rng.f().max(1e-9);
        let stay = ((-u.ln()) * 20_000_000.0) as u64 + 3_000_000;
        nodes.push(NodeSpec {
            leave_us: Some(join + stay),
            graceful: !chance_ppm(&mut rng, PHONE_CRASH_PPM),
            unreachable: k < 2,
            ..NodeSpec::member(p, join)
        });
    }
    base("churn 4+6 phones", nodes, 60_000_000, seed, contention, disc)
}

/// The four-person call with its first member on DSL; with contention on, that household runs
/// a cloud backup (a long upload) through the whole call.
#[must_use]
pub fn scenario_congested(seed: u64, contention: bool, disc: Discipline) -> Scenario {
    let mut sc = scenario_call4(seed, contention, disc);
    sc.name = "call 4, one DSL uplink + backup".to_owned();
    sc.nodes[0].profile = DSL_HOME;
    sc.nodes[0].forced_upload_bytes = 2_000_000_000;
    sc
}

/// A scenario builder.
pub type ScenarioFn = fn(u64, bool, Discipline) -> Scenario;

/// The five traffic shapes.
pub const SCENARIOS: [(&str, ScenarioFn); 5] = [
    ("broadcast", scenario_broadcast),
    ("call4", scenario_call4),
    ("room20", scenario_room20),
    ("churn", scenario_churn),
    ("congested", scenario_congested),
];

/// One way of running a scenario.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Condition {
    /// Label.
    pub label: &'static str,
    /// Household traffic.
    pub contention: bool,
    /// Queue discipline.
    pub disc: Discipline,
    /// Counterfactual CIRISEdge#975.
    pub delivery_report: bool,
}

/// The conditions every scenario runs under: contention off and on is the finding; CoDel and
/// the #975 counterfactual say what each remedy buys.
pub const CONDITIONS: [Condition; 4] = [
    Condition { label: "quiet,fifo", contention: false, disc: Discipline::TailDrop, delivery_report: false },
    Condition { label: "busy,fifo", contention: true, disc: Discipline::TailDrop, delivery_report: false },
    Condition { label: "busy,codel", contention: true, disc: CODEL, delivery_report: false },
    Condition { label: "busy,fifo,#975", contention: true, disc: Discipline::TailDrop, delivery_report: true },
];

// ------------------------------------------------------------------ report

/// Pooled results of one (scenario, condition) over seeds.
#[derive(Debug, Clone, Default)]
pub struct Pooled {
    /// Sessions.
    pub sessions: Vec<SessionQoe>,
    /// Recoveries.
    pub recoveries: Vec<RecoveryObs>,
    /// Amplification buckets.
    pub amp: Vec<AmpBucket>,
    /// Every run balanced its ledger and its links.
    pub conserved: bool,
    /// Voluntary switches.
    pub switches: u64,
    /// Worst uplink queue sojourn among publishers (us), per run.
    pub worst_sojourn_us: Vec<u64>,
    /// Peak CPU busy share among nodes (permille), per run.
    pub peak_cpu_permille: Vec<u64>,
    /// Max publisher fan-out per run.
    pub max_fanout: Vec<u64>,
}

/// Run `f` under `cond` for `seeds` seeds.
#[must_use]
pub fn pool(f: ScenarioFn, cond: Condition, seeds: u64) -> Pooled {
    let mut out = Pooled { conserved: true, amp: amp_buckets(), ..Pooled::default() };
    for seed in 1..=seeds {
        let mut sc = f(seed, cond.contention, cond.disc);
        sc.call.delivery_report = cond.delivery_report;
        let r = Engine::new(sc).run();
        out.conserved &= r.ledger.balanced() && r.links_conserve;
        out.switches += r.switches;
        out.sessions.extend(r.sessions);
        out.recoveries.extend(r.recoveries);
        for (a, b) in out.amp.iter_mut().zip(&r.amp) {
            a.sent += b.sent;
            a.lost += b.lost;
            a.closed_form_sum += b.closed_form_sum;
            a.closed_form_var += b.closed_form_var;
        }
        out.worst_sojourn_us.push(r.links.iter().map(|l| l.1.sojourn_max_us).max().unwrap_or(0));
        out.peak_cpu_permille.push(r.links.iter().map(|l| l.3).max().unwrap_or(0));
        out.max_fanout.push(u64::from(r.publisher_fanout.iter().copied().max().unwrap_or(0)));
    }
    out
}

fn ms(us: u64) -> u64 {
    us / 1_000
}

fn d3(v: &[u64]) -> String {
    let d = crate::qoe::Dist::of(v);
    if d.n == 0 {
        return "-".to_owned();
    }
    format!("{}/{}/{}", d.p50, d.p95, d.p99)
}

fn permille(num: u64, den: u64) -> u64 {
    if den == 0 {
        0
    } else {
        num * 1_000 / den
    }
}

/// One line per (scenario, condition): distributions are p50/p95/p99 over (receiver, stream)
/// sessions pooled across seeds. Every number is MODEL output.
#[must_use]
pub fn report_line(name: &str, cond: &Condition, p: &Pooled) -> String {
    let s = &p.sessions;
    let joined: Vec<u64> = s.iter().filter_map(|q| q.join_us).map(ms).collect();
    let never = s.iter().filter(|q| q.join_us.is_none()).count();
    let g1: Vec<u64> = s.iter().filter_map(|q| q.g2g_median_by_depth[0]).map(ms).collect();
    let g2: Vec<u64> = s.iter().filter_map(|q| q.g2g_median_by_depth[1].or(q.g2g_median_by_depth[2])).map(ms).collect();
    let key: Vec<u64> = s.iter().filter(|q| q.key_sent > 0).map(|q| permille(q.key_lost, q.key_sent)).collect();
    let p0: Vec<u64> = s.iter().filter(|q| q.p0_sent > 0).map(|q| permille(q.p0_lost, q.p0_sent)).collect();
    let frz: Vec<u64> = s.iter().filter(|q| q.span_us > 10_000_000).map(|q| q.freeze_us * 60 / (q.span_us / 1_000).max(1)).collect();
    let aud: Vec<u64> = s.iter().filter(|q| q.audio_sent > 0).map(|q| permille(q.audio_missing, q.audio_sent)).collect();
    let alat: Vec<u64> = s.iter().filter_map(|q| crate::qoe::percentile(&q.audio_lat_samples, 500)).map(ms).collect();
    let layer: Vec<u64> = s.iter().filter(|q| q.layer_windows > 0).map(|q| permille(q.layer_ok_windows, q.layer_windows)).collect();
    let chunk: Vec<u64> = s.iter().map(|q| q.chunk_loss_ppm / 1_000).collect();
    format!(
        "{name:<32} {:<15} n={:<4} noVideo={:<3} join_ms={:<16} g2g_d1_ms={:<14} g2g_d2+_ms={:<14} chunkLoss_pm={:<12} keyLoss_pm={:<14} p0Loss_pm={:<12} freeze_ms/min={:<16} audioMiss_pm={:<12} audioLat_ms={:<12} layerOK_pm={:<12} sw={} sojournMax_ms={} cpuPeak_pm={} fanoutMax={} conserved={}",
        cond.label,
        s.len(),
        never,
        d3(&joined),
        d3(&g1),
        d3(&g2),
        d3(&chunk),
        d3(&key),
        d3(&p0),
        d3(&frz),
        d3(&aud),
        d3(&alat),
        d3(&layer),
        p.switches,
        d3(&p.worst_sojourn_us.iter().map(|&u| ms(u)).collect::<Vec<_>>()),
        d3(&p.peak_cpu_permille),
        d3(&p.max_fanout),
        p.conserved,
    )
}

/// The amplification table of one pool: frames sent, measured loss, closed form.
#[must_use]
pub fn amp_lines(p: &Pooled) -> Vec<String> {
    p.amp
        .iter()
        .filter(|b| b.sent > 0)
        .map(|b| {
            format!(
                "  frags {:>4}-{:<4} frames={:<7} lost_pm={:<5} closed_form_pm={:<5}",
                b.frags_lo,
                b.frags_hi,
                b.sent,
                permille(b.lost, b.sent),
                (b.closed_form_sum * 1_000.0 / b.sent as f64) as u64
            )
        })
        .collect()
}

/// Recovery lines of one pool.
#[must_use]
pub fn recovery_line(p: &Pooled) -> String {
    let (mut a, mut v, mut f, mut lost) = (Vec::new(), Vec::new(), Vec::new(), 0u64);
    for r in &p.recoveries {
        match r.audio_gap_us {
            Some(g) => a.push(ms(g)),
            None => lost += 1,
        }
        if let Some(g) = r.video_gap_us {
            v.push(ms(g));
        }
        if let Some(g) = r.full_restore_us {
            f.push(ms(g));
        }
    }
    let sb = p.recoveries.iter().filter(|r| r.rec.had_standby).count();
    format!(
        "  relay departures={} withStandby={} audioGap_ms={} videoGap_ms={} fullLayer_ms={} neverResumed={}",
        p.recoveries.len(),
        sb,
        d3(&a),
        d3(&v),
        d3(&f),
        lost
    )
}

/// The whole report: every scenario under every condition.
#[must_use]
pub fn report(seeds: u64) -> String {
    let mut out = Vec::new();
    out.push(format!(
        "CEWP media twin (MODEL output; inputs per FSD/twin/media.md): frame budget {} B of a {} B media limit; full chunk {} B on the wire; {} seeds; distributions p50/p95/p99",
        frame_budget(RNS_BASE_MTU),
        media_limit(RNS_BASE_MTU),
        chunk_rns_bytes(frame_budget(RNS_BASE_MTU)) + IP_UDP_HEADER,
        seeds
    ));
    for (name, f) in SCENARIOS {
        for cond in &CONDITIONS {
            let p = pool(f, *cond, seeds);
            out.push(report_line(name, cond, &p));
            if cond.label == "busy,fifo" {
                out.extend(amp_lines(&p));
            }
            if !p.recoveries.is_empty() {
                out.push(recovery_line(&p));
            }
        }
    }
    out.join("\n")
}

// ------------------------------------------------------------------ tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::link::tx_us;

    const AMPLE: AccessProfile = AccessProfile {
        name: "ample",
        up_bps: 1_000_000_000,
        down_bps: 1_000_000_000,
        up_buffer_ms: 100,
        down_buffer_ms: 100,
        last_mile_us: 1_000,
        airtime_permille: 1000,
        cell: None,
        ge: GeParams::NONE,
        verify_us: 60,
        sign_us: 25,
        bg_up: BackgroundModel::NONE,
        bg_down: BackgroundModel::NONE,
        relay: true,
    };

    fn quiet(name: &str, nodes: Vec<NodeSpec>, duration_us: u64) -> Scenario {
        Scenario { name: name.to_owned(), nodes, duration_us, call: CallParams::default(), contention: false, disc: Discipline::TailDrop, impairments: false, seed: 1 }
    }

    #[test]
    fn the_wire_budget_is_what_edge_v45_computes() {
        assert_eq!(link_mdu(500), 431);
        assert_eq!(media_limit(500), 402);
        assert_eq!(frame_budget(500), 246);
        assert_eq!(media_limit(500) - frame_budget(500), 156);
        // A full chunk fills the link MDU exactly and its RNS packet fits the MTU.
        assert_eq!(frame_budget(500) + AV_CHUNK_WIRE_OVERHEAD + CHUNK_TRAILER_LEN + SEAL_OVERHEAD + DATAGRAM_HEADER_LEN, 431);
        assert_eq!(chunk_rns_bytes(frame_budget(500)), 499);
        assert!(chunk_rns_bytes(frame_budget(500)) <= RNS_BASE_MTU);
        assert_eq!(fragments(238, 246), Some(1));
        assert_eq!(fragments(239, 246), Some(2));
        assert_eq!(fragments(0, 246), Some(1));
        assert_eq!(fragments(MAX_FRAME_BYTES + 1, 246), None);
        // 4096 fragments of 238 B is 974,848 B: a 1 MiB frame is refused at the base MTU.
        assert_eq!(fragments(974_848, 246), Some(4_096));
        assert_eq!(fragments(974_849, 246), None);
        for p in MEDIA_PARAM_SPECS {
            assert!(!p.name.is_empty() && !p.source.is_empty());
        }
    }

    #[test]
    fn frame_sizes_hit_the_bitrate() {
        let c = CallParams::default();
        let g = u64::from(c.gop_frames());
        assert_eq!(g, 30);
        let s0 = c.p0_bytes();
        let per_gop = c.key_ratio_milli * s0 / 1_000 + (g / 2 - 1) * s0 + (g / 2) * c.t1_ratio_permille * s0 / 1_000;
        let want = c.video_bps * c.gop_us / 8_000_000;
        assert!(per_gop <= want && want - per_gop < g * 2, "{per_gop} vs {want}");
        assert!(c.standby_permille() > 0 && c.standby_permille() < 1_000);
    }

    fn small_busy(seed: u64) -> Scenario {
        let mut rng = Rng::new(seed);
        let mut nodes: Vec<NodeSpec> = [CABLE_HOME_WIFI, PHONE_LTE, DSL_HOME].iter().map(|&p| NodeSpec::member(p, join_spread(&mut rng, 0, 1_000_000))).collect();
        nodes[1].leave_us = Some(5_000_000);
        nodes[1].graceful = false;
        nodes[2].forced_upload_bytes = 50_000_000;
        Scenario { name: "small".to_owned(), nodes, duration_us: 7_000_000, call: CallParams::default(), contention: true, disc: Discipline::TailDrop, impairments: true, seed }
    }

    #[test]
    fn the_same_seed_gives_the_same_call() {
        let a = Engine::new(small_busy(3)).run();
        let b = Engine::new(small_busy(3)).run();
        let c = Engine::new(small_busy(4)).run();
        assert_eq!(a.sessions, b.sessions);
        assert_eq!(a.ledger, b.ledger);
        assert_eq!(a.links, b.links);
        assert_ne!(a.ledger, c.ledger);
    }

    #[test]
    fn bytes_and_packets_are_conserved() {
        for seed in [1, 2] {
            for disc in [Discipline::TailDrop, CODEL] {
                let mut sc = small_busy(seed);
                sc.disc = disc;
                let r = Engine::new(sc).run();
                assert!(r.links_conserve, "every link: offered = dropped + lost + delivered + flushed + queued");
                assert!(r.ledger.balanced(), "{:?}", r.ledger);
                assert!(r.ledger.tail + r.ledger.air + r.ledger.aqm > 0, "a busy DSL uplink drops something");
            }
        }
    }

    #[test]
    fn ample_lossless_links_give_perfect_quality_at_the_analytic_latency() {
        let c = CallParams { audio_bps: 0, ..CallParams::default() };
        let nodes = vec![
            NodeSpec { receives: false, ..NodeSpec::member(AMPLE, 0) },
            NodeSpec { publishes: false, ..NodeSpec::member(AMPLE, 0) },
        ];
        let mut sc = quiet("ideal", nodes, 20_000_000);
        sc.call = c;
        let (r, logs, vf, _) = Engine::new(sc).run_logged();
        assert!(r.ledger.balanced() && r.links_conserve);
        assert_eq!(r.ledger.air + r.ledger.tail + r.ledger.aqm + r.ledger.stale, 0);
        let q = &r.sessions[0];
        assert_eq!(q.key_lost + q.p0_lost + q.p1_lost + q.undecodable, 0);
        assert_eq!(q.freezes, 0);
        assert_eq!(q.layer_ok_windows, q.layer_windows);
        assert!(q.layer_windows > 10);
        // Glass-to-glass, frame by frame: capture/encode + the first chunk's sign + its uplink,
        // core and downlink time + one verify per fragment (verify > sign + serialisation, so
        // the receiver's CPU paces the frame) + decode/render.
        let log = &logs[1];
        let prop = 2 * AMPLE.last_mile_us + CORE_SAME_REGION_US;
        let budget = frame_budget(RNS_BASE_MTU);
        let first = log.vid_shown.iter().position(|&t| t != 0).expect("video shown");
        let mut checked = 0;
        for (i, f) in vf[0].iter().enumerate().skip(first + 30) {
            let t = log.vid_shown.get(i).copied().unwrap_or(0);
            if t == 0 {
                break;
            }
            let plain0 = if f.frags > 1 { budget } else { f.bytes + FRAGMENT_MARKER_LEN };
            let wire0 = u64::from(chunk_rns_bytes(plain0) + IP_UDP_HEADER);
            let want = AMPLE.sign_us + tx_us(wire0, AMPLE.up_bps) + prop + tx_us(wire0, AMPLE.down_bps) + u64::from(f.frags) * AMPLE.verify_us + c.decode_render_us;
            assert_eq!(t - f.capture_us, want, "frame {i} ({} fragments)", f.frags);
            checked += 1;
        }
        assert!(checked > 100);
        // Audio alone: one chunk per frame, the same sum with one verify.
        let ca = CallParams { video_bps: 0, ..CallParams::default() };
        let nodes = vec![
            NodeSpec { receives: false, ..NodeSpec::member(AMPLE, 0) },
            NodeSpec { publishes: false, ..NodeSpec::member(AMPLE, 0) },
        ];
        let mut sa = quiet("ideal audio", nodes, 10_000_000);
        sa.call = ca;
        let r = Engine::new(sa).run();
        let q = &r.sessions[0];
        assert_eq!(q.audio_missing, 0);
        assert!(q.audio_sent > 300);
        let wire = u64::from(chunk_rns_bytes(60) + IP_UDP_HEADER);
        let want = AMPLE.sign_us + tx_us(wire, AMPLE.up_bps) + prop + tx_us(wire, AMPLE.down_bps) + AMPLE.verify_us;
        assert!(q.audio_lat_samples.iter().skip(5).all(|&l| l == want), "audio latency {:?} vs {want}", &q.audio_lat_samples[..8]);
    }

    #[test]
    fn frame_loss_amplifies_as_the_closed_form_says() {
        // 2% independent loss on the receiver's downlink only; ample capacity; an IDR every
        // four frames so every fragment-count bucket has hundreds of frames.
        let lossy = AccessProfile { ge: GeParams::bernoulli(20_000), ..AMPLE };
        let nodes = vec![
            NodeSpec { receives: false, ..NodeSpec::member(AMPLE, 0) },
            NodeSpec { publishes: false, ..NodeSpec::member(lossy, 0) },
        ];
        let mut sc = quiet("bernoulli", nodes, 120_000_000);
        sc.impairments = true;
        sc.call.gop_us = 266_667;
        let r = Engine::new(sc).run();
        let q = &r.sessions[0];
        assert!(q.chunk_loss_ppm.abs_diff(20_000) < 2_000, "chunk loss {}", q.chunk_loss_ppm);
        let mut tested = 0;
        for b in r.amp.iter().filter(|b| b.sent >= 100) {
            let want = b.closed_form_sum;
            let sd = b.closed_form_var.sqrt();
            assert!((b.lost as f64 - want).abs() <= 4.0 * sd + 1.0, "bucket {}-{}: lost {} of {}, closed form {want:.1} (sd {sd:.1})", b.frags_lo, b.frags_hi, b.lost, b.sent);
            tested += 1;
        }
        assert!(tested >= 2);
        // Keyframes (~54 fragments) lose far more than the 2% a chunk does.
        assert!(q.key_lost * 1_000 / q.key_sent.max(1) > 500);
    }

    fn leave_case(standby: bool) -> (RunResult, u64) {
        let nodes = vec![
            NodeSpec { receives: false, ..NodeSpec::member(AMPLE, 0) },
            NodeSpec { publishes: false, leave_us: Some(20_000_000), ..NodeSpec::member(AMPLE, 0) },
            NodeSpec { publishes: false, ..NodeSpec::member(AMPLE, 0) },
            NodeSpec { publishes: false, relay: false, unreachable: true, ..NodeSpec::member(AMPLE, 0) },
        ];
        let mut sc = quiet("leave", nodes, 40_000_000);
        sc.call.standby = standby;
        let rtt = 2 * (2 * AMPLE.last_mile_us + CORE_SAME_REGION_US);
        (Engine::new(sc).run(), rtt)
    }

    #[test]
    fn a_relay_leaving_costs_a_bounded_gap() {
        let c = CallParams::default();
        let frame = 1_000_000 / u64::from(c.fps);
        // With a warm standby the audio and the base layer never stop; the full layer returns
        // after the tick that sees the closed link, one promotion round trip, the next
        // enhancement frame, and at most one latency budget of path (FSD/twin/media.md §5).
        let (r, rtt) = leave_case(true);
        assert_eq!(r.recoveries.len(), 1);
        let o = r.recoveries[0];
        assert!(o.rec.had_standby && o.rec.graceful);
        assert!(o.audio_gap_us.expect("audio resumed") <= c.audio_frame_us + REORDER_WINDOW_MAX_US);
        assert!(o.video_gap_us.expect("video resumed") <= 2 * frame + REORDER_WINDOW_MAX_US);
        let bound = WATCH_TICK_US + rtt + 2 * frame + LATENCY_BUDGET_US + c.decode_render_us;
        assert!(o.full_restore_us.expect("full layer") <= bound, "{:?} > {bound}", o.full_restore_us);
        // Without one: the tick, a fresh join's round trip, then the base chain is broken until
        // the next IDR.
        let (r, rtt) = leave_case(false);
        let o = r.recoveries[0];
        assert!(!o.rec.had_standby);
        let a_bound = WATCH_TICK_US + rtt + c.audio_frame_us + LATENCY_BUDGET_US;
        let v_bound = WATCH_TICK_US + rtt + c.gop_us + 2 * frame + LATENCY_BUDGET_US + c.decode_render_us;
        assert!(o.audio_gap_us.expect("audio resumed") <= a_bound, "{:?} > {a_bound}", o.audio_gap_us);
        assert!(o.video_gap_us.expect("video resumed") <= v_bound, "{:?} > {v_bound}", o.video_gap_us);
        assert!(o.video_gap_us > Some(2 * frame), "the broken chain waits for an IDR");
    }

    #[test]
    fn household_contention_hurts_a_thin_uplink() {
        let quiet = pool(scenario_congested, CONDITIONS[0], 1);
        let busy = pool(scenario_congested, CONDITIONS[1], 1);
        assert!(quiet.conserved && busy.conserved);
        let miss = |p: &Pooled| p.sessions.iter().filter(|q| q.stream == 0).map(|q| q.audio_missing).sum::<u64>();
        assert!(miss(&busy) > miss(&quiet), "a backup on the DSL uplink costs its publisher's audio");
    }
}
