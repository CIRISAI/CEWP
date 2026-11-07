//! `cewp-csd4`: Holistic CSD/4 Scenario Contract, Step-Window Model,
//! Realizability/Card Validator, and Cost-Minimizing Planner.
//!
//! Phase 2 deliverable of `FSD/SIMULATION_ENGINE.md` (v0.2):
//! - Closed Catalogue (`T0`–`T19`) with exact metadata, steps, traces, and transient windows.
//! - Cross-Plane Spine Invariants (`X1`–`X5`): Latch, Evicted Key Authority, Replication & Consent, Local Tier, Atomic Leave.
//! - Realizability Rules (`R1`–`R7`) & Card Refusal Rules (`R1`–`R5`, including `R5_ATOMS_NEVER_COHOLD`).
//! - Subset-Search Cost-Minimizing Planner (`plan_tests`) with folding optimization (folding `T0` into `T2`).
//! - Explicit Coverage Reporter (`CoverageReport`): `(atoms_exercised / 122 promised / 156 vocabulary / 242 constitutional_state_table)`.
//! - Three-Mode Runner Contract (`Formal`, `Sim`, `Native`), `rows.jsonl` trace exporter, and Checkpoint `MODEL_DRIFT` witness.

pub mod catalogue;
pub mod windows;
pub mod invariants;
pub mod realizability;
pub mod planner;
pub mod coverage;
pub mod bridge;
pub mod namespace_map;

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
