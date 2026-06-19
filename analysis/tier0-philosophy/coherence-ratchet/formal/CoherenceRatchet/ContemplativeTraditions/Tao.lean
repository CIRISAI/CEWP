/-
ContemplativeTraditions.Tao — the framework's reading of Daoist vocabulary

The framework reads three central Daoist features as articulating the same
structural object its mathematics names. The readings are the framework's;
Daoist scholarship has not identified `Corridor` with the Tao or `γ·M(t)`
with wu-wei. The case for the readings is owed (papers/main.md §6, §6.5
null-check). The mappings below are framework-readings of structural
adjacency, not philosophical adjudications.

1. The Tao (the way) ↔ corridor occupation.
   "The Tao that can be named is not the eternal Tao" (Dao De Jing 1): the
   middle path of effective action is not a static prescription but an
   active corridor-occupation that requires continuous re-attunement.
   Mapping: the corridor (ρ_lower, ρ_upper).

2. Wu-wei (non-action / non-forcing) ↔ minimal active maintenance.
   The classical Daoist insight that effective action is the action with
   least forcing. Mapping: γ·M(t) at minimum sufficient to keep ρ in the
   corridor. Excess M(t) tips toward rigidity (rho → 1, k_eff → 1).
   Insufficient M(t) drifts toward chaos.

3. Yin-yang complementarity ↔ within-rung corridor at ρ_n.
   The classical insight that opposites coexist productively. Mapping:
   non-zero within-rung correlation ρ_n (yin-yang coexistence) at corridor
   range. Pure yin (ρ → 0) is dispersal; pure yang (ρ → 1) is collapse.
-/

import CoherenceRatchet.ContemplativeTraditions.CrossTraditionMap

namespace CoherenceRatchet.ContemplativeTraditions.Tao

open CoherenceRatchet.ContemplativeTraditions

/-- Framework reading: the Tao read as corridor occupation. -/
def tao_correspondence : Correspondence :=
  ⟨"Tao",
   ⟨"the Tao", "the way of effective action; middle path"⟩,
   FrameworkFeature.Corridor⟩

/-- Framework reading: wu-wei read as minimal active maintenance. -/
def wuwei_correspondence : Correspondence :=
  ⟨"Tao",
   ⟨"wu-wei", "non-action / non-forcing; minimal interference"⟩,
   FrameworkFeature.Corridor⟩  -- maps to the M(t)-minimization within corridor

/-- Framework reading: yin-yang read as within-rung corridor at ρ_n. -/
def yinyang_correspondence : Correspondence :=
  ⟨"Tao",
   ⟨"yin-yang", "complementary opposites in productive coexistence"⟩,
   FrameworkFeature.Corridor⟩

/-- The framework's Daoist correspondence set (each entry a framework reading). -/
def tao_mapping : List Correspondence :=
  [tao_correspondence, wuwei_correspondence, yinyang_correspondence]

end CoherenceRatchet.ContemplativeTraditions.Tao
