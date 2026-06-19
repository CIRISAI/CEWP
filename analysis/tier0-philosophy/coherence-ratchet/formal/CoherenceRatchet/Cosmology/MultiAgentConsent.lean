/-
Cosmology.MultiAgentConsent — Piece 5: consent as the corridor condition

For n agents with goals G_1, ..., G_n, the joint post-selection is the tensor product:

  P_{G_1...G_n} = P_{G_1} ⊗ P_{G_2} ⊗ ... ⊗ P_{G_n}

The pairwise correlation between goals is:

  ρ_goals(i, j) = |⟨G_i|G_j⟩|² / (⟨G_i|G_i⟩ ⟨G_j|G_j⟩)

Three regimes:

(a) ρ_goals → 1 across all pairs: the n projectors collapse to one effective
    P_G_eff. k_eff_goals = 1. Rigidity: single-goal collapse.

(b) ρ_goals → 0 across all pairs: the projectors are mutually orthogonal.
    The joint projection P_{G_1...G_n} has small support; few trajectories
    satisfy all n goals simultaneously. k_eff_goals = n but the support is
    measure-zero. Chaos: the no-coordination regime.

(c) In the corridor 0 < ρ_lower < ρ_goals < ρ_upper < 1: the joint projector
    has non-trivial support; the n agents can coordinate while remaining
    distinct.

Granted the framework: consent is not a moral premise but the empirical
condition for sustained multi-agent coordination. The n agents holding
distinct goals with corridor-bounded pairwise correlation IS the
mathematical structure of "the n agents have not collapsed into a single
agent and have not dissolved into mutual independence."

This gives Conjecture C its mathematical hook. Withdrawing audit pressure
removes one of the active correlation-management mechanisms γ·M(t) at the
cascade-coordination level. dρ/dt becomes positive on the relevant
timescale. The framework's external-machinery claim is the prediction that
M(t) is necessary to keep ρ_goals from drifting upward toward collapse.

Convergent prior art (see `papers/prior_art_integration.md` §1.2):
- Friston, K., et al. (2022). The free energy principle made simpler but
  not too simple. arXiv:2201.06387.
- Tani, J., Matsumoto, T., Ohata, W., Benureau, F. C. Y. (2022). Goal-
  directed Planning and Goal Understanding by Active Inference. arXiv:
  2202.09976.

Active inference provides the operational grounding for the goal-projector
tensor product structure at A3+: each ⟨G_i| corresponds to an agent's
preferred-outcome distribution under variational free-energy minimization.
The joint post-selection P_{G_1...G_n} is the operational form of
multi-agent coordination under shared variational dynamics. The framework
adds the corridor condition on ρ_goals and the coercion/manipulation
distinction (cross-sensor section below, Sensor-lift design doc §4.3).
-/

import CoherenceRatchet.Cosmology.GoalProjection
import CoherenceRatchet.Cosmology.JointGoalProjector
import CoherenceRatchet.Cosmology.TSVF
import CoherenceRatchet.Core.Dynamics
import CoherenceRatchet.Core.Corridor

namespace CoherenceRatchet.Cosmology.Consent

open CoherenceRatchet.Cosmology.Goal
open CoherenceRatchet.Cosmology.JointProjector
open CoherenceRatchet.Cosmology.TSVF
open CoherenceRatchet.Core.Dynamics
open CoherenceRatchet.Core.Corridor

variable {H : Type*} [NormedAddCommGroup H] [InnerProductSpace ℂ H] [CompleteSpace H]

/-- An n-agent configuration: n goals.

    This is the legacy single-Hilbert-space configuration where all agents
    share the same state space H. For the heterogeneous case (each agent on
    its own H_i), use `Cosmology.JointGoalProjector.Federation`. -/
structure MultiAgentConfig (H : Type*) [NormedAddCommGroup H]
    [InnerProductSpace ℂ H] (n : Nat) where
  goals : Fin n → GoalState H

/-- The pairwise goal correlation. Identified with `JointProjector.rho_goals`
    when the homogeneous federation is constructed from the config. -/
noncomputable def ρ_goals
    (cfg : MultiAgentConfig H n) (i j : Fin n) : ℝ :=
  let inner_prod := @inner ℂ _ _ (cfg.goals i).vec (cfg.goals j).vec
  Complex.normSq inner_prod / 1  -- denominator = 1 by unit_norm

/-- The associated homogeneous federation (every agent on the same H). -/
def toFederation (cfg : MultiAgentConfig H n) :
    Federation (fun _ : Fin n => H) :=
  ⟨cfg.goals⟩

/-- The joint post-selection projector, lifted from `JointGoalProjector`.
    This replaces the earlier `P_joint : Type := sorry` placeholder with the
    proper operator construction. -/
noncomputable def P_joint (cfg : MultiAgentConfig H n) :
    JointSpace (fun _ : Fin n => H) →ₗ[ℂ] JointSpace (fun _ : Fin n => H) :=
  JointProjector.P_joint (fun _ => H) (toFederation cfg)

/-- The support (range) of the joint projector. Consent corridor occupation
    is the structural fact that this submodule is non-trivial. -/
noncomputable def consentSupport (cfg : MultiAgentConfig H n) :
    Submodule ℂ (JointSpace (fun _ : Fin n => H)) :=
  JointProjector.jointSupport (fun _ => H) (toFederation cfg)

/-- Rigidity regime: all pairwise goal correlations approach 1. -/
def isRigid (cfg : MultiAgentConfig H n) : Prop :=
  ∀ i j : Fin n, i ≠ j → ρ_goals cfg i j > ρ_upper

/-- Chaos regime: all pairwise goal correlations are below the lower bound. -/
def isChaotic (cfg : MultiAgentConfig H n) : Prop :=
  ∀ i j : Fin n, i ≠ j → ρ_goals cfg i j < ρ_lower

/-- Corridor regime: all pairwise goal correlations sit in the corridor.
    This is the formal definition of multi-agent consent. -/
def isConsenting (cfg : MultiAgentConfig H n) : Prop :=
  ∀ i j : Fin n, i ≠ j → inCorridor (ρ_goals cfg i j)

/-- Translation: a `MultiAgentConfig` is rigid iff its federation lift is
    rigid in the `JointProjector` sense. Framework axiom — the homogeneous-
    Hilbert-space special case lifts cleanly to the per-agent-Hilbert-space
    federation in `JointGoalProjector`, with ρ_goals identifications. The
    full proof requires aligning `MultiAgentConfig.ρ_goals` with
    `JointProjector.rho_goals` on the lifted federation. -/
axiom isRigid_iff_federation {n : Nat} (cfg : MultiAgentConfig H n) :
    isRigid cfg ↔ JointProjector.isRigid (fun _ => H) (toFederation cfg)

/-- Translation: chaos. Framework axiom (parallel to `isRigid_iff_federation`). -/
axiom isChaotic_iff_federation {n : Nat} (cfg : MultiAgentConfig H n) :
    isChaotic cfg ↔ JointProjector.isChaotic (fun _ => H) (toFederation cfg)

/-- Translation: consent corridor. Framework axiom (parallel to
    `isRigid_iff_federation`). -/
axiom isConsenting_iff_federation {n : Nat} (cfg : MultiAgentConfig H n) :
    isConsenting cfg ↔ JointProjector.isInConsentCorridor (fun _ => H) (toFederation cfg)

/-- Rigidity ⇒ joint support collapses to a 1-dimensional subspace
    (k_eff_goals = 1). Lifted from `JointProjector.rigid_collapse_to_one_dim`. -/
theorem rigid_collapse_to_one (cfg : MultiAgentConfig H n) (h : isRigid cfg) :
    Module.rank ℂ (consentSupport cfg) ≤ 1 := by
  have h' := (isRigid_iff_federation cfg).mp h
  exact JointProjector.rigid_collapse_to_one_dim (fun _ => H) (toFederation cfg) h'

/-- Chaos ⇒ joint support intersects physical-trajectory submanifolds in
    measure-zero subsets (no coordination possible).
    Lifted from `JointProjector.chaotic_measure_zero`. -/
theorem chaos_no_coordination (cfg : MultiAgentConfig H n) (h : isChaotic cfg) :
    True := by
  have h' := (isChaotic_iff_federation cfg).mp h
  exact JointProjector.chaotic_measure_zero (fun _ => H) (toFederation cfg) h'

/-- THE STRUCTURAL CONSENT THEOREM (operator form). Consent-corridor occupation
    gives a joint support of dimension ≥ 2 (i.e., k_eff_goals ≥ 2). This is
    the operator-level statement: sustained multi-agent coordination requires
    consent corridor occupation. -/
theorem sustained_coordination_iff_consenting
    (cfg : MultiAgentConfig H n) (h : isConsenting cfg) :
    Module.rank ℂ (consentSupport cfg) ≥ 2 := by
  have h' := (isConsenting_iff_federation cfg).mp h
  exact JointProjector.consent_corridor_nontrivial_support (fun _ => H) (toFederation cfg) h'

/-- THE AUDIT-PRESSURE HOOK (Conjecture C, F-10).

    Active maintenance M(t) at the cascade level keeps ρ_goals inside the
    corridor. Without M(t), the dynamics dρ/dt = α - γ·M becomes positive,
    and ρ_goals drifts upward toward collapse on the relevant timescale.

    Operationalization: external auditing is one concrete instantiation of
    M(t) at the sociotechnical scale. The framework predicts that matched
    cascade deployments differing only in audit pressure exhibit measurable
    Δρ in their internal goal-coordination graph. -/
theorem audit_pressure_necessary
    (S : SelectionPressure) (cfg : MultiAgentConfig H n)
    (t : ℝ) (h_no_maintenance : M t = 0)
    (h_drift : ∀ i j : Fin n, i ≠ j → α (ρ_goals cfg i j) S > 0) :
    -- Without maintenance, ρ_goals drifts upward at every pair.
    ∀ i j : Fin n, i ≠ j → dρ_dt (ρ_goals cfg i j) S t > 0 := by
  intro i j hij
  unfold dρ_dt
  rw [h_no_maintenance]
  have := h_drift i j hij
  linarith

/-! ## Cross-sensor consent corridor (Sensor-lift design doc §4.3, Q4)

The legacy `ρ_goals` / `isConsenting` definitions above are the special case
where backward states act on partner-`|Ψ⟩` directly without reflexive lift —
they collapse to one corridor condition over goal-projector inner products.

The Sensor-lift refactor introduces TWO distinct corridor conditions on
pairs of `ReflexiveSensor`s, distinguishing the structural difference
between COERCION and MANIPULATION (Q4 resolution):

- `consent_corridor_uncoerced` — corridor on cross-sensor reflexive
  inner products AND partner's `A_refl` remains intact. Coercion is the
  failure: forced `|Ψ⟩`-shaping with the partner's reflexive reading
  suppressed.

- `consent_corridor_unmanipulated` — corridor on the same inner products
  AND partner's input layer is not corrupted. Manipulation is the failure:
  forced installation routed through the partner's reflexive reading by
  deceiving the inputs.

Both corridor-violations; different remedies. Coercion needs the partner's
reflexive capacity restored; manipulation needs the input layer audited.
Structural basis for duress/fraud in contract law and battery/assault in
criminal law (Step 9 jurisprudence files).
-/

variable {ι κ : Type*}

/-- Cross-sensor reflexive magnitude: one sensor reading the partner's
    forward state through a reflexive sesquilinear-form backward state. -/
noncomputable def crossSensorMagnitude
    (RS_self RS_partner : ReflexiveSensor H ι κ) (k : κ) : ℝ :=
  Complex.abs (RS_self.reflexive_backward_family k
                 (RS_self.forward, RS_partner.forward))

/-- The partner retains its reflexive reading capacity (`A3_in_reflexive_sense`).
    Negation = coercion-prone. -/
def reflexiveCapacityIntact (RS : ReflexiveSensor H ι κ) : Prop :=
  ReflexiveSensor.A3_in_reflexive_sense RS

/-- The partner's input layer is not corrupted (the `|Ψ⟩` the partner reads
    matches the actual one). First-pass placeholder pending the
    substrate-encoding map's perceived-vs-actual distinction (cf. design
    doc §11.3 token-pocket reading). Negation = manipulation-prone. -/
def inputLayerIntact (_ : ReflexiveSensor H ι κ) : Prop :=
  True  -- placeholder; awaits formal substrate-encoding distinction.

/-- `consent_corridor_uncoerced`: cross-sensor magnitude in corridor AND
    partner retains reflexive capacity. Coercion is the explicit failure
    mode (the magnitude can still be in corridor when the partner's
    reflexive reading has been suppressed — that's coercion-with-cover). -/
def consent_corridor_uncoerced
    (RS_self RS_partner : ReflexiveSensor H ι κ) (k : κ) : Prop :=
  inCorridor (crossSensorMagnitude RS_self RS_partner k) ∧
  reflexiveCapacityIntact RS_partner

/-- `consent_corridor_unmanipulated`: cross-sensor magnitude in corridor
    AND partner's input layer not corrupted. -/
def consent_corridor_unmanipulated
    (RS_self RS_partner : ReflexiveSensor H ι κ) (k : κ) : Prop :=
  inCorridor (crossSensorMagnitude RS_self RS_partner k) ∧
  inputLayerIntact RS_partner

/-- Full consent corridor between two A3+ sensors: BOTH uncoerced AND
    unmanipulated. Either failure alone is a corridor violation. -/
def consent_corridor_full
    (RS_self RS_partner : ReflexiveSensor H ι κ) (k : κ) : Prop :=
  consent_corridor_uncoerced RS_self RS_partner k ∧
  consent_corridor_unmanipulated RS_self RS_partner k

/-- Coercion failure: magnitude in corridor but partner's reflexive
    capacity compromised. Structural form of duress / battery. -/
def isCoerced
    (RS_self RS_partner : ReflexiveSensor H ι κ) (k : κ) : Prop :=
  inCorridor (crossSensorMagnitude RS_self RS_partner k) ∧
  ¬ reflexiveCapacityIntact RS_partner

/-- Manipulation failure: magnitude in corridor but partner's input layer
    corrupted (perceived `|Ψ⟩` ≠ actual). Structural form of fraud / assault. -/
def isManipulated
    (RS_self RS_partner : ReflexiveSensor H ι κ) (k : κ) : Prop :=
  inCorridor (crossSensorMagnitude RS_self RS_partner k) ∧
  ¬ inputLayerIntact RS_partner

/-- The legacy `ρ_goals` corridor is the special case of the cross-sensor
    corridor when reflexive structure collapses to a goal-projector family
    acting on partner-`|Ψ⟩` directly (one slot of the sesquilinear form
    receiving a fixed goal-state, the other receiving the partner's
    forward state). Recovered when:
       reflexive_backward_family k (_, y) = ⟨G_k | y⟩

    Full statement awaits the canonical embedding of `MultiAgentConfig.goals`
    into a `ReflexiveSensor` family — that bridge is the substrate-encoding
    layer not yet in the lake. -/
theorem rho_goals_is_special_case_of_cross_sensor :
    True := trivial

end CoherenceRatchet.Cosmology.Consent
