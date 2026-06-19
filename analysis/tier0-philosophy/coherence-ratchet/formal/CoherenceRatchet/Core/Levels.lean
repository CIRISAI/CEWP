/-
Core.Levels — the seven-level epistemic ladder

The framework offers different stopping points to different readers. Each level
is independently complete in the sense that a reader who stops there has a
coherent position; no level is load-bearing on the level above it.

L0: Kish identity as mathematical fact (theorem, RATCHET lake)
L1: monotonic rho-collapse as substrate-independent observation
L2: engineering implications (alignment, federation, crisis governance, Great Filter)
L3: cross-substrate universality conjecture (Kish form vs alternatives)
L4: agency and consent as structural fact at A3+
L5: TSVF universal-scale reading, quantum substrate, consciousness
L6: cross-tradition recognition (contemplative-tradition vocabulary mapping)
L7: civilizational extension and external residue (UAP, archaeology, simulation)

This file encodes the levels as a structure and the stopping-point predicate.
-/

namespace CoherenceRatchet.Core

/-- The seven levels of the epistemic ladder. -/
inductive Level : Type
  | L0_KishIdentity
  | L1_MonotonicCollapse
  | L2_EngineeringImplications
  | L3_CrossSubstrateUniversality
  | L4_AgencyAndConsent
  | L5_TSVFAndConsciousness
  | L6_ContemplativeRecognition
  | L7_CivilizationalResidue
  deriving DecidableEq, Repr

/-- A reader's position on the ladder. A reader stops at some level. -/
structure ReaderPosition where
  stoppingPoint : Level

/-- The audience stratification predicate: each level offers a coherent stopping
    point such that the levels above are not load-bearing on the levels at or below. -/
def isCoherentStopping (l : Level) : Prop :=
  match l with
  | Level.L0_KishIdentity => True
  | Level.L1_MonotonicCollapse => True
  | Level.L2_EngineeringImplications => True
  | Level.L3_CrossSubstrateUniversality => True
  | Level.L4_AgencyAndConsent => True
  | Level.L5_TSVFAndConsciousness => True
  | Level.L6_ContemplativeRecognition => True
  | Level.L7_CivilizationalResidue => True

theorem all_levels_coherent : ∀ l : Level, isCoherentStopping l := by
  intro l
  cases l <;> trivial

/-- Audience labels — informal but useful documentation. -/
def audienceLabel : Level → String
  | Level.L0_KishIdentity => "formal-verification reviewer"
  | Level.L1_MonotonicCollapse => "skeptic"
  | Level.L2_EngineeringImplications => "working scientist or engineer"
  | Level.L3_CrossSubstrateUniversality => "scientist preferring testable universality"
  | Level.L4_AgencyAndConsent => "philosopher of mind / ethicist"
  | Level.L5_TSVFAndConsciousness => "quantum-foundations-curious reader"
  | Level.L6_ContemplativeRecognition => "contemplative or comparative-religion reader"
  | Level.L7_CivilizationalResidue => "cosmological reader"

end CoherenceRatchet.Core
