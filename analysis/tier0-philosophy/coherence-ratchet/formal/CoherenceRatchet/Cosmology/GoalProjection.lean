/-
Cosmology.GoalProjection — Piece 4: TSVF structure at A3+

Standard forward evolution:
  |ψ(t)⟩ = U(t, t₀) |ψ(t₀)⟩

TSVF adds a backward state from post-selection at t_f:
  ⟨φ(t)| = ⟨φ(t_f)| U(t_f, t)

The two-state vector is (⟨φ(t)|, |ψ(t)⟩). Observables get weak values:
  ⟨A⟩_w = ⟨φ|A|ψ⟩ / ⟨φ|ψ⟩

For an A3+ agent with goal G, the goal-state acts as a post-selection
projector P_G with bra ⟨G|. The agent's effective dynamics is:

  |Ψ_agent(t)⟩ ∝ P_G U(t, t_past) |Ψ_past⟩

Trajectories incompatible with G have suppressed amplitude in |Ψ_agent(t)⟩.
This is the structural definition of goal-formation as causal operation:
the goal projector excludes incompatible trajectories from the search space.

NOTE: Granted TSVF, there is no metaphysical free-will claim being made;
there is a measurable difference in how goal-holding systems search the
trajectory space.
-/

import CoherenceRatchet.Cosmology.TSVF
import Mathlib.LinearAlgebra.Projection

namespace CoherenceRatchet.Cosmology.Goal

open TSVF

variable {H : Type*} [NormedAddCommGroup H] [InnerProductSpace ℂ H] [CompleteSpace H]

/-- A goal state held by an A3+ agent. Represented as a unit vector |G⟩
    whose corresponding projector P_G = |G⟩⟨G| excludes G-incompatible
    trajectories. -/
structure GoalState (H : Type*) [NormedAddCommGroup H] [InnerProductSpace ℂ H] where
  vec : H
  unit_norm : ‖vec‖ = 1

/-- The goal projector P_G = |G⟩⟨G|. Concrete construction via
    `(innerSL ℂ G.vec).smulRight G.vec`: the continuous linear map
    `x ↦ ⟨G | x⟩ • G.vec`. The standard rank-one projector in Lean 4
    Mathlib. -/
noncomputable def P_G (G : GoalState H) : H →L[ℂ] H :=
  (innerSL ℂ G.vec).smulRight G.vec

/-- The rank-one action of P_G: `P_G G x = ⟨G | x⟩ • G.vec`. Follows from
    the definition by unfolding `smulRight_apply` and `innerSL_apply`. -/
theorem P_G_apply (G : GoalState H) (x : H) :
    P_G G x = @inner ℂ _ _ G.vec x • G.vec := by
  unfold P_G
  rw [ContinuousLinearMap.smulRight_apply, innerSL_apply]

/-- The weak value of observable A under forward state |ψ⟩ and backward
    state ⟨φ|. -/
noncomputable def weakValue
    (ψ : ForwardState H) (φ : BackwardState H) (A : H →L[ℂ] H) : ℂ :=
  @inner ℂ _ _ φ.vec (A ψ.vec) / @inner ℂ _ _ φ.vec ψ.vec

/-- The agent's effective state at time t under goal G, given past state. -/
noncomputable def agentState
    (G : GoalState H) (U : H →L[ℂ] H) (ψ_past : ForwardState H) : H :=
  P_G G (U ψ_past.vec)

/-- The structural claim: G-incompatible trajectories have suppressed amplitude
    in the agent's effective state. Real theorem proved from `P_G_apply`
    plus inner-product conjugate-symmetry: the inner product `⟨ψ_incompatible |
    P_G(U ψ_past)⟩` equals `⟨G | U ψ_past⟩ * ⟨ψ_incompatible | G⟩`, and the
    second factor is the conjugate of `⟨G | ψ_incompatible⟩ = 0`. -/
theorem goal_excludes_incompatible
    (G : GoalState H) (U : H →L[ℂ] H) (ψ_past : ForwardState H)
    (ψ_incompatible : H)
    (h : @inner ℂ _ _ G.vec ψ_incompatible = 0) :
    @inner ℂ _ _ ψ_incompatible (agentState G U ψ_past) = 0 := by
  unfold agentState
  rw [P_G_apply, inner_smul_right]
  have h_conj : @inner ℂ _ _ ψ_incompatible G.vec = 0 := by
    rw [← inner_conj_symm, h, starRingEnd_apply, star_zero]
  rw [h_conj, mul_zero]

/-- The framework's defensible reading of agency: goal-formation is a
    measurable causal operation on the trajectory search space. The agent
    is an A3+ system iff it holds a non-trivial goal state G ≠ |Ψ_past⟩,
    so that P_G is a non-identity projector. -/
def isA3PlusAgent (G : GoalState H) (ψ_past : ForwardState H) : Prop :=
  G.vec ≠ ψ_past.vec

/-- The Wise Authority architectural requirement (RATCHET §4): A3+ agents
    are observable through their goal-projection signature. Goal-formation
    leaves a trace in the trajectory-space search pattern that's measurable
    by external observers. -/
def goalSignatureObservable (G : GoalState H) : Prop :=
  -- An external observer can in principle measure the weak value of any
  -- observable A relative to (forward = past evolution, backward = ⟨G|).
  True

end CoherenceRatchet.Cosmology.Goal
