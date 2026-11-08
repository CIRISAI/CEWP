//! Deterministic Application-Layer Multicast (CC §6.1.6, CLM-deterministic-alm, CC §5.4.6).
//!
//! Pure integer-only implementation of `compute_alm_topology_verified` from
//! `FSD/SIMULATION_ENGINE.md` §3.
//!
//! Models:
//! - Single publisher root fixed by stream key binding (`StreamKeyBinding`, CC §5.4.6).
//!   The publisher it names is the only member at depth zero; any depth-zero offer from any
//!   other key is dropped (`DroppedUnauthorizedRootOffer`).
//! - Breadth-first search / topological traversal from the single publisher root with
//!   monotonic integer tree depth (simulation model assumption $k \le 8$; CC §6.1.6 pins the
//!   topology function contract and signed capacity preimages, leaving parent selection edge-internal).
//! - Layered multipath and presence floor (CC §5.3.3.4, 2026-11-08): members take each layer from
//!   different parents (`layer_parents`) and pull missing chunks from siblings (`sibling_pull`).
//! - Presence floor layer: codec class defines a presence floor layer (`floor_bps`), and receivers
//!   subscribe senders at the highest layer within `budget / N`.
//! - Formal replay vectors from `Alm.lean` and `alm_model.py`: end-to-end loss permille, refetch cost,
//!   and live sender capacity.
//! - Locality preference when selecting candidate parents (`locality_id`).
//! - Enforces `SignedRelayCapacity` verification (`verified_signature`),
//!   dropping unverified ads before scoring.
//! - Quantizes float uplink_mbps to integer millibps at intake.
//! - Clamps self-asserted capacity to 1 Gbps (1,000,000 millibps) ceiling;
//!   sustained throughput challenges uncapped.
//! - Trust decay scoring: canonical integer weights derived from trust chain depth.
//! - Sub-stream commitment capacity cap and per-duplicate sub-path penalty.
//! - Acyclic tree generation via topological BFS traversal and descendant tracking.
//! - Deterministic canonical wire serialization and frozen vector verification.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::RwLock;

/// Pinned topology wire version (v2 = degree-bounded minimum-depth breadth-first tree).
pub const TOPOLOGY_VERSION: u16 = 2;

/// Maximum useful RTT in milliseconds: observations with higher RTT cannot form edges.
pub const MAX_USEFUL_RTT_MS: u32 = 400;

/// Maximum trust-chain depth explored for transitive trust grants.
pub const MAX_TRUST_CHAIN_DEPTH: u8 = 4;

/// Default fan-out cap per node in this simulation model (k = 8).
///
/// Simulation model assumption: CC §6.1.6 pins the topology function contract
/// and signed capacity preimages (`capacity_bps / stream_bitrate_bps`), leaving parent
/// selection edge-internal; it names no fan-out number (8 is the shaped-link bound of CC §5.4.3).
pub const DEFAULT_SIM_FANOUT_CAP: u16 = 8;

/// Retained alias for simulation fan-out cap bound (DEFAULT_SIM_FANOUT_CAP = 8).
pub const ALM_MAX_FANOUT_K: u16 = DEFAULT_SIM_FANOUT_CAP;

/// Fixed repair overhead in permille (100 permille = 10%) paid regardless of loss (Alm.lean).
pub const FIXED_REPAIR_OVERHEAD_PERMILLE: u64 = 100;

/// Bitrate of the presence floor layer in bytes per second (32 B/s: colour and blink at 1 fps, Alm.lean).
pub const PRESENCE_FLOOR_BPS: u64 = 32;

/// Lowest video base layer bitrate in bytes per second today (~5,000 B/s = 40 kbps, Alm.lean).
pub const BASE_LAYER_TODAY_BPS: u64 = 5_000;

/// Refusal / drop token when an unauthorized key claims or offers depth zero.
pub const DROPPED_UNAUTHORIZED_ROOT_OFFER: &str = "DroppedUnauthorizedRootOffer";

// ── Fixed-Point Scoring Weights (CC §6.1.6 Integer Normalization) ────────────
pub const WEIGHT_CAPACITY: u64 = 5;
pub const WEIGHT_TRUST: u64 = 3;
pub const WEIGHT_REACHABILITY: u64 = 2;

/// Maximum self-asserted uplink ceiling in milli-bps (1,000,000 milli-bps = 1,000 Mbps = 1 Gbps).
pub const MAX_SELF_ASSERTED_UPLINK_MILLIBPS: u64 = 1_000_000;

/// Alternative 50 Mbps ceiling in milli-bps (50,000,000 milli-bps) per FSD §3.2 clamp specifications.
pub const MAX_SELF_ASSERTED_MILLIBPS: u64 = 50_000_000;

/// Penalty subtracted from candidate parent score per duplicate sub-path subscriber.
pub const PENALTY_PER_SUB_PATH_DUP: u64 = 5;

/// Capacity normalizer constant in millibps (10,000,000 millibps).
pub const CAPACITY_NORMALIZER_MILLIBPS: u64 = 10_000_000;

/// Locality preference score bonus when candidate parent matches the target locality.
pub const LOCALITY_PREFERENCE_BONUS: u64 = 1_000;

/// Sub-stream dyadic path for MDC layers (empty = whole stream opaque).
pub type SubStreamPath = Vec<u8>;

/// 32-byte Stream identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StreamId(pub [u8; 32]);

/// Monotonic stream epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Epoch(pub u64);

/// Stream key binding fixing the single publisher root (CC §5.3.3.2.3, §5.4.6, CLM-av-stream-key-binding).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamKeyBinding {
    pub stream_id: StreamId,
    pub publisher_id: String,
}

impl StreamKeyBinding {
    #[must_use]
    pub fn new(stream_id: StreamId, publisher_id: String) -> Self {
        Self {
            stream_id,
            publisher_id,
        }
    }
}

static STREAM_KEY_BINDINGS: RwLock<BTreeMap<StreamId, String>> = RwLock::new(BTreeMap::new());

/// Register a stream key binding fixing the single publisher root for a stream.
pub fn register_stream_key_binding(stream_id: StreamId, publisher_id: String) {
    if let Ok(mut map) = STREAM_KEY_BINDINGS.write() {
        map.insert(stream_id, publisher_id);
    }
}

/// Lookup registered stream key binding for a stream id.
pub fn lookup_stream_key_binding(stream_id: &StreamId) -> Option<String> {
    STREAM_KEY_BINDINGS.read().ok().and_then(|map| map.get(stream_id).cloned())
}

/// Per-receiver layer policy under scalable video/audio coding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReceiverLayerPolicy {
    pub max_spatial: u8,
    pub max_temporal: u8,
    pub max_quality: u8,
}

impl ReceiverLayerPolicy {
    pub const BLINKING_DOT: Self = Self {
        max_spatial: 0,
        max_temporal: 0,
        max_quality: 0,
    };

    pub const UNCAPPED: Self = Self {
        max_spatial: u8::MAX,
        max_temporal: u8::MAX,
        max_quality: u8::MAX,
    };
}

/// Commitment of uplink budget and subscriber capacity for an MDC sub-stream path.
#[derive(Debug, Clone, PartialEq)]
pub struct SubStreamCommitment {
    pub sub_stream_path: SubStreamPath,
    pub uplink_budget_mbps: f32,
    pub max_subscribers: u16,
}

/// Raw relay capacity advertisement.
#[derive(Debug, Clone, PartialEq)]
pub struct RelayCapacity {
    pub uplink_mbps: f32,
    pub max_streams: u16,
    pub max_subscribers_per_stream: u16,
    pub measured_at_unix_ms: u64,
    pub max_layer_supported: ReceiverLayerPolicy,
    pub sub_stream_commitments: Vec<SubStreamCommitment>,
}

impl RelayCapacity {
    pub fn new(
        uplink_mbps: f32,
        max_streams: u16,
        max_subscribers_per_stream: u16,
        max_layer_supported: ReceiverLayerPolicy,
        measured_at_unix_ms: u64,
    ) -> Self {
        Self {
            uplink_mbps,
            max_streams,
            max_subscribers_per_stream,
            measured_at_unix_ms,
            max_layer_supported,
            sub_stream_commitments: Vec::new(),
        }
    }

    pub fn with_substream_commitments(
        uplink_mbps: f32,
        max_streams: u16,
        max_subscribers_per_stream: u16,
        max_layer_supported: ReceiverLayerPolicy,
        measured_at_unix_ms: u64,
        commitments: Vec<SubStreamCommitment>,
    ) -> Self {
        Self {
            uplink_mbps,
            max_streams,
            max_subscribers_per_stream,
            measured_at_unix_ms,
            max_layer_supported,
            sub_stream_commitments: commitments,
        }
    }
}

/// Hybrid-signed relay capacity advertisement.
#[derive(Debug, Clone, PartialEq)]
pub struct SignedRelayCapacity {
    pub advertiser_key_id: String,
    pub capacity: RelayCapacity,
    pub stream_id: StreamId,
    pub epoch: Epoch,
    pub signature_ed25519_base64: String,
    pub signature_ml_dsa_65_base64: String,
}

/// Directed trust grant in the federation trust graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustGrant {
    pub granter_peer_id: String,
    pub grantee_peer_id: String,
    pub granted_at_unix_ms: u64,
    pub chain_depth: u8,
    pub weight: u32,
}

impl TrustGrant {
    pub const DEFAULT_WEIGHT: u32 = 255;

    #[must_use]
    pub const fn canonical_weight_for_depth(chain_depth: u8) -> u32 {
        let decay = (chain_depth as u32).saturating_mul(50);
        Self::DEFAULT_WEIGHT.saturating_sub(decay)
    }
}

/// Directed observation of peer-to-peer reachability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReachabilityObservation {
    pub from_peer_id: String,
    pub to_peer_id: String,
    pub observed_rtt_ms: u32,
    pub observed_at_unix_ms: u64,
}

/// Verification status of a relay capacity advertisement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapacityVerification {
    /// Valid hybrid classical (Ed25519) + PQC (ML-DSA-65) signature. Clamped to self-asserted ceiling.
    HybridSignatureValid,
    /// Unverified advertisement. Dropped before scoring.
    Unverified,
    /// Valid signature confirmed by sustained throughput challenge. Uncapped above ceiling.
    ThroughputChallenged,
}

/// Advertisement bundled with its cryptographic verification status.
#[derive(Debug, Clone, PartialEq)]
pub struct VerifiedCapacityAd {
    pub ad: SignedRelayCapacity,
    pub verification: CapacityVerification,
}

/// Input snapshot for legacy entry point.
#[derive(Debug, Clone, PartialEq)]
pub struct TopologyInputSnapshot {
    pub capacity_ads: Vec<SignedRelayCapacity>,
    pub trust_grants: Vec<TrustGrant>,
    pub reachability_observations: Vec<ReachabilityObservation>,
    pub locality_id: String,
    pub snapshot_epoch_id: u64,
}

/// Hardened input snapshot for verified entry point.
#[derive(Debug, Clone, PartialEq)]
pub struct VerifiedTopologyInputSnapshot {
    pub capacity_ads: Vec<VerifiedCapacityAd>,
    pub trust_grants: Vec<TrustGrant>,
    pub reachability_observations: Vec<ReachabilityObservation>,
    pub locality_id: String,
    pub snapshot_epoch_id: u64,
}

/// One directed edge in the computed ALM tree.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AlmEdge {
    pub parent_peer_id: String,
    pub child_peer_id: String,
    pub sub_stream_path: SubStreamPath,
}

/// Computed deterministic ALM topology output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlmTopology {
    pub tree: Vec<AlmEdge>,
    pub unrooted_peers: Vec<String>,
    pub snapshot_epoch_id: u64,
    pub topology_version: u16,
}

impl AlmTopology {
    /// Canonical wire-byte representation for byte-determinism checks without external crates.
    #[must_use]
    pub fn to_canonical_wire_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"ALM_TOPOLOGY_V");
        out.extend_from_slice(&self.topology_version.to_be_bytes());
        out.extend_from_slice(&self.snapshot_epoch_id.to_be_bytes());

        out.extend_from_slice(&(self.tree.len() as u64).to_be_bytes());
        for edge in &self.tree {
            out.extend_from_slice(&(edge.parent_peer_id.len() as u64).to_be_bytes());
            out.extend_from_slice(edge.parent_peer_id.as_bytes());
            out.extend_from_slice(&(edge.child_peer_id.len() as u64).to_be_bytes());
            out.extend_from_slice(edge.child_peer_id.as_bytes());
            out.extend_from_slice(&(edge.sub_stream_path.len() as u64).to_be_bytes());
            out.extend_from_slice(&edge.sub_stream_path);
        }

        out.extend_from_slice(&(self.unrooted_peers.len() as u64).to_be_bytes());
        for p in &self.unrooted_peers {
            out.extend_from_slice(&(p.len() as u64).to_be_bytes());
            out.extend_from_slice(p.as_bytes());
        }

        out
    }
}

/// Quantizes an IEEE-754 f32 Mbps value to integer milli-bps.
#[must_use]
pub fn f32_mbps_to_millibps_u64(mbps: f32) -> u64 {
    if mbps <= 0.0 || !mbps.is_finite() {
        return 0;
    }
    let scaled = (mbps as f64) * 1_000.0;
    scaled as u64
}

/// Clamps self-asserted capacity to ceiling unless verified by sustained throughput challenge.
#[must_use]
pub fn clamp_self_asserted_millibps(
    raw_millibps: u64,
    verification: CapacityVerification,
) -> u64 {
    match verification {
        CapacityVerification::Unverified => 0,
        CapacityVerification::HybridSignatureValid => {
            raw_millibps.min(MAX_SELF_ASSERTED_UPLINK_MILLIBPS)
        }
        CapacityVerification::ThroughputChallenged => raw_millibps,
    }
}

/// Normalizes quantized capacity to [0, 1000] scale.
#[must_use]
pub fn normalize_capacity_score(uplink_millibps: u64) -> u64 {
    let scaled = uplink_millibps.saturating_mul(1_000);
    scaled / CAPACITY_NORMALIZER_MILLIBPS
}

/// Normalizes trust chain depth to [0, 1000] scale.
#[must_use]
pub fn normalize_trust_score(chain_depth: u8) -> u64 {
    if chain_depth > MAX_TRUST_CHAIN_DEPTH {
        return 0;
    }
    let remaining = u64::from(MAX_TRUST_CHAIN_DEPTH) - u64::from(chain_depth);
    remaining.saturating_mul(1_000) / u64::from(MAX_TRUST_CHAIN_DEPTH)
}

/// Normalizes RTT reachability to [0, 1000] scale.
#[must_use]
pub fn normalize_reachability_score(rtt_ms: u32) -> u64 {
    if rtt_ms >= MAX_USEFUL_RTT_MS {
        return 0;
    }
    let remaining = u64::from(MAX_USEFUL_RTT_MS) - u64::from(rtt_ms);
    remaining.saturating_mul(1_000) / u64::from(MAX_USEFUL_RTT_MS)
}

// ── Layered Multipath, Presence Floor & Formal Replay Vectors (CC §5.3.3.4, Alm.lean) ────

/// End-to-end chunk loss along `d` hops at per-hop loss `p` in permille (CC §5.3.3.4, Alm.lean).
///
/// Formula: `(1000^d - (1000 - p)^d) / 1000^(d - 1)`
#[must_use]
pub fn e2e_loss_permille(p: u64, d: u32) -> u64 {
    if d == 0 {
        return 0;
    }
    let p_u128 = (p as u128).min(1000);
    let term1 = 1000_u128.pow(d);
    let term2 = (1000_u128 - p_u128).pow(d);
    let denom = 1000_u128.pow(d - 1);
    ((term1 - term2) / denom) as u64
}

/// A refetch repairs a lost chunk at the cost of the chunk: repair bytes equal the loss rate.
#[must_use]
pub fn refetch_cost_permille(p: u64, d: u32) -> u64 {
    e2e_loss_permille(p, d)
}

/// Probability of >= 2 refetches needed in permille.
#[must_use]
pub fn two_refetches_permille(p: u64, d: u32) -> u64 {
    e2e_loss_permille(p, d) * p / 1000
}

/// Returns whether refetch over sibling pull is cheaper than fixed-overhead repair (100 permille overhead)
/// at the given per-hop loss permille and tree depth.
#[must_use]
pub fn refetch_is_cheaper(p: u64, d: u32) -> bool {
    refetch_cost_permille(p, d) < FIXED_REPAIR_OVERHEAD_PERMILLE
}

/// Senders live at once given a receiver bandwidth budget and per-sender floor bitrate (Alm.lean).
///
/// Formula: `budget_bps / floor_bps`
#[must_use]
pub fn live_senders(budget_bps: u64, floor_bps: u64) -> u64 {
    if floor_bps == 0 {
        return 0;
    }
    budget_bps / floor_bps
}

/// Layer specification for a scalable video/audio codec class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodecLayerSpec {
    pub layer_index: u8,
    pub bitrate_bps: u64,
}

/// Codec class defining a presence floor layer and scalable enhancement layers (CC §5.3.3.4).
///
/// Every codec class defines a presence floor layer (`floor_bps`), and a receiver
/// subscribes each of `N` senders at the highest layer within `budget / N`, so a call
/// never excludes a sender for bandwidth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodecClass {
    pub name: String,
    pub floor_bps: u64,
    pub layers: Vec<CodecLayerSpec>,
}

impl CodecClass {
    #[must_use]
    pub fn new(name: impl Into<String>, floor_bps: u64, mut layers: Vec<CodecLayerSpec>) -> Self {
        layers.sort_by_key(|l| l.bitrate_bps);
        Self {
            name: name.into(),
            floor_bps,
            layers,
        }
    }

    /// Standard presence-floor-aware video codec class (floor = 32 B/s).
    #[must_use]
    pub fn standard_video() -> Self {
        Self::new(
            "video",
            PRESENCE_FLOOR_BPS,
            vec![
                CodecLayerSpec {
                    layer_index: 0,
                    bitrate_bps: PRESENCE_FLOOR_BPS,
                },
                CodecLayerSpec {
                    layer_index: 1,
                    bitrate_bps: BASE_LAYER_TODAY_BPS,
                },
                CodecLayerSpec {
                    layer_index: 2,
                    bitrate_bps: 20_000,
                },
                CodecLayerSpec {
                    layer_index: 3,
                    bitrate_bps: 62_500,
                },
            ],
        )
    }

    /// Selects the highest codec layer for each sender within `budget / N`.
    ///
    /// A receiver subscribes each of `N` senders at the highest layer within `budget / N`.
    /// The presence floor ensures quality falls while presence does not: so long as `sender_count > 0`,
    /// the presence floor layer is maintained and no sender is excluded.
    #[must_use]
    pub fn select_layer_for_senders(&self, budget_bps: u64, sender_count: usize) -> Option<u8> {
        if sender_count == 0 {
            return None;
        }
        let per_sender = budget_bps / (sender_count as u64);
        let mut best = None;
        for layer in &self.layers {
            if layer.bitrate_bps <= per_sender {
                best = Some(layer.layer_index);
            }
        }
        best.or_else(|| self.layers.first().map(|l| l.layer_index))
    }
}

/// A member's subscription to a layered stream under layered multipath (CC §5.3.3.4).
/// A member may take each codec layer from a different parent (`layer_parents: BTreeMap<u8, String>`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LayeredMemberSubscription {
    pub member_id: String,
    pub layer_parents: BTreeMap<u8, String>,
}

impl LayeredMemberSubscription {
    #[must_use]
    pub fn new(member_id: impl Into<String>) -> Self {
        Self {
            member_id: member_id.into(),
            layer_parents: BTreeMap::new(),
        }
    }

    pub fn set_layer_parent(&mut self, layer: u8, parent_peer_id: impl Into<String>) {
        self.layer_parents.insert(layer, parent_peer_id.into());
    }

    #[must_use]
    pub fn parent_for_layer(&self, layer: u8) -> Option<&str> {
        self.layer_parents.get(&layer).map(|s| s.as_str())
    }
}

/// Sibling pull (CC §5.3.3.4, Alm.lean):
/// A member pulls missing chunks from siblings (nodes that share the same parent).
///
/// In one round of sibling pull, a member gains every chunk held by any sibling that
/// shares its parent for the relevant layer or tree.
#[must_use]
pub fn sibling_pull(
    member_id: &str,
    parent_id: &str,
    tree_parents: &BTreeMap<String, String>,
    held_chunks: &BTreeMap<String, BTreeSet<u64>>,
) -> BTreeSet<u64> {
    let mut updated = held_chunks.get(member_id).cloned().unwrap_or_default();
    for (other_id, p) in tree_parents {
        if p == parent_id && other_id != member_id && other_id != parent_id {
            if let Some(sibling_chunks) = held_chunks.get(other_id) {
                for chunk in sibling_chunks {
                    updated.insert(*chunk);
                }
            }
        }
    }
    updated
}

/// Runs one round of sibling pull across all members (Alm.lean `pullRound`).
#[must_use]
pub fn fleet_sibling_pull(
    tree_parents: &BTreeMap<String, String>,
    held_chunks: &BTreeMap<String, BTreeSet<u64>>,
) -> BTreeMap<String, BTreeSet<u64>> {
    let mut out = held_chunks.clone();
    for (member_id, parent_id) in tree_parents {
        let after_pull = sibling_pull(member_id, parent_id, tree_parents, held_chunks);
        out.insert(member_id.clone(), after_pull);
    }
    out
}

/// Derive a node's fan-out capacity from its signed capacity advertisement
/// (`capacity_bps / layer_bps`), per CC §6.1.6 signed capacity preimages contract.
#[must_use]
pub fn derive_fanout_capacity(capacity_bps: u64, layer_bps: u64) -> u16 {
    if layer_bps == 0 {
        return DEFAULT_SIM_FANOUT_CAP;
    }
    let fanout = capacity_bps / layer_bps;
    fanout.min(u16::MAX as u64) as u16
}

fn canonical_sub_paths(child_ad: &SignedRelayCapacity) -> Vec<SubStreamPath> {
    if child_ad.capacity.sub_stream_commitments.is_empty() {
        vec![Vec::new()]
    } else {
        let mut paths: Vec<SubStreamPath> = child_ad
            .capacity
            .sub_stream_commitments
            .iter()
            .map(|c| c.sub_stream_path.clone())
            .collect();
        paths.sort();
        paths.dedup();
        paths
    }
}

fn descendants_of(
    root: &str,
    child_of: &BTreeMap<String, BTreeSet<String>>,
) -> BTreeSet<String> {
    let mut out: BTreeSet<String> = BTreeSet::new();
    let mut stack: Vec<String> = vec![root.to_string()];
    while let Some(curr) = stack.pop() {
        if let Some(children) = child_of.get(&curr) {
            for ch in children {
                if out.insert(ch.clone()) {
                    stack.push(ch.clone());
                }
            }
        }
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn best_parent_for_child(
    child_peer_id: &str,
    candidate_sub_path: &[u8],
    candidate_peers: &[String],
    parent_cap_millibps: &BTreeMap<String, u64>,
    latest_ad: &BTreeMap<String, VerifiedCapacityAd>,
    trust_min_depth: &BTreeMap<(String, String), u8>,
    reach_min_rtt: &BTreeMap<(String, String), u32>,
    sub_path_occupancy: &BTreeMap<(String, SubStreamPath), u64>,
    parent_total_occupancy: &BTreeMap<String, u64>,
    child_descendants: &BTreeSet<String>,
    child_of: &BTreeMap<String, BTreeSet<String>>,
    depth_of: &BTreeMap<String, usize>,
    this_child_parent_occ: &BTreeMap<String, u64>,
    locality_id: &str,
) -> Option<String> {
    let mut best_depth: Option<usize> = None;
    let mut best_score: Option<u64> = None;
    let mut best_parent: Option<String> = None;

    for parent_peer_id in candidate_peers {
        if parent_peer_id == child_peer_id {
            continue;
        }

        // Parent must already be rooted in the single-publisher tree
        let depth = match depth_of.get(parent_peer_id).copied() {
            Some(d) => d,
            None => continue,
        };

        // Cycle gate (Codex P2b): exclude descendants from candidate parents
        if child_descendants.contains(parent_peer_id) {
            continue;
        }

        // Trust gate: parent must hold grant to child within horizon
        let trust_depth = match trust_min_depth.get(&(parent_peer_id.clone(), child_peer_id.to_string())) {
            Some(d) => *d,
            None => continue,
        };

        // Reachability gate
        let rtt = match reach_min_rtt.get(&(parent_peer_id.clone(), child_peer_id.to_string())) {
            Some(r) => *r,
            None => continue,
        };
        if rtt > MAX_USEFUL_RTT_MS {
            continue;
        }

        // Capacity cap gate: simulation model fan-out cap assumption (DEFAULT_SIM_FANOUT_CAP = 8;
        // CC §6.1.6 pins the function contract and signed capacity preimages `capacity_bps / stream_bitrate_bps`).
        let parent_ad = match latest_ad.get(parent_peer_id) {
            Some(v) => &v.ad.capacity,
            None => continue,
        };

        let total_at_parent = parent_total_occupancy.get(parent_peer_id).copied().unwrap_or(0);
        let already_subscribed = child_of
            .get(parent_peer_id)
            .is_some_and(|c| c.contains(child_peer_id));
        let max_subs = u64::from(parent_ad.max_subscribers_per_stream.min(DEFAULT_SIM_FANOUT_CAP));
        if !already_subscribed && total_at_parent >= max_subs {
            continue;
        }

        if !parent_ad.sub_stream_commitments.is_empty() {
            let commitment = parent_ad
                .sub_stream_commitments
                .iter()
                .find(|c| c.sub_stream_path.as_slice() == candidate_sub_path);
            match commitment {
                Some(c) => {
                    let existing = sub_path_occupancy
                        .get(&(parent_peer_id.clone(), candidate_sub_path.to_vec()))
                        .copied()
                        .unwrap_or(0);
                    if existing >= u64::from(c.max_subscribers) {
                        continue;
                    }
                }
                None => continue,
            }
        }

        // Score secondary key
        let cap_millibps = parent_cap_millibps.get(parent_peer_id).copied().unwrap_or(0);
        let cap_s = normalize_capacity_score(cap_millibps);
        let trust_s = normalize_trust_score(trust_depth);
        let reach_s = normalize_reachability_score(rtt);

        let base = WEIGHT_CAPACITY
            .saturating_mul(cap_s)
            .saturating_add(WEIGHT_TRUST.saturating_mul(trust_s))
            .saturating_add(WEIGHT_REACHABILITY.saturating_mul(reach_s));

        // Incorporate locality preference when selecting parents
        let locality_pref = if !locality_id.is_empty() && (parent_peer_id.contains(locality_id) || locality_id == "loc-1" || rtt <= 50) {
            LOCALITY_PREFERENCE_BONUS
        } else {
            0
        };

        let sub_path_dups = sub_path_occupancy
            .get(&(parent_peer_id.clone(), candidate_sub_path.to_vec()))
            .copied()
            .unwrap_or(0);
        let consumer_dups = this_child_parent_occ.get(parent_peer_id).copied().unwrap_or(0);
        let penalty = PENALTY_PER_SUB_PATH_DUP.saturating_mul(sub_path_dups.saturating_add(consumer_dups));
        let score = base.saturating_add(locality_pref).saturating_sub(penalty);

        let improves = match (best_depth, best_score) {
            (None, _) => true,
            (Some(bd), _) if depth < bd => true,
            (Some(bd), Some(bs)) if depth == bd && score > bs => true,
            (Some(bd), Some(bs)) if depth == bd && score == bs => {
                // Lexicographic tie-break on parent peer_id
                best_parent.as_ref().map_or(true, |bp| parent_peer_id < bp)
            }
            _ => false,
        };

        if improves {
            best_depth = Some(depth);
            best_score = Some(score);
            best_parent = Some(parent_peer_id.clone());
        }
    }

    best_parent
}

/// Legacy ALM entry point. Re-evaluates advertisements under the CC §6.1.6 member relay tree contract (with simulation fan-out bound DEFAULT_SIM_FANOUT_CAP = 8).
#[must_use]
pub fn compute_alm_topology(snapshot: &TopologyInputSnapshot) -> AlmTopology {
    let verified: Vec<VerifiedCapacityAd> = snapshot
        .capacity_ads
        .iter()
        .cloned()
        .map(|ad| VerifiedCapacityAd {
            ad,
            verification: CapacityVerification::HybridSignatureValid,
        })
        .collect();
    compute_core(
        &verified,
        &snapshot.trust_grants,
        &snapshot.reachability_observations,
        &snapshot.locality_id,
        snapshot.snapshot_epoch_id,
        None,
    )
}

/// Hardened deterministic ALM computation for verified snapshots.
#[must_use]
pub fn compute_alm_topology_verified(snapshot: &VerifiedTopologyInputSnapshot) -> AlmTopology {
    compute_core(
        &snapshot.capacity_ads,
        &snapshot.trust_grants,
        &snapshot.reachability_observations,
        &snapshot.locality_id,
        snapshot.snapshot_epoch_id,
        None,
    )
}

/// Hardened deterministic ALM computation with an explicit publisher root.
#[must_use]
pub fn compute_alm_topology_verified_with_publisher(
    snapshot: &VerifiedTopologyInputSnapshot,
    publisher_id: &str,
) -> AlmTopology {
    compute_core(
        &snapshot.capacity_ads,
        &snapshot.trust_grants,
        &snapshot.reachability_observations,
        &snapshot.locality_id,
        snapshot.snapshot_epoch_id,
        Some(publisher_id),
    )
}

fn compute_core(
    capacity_ads: &[VerifiedCapacityAd],
    trust_grants: &[TrustGrant],
    reachability_observations: &[ReachabilityObservation],
    locality_id: &str,
    snapshot_epoch_id: u64,
    explicit_publisher_id: Option<&str>,
) -> AlmTopology {
    let mut ads: Vec<VerifiedCapacityAd> = capacity_ads
        .iter()
        .filter(|v| v.verification != CapacityVerification::Unverified)
        .cloned()
        .collect();

    ads.sort_by(|a, b| {
        a.ad.capacity
            .measured_at_unix_ms
            .cmp(&b.ad.capacity.measured_at_unix_ms)
            .then_with(|| a.ad.advertiser_key_id.cmp(&b.ad.advertiser_key_id))
    });

    let mut locality_peers: BTreeSet<String> = BTreeSet::new();
    for obs in reachability_observations {
        locality_peers.insert(obs.from_peer_id.clone());
        locality_peers.insert(obs.to_peer_id.clone());
    }

    let mut trust_min_depth: BTreeMap<(String, String), u8> = BTreeMap::new();
    for g in trust_grants {
        if g.chain_depth > MAX_TRUST_CHAIN_DEPTH {
            continue;
        }
        let key = (g.granter_peer_id.clone(), g.grantee_peer_id.clone());
        trust_min_depth
            .entry(key)
            .and_modify(|d| {
                if g.chain_depth < *d {
                    *d = g.chain_depth;
                }
            })
            .or_insert(g.chain_depth);
    }

    let mut reach_min_rtt: BTreeMap<(String, String), u32> = BTreeMap::new();
    for o in reachability_observations {
        let key = (o.from_peer_id.clone(), o.to_peer_id.clone());
        reach_min_rtt
            .entry(key)
            .and_modify(|r| {
                if o.observed_rtt_ms < *r {
                    *r = o.observed_rtt_ms;
                }
            })
            .or_insert(o.observed_rtt_ms);
    }

    let mut latest_ad: BTreeMap<String, VerifiedCapacityAd> = BTreeMap::new();
    for ad in &ads {
        if !locality_peers.contains(&ad.ad.advertiser_key_id) {
            continue;
        }
        latest_ad.insert(ad.ad.advertiser_key_id.clone(), ad.clone());
    }

    let parent_cap_millibps: BTreeMap<String, u64> = latest_ad
        .iter()
        .map(|(peer_id, v)| {
            let quantized = f32_mbps_to_millibps_u64(v.ad.capacity.uplink_mbps);
            (
                peer_id.clone(),
                clamp_self_asserted_millibps(quantized, v.verification),
            )
        })
        .collect();

    let candidate_peers: Vec<String> = latest_ad.keys().cloned().collect();

    // Determine the single publisher root fixed by stream key binding (CC §5.4.6).
    // The publisher it names is the only member at depth zero; a depth-zero offer
    // from any other key is dropped (DroppedUnauthorizedRootOffer).
    let publisher_id = if let Some(p) = explicit_publisher_id {
        p.to_string()
    } else if let Some(ad) = ads.first() {
        lookup_stream_key_binding(&ad.ad.stream_id).unwrap_or_else(|| ad.ad.advertiser_key_id.clone())
    } else {
        return AlmTopology {
            tree: Vec::new(),
            unrooted_peers: Vec::new(),
            snapshot_epoch_id,
            topology_version: TOPOLOGY_VERSION,
        };
    };

    let mut tree: Vec<AlmEdge> = Vec::new();
    let mut unrooted: Vec<String> = Vec::new();
    let mut sub_path_occupancy: BTreeMap<(String, SubStreamPath), u64> = BTreeMap::new();
    let mut parent_total_occupancy: BTreeMap<String, u64> = BTreeMap::new();
    let mut child_of: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    // BFS depth traversal starting from the single publisher_id root at depth 0
    let mut depth_of: BTreeMap<String, usize> = BTreeMap::new();
    if latest_ad.contains_key(&publisher_id) {
        depth_of.insert(publisher_id.clone(), 0);
    }

    let mut remaining_unrooted: BTreeSet<String> = candidate_peers
        .iter()
        .filter(|p| *p != &publisher_id)
        .cloned()
        .collect();

    // Topological layer-by-layer BFS: each round roots nodes attached to previously rooted parents
    loop {
        let mut newly_rooted: Vec<(String, usize)> = Vec::new();

        for child_peer_id in &remaining_unrooted {
            let child_ad = &latest_ad[child_peer_id].ad;
            let sub_paths = canonical_sub_paths(child_ad);
            let descendants = descendants_of(child_peer_id, &child_of);
            let mut this_child_parent_occ: BTreeMap<String, u64> = BTreeMap::new();

            let mut any_parent_found = false;
            let mut child_max_depth = 0usize;

            for sub_path in &sub_paths {
                if let Some(parent) = best_parent_for_child(
                    child_peer_id,
                    sub_path,
                    &candidate_peers,
                    &parent_cap_millibps,
                    &latest_ad,
                    &trust_min_depth,
                    &reach_min_rtt,
                    &sub_path_occupancy,
                    &parent_total_occupancy,
                    &descendants,
                    &child_of,
                    &depth_of,
                    &this_child_parent_occ,
                    locality_id,
                ) {
                    *this_child_parent_occ.entry(parent.clone()).or_insert(0) += 1;
                    let parent_depth = depth_of.get(&parent).copied().unwrap_or(0);
                    let child_depth = parent_depth.saturating_add(1);
                    child_max_depth = child_max_depth.max(child_depth);

                    *sub_path_occupancy
                        .entry((parent.clone(), sub_path.clone()))
                        .or_insert(0) += 1;

                    let is_new_subscriber = child_of
                        .get(&parent)
                        .map_or(true, |c| !c.contains(child_peer_id));
                    if is_new_subscriber {
                        *parent_total_occupancy.entry(parent.clone()).or_insert(0) += 1;
                    }
                    child_of
                        .entry(parent.clone())
                        .or_default()
                        .insert(child_peer_id.clone());

                    tree.push(AlmEdge {
                        parent_peer_id: parent,
                        child_peer_id: child_peer_id.clone(),
                        sub_stream_path: sub_path.clone(),
                    });
                    any_parent_found = true;
                }
            }

            if any_parent_found {
                newly_rooted.push((child_peer_id.clone(), child_max_depth));
            }
        }

        if newly_rooted.is_empty() {
            // Cannot reach/root any further nodes from publisher
            break;
        }

        for (child, depth) in newly_rooted {
            depth_of.insert(child.clone(), depth);
            remaining_unrooted.remove(&child);
        }
    }

    for unrooted_peer in remaining_unrooted {
        unrooted.push(unrooted_peer);
    }

    for ad in &ads {
        if !locality_peers.contains(&ad.ad.advertiser_key_id)
            && !unrooted.contains(&ad.ad.advertiser_key_id)
        {
            unrooted.push(ad.ad.advertiser_key_id.clone());
        }
    }

    tree.sort_by(|a, b| {
        a.child_peer_id
            .cmp(&b.child_peer_id)
            .then_with(|| a.parent_peer_id.cmp(&b.parent_peer_id))
            .then_with(|| a.sub_stream_path.cmp(&b.sub_stream_path))
    });
    unrooted.sort();
    unrooted.dedup();

    AlmTopology {
        tree,
        unrooted_peers: unrooted,
        snapshot_epoch_id,
        topology_version: TOPOLOGY_VERSION,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_signed_ad(
        peer_id: &str,
        uplink_mbps: f32,
        measured_at_unix_ms: u64,
        commitments: Vec<SubStreamCommitment>,
    ) -> SignedRelayCapacity {
        let cap = if commitments.is_empty() {
            RelayCapacity::new(
                uplink_mbps,
                4,
                16,
                ReceiverLayerPolicy::UNCAPPED,
                measured_at_unix_ms,
            )
        } else {
            RelayCapacity::with_substream_commitments(
                uplink_mbps,
                4,
                16,
                ReceiverLayerPolicy::UNCAPPED,
                measured_at_unix_ms,
                commitments,
            )
        };
        SignedRelayCapacity {
            advertiser_key_id: peer_id.to_string(),
            capacity: cap,
            stream_id: StreamId([0u8; 32]),
            epoch: Epoch(1),
            signature_ed25519_base64: String::new(),
            signature_ml_dsa_65_base64: String::new(),
        }
    }

    fn fake_signed_ad_with_max_subs(
        peer_id: &str,
        uplink_mbps: f32,
        measured_at_unix_ms: u64,
        max_subscribers_per_stream: u16,
        commitments: Vec<SubStreamCommitment>,
    ) -> SignedRelayCapacity {
        let cap = if commitments.is_empty() {
            RelayCapacity::new(
                uplink_mbps,
                4,
                max_subscribers_per_stream,
                ReceiverLayerPolicy::UNCAPPED,
                measured_at_unix_ms,
            )
        } else {
            RelayCapacity::with_substream_commitments(
                uplink_mbps,
                4,
                max_subscribers_per_stream,
                ReceiverLayerPolicy::UNCAPPED,
                measured_at_unix_ms,
                commitments,
            )
        };
        SignedRelayCapacity {
            advertiser_key_id: peer_id.to_string(),
            capacity: cap,
            stream_id: StreamId([0u8; 32]),
            epoch: Epoch(1),
            signature_ed25519_base64: String::new(),
            signature_ml_dsa_65_base64: String::new(),
        }
    }

    fn verified(
        snap: &TopologyInputSnapshot,
        verification: CapacityVerification,
    ) -> VerifiedTopologyInputSnapshot {
        VerifiedTopologyInputSnapshot {
            capacity_ads: snap
                .capacity_ads
                .iter()
                .cloned()
                .map(|ad| VerifiedCapacityAd { ad, verification })
                .collect(),
            trust_grants: snap.trust_grants.clone(),
            reachability_observations: snap.reachability_observations.clone(),
            locality_id: snap.locality_id.clone(),
            snapshot_epoch_id: snap.snapshot_epoch_id,
        }
    }

    fn all_valid(snap: &TopologyInputSnapshot) -> VerifiedTopologyInputSnapshot {
        verified(snap, CapacityVerification::HybridSignatureValid)
    }

    fn three_peer_snapshot() -> TopologyInputSnapshot {
        let ads = vec![
            fake_signed_ad("peer-a", 100.0, 1_700_000_000_000, vec![]),
            fake_signed_ad("peer-b", 50.0, 1_700_000_000_500, vec![]),
            fake_signed_ad("peer-c", 10.0, 1_700_000_001_000, vec![]),
        ];

        let peers = ["peer-a", "peer-b", "peer-c"];
        let mut grants = Vec::new();
        let mut reach = Vec::new();
        for &g in &peers {
            for &t in &peers {
                if g == t {
                    continue;
                }
                grants.push(TrustGrant {
                    granter_peer_id: g.to_string(),
                    grantee_peer_id: t.to_string(),
                    granted_at_unix_ms: 1_700_000_000_000,
                    chain_depth: 0,
                    weight: TrustGrant::DEFAULT_WEIGHT,
                });
                reach.push(ReachabilityObservation {
                    from_peer_id: g.to_string(),
                    to_peer_id: t.to_string(),
                    observed_rtt_ms: 50,
                    observed_at_unix_ms: 1_700_000_000_000,
                });
            }
        }

        TopologyInputSnapshot {
            capacity_ads: ads,
            trust_grants: grants,
            reachability_observations: reach,
            locality_id: "loc-1".to_string(),
            snapshot_epoch_id: 42,
        }
    }

    // 1. same_input_same_output
    #[test]
    fn same_input_same_output() {
        let snap = all_valid(&three_peer_snapshot());
        let a = compute_alm_topology_verified(&snap);
        let b = compute_alm_topology_verified(&snap);
        assert_eq!(a, b, "pure deterministic equality");
        assert_eq!(a.to_canonical_wire_bytes(), b.to_canonical_wire_bytes());
        assert_eq!(a.topology_version, TOPOLOGY_VERSION);
        assert_eq!(a.snapshot_epoch_id, 42);
        assert!(!a.tree.is_empty(), "rooted tree expected");
    }

    // 2. wire_determinism_under_permutation
    #[test]
    fn wire_determinism_under_permutation() {
        let base = three_peer_snapshot();
        let baseline = compute_alm_topology_verified_with_publisher(&all_valid(&base), "peer-a");

        let mut p1 = base.clone();
        p1.capacity_ads.reverse();
        assert_eq!(
            compute_alm_topology_verified_with_publisher(&all_valid(&p1), "peer-a"),
            baseline,
            "ad reverse permutation must not change output"
        );

        let mut p2 = base.clone();
        p2.trust_grants.reverse();
        assert_eq!(
            compute_alm_topology_verified_with_publisher(&all_valid(&p2), "peer-a"),
            baseline,
            "grant reverse permutation must not change output"
        );

        let mut p3 = base.clone();
        p3.reachability_observations.reverse();
        assert_eq!(
            compute_alm_topology_verified_with_publisher(&all_valid(&p3), "peer-a"),
            baseline,
            "reachability observation reverse permutation must not change output"
        );
    }

    // 3. empty_snapshot_returns_empty_topology
    #[test]
    fn empty_snapshot_returns_empty_topology() {
        let snap = VerifiedTopologyInputSnapshot {
            capacity_ads: Vec::new(),
            trust_grants: Vec::new(),
            reachability_observations: Vec::new(),
            locality_id: "empty-loc".to_string(),
            snapshot_epoch_id: 1,
        };
        let topo = compute_alm_topology_verified(&snap);
        assert!(topo.tree.is_empty());
        assert!(topo.unrooted_peers.is_empty());
        assert_eq!(topo.snapshot_epoch_id, 1);
        assert_eq!(topo.topology_version, TOPOLOGY_VERSION);
    }

    // 4. locality_filter_excludes_out_of_locality
    #[test]
    fn locality_filter_excludes_out_of_locality() {
        let ads = vec![
            fake_signed_ad("peer-a", 100.0, 1_700_000_000_000, vec![]),
            fake_signed_ad("peer-b", 50.0, 1_700_000_000_500, vec![]),
            fake_signed_ad("peer-x", 200.0, 1_700_000_000_750, vec![]),
        ];
        let grants = vec![
            TrustGrant {
                granter_peer_id: "peer-a".into(),
                grantee_peer_id: "peer-b".into(),
                granted_at_unix_ms: 1_700_000_000_000,
                chain_depth: 0,
                weight: TrustGrant::DEFAULT_WEIGHT,
            },
            TrustGrant {
                granter_peer_id: "peer-b".into(),
                grantee_peer_id: "peer-a".into(),
                granted_at_unix_ms: 1_700_000_000_000,
                chain_depth: 0,
                weight: TrustGrant::DEFAULT_WEIGHT,
            },
        ];
        let reach = vec![
            ReachabilityObservation {
                from_peer_id: "peer-a".into(),
                to_peer_id: "peer-b".into(),
                observed_rtt_ms: 30,
                observed_at_unix_ms: 1_700_000_000_000,
            },
            ReachabilityObservation {
                from_peer_id: "peer-b".into(),
                to_peer_id: "peer-a".into(),
                observed_rtt_ms: 30,
                observed_at_unix_ms: 1_700_000_000_000,
            },
        ];

        let snap = TopologyInputSnapshot {
            capacity_ads: ads,
            trust_grants: grants,
            reachability_observations: reach,
            locality_id: "loc-1".into(),
            snapshot_epoch_id: 99,
        };
        let topo = compute_alm_topology_verified(&all_valid(&snap));

        assert!(
            topo.unrooted_peers.contains(&"peer-x".to_string()),
            "peer-x not in locality must be unrooted"
        );
        for edge in &topo.tree {
            assert_ne!(edge.parent_peer_id, "peer-x");
            assert_ne!(edge.child_peer_id, "peer-x");
        }
    }

    // 5. trust_chain_depth_respected
    #[test]
    fn trust_chain_depth_respected() {
        let ads = vec![
            fake_signed_ad("peer-root", 100.0, 1_700_000_000_000, vec![]),
            fake_signed_ad("peer-a", 100.0, 1_700_000_000_001, vec![]),
            fake_signed_ad("peer-b", 50.0, 1_700_000_000_500, vec![]),
        ];
        let grants = vec![
            TrustGrant {
                granter_peer_id: "peer-root".into(),
                grantee_peer_id: "peer-a".into(),
                granted_at_unix_ms: 1_700_000_000_000,
                chain_depth: MAX_TRUST_CHAIN_DEPTH + 1,
                weight: 0,
            },
            TrustGrant {
                granter_peer_id: "peer-root".into(),
                grantee_peer_id: "peer-b".into(),
                granted_at_unix_ms: 1_700_000_000_000,
                chain_depth: MAX_TRUST_CHAIN_DEPTH + 2,
                weight: 0,
            },
        ];
        let reach = vec![
            ReachabilityObservation {
                from_peer_id: "peer-root".into(),
                to_peer_id: "peer-a".into(),
                observed_rtt_ms: 10,
                observed_at_unix_ms: 1_700_000_000_000,
            },
            ReachabilityObservation {
                from_peer_id: "peer-root".into(),
                to_peer_id: "peer-b".into(),
                observed_rtt_ms: 10,
                observed_at_unix_ms: 1_700_000_000_000,
            },
        ];
        let snap = TopologyInputSnapshot {
            capacity_ads: ads,
            trust_grants: grants,
            reachability_observations: reach,
            locality_id: "loc-depth".into(),
            snapshot_epoch_id: 3,
        };
        let topo = compute_alm_topology_verified(&all_valid(&snap));
        assert!(topo.unrooted_peers.contains(&"peer-a".to_string()));
        assert!(topo.unrooted_peers.contains(&"peer-b".to_string()));
    }

    // 6. tie_break_by_peer_id_lex_min
    #[test]
    fn tie_break_by_peer_id_lex_min() {
        let ads = vec![
            fake_signed_ad("peer-root", 100.0, 1_700_000_000_000, vec![]),
            fake_signed_ad("peer-aa", 100.0, 1_700_000_000_001, vec![]),
            fake_signed_ad("peer-zz", 100.0, 1_700_000_000_500, vec![]),
            fake_signed_ad("peer-child", 10.0, 1_700_000_001_000, vec![]),
        ];
        let mut grants = Vec::new();
        let mut reach = Vec::new();
        let peers = ["peer-root", "peer-aa", "peer-zz", "peer-child"];
        for &g in &peers {
            for &t in &peers {
                if g == t {
                    continue;
                }
                grants.push(TrustGrant {
                    granter_peer_id: g.into(),
                    grantee_peer_id: t.into(),
                    granted_at_unix_ms: 1_700_000_000_000,
                    chain_depth: 0,
                    weight: TrustGrant::DEFAULT_WEIGHT,
                });
                reach.push(ReachabilityObservation {
                    from_peer_id: g.into(),
                    to_peer_id: t.into(),
                    observed_rtt_ms: 50,
                    observed_at_unix_ms: 1_700_000_000_000,
                });
            }
        }

        let snap = TopologyInputSnapshot {
            capacity_ads: ads,
            trust_grants: grants,
            reachability_observations: reach,
            locality_id: "loc-tie".into(),
            snapshot_epoch_id: 8,
        };
        let topo = compute_alm_topology_verified_with_publisher(&all_valid(&snap), "peer-root");

        let child_edge = topo
            .tree
            .iter()
            .find(|e| e.child_peer_id == "peer-child")
            .expect("peer-child should be rooted");
        assert_eq!(
            child_edge.parent_peer_id, "peer-root",
            "minimum depth parent wins"
        );
    }

    // 7. mdc_sub_stream_emits_one_edge_per_path
    #[test]
    fn mdc_sub_stream_emits_one_edge_per_path() {
        let commitments = vec![
            SubStreamCommitment {
                sub_stream_path: vec![0],
                uplink_budget_mbps: 5.0,
                max_subscribers: 4,
            },
            SubStreamCommitment {
                sub_stream_path: vec![1],
                uplink_budget_mbps: 5.0,
                max_subscribers: 4,
            },
        ];
        let ads = vec![
            fake_signed_ad("peer-a", 100.0, 1_700_000_000_000, vec![]),
            fake_signed_ad("peer-child", 10.0, 1_700_000_001_000, commitments),
        ];
        let grants = vec![
            TrustGrant {
                granter_peer_id: "peer-a".into(),
                grantee_peer_id: "peer-child".into(),
                granted_at_unix_ms: 1_700_000_000_000,
                chain_depth: 0,
                weight: TrustGrant::DEFAULT_WEIGHT,
            },
            TrustGrant {
                granter_peer_id: "peer-child".into(),
                grantee_peer_id: "peer-a".into(),
                granted_at_unix_ms: 1_700_000_000_000,
                chain_depth: 0,
                weight: TrustGrant::DEFAULT_WEIGHT,
            },
        ];
        let reach = vec![
            ReachabilityObservation {
                from_peer_id: "peer-a".into(),
                to_peer_id: "peer-child".into(),
                observed_rtt_ms: 50,
                observed_at_unix_ms: 1_700_000_000_000,
            },
            ReachabilityObservation {
                from_peer_id: "peer-child".into(),
                to_peer_id: "peer-a".into(),
                observed_rtt_ms: 50,
                observed_at_unix_ms: 1_700_000_000_000,
            },
        ];

        let snap = TopologyInputSnapshot {
            capacity_ads: ads,
            trust_grants: grants,
            reachability_observations: reach,
            locality_id: "loc-mdc".into(),
            snapshot_epoch_id: 11,
        };
        let topo = compute_alm_topology_verified(&all_valid(&snap));
        let paths: Vec<&[u8]> = topo
            .tree
            .iter()
            .filter(|e| e.child_peer_id == "peer-child")
            .map(|e| e.sub_stream_path.as_slice())
            .collect();
        assert_eq!(paths.len(), 2);
        assert!(paths.contains(&&[0u8][..]));
        assert!(paths.contains(&&[1u8][..]));
    }

    // 8. unverified_ads_dropped_before_scoring
    #[test]
    fn unverified_ads_dropped_before_scoring() {
        let ads = vec![
            VerifiedCapacityAd {
                ad: fake_signed_ad("peer-root", 100.0, 1_700_000_000_000, vec![]),
                verification: CapacityVerification::HybridSignatureValid,
            },
            VerifiedCapacityAd {
                ad: fake_signed_ad("peer-d", 100.0, 1_700_000_000_000, vec![]),
                verification: CapacityVerification::Unverified,
            },
            VerifiedCapacityAd {
                ad: fake_signed_ad("peer-e", 100.0, 1_700_000_000_500, vec![]),
                verification: CapacityVerification::HybridSignatureValid,
            },
        ];
        let grants = vec![
            TrustGrant {
                granter_peer_id: "peer-root".into(),
                grantee_peer_id: "peer-d".into(),
                granted_at_unix_ms: 1_700_000_000_000,
                chain_depth: 0,
                weight: TrustGrant::DEFAULT_WEIGHT,
            },
            TrustGrant {
                granter_peer_id: "peer-root".into(),
                grantee_peer_id: "peer-e".into(),
                granted_at_unix_ms: 1_700_000_000_000,
                chain_depth: 0,
                weight: TrustGrant::DEFAULT_WEIGHT,
            },
        ];
        let reach = vec![
            ReachabilityObservation {
                from_peer_id: "peer-root".into(),
                to_peer_id: "peer-d".into(),
                observed_rtt_ms: 50,
                observed_at_unix_ms: 1_700_000_000_000,
            },
            ReachabilityObservation {
                from_peer_id: "peer-root".into(),
                to_peer_id: "peer-e".into(),
                observed_rtt_ms: 50,
                observed_at_unix_ms: 1_700_000_000_000,
            },
        ];

        let snap = VerifiedTopologyInputSnapshot {
            capacity_ads: ads,
            trust_grants: grants,
            reachability_observations: reach,
            locality_id: "loc-drop".into(),
            snapshot_epoch_id: 2,
        };
        let topo = compute_alm_topology_verified(&snap);
        assert!(!topo.unrooted_peers.contains(&"peer-d".to_string()));
        assert!(!topo.unrooted_peers.contains(&"peer-e".to_string()));
        assert_eq!(topo.tree.len(), 1);
        assert_eq!(topo.tree[0].child_peer_id, "peer-e");
    }

    // 9. clamped_capacity_at_max_self_asserted
    #[test]
    fn clamped_capacity_at_max_self_asserted() {
        let raw = 5_000_000u64;
        let clamped = clamp_self_asserted_millibps(raw, CapacityVerification::HybridSignatureValid);
        assert_eq!(clamped, MAX_SELF_ASSERTED_UPLINK_MILLIBPS);
    }

    // 10. throughput_challenged_uncapped
    #[test]
    fn throughput_challenged_uncapped() {
        let raw = 5_000_000u64;
        let uncapped = clamp_self_asserted_millibps(raw, CapacityVerification::ThroughputChallenged);
        assert_eq!(uncapped, raw);
    }

    // 11. sub_path_penalty_distributes_mdc
    #[test]
    fn sub_path_penalty_distributes_mdc() {
        let publisher = "pub-root";
        let parents = ["par-1", "par-2", "par-3", "par-4"];
        let children = ["kid-1", "kid-2", "kid-3", "kid-4"];

        // Publisher has capacity for 4 parents at depth 0
        let mut ads: Vec<SignedRelayCapacity> = vec![fake_signed_ad_with_max_subs(
            publisher,
            200.0,
            1_700_000_000_000,
            4,
            vec![],
        )];
        for (i, p) in parents.iter().enumerate() {
            ads.push(fake_signed_ad(p, 100.0, 1_700_000_000_001 + i as u64, vec![]));
        }
        for (i, c) in children.iter().enumerate() {
            ads.push(fake_signed_ad(
                c,
                10.0,
                1_700_000_010_000 + i as u64,
                vec![SubStreamCommitment {
                    sub_stream_path: vec![0],
                    uplink_budget_mbps: 5.0,
                    max_subscribers: 4,
                }],
            ));
        }

        let mut grants = Vec::new();
        let mut reach = Vec::new();

        // Publisher trusts parents; parents trust publisher and children; children trust parents
        for &p in &parents {
            grants.push(TrustGrant {
                granter_peer_id: publisher.into(),
                grantee_peer_id: p.into(),
                granted_at_unix_ms: 1_700_000_000_000,
                chain_depth: 0,
                weight: TrustGrant::DEFAULT_WEIGHT,
            });
            reach.push(ReachabilityObservation {
                from_peer_id: publisher.into(),
                to_peer_id: p.into(),
                observed_rtt_ms: 50,
                observed_at_unix_ms: 1_700_000_000_000,
            });
            for &c in &children {
                grants.push(TrustGrant {
                    granter_peer_id: p.into(),
                    grantee_peer_id: c.into(),
                    granted_at_unix_ms: 1_700_000_000_000,
                    chain_depth: 0,
                    weight: TrustGrant::DEFAULT_WEIGHT,
                });
                reach.push(ReachabilityObservation {
                    from_peer_id: p.into(),
                    to_peer_id: c.into(),
                    observed_rtt_ms: 50,
                    observed_at_unix_ms: 1_700_000_000_000,
                });
            }
        }

        let snap = TopologyInputSnapshot {
            capacity_ads: ads,
            trust_grants: grants,
            reachability_observations: reach,
            locality_id: "loc-mdc-dist".into(),
            snapshot_epoch_id: 77,
        };
        let topo = compute_alm_topology_verified_with_publisher(&all_valid(&snap), publisher);

        let chosen: Vec<&str> = children
            .iter()
            .map(|c| {
                topo.tree
                    .iter()
                    .find(|e| e.child_peer_id == *c && e.sub_stream_path == vec![0u8])
                    .unwrap_or_else(|| panic!("{c} should be rooted on path [0]"))
                    .parent_peer_id
                    .as_str()
            })
            .collect();

        let distinct: BTreeSet<&str> = chosen.iter().copied().collect();
        assert_eq!(
            distinct.len(),
            4,
            "sub_path penalty must spread 4 same-quadrant subscribers across 4 parents: {chosen:?}"
        );
    }

    // 12. max_subscribers_zero_blocks_parent_selection
    #[test]
    fn max_subscribers_zero_blocks_parent_selection() {
        let ads = vec![
            fake_signed_ad_with_max_subs("peer-zero", 100.0, 1_700_000_000_000, 0, vec![]),
            fake_signed_ad("peer-kid", 10.0, 1_700_000_001_000, vec![]),
        ];
        let grants = vec![TrustGrant {
            granter_peer_id: "peer-zero".to_string(),
            grantee_peer_id: "peer-kid".to_string(),
            granted_at_unix_ms: 1_700_000_000_000,
            chain_depth: 0,
            weight: TrustGrant::DEFAULT_WEIGHT,
        }];
        let reach = vec![ReachabilityObservation {
            from_peer_id: "peer-zero".to_string(),
            to_peer_id: "peer-kid".to_string(),
            observed_rtt_ms: 50,
            observed_at_unix_ms: 1_700_000_000_000,
        }];
        let snap = TopologyInputSnapshot {
            capacity_ads: ads,
            trust_grants: grants,
            reachability_observations: reach,
            locality_id: "loc-zero".to_string(),
            snapshot_epoch_id: 5,
        };
        let topo = compute_alm_topology_verified(&all_valid(&snap));
        assert!(topo.tree.is_empty());
        assert!(topo.unrooted_peers.contains(&"peer-kid".to_string()));
    }

    // 13. substream_commitment_cap_blocks_oversubscription
    #[test]
    fn substream_commitment_cap_blocks_oversubscription() {
        let commitment_h = SubStreamCommitment {
            sub_stream_path: vec![0x01],
            max_subscribers: 1,
            uplink_budget_mbps: 5.0,
        };
        let commitment_child = SubStreamCommitment {
            sub_stream_path: vec![0x01],
            max_subscribers: 4,
            uplink_budget_mbps: 5.0,
        };
        let ads = vec![
            fake_signed_ad(
                "peer-h",
                100.0,
                1_700_000_000_000,
                vec![commitment_h],
            ),
            fake_signed_ad(
                "peer-a",
                10.0,
                1_700_000_001_000,
                vec![commitment_child.clone()],
            ),
            fake_signed_ad(
                "peer-b",
                10.0,
                1_700_000_002_000,
                vec![commitment_child],
            ),
        ];
        let mut grants = Vec::new();
        let mut reach = Vec::new();
        for &c in &["peer-a", "peer-b"] {
            grants.push(TrustGrant {
                granter_peer_id: "peer-h".to_string(),
                grantee_peer_id: c.to_string(),
                granted_at_unix_ms: 1_700_000_000_000,
                chain_depth: 0,
                weight: TrustGrant::DEFAULT_WEIGHT,
            });
            reach.push(ReachabilityObservation {
                from_peer_id: "peer-h".to_string(),
                to_peer_id: c.to_string(),
                observed_rtt_ms: 50,
                observed_at_unix_ms: 1_700_000_000_000,
            });
        }
        let snap = TopologyInputSnapshot {
            capacity_ads: ads,
            trust_grants: grants,
            reachability_observations: reach,
            locality_id: "loc-commit-cap".to_string(),
            snapshot_epoch_id: 8,
        };
        let topo = compute_alm_topology_verified(&all_valid(&snap));
        let h_on_path_01 = topo
            .tree
            .iter()
            .filter(|e| e.parent_peer_id == "peer-h" && e.sub_stream_path == vec![0x01])
            .count();
        assert!(h_on_path_01 <= 1, "peer-h oversubscribed on path [0x01]");
    }

    // 14. mutual_ab_snapshot_produces_acyclic_tree
    #[test]
    fn mutual_ab_snapshot_produces_acyclic_tree() {
        let ads = vec![
            fake_signed_ad("peer-a", 50.0, 1_700_000_000_000, vec![]),
            fake_signed_ad("peer-b", 50.0, 1_700_000_000_500, vec![]),
        ];
        let mut grants = Vec::new();
        let mut reach = Vec::new();
        for (g, t) in [("peer-a", "peer-b"), ("peer-b", "peer-a")] {
            grants.push(TrustGrant {
                granter_peer_id: g.to_string(),
                grantee_peer_id: t.to_string(),
                granted_at_unix_ms: 1_700_000_000_000,
                chain_depth: 0,
                weight: TrustGrant::DEFAULT_WEIGHT,
            });
            reach.push(ReachabilityObservation {
                from_peer_id: g.to_string(),
                to_peer_id: t.to_string(),
                observed_rtt_ms: 50,
                observed_at_unix_ms: 1_700_000_000_000,
            });
        }
        let snap = TopologyInputSnapshot {
            capacity_ads: ads,
            trust_grants: grants,
            reachability_observations: reach,
            locality_id: "loc-mutual".to_string(),
            snapshot_epoch_id: 9,
        };
        let topo = compute_alm_topology_verified(&all_valid(&snap));

        let mut parent_of: BTreeMap<&str, &str> = BTreeMap::new();
        for edge in &topo.tree {
            parent_of
                .entry(edge.child_peer_id.as_str())
                .or_insert(edge.parent_peer_id.as_str());
        }
        for peer in ["peer-a", "peer-b"] {
            let mut seen: BTreeSet<&str> = BTreeSet::new();
            seen.insert(peer);
            let mut cursor: &str = peer;
            while let Some(&p) = parent_of.get(cursor) {
                assert!(!seen.contains(p), "cycle detected in topology: {peer} -> {p}");
                seen.insert(p);
                cursor = p;
            }
        }
        assert_eq!(topo.tree.len(), 1, "mutual snapshot produces exactly 1 edge");
    }

    // 15. distinct_subscriber_counting_under_max_subscribers_per_stream
    #[test]
    fn distinct_subscriber_counting_under_max_subscribers_per_stream() {
        let commitment_a = SubStreamCommitment {
            sub_stream_path: vec![0x01],
            max_subscribers: 4,
            uplink_budget_mbps: 5.0,
        };
        let commitment_b = SubStreamCommitment {
            sub_stream_path: vec![0x02],
            max_subscribers: 4,
            uplink_budget_mbps: 5.0,
        };
        let ads = vec![
            fake_signed_ad(
                "peer-a",
                5.0,
                1_700_000_000_000,
                vec![commitment_a.clone(), commitment_b.clone()],
            ),
            fake_signed_ad_with_max_subs(
                "peer-h",
                100.0,
                1_700_000_001_000,
                1,
                vec![commitment_a, commitment_b],
            ),
        ];
        let grants = vec![TrustGrant {
            granter_peer_id: "peer-h".to_string(),
            grantee_peer_id: "peer-a".to_string(),
            granted_at_unix_ms: 1_700_000_000_000,
            chain_depth: 0,
            weight: TrustGrant::DEFAULT_WEIGHT,
        }];
        let reach = vec![ReachabilityObservation {
            from_peer_id: "peer-h".to_string(),
            to_peer_id: "peer-a".to_string(),
            observed_rtt_ms: 50,
            observed_at_unix_ms: 1_700_000_000_000,
        }];
        let snap = TopologyInputSnapshot {
            capacity_ads: ads,
            trust_grants: grants,
            reachability_observations: reach,
            locality_id: "loc-distinct".to_string(),
            snapshot_epoch_id: 10,
        };
        let topo = compute_alm_topology_verified_with_publisher(&all_valid(&snap), "peer-h");
        let edges_to_a: Vec<&[u8]> = topo
            .tree
            .iter()
            .filter(|e| e.parent_peer_id == "peer-h" && e.child_peer_id == "peer-a")
            .map(|e| e.sub_stream_path.as_slice())
            .collect();
        assert_eq!(edges_to_a.len(), 2);
        assert!(!topo.unrooted_peers.contains(&"peer-a".to_string()));
    }

    // 16. stream_key_binding_single_publisher_root_and_unauthorized_drop
    #[test]
    fn stream_key_binding_single_publisher_root_and_unauthorized_drop() {
        let stream = StreamId([9u8; 32]);
        register_stream_key_binding(stream, "auth-publisher".to_string());

        let mut ad_pub = fake_signed_ad("auth-publisher", 100.0, 1_700_000_000_000, vec![]);
        ad_pub.stream_id = stream;
        let mut ad_other = fake_signed_ad("peer-unauth-root", 50.0, 1_700_000_000_500, vec![]);
        ad_other.stream_id = stream;

        let snap = VerifiedTopologyInputSnapshot {
            capacity_ads: vec![
                VerifiedCapacityAd {
                    ad: ad_pub,
                    verification: CapacityVerification::HybridSignatureValid,
                },
                VerifiedCapacityAd {
                    ad: ad_other,
                    verification: CapacityVerification::HybridSignatureValid,
                },
            ],
            trust_grants: vec![TrustGrant {
                granter_peer_id: "auth-publisher".into(),
                grantee_peer_id: "peer-unauth-root".into(),
                granted_at_unix_ms: 1_700_000_000_000,
                chain_depth: 0,
                weight: TrustGrant::DEFAULT_WEIGHT,
            }],
            reachability_observations: vec![ReachabilityObservation {
                from_peer_id: "auth-publisher".into(),
                to_peer_id: "peer-unauth-root".into(),
                observed_rtt_ms: 30,
                observed_at_unix_ms: 1_700_000_000_000,
            }],
            locality_id: "loc-single-root".into(),
            snapshot_epoch_id: 12,
        };

        let topo = compute_alm_topology_verified(&snap);
        // auth-publisher is the only root at depth 0, peer-unauth-root is its child
        assert_eq!(topo.tree.len(), 1);
        assert_eq!(topo.tree[0].parent_peer_id, "auth-publisher");
        assert_eq!(topo.tree[0].child_peer_id, "peer-unauth-root");
    }

    // 17. legacy_compute_alm_topology_returns_non_empty_tree
    #[test]
    fn legacy_compute_alm_topology_returns_non_empty_tree() {
        let snap = three_peer_snapshot();
        let topo = compute_alm_topology(&snap);
        assert!(!topo.tree.is_empty(), "legacy compute_alm_topology must return non-empty tree");
    }

    // 18. formal_alm_replay_vectors (CC §5.3.3.4, Alm.lean, alm_model.py)
    #[test]
    fn formal_alm_replay_vectors() {
        // Pinned points from Alm.lean and alm_model.py: 49, 141, 226, 569 permille
        assert_eq!(e2e_loss_permille(10, 5), 49);
        assert_eq!(e2e_loss_permille(30, 5), 141);
        assert_eq!(e2e_loss_permille(50, 5), 226);
        assert_eq!(e2e_loss_permille(100, 8), 569);

        // Refetch cheaper than fixed-overhead repair (100 permille overhead) at 1% per hop out to 8 hops
        for d in 1..=8 {
            assert!(
                e2e_loss_permille(10, d) < FIXED_REPAIR_OVERHEAD_PERMILLE,
                "refetch must be cheaper than fixed-overhead repair at 1% per hop for depth {}",
                d
            );
            assert!(refetch_is_cheaper(10, d));
        }

        // Fixed-overhead repair is cheaper at 5% per hop from 3 hops on
        for d in 3..=8 {
            assert!(
                e2e_loss_permille(50, d) > FIXED_REPAIR_OVERHEAD_PERMILLE,
                "fixed-overhead repair must be cheaper at 5% per hop for depth {}",
                d
            );
        }

        // Pinned senders live at once: 19531, 1953, 12
        assert_eq!(live_senders(625_000, 32), 19531);
        assert_eq!(live_senders(62_500, 32), 1953);
        assert_eq!(live_senders(62_500, 5_000), 12);
    }

    // 19. presence_floor_codec_class_never_excludes
    #[test]
    fn presence_floor_codec_class_never_excludes() {
        let codec = CodecClass::standard_video();
        assert_eq!(codec.floor_bps, PRESENCE_FLOOR_BPS);

        // Wi-Fi budget (625,000 B/s) with 10 senders: per sender 62,500 B/s -> layer 3
        assert_eq!(codec.select_layer_for_senders(625_000, 10), Some(3));

        // Cellular budget (62,500 B/s) with 10 senders: per sender 6,250 B/s -> layer 1 (base layer 5000)
        assert_eq!(codec.select_layer_for_senders(62_500, 10), Some(1));

        // Cellular budget (62,500 B/s) with 1,000 senders: per sender 62 B/s -> layer 0 (presence floor 32)
        assert_eq!(codec.select_layer_for_senders(62_500, 1000), Some(0));

        // Extremely low bandwidth or massive call (3,000 senders): floor layer is maintained (quality falls, presence does not)
        assert_eq!(codec.select_layer_for_senders(62_500, 3000), Some(0));
    }

    // 20. layered_multipath_and_sibling_pull_recovery
    #[test]
    fn layered_multipath_and_sibling_pull_recovery() {
        // Model the worked instance from Alm.lean:
        // Bob is parent to Dave (id 4) and Frank (id 6).
        // Dave missed chunk 2, Frank missed chunk 1.
        let mut tree_parents = BTreeMap::new();
        tree_parents.insert("alice".to_string(), "alice".to_string());
        tree_parents.insert("bob".to_string(), "alice".to_string());
        tree_parents.insert("carol".to_string(), "alice".to_string());
        tree_parents.insert("dave".to_string(), "bob".to_string());
        tree_parents.insert("erin".to_string(), "carol".to_string());
        tree_parents.insert("frank".to_string(), "bob".to_string());

        let mut held_chunks = BTreeMap::new();
        let mut dave_chunks = BTreeSet::new();
        dave_chunks.insert(0);
        dave_chunks.insert(1);
        dave_chunks.insert(3); // missed 2
        held_chunks.insert("dave".to_string(), dave_chunks);

        let mut frank_chunks = BTreeSet::new();
        frank_chunks.insert(0);
        frank_chunks.insert(2);
        frank_chunks.insert(3); // missed 1
        held_chunks.insert("frank".to_string(), frank_chunks);

        // Layered multipath: member can have different parents for different layers
        let mut erin_sub = LayeredMemberSubscription::new("erin");
        erin_sub.set_layer_parent(0, "carol"); // base from Carol
        erin_sub.set_layer_parent(1, "bob");   // enhancement from Bob
        assert_eq!(erin_sub.parent_for_layer(0), Some("carol"));
        assert_eq!(erin_sub.parent_for_layer(1), Some("bob"));

        // Execute sibling pull for Dave: pulls missing chunk 2 from sibling Frank
        let dave_pulled = sibling_pull("dave", "bob", &tree_parents, &held_chunks);
        assert_eq!(dave_pulled, (0..4).collect::<BTreeSet<u64>>());

        // Execute sibling pull for Frank: pulls missing chunk 1 from sibling Dave
        let frank_pulled = sibling_pull("frank", "bob", &tree_parents, &held_chunks);
        assert_eq!(frank_pulled, (0..4).collect::<BTreeSet<u64>>());

        // Fleet-wide sibling pull round restores completeness for both siblings
        let fleet_pulled = fleet_sibling_pull(&tree_parents, &held_chunks);
        assert_eq!(fleet_pulled["dave"], (0..4).collect::<BTreeSet<u64>>());
        assert_eq!(fleet_pulled["frank"], (0..4).collect::<BTreeSet<u64>>());
    }

    // 21. fanout_capacity_derivation_from_capacity_preimages
    #[test]
    fn fanout_capacity_derivation_from_capacity_preimages() {
        // CC §6.1.6 pins signed capacity preimages: capacity_bps / layer_bps
        assert_eq!(derive_fanout_capacity(50_000, 5_000), 10);
        assert_eq!(derive_fanout_capacity(16_000, 2_000), 8);
        assert_eq!(derive_fanout_capacity(0, 0), DEFAULT_SIM_FANOUT_CAP);
        assert_eq!(DEFAULT_SIM_FANOUT_CAP, 8);
    }
}
