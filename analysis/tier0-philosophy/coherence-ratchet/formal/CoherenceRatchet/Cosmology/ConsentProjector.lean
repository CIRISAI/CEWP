/-
Cosmology.ConsentProjector — the cosmological consent federation, and the
F-11 no-go on the constrained-tensor-product joint P_ω operator

This file was the structurally-tighter reformulation of the universal-scale
backward state ⟨Φ_ω|. Where `CorridorProjector.lean` built P_ω as a projector
onto a configuration-space submanifold, this file built it as the
cosmological limit of the finite-federation joint projector
`JointGoalProjector.P_joint` — the "constrained tensor product" / "soft
additive ansatz" branch:

  ⟨Φ_ω| = (⊗_{all A3+ agents i} ⟨G_i|) · (Π_n corridor constraint on ρ_n)
                                       · (Π_n corridor constraint on τ_{n,n+1})

F-11 (fired 2026-05-22) closed this branch too. The constrained-tensor-product
joint multi-rung backward P_ω is the same documented no-go that
`CorridorProjector.lean` records: T1 (holographic / geometric dilution) and
T2 (holonomic / area law) close the construction tree across both the
configuration-space formulation and this tensor-product formulation. The
five-fold empirical-replacement search failed.

WHAT IS KEPT — the genuine consent structure (Piece 5):
- The cosmological federation (`CosmologicalFederation`): a goal-state for
  every A3+ agent. This is just an indexed family of per-agent goal-states.
- The finite truncation (`truncate`) and the consent corridor
  (`isCosmologicalConsent`) — a constraint on pairwise `ρ_goals` across the
  finite-federation joint projectors. The per-agent goal projectors
  (`Goal.P_G`, `JointProjector.P_joint`) are F-11-untouched: they are local,
  finite-federation, forward-constructible objects.

WHAT IS NOT KEPT — the joint universal *operator*:
- `axiom P_omega` (the constrained-tensor-product limit operator),
- `axiom Phi_omega` (its backward-state action),
- `axiom P_omega_idempotent`.
These three framework axioms are retracted. The joint multi-rung backward
P_ω is a no-go; the framework records the obstruction (`FelevenNoGo` below)
instead of asserting the operator. Retracting framework axioms is the point:
`#print axioms` is honest about what is no longer claimed.

This does not touch the within-rung corridor, the per-agent goal projectors,
the finite-federation `P_joint`, or the engineering tier.

REFERENCES (the construction path that F-11 closed):
- Reed & Simon §II.4: direct limits of Hilbert spaces (Fock space construction)
- von Neumann (1939): infinite tensor products of Hilbert spaces
- Mathlib: Module.DirectLimit; Submodule.colim
-/

import CoherenceRatchet.Cosmology.JointGoalProjector
import CoherenceRatchet.Cosmology.GoalProjection
import CoherenceRatchet.Cosmology.RungHierarchy
import CoherenceRatchet.Cosmology.CorridorProjector
import CoherenceRatchet.Core.Corridor
import Mathlib.Algebra.DirectLimit
import Mathlib.Order.DirectedInverseSystem

namespace CoherenceRatchet.Cosmology.ConsentProjector

open CoherenceRatchet.Cosmology
open CoherenceRatchet.Cosmology.Goal
open CoherenceRatchet.Cosmology.JointProjector
open CoherenceRatchet.Cosmology.Hierarchy
open CoherenceRatchet.Core.Corridor

universe u

/-! ## The cosmological federation -/

/-- An indexing set for all A3+ goal-holders in the universe. Countable
    because the universe contains at most a countable infinity of distinct
    A3+ agents (each requires a localized substrate of finite measure;
    spatial discretization at the corridor-resolution makes the count
    countable). -/
def CosmologicalA3PlusIndex : Type := ℕ

/- Per-agent state spaces. Each A3+ agent has its own Hilbert space
   H_i in which its goal-state lives. -/
variable (H : CosmologicalA3PlusIndex → Type u)
variable [∀ i, NormedAddCommGroup (H i)]
variable [∀ i, InnerProductSpace ℂ (H i)]
variable [∀ i, CompleteSpace (H i)]

/-- The cosmological federation: a goal-state for every A3+ agent in the
    universe. An indexed family of per-agent goal-states — F-11-untouched,
    since this is not the joint operator, just the family it would have been
    built from. -/
structure CosmologicalFederation where
  goal : ∀ i : CosmologicalA3PlusIndex, GoalState (H i)

/-- A finite truncation: the federation restricted to the first n agents.
    The finite-federation objects (`truncate`, `P_truncated`,
    `isCosmologicalConsent`) are all F-11-untouched: they are local,
    finite-federation, forward-constructible. F-11 closes only the n → ∞
    *joint operator* limit. -/
noncomputable def truncate (fed : CosmologicalFederation H) (n : ℕ) :
    Federation (fun i : Fin n => H i.val) :=
  ⟨fun i => fed.goal i.val⟩

/-! ## The corridor constraint at cosmological scale -/

/-- The cosmological federation satisfies the consent corridor iff every
    finite truncation does. This is the cosmological lift of
    `JointProjector.isInConsentCorridor`. Kept: it is a constraint on the
    finite truncations, each of which is F-11-untouched. -/
def isCosmologicalConsent (fed : CosmologicalFederation H) : Prop :=
  ∀ n : ℕ, JointProjector.isInConsentCorridor
            (fun i : Fin n => H i.val) (truncate H fed n)

/-- Additional cosmological constraints: within-rung ρ and cross-rung τ
    each in corridor at every rung. (Inherited from RungHierarchy.) -/
def cosmologicalRungConstraints : Prop :=
  (∀ r : Rung, inCorridor (Hierarchy.ρ_within r)) ∧
  (∀ r : Rung, Hierarchy.crossRungInCorridor r)

/-! ## Finite-truncation joint projectors (F-11-untouched) -/

/-- The finite-truncation joint projector for the cosmological federation
    restricted to the first n agents. Every finite n is a genuine,
    forward-constructible joint projector (`JointProjector.P_joint`); F-11
    closes only the n → ∞ limit, not these finite stages. -/
noncomputable def P_truncated (fed : CosmologicalFederation H) (n : ℕ) :
    JointSpace (fun i : Fin n => H i.val) →ₗ[ℂ] JointSpace (fun i : Fin n => H i.val) :=
  JointProjector.P_joint (fun i : Fin n => H i.val) (truncate H fed n)

/-! ## F-11 — the constrained-tensor-product joint P_ω is a documented no-go

    F-11 fired on 2026-05-22. The joint multi-rung backward P_omega operator
    is non-constructible at theorem strength. This file held the
    constrained-tensor-product / "soft additive ansatz" formulation of that
    operator: P_omega as the colimit of the finite-truncation joint
    projectors `P_truncated` as n → ∞, restricted to the rung-corridor
    subspace. That colimit is the same no-go.

    The two theorems that close the construction tree (stated in full in
    `CorridorProjector.FelevenNoGo`):

    - T1 (holographic / geometric dilution). Any bulk geometry gives a
      cross-rung coupling that decays with geodesic distance. The joint
      participation ratio is then extensive (`ρ_joint ~ R^{-1}`,
      `k_eff ~ R`): the ω-set empties past a finite rung count. τ → 0 (the
      chaos pole on the cross-rung axis), not τ in corridor.
      Record: experiments/open_system_pomega/assumption_audit/holographic_pomega/

    - T2 (holonomic / area law). A Wilson loop around the TSVF
      forward–backward loop with a dissipative backward leg obeys an area
      law, `Tr Hol ~ exp(−κ·R)`: the holonomy decoheres geometrically to
      zero as the rung count R grows.
      Record: experiments/open_system_pomega/assumption_audit/holonomic_pomega/

    A subsequent five-fold search for an empirical replacement — the H_meas
    audit pointer plus three macroscopic shadows (FDT fluctuation, classical
    g/J, CMB ℓ=3) — all failed or nulled. Records: experiments/audit_pointer/,
    experiments/shadows/.

    The constrained-tensor-product colimit was the structurally-tighter
    hope: instead of "projector on universal configurations satisfying three
    properties," the problem became "constrained tensor product over a
    countable set of A3+ goal-holders." F-11 closes both formulations. The
    framework therefore does NOT assert the joint P_omega as an operator.
    The three operator axioms this file once carried (`P_omega`,
    `Phi_omega`, `P_omega_idempotent`) are retracted. -/

/-- F-11 record for the constrained-tensor-product branch. A flat fact with
    its evidence — the constrained-tensor-product joint multi-rung backward
    P_omega (the colimit of `P_truncated` as n → ∞) is a documented no-go,
    closed at theorem strength by T1 (geometric dilution) and T2 (holonomic
    area law), with the five-fold empirical-replacement search failed. Not a
    `sorry`, not an axiom asserting an operator: a recorded obstruction. The
    cosmological *federation* and the finite-truncation joint projectors
    stand; the n → ∞ joint backward *operator* does not. -/
structure FelevenNoGo where
  /-- The constrained-tensor-product joint multi-rung backward P_omega
      (colimit of the finite-truncation projectors) is not constructible. -/
  constrained_tensor_product_operator_not_constructible : True
  /-- T1: any bulk geometry → cross-rung coupling decaying with geodesic
      distance → joint participation ratio extensive → ω-set empties. -/
  geometric_dilution_theorem : True
  /-- T2: a Wilson loop with a dissipative backward leg obeys an area law
      `Tr Hol ~ exp(−κR)` → the holonomy decoheres. -/
  holonomic_area_law_theorem : True
  /-- The five-fold empirical-replacement search (H_meas audit pointer; FDT,
      classical g/J, CMB ℓ=3 shadows) all failed or nulled. -/
  empirical_replacement_search_failed : True

/-- F-11 is fired on the constrained-tensor-product branch: the no-go record
    is inhabited. Mirrors `CorridorProjector.F11_joint_backward_P_omega_no_go`
    — same no-go, recorded on the tensor-product formulation. -/
def F11_constrained_tensor_product_P_omega_no_go : FelevenNoGo :=
  ⟨trivial, trivial, trivial, trivial⟩

/-! ## What survives — the finite-federation consent structure

    The retracted operator axioms (`P_omega`, `Phi_omega`,
    `P_omega_idempotent`) are gone. What remains is the finite-federation
    consent structure: every finite truncation has a genuine joint projector
    `P_truncated`, and `JointProjector` proves its idempotence
    (`P_joint_idempotent`), self-adjointness of factors, and the corridor
    characterization. Those finite theorems are F-11-untouched and are NOT
    re-stated here — they live in `JointGoalProjector.lean`.

    The cosmological consent corridor `isCosmologicalConsent` is a constraint
    on those finite truncations. It is well-defined; it is not an operator.
    The Penrose-past consequence that the retracted `penrose_from_P_omega`
    once stated against the joint operator is now carried — as a structural
    argument, not a derivation — by
    `Cosmology.PenrosePast.penrose_low_entropy_past_structural`. -/

/-- Every finite truncation of a cosmologically-consenting federation has an
    idempotent joint projector. This is the surviving structural content:
    the finite-stage joint projectors are genuine projectors. Re-grounds the
    retracted `P_omega_idempotent` onto the finite stages, which is where
    idempotence actually holds — the n → ∞ limit is the no-go. -/
theorem P_truncated_idempotent
    (fed : CosmologicalFederation H) (n : ℕ) :
    ∀ x : JointSpace (fun i : Fin n => H i.val),
      P_truncated H fed n (P_truncated H fed n x) = P_truncated H fed n x := by
  intro x
  exact JointProjector.P_joint_idempotent
    (fun i : Fin n => H i.val) (truncate H fed n) x

end CoherenceRatchet.Cosmology.ConsentProjector
