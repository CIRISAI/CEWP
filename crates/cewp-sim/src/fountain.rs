//! Symbol-Rank Storage, Repair, and Noise-Floor Descent (CC §6.1.2, CLM-noise-classify).
//!
//! Models:
//! - `FountainManifestV1` (n_source, k_repair, min_viable_symbols, symbol_size)
//! - Exact per-node symbol holdings (indices 0..n_source+k_repair, withholding/corrupt adversarial flags)
//! - Three constitutional noise-floor states: Full, Partial, EnvelopeOnly (with signed manifest header retention)
//! - Prohibited categories zeroing envelope to content-hash (CC §6.1.2.2)
//! - Revocation dominance (CLM-noise-N5): ConsentState::Revoked -> EjectionVerdict::EjectHardDelete -> EnvelopeOnly
//! - Target-holder trimming: `should_eject_above_target` with 15% safety margin
//! - `AggregationMetaV1`: Kish mass-dominance gate (n_eff * 2 >= source_count) and R9 multiplicity gate (max_source_multiplicity * n_min <= source_count)
//! - Unit tests for exact CIRISConformance ladder (test_540), test_541, and test_542.

use std::collections::BTreeSet;

/// Safety margin percent above target holders before trimming occurs (15%).
pub const EJECT_ABOVE_TARGET_SAFETY_MARGIN_PCT: u32 = 15;

/// Default target holders in the holonomic federation.
pub const DEFAULT_TARGET_HOLDERS: u32 = 30;

/// Signed fountain manifest header (V1 contract, CIRISPersist#227).
/// The signed header is retained forever even when payload symbols descend below noise floor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FountainManifestV1 {
    pub content_id: String,
    pub corpus_kind: String,
    pub manifest_version: u16,
    pub n_source: u32,
    pub k_repair: u32,
    pub symbol_size: u32,
    pub original_content_length: u64,
    pub min_viable_symbols: u32,
    pub symbol_hashes: Vec<String>,
    pub envelope_header: String,
    pub signature_ed25519: String,
    pub signature_ml_dsa_65: String,
    pub pqc_key_id: String,
}

impl FountainManifestV1 {
    #[must_use]
    pub fn new(
        content_id: String,
        corpus_kind: String,
        n_source: u32,
        k_repair: u32,
        min_viable_symbols: u32,
        symbol_size: u32,
        original_content_length: u64,
    ) -> Self {
        let total = n_source + k_repair;
        let hashes = (0..total)
            .map(|i| format!("sha256-hash-symbol-{i}"))
            .collect();
        Self {
            content_id: content_id.clone(),
            corpus_kind,
            manifest_version: 1,
            n_source,
            k_repair,
            symbol_size,
            original_content_length,
            min_viable_symbols,
            symbol_hashes: hashes,
            envelope_header: format!("signed-manifest-envelope-header-for-{content_id}"),
            signature_ed25519: "ed25519-sig".into(),
            signature_ml_dsa_65: "ml-dsa-65-sig".into(),
            pqc_key_id: "pqc-key-1".into(),
        }
    }

    #[must_use]
    pub const fn total_symbols(&self) -> u32 {
        self.n_source + self.k_repair
    }
}

/// The three constitutional noise-floor states (CC §6.1.2, CLM-noise-classify).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NoiseFloorState {
    /// Lossless reconstruction (present >= n_source).
    Full,
    /// Degraded layered-codec / summary recovery (min_viable_symbols <= present < n_source).
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

/// Classify a reachable symbol count against manifest thresholds into its NoiseFloorState.
#[must_use]
pub fn classify(present: u32, n_source: u32, min_viable_symbols: u32) -> NoiseFloorState {
    if present >= n_source {
        NoiseFloorState::Full
    } else if present >= min_viable_symbols {
        NoiseFloorState::Partial
    } else {
        NoiseFloorState::EnvelopeOnly
    }
}

/// Tracks exact per-node symbol holdings with adversarial withholding and corruption modeling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeSymbolHoldings {
    pub peer_id: String,
    pub held_symbol_indices: BTreeSet<u32>,
    /// Adversarial: attests custody (`holds_bytes`) while withholding symbols.
    pub withholding: bool,
    /// Adversarial: serves corrupted symbols that fail SHA-256 verification.
    pub corrupt: bool,
}

impl NodeSymbolHoldings {
    #[must_use]
    pub fn new(peer_id: String, held: &[u32]) -> Self {
        Self {
            peer_id,
            held_symbol_indices: held.iter().copied().collect(),
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

/// Calculate the count of valid, non-corrupt, non-withholding distinct symbols present.
#[must_use]
pub fn calculate_effective_present_symbols(
    holders: &[NodeSymbolHoldings],
    manifest: &FountainManifestV1,
) -> u32 {
    let mut verified_symbols = BTreeSet::new();
    let total = manifest.total_symbols();

    for holder in holders {
        if holder.withholding || holder.corrupt {
            continue;
        }
        for &sym_idx in &holder.held_symbol_indices {
            if sym_idx < total {
                verified_symbols.insert(sym_idx);
            }
        }
    }

    verified_symbols.len() as u32
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
    /// Step down one tier under capacity pressure (intra-object symbol shed or N->1 aggregation).
    EjectToTier,
    /// Shed exactly one intermediate pyramid stratum.
    EjectAggregatedTierOnly { tier: u32 },
    /// Forced hard delete below noise floor (N5 revocation); drops all symbols immediately.
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

/// Policy parameters governing fountain replica density.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FountainPolicy {
    pub target_holders: u32,
}

impl Default for FountainPolicy {
    fn default() -> Self {
        Self {
            target_holders: DEFAULT_TARGET_HOLDERS,
        }
    }
}

/// Proactive trim primitive: determines if an over-replicated item should eject a local symbol.
///
/// Threshold: `target_holders + target_holders * 15 / 100` (e.g. 30 + 4 = 34).
/// If `holders_observed <= threshold` -> Keep.
/// If `holders_observed > threshold` -> EjectToTier if local symbol is common
/// (`local_symbol_rarity >= target_holders / 2`), else Keep if rare.
/// Revocation dominates everything -> EjectHardDelete.
#[must_use]
pub fn should_eject_above_target(
    holders_observed: u32,
    policy: &FountainPolicy,
    consent: ConsentState,
    local_symbol_rarity: u32,
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
    if local_symbol_rarity >= common_threshold {
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

/// Compute the Kish effective sample size `n_eff = (Σ mᵢ)² / Σ mᵢ²` rounded to nearest integer.
#[must_use]
pub fn effective_source_count(member_masses: &[f64]) -> u32 {
    let mut sum = 0.0f64;
    let mut sum_sq = 0.0f64;
    for &m in member_masses {
        if m > 0.0 {
            sum += m;
            sum_sq += m * m;
        }
    }
    if sum <= 0.0 || sum_sq <= 0.0 {
        return 0;
    }
    let n_eff = (sum * sum) / sum_sq;
    n_eff.round().max(1.0) as u32
}

/// Kish mass-dominance gate (CC §6.1.2, §19.7.1.2):
/// A tier passes iff `n_eff * 2 >= source_count` (i.e. n_eff is at least 50% of raw sources).
#[must_use]
pub fn passes_dominance_gate(meta: &AggregationMetaV1) -> bool {
    if meta.version < 2 || meta.source_count == 0 {
        return false;
    }
    u64::from(meta.n_eff) * 2 >= u64::from(meta.source_count)
}

/// R9 content-multiplicity gate (CC §6.1.2.1.2 R9, §19.7.1.3):
/// A tier passes iff `max_source_multiplicity * n_min <= source_count`.
#[must_use]
pub fn passes_multiplicity_gate(meta: &AggregationMetaV1, n_min: u32) -> bool {
    if meta.version < 3 || meta.source_count == 0 || n_min == 0 {
        return false;
    }
    u64::from(meta.max_source_multiplicity) * u64::from(n_min) <= u64::from(meta.source_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verification of the exact CIRISConformance test_540_noise_floor.py ladder:
    /// n_source = 10, min_viable = 3:
    /// - 14 -> Full
    /// - 10 -> Full
    /// - 7 -> Partial
    /// - 3 -> Partial
    /// - 0 -> EnvelopeOnly
    /// - N5 revocation -> EjectHardDelete -> EnvelopeOnly regardless of symbol rarity.
    #[test]
    fn test_540_noise_floor_ladder_and_n5_revocation() {
        let n_source = 10;
        let min_viable = 3;

        // 14 symbols (with repair headroom) -> Full
        assert_eq!(classify(14, n_source, min_viable), NoiseFloorState::Full);

        // Exactly 10 symbols -> Full
        assert_eq!(classify(10, n_source, min_viable), NoiseFloorState::Full);

        // 7 symbols (degraded but above min_viable) -> Partial
        assert_eq!(classify(7, n_source, min_viable), NoiseFloorState::Partial);

        // Exactly 3 symbols (blinking dot threshold) -> Partial
        assert_eq!(classify(3, n_source, min_viable), NoiseFloorState::Partial);

        // 2 symbols (below noise floor) -> EnvelopeOnly
        assert_eq!(classify(2, n_source, min_viable), NoiseFloorState::EnvelopeOnly);

        // 0 symbols -> EnvelopeOnly
        assert_eq!(classify(0, n_source, min_viable), NoiseFloorState::EnvelopeOnly);

        // Manifest header is always retained in EnvelopeOnly
        let manifest = FountainManifestV1::new(
            "test-doc-1".into(),
            "blob".into(),
            n_source,
            4,
            min_viable,
            1024,
            10240,
        );
        assert!(!manifest.envelope_header.is_empty());
        assert_eq!(manifest.total_symbols(), 14);

        // CC §6.1.2.2 prohibited categories zero the envelope header
        let zeroed = ProhibitedCategory::sanitize_envelope_header(
            &manifest.content_id,
            ProhibitedCategory::CourtOrder,
        );
        assert_eq!(zeroed, "zeroed-content-hash-only:sha256:test-doc-1");

        // CLM-noise-N5: Revocation dominates rarity and forces immediate EjectHardDelete
        let verdict_revoked_rare = should_eject_above_target(
            10,
            &FountainPolicy::default(),
            ConsentState::Revoked,
            1, // very rare
        );
        assert_eq!(verdict_revoked_rare, EjectionVerdict::EjectHardDelete);

        let verdict_revoked_common = should_eject_above_target(
            50,
            &FountainPolicy::default(),
            ConsentState::Revoked,
            50, // common
        );
        assert_eq!(verdict_revoked_common, EjectionVerdict::EjectHardDelete);
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
        let policy = FountainPolicy { target_holders: 30 };
        // At 34 holders, still under threshold -> Keep even if common
        assert_eq!(
            should_eject_above_target(34, &policy, ConsentState::Active, 20),
            EjectionVerdict::Keep
        );

        // At 35 holders (> 34): common symbol (rarity 15 >= 30/2) -> EjectToTier
        assert_eq!(
            should_eject_above_target(35, &policy, ConsentState::Active, 15),
            EjectionVerdict::EjectToTier
        );

        // At 35 holders (> 34): rare symbol (rarity 5 < 15) -> Keep
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
        // n_eff = (999^2) / 810099 = 998001 / 810099 ≈ 1.23 -> 1
        let mut masses_dominated = vec![1.0; 99];
        masses_dominated.push(900.0);
        let n_eff_dominated = effective_source_count(&masses_dominated);
        assert_eq!(n_eff_dominated, 1);

        let mut meta = AggregationMetaV1 {
            version: 2,
            content_id: "agg-1".into(),
            corpus_kind: "trace".into(),
            tier: 1,
            aggregation_algorithm_id: "raptorq-pyramid-v1".into(),
            source_count: 100,
            n_eff: n_eff_dominated, // 1
            max_source_multiplicity: 1,
            member_commitment: [0u8; 32],
            mass_commitment: [0u8; 32],
        };

        // Dominated source fails mass-dominance gate: 1 * 2 < 100
        assert!(!passes_dominance_gate(&meta));

        // Balanced fold passes dominance gate: 60 * 2 >= 100
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
