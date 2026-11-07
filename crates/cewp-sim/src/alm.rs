//! Deterministic Application-Layer Multicast (CC §6.1.6, CLM-deterministic-alm).
//!
//! Pure integer-only implementation of `compute_alm_topology_verified` from
//! `CIRISEdge/src/holonomic/deterministic_topology.rs`.
//!
//! Key properties:
//! - Pure function: byte-equal inputs => byte-equal outputs.
//! - ZERO floating-point operations in decision branches; quantizes `f32` Mbps to
//!   `u64` millibits-per-second (`f32_mbps_to_millibps_u64`).
//! - Enforces `SignedRelayCapacity` verification (`verified_signature`),
//!   self-asserted capacity clamping (`MAX_SELF_ASSERTED_UPLINK_MILLIBPS` / `MAX_SELF_ASSERTED_MILLIBPS`),
//!   depth-primary parent selection (v2: `depth ≈ ⌈log_k N⌉`),
//!   MDC sub-path diversity penalty (F-4 anti-concentration),
//!   cycle prevention via descendant closure, and per-distinct-subscriber caps.

use std::collections::{BTreeMap, BTreeSet};

/// Pinned topology wire version (v2 = degree-bounded minimum-depth breadth-first tree).
pub const TOPOLOGY_VERSION: u16 = 2;

/// Max RTT in milliseconds beyond which a peer-pair is treated as unreachable.
pub const MAX_USEFUL_RTT_MS: u32 = 5_000;

/// Maximum trust-chain depth explored for transitive trust grants.
pub const MAX_TRUST_CHAIN_DEPTH: u8 = 4;

/// Capacity weight in candidate-edge scoring sum.
pub const WEIGHT_CAPACITY: u64 = 50;

/// Trust weight in candidate-edge scoring sum.
pub const WEIGHT_TRUST: u64 = 30;

/// Reachability weight in candidate-edge scoring sum.
pub const WEIGHT_REACHABILITY: u64 = 20;

/// Cap on self-asserted uplink in Mbps (1000 Mbps = 1 Gbps).
pub const MAX_SELF_ASSERTED_UPLINK_MBPS: f32 = 1000.0;

/// Wire-determinism cap on self-asserted uplink in quantized millibps (1,000,000 millibps).
pub const MAX_SELF_ASSERTED_UPLINK_MILLIBPS: u64 = 1_000_000;

/// Alternative 50 Mbps ceiling in milli-bps (50,000,000 milli-bps) per FSD §3.2 clamp specifications.
pub const MAX_SELF_ASSERTED_MILLIBPS: u64 = 50_000_000;

/// Penalty subtracted from candidate parent score per duplicate sub-path subscriber.
pub const PENALTY_PER_SUB_PATH_DUP: u64 = 5;

/// Capacity normalizer constant in millibps (10,000,000 millibps).
pub const CAPACITY_NORMALIZER_MILLIBPS: u64 = 10_000_000;

/// Sub-stream dyadic path for MDC layers (empty = whole stream opaque).
pub type SubStreamPath = Vec<u8>;

/// 32-byte Stream identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StreamId(pub [u8; 32]);

/// Monotonic stream epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Epoch(pub u64);

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
            out.extend_from_slice(&(edge.child_peer_id.len() as u32).to_be_bytes());
            out.extend_from_slice(edge.child_peer_id.as_bytes());
            out.extend_from_slice(&(edge.parent_peer_id.len() as u32).to_be_bytes());
            out.extend_from_slice(edge.parent_peer_id.as_bytes());
            out.extend_from_slice(&(edge.sub_stream_path.len() as u32).to_be_bytes());
            out.extend_from_slice(&edge.sub_stream_path);
        }
        out.extend_from_slice(&(self.unrooted_peers.len() as u64).to_be_bytes());
        for p in &self.unrooted_peers {
            out.extend_from_slice(&(p.len() as u32).to_be_bytes());
            out.extend_from_slice(p.as_bytes());
        }
        out
    }
}

/// Quantize float Mbps to u64 milli-bps with defined truncation and non-negative clamping.
#[must_use]
pub fn f32_mbps_to_millibps_u64(mbps: f32) -> u64 {
    if !mbps.is_finite() || mbps <= 0.0 {
        return 0;
    }
    (mbps * 1_000.0) as u64
}

/// Apply verification-state dependent clamp to millibps.
#[must_use]
pub fn clamp_self_asserted_millibps(
    uplink_millibps: u64,
    verification: CapacityVerification,
) -> u64 {
    match verification {
        CapacityVerification::ThroughputChallenged => uplink_millibps,
        CapacityVerification::HybridSignatureValid => {
            uplink_millibps.min(MAX_SELF_ASSERTED_UPLINK_MILLIBPS)
        }
        CapacityVerification::Unverified => 0,
    }
}

/// Integer-only normalization functions.
#[must_use]
pub fn normalize_capacity_score(uplink_millibps: u64) -> u64 {
    let clamped = uplink_millibps.min(CAPACITY_NORMALIZER_MILLIBPS);
    clamped.saturating_mul(1_000) / CAPACITY_NORMALIZER_MILLIBPS
}

#[must_use]
pub fn normalize_trust_score(chain_depth: u8) -> u64 {
    if chain_depth > MAX_TRUST_CHAIN_DEPTH {
        return 0;
    }
    let remaining = u64::from(MAX_TRUST_CHAIN_DEPTH) - u64::from(chain_depth);
    remaining.saturating_mul(1_000) / u64::from(MAX_TRUST_CHAIN_DEPTH)
}

#[must_use]
pub fn normalize_reachability_score(rtt_ms: u32) -> u64 {
    let rtt = u64::from(rtt_ms).min(u64::from(MAX_USEFUL_RTT_MS));
    let remaining = u64::from(MAX_USEFUL_RTT_MS) - rtt;
    remaining.saturating_mul(1_000) / u64::from(MAX_USEFUL_RTT_MS)
}

fn canonical_sub_paths(child_ad: &SignedRelayCapacity) -> Vec<SubStreamPath> {
    let mut sub_paths: Vec<SubStreamPath> = if child_ad.capacity.sub_stream_commitments.is_empty() {
        vec![Vec::new()]
    } else {
        child_ad
            .capacity
            .sub_stream_commitments
            .iter()
            .map(|c| c.sub_stream_path.clone())
            .collect()
    };
    sub_paths.sort();
    sub_paths.dedup();
    sub_paths
}

fn descendants_of(
    root: &str,
    child_of: &BTreeMap<String, BTreeSet<String>>,
) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut stack: Vec<String> = vec![root.to_string()];
    while let Some(p) = stack.pop() {
        if let Some(children) = child_of.get(&p) {
            for c in children {
                if out.insert(c.clone()) {
                    stack.push(c.clone());
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
) -> Option<String> {
    let mut best_depth: Option<usize> = None;
    let mut best_score: Option<u64> = None;
    let mut best_parent: Option<String> = None;

    for parent_peer_id in candidate_peers {
        if parent_peer_id == child_peer_id {
            continue;
        }

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

        // Capacity cap gate
        let parent_ad = match latest_ad.get(parent_peer_id) {
            Some(v) => &v.ad.capacity,
            None => continue,
        };

        let total_at_parent = parent_total_occupancy.get(parent_peer_id).copied().unwrap_or(0);
        let already_subscribed = child_of
            .get(parent_peer_id)
            .is_some_and(|c| c.contains(child_peer_id));
        if !already_subscribed && total_at_parent >= u64::from(parent_ad.max_subscribers_per_stream) {
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

        // Depth primary key
        let depth = depth_of.get(parent_peer_id).copied().unwrap_or(usize::MAX);

        // Score secondary key
        let cap_millibps = parent_cap_millibps.get(parent_peer_id).copied().unwrap_or(0);
        let cap_s = normalize_capacity_score(cap_millibps);
        let trust_s = normalize_trust_score(trust_depth);
        let reach_s = normalize_reachability_score(rtt);

        let base = WEIGHT_CAPACITY
            .saturating_mul(cap_s)
            .saturating_add(WEIGHT_TRUST.saturating_mul(trust_s))
            .saturating_add(WEIGHT_REACHABILITY.saturating_mul(reach_s));

        let sub_path_dups = sub_path_occupancy
            .get(&(parent_peer_id.clone(), candidate_sub_path.to_vec()))
            .copied()
            .unwrap_or(0);
        let consumer_dups = this_child_parent_occ.get(parent_peer_id).copied().unwrap_or(0);
        let penalty = PENALTY_PER_SUB_PATH_DUP.saturating_mul(sub_path_dups.saturating_add(consumer_dups));
        let score = base.saturating_sub(penalty);

        let improves = match (best_depth, best_score) {
            (None, _) => true,
            (Some(bd), _) if depth < bd => true,
            (Some(bd), Some(bs)) if depth == bd && score > bs => true,
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

/// Pure deterministic ALM topology computation.
#[must_use]
pub fn compute_alm_topology(snapshot: &TopologyInputSnapshot) -> AlmTopology {
    let verified: Vec<VerifiedCapacityAd> = snapshot
        .capacity_ads
        .iter()
        .cloned()
        .map(|ad| VerifiedCapacityAd {
            ad,
            verification: CapacityVerification::Unverified,
        })
        .collect();
    compute_core(
        &verified,
        &snapshot.trust_grants,
        &snapshot.reachability_observations,
        snapshot.snapshot_epoch_id,
    )
}

/// Hardened deterministic ALM computation for verified snapshots.
#[must_use]
pub fn compute_alm_topology_verified(snapshot: &VerifiedTopologyInputSnapshot) -> AlmTopology {
    compute_core(
        &snapshot.capacity_ads,
        &snapshot.trust_grants,
        &snapshot.reachability_observations,
        snapshot.snapshot_epoch_id,
    )
}

fn compute_core(
    capacity_ads: &[VerifiedCapacityAd],
    trust_grants: &[TrustGrant],
    reachability_observations: &[ReachabilityObservation],
    snapshot_epoch_id: u64,
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

    let mut tree: Vec<AlmEdge> = Vec::new();
    let mut unrooted: Vec<String> = Vec::new();
    let mut sub_path_occupancy: BTreeMap<(String, SubStreamPath), u64> = BTreeMap::new();
    let mut parent_total_occupancy: BTreeMap<String, u64> = BTreeMap::new();
    let mut child_of: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    let mut has_eligible_parent: BTreeSet<String> = BTreeSet::new();
    for (granter, grantee) in trust_min_depth.keys() {
        if granter == grantee
            || !latest_ad.contains_key(granter)
            || !latest_ad.contains_key(grantee)
        {
            continue;
        }
        if reach_min_rtt
            .get(&(granter.clone(), grantee.clone()))
            .is_some_and(|rtt| *rtt <= MAX_USEFUL_RTT_MS)
        {
            has_eligible_parent.insert(grantee.clone());
        }
    }
    let mut depth_of: BTreeMap<String, usize> = BTreeMap::new();
    for peer in &candidate_peers {
        if !has_eligible_parent.contains(peer) {
            depth_of.insert(peer.clone(), 0);
        }
    }

    for child_peer_id in &candidate_peers {
        let child_ad = &latest_ad[child_peer_id].ad;
        let sub_paths = canonical_sub_paths(child_ad);
        let descendants = descendants_of(child_peer_id, &child_of);
        let mut this_child_parent_occ: BTreeMap<String, u64> = BTreeMap::new();

        let mut any_rooted_for_child = false;
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
            ) {
                *this_child_parent_occ.entry(parent.clone()).or_insert(0) += 1;
                let child_depth = depth_of
                    .get(&parent)
                    .copied()
                    .unwrap_or(usize::MAX)
                    .saturating_add(1);
                depth_of
                    .entry(child_peer_id.clone())
                    .and_modify(|d| *d = (*d).min(child_depth))
                    .or_insert(child_depth);
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
                any_rooted_for_child = true;
            }
        }

        if !any_rooted_for_child {
            unrooted.push(child_peer_id.clone());
        }
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
        assert!(!a.tree.is_empty(), "three rooted peers expected");
    }

    // 2. wire_determinism_under_permutation
    #[test]
    fn wire_determinism_under_permutation() {
        let base = three_peer_snapshot();
        let baseline = compute_alm_topology_verified(&all_valid(&base));

        let mut p1 = base.clone();
        p1.capacity_ads.reverse();
        assert_eq!(
            compute_alm_topology_verified(&all_valid(&p1)),
            baseline,
            "ad reverse permutation must not change output"
        );

        let mut p2 = base.clone();
        p2.capacity_ads.rotate_left(1);
        assert_eq!(
            compute_alm_topology_verified(&all_valid(&p2)),
            baseline,
            "ad rotate permutation must not change output"
        );

        let mut p3 = base.clone();
        p3.reachability_observations.reverse();
        assert_eq!(
            compute_alm_topology_verified(&all_valid(&p3)),
            baseline,
            "reachability permutation must not change output"
        );

        let mut p4 = base.clone();
        p4.trust_grants.reverse();
        assert_eq!(
            compute_alm_topology_verified(&all_valid(&p4)),
            baseline,
            "trust permutation must not change output"
        );

        let mut v = all_valid(&base);
        v.capacity_ads.reverse();
        assert_eq!(
            compute_alm_topology_verified(&v),
            baseline,
            "verified-ad reverse must not change output"
        );
    }

    // 3. empty_snapshot_returns_empty_topology
    #[test]
    fn empty_snapshot_returns_empty_topology() {
        let snap = VerifiedTopologyInputSnapshot {
            capacity_ads: vec![],
            trust_grants: vec![],
            reachability_observations: vec![],
            locality_id: "loc-empty".to_string(),
            snapshot_epoch_id: 7,
        };
        let topo = compute_alm_topology_verified(&snap);
        assert!(topo.tree.is_empty());
        assert!(topo.unrooted_peers.is_empty());
        assert_eq!(topo.snapshot_epoch_id, 7);
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
            fake_signed_ad("peer-a", 100.0, 1_700_000_000_000, vec![]),
            fake_signed_ad("peer-b", 50.0, 1_700_000_000_500, vec![]),
        ];
        let grants = vec![TrustGrant {
            granter_peer_id: "peer-a".into(),
            grantee_peer_id: "peer-b".into(),
            granted_at_unix_ms: 1_700_000_000_000,
            chain_depth: MAX_TRUST_CHAIN_DEPTH + 1,
            weight: TrustGrant::DEFAULT_WEIGHT,
        }];
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
            snapshot_epoch_id: 1,
        };
        let topo = compute_alm_topology_verified(&all_valid(&snap));
        assert!(topo.tree.is_empty(), "no trust within horizon => no edges");
        assert!(topo.unrooted_peers.contains(&"peer-a".to_string()));
        assert!(topo.unrooted_peers.contains(&"peer-b".to_string()));
    }

    // 6. tie_break_by_peer_id_lex_min
    #[test]
    fn tie_break_by_peer_id_lex_min() {
        let ads = vec![
            fake_signed_ad("peer-aa", 100.0, 1_700_000_000_000, vec![]),
            fake_signed_ad("peer-zz", 100.0, 1_700_000_000_500, vec![]),
            fake_signed_ad("peer-child", 10.0, 1_700_000_001_000, vec![]),
        ];
        let mut grants = Vec::new();
        let mut reach = Vec::new();
        let peers = ["peer-aa", "peer-zz", "peer-child"];
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
        let topo = compute_alm_topology_verified(&all_valid(&snap));

        let child_edge = topo
            .tree
            .iter()
            .find(|e| e.child_peer_id == "peer-child")
            .expect("peer-child should be rooted");
        assert_eq!(
            child_edge.parent_peer_id, "peer-aa",
            "lex-min parent wins on tie"
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

        let child_edges: Vec<&AlmEdge> = topo
            .tree
            .iter()
            .filter(|e| e.child_peer_id == "peer-child")
            .collect();
        assert_eq!(child_edges.len(), 2);
        assert_eq!(child_edges[0].sub_stream_path, vec![0u8]);
        assert_eq!(child_edges[1].sub_stream_path, vec![1u8]);
    }

    // 8. unverified_ads_dropped_before_scoring
    #[test]
    fn unverified_ads_dropped_before_scoring() {
        let ads = vec![
            fake_signed_ad("peer-a", 100.0, 1_700_000_000_000, vec![]),
            fake_signed_ad("peer-b", 50.0, 1_700_000_000_500, vec![]),
            fake_signed_ad("peer-c", 10.0, 1_700_000_001_000, vec![]),
            fake_signed_ad("peer-d", 1_000.0, 1_700_000_001_500, vec![]),
            fake_signed_ad("peer-e", 1_000.0, 1_700_000_002_000, vec![]),
        ];

        let peers = ["peer-a", "peer-b", "peer-c", "peer-d", "peer-e"];
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

        let capacity_ads = ads
            .into_iter()
            .map(|ad| {
                let verification = if ["peer-d", "peer-e"].contains(&ad.advertiser_key_id.as_str()) {
                    CapacityVerification::Unverified
                } else {
                    CapacityVerification::HybridSignatureValid
                };
                VerifiedCapacityAd { ad, verification }
            })
            .collect();

        let snap = VerifiedTopologyInputSnapshot {
            capacity_ads,
            trust_grants: grants,
            reachability_observations: reach,
            locality_id: "loc-drop".into(),
            snapshot_epoch_id: 5,
        };
        let topo = compute_alm_topology_verified(&snap);

        for edge in &topo.tree {
            for id in [&edge.parent_peer_id, &edge.child_peer_id] {
                assert!(["peer-a", "peer-b", "peer-c"].contains(&id.as_str()));
            }
        }
        assert!(!topo.unrooted_peers.contains(&"peer-d".to_string()));
        assert!(!topo.unrooted_peers.contains(&"peer-e".to_string()));
    }

    // 9. clamped_capacity_at_max_self_asserted
    #[test]
    fn clamped_capacity_at_max_self_asserted() {
        let ten_gbps = f32_mbps_to_millibps_u64(10_000.0);
        assert_eq!(ten_gbps, 10_000_000);
        let clamped = clamp_self_asserted_millibps(ten_gbps, CapacityVerification::HybridSignatureValid);
        assert_eq!(clamped, 1_000_000, "10 Gbps self-asserted clamps to 1 Gbps");
        let honest = f32_mbps_to_millibps_u64(1000.0);
        assert_eq!(
            clamp_self_asserted_millibps(honest, CapacityVerification::HybridSignatureValid),
            clamped,
            "clamped liar ties an honest 1 Gbps peer"
        );
    }

    // 10. throughput_challenged_uncapped
    #[test]
    fn throughput_challenged_uncapped() {
        let ten_gbps = f32_mbps_to_millibps_u64(10_000.0);
        let challenged = clamp_self_asserted_millibps(ten_gbps, CapacityVerification::ThroughputChallenged);
        assert_eq!(challenged, 10_000_000, "throughput-challenged is uncapped");
        assert!(challenged > clamp_self_asserted_millibps(ten_gbps, CapacityVerification::HybridSignatureValid));
    }

    // 11. sub_path_penalty_distributes_mdc
    #[test]
    fn sub_path_penalty_distributes_mdc() {
        let parents = ["par-1", "par-2", "par-3", "par-4"];
        let children = ["kid-1", "kid-2", "kid-3", "kid-4"];

        let mut ads: Vec<SignedRelayCapacity> = parents
            .iter()
            .enumerate()
            .map(|(i, p)| fake_signed_ad(p, 100.0, 1_700_000_000_000 + i as u64, vec![]))
            .collect();
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
        let all: Vec<&str> = parents.iter().chain(children.iter()).copied().collect();
        for &g in &all {
            for &t in &all {
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
            locality_id: "loc-mdc-dist".into(),
            snapshot_epoch_id: 77,
        };
        let topo = compute_alm_topology_verified(&all_valid(&snap));

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
            fake_signed_ad_with_max_subs("peer-x", 1_000.0, 1_700_000_000_000, 0, vec![]),
            fake_signed_ad("peer-a", 10.0, 1_700_000_000_500, vec![]),
            fake_signed_ad("peer-b", 10.0, 1_700_000_001_000, vec![]),
        ];
        let mut grants = Vec::new();
        let mut reach = Vec::new();
        let peers = ["peer-x", "peer-a", "peer-b"];
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
            locality_id: "loc-zero".into(),
            snapshot_epoch_id: 1,
        };
        let topo = compute_alm_topology_verified(&all_valid(&snap));

        for edge in &topo.tree {
            assert_ne!(
                edge.parent_peer_id, "peer-x",
                "peer-x advertised max_subscribers_per_stream = 0 and must never be chosen as parent"
            );
        }
    }

    // 13. substream_commitment_cap_blocks_oversubscription
    #[test]
    fn substream_commitment_cap_blocks_oversubscription() {
        let commitment = SubStreamCommitment {
            sub_stream_path: vec![0x01],
            max_subscribers: 1,
            uplink_budget_mbps: 5.0,
        };
        let ads = vec![
            fake_signed_ad("peer-a", 5.0, 1_700_000_000_000, vec![commitment.clone()]),
            fake_signed_ad("peer-b", 5.0, 1_700_000_000_500, vec![commitment.clone()]),
            fake_signed_ad_with_max_subs("peer-h", 100.0, 1_700_000_001_000, 16, vec![commitment]),
        ];
        let mut grants = Vec::new();
        let mut reach = Vec::new();
        for &t in &["peer-a", "peer-b"] {
            grants.push(TrustGrant {
                granter_peer_id: "peer-h".to_string(),
                grantee_peer_id: t.to_string(),
                granted_at_unix_ms: 1_700_000_000_000,
                chain_depth: 0,
                weight: TrustGrant::DEFAULT_WEIGHT,
            });
            reach.push(ReachabilityObservation {
                from_peer_id: "peer-h".to_string(),
                to_peer_id: t.to_string(),
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
        let topo = compute_alm_topology_verified(&all_valid(&snap));
        let edges_to_a: Vec<&[u8]> = topo
            .tree
            .iter()
            .filter(|e| e.parent_peer_id == "peer-h" && e.child_peer_id == "peer-a")
            .map(|e| e.sub_stream_path.as_slice())
            .collect();
        assert_eq!(edges_to_a.len(), 2);
        assert!(!topo.unrooted_peers.contains(&"peer-a".to_string()));
    }
}
