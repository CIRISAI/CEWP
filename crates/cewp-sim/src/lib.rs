//! CEWP Simulation, Verification, and Evidence Engine (`cewp-sim`).
//!
//! Implements Layers 3, 4 & 6 of `FSD/SIMULATION_ENGINE.md` (v0.2):
//!
//! 1. `transport` — Constitutional Transport Contract (CC §5.4.3 & §5.4.6, Tiers.lean, claims.tsv):
//!    - Budget classes (Off, PhoneCellular, PhoneWifi, Home, Server, Seat)
//!    - Shaped-link bound (L <= 8) & rank-based displacement
//!    - One direct hop & reach ≠ capacity separate failure counters
//!    - Sealed referrals (up to 3) & proxy pull with federation-tier gate
//!    - 64 KiB shaped-link payload boundary & exact refusal string
//!    - Carriage axis (ConsentOnly vs Full) with 16 consent-free envelope kinds
//!    - Anonymous Tier (3-hop CONP, floor >= 3 answering relays, 2-member room-mailbox)
//!    - Two sticky guards for reach from consented contacts & serve seat guard of last resort (CC §5.4.6)
//!    - Shaping defaults (Server on, off elsewhere) & 10-minute idle timeout on asked shaped links (CC §5.4.3 / §5.4.6)
//!    - Phone relay gate: only while charging AND on unmetered link (CC §5.4.6)
//!
//! 2. `alm` — Deterministic Application-Layer Multicast (CC §6.1.6, CC §5.3.3.4, CLM-deterministic-alm):
//!    - Pure integer-only `compute_alm_topology_verified` (zero float branches)
//!    - Single publisher root via stream key binding (publisher is only member at depth 0)
//!    - Depth-primary parent selection via single-root BFS traversal with locality preference
//!    - Layered multipath (`layer_parents`) and sibling pull (`sibling_pull`) for complete reassembly (CC §5.3.3.4)
//!    - Presence floor layer (`floor_bps` = 32 B/s) ensuring no sender is excluded for bandwidth
//!    - Replay vectors from `Alm.lean` + `alm_model.py`: loss permille 49/141/226/569, refetch cheaper at 1%, live senders 19531/1953/12
//!    - MDC sub-path diversity penalty & cycle prevention
//!    - Single-publisher member relay tree (simulation fan-out cap assumption $k \le 8$; CC §6.1.6 pins capacity preimages)
//!
//! 3. `chunk_custody` — Whole-Chunk Storage & Layered Fidelity Model (CC §5.3.3.2.3, §5.4.3, §6.1.2, §6.1.5):
//!    - Whole-chunk storage contract (`ChunkManifestV1`, `WholeChunkManifestV1`) with `custody:ack:v1`
//!    - Layered-codec fidelity classification over `ChunkLayer` sets (R, ε)
//!    - Full / Partial / EnvelopeOnly noise floor states
//!    - Revocation dominance (CLM-noise-N5: Revoked -> EjectHardDelete)
//!    - Proactive trimming (`should_eject_above_target`) at 15% safety margin
//!    - Kish mass-dominance gate (N_EFF_MIN = 3) & R9 multiplicity gate (R9_HOLDER_FLOOR = 9) via integer floor
//!    - Unit tests for test_540, test_541, and test_542
//!
//! 4. `workload` — Compound-Poisson Workload & 4 Baselines (FSD §4):
//!    - Dimensional arrival rate λ(t) and timestep arrival probability P(K >= 1)
//!    - Diurnal activity multiplier normalized to 1.0
//!    - IMIX payload sampling across 6 categories
//!    - 4 Baseline architectures (A1_HYPERSCALE_CDN, A2_MULTIREGION_SFU, A3_HYBRID_P2P, A4_CEWP_RC17)
//!    - 4 Parameter regimes (Optimistic, Expected, Pessimistic, Adversarial)
//!
//! 5. `holon` — CIRISHolon Coarse-Graining Certification Contracts:
//!    - `ClosureCertificate` (D_v^2 <= β)
//!    - `WitnessPair` detection for non-factoring states
//!    - `RemovabilityGate` with `PriceCertificate` vs `MaintenanceRent` (G_inf)
//!    - `TierStack::compose`
//!    - Non-expansive Lipschitz gate (K <= 1.05) & `horizon_budget`
//!    - `SecondSquareFence` refusing uncertified claim transport

pub mod transport;
pub mod alm;
pub mod chunk_custody;
pub mod workload;
pub mod holon;
pub mod holon_dist;
pub mod holon_demo;
pub mod holon_view;
pub mod micro;
pub mod tiling;
pub mod capacity;
pub mod reach;
pub mod netgraph;
pub mod social;
pub mod activity;
pub mod locality;
pub mod people_hooks;
pub mod seat_calibration;
pub mod seat_hooks;
pub mod link;
pub mod media;
pub mod media_view;
pub mod media_call2;
pub mod media_thin;
pub mod qoe;
pub mod failure;
pub mod durability;
pub mod failure_view;
pub mod seat_view;

pub use transport::*;
pub use alm::*;
pub use chunk_custody::*;
pub use chunk_custody as chunk_storage;
pub use workload::*;
pub use holon::*;
