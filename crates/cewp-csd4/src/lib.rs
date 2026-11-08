//! `cewp-csd4`: Holistic CSD/4 Scenario Contract, Step-Window Model,
//! Realizability/Card Validator, and Cost-Minimizing Planner.
//!
//! # CSD/4 Architecture
//! **CSD/4** stands for **Card-first Specification Design (4-field card format: `shows` / `reads` / `emits` / `checks`)**.
//! It grounds UX and protocol interaction designs directly in executable constitutional semantics,
//! bounded by the **Cornerstone Invariants `X1–X5`** as formal cross-plane safety properties:
//! - `X1`: Halt is not a trust leg (`accord_halt` freezes agent operations without creating a trust edge).
//! - `X2`: Evicted roster key's authority ends (revoked/evicted key cannot exercise authority after eviction).
//! - `X3`: Replication needs peer's consent (no replication without bilateral consent / custody agreement).
//! - `X4`: Local-tier row never leaves node (`local_only` / local-tier rows never cross a federation link).
//! - `X5`: Leaving a root is one act (unilateral exit from a root requires no counter-signature).
//!
//! # Card Counts Provenance (101 vs 105)
//! - `CIRISClient`: **101 CSD cards** across 22 files (**43 CSD/4 four-field cards** + **58 earlier bullet cards**).
//! - `CEWPOS`: **4 cards** (`CSD-CEWPOS-001..004`) for node OS and runtime primitives.
//! - **Total**: **105 cards**.
//!
//! # Catalogue Specifications (Catalogue Version 6)
//! - Closed Catalogue (`T0`–`T19`) with 110 distinct promised atoms across entries.
//! - Complete vocabulary of 156 atoms (152 mapped to `formal/STATE_TABLE.tsv`, 242 total constitutional states).
//! - 6 replication durability atoms (`file.durable.*`) awaiting `T20` (`CIRISServer#809`).
//! - Realizability Rules (`R1`–`R7`) & Card Refusal Rules (`R1`–`R5`, including `R5_ATOMS_NEVER_COHOLD`).
//! - Subset-Search Cost-Minimizing Planner (`plan_tests`) with folding optimization (folding `T0` into `T2`).
//! - Explicit Coverage Reporter (`CoverageReport`): `(atoms_exercised / 110 promised / 156 vocabulary / 242 constitutional_state_table)`.
//! - Three-Mode Runner Contract (`Formal`, `Sim`, `Native`), `rows.jsonl` trace exporter, and Checkpoint `MODEL_DRIFT` witness.

pub mod atoms;
pub mod cards;
pub mod catalogue;
pub mod windows;
pub mod invariants;
pub mod realizability;
pub mod planner;
pub mod coverage;
pub mod bridge;
pub mod namespace_map;

pub use atoms::{
    is_vocabulary_atom, is_state_table_atom, is_awaiting_t20_durable_atom,
    TOTAL_STATE_TABLE_ATOMS, NON_STATE_TABLE_ATOMS,
};
pub use cards::{
    CIRIS_CLIENT_CARD_COUNT, CIRIS_CLIENT_CSD4_FOUR_FIELD_CARDS,
    CIRIS_CLIENT_EARLIER_BULLET_CARDS, CEWPOS_CARD_COUNT, TOTAL_CSD_CARDS,
    CornerstoneInvariant,
};
pub use catalogue::{
    TopologyId, TopologyDecl, WindowDecl, StepDecl, TraceStepDecl, RootDecl, CanonicalDecl,
    NodeDecl, PersonDecl, RelationDecl, NegativeDecl, ActorDecl, TOPOLOGIES, VOCABULARY_ATOMS,
    AWAITING_T20_DURABLE_ATOMS, TOTAL_PROMISED_ATOMS, TOTAL_VOCABULARY_ATOMS,
    TOTAL_CONSTITUTIONAL_STATE_TABLE, CATALOGUE_VERSION,
};
pub use windows::{
    compute_transient_windows, simulate_steps, atoms_co_hold_in_topology,
    co_holding_step_indices, step_in_window, active_atoms_at_step,
};
pub use invariants::{
    check_inv_x1_latch_does_not_block_serving,
    check_inv_x2_evicted_key_authority_ends,
    check_inv_x3_replication_needs_consent_and_revocation_is_boundary,
    check_inv_x4_local_tier_never_leaves_node,
    check_inv_x5_leave_root_atomic,
    SpineViolation, HaltServingState, KeyRosterState, ConsentReplicationState,
    RowTransmissionState, LeaveRootState, RowTier,
};
pub use realizability::{
    check_topology_realizability, check_csd_card, TopologyRefusal, CardRefusal,
    CsdCardDecl, CardTestDecl,
};
pub use planner::{
    plan_tests, CsdTestRequest, Plan, PlanCost, PlannerOptions,
};
pub use coverage::CoverageReport;
pub use bridge::{
    ExecutionMode, FederationRow, CheckpointSummary, ModelDriftWitness,
    export_rows_jsonl, sha256_digest,
};
pub use namespace_map::{
    CsdNamespaceMap, NamespaceSummary, CsdShowsField, CsdCardInfo,
    NamespaceFamilyInfo, TopologyMapping, VENDORED_CSD_NAMESPACE_MAP_JSON,
};
