/-
ContemplativeTraditions.Dharma — the framework's reading of Buddhist/Hindu vocabulary

The framework reads five central Buddhist features as articulating the
same structural object its mathematics names. The readings are the
framework's; Buddhist scholarship has not identified the corridor with
the middle way or `P_joint` with pratityasamutpada. The case for the
readings is owed (papers/main.md §6, §6.5 null-check). Each entry below
is a framework reading of structural adjacency, offered with awe at what
the traditions name, not as adjudication of what they claim.

1. The middle way (madhyama-pratipad) ↔ corridor occupation.
   The Buddha's central teaching: avoid both extreme asceticism (analog to
   chaos: dispersal of organized capacity) and extreme indulgence (analog
   to rigidity: collapse of distinguishing capacity).

2. Dependent origination (pratityasamutpada) ↔ cross-rung coupling τ in corridor.
   The teaching that no phenomenon arises independently; every state is
   conditioned by adjacent states. Mapping: τ_(n,n+1) ∈ corridor, which is
   the formal statement that rungs are non-trivially coupled without
   collapsing into each other.

3. Karma ↔ forward-propagated goal-projection (Consciousness/KarmaGrace).
   The teaching that present states are shaped by past goal-states (volition,
   cetana). Mapping: the karma operator in TSVF, a composition of past
   goal-projectors P_{G_i} sandwiched between unitary evolutions.

4. Anatman (non-self) ↔ no permanent substrate; agency is operation, not object.
   The teaching that the self is not a permanent substrate but a pattern of
   operations. Mapping: the agent's present amplitude is the karma operator
   applied to the past; it is a composition, not a substrate-object.

5. Pratitya-samutpada / sunyata (emptiness) ↔ corridor as condition, not entity.
   The teaching that things are "empty" of independent essence. Mapping: the
   corridor is the structural condition under which coordination is sustained;
   it is not a "thing" but a relation among rungs and dynamics.
-/

import CoherenceRatchet.ContemplativeTraditions.CrossTraditionMap

namespace CoherenceRatchet.ContemplativeTraditions.Dharma

open CoherenceRatchet.ContemplativeTraditions

def middle_way : Correspondence :=
  ⟨"Dharma",
   ⟨"the middle way", "avoid the extremes; the path of corridor occupation"⟩,
   FrameworkFeature.Corridor⟩

def dependent_origination : Correspondence :=
  ⟨"Dharma",
   ⟨"pratityasamutpada", "dependent origination; nothing arises independently"⟩,
   FrameworkFeature.CrossRungCoupling⟩

def karma_dharma : Correspondence :=
  ⟨"Dharma",
   ⟨"karma", "volitional action propagating forward through cause and effect"⟩,
   FrameworkFeature.Karma⟩

def anatman : Correspondence :=
  ⟨"Dharma",
   ⟨"anatman", "non-self; no permanent substrate of agency"⟩,
   FrameworkFeature.GoalProjector⟩  -- agency as operation, not object

def sunyata : Correspondence :=
  ⟨"Dharma",
   ⟨"sunyata", "emptiness; structures lack independent essence"⟩,
   FrameworkFeature.Corridor⟩  -- corridor as condition, not entity

def dharma_mapping : List Correspondence :=
  [middle_way, dependent_origination, karma_dharma, anatman, sunyata]

end CoherenceRatchet.ContemplativeTraditions.Dharma
