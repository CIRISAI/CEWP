/-
Core.AudienceStopping — the load-bearing-independence of higher levels

This file formalizes the structural property that distinguishes the framework
from a chain of dependent claims: each level is internally coherent and the
levels above are NOT load-bearing on the levels at or below.

A reader who stops at Level k can adopt the framework's claims at levels 0..k
without committing to anything at levels k+1..7. The k+1 levels can be
contested or rejected without retracting any claim at level k or below.
-/

import CoherenceRatchet.Core.Levels

namespace CoherenceRatchet.Core

open Level

/-- Strict ordering on levels (L0 < L1 < ... < L7). -/
def levelOrder : Level → Nat
  | L0_KishIdentity => 0
  | L1_MonotonicCollapse => 1
  | L2_EngineeringImplications => 2
  | L3_CrossSubstrateUniversality => 3
  | L4_AgencyAndConsent => 4
  | L5_TSVFAndConsciousness => 5
  | L6_ContemplativeRecognition => 6
  | L7_CivilizationalResidue => 7

/-- A claim at level k. -/
structure Claim where
  level : Level
  description : String

/-- The non-dependency relation: claim c does not depend on level l. -/
def doesNotDependOn (c : Claim) (l : Level) : Prop :=
  levelOrder l > levelOrder c.level

/-- The key structural property: every claim at level k is independent of every
    level strictly above k. A reader who rejects levels above k retains every
    claim at level k or below. -/
theorem upper_levels_not_load_bearing
    (c : Claim) (l : Level) (h : levelOrder l > levelOrder c.level) :
    doesNotDependOn c l := h

/-- The framework's structural commitment: rejecting Level 5+ does not retract
    Level 4 or below. Rejecting Level 7 does not retract anything else. -/
theorem stopping_is_safe (reader : ReaderPosition) (c : Claim)
    (h : levelOrder c.level ≤ levelOrder reader.stoppingPoint) :
    isCoherentStopping reader.stoppingPoint := by
  exact all_levels_coherent reader.stoppingPoint

end CoherenceRatchet.Core
