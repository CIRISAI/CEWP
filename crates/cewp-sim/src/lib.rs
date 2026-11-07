//! CEWP Simulation, Verification, and Evidence Engine (`cewp-sim`).
//!
//! Implements Layers 3, 4 & 6 of `FSD/SIMULATION_ENGINE.md` (v0.2):
//!
//! 1. `transport` — Constitutional Transport Contract (CC §5.4.3 & §5.4.6, Tiers.lean, claims.tsv):
//!    - Budget classes (Off, PhoneCellular, PhoneWifi, Home, Server, Seat)
//!    - Shaped-link bound (L <= 8) & rank-based displacement
//!    - One direct hop & reach ≠ capacity separate failure counters
//!    - Sealed referrals (up to 3) & proxy pull
//!    - 64 KiB shaped-link payload boundary & exact refusal string
//!    - Carriage axis (ConsentOnly vs Full) with 16 consent-free envelope kinds
//!    - Anonymous Tier (3-hop CONP, floor >= 3 answering relays, 2-member room-mailbox)
//!
//! 2. `alm` — Deterministic Application-Layer Multicast (CC §6.1.6, CLM-deterministic-alm):
//!    - Pure integer-only `compute_alm_topology_verified` (zero float branches)
//!    - Signed capacity verification & self-assertion clamping
//!    - Depth-primary parent selection (v2: depth ≈ ⌈log_k N⌉)
//!    - MDC sub-path diversity penalty & cycle prevention
//!    - All 15 frozen vectors verified
//!
//! 3. `fountain` — Symbol-Rank Storage & Noise-Floor Descent (CC §6.1.2, CLM-noise-classify):
//!    - `FountainManifestV1` & per-node symbol holdings with adversarial flags
//!    - Full / Partial / EnvelopeOnly noise floor states
//!    - Revocation dominance (CLM-noise-N5: Revoked -> EjectHardDelete)
//!    - Proactive trimming (`should_eject_above_target`) at 15% safety margin
//!    - Kish mass-dominance gate & R9 multiplicity gate
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
pub mod fountain;
pub mod workload;
pub mod holon;

pub use transport::*;
pub use alm::*;
pub use fountain::*;
pub use workload::*;
pub use holon::*;
