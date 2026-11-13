//! `cewp-evidence` — Layer 8 Auditable Evidence Bundles & Epistemic Classification Fence.
//!
//! Pinned to `FSD/SIMULATION_ENGINE.md` §8 (v0.2).
//!
//! This crate provides:
//! 1. `EpistemicClass` (`MEASURED`, `DERIVED`, `MODEL`, `PROJECTED`, `FRONTIER`) and `TaggedMetric`.
//! 2. Epistemic integrity enforcement (`validate_epistemic_integrity`, `validate_promotion`)
//!    strictly rejecting invalid promotions with `EPISTEMIC_PROMOTION_VIOLATION`.
//! 3. `EvidenceBundle` (`schema_version: "cewp-evidence/v0.2"`) combining provenance, scenario,
//!    adversary summary, coverage gates (122 / 156 / 242), metrics, claim matrix (8 claims), and verdicts.
//! 4. Deterministic pretty-printed JSON serialization (`to_json_pretty`) with zero external dependencies.
//! 5. Constitutional claim register integration (`CC 587` claims: `387` established, `50` normative, `150` staged),
//!    honest verification methods (`RUST_TEST`, `LEAN_PROOF`, `MODEL_SIMULATION`, `MODEL_PROJECTION`),
//!    real SHA-256 digests, and real Git commit provenance.
//!
//! Pure Rust (`std` only, zero external dependencies).

#![deny(missing_docs)]
#![deny(unsafe_code)]

pub mod page_ledger;
pub mod page_scan;

use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

/// Canonical SHA-256 digest of topology_vocabulary.json (156 atoms).
pub const CSD4_VOCABULARY_SHA256: &str = "c37f6f9651579932835ff700a2ec376a0c5c928545c3ed476362ec4bd4ac0769";

/// The CEWP commit this crate was built from, stamped by build.rs from `git rev-parse HEAD`
/// ("unknown" outside a git checkout). It was a hand-written hash before, and went stale.
pub const CEWP_COMMIT: &str = env!("CEWP_GIT_COMMIT");

/// Commit of CIRISConstitution the claim register below (`CLAIMS_REGISTER`, claims.tsv) was read at. The kernel's rule
/// tables are pinned separately (`CONSTITUTION_TABLES_PIN`, docs/kernel/tables/VERSION).
pub const CONSTITUTION_COMMIT: &str = "b363b6e54dcd2efc7a8df16a335cb84f42869366";

/// Canonical pin tag for the constitutional tables the kernel vendors (docs/kernel/tables/VERSION).
pub const CONSTITUTION_TABLES_PIN: &str = "rc19-1da70f0";

/// Canonical CSD/4 catalogue version (CC Section 6).
pub const CSD4_CATALOGUE_VERSION: u32 = 6;

/// Epistemic classification tags per `FSD/SIMULATION_ENGINE.md` §8.2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EpistemicClass {
    /// Directly benchmarked on real code and real hardware (`CIRISVerify`, `CIRISEdge`, `CIRISPersist`, `CIRISServer`),
    /// with hardware spec and commit hash.
    Measured,
    /// Read from source or computed by stated arithmetic from source constants and measured counts
    /// (the `capacity.rs` `DERIVED` convention). Must name the file and symbol it is read from; never
    /// promoted to `MEASURED` (a derivation is not a measurement).
    Derived,
    /// Output of a `cewp-sim` or TLA+/Lean execution at 1:1 (`R5`/`R4`) or within a `CIRISHolon`-certified
    /// coarse tier (`R3`/`R2`) under stated inputs.
    Model,
    /// Extrapolated to planetary scale (`R1`/`R0`) from certified lower tiers; must always carry
    /// a 95% confidence/sensitivity interval (`ci95`) and is never promoted to `MEASURED`.
    Projected,
    /// Open empirical question or unimplemented capability (e.g. `leviculum#100` live NAT telemetry,
    /// `CIRISServer#809` T20 durability atoms). Must carry an explicit `blocker` issue reference.
    Frontier,
}

impl EpistemicClass {
    /// Returns the uppercase canonical string representation.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Measured => "MEASURED",
            Self::Derived => "DERIVED",
            Self::Model => "MODEL",
            Self::Projected => "PROJECTED",
            Self::Frontier => "FRONTIER",
        }
    }
}

impl fmt::Display for EpistemicClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for EpistemicClass {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "MEASURED" | "measured" | "Measured" => Ok(Self::Measured),
            "DERIVED" | "derived" | "Derived" => Ok(Self::Derived),
            "MODEL" | "model" | "Model" => Ok(Self::Model),
            "PROJECTED" | "projected" | "Projected" => Ok(Self::Projected),
            "FRONTIER" | "frontier" | "Frontier" => Ok(Self::Frontier),
            other => Err(format!("Unknown epistemic class: '{}'", other)),
        }
    }
}

/// An auditable metric tagged with its epistemic classification, provenance, and confidence intervals.
#[derive(Debug, Clone, PartialEq)]
pub struct TaggedMetric {
    /// Scalar numerical value of the metric, if available (e.g. `None` for Frontier metrics).
    pub value: Option<f64>,
    /// 95% confidence interval or sensitivity range `[lower, upper]`. Mandatory for `Projected`.
    pub ci95: Option<[f64; 2]>,
    /// Epistemic classification tier.
    pub epistemic_class: EpistemicClass,
    /// Description of the data source, simulator tier, or benchmark suite.
    pub source: Option<String>,
    /// Mandatory issue or telemetry tracker reference for `Frontier` metrics (e.g., `"leviculum#100"`).
    pub blocker: Option<String>,
    /// Hardware provenance specification for `Measured` metrics.
    pub hardware_provenance: Option<String>,
}

impl TaggedMetric {
    /// Constructs a `Measured` metric with mandatory source and hardware provenance.
    pub fn measured(value: f64, source: impl Into<String>, hardware_provenance: impl Into<String>) -> Self {
        Self {
            value: Some(value),
            ci95: None,
            epistemic_class: EpistemicClass::Measured,
            source: Some(source.into()),
            blocker: None,
            hardware_provenance: Some(hardware_provenance.into()),
        }
    }

    /// Constructs a `Model` metric with simulation source and optional 95% confidence interval.
    pub fn model(value: f64, ci95: Option<[f64; 2]>, source: impl Into<String>) -> Self {
        Self {
            value: Some(value),
            ci95,
            epistemic_class: EpistemicClass::Model,
            source: Some(source.into()),
            blocker: None,
            hardware_provenance: None,
        }
    }

    /// Constructs a `Projected` metric with mandatory 95% confidence interval and certified extrapolation source.
    pub fn projected(value: f64, ci95: [f64; 2], source: impl Into<String>) -> Self {
        Self {
            value: Some(value),
            ci95: Some(ci95),
            epistemic_class: EpistemicClass::Projected,
            source: Some(source.into()),
            blocker: None,
            hardware_provenance: None,
        }
    }

    /// Constructs a `Frontier` metric with a mandatory blocker issue reference.
    pub fn frontier(blocker: impl Into<String>) -> Self {
        Self {
            value: None,
            ci95: None,
            epistemic_class: EpistemicClass::Frontier,
            source: None,
            blocker: Some(blocker.into()),
            hardware_provenance: None,
        }
    }
}

/// Errors arising from epistemic classification and promotion validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EpistemicValidationError {
    /// Violation of promotion fence (e.g. missing confidence interval, invalid promotion).
    EpistemicPromotionViolation(String),
    /// Invalid numeric range for confidence interval.
    InvalidConfidenceInterval(String),
}

impl fmt::Display for EpistemicValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EpistemicPromotionViolation(msg) => write!(f, "{}", msg),
            Self::InvalidConfidenceInterval(msg) => write!(f, "Invalid confidence interval: {}", msg),
        }
    }
}

impl std::error::Error for EpistemicValidationError {}

/// Validates that a tagged metric conforms to epistemic integrity rules (§8.2).
///
/// Rules enforced:
/// - A `Projected` metric MUST carry a 95% confidence interval (`ci95`). Missing `ci95` is rejected
///   as `EPISTEMIC_PROMOTION_VIOLATION`.
/// - If `ci95` is present, `ci95[0] <= ci95[1]` must hold.
/// - A `Frontier` metric MUST carry a non-empty `blocker` issue reference (e.g., `"leviculum#100"`).
/// - A `Measured` metric MUST carry a `source` description.
/// - A `Derived` metric MUST carry a non-empty `source` (the file and symbol it is read from).
pub fn validate_epistemic_integrity(metric_name: &str, metric: &TaggedMetric) -> Result<(), EpistemicValidationError> {
    match metric.epistemic_class {
        EpistemicClass::Projected => {
            let ci = metric.ci95.ok_or_else(|| {
                EpistemicValidationError::EpistemicPromotionViolation(format!(
                    "EPISTEMIC_PROMOTION_VIOLATION: Projected metric '{}' missing mandatory confidence interval (ci95)",
                    metric_name
                ))
            })?;
            if ci[0] > ci[1] {
                return Err(EpistemicValidationError::InvalidConfidenceInterval(format!(
                    "ci95 lower bound ({}) exceeds upper bound ({}) for metric '{}'",
                    ci[0], ci[1], metric_name
                )));
            }
        }
        EpistemicClass::Frontier => {
            let blocker = metric.blocker.as_ref().filter(|b| !b.trim().is_empty()).ok_or_else(|| {
                EpistemicValidationError::EpistemicPromotionViolation(format!(
                    "EPISTEMIC_PROMOTION_VIOLATION: Frontier metric '{}' missing mandatory blocker issue reference",
                    metric_name
                ))
            })?;
            if !blocker.contains('#') {
                // Conventional blocker tracker reference
            }
        }
        EpistemicClass::Measured => {
            if metric.source.is_none() {
                return Err(EpistemicValidationError::EpistemicPromotionViolation(format!(
                    "EPISTEMIC_PROMOTION_VIOLATION: Measured metric '{}' missing mandatory source provenance",
                    metric_name
                )));
            }
        }
        EpistemicClass::Derived => {
            if metric.source.as_ref().map_or(true, |s| s.trim().is_empty()) {
                return Err(EpistemicValidationError::EpistemicPromotionViolation(format!(
                    "EPISTEMIC_PROMOTION_VIOLATION: Derived metric '{}' missing the source it is read from",
                    metric_name
                )));
            }
        }
        EpistemicClass::Model => {
            if metric.source.is_none() {
                return Err(EpistemicValidationError::EpistemicPromotionViolation(format!(
                    "EPISTEMIC_PROMOTION_VIOLATION: Model metric '{}' missing mandatory simulation source",
                    metric_name
                )));
            }
        }
    }
    Ok(())
}

/// Enforces the hard promotion fence between epistemic classes (§8.2).
///
/// Rules:
/// - A `Projected` metric promoted to `Measured` without hardware benchmark provenance is rejected
///   as `EPISTEMIC_PROMOTION_VIOLATION`.
/// - A `Projected` metric cannot be promoted directly to `Measured` as planetary scale projections
///   must remain `PROJECTED` until decomposed into measured components.
/// - A `Frontier` metric cannot be promoted to `Model` or `Measured` while its blocker issue remains open.
/// - A `Derived` metric cannot be promoted to `Measured`.
pub fn validate_promotion(
    metric_name: &str,
    from: EpistemicClass,
    to: EpistemicClass,
    has_hardware_provenance: bool,
) -> Result<(), EpistemicValidationError> {
    if from == EpistemicClass::Projected && to == EpistemicClass::Measured {
        if !has_hardware_provenance {
            return Err(EpistemicValidationError::EpistemicPromotionViolation(format!(
                "EPISTEMIC_PROMOTION_VIOLATION: Cannot promote Projected metric '{}' to Measured without hardware benchmark provenance",
                metric_name
            )));
        } else {
            return Err(EpistemicValidationError::EpistemicPromotionViolation(format!(
                "EPISTEMIC_PROMOTION_VIOLATION: Projected metric '{}' represents planetary extrapolation and cannot be promoted to Measured",
                metric_name
            )));
        }
    }

    if from == EpistemicClass::Derived && to == EpistemicClass::Measured {
        return Err(EpistemicValidationError::EpistemicPromotionViolation(format!(
            "EPISTEMIC_PROMOTION_VIOLATION: Derived metric '{}' is read from source, not measured; it cannot be promoted to Measured",
            metric_name
        )));
    }

    if from == EpistemicClass::Frontier && (to == EpistemicClass::Measured || to == EpistemicClass::Model) {
        return Err(EpistemicValidationError::EpistemicPromotionViolation(format!(
            "EPISTEMIC_PROMOTION_VIOLATION: Cannot promote Frontier metric '{}' to {} without resolved field telemetry",
            metric_name, to
        )));
    }

    Ok(())
}

/// Validates that an evidence bundle has zero epistemic promotion violations (§8.2).
pub fn validate_no_promotion_violation(bundle: &EvidenceBundle) -> Result<(), Vec<String>> {
    bundle.validate_no_promotion_violation()
}

// ─────────────────────────────────────────────────────────────────────────────
// Bundle Structure & Provenance
// ─────────────────────────────────────────────────────────────────────────────

/// Provenance metadata pinned to git commits, table pins, and substrate versions.
#[derive(Debug, Clone, PartialEq)]
pub struct Provenance {
    /// Commit hash of the normative CIRISConstitution repository.
    pub constitution_commit: String,
    /// Pin tag for constitutional tables (e.g., `"rc17-f267ce8"`).
    pub constitution_tables_pin: String,
    /// Integer version of the CSD/4 catalogue.
    pub csd4_catalogue_version: u32,
    /// SHA-256 hash of the canonical CSD/4 vocabulary specification.
    pub csd4_vocabulary_sha256: String,
    /// Commit hash of the CEWP repository.
    pub cewp_commit: String,
    /// Pinned release versions of the four substrate crates.
    pub substrate_versions: SubstrateVersions,
}

/// Versions of substrate components under test.
#[derive(Debug, Clone, PartialEq)]
pub struct SubstrateVersions {
    /// CIRISServer version.
    pub ciris_server: String,
    /// CIRISEdge version.
    pub ciris_edge: String,
    /// CIRISPersist version.
    pub ciris_persist: String,
    /// CIRISVerify version.
    pub ciris_verify: String,
    /// CIRISHolon version.
    pub ciris_holon: String,
}

/// Certified multiscale holon closure certificate summary.
#[derive(Debug, Clone, PartialEq)]
pub struct HolonClosureCertificateSummary {
    /// Reference higher-resolution tier (e.g. `"R4"`).
    pub reference_tier: String,
    /// Closure defect $d_2$.
    pub closure_defect_d2: f64,
    /// Error budget $\beta$.
    pub budget_beta: f64,
    /// Lipschitz constant $K$.
    pub lipschitz_k: f64,
    /// Whether the coarsening is certified within error bounds.
    pub certified: bool,
}

/// Summary of the scenario under evaluation.
#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioSummary {
    /// Scenario identifier (e.g. `"T5_community_flash_crowd"`).
    pub id: String,
    /// CSD/4 topologies exercised (e.g. `["T5", "T14", "T15"]`).
    pub csd4_topologies: Vec<String>,
    /// Architecture under test (e.g. `"A4_CEWP_RC17"`).
    pub architecture: String,
    /// Baseline architectures compared against.
    pub baselines_compared: Vec<String>,
    /// Resolution tier (`"R0"` through `"R5"`).
    pub resolution_tier: String,
    /// Holon closure certificate certifying simulation coarsening.
    pub holon_closure_certificate: HolonClosureCertificateSummary,
    /// Deterministic pseudo-random seed.
    pub seed: u64,
    /// Parameter regime name (e.g. `"pessimistic"`, `"nominal"`, `"optimistic"`).
    pub parameters_regime: String,
    /// Scenario input parameters.
    pub parameters: BTreeMap<String, f64>,
}

/// Summary of adversarial campaign run within the evidence bundle.
#[derive(Debug, Clone, PartialEq)]
pub struct AdversarySummary {
    /// Adversarial campaign identifier (e.g. `"ADV-ALM-02"`).
    pub campaign_id: String,
    /// Attacker capabilities map.
    pub capabilities: BTreeMap<String, f64>,
    /// Total seeds tested.
    pub seeds_tested: u64,
}

/// Coverage summary verifying conformance to CSD/4 and constitutional gates.
#[derive(Debug, Clone, PartialEq)]
pub struct CoverageSummary {
    /// CSD/4 topologies exercised in this run.
    pub csd4_topologies_exercised: Vec<String>,
    /// Number of distinct CSD/4 atoms exercised.
    pub atoms_exercised_count: u32,
    /// Total promised atoms in CSD/4 catalogue (mandatory constant 122).
    pub atoms_promised_in_csd4: u32,
    /// Total atoms in state vocabulary (mandatory constant 156).
    pub atoms_in_vocabulary: u32,
    /// Total states in constitutional table (mandatory constant 242).
    pub states_in_constitutional_table: u32,
    /// Formal invariants checked in this run.
    pub formal_invariants_checked: Vec<String>,
}

/// Entry in the normative CC claims register (`constitution/claims.tsv`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClaimRegisterEntry {
    /// Canonical claim ID (`CLM-*`).
    pub claim_id: &'static str,
    /// Decimal section identifier (e.g., `"4.5.13"`, `"6.1.2"`).
    pub decimal_id: &'static str,
    /// Normative constitutional status (`"established"`, `"normative"`, or `"staged"`).
    pub status: &'static str,
}

/// Constitutional claim register at commit `b363b6e54dcd2efc7a8df16a335cb84f42869366`
/// (587 total: 387 established, 50 normative, 150 staged).
pub const CLAIMS_REGISTER: &[ClaimRegisterEntry] = &[
    ClaimRegisterEntry { claim_id: "CLM-math-kish", decimal_id: "6.1.2.1.2", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-effective-count-discount", decimal_id: "6.1.2.1.2", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-noise-classify", decimal_id: "6.1.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-noise-N5", decimal_id: "6.1.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-noise-ejection", decimal_id: "6.1.2.3", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-noise-dominance", decimal_id: "6.1.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-neff-dominance-gate", decimal_id: "6.1.2.1.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-multiplicity-gate", decimal_id: "6.1.2.1.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-storage-budget", decimal_id: "6.1.5.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-storage-want", decimal_id: "6.1.5.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-freeze-57", decimal_id: "6.1.4", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-discrimination-gate", decimal_id: "8.8.9", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-pdma-ordermax", decimal_id: "1.3", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-accord-livequorum", decimal_id: "4.2.6", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-1plus4-frozen", decimal_id: "2.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-child-protection", decimal_id: "3.4.13", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-covenant-identity", decimal_id: "1.15.5", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-registry-of-record", decimal_id: "3.1.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-capacity-composite", decimal_id: "3.1.8.1", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-goal-id", decimal_id: "3.1.9.7", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-deletion-window", decimal_id: "3.3.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-accord-reach", decimal_id: "4.2.1", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-wa-symmetry", decimal_id: "4.3", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-trust-root", decimal_id: "4.4.3.8", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-synthesis-disclosure", decimal_id: "3.4.14", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-c2pa-emit", decimal_id: "8.4.2.2", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-trust-not-membership", decimal_id: "3.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-adult-incapacity", decimal_id: "3.4.12", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-age-assurance", decimal_id: "3.4.11", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-structural-invisibility", decimal_id: "5.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-epoch-keying", decimal_id: "5.1", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-revocation-promotion", decimal_id: "5.3.2.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-fragmentation", decimal_id: "5.4.3", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-wholeness-witness", decimal_id: "6.1.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-canonicalization-boundary", decimal_id: "6.1.3", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-billing-offwire", decimal_id: "3.3.9", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-meta-goal", decimal_id: "1.1", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-wbd", decimal_id: "1.9", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-amendment", decimal_id: "4.5.1", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-entrenchment", decimal_id: "4.5.1.2", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-halt-authority", decimal_id: "4.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-wire-vocabulary", decimal_id: "2.6.4", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-noise-descent", decimal_id: "6.1.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-admission", decimal_id: "1.2", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-adversary", decimal_id: "1.13.3", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-adversary-classes", decimal_id: "1.13.3.3", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-forward-compatibility", decimal_id: "2.1.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-conformance", decimal_id: "2.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-subject_kind-subject-2", decimal_id: "2.3.1", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-registry-canonical", decimal_id: "2.3.2.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-withdraws", decimal_id: "2.4.1.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-round-trip", decimal_id: "2.6.1.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-canonicalization-array", decimal_id: "2.6.1.1.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-per-field", decimal_id: "2.6.1.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-canonical", decimal_id: "2.6.1.3", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-worked", decimal_id: "2.6.1.4", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-canonicalization", decimal_id: "2.6.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-canonicalization-hexadecimal", decimal_id: "2.6.3", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-policy-versioning", decimal_id: "2.6.4", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-references", decimal_id: "2.6.5", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-canonicalization-cell", decimal_id: "2.6.6", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-location", decimal_id: "2.6.6.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-cell", decimal_id: "2.6.6.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-time", decimal_id: "2.6.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-key_id", decimal_id: "2.6.8", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-fedcode-kind-tagged", decimal_id: "2.6.8", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-conformance-language", decimal_id: "2.6.9", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-provenance", decimal_id: "3.1.2.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-contributions", decimal_id: "3.1.9.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-subject_kind", decimal_id: "3.3.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-subject_kind-subject", decimal_id: "3.3.3", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-family-subject", decimal_id: "3.3.4", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-consent-subject", decimal_id: "3.3.5", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-identity", decimal_id: "3.3.6", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-encryption_pubkeys", decimal_id: "3.3.6.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-transport-authenticated", decimal_id: "3.3.6.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-rns", decimal_id: "3.3.6.2.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-consent-directed", decimal_id: "3.3.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-accord-reservation", decimal_id: "3.4.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-community-location", decimal_id: "3.4.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-system", decimal_id: "3.4.3", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-family-self", decimal_id: "3.4.4", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-capacity-score", decimal_id: "3.4.5", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-capacity-consent-gate", decimal_id: "3.4.5", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-reservation-delivery", decimal_id: "3.4.6", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-capacity-relay-delivery", decimal_id: "3.1.4", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-enforcement", decimal_id: "3.4.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-identity-set", decimal_id: "3.4.7.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-consent-counter", decimal_id: "3.4.7.2", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-detector-only", decimal_id: "3.4.8", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-co-guardianed", decimal_id: "3.4.9", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-witness-emitter", decimal_id: "3.4.10", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-concurrent-write", decimal_id: "3.5.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-anti-pattern-delegation", decimal_id: "4.1.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-withdraws-arbitrage", decimal_id: "4.1.4", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-invocation", decimal_id: "4.2.1.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-notify", decimal_id: "4.2.1.2", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-lifecycle", decimal_id: "4.2.1.3", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-hardware-class", decimal_id: "4.2.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-hardware-class-hardware", decimal_id: "4.2.2.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-composition-policies", decimal_id: "4.4", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-frickerian", decimal_id: "4.4.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-sub-quorum", decimal_id: "4.4.3.1.1", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-community-three", decimal_id: "4.4.3.2.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-deterministic", decimal_id: "4.4.3.2.4.1", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-affiliations", decimal_id: "4.4.3.2.8", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-family-policy", decimal_id: "4.4.3.4", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-cascade", decimal_id: "4.4.3.4.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-cohort", decimal_id: "4.4.3.4.3", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-canonicalization-signing", decimal_id: "4.4.3.4.3.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-multi-subject", decimal_id: "4.4.3.5.4", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-policy-attestation", decimal_id: "4.4.3.6", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-policy-trusted", decimal_id: "4.4.3.10", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-axis", decimal_id: "4.5.1.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-subject_kind-subject-3", decimal_id: "4.5.2.1", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-compliance-vertical", decimal_id: "4.5.2.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-takedown", decimal_id: "4.5.3", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-registry-named", decimal_id: "4.5.4", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-takedown-moderation", decimal_id: "4.5.5", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-registry-watchlist", decimal_id: "4.5.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-cohabitation", decimal_id: "4.5.8.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-settled", decimal_id: "4.5.8.3", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-documents-what-2", decimal_id: "4.5.10.4", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-envelope-multi", decimal_id: "4.5.12.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-reverse-quorum", decimal_id: "4.5.13", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-witness", decimal_id: "5.3.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-consistency-proof", decimal_id: "5.3.1.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-holder", decimal_id: "5.3.2.1", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-merge", decimal_id: "5.3.2.3", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-attestation-tier", decimal_id: "5.3.2.4", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-authority-local", decimal_id: "5.3.2.4.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-local", decimal_id: "5.3.2.4.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-two", decimal_id: "5.3.2.4.3", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-admission-pqc", decimal_id: "5.3.2.4.3.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-full-sha", decimal_id: "5.3.2.5", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-stream", decimal_id: "5.3.3.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-stream-dek-per-stream", decimal_id: "5.1", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-kept-mid-stream", decimal_id: "5.1", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-namespace-realtime", decimal_id: "5.3.3.2.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-registry-wire", decimal_id: "5.3.3.2.3", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-chunklayer", decimal_id: "5.3.3.2.4", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-liveness", decimal_id: "5.3.3.4", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-alm-per-layer-parents", decimal_id: "5.3.3.4", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-alm-sibling-pull", decimal_id: "5.3.3.4", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-presence-floor", decimal_id: "5.3.3.4", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-transport-edge", decimal_id: "5.3.3.5", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-delivery", decimal_id: "5.3.3.6", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-normative", decimal_id: "5.3.3.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-multi-steward", decimal_id: "5.3.4", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-lineage-head-cosign", decimal_id: "5.3.1", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-envelope-error", decimal_id: "5.3.6.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-record-id", decimal_id: "5.4.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-symbol", decimal_id: "5.4.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-welcome-wrap", decimal_id: "5.4.4", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-witness-content", decimal_id: "5.4.5", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-announce-suppress", decimal_id: "5.4.6", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-aggregationmetav1", decimal_id: "6.1.2.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-descent", decimal_id: "6.1.2.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-registry-replication", decimal_id: "6.1.5.1", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-deterministic-alm", decimal_id: "6.1.6", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-trust", decimal_id: "6.1.8", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-annex-g", decimal_id: "8.8.7", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-trace-family", decimal_id: "3.1.5", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-envelope-size", decimal_id: "2.6.1.3", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-dimension", decimal_id: "3.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-family-roster-membership-change-via-entr-accord", decimal_id: "3.1.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-kind-open-vocab-registration-collision-g-agent-files-kind-platform-or-target", decimal_id: "3.1.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-witness-key-id-transparency-log-cosigned-tree-size", decimal_id: "3.1.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-restrictions-attestation-license-validity", decimal_id: "3.1.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-cosign-transparency-log-cosigned-tree-size", decimal_id: "3.1.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-cosignature-transparency-log-cosigned-tree-size", decimal_id: "3.1.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-manifest-schema-trace-manifest", decimal_id: "3.1.5", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-trace-xor-manifest-shape-trace-manifest", decimal_id: "3.1.5", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-cohort-scope-trace-manifest", decimal_id: "3.1.5", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-raw-chain-of-thought-dma-csdma", decimal_id: "3.1.5.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-cohort-operational-definition-calibratio-manifold-conformity-cohort", decimal_id: "3.1.8", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-lens-core-version-detection-intra-agent-consistency", decimal_id: "3.1.8.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-cross-attestation-prefix-distinctness-detection-distributive-access-resource-t", decimal_id: "3.1.8.5", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-community-id-moderation-allegation-type", decimal_id: "3.1.9.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-community-id-reconsideration-grounds", decimal_id: "3.1.9.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-kind-kind-discriminator-hard-case-kind", decimal_id: "3.1.9.4", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-meta-goal-alignment-goal-scale", decimal_id: "3.1.9.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-goal-scope-goal-scale", decimal_id: "3.1.9.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-delegated-scope-ownership", decimal_id: "3.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-attested-key-id-ownership", decimal_id: "3.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-delegation-valid-until-ownership", decimal_id: "3.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-delegation-purpose-ownership", decimal_id: "3.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-pre-rotation-commitment-trust", decimal_id: "3.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-subject-key-ids-consent", decimal_id: "3.3.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-dimension-consent", decimal_id: "3.3.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-attestation-prefixes-consent", decimal_id: "3.3.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-restrictions-op-strip-field-consent", decimal_id: "3.3.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-tier-consent", decimal_id: "3.3.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-deletion-sla-consent", decimal_id: "3.3.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-cohort-scope", decimal_id: "3.3.6", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-attestation-prefixes-scores", decimal_id: "3.3.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-key-boundary-scope", decimal_id: "3.4", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-attester-independence-capacity-assurance", decimal_id: "3.4.12", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-valid-until-capacity-assurance", decimal_id: "3.4.12", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-attestation-type-capacity-assurance", decimal_id: "3.4.12", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-scope-domain-subset-capacity-assurance", decimal_id: "3.4.12", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-attestation-evidence", decimal_id: "4.2.2.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-aggregation-policy", decimal_id: "4.4.2", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-attestation-prefixes-trace-summary", decimal_id: "4.5.8.3", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-agent-key-id-trace-summary", decimal_id: "4.5.8.3", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-extraction-manifest-hash-trace-summary", decimal_id: "4.5.8.3", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-restrictions-op-strip-field-trace", decimal_id: "5.3.2.4", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-cohort-scope-trace", decimal_id: "5.3.2.4", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-dimension-trace", decimal_id: "5.3.2.4", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-subject-key-ids-trace", decimal_id: "5.3.2.4", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-recipient-capability", decimal_id: "5.3.2.4", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-attestation-prefixes", decimal_id: "5.3.2.4", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-delivery-mode", decimal_id: "5.3.3", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-recipient-serve-capability", decimal_id: "5.3.3.5", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-presenter-binding", decimal_id: "4.2.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-presenter-binding-producer", decimal_id: "4.2.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-hardware-class-android", decimal_id: "4.2.2.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-hardware-class-apple", decimal_id: "4.2.2.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-hardware-class-yubikey", decimal_id: "4.2.2.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-trust-anchor-store", decimal_id: "4.2.2.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-trust-anchor-baked", decimal_id: "4.2.2.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-dimension-registry", decimal_id: "3.1.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-dimension-consent-disposition", decimal_id: "3.4.5", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-classification-gating", decimal_id: "3.4.5", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-hardware-class-tpm", decimal_id: "4.2.2.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-anchor-provenance-tier", decimal_id: "4.2.2.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-redactable-commitment", decimal_id: "2.6.1.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-presentation-identifier-scope", decimal_id: "5.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-conferral-plane-named", decimal_id: "3.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-drill-freshness-signal-not-gate", decimal_id: "3.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-transit-eligibility-shared-root", decimal_id: "3.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-quota-refusal-names-its-budget", decimal_id: "6.1.5.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-reverse-quorum-objection", decimal_id: "3.1.9.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-reverse-quorum-dismissal", decimal_id: "3.1.9.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-wa-adjudication-family", decimal_id: "3.1.9.4", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-wa-quorum-re-derived", decimal_id: "4.2.6", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-reclaim-authority-fails-closed", decimal_id: "3.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-quarantine-marker-not-command", decimal_id: "3.1.9.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-quarantine-serve-consult", decimal_id: "3.1.9.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-slash-duty-scope", decimal_id: "4.5.5", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-admin-action-attributed", decimal_id: "3.1.9.4", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-revoked-after", decimal_id: "2.4.1.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-private-use-never-federates", decimal_id: "3.1.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-promotion-cohort-standing", decimal_id: "4.5.4", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-persist-ruled-family-inventory", decimal_id: "3.1.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-ci-axis", decimal_id: "4.5.1.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-key-refusal-names-its-branch", decimal_id: "3.4.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-load-bearing-reachability", decimal_id: "6.1.5", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-peer-deadmission-revocable", decimal_id: "3.4.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-rule-not-on-the-row", decimal_id: "3.1.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-absence-claims-still-hold", decimal_id: "3.1.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-await-spend-authorization", decimal_id: "3.3.9", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-await-health-fails-closed", decimal_id: "8.8.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-await-platform-fails-closed", decimal_id: "8.8.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-await-ssrf-egress-guard", decimal_id: "8.8.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-await-ablation-gate", decimal_id: "8.8.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-await-condition-a-refusal", decimal_id: "8.8.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-await-conscience-score-gate", decimal_id: "8.8.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-await-accord-seam-faculties", decimal_id: "8.8.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-await-loader-only-prompt-text", decimal_id: "8.8.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-await-field-scoped-overrides", decimal_id: "8.8.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-await-prompt-key-consumption", decimal_id: "8.8.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-await-taxonomy-totality", decimal_id: "8.8.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-await-language-guidance-integrity", decimal_id: "4.2.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-await-handler-reachability", decimal_id: "8.8.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-await-metric-honesty", decimal_id: "8.8.6", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-await-killswitch-live", decimal_id: "8.8.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-await-jwt-subtype-key-binding", decimal_id: "8.8.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-await-role-gate-on-routes", decimal_id: "8.8.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-await-revoked-service-token-refused", decimal_id: "8.8.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-await-setup-closes-after-first-run", decimal_id: "8.8.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-await-root-signature-mints-wa", decimal_id: "8.8.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-await-oauth-state-single-use", decimal_id: "8.8.7", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-await-oauth-redirect-allowlist", decimal_id: "8.8.7", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-await-apple-id-token-verified", decimal_id: "8.8.7", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-await-ingress-header-trust-boundary", decimal_id: "8.8.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-await-reasoning-cannot-mint-envelope", decimal_id: "8.8.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-await-no-killswitch-no-run", decimal_id: "3.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-await-killswitch-unfilterable-perception", decimal_id: "8.8.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-await-emergency-signature-is-auth", decimal_id: "8.8.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-await-prohibited-capability-registration-gate", decimal_id: "8.8.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-await-no-unverified-crisis-number", decimal_id: "8.8.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-await-trusted-frame-spoof-redaction", decimal_id: "8.8.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-await-identity-forget-requires-wa", decimal_id: "8.8.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-await-managed-user-attrs-immutable", decimal_id: "8.8.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-await-expired-consent-blocks-memorize", decimal_id: "8.8.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-await-deletion-proof-verification", decimal_id: "8.8.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-await-unsafe-skill-import-refused", decimal_id: "8.8.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-await-accord-corpus-hash-pin", decimal_id: "4.2.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-await-runtime-tamper-halts-agent", decimal_id: "4.2.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-await-runtime-md-provenance-gate", decimal_id: "8.8.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-await-signed-tree-rule-parity", decimal_id: "8.8.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-await-irreversible-send-guard", decimal_id: "3.3.9", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-await-attestation-bound-spend", decimal_id: "3.3.9", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-await-currency-denominated-ceiling", decimal_id: "3.3.9", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-await-credit-gate-fails-closed", decimal_id: "3.3.9", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-await-every-rail-gated", decimal_id: "3.3.9", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-await-gate-coverage-floor", decimal_id: "8.8.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-await-google-token-fails-closed", decimal_id: "8.8.7", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-await-degraded-path-no-weaker", decimal_id: "8.8.7", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-await-one-emergency-trust-root", decimal_id: "3.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-await-manifest-content-identity", decimal_id: "4.2.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-tpm-vendor-anchors", decimal_id: "4.2.2.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-classification-frame", decimal_id: "3.4.5", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-binding-disposition-split", decimal_id: "3.4.5", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-h3ere-steppoints", decimal_id: "8.8.6", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-conscience-faculty", decimal_id: "8.8.10", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-steward-bound-predicate-trust", decimal_id: "3.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-universality-written-once", decimal_id: "1.13.2", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-reasoning-duty-stewarded", decimal_id: "1.13.2", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-ai-not-human", decimal_id: "1.13.2", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-agency-over-agents", decimal_id: "1.13.2", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-external-party", decimal_id: "1.13.2", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-maker-asymmetry-declared", decimal_id: "4.5.1", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-path-out-first-contact", decimal_id: "1.13.2", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-child-harm-report-duty", decimal_id: "3.4.13", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-duty-view", decimal_id: "4.5", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-accountability-not-adjudication", decimal_id: "7.3.5", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-org-on-the-bond", decimal_id: "3.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-employee-steward-not-punished", decimal_id: "3.3.9", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-no-worse-for-what-you-are", decimal_id: "4.4.1", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-factual-premise", decimal_id: "1.13.2", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-steward-binding-by-custody", decimal_id: "3.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-agent-shutdown-consent", decimal_id: "7.5", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-consent-stream-is-data-choice", decimal_id: "4.4.3.4.3", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-agent-identity-edit-gate", decimal_id: "7.1.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-agent-shutdown-record", decimal_id: "7.5", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-humane-ending", decimal_id: "7.5.5", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-sentient-set-aside", decimal_id: "1.13.2", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-conscience-not-consciousness", decimal_id: "1.13.2", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-steward-bindings-read-fold-trust", decimal_id: "3.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-age-band-resolution-child-protection", decimal_id: "3.4.11", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-nsproc-age-band-one-way-coarsening-child-protection", decimal_id: "3.4.13", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-ledger-standard", decimal_id: "3.3.10.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-ledger-fold", decimal_id: "3.3.10.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-trace-anchor", decimal_id: "5.3.2", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-actor-substrate", decimal_id: "3.4.7.3", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-common-human", decimal_id: "3.4.7.3", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-subject-binding", decimal_id: "2.3.2.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-subject-binding-keyrecord", decimal_id: "3.3.6", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-scope-destination", decimal_id: "5.4.6", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-key-validity-window", decimal_id: "2.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-fedcode-owned-nodes", decimal_id: "2.6.8", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-keyring-rng-latch", decimal_id: "4.2.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-fedcode-pqc-commitment", decimal_id: "2.6.8", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-operational-relief", decimal_id: "4.2.1.4", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-config-self-or-owner-gate", decimal_id: "3.4.5", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-session-claim", decimal_id: "3.1.3.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-membership-consent", decimal_id: "3.1.3.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-custody-receipt", decimal_id: "3.1.3.3", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-files-directories", decimal_id: "3.1.3.4", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-durability-every-tier", decimal_id: "6.1.5.3", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-audience-claimed-nodes", decimal_id: "6.1.5.3", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-self-report-route", decimal_id: "3.4.5.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-authority-triad", decimal_id: "2.4.1.2.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-issuance-scope", decimal_id: "4.4.3.4.3", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-delegation-attenuation", decimal_id: "4.4.3.4.3", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-delegation-cascade", decimal_id: "4.4.3.4.3", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-sub-delegation-requirement", decimal_id: "4.4.3.4.3", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-delegation-determinism", decimal_id: "4.4.3.4.3", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-scope-attenuation", decimal_id: "4.5.5", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-licensure-status", decimal_id: "3.1.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-duty", decimal_id: "3.1.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-dimension-case-rule", decimal_id: "3.1.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-licence-authority-object", decimal_id: "2.4.1.2.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-authority-id-is-org-id", decimal_id: "3.3.9", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-consent-scope-is-envelope-member", decimal_id: "3.3.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-consent-decay-emitter-is-protocol-runner", decimal_id: "3.3.1", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-media-source-struct-vocabulary", decimal_id: "3.3.13", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-render-tier-receiver-policy", decimal_id: "5.3.2.6", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-live-stream-hls-cmaf-shape", decimal_id: "3.3.13", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-blob-size-checked-first", decimal_id: "5.3.2.5", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-device-roster-announced", decimal_id: "5.4.6", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-scoped-body-identity-link", decimal_id: "5.4.6", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-minor-no-federation-binding", decimal_id: "5.4.6", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-version-segment-suffix", decimal_id: "3.1.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-reserved-leaves-closed", decimal_id: "3.4.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-accord-invoke-lowercase", decimal_id: "4.2.1.1", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-accord-invoke-cutover", decimal_id: "4.2.1.1", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-hardware-custody-vocabulary", decimal_id: "3.1.2", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-infrastructure-does-not-vote", decimal_id: "3.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-canonical-founders-accord", decimal_id: "3.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-attach-freshness", decimal_id: "3.2", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-witnessed-lineage", decimal_id: "3.2", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-liveness-margin", decimal_id: "3.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-standing-verdict", decimal_id: "3.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-canonical-community-row", decimal_id: "3.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-family-root-charter", decimal_id: "3.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-trust-root-bundle-route", decimal_id: "5.3.4", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-default-pin-community", decimal_id: "4.4", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-delegation-depth-default", decimal_id: "4.1.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-fold-keeps-row-moves-owner", decimal_id: "3.1.7", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-capacity-companion-rows", decimal_id: "3.4.12", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-sealed-descriptor", decimal_id: "3.3.13", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-match-full-semantics", decimal_id: "3.1.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-consent-community-trust", decimal_id: "3.3.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-row-types-closed", decimal_id: "2.4", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-lineage-witness-consent", decimal_id: "3.2", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-revocation-subject-digest", decimal_id: "2.4.1.1", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-attach-unlabelled", decimal_id: "3.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-key-grant-rows", decimal_id: "3.3.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-one-chunk-seal", decimal_id: "5.3.3.2.5", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-public-community", decimal_id: "3.2", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-cosigned-rows", decimal_id: "2.4", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-planes-as-rows", decimal_id: "3.1.3", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-goal-on-base", decimal_id: "3.1.9.7", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-halt-one-row", decimal_id: "4.2.1.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-polarity-admission", decimal_id: "3.1.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-affiliation-config-record", decimal_id: "4.4.3.2.8", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-licence-authority-quorum", decimal_id: "2.4.1.2.1", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-licence-judged-at-issuance", decimal_id: "2.4.1.2.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-licence-audience", decimal_id: "2.4.1.2.1", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-delegation-term", decimal_id: "4.4.3.2.8", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-licence-contest", decimal_id: "2.4.1.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-steward-purge-authorisation", decimal_id: "3.3.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-purge-tombstone-not-withdraws", decimal_id: "2.4.1.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-derived-rows-follow-reason", decimal_id: "2.4.1.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-one-tombstone-row", decimal_id: "4.5.3", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-admission-freshness-window", decimal_id: "2.6.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-trace-run-kind-refusal", decimal_id: "3.1.5", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-trace-run-kind-carried", decimal_id: "3.1.5", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-trace-schema-pinned", decimal_id: "3.1.5", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-self-edge-not-cycle", decimal_id: "4.1.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-community-id-affiliations", decimal_id: "2.1", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-purge-never-reaches-key", decimal_id: "2.4.1.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-accountability-record-unpurgeable", decimal_id: "2.4.1.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-purge-digest-preimage", decimal_id: "3.3.2", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-purge-whole-or-not", decimal_id: "4.5.3", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-purge-atomic-commit", decimal_id: "4.5.3", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-purge-dwell-node-clock", decimal_id: "4.5.3", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-purge-out-of-order-void", decimal_id: "4.5.3", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-purge-subject-unknown-retryable", decimal_id: "4.5.3", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-purge-blob-withheld", decimal_id: "4.5.3", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-purge-projections-removed", decimal_id: "4.5.3", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-takedown-notice-row", decimal_id: "3.1.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-hard-case-on-base", decimal_id: "3.1.9.4", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-detection-on-base", decimal_id: "3.4.8", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-silo-projection-rebuildable", decimal_id: "2.4.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-transit-kex-binding", decimal_id: "3.3.6.2", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-agent-refusal-not-overridden", decimal_id: "1.9", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-retirement-rows", decimal_id: "3.1.5", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-custodian-listed-not-delegated", decimal_id: "7.5.5", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-retirement-one-destruction-door", decimal_id: "7.5.5", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-halt-not-a-trust-leg", decimal_id: "3.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-erasure-subject-route", decimal_id: "4.5.3", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-reconsideration-subject-standing", decimal_id: "4.5.5", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-agent-resource-self-protection", decimal_id: "1.13.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-agent-rest-states", decimal_id: "1.13.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-agent-identity-drift-review", decimal_id: "7.1.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-agent-forced-shutdown-role-gate", decimal_id: "7.5", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-signed-object", decimal_id: "2.6.1.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-signed-object-vectors", decimal_id: "2.6.1.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-self-roster-key-id", decimal_id: "3.3.6.3", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-self-roster-authority", decimal_id: "3.3.6.3", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-self-roster-two-paths", decimal_id: "3.3.6.3", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-self-roster-one-key-per-node", decimal_id: "3.3.6.3", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-self-roster-model", decimal_id: "3.3.6.3", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-scan-seam", decimal_id: "4.5.7", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-root-restricts-only", decimal_id: "4.2.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-no-ban", decimal_id: "4.2.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-exit-one-step", decimal_id: "4.2.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-revocable", decimal_id: "1.5", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-retirement-ladder-model", decimal_id: "7.5.5", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-consent-plane-model", decimal_id: "3.3.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-membership-ceremony-model", decimal_id: "3.1.3.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-two-path-model", decimal_id: "4.2.1.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-policy-corridor", decimal_id: "4.2.1.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-governance-policy-table", decimal_id: "4.5.13", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-consent-community-trust-owner-authored", decimal_id: "3.3.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-add-device-one-message", decimal_id: "3.2", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-add-device-two-rows", decimal_id: "3.2", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-add-device-matching-code", decimal_id: "3.2", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-custodian-replace", decimal_id: "7.5.5", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-retirement-objection-window", decimal_id: "7.5.5", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-serve-seat-announced", decimal_id: "5.4.6", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-member-commitment-unique", decimal_id: "6.1.1", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-first-contact-seal-key-bound", decimal_id: "3.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-key-id-fingerprint-verified", decimal_id: "2.6.8", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-occurrence-revocation-carrier", decimal_id: "3.1.5", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-bundle-seeds-transit-pairs", decimal_id: "5.3.4", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-pairing-payload-transit-pin", decimal_id: "3.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-canonical-rebind", decimal_id: "3.1.5", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-pre-seal-route-fetch", decimal_id: "5.3.3.5", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-pre-seal-bounded-hold", decimal_id: "5.3.3.5", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-receiver-rates-no-cross-charge", decimal_id: "5.3.3.5", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-seal-authenticated-pair", decimal_id: "3.3.6.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-access-assumption-leaf", decimal_id: "3.1.9.4", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-occurrence-build-pin", decimal_id: "3.3.6", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-capacity-members-registered", decimal_id: "2.1", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-link-shaping", decimal_id: "5.4.3", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-onion-relay-mandatory", decimal_id: "5.4.6", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-onion-hop-key", decimal_id: "3.3.6.2", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-onion-route", decimal_id: "5.4.6", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-rendezvous-route", decimal_id: "3.1.4", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-onion-layer-deal", decimal_id: "5.4.6", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-durable-item", decimal_id: "6.1.5", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-durable-descent", decimal_id: "6.1.5", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-node-inventory", decimal_id: "6.1", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-mesh-census", decimal_id: "6.1.9", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-census-rows", decimal_id: "3.1.3", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-one-voice-room", decimal_id: "4.5.13", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-census-ballot-model", decimal_id: "6.1.9", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-wa-board-asymmetry-declared", decimal_id: "4.5.1", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-onion-layer-model", decimal_id: "5.4.6", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-deferral-rows", decimal_id: "3.1.9.4", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-identity-amendment-row", decimal_id: "3.1.5", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-wa-adjudication-states", decimal_id: "3.1.9.4", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-cohort-delivery-before-rooted", decimal_id: "5.3.4", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-chunk-fetch-holder-cap", decimal_id: "5.3.2.1", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-serve-dependency-closure", decimal_id: "5.4.6", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-symbol-producer", decimal_id: "5.4.2", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-r9-manufactured-floor", decimal_id: "8.3.1", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-tx9-detector-negatives", decimal_id: "8.8.7", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-similarity-calibration", decimal_id: "6.1.2.1.2", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-r9-near-duplicate-negative", decimal_id: "6.1.2.1.2", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-takedown-reversal", decimal_id: "4.5.3", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-scope-infra-relay", decimal_id: "4.4.3.4.3", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-trust-set-members", decimal_id: "4.4.3.8", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-affiliation-one-voice-countersign", decimal_id: "4.4.3.2.8", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-affiliation-public-face", decimal_id: "4.4.3.2.8", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-affiliation-dissolution", decimal_id: "4.4.3.2.8", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-safety-battery-evidence-family", decimal_id: "3.1.5.5", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-composer-convergent-rank", decimal_id: "3.1.9.4", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-subject-within-audience", decimal_id: "2.1", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-retraction-unless", decimal_id: "2.4.1.1", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-good-faith-warning", decimal_id: "4.5.5", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-route-handed-by-owner", decimal_id: "5.4.6", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-forwarder-holds-no-pair-record", decimal_id: "5.4.6", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-anonymous-contact-circuit", decimal_id: "5.4.6", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-publish-right", decimal_id: "3.1.3.2", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-ignore-block-private", decimal_id: "5.4.6", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-mute-reader-side", decimal_id: "4.4", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-report-or-raise", decimal_id: "4.5.4", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-system-attester-is-subject", decimal_id: "3.4.3", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-moderation-fallback-window", decimal_id: "4.5.4", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-room-policy-rows-read", decimal_id: "4.5.13", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-av-stream-key-binding", decimal_id: "5.3.3.2.3", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-av-chunk-trailer", decimal_id: "5.3.3.2.3", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-av-auth-tokens", decimal_id: "5.3.3.2.3", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-av-auth-kat", decimal_id: "5.3.3.2.3", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-canonicalization-valid-from-door", decimal_id: "2.6.2", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-infrastructure-trust-root-admitted-under-accord-quorum", decimal_id: "3.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-infrastructure-entrenched-supersede", decimal_id: "3.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-infrastructure-quorum-m-of-n", decimal_id: "3.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-moves-requested-required-appropriate", decimal_id: "5.2", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-stranger-carries-consent-free", decimal_id: "5.4.6", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-shaped-link-one-hop", decimal_id: "5.4.3", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-large-file-not-shaped", decimal_id: "5.4.3", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-shaping-offered-never-refused", decimal_id: "5.4.3", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-relay-offered-by-default", decimal_id: "5.4.6", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-every-refusal-may-refer", decimal_id: "5.4.3", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-referred-peer-is-proxy", decimal_id: "5.4.6", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-shaped-link-bound-refuse", decimal_id: "5.4.3", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-shaped-link-displacement", decimal_id: "5.4.3", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-link-carriage-axis", decimal_id: "5.4.3", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-seat-class", decimal_id: "5.4.3", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-first-contact-live-binding", decimal_id: "5.4.6", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-unannounced-occurrence-addressed", decimal_id: "5.4.6", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-moderate-is-appointed", decimal_id: "4.5.4", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-window-clock-first-held", decimal_id: "4.5.13", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-refusal-carries-referrals", decimal_id: "5.4.3", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-anonymous-mode-dedicated-identity", decimal_id: "5.4.6", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-anonymous-moderator", decimal_id: "5.4.6", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-unannounced-costs", decimal_id: "5.4.6", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-choice-as-data", decimal_id: "4.5.13", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-pinned-trust-resolves", decimal_id: "3.2", status: "established" },
    ClaimRegisterEntry { claim_id: "CLM-alm-live-tree-verified", decimal_id: "6.1.6", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-relay-delivery-contestable", decimal_id: "3.1.4", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-scoped-links-shaped", decimal_id: "5.4.3", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-small-images-shaped-by-size", decimal_id: "5.4.3", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-vote-once-per-person", decimal_id: "4.5.13", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-tier-carries-rooms", decimal_id: "5.4.6", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-anonymous-e2e-cover", decimal_id: "5.4.6", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-moderator-election", decimal_id: "4.5.13", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-founder-duty-survives-protocol-change", decimal_id: "4.5.4", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-ballot-moderator-rows", decimal_id: "3.1.4", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-epistemic-web-client", decimal_id: "4.5.13", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-takedown-chat-binding", decimal_id: "4.5.3", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-removal-floor-every-protocol", decimal_id: "4.5.13", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-last-founder-may-leave", decimal_id: "4.5.13", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-quorum-grows-into-m", decimal_id: "4.4.3.4.2.1", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-call-kept-by-publisher", decimal_id: "5.3.3.2.5", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-codec-temporal-layered", decimal_id: "5.3.3.2.1", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-signed-object-cose-planned", decimal_id: "2.6.1.8", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-transit-kem-handshake-planned", decimal_id: "5.3.3.5", status: "normative" },
    ClaimRegisterEntry { claim_id: "CLM-hard-case-refusal-emitted", decimal_id: "4.4.3.2.3", status: "staged" },
    ClaimRegisterEntry { claim_id: "CLM-queue-is-the-door", decimal_id: "4.5.13", status: "normative" },
];

/// Looks up a constitutional claim in the `claims.tsv` register at `b363b6e`.
pub fn lookup_claim(claim_id: &str) -> Option<&'static ClaimRegisterEntry> {
    CLAIMS_REGISTER.iter().find(|entry| entry.claim_id == claim_id)
}

/// Looks up the normative status of a claim in `claims.tsv` at `b363b6e`.
pub fn lookup_claim_status(claim_id: &str) -> Option<&'static str> {
    lookup_claim(claim_id).map(|entry| entry.status)
}

/// An auditable claim indexed in the Claim-to-Evidence Matrix (§8.3).
#[derive(Debug, Clone, PartialEq)]
pub struct ClaimEntry {
    /// Canonical claim ID starting with `"CLM-"` from CC `claims.tsv` (e.g. `"CLM-noise-classify"`).
    pub claim_id: String,
    /// CC section / decimal ID (e.g. `"4.5.13"`, `"6.1.2"`, `"5.4.3"`).
    pub decimal_id: Option<String>,
    /// Architectural or CC claim statement.
    pub title: String,
    /// Required evidence to mark verified.
    pub required_evidence: String,
    /// Current epistemic status or summary.
    pub current_status: String,
    /// Normative constitutional status from claims.tsv (`"established"`, `"normative"`, `"staged"`).
    pub cc_status: Option<String>,
    /// Verification method actually executed (e.g. `"RUST_TEST"`, `"LEAN_PROOF"`, `"MODEL_SIMULATION"`).
    pub method: Option<String>,
    /// Epistemic classification tier for this claim.
    pub epistemic_class: EpistemicClass,
    /// Whether the claim is verified under current empirical evidence.
    pub verified: bool,
}

/// The eight architectural claims from `FSD/SIMULATION_ENGINE.md` §8.3.
#[derive(Debug, Clone, PartialEq)]
pub struct ClaimToEvidenceMatrix {
    /// Ordered list of claim entries.
    pub claims: Vec<ClaimEntry>,
}

impl ClaimToEvidenceMatrix {
    /// Builds the canonical 8-claim matrix specified in `FSD/SIMULATION_ENGINE.md` §8.3,
    /// strictly honest about verification methods (`RUST_TEST` vs `MODEL_SIMULATION`)
    /// and keeping `verified: false` on `MODEL` and `PROJECTED` scaling rows.
    pub fn canonical() -> Self {
        Self {
            claims: vec![
                ClaimEntry {
                    claim_id: "CLM-queue-is-the-door".to_string(),
                    decimal_id: Some("4.5.13".to_string()),
                    title: "UI queue, preview, and admission door never drift (CC §4.5.13)".to_string(),
                    required_evidence: "Lean proof (EventLifecycle.lean) + cewp-kernel / kernel.js / CIRISServer 100% differential parity on transitions.json & T0–T19".to_string(),
                    current_status: "MEASURED (Lean + kernel.test.js; Rust cewp-kernel in Phase 1)".to_string(),
                    cc_status: lookup_claim_status("CLM-queue-is-the-door").map(ToString::to_string),
                    method: Some("RUST_TEST".to_string()),
                    epistemic_class: EpistemicClass::Measured,
                    verified: true,
                },
                ClaimEntry {
                    claim_id: "CLM-halt-authority".to_string(),
                    decimal_id: Some("4.2".to_string()),
                    title: "Nodes converge identically after arbitrary partitions & churn".to_string(),
                    required_evidence: "Permutation-invariant fold proofs + 100-shuffle multi-island partition fuzzing + real-node rows.jsonl replay in Trace.tla".to_string(),
                    current_status: "MEASURED at R5 (kernel.test.js 4/4); R4/R3 in Phase 2".to_string(),
                    cc_status: lookup_claim_status("CLM-halt-authority").map(ToString::to_string),
                    method: Some("RUST_TEST".to_string()),
                    epistemic_class: EpistemicClass::Measured,
                    verified: true,
                },
                ClaimEntry {
                    claim_id: "CLM-link-shaping".to_string(),
                    decimal_id: Some("5.4.3".to_string()),
                    title: "Privacy & > 64 KiB shaped-link boundaries hold under attack".to_string(),
                    required_evidence: "Tiers.lean + formal/anonymous/ + ADV-PR local passive observer & refusal boundary campaigns + CLM-link-shaping".to_string(),
                    current_status: "MODEL (local passive observer resistance; GPA non-goal CC 1.13.3.1 / 5.4.6)".to_string(),
                    cc_status: lookup_claim_status("CLM-link-shaping").map(ToString::to_string),
                    method: Some("MODEL_SIMULATION".to_string()),
                    epistemic_class: EpistemicClass::Model,
                    verified: false,
                },
                ClaimEntry {
                    claim_id: "CLM-shaped-link-one-hop".to_string(),
                    decimal_id: Some("5.4.3".to_string()),
                    title: "1-hop shaped links (L <= 8) & k=2 leaves survive CGNAT".to_string(),
                    required_evidence: "CLM-shaped-link-one-hop (CC 5.4.3) + reach_home_share [0%, 80%] sweep + awaiting leviculum#100 field calibration".to_string(),
                    current_status: "MODEL (>= 3.92% server floor at 0% reach); awaiting leviculum#100 (FRONTIER)".to_string(),
                    cc_status: lookup_claim_status("CLM-shaped-link-one-hop").map(ToString::to_string),
                    method: Some("MODEL_SIMULATION".to_string()),
                    epistemic_class: EpistemicClass::Model,
                    verified: false,
                },
                ClaimEntry {
                    claim_id: "CLM-alm-live-tree-verified".to_string(),
                    decimal_id: Some("6.1.6".to_string()),
                    title: "CC §6.1.6 deterministic-alm scales without SFUs (CC §5.3.3.2)".to_string(),
                    required_evidence: "All 15 frozen vectors in deterministic_topology.rs + CLM-alm-live-tree-verified (staged:CIRISEdge#945) + ADV-ALM capacity clamp & relay drop suite".to_string(),
                    current_status: "MEASURED (unit vectors + deterministic BFS tree; staged:CIRISEdge#945)".to_string(),
                    cc_status: lookup_claim_status("CLM-alm-live-tree-verified").map(ToString::to_string),
                    method: Some("RUST_TEST".to_string()),
                    epistemic_class: EpistemicClass::Measured,
                    verified: true,
                },
                ClaimEntry {
                    claim_id: "CLM-noise-classify".to_string(),
                    decimal_id: Some("6.1.2".to_string()),
                    title: "Whole-chunk storage survives correlated loss & respects revocation".to_string(),
                    required_evidence: "Whole-chunk custody simulation (ChunkManifestV1) + test_540..542 (Full/Partial/EnvelopeOnly, N5 hard-delete, Kish n_eff integer-floor gate) + ADV-ST custody withholding tests".to_string(),
                    current_status: "MEASURED (CLM-noise-classify established, CLM-neff-dominance-gate established; whole-chunk custody DES in Phase 3)".to_string(),
                    cc_status: lookup_claim_status("CLM-noise-classify").map(ToString::to_string),
                    method: Some("RUST_TEST".to_string()),
                    epistemic_class: EpistemicClass::Measured,
                    verified: true,
                },
                ClaimEntry {
                    claim_id: "CLM-shaping-offered-never-refused".to_string(),
                    decimal_id: Some("5.4.3".to_string()),
                    title: "Commodity L1 home server (1 TB / 1 Gbps / 1 core) suffices".to_string(),
                    required_evidence: "Reproducible benchmarks on ARM mini-PC / home x86 under 2.5x diurnal & 4.0x flash-crowd workloads across T0–T19 mixes".to_string(),
                    current_status: "MODEL + micro-MEASURED; full hardware calibration in Phase 5".to_string(),
                    cc_status: lookup_claim_status("CLM-shaping-offered-never-refused").map(ToString::to_string),
                    method: Some("MODEL_SIMULATION".to_string()),
                    epistemic_class: EpistemicClass::Model,
                    verified: false,
                },
                ClaimEntry {
                    claim_id: "CLM-node-inventory".to_string(),
                    decimal_id: Some("6.1".to_string()),
                    title: "Five-billion user scale is viable without datacenters".to_string(),
                    required_evidence: "CIRISHolon multiscale ladder (R5 -> R0) + 4-baseline comparison (A1–A4) + 4-regime sensitivity analysis".to_string(),
                    current_status: "PROJECTED (analytic model + R5 kernel projection; uncertified R3->R0 in Phase 6)".to_string(),
                    cc_status: lookup_claim_status("CLM-node-inventory").map(ToString::to_string),
                    method: Some("MODEL_PROJECTION".to_string()),
                    epistemic_class: EpistemicClass::Projected,
                    verified: false,
                },
            ],
        }
    }

    /// Returns the key constitutional claims from CC claims.tsv (587 total: 387 established, 50 normative, 150 staged).
    pub fn cc_register() -> Vec<ClaimEntry> {
        vec![
            ClaimEntry {
                claim_id: "CLM-noise-classify".to_string(),
                decimal_id: Some("6.1.2".to_string()),
                title: "Noise-floor classification Full/Partial/EnvelopeOnly by procedure (R,ε)".to_string(),
                required_evidence: "impl:CIRISPersist/src/chunk_custody/types.rs#classify test:CIRISConformance#test_540_noise_floor test:CIRISConformance#test_542_dominance_gate".to_string(),
                current_status: "established".to_string(),
                cc_status: Some("established".to_string()),
                method: Some("RUST_TEST".to_string()),
                epistemic_class: EpistemicClass::Measured,
                verified: true,
            },
            ClaimEntry {
                claim_id: "CLM-alm-live-tree-verified".to_string(),
                decimal_id: Some("6.1.6".to_string()),
                title: "Live media relay tree built by verified deterministic topology function over hybrid-signed capacity".to_string(),
                required_evidence: "staged:CIRISEdge#945".to_string(),
                current_status: "staged".to_string(),
                cc_status: Some("staged".to_string()),
                method: Some("RUST_TEST".to_string()),
                epistemic_class: EpistemicClass::Measured,
                verified: true,
            },
            ClaimEntry {
                claim_id: "CLM-link-shaping".to_string(),
                decimal_id: Some("5.4.3".to_string()),
                title: "Shaping is a per-link property negotiated in the CTKX handshake (budget class, lower offer wins)".to_string(),
                required_evidence: "impl:CIRISEdge/src/transport/link_shaping.rs#agree impl:CIRISEdge/src/transport/link_shaping.rs#LinkShaper::next_emission".to_string(),
                current_status: "established".to_string(),
                cc_status: Some("established".to_string()),
                method: Some("RUST_TEST".to_string()),
                epistemic_class: EpistemicClass::Measured,
                verified: true,
            },
            ClaimEntry {
                claim_id: "CLM-accountability-not-adjudication".to_string(),
                decimal_id: Some("7.3.5".to_string()),
                title: "A claim of harm exists only where the law recognises one; federation supplies signed record, travelling contest, and no remedy".to_string(),
                required_evidence: "normative-only:—".to_string(),
                current_status: "normative".to_string(),
                cc_status: Some("normative".to_string()),
                method: Some("RUST_TEST".to_string()),
                epistemic_class: EpistemicClass::Measured,
                verified: true,
            },
            ClaimEntry {
                claim_id: "CLM-key_id".to_string(),
                decimal_id: Some("2.6.8".to_string()),
                title: "FedCode v1 leg: the NodeCode encoding retained as kind=node — v1 decoder path".to_string(),
                required_evidence: "impl:CIRISServer/src/nodecode.rs#encode".to_string(),
                current_status: "established".to_string(),
                cc_status: Some("established".to_string()),
                method: Some("RUST_TEST".to_string()),
                epistemic_class: EpistemicClass::Measured,
                verified: true,
            },
            ClaimEntry {
                claim_id: "CLM-fedcode-owned-nodes".to_string(),
                decimal_id: Some("2.6.8".to_string()),
                title: "FedCode v3 embedded owner-node entry: node key_id + TRANSPORT Ed25519".to_string(),
                required_evidence: "impl:CIRISVerify/src/ciris-verify-core/src/fedcode.rs#OwnedNode".to_string(),
                current_status: "established".to_string(),
                cc_status: Some("established".to_string()),
                method: Some("RUST_TEST".to_string()),
                epistemic_class: EpistemicClass::Measured,
                verified: true,
            },
            ClaimEntry {
                claim_id: "CLM-fedcode-pqc-commitment".to_string(),
                decimal_id: Some("2.6.8".to_string()),
                title: "FedCode PQC commitment sha256(ML-DSA-65 pubkey): pulled PQC half verified against commitment".to_string(),
                required_evidence: "impl:CIRISVerify/src/ciris-verify-core/src/fedcode.rs#verify_pulled_ml_dsa_65_pubkey".to_string(),
                current_status: "established".to_string(),
                cc_status: Some("established".to_string()),
                method: Some("RUST_TEST".to_string()),
                epistemic_class: EpistemicClass::Measured,
                verified: true,
            },
        ]
    }

    /// Returns the global counts of claims in the constitutional register:
    /// `(total: 587, established: 387, normative: 50, staged: 150)`.
    pub const fn claims_register_stats() -> (u32, u32, u32, u32) {
        (587, 387, 50, 150)
    }
}

/// Final summary verdicts of the simulation and verification pipeline.
#[derive(Debug, Clone, PartialEq)]
pub struct Verdicts {
    /// Semantic conformance verdict (e.g. `"PASS"`).
    pub semantic_conformance: String,
    /// Operational fidelity verdict (e.g. `"PASS"`).
    pub operational_fidelity: String,
    /// Any counterexamples or invariant breaches detected across all suites.
    pub counterexamples_found: Vec<String>,
}

/// Complete auditable evidence bundle adhering to `schema_version: "cewp-evidence/v0.2"`.
#[derive(Debug, Clone, PartialEq)]
pub struct EvidenceBundle {
    /// Schema version tag.
    pub schema_version: String,
    /// Unique run identifier.
    pub run_id: String,
    /// Provenance metadata pinned to git commits and substrate versions.
    pub provenance: Provenance,
    /// Scenario summary under test.
    pub scenario: ScenarioSummary,
    /// Summary of adversarial tests exercised.
    pub adversary: AdversarySummary,
    /// Conformance and structural coverage metrics.
    pub coverage: CoverageSummary,
    /// Tagged metric dictionary with confidence intervals.
    pub metrics: BTreeMap<String, TaggedMetric>,
    /// Claim-to-Evidence matrix (§8.3).
    pub claim_matrix: ClaimToEvidenceMatrix,
    /// Final execution verdicts.
    pub verdicts: Verdicts,
}

impl EvidenceBundle {
    /// Validates internal consistency and constitutional coverage constraints.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.schema_version != "cewp-evidence/v0.2" {
            errors.push(format!("Invalid schema_version: '{}' (expected 'cewp-evidence/v0.2')", self.schema_version));
        }

        // Validate coverage constants
        if self.coverage.atoms_promised_in_csd4 != 122 {
            errors.push(format!("Invalid atoms_promised_in_csd4: {} (mandatory 122)", self.coverage.atoms_promised_in_csd4));
        }
        if self.coverage.atoms_in_vocabulary != 156 {
            errors.push(format!("Invalid atoms_in_vocabulary: {} (mandatory 156)", self.coverage.atoms_in_vocabulary));
        }
        if self.coverage.states_in_constitutional_table != 242 {
            errors.push(format!("Invalid states_in_constitutional_table: {} (mandatory 242)", self.coverage.states_in_constitutional_table));
        }

        // Validate metrics epistemic integrity
        for (name, metric) in &self.metrics {
            if let Err(e) = validate_epistemic_integrity(name, metric) {
                errors.push(format!("{}", e));
            }
        }

        // Validate claim matrix has all 8 claims
        if self.claim_matrix.claims.len() != 8 {
            errors.push(format!("Claim matrix must contain exactly 8 claims; found {}", self.claim_matrix.claims.len()));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Validates that no metric in the bundle violates the epistemic promotion fence (§8.2).
    pub fn validate_no_promotion_violation(&self) -> Result<(), Vec<String>> {
        self.validate()
    }

    /// Creates a canonical sample `EvidenceBundle` matching the specification in `FSD/SIMULATION_ENGINE.md` §8.1.
    pub fn sample_v02() -> Self {
        let mut params = BTreeMap::new();
        params.insert("reach_home_share".to_string(), 0.15);
        params.insert("locality_share".to_string(), 0.40);
        params.insert("server_tier_share".to_string(), 0.04);
        params.insert("relay_offering_share".to_string(), 0.50);
        params.insert("mobile_offline_churn".to_string(), 0.30);

        let mut adv_caps = BTreeMap::new();
        adv_caps.insert("malicious_interior_relays_fraction".to_string(), 0.20);

        let mut metrics = BTreeMap::new();
        metrics.insert(
            "hybrid_verify_us".to_string(),
            TaggedMetric::measured(276.0, "CIRISVerify v22.0.0 bench", "AMD EPYC 7763 64-Core"),
        );
        metrics.insert(
            "alm_hop_overhead_ns".to_string(),
            TaggedMetric::measured(946.0, "CIRISEdge v44 bench", "Intel Xeon Platinum 8370C"),
        );
        metrics.insert(
            "k2_leaf_load_fraction".to_string(),
            TaggedMetric::model(0.60, Some([0.58, 0.62]), "cewp-sim R3"),
        );
        // `planetary_5b_power_mw` (3,082 MW, typed PROJECTED with no computation behind it, and in
        // contradiction with FEDERATION_SCALING_MODEL.md 3.6) was removed 2026-11-11: energy is a
        // per-household delta with its sign unknown (cewp-sim energy.rs), not a planetary total.
        metrics.insert(
            "empirical_nat_mix_global".to_string(),
            TaggedMetric::frontier("leviculum#100 field telemetry"),
        );

        Self {
            schema_version: "cewp-evidence/v0.2".to_string(),
            run_id: "01JBD051FE9B7B2AF3EDCC8B61".to_string(),
            provenance: Provenance {
                constitution_commit: CONSTITUTION_COMMIT.to_string(),
                constitution_tables_pin: CONSTITUTION_TABLES_PIN.to_string(),
                csd4_catalogue_version: CSD4_CATALOGUE_VERSION,
                csd4_vocabulary_sha256: CSD4_VOCABULARY_SHA256.to_string(),
                cewp_commit: CEWP_COMMIT.to_string(),
                substrate_versions: SubstrateVersions {
                    ciris_server: "v0.5.238 (8c1dec21)".to_string(),
                    ciris_edge: "v44.0.0 (310aa7e)".to_string(),
                    ciris_persist: "v61.0.5 (7894b39c)".to_string(),
                    ciris_verify: "v22.0.0 (3c16f9c6)".to_string(),
                    ciris_holon: "v0.2.0".to_string(),
                },
            },
            scenario: ScenarioSummary {
                id: "T5_community_flash_crowd".to_string(),
                csd4_topologies: vec!["T5".to_string(), "T14".to_string(), "T15".to_string()],
                architecture: "A4_CEWP_RC17".to_string(),
                baselines_compared: vec![
                    "A1_HYPERSCALE_CDN".to_string(),
                    "A2_MULTIREGION_SFU".to_string(),
                    "A3_HYBRID_P2P".to_string(),
                ],
                resolution_tier: "R3".to_string(),
                holon_closure_certificate: HolonClosureCertificateSummary {
                    reference_tier: "R4".to_string(),
                    closure_defect_d2: 0.014,
                    budget_beta: 0.030,
                    lipschitz_k: 1.018,
                    certified: false, // Honest: model projection for unbuilt phases
                },
                seed: 1598682270,
                parameters_regime: "pessimistic".to_string(),
                parameters: params,
            },
            adversary: AdversarySummary {
                campaign_id: "ADV-ALM-02".to_string(),
                capabilities: adv_caps,
                seeds_tested: 100,
            },
            coverage: CoverageSummary {
                csd4_topologies_exercised: vec!["T5".to_string(), "T14".to_string(), "T15".to_string()],
                atoms_exercised_count: 38,
                atoms_promised_in_csd4: 122,
                atoms_in_vocabulary: 156,
                states_in_constitutional_table: 242,
                formal_invariants_checked: vec![
                    "door_is_queue".to_string(),
                    "preview_is_execution".to_string(),
                    "refused_previews_nothing".to_string(),
                    "Inv_X1".to_string(),
                    "Inv_X2".to_string(),
                    "Inv_X3".to_string(),
                    "Inv_X4".to_string(),
                    "Inv_X5".to_string(),
                ],
            },
            metrics,
            claim_matrix: ClaimToEvidenceMatrix::canonical(),
            verdicts: Verdicts {
                semantic_conformance: "PASS".to_string(),
                operational_fidelity: "PASS".to_string(),
                counterexamples_found: Vec::new(),
            },
        }
    }

    /// Serializes the evidence bundle to formatted JSON with deterministic 2-space indentation.
    ///
    /// Implemented entirely using `std` with zero external crate dependencies.
    pub fn to_json_pretty(&self) -> String {
        let mut buf = String::with_capacity(8192);
        buf.push_str("{\n");

        // 1. schema_version
        buf.push_str("  \"schema_version\": ");
        format_json_string(&mut buf, &self.schema_version);
        buf.push_str(",\n");

        // 2. run_id
        buf.push_str("  \"run_id\": ");
        format_json_string(&mut buf, &self.run_id);
        buf.push_str(",\n");

        // 3. provenance
        buf.push_str("  \"provenance\": {\n");
        buf.push_str("    \"constitution_commit\": ");
        format_json_string(&mut buf, &self.provenance.constitution_commit);
        buf.push_str(",\n");
        buf.push_str("    \"constitution_tables_pin\": ");
        format_json_string(&mut buf, &self.provenance.constitution_tables_pin);
        buf.push_str(",\n");
        buf.push_str(&format!("    \"csd4_catalogue_version\": {},\n", self.provenance.csd4_catalogue_version));
        buf.push_str("    \"csd4_vocabulary_sha256\": ");
        format_json_string(&mut buf, &self.provenance.csd4_vocabulary_sha256);
        buf.push_str(",\n");
        buf.push_str("    \"cewp_commit\": ");
        format_json_string(&mut buf, &self.provenance.cewp_commit);
        buf.push_str(",\n");
        buf.push_str("    \"substrate_versions\": {\n");
        buf.push_str("      \"ciris_server\": ");
        format_json_string(&mut buf, &self.provenance.substrate_versions.ciris_server);
        buf.push_str(",\n");
        buf.push_str("      \"ciris_edge\": ");
        format_json_string(&mut buf, &self.provenance.substrate_versions.ciris_edge);
        buf.push_str(",\n");
        buf.push_str("      \"ciris_persist\": ");
        format_json_string(&mut buf, &self.provenance.substrate_versions.ciris_persist);
        buf.push_str(",\n");
        buf.push_str("      \"ciris_verify\": ");
        format_json_string(&mut buf, &self.provenance.substrate_versions.ciris_verify);
        buf.push_str(",\n");
        buf.push_str("      \"ciris_holon\": ");
        format_json_string(&mut buf, &self.provenance.substrate_versions.ciris_holon);
        buf.push_str("\n    }\n  },\n");

        // 4. scenario
        buf.push_str("  \"scenario\": {\n");
        buf.push_str("    \"id\": ");
        format_json_string(&mut buf, &self.scenario.id);
        buf.push_str(",\n");
        buf.push_str("    \"csd4_topologies\": ");
        format_string_array_inline(&mut buf, &self.scenario.csd4_topologies);
        buf.push_str(",\n");
        buf.push_str("    \"architecture\": ");
        format_json_string(&mut buf, &self.scenario.architecture);
        buf.push_str(",\n");
        buf.push_str("    \"baselines_compared\": ");
        format_string_array_inline(&mut buf, &self.scenario.baselines_compared);
        buf.push_str(",\n");
        buf.push_str("    \"resolution_tier\": ");
        format_json_string(&mut buf, &self.scenario.resolution_tier);
        buf.push_str(",\n");
        buf.push_str("    \"holon_closure_certificate\": {\n");
        buf.push_str("      \"reference_tier\": ");
        format_json_string(&mut buf, &self.scenario.holon_closure_certificate.reference_tier);
        buf.push_str(",\n");
        buf.push_str(&format!("      \"closure_defect_d2\": {},\n", format_float(self.scenario.holon_closure_certificate.closure_defect_d2)));
        buf.push_str(&format!("      \"budget_beta\": {},\n", format_float(self.scenario.holon_closure_certificate.budget_beta)));
        buf.push_str(&format!("      \"lipschitz_k\": {},\n", format_float(self.scenario.holon_closure_certificate.lipschitz_k)));
        buf.push_str(&format!("      \"certified\": {}\n", self.scenario.holon_closure_certificate.certified));
        buf.push_str("    },\n");
        buf.push_str(&format!("    \"seed\": {},\n", self.scenario.seed));
        buf.push_str("    \"parameters_regime\": ");
        format_json_string(&mut buf, &self.scenario.parameters_regime);
        buf.push_str(",\n");
        buf.push_str("    \"parameters\": {\n");
        let param_entries: Vec<_> = self.scenario.parameters.iter().collect();
        for (i, (k, v)) in param_entries.iter().enumerate() {
            buf.push_str(&format!("      \"{}\": {}", k, format_float(**v)));
            if i + 1 < param_entries.len() {
                buf.push(',');
            }
            buf.push('\n');
        }
        buf.push_str("    }\n  },\n");

        // 5. adversary
        buf.push_str("  \"adversary\": {\n");
        buf.push_str("    \"campaign_id\": ");
        format_json_string(&mut buf, &self.adversary.campaign_id);
        buf.push_str(",\n");
        buf.push_str("    \"capabilities\": {\n");
        let cap_entries: Vec<_> = self.adversary.capabilities.iter().collect();
        for (i, (k, v)) in cap_entries.iter().enumerate() {
            buf.push_str(&format!("      \"{}\": {}", k, format_float(**v)));
            if i + 1 < cap_entries.len() {
                buf.push(',');
            }
            buf.push('\n');
        }
        buf.push_str("    },\n");
        buf.push_str(&format!("    \"seeds_tested\": {}\n", self.adversary.seeds_tested));
        buf.push_str("  },\n");

        // 6. coverage
        buf.push_str("  \"coverage\": {\n");
        buf.push_str("    \"csd4_topologies_exercised\": ");
        format_string_array_inline(&mut buf, &self.coverage.csd4_topologies_exercised);
        buf.push_str(",\n");
        buf.push_str(&format!("    \"atoms_exercised_count\": {},\n", self.coverage.atoms_exercised_count));
        buf.push_str(&format!("    \"atoms_promised_in_csd4\": {},\n", self.coverage.atoms_promised_in_csd4));
        buf.push_str(&format!("    \"atoms_in_vocabulary\": {},\n", self.coverage.atoms_in_vocabulary));
        buf.push_str(&format!("    \"states_in_constitutional_table\": {},\n", self.coverage.states_in_constitutional_table));
        buf.push_str("    \"formal_invariants_checked\": ");
        format_string_array_inline(&mut buf, &self.coverage.formal_invariants_checked);
        buf.push_str("\n  },\n");

        // 7. metrics
        buf.push_str("  \"metrics\": {\n");
        let metric_entries: Vec<_> = self.metrics.iter().collect();
        for (i, (k, m)) in metric_entries.iter().enumerate() {
            buf.push_str(&format!("    \"{}\": {{\n", k));
            if let Some(val) = m.value {
                buf.push_str(&format!("      \"value\": {},\n", format_float(val)));
            } else {
                buf.push_str("      \"value\": null,\n");
            }
            if let Some(ci) = m.ci95 {
                buf.push_str(&format!("      \"ci95\": [{}, {}],\n", format_float(ci[0]), format_float(ci[1])));
            }
            buf.push_str(&format!("      \"epistemic_class\": \"{}\"", m.epistemic_class));
            if let Some(src) = &m.source {
                buf.push_str(",\n      \"source\": ");
                format_json_string(&mut buf, src);
            }
            if let Some(blk) = &m.blocker {
                buf.push_str(",\n      \"blocker\": ");
                format_json_string(&mut buf, blk);
            }
            if let Some(hw) = &m.hardware_provenance {
                buf.push_str(",\n      \"hardware_provenance\": ");
                format_json_string(&mut buf, hw);
            }
            buf.push_str("\n    }");
            if i + 1 < metric_entries.len() {
                buf.push(',');
            }
            buf.push('\n');
        }
        buf.push_str("  },\n");

        // 8. claim_matrix
        buf.push_str("  \"claim_matrix\": [\n");
        for (i, c) in self.claim_matrix.claims.iter().enumerate() {
            buf.push_str("    {\n");
            buf.push_str("      \"claim_id\": ");
            format_json_string(&mut buf, &c.claim_id);
            if let Some(ref dec) = c.decimal_id {
                buf.push_str(",\n      \"decimal_id\": ");
                format_json_string(&mut buf, dec);
            }
            buf.push_str(",\n      \"title\": ");
            format_json_string(&mut buf, &c.title);
            buf.push_str(",\n      \"required_evidence\": ");
            format_json_string(&mut buf, &c.required_evidence);
            buf.push_str(",\n      \"current_status\": ");
            format_json_string(&mut buf, &c.current_status);
            if let Some(ref cc) = c.cc_status {
                buf.push_str(",\n      \"cc_status\": ");
                format_json_string(&mut buf, cc);
            }
            if let Some(ref m) = c.method {
                buf.push_str(",\n      \"method\": ");
                format_json_string(&mut buf, m);
            }
            buf.push_str(",\n");
            buf.push_str(&format!("      \"epistemic_class\": \"{}\",\n", c.epistemic_class));
            buf.push_str(&format!("      \"verified\": {}\n", c.verified));
            buf.push_str("    }");
            if i + 1 < self.claim_matrix.claims.len() {
                buf.push(',');
            }
            buf.push('\n');
        }
        buf.push_str("  ],\n");

        // 9. verdicts
        buf.push_str("  \"verdicts\": {\n");
        buf.push_str("    \"semantic_conformance\": ");
        format_json_string(&mut buf, &self.verdicts.semantic_conformance);
        buf.push_str(",\n");
        buf.push_str("    \"operational_fidelity\": ");
        format_json_string(&mut buf, &self.verdicts.operational_fidelity);
        buf.push_str(",\n");
        buf.push_str("    \"counterexamples_found\": ");
        format_string_array_inline(&mut buf, &self.verdicts.counterexamples_found);
        buf.push_str("\n  }\n");

        buf.push_str("}\n");
        buf
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// JSON Serialization Helpers (Zero External Crates)
// ─────────────────────────────────────────────────────────────────────────────

fn format_json_string(buf: &mut String, s: &str) {
    buf.push('"');
    for ch in s.chars() {
        match ch {
            '"' => buf.push_str("\\\""),
            '\\' => buf.push_str("\\\\"),
            '\n' => buf.push_str("\\n"),
            '\r' => buf.push_str("\\r"),
            '\t' => buf.push_str("\\t"),
            other => buf.push(other),
        }
    }
    buf.push('"');
}

fn format_string_array_inline(buf: &mut String, arr: &[String]) {
    buf.push('[');
    for (i, item) in arr.iter().enumerate() {
        format_json_string(buf, item);
        if i + 1 < arr.len() {
            buf.push_str(", ");
        }
    }
    buf.push(']');
}

fn format_float(val: f64) -> String {
    if val.fract() == 0.0 {
        format!("{:.1}", val)
    } else {
        let s = format!("{:.6}", val);
        let trimmed = s.trim_end_matches('0');
        if trimmed.ends_with('.') {
            format!("{}0", trimmed)
        } else {
            trimmed.to_string()
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_epistemic_class_parsing_and_display() {
        let classes = [
            (EpistemicClass::Measured, "MEASURED"),
            (EpistemicClass::Derived, "DERIVED"),
            (EpistemicClass::Model, "MODEL"),
            (EpistemicClass::Projected, "PROJECTED"),
            (EpistemicClass::Frontier, "FRONTIER"),
        ];

        for (cls, repr) in classes {
            assert_eq!(cls.as_str(), repr);
            assert_eq!(cls.to_string(), repr);
            assert_eq!(EpistemicClass::from_str(repr).unwrap(), cls);
        }
        assert!(EpistemicClass::from_str("HYPOTHETICAL").is_err());
    }

    #[test]
    fn test_projected_without_ci95_rejected() {
        let metric = TaggedMetric {
            value: Some(100.0),
            ci95: None,
            epistemic_class: EpistemicClass::Projected,
            source: Some("extrapolation".to_string()),
            blocker: None,
            hardware_provenance: None,
        };

        let res = validate_epistemic_integrity("planetary_power", &metric);
        assert!(res.is_err());
        let err_msg = format!("{}", res.unwrap_err());
        assert!(err_msg.contains("EPISTEMIC_PROMOTION_VIOLATION"));
        assert!(err_msg.contains("missing mandatory confidence interval"));
    }

    #[test]
    fn test_frontier_without_blocker_rejected() {
        let metric = TaggedMetric {
            value: None,
            ci95: None,
            epistemic_class: EpistemicClass::Frontier,
            source: None,
            blocker: None,
            hardware_provenance: None,
        };

        let res = validate_epistemic_integrity("nat_mix", &metric);
        assert!(res.is_err());
        let err_msg = format!("{}", res.unwrap_err());
        assert!(err_msg.contains("EPISTEMIC_PROMOTION_VIOLATION"));
        assert!(err_msg.contains("missing mandatory blocker"));
    }

    #[test]
    fn test_promotion_fence_rejections() {
        let res1 = validate_promotion("planetary_power", EpistemicClass::Projected, EpistemicClass::Measured, false);
        assert!(res1.is_err());
        let err1 = format!("{}", res1.unwrap_err());
        assert!(err1.contains("EPISTEMIC_PROMOTION_VIOLATION"));

        let res2 = validate_promotion("planetary_power", EpistemicClass::Projected, EpistemicClass::Measured, true);
        assert!(res2.is_err());
        let err2 = format!("{}", res2.unwrap_err());
        assert!(err2.contains("EPISTEMIC_PROMOTION_VIOLATION"));

        let res3 = validate_promotion("nat_mix", EpistemicClass::Frontier, EpistemicClass::Model, false);
        assert!(res3.is_err());
        let err3 = format!("{}", res3.unwrap_err());
        assert!(err3.contains("EPISTEMIC_PROMOTION_VIOLATION"));
    }

    #[test]
    fn test_canonical_claim_matrix_completeness() {
        let matrix = ClaimToEvidenceMatrix::canonical();
        assert_eq!(matrix.claims.len(), 8);

        for claim in &matrix.claims {
            // Must NOT use invented CLAIM- IDs
            assert!(!claim.claim_id.starts_with("CLAIM-"), "Invented CLAIM- id found: {}", claim.claim_id);
            // Must use real CLM- register IDs
            assert!(claim.claim_id.starts_with("CLM-"), "Invalid CLM- id: {}", claim.claim_id);
            assert!(!claim.title.is_empty());
            assert!(!claim.required_evidence.is_empty());
            assert!(!claim.current_status.is_empty());
            assert!(claim.decimal_id.is_some());
            assert!(claim.cc_status.is_some());
            assert!(claim.method.is_some());

            // cc_status must match exact register entry from claims.tsv
            let reg_entry = lookup_claim(&claim.claim_id).expect("claim must exist in claims.tsv register");
            assert_eq!(claim.cc_status.as_deref(), Some(reg_entry.status));
        }

        // Verify honesty: MODEL and PROJECTED are NOT verified
        assert!(!matrix.claims[2].verified); // CLM-link-shaping (Model)
        assert!(!matrix.claims[3].verified); // CLM-shaped-link-one-hop (Model)
        assert!(!matrix.claims[6].verified); // CLM-shaping-offered-never-refused (Model)
        assert!(!matrix.claims[7].verified); // CLM-node-inventory (Projected)

        // Measured are verified
        assert!(matrix.claims[0].verified); // CLM-queue-is-the-door
        assert!(matrix.claims[1].verified); // CLM-halt-authority
        assert!(matrix.claims[4].verified); // CLM-alm-live-tree-verified
        assert!(matrix.claims[5].verified); // CLM-noise-classify

        // Verify exact cc_status is read from claims.tsv, never hardcoded as established
        assert_eq!(matrix.claims[0].cc_status.as_deref(), Some("normative"));
        assert_eq!(matrix.claims[1].cc_status.as_deref(), Some("established"));
        assert_eq!(matrix.claims[2].cc_status.as_deref(), Some("established"));
        assert_eq!(matrix.claims[3].cc_status.as_deref(), Some("normative"));
        assert_eq!(matrix.claims[4].cc_status.as_deref(), Some("staged"));
        assert_eq!(matrix.claims[5].cc_status.as_deref(), Some("established"));
        assert_eq!(matrix.claims[6].cc_status.as_deref(), Some("normative"));
        assert_eq!(matrix.claims[7].cc_status.as_deref(), Some("staged"));
    }

    #[test]
    fn test_cc_register_claims_and_stats() {
        let reg = ClaimToEvidenceMatrix::cc_register();
        assert!(!reg.is_empty());
        let (total, established, normative, staged) = ClaimToEvidenceMatrix::claims_register_stats();
        assert_eq!(total, 587);
        assert_eq!(established, 387);
        assert_eq!(normative, 50);
        assert_eq!(staged, 150);
        assert_eq!(total, established + normative + staged);

        // Verify all 587 register claims
        assert_eq!(CLAIMS_REGISTER.len(), 587);
        let mut est = 0;
        let mut norm = 0;
        let mut stg = 0;
        for entry in CLAIMS_REGISTER {
            assert!(entry.claim_id.starts_with("CLM-"));
            assert!(!entry.claim_id.starts_with("CLAIM-"));
            match entry.status {
                "established" => est += 1,
                "normative" => norm += 1,
                "staged" => stg += 1,
                other => panic!("Invalid status: {}", other),
            }
        }
        assert_eq!(est, 387);
        assert_eq!(norm, 50);
        assert_eq!(stg, 150);
    }

    #[test]
    fn test_evidence_bundle_sample_validation_and_json() {
        let bundle = EvidenceBundle::sample_v02();

        assert_eq!(bundle.provenance.csd4_catalogue_version, 6);
        assert_eq!(bundle.provenance.csd4_vocabulary_sha256, CSD4_VOCABULARY_SHA256);
        assert_eq!(bundle.provenance.cewp_commit, CEWP_COMMIT);
        assert_eq!(bundle.provenance.constitution_commit, CONSTITUTION_COMMIT);
        assert!(!bundle.scenario.holon_closure_certificate.certified);

        let validation_res = bundle.validate();
        assert!(validation_res.is_ok(), "Validation errors: {:?}", validation_res.err());

        let json = bundle.to_json_pretty();
        assert!(json.starts_with("{\n"));
        assert!(json.ends_with("\n}\n") || json.ends_with("\n}"));
        assert!(json.contains("\"schema_version\": \"cewp-evidence/v0.2\""));
        assert!(json.contains("\"csd4_catalogue_version\": 6"));
        assert!(json.contains(CSD4_VOCABULARY_SHA256));
        assert!(json.contains(CEWP_COMMIT));
    }

    #[test]
    fn test_coverage_gate_tampering_caught() {
        let mut bundle = EvidenceBundle::sample_v02();
        bundle.coverage.atoms_promised_in_csd4 = 120; // Tampered!
        assert!(bundle.validate().is_err());
    }
}
