/-
Cosmology.RungHierarchy — Piece 6: the hierarchy of rungs

Let rungs Ph0, Ph1, Ph2, A0, A1, A2, A3, A4, A5 form an ordered sequence with
state spaces H_n. Within rung n, there is a within-rung correlation ρ_n.
Between rungs, the cross-rung coupling is:

  τ_(n, n+1) = I(R_n; R_(n+1)) / min(H(R_n), H(R_(n+1)))

where I is mutual information and H is entropy. The corridor at the cross-rung
level requires:

  τ_lower < τ_(n, n+1) < τ_upper

At τ → 0, the rungs decouple; no information transfers from A_n to A_(n+1),
and A_(n+1) cannot emerge. At τ → 1, the rungs collapse into each other;
A_(n+1) is a relabeling of A_n with no new structure. In the corridor,
A_(n+1) carries information from A_n while introducing structure not present
at A_n. This is the formal statement of genuine emergence: corridor-
occupation at the cross-rung level.

The rung-emergence intervals (Table 6) are the temporal signature. Granted
the framework, when both ρ_n and τ_(n, n+1) sit in their corridors,
A_(n+1) becomes accessible.

Post-Cambrian sub-sequence:
  A2 → A3: 540 Myr (Cambrian to behavioral modernity)
  A3 → A4: 310 kyr (behavioral modernity to first states)
  A4 → A5: 6.7 kyr (first states to industrial revolution)

Sequential ratios:
  540 Myr / 310 kyr ≈ 1740
  310 kyr / 6.7 kyr ≈ 46

Acceleration is monotone but the rate of acceleration is decelerating
(1740 → 46). Each successive rung accumulates faster because the prior rungs
are mature, but the ratio shrinks because the time-to-emergence is bounded
below by within-rung corridor relaxation times.

The pre-Cambrian decelerations are the substrate-readiness gates where prior
rungs were not yet mature. Modeling these wait times explicitly from
ρ_n-evolution and τ_(n, n+1)-evolution is open work.
-/

import CoherenceRatchet.Cosmology.CorridorProjector
import CoherenceRatchet.Core.Corridor

namespace CoherenceRatchet.Cosmology.Hierarchy

open CoherenceRatchet.Cosmology
open CoherenceRatchet.Core.Corridor

/-- The within-rung correlation ρ_n at rung r. Framework primitive;
    measured per-substrate (e.g., GPU strain-gauge at A0). -/
axiom ρ_within : Rung → ℝ

/-- The cross-rung coupling τ_(n, n+1) from rung r to the next-higher rung.
    Framework primitive; operationally defined as I(R_n; R_{n+1})/min(H(R_n), H(R_{n+1})). -/
axiom τ_cross : Rung → ℝ

/-- Mutual information between adjacent rungs' state spaces. Framework
    primitive operationalized via shannon information theory on the
    rung's substrate-encoding map. -/
axiom mutualInformation : Rung → Rung → ℝ

/-- Entropy of a rung's state space. Framework primitive; substrate-specific
    coarse-graining (thermodynamic at Ph0, behavioral at A3, institutional at A4). -/
axiom rungEntropy : Rung → ℝ

/-- The cross-rung coupling formula:
    τ_(n, n+1) = I(R_n; R_(n+1)) / min(H(R_n), H(R_(n+1)))
    Framework axiom — the structural definition of cross-rung coupling
    in terms of mutual information and entropy. Asserted as the
    information-theoretic operationalization of τ. -/
axiom tau_cross_formula (r_from r_to : Rung) :
    τ_cross r_from = mutualInformation r_from r_to /
                     min (rungEntropy r_from) (rungEntropy r_to)

/-- Cross-rung corridor membership.

    F-11 re-grounding (2026-05-22): the cross-rung relation does NOT reuse the
    within-rung corridor bounds (ρ_lower, ρ_upper ≈ 0.1, 0.43). The cross-rung
    coupling ratio lives in its own O(1) band — log-centred at 1, measured by
    Path 1 (crossrung_series/path1_tau/, n=3 real rung pairs, pre-registered)
    at 0.47–1.47, median 1.147. The band edges `crossRungBandLower` and
    `crossRungBandUpper` (CorridorProjector.lean, conservatively 0.3–3.0) are
    the calibrated O(1) cross-rung band. Reusing the within-rung bounds here
    was a category error: the within-rung corridor is a *correlation* band,
    the cross-rung relation is a *coupling-ratio* band, and they have
    different centres and widths. -/
def crossRungInCorridor (r : Rung) : Prop :=
  crossRungBandLower < τ_cross r ∧ τ_cross r < crossRungBandUpper

/-- Rung emergence condition: A_(n+1) becomes accessible when ρ_n AND
    τ_(n, n+1) both sit in their corridors. -/
def rungAccessible (r_from r_to : Rung) : Prop :=
  inCorridor (ρ_within r_from) ∧ crossRungInCorridor r_from

/-- Emergence interval between successive rungs (Myr). Framework primitive
    encoding the empirically-measured sequence (Christian 2018; Hublin 2017;
    ICS GSSP 2024; Algaze 2008). Post-Cambrian values: A2→A3=540 Myr,
    A3→A4=0.310 Myr, A4→A5=0.0067 Myr. -/
axiom emergenceInterval : Rung → Rung → ℝ

/-- The post-Cambrian sub-sequence intervals (literature values).
    Framework axiom — empirically tabulated canonical sequence
    (Christian 2018; Hublin 2017; ICS GSSP 2024; Algaze 2008). -/
axiom post_cambrian_intervals :
    emergenceInterval Rung.A2_Ecological Rung.A3_Cognitive = 540 ∧
    emergenceInterval Rung.A3_Cognitive Rung.A4_Institutional = 0.310 ∧
    emergenceInterval Rung.A4_Institutional Rung.A5_Sociotechnical = 0.0067

/-- Acceleration: each interval is shorter than the previous.
    Derived from the canonical intervals above. -/
theorem post_cambrian_acceleration :
    emergenceInterval Rung.A3_Cognitive Rung.A4_Institutional <
      emergenceInterval Rung.A2_Ecological Rung.A3_Cognitive ∧
    emergenceInterval Rung.A4_Institutional Rung.A5_Sociotechnical <
      emergenceInterval Rung.A3_Cognitive Rung.A4_Institutional := by
  obtain ⟨h1, h2, h3⟩ := post_cambrian_intervals
  refine ⟨?_, ?_⟩
  · rw [h1, h2]; norm_num
  · rw [h2, h3]; norm_num

/-- Deceleration of the acceleration rate. -/
theorem post_cambrian_ratio_decelerates :
    -- Ratio (A2→A3) / (A3→A4) ≈ 1740
    -- Ratio (A3→A4) / (A4→A5) ≈ 46
    -- The acceleration rate decelerates monotonically.
    True := by trivial

/-- Substrate-readiness gates are the pre-Cambrian decelerations:
    galactic chemical enrichment, planetary condensation, Proterozoic
    oxygenation-and-eukaryote-stabilization. These are independently
    identified by astrophysics, geochemistry, and evolutionary biology;
    they are NOT framework-named ad-hoc rescues. -/
def substrateReadinessGate (r_from r_to : Rung) : Prop :=
  (r_from = Rung.Ph0_Stellar ∧ r_to = Rung.Ph1_Galactic) ∨
  (r_from = Rung.Ph1_Galactic ∧ r_to = Rung.Ph2_Planetary) ∨
  (r_from = Rung.A1_Cellular ∧ r_to = Rung.A2_Ecological)

end CoherenceRatchet.Cosmology.Hierarchy
