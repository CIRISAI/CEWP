/-
CoherenceRatchet.StructuralClaims
=================================

The five EMPIRICAL structural commitments of the framework, stated so that what
each test is trying to prove or disprove — and how — is explicit and
machine-checked.

This file deliberately proves NONE of the five claims. They are empirical: they
are about what coordinated substrates actually do, and they could be false.
What this file DOES prove is the falsification logic — that each claim is
equivalent to the non-existence of a specific falsifying witness — so that an
experiment exhibiting such a witness disproves the claim by a checked step.

Contrast with the Kish identity (k_eff = k/(1+ρ(k-1)), Core.BaseIdentity, with
K1–K4 proven): that IS a theorem — true by construction given its definitions,
not falsifiable. The Kish identity is the COORDINATE SYSTEM. The five claims
below are the COMMITMENTS. A theorem is not an empirical claim and an empirical
claim is not a theorem; this file holds the empirical claims.

Each claim is recorded as `framework_asserts_N : ClaimN` — an axiom, an
ASSERTION the framework makes and experiment N tries to break. The axioms are
not proved and not provable; that is the point. Finding a falsifier witness
contradicts the corresponding axiom via `claimN_iff`.
-/

namespace CoherenceRatchet.StructuralClaims

/-! ## Abstract carriers -/

/-- Any coordinated system: gene-regulatory network, brain, LLM, OSS project,
    social group, CMB mode-space. -/
axiom Substrate : Type

/-- A level of the emergence hierarchy (Ph0 … A5). -/
axiom Rung : Type

/-! ## Operational predicates — each answered by MEASUREMENT, not by proof -/

axiom Coordinated  : Substrate → Prop   -- exhibits sustained coordination
axiom Persistent   : Substrate → Prop   -- survives on long timescales
axiom InCorridor   : Substrate → Prop   -- occupies the bounded band, not a pole
axiom Maintained   : Substrate → Prop   -- has identifiable active maintenance (γM) work
axiom ThermalClosed : Substrate → Prop  -- evolves by closed-system thermal dynamics
axiom GenericInitial : Substrate → Prop -- generic high-entropy initial conditions
axiom MultiRung    : Substrate → Prop   -- exhibits multi-rung corridor structure
axiom BackwardConditioned : Substrate → Prop -- structure delivered by P_ω post-selection
axiom Carries      : Rung → Prop        -- the rung carries coordinated structure
axiom ForwardErgodic     : Rung → Prop  -- forward generator: a unique steady state
axiom BackwardNonErgodic : Rung → Prop  -- backward generator: many conserved quantities
axiom RungInCorridor     : Rung → Prop  -- the rung occupies a corridor under some k_eff
axiom RungPair : Type                   -- a pair of adjacent coordinated rungs
axiom CoordinatedPair   : RungPair → Prop -- both members are coordinated rungs
axiom MultiRungCorridor : RungPair → Prop -- both within-rung corridors AND the
                                          -- cross-rung τ corridor hold at once
axiom CouplingComparable : RungPair → Prop -- cross-rung coupling COMPARABLE to
                                           -- within-rung: g/J ~ O(1), Path-1
                                           -- measured band 0.47–1.47
axiom MaintenanceBreaksSymmetry : Rung → Prop -- the γM maintenance breaks the
                                              -- dynamical symmetries (Test E2)

/-! ## The falsification lemma — the one logical fact this file rests on -/

/-- A universally-quantified claim `∀ x, A x → B x` holds iff no falsifying
    witness `∃ x, A x ∧ ¬ B x` exists. Proven once; reused for all five. -/
theorem claim_iff_no_witness {α : Type} (A B : α → Prop) :
    (∀ x, A x → B x) ↔ ¬ ∃ x, A x ∧ ¬ B x := by
  constructor
  · intro h ⟨x, hA, hnB⟩
    exact hnB (h x hA)
  · intro h x hA
    apply Classical.byContradiction
    intro hnB
    exact h ⟨x, hA, hnB⟩

/-! ## Claim 1 — the corridor is a bounded attractor at every coordinated substrate

    A persistent coordinated substrate either occupies the corridor or pays
    identifiable maintenance work (Mode-(iii) absence).
    TEST E1: corridor-existence measurement on new A0+ substrates.
    FALSIFIER: a persistent coordinated substrate at a pole, unmaintained. -/

def Claim1 : Prop :=
  ∀ s, (Coordinated s ∧ Persistent s) → (InCorridor s ∨ Maintained s)

def Falsifier1 : Prop :=
  ∃ s, (Coordinated s ∧ Persistent s) ∧ ¬ (InCorridor s ∨ Maintained s)

theorem claim1_iff : Claim1 ↔ ¬ Falsifier1 :=
  claim_iff_no_witness _ _

axiom framework_asserts_1 : Claim1

/-! ## Claim 2 — forward and backward generators in different ergodicity classes

    At every rung carrying coordinated structure WHOSE MAINTENANCE BREAKS THE
    DYNAMICAL SYMMETRIES, the forward generator is ergodic (a unique steady
    state) and the backward generator is non-ergodic (many conserved
    quantities) — operator-level distinct.
    TEST E2: ergodicity-class measurement on each operational instantiation.
    FALSIFIER: a rung with symmetry-breaking maintenance where the split fails.
    STATUS: tested at n = 6 instances (exp_E2_ergodicity_split.py). The split
    holds for symmetry-breaking maintenance (4/6); it FAILS for symmetric
    maintenance (2/6: Heisenberg + symmetric bit-flip → non-ergodic forward).
    The `MaintenanceBreaksSymmetry` antecedent is the amendment E2 forced — the
    original unqualified Claim 2 was falsified as stated. γM ("injects
    distinction", Piece 2) is generically symmetry-breaking. -/

def Claim2 : Prop :=
  ∀ r, (Carries r ∧ MaintenanceBreaksSymmetry r) →
       (ForwardErgodic r ∧ BackwardNonErgodic r)

def Falsifier2 : Prop :=
  ∃ r, (Carries r ∧ MaintenanceBreaksSymmetry r) ∧
       ¬ (ForwardErgodic r ∧ BackwardNonErgodic r)

theorem claim2_iff : Claim2 ↔ ¬ Falsifier2 :=
  claim_iff_no_witness _ _

axiom framework_asserts_2 : Claim2

/-! ## Claim 3 — closed-system thermal dynamics cannot sustain a corridor unmaintained

    If a closed-system thermal substrate persistently occupies the corridor, it
    must be maintained. (Contrapositive: closed thermal + unmaintained ⇒ no
    persistent corridor.)
    TEST E3: closed-system thermalization simulation.
    FALSIFIER: a verifiably thermal closed system persistently in-corridor and
    unmaintained. -/

def Claim3 : Prop :=
  ∀ s, (ThermalClosed s ∧ InCorridor s ∧ Persistent s) → Maintained s

def Falsifier3 : Prop :=
  ∃ s, (ThermalClosed s ∧ InCorridor s ∧ Persistent s) ∧ ¬ Maintained s

theorem claim3_iff : Claim3 ↔ ¬ Falsifier3 :=
  claim_iff_no_witness _ _

axiom framework_asserts_3 : Claim3

/-! ## Claim 4 — the corridor recurs at every coordinated rung (fractal recurrence)

    Every rung carrying coordinated structure occupies a corridor under some
    effective-dimensionality coordinate.
    TEST E4: corridor-existence per rung, chemistry through cosmology.
    FALSIFIER: a coordinated rung with no corridor under any reasonable k_eff. -/

def Claim4 : Prop :=
  ∀ r, Carries r → RungInCorridor r

def Falsifier4 : Prop :=
  ∃ r, Carries r ∧ ¬ RungInCorridor r

theorem claim4_iff : Claim4 ↔ ¬ Falsifier4 :=
  claim_iff_no_witness _ _

axiom framework_asserts_4 : Claim4

/-! ## Claim 5 — multi-rung structure from generic initial conditions needs backward conditioning

    A substrate that reaches multi-rung corridor structure FROM GENERIC
    high-entropy initial conditions did so via backward conditioning (P_ω
    post-selection). Scope is cosmological ORIGIN — not corridor occupation in
    general, which forward dissipative dynamics with maintenance does produce
    (construct_pomega_lindblad.py).
    TEST E6: under the constructed soft P_ω, the forward amplitude from generic
    high-entropy initial conditions to multi-rung corridor structure.
    FALSIFIER: multi-rung corridor structure reached from generic initial
    conditions by purely forward dynamics. -/

def Claim5 : Prop :=
  ∀ s, (GenericInitial s ∧ MultiRung s) → BackwardConditioned s

def Falsifier5 : Prop :=
  ∃ s, (GenericInitial s ∧ MultiRung s) ∧ ¬ BackwardConditioned s

theorem claim5_iff : Claim5 ↔ ¬ Falsifier5 :=
  claim_iff_no_witness _ _

axiom framework_asserts_5 : Claim5

/-! ## Claim 6 — the cross-rung coupling corridor (AMENDED)

    At every coordinated rung PAIR that jointly satisfies the multi-rung
    corridor, the cross-rung coupling is COMPARABLE to the within-rung scale:
    g/J ~ O(1) — Path-1 measured 0.47–1.47, neither pole.

    AMENDMENT (2026-05-21, the second test-driven amendment after Claim 2).
    The original Claim 6 was a DOMINANCE gate, g/J ≳ 3, read off the abstract
    tower (crossrung_tower_scan.py). Path 1 of the six-pair series
    (crossrung_series/path1_tau/, real data, 2 rung pairs, pre-registered)
    measured the coupling ratio at 0.47–1.47 — neither Simon's near-
    decomposability (g/J ≪ 1) nor the strong dominance (g/J ≳ 3). The strong
    g/J ≳ 3 form is RETRACTED as tower-specific. The amended claim is the
    intermediate-coupling form: g/J ~ O(1), the measured band 0.47–1.47.
    Rationale that STANDS — O(1) cross-rung coupling is the existence condition
    for a hierarchy that is both stratified (distinct rungs; fails at g/J ≪ 1)
    and integrated (rungs coupled; fails at g/J ≫ 1). Rationale RETRACTED — an
    earlier draft called the band "one OOM (0.3, 3), the natural width of a
    corridor on a log axis." Wave 1's oom_width check (oom_width/RESULT.md)
    refuted that: the framework's measured corridors are SUB-decade (~0.5
    decade typical, range 0.16–0.68); the only clean one-OOM "corridor" was the
    (0.3, 3) band, which had been DEFINED as one OOM, not measured. So Claim 6
    is the measured O(1) band, ~0.5 decade wide like the others — not a special
    decade. It is still a corridor (bounded, off both poles); "one OOM" was an
    over-fit and is dropped.

    HEAD TO HEAD WITH SIMON. The amended claim still contradicts Simon's
    near-decomposability (Architecture of Complexity, 1962): Simon holds stable
    hierarchies have g/J ≪ 1; Pair A's 1.15 directly denies that. Claim 6 says
    coordinated rungs sit at intermediate coupling — territory neither Simon
    nor the strong-tower form predicted.
    TEST: the six rung-pair g/J series. Path 1 (the COUPLING RATIO) done — 2
    pairs + 24 LLM cells (w3), all O(1). Paths 2-3 (the canonical TIMESCALE
    g/J) ATTEMPTED at 2 substrates and measured at NEITHER: w3 (LLM) noise-
    blocked (1/48 gate-passes); w3b (GPU block pair) a bad-observable failure —
    the running-correlation-of-windows is noise-dominated block-resolved, its
    negative overshoot an artifact (oscillation_check.py refuted the normal-
    mode reading). Claim 6 holds in the coupling-ratio measure; the timescale
    measure is not accessible with the observables/substrates tried.
    FALSIFIER: a coordinated pair with the multi-rung corridor satisfied but
    g/J at a pole (≪ 1 or ≫ 1), outside the measured O(1) band. This is an
    empirical claim, not a theorem. -/

def Claim6 : Prop :=
  ∀ p, (CoordinatedPair p ∧ MultiRungCorridor p) → CouplingComparable p

def Falsifier6 : Prop :=
  ∃ p, (CoordinatedPair p ∧ MultiRungCorridor p) ∧ ¬ CouplingComparable p

theorem claim6_iff : Claim6 ↔ ¬ Falsifier6 :=
  claim_iff_no_witness _ _

axiom framework_asserts_6 : Claim6

/-! ## Summary

    Six empirical claims, each `ClaimN ↔ ¬ FalsifierN` machine-checked above.
    The framework asserts each (`framework_asserts_N`). An experiment exhibits a
    `FalsifierN` witness ⇒ `¬ ClaimN` ⇒ contradiction with `framework_asserts_N`
    ⇒ the framework is wrong at claim N. That is the whole test logic, and it is
    checked here. The corridor is the empirical object; the Kish algebra
    (Core.BaseIdentity) is the coordinate system it is stated in.

    ## Test ledger (2026-05-21 structural series; experiments/structural_series/)

    Claim 1 — E1 (LLM) weak. data_fmri (ABIDE-PCP, 139 controls) SUPPORTED —
      clean: functional-connectivity ρ median 0.266, in the A3+ corridor.
      data_allen (Allen Brain, 25 mouse visual-cortex sessions) is a DATA
      POINT, not a `Falsifier1` witness: mean pairwise neuron ρ ≈ 0.023, at the
      chaos pole on THAT observable. It is not a falsification, because the
      framework's canonical shape observable is the participation-ratio k_eff
      of the activity covariance, NOT the mean pairwise correlation, and cortex
      is the textbook case where the two diverge (small pairwise correlation,
      strong low-rank population structure). The owed test is the
      k_eff-of-covariance re-run on the same Allen data. If that ALSO lands at
      the chaos pole it becomes a `Falsifier1` witness and is recorded as one;
      the data-point status holds only until the canonical observable is run.
      data_tcga (7 new TCGA cancers, real GDC data, pre-registered) SUPPORTED —
      healthy tissue tight-banded off both poles, tumour drift 201/201 chaos-
      ward (reproduces the prior 176/176). Caveat: the absolute band centre is
      pipeline-dependent (GDC STAR-Counts ρ ≈ 0.34 vs the prior pipeline 0.27).
    Claim 2 — E2 (6 Lindbladian instances) FALSIFIED AS STATED → AMENDED. 2/6
      had non-ergodic forward generators; the `MaintenanceBreaksSymmetry`
      antecedent is the amendment the test forced.
    Claim 3 — E3 (closed chaotic chain) CONSISTENT. Energy conservation ⇒ no
      corridor attractor without maintenance.
    Claim 4 — E1 (LLM) weak; data_fmri SUPPORTED at the fMRI region scale;
      data_tcga the corridor recurs at 7 new cancers (tight bands, 0 pole-
      piled, no Falsifier4 witness); data_allen a chaos-pole data point at the
      cortical-neuron scale. Recurrence holds as "a tight band off both poles"
      — but the band CENTRE ranges across substrates and pipelines (TCGA ~0.34,
      fMRI ~0.27, 2/6 TCGA cancers above the A3+ ceiling). Recurrence-of-
      tightness is supported; recurrence-as-a-fixed-common-band is not, and the
      framework should state Claim 4 as the former.
    Claim 5 — E6 (Penrose-scope, 200 generic ICs) CONSISTENT — 0/200
      forward-evolve to the multi-rung corridor.
    Claim 6 — AMENDED. Path 1 (crossrung_series/path1_tau/, 2 real rung pairs,
      pre-registered) measured the cross-rung/within-rung coupling ratio at
      0.47–1.47 — neither Simon's near-decomposability nor the abstract tower's
      strong g/J ≳ 3. The strong-dominance form is retracted as tower-specific;
      Claim 6 is amended to the cross-rung COUPLING CORRIDOR at g/J ~ O(1)
      (coupling-ratio measure: Path 1's 2 pairs + w3's 24 LLM cells, median
      0.73). The canonical TIMESCALE g/J is measured at NO substrate — w3 (LLM)
      noise-blocked, w3b (GPU block) a bad-observable failure; oscillation_check
      refuted the normal-mode escape. Claim 6 holds in the coupling-ratio
      measure only. The second test-driven amendment (after Claim 2).

    Plus the orthogonality theorem (CMBOrthogonality.lean): the soft P_ω leaves
    the bulk CMB power spectrum exactly invariant — the framework is a strict
    extension of ΛCDM at the cosmological tier. PROVED, not an empirical claim.

    The spec is live: E2 amended Claim 2. data_fmri (human-neural) and
    data_tcga (cellular) are clean pre-registered positives for Claims 1 & 4;
    data_allen is a chaos-pole data point that surfaced an observable-
    consistency issue — the series so far used mean-pairwise ρ, while the
    framework's canonical shape observable is the participation-ratio k_eff.
    The k_eff re-test across E1, fMRI and Allen is owed; until it runs, Allen
    is a data point, not a `Falsifier1` witness, and no claim is retracted
    here. The consistent finding across positives: corridor EXISTENCE (tight
    band, off both poles) is robust; the band CENTRE is substrate- and
    pipeline-dependent. -/

end CoherenceRatchet.StructuralClaims
