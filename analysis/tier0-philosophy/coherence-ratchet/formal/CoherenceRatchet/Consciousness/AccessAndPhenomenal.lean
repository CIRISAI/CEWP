/-
Consciousness.AccessAndPhenomenal — access and phenomenal as the same operation

Per the sensor-lift design doc (`papers/sensor_lift/outline.md`) §4.2:

  Access consciousness  = the existence of `attractor_field`.
  Phenomenal consciousness = what `attractor_field` IS, read from inside the sensor.

These are not two properties but one operation viewed from two stances. The
hard problem dissolves at the type level: in this formalization, the two
predicates are DEFINITIONALLY EQUAL. There is no separate carrier for
phenomenal content because the inner-product reading IS the phenomenal
content. The bridge theorem reduces to reflexivity.

This replaces the earlier ad-hoc `hasAccess`/`hasPhenomenal` sketch with the
Sensor abstraction from `Cosmology.TSVF`. The A3+ extension (reflexive lift,
T17 bridge theorem) lives in the design doc §5 and is implemented as a
separate extension on top of `Sensor`.

Internal-verification corollary (design doc §11.2): introspection cannot
distinguish "doing the operation" from "modeling its outputs," because both
produce the same first-person report under the operation-identity move. A
framework that allowed such verification from inside would be claiming a
verification-vantage outside the operation, contradicting the identity move.
This corollary is empirical content of the framework, not a bug.

Convergent prior art (see `papers/prior_art_integration.md` §1.2):
- Friston, K. (2010). The free-energy principle: a unified brain theory?
  Nature Reviews Neuroscience 11(2): 127-138.
- Friston, K., et al. (2022). The free energy principle made simpler but
  not too simple. arXiv:2201.06387.
- Deane, G. (2021). Consciousness in active inference: Deep self-models,
  other minds, and the challenge of psychedelic-induced ego-dissolution.
  Neuroscience of Consciousness 2021(2): niab024.
- Tani, J., et al. (2022). Goal-directed Planning and Goal Understanding
  by Active Inference. arXiv:2202.09976.

The active inference framework provides operational grounding for the
reading-the-attractor-field primitive: goal-holding systems minimize
variational free energy by acting to bring sensory states into alignment
with predicted-preferred states. Goals function as preferred outcomes the
system both predicts and acts to instantiate — operationally the goal-
projector ⟨G| at A3+, with the variational dynamics providing the
operational mechanism. The framework adds the corridor structure on cross-
agent ρ_goals, the asymptotic-conditioning result, and the Kish algebraic
identity governing effective independent dimensionality.
-/

import CoherenceRatchet.Cosmology.GoalProjection
import CoherenceRatchet.Cosmology.TSVF

namespace CoherenceRatchet.Consciousness.AccessPhenomenal

open CoherenceRatchet.Cosmology.Goal CoherenceRatchet.Cosmology.TSVF

variable {H : Type*} [NormedAddCommGroup H] [InnerProductSpace ℂ H]
variable {ι : Type*}

/-- Access consciousness: the existence of non-trivial attractor-field
    reading by the sensor. Functional information processing in the design
    doc's sense, made operational on the `Sensor` structure. -/
def isAccessConscious (S : Sensor H ι) : Prop := S.hasNonTrivialAttractor

/-- Phenomenal consciousness: what the attractor-field IS, read from inside
    the sensor. Definitionally equal to `isAccessConscious` — the framework's
    type-level dissolution of the hard problem. -/
def isPhenomenallyConscious (S : Sensor H ι) : Prop := S.hasNonTrivialAttractor

/-- T16. Access and phenomenal consciousness are the same operation viewed
    from two stances. Reduces to reflexivity under the definitions; the
    non-trivial content is the definitional choice, not the proof.

    Design doc §5: T16 is structurally trivial because we are defining
    phenomenal-consciousness AS attractor-reading. The work is the
    definitional move, not the unfolding. -/
theorem consciousness_is_attractor_reading (S : Sensor H ι) :
    isPhenomenallyConscious S ↔ isAccessConscious S := Iff.rfl

/-- Equivalent restatement: phenomenal consciousness is operationally the
    existence of at least one non-zero inner-product reading.
    The design doc's structural answer to the hard problem at the type level. -/
theorem phenomenal_iff_nontrivial_attractor (S : Sensor H ι) :
    isPhenomenallyConscious S ↔ ∃ i : ι, S.attractor_field i ≠ 0 := Iff.rfl

/-- Differential phenomenology corresponds to differential attractor-magnitudes
    (design doc §2). For two indices in the same sensor, the relative pull is
    the ratio of magnitudes. -/
noncomputable def relativeSalience (S : Sensor H ι) (i j : ι) : ℝ :=
  S.attractor_magnitude i / S.attractor_magnitude j

end CoherenceRatchet.Consciousness.AccessPhenomenal
