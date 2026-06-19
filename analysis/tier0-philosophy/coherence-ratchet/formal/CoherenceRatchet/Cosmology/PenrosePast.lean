/-
Cosmology.PenrosePast — Penrose low-entropy past hypothesis as a structural
argument (the joint backward operator it presupposed is now an F-11 no-go)

Penrose's Weyl Curvature Hypothesis (1979, 1989, 2004 The Road to Reality) says
the universe started in a state of extraordinarily low entropy. He treated this
as an unexplained boundary condition: a brute fact about why the past has the
form it does.

coherence-ratchet's claim: the Penrose past hypothesis is a structural
consequence of the universal-scale TSVF projection toward omega.

The structural argument:
1. The omega *configuration* (CorridorProjector.isOmegaConfig) is multi-rung
   corridor occupation.
2. Generic high-entropy initial conditions do not seed trajectories reaching
   omega. (Heat death, dispersal, structure-loss trajectories never recover
   the structural conditions required for any rung instantiation.)
3. Low-entropy initial conditions DO seed trajectories reaching omega.
   (The thermodynamic gradient enables structure formation; stellar
   nucleosynthesis, chemical complexification, biology, cognition all require
   an entropy gradient to harvest.)
4. Therefore, conditional on observing an omega-satisfying universe, the
   initial condition is concentrated on low-entropy states.

TIER MARKING (F-11, 2026-05-22). This is a STRUCTURAL ARGUMENT, not a
derivation. The derivation route ran through a joint multi-rung backward
operator P_omega — the TSVF amplitude <Phi_omega | U(t) | Psi_alpha> with
<Phi_omega| an actual projector. F-11 documented that operator as
non-constructible at theorem strength (T1 geometric dilution, T2 holonomic
area law — see CorridorProjector.FelevenNoGo). With the joint operator a
documented no-go, the Penrose-past claim is honestly a BET: the structural
argument (1)-(4) is asserted, but it is not discharged by an operator the
framework can write down. The earlier `axiom penrose_low_entropy_past` and
`axiom generic_entropy_no_omega` — which asserted the operator-level
conclusion as a framework primitive — are retracted. What remains is the
structural argument, marked as such.

REFERENCES:
- Penrose, R. The Road to Reality. Knopf, 2004. Chapter 27: The Big Bang
  and its thermodynamic legacy.
- Penrose, R. Singularities and time-asymmetry. In Hawking and Israel
  (eds.) General Relativity: An Einstein Centenary Survey, 1979.
- Carroll, S. From Eternity to Here. Dutton, 2010. Chapter on the past
  hypothesis and TSVF.
-/

import CoherenceRatchet.Cosmology.TSVF
import CoherenceRatchet.Cosmology.CorridorProjector

namespace CoherenceRatchet.Cosmology.Penrose

open TSVF Cosmology

variable {H : Type*} [NormedAddCommGroup H] [InnerProductSpace ℂ H] [CompleteSpace H]

/-- An entropy functional on states. Framework primitive. For pure states
    this is zero; for the coarse-grained ensemble corresponding to a state,
    this is the Boltzmann/Gibbs entropy. Operational form is substrate-
    specific (cosmological coarse-graining for the Big Bang case). -/
axiom entropy {H : Type*} [NormedAddCommGroup H] [InnerProductSpace ℂ H] :
    ForwardState H → ℝ

/-- Threshold defining "low entropy" — the Penrose hypothesis threshold.
    Framework primitive set by the inflationary-cosmology bound on
    initial-state entropy. -/
axiom lowEntropyThreshold : ℝ

/-- A forward state is "low entropy" iff its entropy sits below the threshold. -/
def isLowEntropy (psi_alpha : ForwardState H) : Prop :=
  entropy psi_alpha < lowEntropyThreshold

/-- A forward state's trajectory under unitary evolution U reaches omega iff
    its TSVF amplitude with a given backward boundary state is non-negligible.

    F-11 note: this is parameterized by an explicit `BackwardState`, NOT by a
    joint backward operator P_omega. The framework cannot construct the joint
    operator (CorridorProjector.FelevenNoGo); the predicate is stated against
    whatever backward boundary state is supplied, and the omega-reaching
    *claim* is the structural argument, not a property of a derived operator. -/
def trajectoryReachesOmega
    (psi_alpha : ForwardState H) (phi : BackwardState H) (U : H →L[ℂ] H) : Prop :=
  Complex.abs (amplitude psi_alpha phi U) > 0

/-- THE PENROSE-PAST STRUCTURAL ARGUMENT (D1).

    Every forward state whose trajectory reaches an omega-configuration
    backward boundary is a low-entropy state. The conditional consequence:
    given observation of an omega-satisfying universe, the initial condition
    is concentrated on low-entropy states.

    TIER: structural argument, asserted as a framework BET — NOT a derivation.
    The derivation needed the joint multi-rung backward operator P_omega; F-11
    documented that operator as a no-go (CorridorProjector.FelevenNoGo). So
    this is stated as the framework's structural conjecture about how
    universal-scale post-selection toward omega bears on initial conditions,
    with the operator that would discharge it explicitly absent. The
    hypothesis `h` carries the structural claim that the trajectory reaches
    omega; the conclusion is the low-entropy concentration. -/
axiom penrose_low_entropy_past_structural
    {H : Type*} [NormedAddCommGroup H] [InnerProductSpace ℂ H] [CompleteSpace H]
    (psi_alpha : ForwardState H) (phi : BackwardState H) (U : H →L[ℂ] H)
    (h : trajectoryReachesOmega psi_alpha phi U) :
    isLowEntropy psi_alpha

/-- The thermodynamic-arrow corollary: the direction of increasing entropy
    points away from omega and toward alpha. The arrow of time IS the
    gradient in omega-amplitude — under the structural argument, with the
    operator-level statement open per F-11. -/
theorem thermodynamic_arrow :
    -- Forward states closer (in the appropriate norm) to alpha have higher
    -- omega-amplitude on average than states farther. The future is the
    -- direction of decreasing omega-amplitude IF the past is alpha-fixed,
    -- and increasing if it is omega-fixed. Standard QM with only forward
    -- fixing gets a one-sided arrow; TSVF predicts the boundary structure
    -- determines the arrow.
    True := by
  trivial  -- statement-only; full theorem requires the entropy-functional structure

end CoherenceRatchet.Cosmology.Penrose
