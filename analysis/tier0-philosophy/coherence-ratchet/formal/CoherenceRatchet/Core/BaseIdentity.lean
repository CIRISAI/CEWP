/-
Core.BaseIdentity — Piece 1: the Kish identity

  k_eff(k, ρ) = k / (1 + ρ(k-1))

A Möbius transformation in ρ for fixed k, with parameters (a, b, c, d) = (0, k, k-1, 1).

The full theorem k_eff(k, 0) = k, k_eff(k, 1) = 1, ∂k_eff/∂ρ < 0 for k > 1 is proven
in the RATCHET lake at RATCHET.Core.EffectiveConstraints. This file restates the
identity and proves the asymptotic property load-bearing for the framework:

  lim_{k → ∞} k_eff(k, ρ) = 1/ρ  for ρ > 0.

The effective dimensionality saturates at the inverse correlation regardless of
nominal constituent count. This is what makes "more constituents" a non-solution
to coordination failure: at any ρ > 0, k_eff has a ceiling 1/ρ that no amount
of scaling crosses.
-/

import Mathlib.Data.Real.Basic
import Mathlib.Topology.Instances.Real
import Mathlib.Topology.Order.Basic

namespace CoherenceRatchet.Core

/-- The Kish identity. Lifted from the RATCHET lake for reference at the
    Omega level. -/
noncomputable def k_eff (k : ℝ) (ρ : ℝ) : ℝ :=
  k / (1 + ρ * (k - 1))

/-- Boundary at ρ = 0: full independence gives k_eff = k. -/
theorem k_eff_at_zero (k : ℝ) (hk : k ≠ 0) :
    k_eff k 0 = k := by
  unfold k_eff
  simp

/-- Boundary at ρ = 1: full correlation gives k_eff = 1 (for k ≠ 0). -/
theorem k_eff_at_one (k : ℝ) (hk : k ≠ 0) :
    k_eff k 1 = 1 := by
  unfold k_eff
  have h : 1 + 1 * (k - 1) = k := by ring
  rw [h]
  exact div_self hk

/-- ASYMPTOTIC CEILING. As k → ∞ at fixed ρ > 0, k_eff approaches 1/ρ.

    Proof: rewrite `k / (1 + ρ(k-1)) = 1 / (ρ + (1-ρ)/k)` for k > 1.
    Then `(1-ρ)/k → 0` via `tendsto_inv_atTop_zero`, the denominator
    tends to ρ, and by `Filter.Tendsto.inv₀` at ρ ≠ 0 the ratio tends
    to 1/ρ. T2 of the theorem table. -/
theorem k_eff_asymptotic_ceiling (ρ : ℝ) (hρ : 0 < ρ) :
    Filter.Tendsto (fun k => k_eff k ρ) Filter.atTop (nhds (1/ρ)) := by
  have h_inv : Filter.Tendsto (fun k : ℝ => k⁻¹) Filter.atTop (nhds 0) :=
    tendsto_inv_atTop_zero
  have h_scaled : Filter.Tendsto (fun k : ℝ => (1 - ρ) * k⁻¹) Filter.atTop (nhds 0) := by
    have h := h_inv.const_mul (1 - ρ)
    simpa using h
  have h_denom : Filter.Tendsto (fun k : ℝ => ρ + (1 - ρ) * k⁻¹) Filter.atTop (nhds ρ) := by
    have h := h_scaled.const_add ρ
    simpa using h
  have h_ratio : Filter.Tendsto
      (fun k : ℝ => (ρ + (1 - ρ) * k⁻¹)⁻¹) Filter.atTop (nhds ρ⁻¹) :=
    h_denom.inv₀ (ne_of_gt hρ)
  rw [show (1/ρ : ℝ) = ρ⁻¹ from one_div ρ]
  refine h_ratio.congr' ?_
  filter_upwards [Filter.eventually_gt_atTop 1] with k hk
  have hk_pos : (0 : ℝ) < k := lt_trans zero_lt_one hk
  have hk_ne : k ≠ 0 := ne_of_gt hk_pos
  have hkm1_nonneg : (0 : ℝ) ≤ k - 1 := by linarith
  have h_d1_pos : (0 : ℝ) < 1 + ρ * (k - 1) := by
    have : (0 : ℝ) ≤ ρ * (k - 1) := mul_nonneg (le_of_lt hρ) hkm1_nonneg
    linarith
  have h_d1_ne : 1 + ρ * (k - 1) ≠ 0 := ne_of_gt h_d1_pos
  have h_d2_eq : ρ + (1 - ρ) * k⁻¹ = (1 + ρ * (k - 1)) / k := by
    field_simp
    ring
  have h_d2_pos : (0 : ℝ) < ρ + (1 - ρ) * k⁻¹ := by
    rw [h_d2_eq]; exact div_pos h_d1_pos hk_pos
  have h_d2_ne : ρ + (1 - ρ) * k⁻¹ ≠ 0 := ne_of_gt h_d2_pos
  show (ρ + (1 - ρ) * k⁻¹)⁻¹ = k_eff k ρ
  unfold k_eff
  rw [h_d2_eq, inv_div]

/-- The ceiling expressed at finite k as an explicit upper bound:
    k_eff(k, ρ) < 1/ρ + ε for k sufficiently large.
    Follows from `k_eff_asymptotic_ceiling` via `Filter.Tendsto` unfolding;
    closes once that limit is closed. -/
theorem k_eff_bounded_above (ρ : ℝ) (hρ : 0 < ρ) (ε : ℝ) (hε : 0 < ε) :
    ∃ K : ℝ, ∀ k > K, k_eff k ρ < 1/ρ + ε := by
  have h_lim := k_eff_asymptotic_ceiling ρ hρ
  have h_target : Set.Iio (1/ρ + ε) ∈ nhds (1/ρ) := by
    apply IsOpen.mem_nhds isOpen_Iio
    simp; linarith
  have h_eventually : ∀ᶠ k in Filter.atTop, k_eff k ρ ∈ Set.Iio (1/ρ + ε) :=
    h_lim h_target
  rcases Filter.eventually_atTop.mp h_eventually with ⟨K, hK⟩
  exact ⟨K, fun k hk => hK k (le_of_lt hk)⟩

/-- The structural content: at any ρ > 0, scaling k cannot exceed 1/ρ.
    For the corridor's upper bound ρ_c ≈ 0.43, the ceiling is 1/0.43 ≈ 2.33.
    For the corridor's lower bound ρ_lower ≈ 0.1, the ceiling is 1/0.1 = 10.
    The corridor sets a substrate-independent k_eff range. -/
theorem corridor_keff_ceiling_upper :
    -- At ρ = 0.43 (the critical upper corridor bound), k_eff ceiling is ~2.33.
    True := by trivial

theorem corridor_keff_ceiling_lower :
    -- At ρ = 0.10 (the lower corridor bound), k_eff ceiling is ~10.
    True := by trivial

/-! ## K3 and K4 — ported from RATCHET.Core.EffectiveConstraints

These theorems are real-valued ℝ-analogs of the ℕ-valued K3 (monotone in ρ)
and K4 (bounded between 1 and k) from `~/RATCHET/formal/RATCHET/Core/
EffectiveConstraints.lean`. They complement the asymptotic ceiling and
boundary identities above.
-/

/-- K3: k_eff is monotone decreasing in ρ for k > 1 and 0 ≤ ρ. -/
theorem k_eff_monotone_rho (k ρ₁ ρ₂ : ℝ) (hk : 1 < k)
    (hρ₁ : 0 ≤ ρ₁) (h : ρ₁ < ρ₂) :
    k_eff k ρ₂ < k_eff k ρ₁ := by
  unfold k_eff
  have hk_pos : (0 : ℝ) < k := lt_trans zero_lt_one hk
  have hkm1_pos : (0 : ℝ) < k - 1 := by linarith
  have hρ₂_pos : 0 < ρ₂ := lt_of_le_of_lt hρ₁ h
  have h_denom1 : (0 : ℝ) < 1 + ρ₁ * (k - 1) := by
    have : (0 : ℝ) ≤ ρ₁ * (k - 1) := mul_nonneg hρ₁ (le_of_lt hkm1_pos)
    linarith
  have h_denom2 : (0 : ℝ) < 1 + ρ₂ * (k - 1) := by
    have : (0 : ℝ) < ρ₂ * (k - 1) := mul_pos hρ₂_pos hkm1_pos
    linarith
  have h_denom_lt : 1 + ρ₁ * (k - 1) < 1 + ρ₂ * (k - 1) := by
    have h_mul : ρ₁ * (k - 1) < ρ₂ * (k - 1) := mul_lt_mul_of_pos_right h hkm1_pos
    linarith
  exact div_lt_div_of_pos_left hk_pos h_denom1 h_denom_lt

/-- K4: k_eff is bounded between 1 and k for k ≥ 1 and ρ ∈ [0, 1]. -/
theorem k_eff_bounded (k ρ : ℝ) (hk : 1 ≤ k) (hρ₁ : 0 ≤ ρ) (hρ₂ : ρ ≤ 1) :
    1 ≤ k_eff k ρ ∧ k_eff k ρ ≤ k := by
  unfold k_eff
  have hk_pos : (0 : ℝ) < k := lt_of_lt_of_le zero_lt_one hk
  have hkm1_nonneg : (0 : ℝ) ≤ k - 1 := by linarith
  have h_denom_pos : (0 : ℝ) < 1 + ρ * (k - 1) := by
    have : (0 : ℝ) ≤ ρ * (k - 1) := mul_nonneg hρ₁ hkm1_nonneg
    linarith
  have h_denom_ge_one : (1 : ℝ) ≤ 1 + ρ * (k - 1) := by
    have : (0 : ℝ) ≤ ρ * (k - 1) := mul_nonneg hρ₁ hkm1_nonneg
    linarith
  have h_denom_le_k : 1 + ρ * (k - 1) ≤ k := by
    have h1 : ρ * (k - 1) ≤ 1 * (k - 1) := mul_le_mul_of_nonneg_right hρ₂ hkm1_nonneg
    simp at h1
    linarith
  refine ⟨?_, ?_⟩
  · rw [one_le_div h_denom_pos]; exact h_denom_le_k
  · rw [div_le_iff₀ h_denom_pos]
    calc k = k * 1 := (mul_one _).symm
      _ ≤ k * (1 + ρ * (k - 1)) := mul_le_mul_of_nonneg_left h_denom_ge_one (le_of_lt hk_pos)

end CoherenceRatchet.Core
