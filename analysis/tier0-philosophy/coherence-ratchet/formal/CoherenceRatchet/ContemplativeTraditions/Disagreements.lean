/-
ContemplativeTraditions.Disagreements — testing the recognition claim against
disagreement structure

The recognition claim at Level 6 says: contemplative-tradition vocabularies
correspond exactly to framework features where they speak about coherence
management, and disagree where they make distinct empirical or metaphysical
claims about substrates beyond corridor-occupation.

This file collects canonical disagreements and the framework's structural
prediction for what each disagreement is actually about.
-/

import CoherenceRatchet.ContemplativeTraditions.CrossTraditionMap

namespace CoherenceRatchet.ContemplativeTraditions.Disagreements

open CoherenceRatchet.ContemplativeTraditions

/-- DISAGREEMENT 1: Anatman (Buddhist non-self) vs Atman (Hindu self).

    Buddhist: no permanent self-substrate.
    Hindu (Advaita Vedanta): Atman is the eternal self, identical with Brahman.

    Framework prediction: at A3+, the goal-projector P_G IS the agent's
    identity-structure, but it is NOT a permanent substrate — it is a
    sequence of post-selection operations. The agent's present amplitude is
    a karma-and-grace decomposition, both terms of which are operations
    and structures, not substrate-objects.

    The framework agrees with anatman on "no permanent substrate" and
    disagrees with strict atman on "eternal self-substrate". However, the
    framework agrees with atman that the agent has continuity-of-identity
    across time (through karma-continuity) and that grace is partial-
    participation in something larger than the agent's own goals.
    The dispute is real (different inferences from observed phenomenology)
    but the framework shows both sides what they're pointing at: anatman
    correctly identifies that there is no substrate; atman correctly
    identifies that there is structural continuity. -/
def anatman_vs_atman : DivergenceEdgeCase :=
  ⟨"Dharma",
   "Hindu-Vedanta",
   "is the self a permanent substrate (atman) or a sequence of operations (anatman)?",
   "neither: agency is the karma-and-grace operator-composition; continuity is real but substrate-free"⟩

/-- DISAGREEMENT 2: Wu-wei (Daoist effortlessness) vs Logos (rational ordering).

    Daoist: effective action requires minimal forcing.
    Logos: the rational ordering principle is the universal framework.

    Framework prediction: both are correct at different scales.
    Wu-wei is the local prescription: at any agent's scale, minimize γ·M(t)
    subject to corridor maintenance.
    Logos is the universal prescription: the universal post-selection P_ω
    structures the corridor itself.

    The disagreement dissolves once the scales are distinguished: wu-wei
    is what an A3+ agent does inside the corridor; Logos is the structural
    pattern the corridor itself instantiates. -/
def wuwei_vs_logos : DivergenceEdgeCase :=
  ⟨"Tao",
   "Logos",
   "is effective action wu-wei (minimal forcing) or Logos (rational ordering)?",
   "both: wu-wei at the agent scale; Logos at the universal scale; corridor mediates"⟩

/-- DISAGREEMENT 3: Karma (Buddhist/Hindu causality) vs Grace (Christian gift).

    Buddhist/Hindu: present is shaped by past actions.
    Christian: present can be reshaped by grace from outside the agent.

    Framework prediction: both are correct components of the agent's present
    amplitude. The decomposition |Ψ_present⟩ = karma_operator · grace · ...
    formalizes this: karma is the self-authored component (past goal-
    projector composition), grace is the non-self-authored component (P_ω
    contribution beyond P_{G_agent}).

    The disagreement is about emphasis (which component is decisive for
    soteriology) and is real, but the framework dissolves the apparent
    contradiction: both components are present in any agent's structure. -/
def karma_vs_grace : DivergenceEdgeCase :=
  ⟨"Dharma",
   "Logos",
   "is present determined by past actions (karma) or non-self-authored gift (grace)?",
   "both: karma_operator and grace operator decompose the agent's present amplitude"⟩

/-- DISAGREEMENT 4: Sunyata (Buddhist emptiness) vs Providence (Christian
    teleology).

    Buddhist: things are "empty" of independent essence; no fixed teleology.
    Christian: history has a teleological orientation toward redemption.

    Framework prediction: both are correct about different framework
    features. Sunyata correctly identifies the corridor as condition, not
    entity (no fixed substrate). Providence correctly identifies the
    omega projector P_ω as a structural orientation of history toward
    multi-rung corridor occupation.

    The disagreement is about whether the structural orientation counts as
    "teleology". Framework reading: yes, but it's a post-selection
    teleology, not a predetermined-path teleology. -/
def sunyata_vs_providence : DivergenceEdgeCase :=
  ⟨"Dharma",
   "Logos",
   "is there a teleological orientation of history (providence) or is everything 'empty' of fixed essence (sunyata)?",
   "both: post-selection teleology via P_ω; structural orientation without substrate-essence"⟩

/-- The framework's overall prediction: contemplative-tradition disagreements
    are real but not unresolvable. Each disagreement points at a different
    aspect of the framework's structural features. The traditions agree on
    the corridor; they disagree on which adjacent feature to emphasize. -/
def all_disagreements : List DivergenceEdgeCase :=
  [anatman_vs_atman, wuwei_vs_logos, karma_vs_grace, sunyata_vs_providence]

end CoherenceRatchet.ContemplativeTraditions.Disagreements
