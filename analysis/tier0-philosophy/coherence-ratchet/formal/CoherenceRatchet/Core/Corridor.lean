/-
Core.Corridor — Piece 3: the corridor as substrate-specific attractor

The corridor is the open interval

  ρ ∈ (ρ_lower, ρ_upper)

between the chaos regime (ρ ≤ ρ_lower; insufficient correlation for
coordination) and the rigidity regime (ρ ≥ ρ_upper; single-voice collapse).
The bounds are framework primitives, axiomatized as substrate-specific.

What was previously asserted in this file — that the bounds are
substrate-independently (0.1, 0.43) — was the framework's empirical claim
based on the CCA v3 cross-substrate measurement. Cross-substrate ρ
measurements in the RATCHET repo (experiments/exp2_cross_substrate) show
substrate-mean ρ values spanning the (0, 1) interval, with substrate-rung
correlation rather than a single universal band. The substrate-independent
corridor claim does not survive the data at face value; what remains is
the substrate-local corridor structure (a band between local rigidity
and local chaos, where sustained coordination occurs in that substrate),
with cross-substrate consistency of bounds an open empirical question.

Within any substrate's local corridor, k_eff is bounded between
1/ρ_upper (floor) and 1/ρ_lower (ceiling) asymptotically, by the Kish
identity. Specific numerical values (formerly ceiling = 10, floor ≈ 2.33)
were derived from the substrate-independent (0.1, 0.43) claim; those are
removed.
-/

import CoherenceRatchet.Core.BaseIdentity
import CoherenceRatchet.Core.Dynamics

namespace CoherenceRatchet.Core.Corridor

open CoherenceRatchet.Core
open CoherenceRatchet.Core.Dynamics

/-- The lower corridor bound. Substrate-specific framework primitive;
    no universal numerical value. Required structural property:
    0 < ρ_lower < ρ_upper < 1. -/
axiom ρ_lower : ℝ

/-- The upper corridor bound. Substrate-specific framework primitive
    (= ρ_c from Dynamics in the per-substrate calibration). -/
axiom ρ_upper : ℝ

/-- The corridor bounds satisfy the open-interval structural condition.
    Required for the dynamical reading to be coherent. -/
axiom corridor_bounds_well_formed : 0 < ρ_lower ∧ ρ_lower < ρ_upper ∧ ρ_upper < 1

/-- The corridor membership predicate. -/
def inCorridor (ρ : ℝ) : Prop :=
  ρ_lower < ρ ∧ ρ < ρ_upper

/-- The asymptotic ceiling at ρ_lower (Kish-identity limit as k → ∞). -/
noncomputable def k_eff_ceiling_at_lower : ℝ := 1 / ρ_lower

/-- The asymptotic floor at ρ_upper (Kish-identity limit as k → ∞). -/
noncomputable def k_eff_floor_at_upper : ℝ := 1 / ρ_upper

/-- The substrate-local k_eff range: any corridor-occupying coordinating
    system at the asymptotic limit k → ∞ has k_eff between the local floor
    and the local ceiling, by the Kish identity. -/
theorem corridor_keff_range_asymptotic (ρ : ℝ) (h : inCorridor ρ) :
    1 / ρ_upper < 1 / ρ ∧ 1 / ρ < 1 / ρ_lower := by
  obtain ⟨h_lo, h_hi⟩ := h
  obtain ⟨h_lower_pos, h_lower_lt_upper, h_upper_lt_one⟩ := corridor_bounds_well_formed
  have h_rho_pos : (0 : ℝ) < ρ := lt_trans h_lower_pos h_lo
  have h_upper_pos : (0 : ℝ) < ρ_upper := lt_trans h_lower_pos h_lower_lt_upper
  refine ⟨?_, ?_⟩
  · rw [div_lt_div_iff h_upper_pos h_rho_pos]
    linarith
  · rw [div_lt_div_iff h_rho_pos h_lower_pos]
    linarith

end CoherenceRatchet.Core.Corridor
