//! Whole-Chunk Storage, Layered Fidelity, and Noise-Floor Descent (CC §5.3.3.2.3, §5.4.3, §6.1.2, §6.1.5).
//!
//! Models:
//! - Whole-chunk storage contract (`ChunkManifestV1`, `WholeChunkManifestV1`): sealed chunks are stored
//!   whole (CC §5.4.3, §6.1.5) and attested by `custody:ack:v1`; `holds_bytes` is retired and refused (CC §3.1.3.3).
//! - Live A/V chunks (CC §5.3.3.2.3) travel whole and are never fragmented (unfragmented sealed whole-chunk envelope).
//! - Three constitutional noise-floor states: `Full`, `Partial`, `EnvelopeOnly` (with signed manifest header retention).
//! - Layered-codec fidelity classification (CC §6.1.2, CLM-noise-classify) over `ChunkLayer` sets `(R, ε)`
//!   (rate $R$ and distortion/reconstruction error $\varepsilon \le \varepsilon_{\text{max}}$).
//! - Prohibited categories zeroing envelope to content-hash (CC §6.1.2.2).
//! - Revocation dominance (CLM-noise-N5): `ConsentState::Revoked` -> `EjectionVerdict::EjectHardDelete` -> `EnvelopeOnly`.
//! - Target-holder proactive trimming at 15% safety margin: `should_eject_above_target`.
//! - Kish mass-dominance gate (`passes_dominance_gate`, `N_EFF_MIN = 3`) and R9 multiplicity gate
//!   (`passes_multiplicity_gate`, `R9_HOLDER_FLOOR = 9`) using integer floor over fixed-point masses (PPM), NEVER `f64::round()`.
//! - Unit tests for exact CIRISConformance ladder (test_540), test_541, and test_542.

use std::collections::BTreeSet;

pub use crate::transport::{
    validate_holding_claim_token, CUSTODY_ACK_V1_PREFIX, RETIRED_HOLDS_BYTES_PREFIX,
};

/// Safety margin percent above target holders before trimming occurs (15%).
pub const EJECT_ABOVE_TARGET_SAFETY_MARGIN_PCT: u32 = 15;

/// Default target holders in the holonomic federation.
pub const DEFAULT_TARGET_HOLDERS: u32 = 30;

/// Minimum effective sample size floor for Kish dominance gate (CC §6.1.2.1.2).
pub const N_EFF_MIN: u32 = 3;

/// Multiplicity holder floor for R9 content-multiplicity gate (CC §6.1.2.1.2 R9).
pub const R9_HOLDER_FLOOR: u32 = 9;

/// Fixed-point scale for mass calculations (parts per million: 1_000_000 = 1.0).
pub const PPM_SCALE: u64 = 1_000_000;



/// A single scalable / temporal codec layer in the layered fidelity model (CC §5.3.3.2.1, §6.1.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChunkLayer {
    pub spatial: u8,
    pub temporal: u8,
    pub quality: u8,
    pub bitrate_bps: u64,
    pub reconstruction_error_ppm: u64,
}

/// Signed chunk manifest header (V1 contract, CIRISPersist#227, CC §5.4.3, §6.1.5).
/// Chunks are stored whole; the signed header is retained forever even when payload descends below noise floor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkManifestV1 {
    pub content_id: String,
    pub corpus_kind: String,
    pub manifest_version: u16,
    pub original_content_length: u64,
    pub target_holders: u32,
    pub min_viable_holders: u32,
    pub layers: Vec<ChunkLayer>,
    pub envelope_header: String,
    pub signature_ed25519: String,
    pub signature_ml_dsa_65: String,
    pub pqc_key_id: String,
}

pub type WholeChunkManifestV1 = ChunkManifestV1;

impl ChunkManifestV1 {
    #[must_use]
    pub fn new(
        content_id: String,
        corpus_kind: String,
        target_holders: u32,
        min_viable_holders: u32,
        original_content_length: u64,
    ) -> Self {
        Self {
            content_id: content_id.clone(),
            corpus_kind,
            manifest_version: 1,
            original_content_length,
            target_holders,
            min_viable_holders,
            layers: vec![
                ChunkLayer {
                    spatial: 0,
                    temporal: 0,
                    quality: 0,
                    bitrate_bps: 40_000,
                    reconstruction_error_ppm: 50_000,
                },
            ],
            envelope_header: format!("signed-manifest-envelope-header-for-{content_id}"),
            signature_ed25519: "ed25519-sig".into(),
            signature_ml_dsa_65: "ml-dsa-65-sig".into(),
            pqc_key_id: "pqc-key-1".into(),
        }
    }
}

/// The three constitutional noise-floor states (CC §6.1.2, CLM-noise-classify).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NoiseFloorState {
    /// Lossless / full fidelity reconstruction (present >= target_holders).
    Full,
    /// Degraded layered-codec / summary recovery (min_viable_holders <= present < target_holders).
    Partial,
    /// Below noise floor: zero payload bytes recoverable, signed envelope header retained forever.
    EnvelopeOnly,
}

/// Prohibited categories under CC §6.1.2.2 that zero the envelope header to content-hash only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProhibitedCategory {
    NcmecCsam,
    PerceptualHashCsam,
    GifctCip,
    CourtOrder,
}

impl ProhibitedCategory {
    /// Zero an envelope header when content violates prohibited categories.
    #[must_use]
    pub fn sanitize_envelope_header(content_id: &str, _category: Self) -> String {
        format!("zeroed-content-hash-only:sha256:{content_id}")
    }
}

/// Classify a reachable whole-chunk replica count against manifest thresholds into its NoiseFloorState.
#[must_use]
pub fn classify(present: u32, target: u32, min_viable: u32) -> NoiseFloorState {
    if present >= target {
        NoiseFloorState::Full
    } else if present >= min_viable {
        NoiseFloorState::Partial
    } else {
        NoiseFloorState::EnvelopeOnly
    }
}

/// Classify layered-codec fidelity over ChunkLayer parameters (R, ε) per CC §6.1.2 (CLM-noise-classify).
#[must_use]
pub fn classify_fidelity(
    rate_bps: u64,
    min_rate_bps: u64,
    distortion_epsilon_ppm: u64,
    max_distortion_epsilon_ppm: u64,
) -> NoiseFloorState {
    if distortion_epsilon_ppm <= max_distortion_epsilon_ppm && rate_bps >= min_rate_bps {
        NoiseFloorState::Full
    } else if rate_bps >= (min_rate_bps / 4) {
        NoiseFloorState::Partial
    } else {
        NoiseFloorState::EnvelopeOnly
    }
}



/// Tracks exact per-node whole chunk holdings with custody attestation and adversarial modeling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeChunkHoldings {
    pub peer_id: String,
    pub held_chunks: BTreeSet<String>,
    /// Custody attestation via `custody:ack:v1` (CC §3.1.3.3).
    pub custody_ack: bool,
    /// Adversarial: claims custody while withholding the chunk.
    pub withholding: bool,
    /// Adversarial: serves corrupted chunk failing hash verification.
    pub corrupt: bool,
}

impl NodeChunkHoldings {
    #[must_use]
    pub fn new(peer_id: String, held_chunk_ids: &[&str]) -> Self {
        Self {
            peer_id,
            held_chunks: held_chunk_ids.iter().map(|&s| s.to_string()).collect(),
            custody_ack: true,
            withholding: false,
            corrupt: false,
        }
    }

    #[must_use]
    pub fn with_withholding(mut self, withholding: bool) -> Self {
        self.withholding = withholding;
        self
    }

    #[must_use]
    pub fn with_corrupt(mut self, corrupt: bool) -> Self {
        self.corrupt = corrupt;
        self
    }
}

/// Calculate the count of valid, non-corrupt, non-withholding distinct whole chunk holders.
#[must_use]
pub fn calculate_effective_present_chunks(
    holders: &[NodeChunkHoldings],
    content_id: &str,
) -> u32 {
    let mut count = 0u32;
    for holder in holders {
        if holder.withholding || holder.corrupt || !holder.custody_ack {
            continue;
        }
        if holder.held_chunks.contains(content_id) {
            count += 1;
        }
    }
    count
}

/// Consent state of a content item or data subject (CC §3.3.7, CC §6.1.2.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConsentState {
    /// Active consent: normal retention and rarity apply.
    Active,
    /// Revoked / Withdrawn: forces immediate descent to EnvelopeOnly under N5 hard-delete.
    Revoked,
    /// Unknown: default fail-secure (not retained as rare).
    Unknown,
}

/// Ejection verdict for tiered storage and proactive noise-floor descent (CC §6.1.2, §19.7.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EjectionVerdict {
    /// Keep item at current fidelity.
    Keep,
    /// Step down one tier under capacity pressure (intra-object layer shed or N->1 aggregation).
    EjectToTier,
    /// Shed exactly one intermediate pyramid stratum.
    EjectAggregatedTierOnly { tier: u32 },
    /// Forced hard delete below noise floor (N5 revocation); drops all chunk copies immediately.
    EjectHardDelete,
}

/// Targeted stratum ejection helper.
#[must_use]
pub fn eject_aggregated_tier(tier: u32) -> EjectionVerdict {
    EjectionVerdict::EjectAggregatedTierOnly { tier }
}

/// Resolve ejection verdict from consent state and capacity pressure.
/// Revocation forces immediate hard delete (N5), overriding rarity and pressure.
#[must_use]
pub fn ejection_verdict(consent: ConsentState, under_capacity_pressure: bool) -> EjectionVerdict {
    match consent {
        ConsentState::Revoked => EjectionVerdict::EjectHardDelete,
        ConsentState::Active | ConsentState::Unknown => {
            if under_capacity_pressure {
                EjectionVerdict::EjectToTier
            } else {
                EjectionVerdict::Keep
            }
        }
    }
}

/// Policy parameters governing whole-chunk replica density (CC §5.4.3, §6.1.5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkStoragePolicy {
    pub target_holders: u32,
}

pub type WholeChunkStoragePolicy = ChunkStoragePolicy;

impl Default for ChunkStoragePolicy {
    fn default() -> Self {
        Self {
            target_holders: DEFAULT_TARGET_HOLDERS,
        }
    }
}

/// Proactive trim primitive: determines if an over-replicated whole chunk should be trimmed locally.
///
/// Threshold: `target_holders + target_holders * 15 / 100` (e.g. 30 + 4 = 34).
/// If `holders_observed <= threshold` -> Keep.
/// If `holders_observed > threshold` -> EjectToTier if local chunk is common
/// (`local_chunk_rarity >= target_holders / 2`), else Keep if rare.
/// Revocation dominates everything -> EjectHardDelete.
#[must_use]
pub fn should_eject_above_target(
    holders_observed: u32,
    policy: &ChunkStoragePolicy,
    consent: ConsentState,
    local_chunk_rarity: u32,
) -> EjectionVerdict {
    if consent == ConsentState::Revoked {
        return EjectionVerdict::EjectHardDelete;
    }

    let safety = policy.target_holders * EJECT_ABOVE_TARGET_SAFETY_MARGIN_PCT / 100;
    let over_target_threshold = policy.target_holders + safety;

    if holders_observed <= over_target_threshold {
        return EjectionVerdict::Keep;
    }

    let common_threshold = policy.target_holders / 2;
    if local_chunk_rarity >= common_threshold {
        EjectionVerdict::EjectToTier
    } else {
        EjectionVerdict::Keep
    }
}

/// Inter-object aggregation metadata for pyramid mipmaps (CC §6.1.2, §19.7.1).
#[derive(Debug, Clone, PartialEq)]
pub struct AggregationMetaV1 {
    pub version: u32,
    pub content_id: String,
    pub corpus_kind: String,
    pub tier: u32,
    pub aggregation_algorithm_id: String,
    pub source_count: u32,
    /// Kish effective sample size: (Σ mᵢ)² / Σ mᵢ².
    pub n_eff: u32,
    /// R9 maximum source multiplicity (size of largest content-similar cluster).
    pub max_source_multiplicity: u32,
    pub member_commitment: [u8; 32],
    pub mass_commitment: [u8; 32],
}

/// Compute the Kish effective sample size `n_eff = (Σ mᵢ)² / Σ mᵢ²` using integer floor
/// over fixed-point masses in parts-per-million (CC §6.1.2.1.2). NEVER uses `f64::round()`.
#[must_use]
pub fn effective_source_count_fixed_point(member_masses_ppm: &[u64]) -> u32 {
    let mut sum_ppm: u128 = 0;
    let mut sum_sq: u128 = 0;
    for &m in member_masses_ppm {
        if m > 0 {
            let m_u128 = m as u128;
            sum_ppm += m_u128;
            sum_sq += m_u128 * m_u128;
        }
    }
    if sum_ppm == 0 || sum_sq == 0 {
        return 0;
    }
    let n_eff = (sum_ppm * sum_ppm) / sum_sq;
    (n_eff as u32).max(1)
}

/// Compute Kish effective sample size from f64 mass values by scaling to fixed-point PPM
/// and computing the integer floor (CC §6.1.2.1.2). NEVER uses `f64::round()`.
#[must_use]
pub fn effective_source_count(member_masses: &[f64]) -> u32 {
    let masses_ppm: Vec<u64> = member_masses
        .iter()
        .map(|&m| if m > 0.0 { (m * (PPM_SCALE as f64)) as u64 } else { 0 })
        .collect();
    effective_source_count_fixed_point(&masses_ppm)
}

/// Kish mass-dominance gate (CC §6.1.2, §6.1.2.1.2, §19.7.1.2):
/// A tier passes iff `n_eff >= N_EFF_MIN` (3) and `n_eff * 2 >= source_count` (at least 50% of raw sources).
#[must_use]
pub fn passes_dominance_gate(meta: &AggregationMetaV1) -> bool {
    if meta.version < 2 || meta.source_count == 0 {
        return false;
    }
    meta.n_eff >= N_EFF_MIN && u64::from(meta.n_eff) * 2 >= u64::from(meta.source_count)
}

/// R9 content-multiplicity gate (CC §6.1.2.1.2 R9, §19.7.1.3):
/// A tier passes iff `source_count >= R9_HOLDER_FLOOR` (9) and `max_source_multiplicity * n_min <= source_count`.
#[must_use]
pub fn passes_multiplicity_gate(meta: &AggregationMetaV1, n_min: u32) -> bool {
    if meta.version < 3 || meta.source_count < R9_HOLDER_FLOOR || n_min == 0 {
        return false;
    }
    u64::from(meta.max_source_multiplicity) * u64::from(n_min) <= u64::from(meta.source_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verification of the exact CIRISConformance test_540 noise floor ladder:
    /// target_holders = 10, min_viable = 3:
    /// - 14 -> Full
    /// - 10 -> Full
    /// - 7 -> Partial
    /// - 3 -> Partial
    /// - 0 -> EnvelopeOnly
    /// - N5 revocation -> EjectHardDelete -> EnvelopeOnly regardless of chunk rarity.
    #[test]
    fn test_540_noise_floor_ladder_and_n5_revocation() {
        let target_holders = 10;
        let min_viable = 3;

        // 14 holders -> Full
        assert_eq!(classify(14, target_holders, min_viable), NoiseFloorState::Full);

        // Exactly 10 holders -> Full
        assert_eq!(classify(10, target_holders, min_viable), NoiseFloorState::Full);

        // 7 holders (degraded but above min_viable) -> Partial
        assert_eq!(classify(7, target_holders, min_viable), NoiseFloorState::Partial);

        // Exactly 3 holders (blinking dot floor) -> Partial
        assert_eq!(classify(3, target_holders, min_viable), NoiseFloorState::Partial);

        // 2 holders (below noise floor) -> EnvelopeOnly
        assert_eq!(classify(2, target_holders, min_viable), NoiseFloorState::EnvelopeOnly);

        // 0 holders -> EnvelopeOnly
        assert_eq!(classify(0, target_holders, min_viable), NoiseFloorState::EnvelopeOnly);

        // Manifest header is always retained in EnvelopeOnly
        let manifest = ChunkManifestV1::new(
            "test-doc-1".into(),
            "blob".into(),
            target_holders,
            min_viable,
            10240,
        );
        assert!(!manifest.envelope_header.is_empty());

        // CC §6.1.2.2 prohibited categories zero the envelope header
        let zeroed = ProhibitedCategory::sanitize_envelope_header(
            &manifest.content_id,
            ProhibitedCategory::CourtOrder,
        );
        assert_eq!(zeroed, "zeroed-content-hash-only:sha256:test-doc-1");

        // CLM-noise-N5: Revocation dominates rarity and forces immediate EjectHardDelete
        let verdict_revoked_rare = should_eject_above_target(
            10,
            &ChunkStoragePolicy::default(),
            ConsentState::Revoked,
            1, // very rare
        );
        assert_eq!(verdict_revoked_rare, EjectionVerdict::EjectHardDelete);

        let verdict_revoked_common = should_eject_above_target(
            50,
            &ChunkStoragePolicy::default(),
            ConsentState::Revoked,
            50, // common
        );
        assert_eq!(verdict_revoked_common, EjectionVerdict::EjectHardDelete);

        // CC §3.1.3.3 token validation: custody:ack:v1 accepted, holds_bytes refused
        assert_eq!(
            validate_holding_claim_token("custody:ack:v1:sha256:abc"),
            Ok(CUSTODY_ACK_V1_PREFIX)
        );
        assert_eq!(
            validate_holding_claim_token("holds_bytes:sha256:abc"),
            Err("RefusedRetiredHoldsBytes")
        );
    }

    /// Verification of test_541 ejection routing across consent states, capacity pressure,
    /// and targeted tier shedding.
    #[test]
    fn test_541_ejection_routing() {
        // Active consent, under capacity pressure -> EjectToTier
        assert_eq!(
            ejection_verdict(ConsentState::Active, true),
            EjectionVerdict::EjectToTier
        );

        // Active consent, no pressure -> Keep
        assert_eq!(
            ejection_verdict(ConsentState::Active, false),
            EjectionVerdict::Keep
        );

        // Unknown consent, under pressure -> EjectToTier
        assert_eq!(
            ejection_verdict(ConsentState::Unknown, true),
            EjectionVerdict::EjectToTier
        );

        // Unknown consent, no pressure -> Keep
        assert_eq!(
            ejection_verdict(ConsentState::Unknown, false),
            EjectionVerdict::Keep
        );

        // Revoked consent under pressure -> EjectHardDelete
        assert_eq!(
            ejection_verdict(ConsentState::Revoked, true),
            EjectionVerdict::EjectHardDelete
        );

        // Revoked consent without pressure -> EjectHardDelete
        assert_eq!(
            ejection_verdict(ConsentState::Revoked, false),
            EjectionVerdict::EjectHardDelete
        );

        // Targeted tier shedding constructs EjectAggregatedTierOnly
        for tier in [0u32, 1, 2, 5, 10] {
            assert_eq!(
                eject_aggregated_tier(tier),
                EjectionVerdict::EjectAggregatedTierOnly { tier }
            );
        }

        // Proactive trimming: 30 target holders * 1.15 = 34.5 -> threshold 34.
        let policy = ChunkStoragePolicy { target_holders: 30 };
        // At 34 holders, still under threshold -> Keep even if common
        assert_eq!(
            should_eject_above_target(34, &policy, ConsentState::Active, 20),
            EjectionVerdict::Keep
        );

        // At 35 holders (> 34): common chunk (rarity 15 >= 30/2) -> EjectToTier
        assert_eq!(
            should_eject_above_target(35, &policy, ConsentState::Active, 15),
            EjectionVerdict::EjectToTier
        );

        // At 35 holders (> 34): rare chunk (rarity 5 < 15) -> Keep
        assert_eq!(
            should_eject_above_target(35, &policy, ConsentState::Active, 5),
            EjectionVerdict::Keep
        );
    }

    /// Verification of test_542 Kish mass-dominance gate and R9 multiplicity gate.
    #[test]
    fn test_542_dominance_and_multiplicity_gates() {
        // Balanced 10 sources of equal mass 1.0 -> n_eff == 10
        let masses_balanced = vec![1.0; 10];
        let n_eff_balanced = effective_source_count(&masses_balanced);
        assert_eq!(n_eff_balanced, 10);

        // Dominated sources: 1 source holds mass 900.0, 99 sources hold mass 1.0
        // sum = 999. sum_sq = 810000 + 99 = 810099.
        // n_eff = (999^2) / 810099 = 998001 / 810099 = 1 (integer floor, never f64 round)
        let mut masses_dominated = vec![1.0; 99];
        masses_dominated.push(900.0);
        let n_eff_dominated = effective_source_count(&masses_dominated);
        assert_eq!(n_eff_dominated, 1);

        let mut meta = AggregationMetaV1 {
            version: 2,
            content_id: "agg-1".into(),
            corpus_kind: "trace".into(),
            tier: 1,
            aggregation_algorithm_id: "whole-chunk-pyramid-v1".into(),
            source_count: 100,
            n_eff: n_eff_dominated, // 1
            max_source_multiplicity: 1,
            member_commitment: [0u8; 32],
            mass_commitment: [0u8; 32],
        };

        // Dominated source fails mass-dominance gate: 1 * 2 < 100 and n_eff < N_EFF_MIN (3)
        assert!(!passes_dominance_gate(&meta));

        // Balanced fold passes dominance gate: 60 * 2 >= 100 and 60 >= 3
        meta.n_eff = 60;
        assert!(passes_dominance_gate(&meta));

        // Version 1 lacks dominance surface and fails closed
        meta.version = 1;
        assert!(!passes_dominance_gate(&meta));

        // R9 multiplicity gate test:
        // 900 near-duplicate items under distinct IDs folded into 1000 total items
        // Even if masses are balanced (n_eff = 1000), max_source_multiplicity = 900.
        // With n_min = 2: 900 * 2 = 1800 > 1000 -> rejected!
        meta.version = 3;
        meta.source_count = 1000;
        meta.n_eff = 1000;
        meta.max_source_multiplicity = 900;
        assert!(!passes_multiplicity_gate(&meta, 2));

        // Honest diverse fold: max_source_multiplicity = 100
        // 100 * 2 = 200 <= 1000 -> passed!
        meta.max_source_multiplicity = 100;
        assert!(passes_multiplicity_gate(&meta, 2));

        // Version 2 lacks multiplicity surface and fails closed
        meta.version = 2;
        assert!(!passes_multiplicity_gate(&meta, 2));
    }
}
