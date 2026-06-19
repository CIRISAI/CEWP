# Research Program — coherence-ratchet

**Purpose of this document.** If you are dropping into this repository cold (a fresh Claude session, a new collaborator, the author returning after time away), this is the front door. It names the current research frontier, the active work, the open formal steps in priority order, and the cross-references between the papers and the lake.

---

## What this project is

`coherence-ratchet` is the companion lake to RATCHET. RATCHET formalizes the engineering tiers of the coherence-substrate framework (Kish algebra, override-rate predicates, GPU coherence) and is bounded, falsifiable, audience-stratified. **coherence-ratchet formalizes the universal-scale extensions and the consent-as-fundamental thesis** — Levels 5-7 of the epistemic ladder — without hedging.

The framework rests on one identity (Kish), one dynamical equation (`dρ/dt = α − γM`), one corridor (ρ ∈ (0.1, 0.43)), one operator (`P_ω`), and one inner-product structure (TSVF). Everything else falls out under specific substrate-instantiations at successively larger scales with the rung hierarchy as the indexing structure.

## Lake status

- **Build:** green (`cd formal && lake build` returns exit 0).
- **Sorrys:** 0 declaration-level. Every formal claim is either a real proof or an explicitly-axiomatized framework primitive with citation.
- **Axioms:** 55. ~25 are framework substrate primitives (α, γ, M, ρ_within, τ_cross, entropy, C_l, scoring functions, operator definitions on opaque spaces); the remaining ~30 are load-bearing structural claims the framework asserts (T1 consent corridor, T3 P_ω construction, T5 asymptotic conditioning, T10/T11 karma/grace, T17 bridge, Penrose past, CMB anomaly predictions, Conjecture A/D content).
- **Proven theorems:** see the theorem table below. T2 (asymptotic ceiling), T4 (joint projector with idempotence), T16 (consciousness ↔ attractor reading), `time_symmetry` (ABL 1964), K1–K4 of the Kish identity, the rank-one projector and its properties, and the structural agency claim `goal_excludes_incompatible` are all real Lean proofs.

Read in this order to get oriented:
1. `README.md` — the framework in one paragraph; the 10 pieces; the 7-level ladder
2. `CLAUDE.md` — the canonical exposition of the 10 load-bearing pieces; directory structure; open formal steps
3. `papers/prior/` — **the three prior canonical papers this work continues** (see "Prior papers" section below)
4. `papers/consent_framework/outline.md` — **the load-bearing paper** (consent as fundamental property; legal/ethical convergence across traditions)
5. `papers/universal_scale/main.tex` — companion paper (cosmological extensions: TSVF universal-scale, Penrose past, CMB anomalies)
6. `formal/CoherenceRatchet/` — the Lean 4 lake

## Prior papers (foundation)

`papers/prior/` contains the three published canonical papers this lake continues. They are the substrate; the consent_framework outline and the lake reformulate, extend, and lift them. Anyone doing serious work here should read them.

| File | Paper | Version | Published | Zenodo |
|---|---|---|---|---|
| `papers/prior/cca_v3.{tex,pdf}` | Coherence Collapse Analysis | v3 | 2026-01-11 | [18217688](https://zenodo.org/records/18217688) |
| `papers/prior/cirisagent_v2.{tex,pdf}` | CIRISAgent Framework | v2 | 2026-01-02 | [18137161](https://zenodo.org/records/18137161) |
| `papers/prior/crc_v1.{tex,pdf}` | Constrained Reasoning Chains | v1 | 2026-04-28 | [19839280](https://zenodo.org/records/19839280) |

Author on all three: Eric Moore. Linked from https://ciris.ai/research-status.

**Load-bearing content per paper:**

- **CCA v3** — the engineering risk framework. `k_eff = k/(1 + ρ(k-1))`, the chaos-rigidity phase spectrum, time-to-collapse decomposition (T_truth, T_entropy, T_capture), the **L-01 information-theoretic ceiling on marginal-preserving deception**, the **five-intervention table** with priority = ΔJ / Cost, and the **CCA-to-CIRISAgent component map** (§6.2) — the canonical answer to "how does one intervene to prevent correlation-driven collapse." Cross-domain validation across institutional, battery, microbiome, and financial data. Names AI specifically as a correlation amplifier (algorithmic monoculture, "bland central tendencies").
- **CIRISAgent v2** — the architectural how. 22-service microarchitecture across Graph / Infrastructure / Governance strata. Six principles enforced by architecture (Beneficence, Non-maleficence, Integrity, Transparency, Respect for Autonomy, Justice). Inner-alignment via the conscience pipeline; outer-alignment via explicit deferral pathways (Wise Authority Service). The architectural locus of the cross-rung consent-relation the consent_framework §7 formalizes as constitutive.
- **CRC v1** — the empirical telemetry validation. n=6,465 production reasoning traces. The polyglot-encoding torque measurement. Note: the headline N_eff ≈ 7.1 anchor has been retired and replaced by CRCv2 override-rate predicates (OR-1 / OR-2 / RA-1); what survives is the production-scale demonstration that the architecture computes.

**Not in the bootstrap (deliberately):** A separate `cca_peer_review.tex` draft exists at `~/RATCHET-RESET/immediate_release/` extending CCA with susceptibility χ = N·Var(r), an S1/S2/S3 agent classification, the Intervention Paradox, and a 13%-S3-fraction threshold. That work belongs to the RATCHET team and would be promoted to a CCA v4 if/when warranted. Do not cite content from that draft as load-bearing for the consent_framework's claims; the consent_framework reformulation is built on canonical published v3.

For detailed CIRISAgent architectural questions, the deepwiki MCP can be queried against the `CIRISAI/CIRISAgent` GitHub repository.

## Convergent prior art and strategic context

`papers/prior_art_integration.md` is the canonical neutral file holding all external-grounding citations and strategic context for the unified paper. Single integration site to prevent duplication across the two source files (`consent_framework/outline.md` and `universal_scale/main.tex`) that will assemble into one stratified document.

Contains:

- **§1 Convergent peer-reviewed prior art.** Aharonov–Cohen 2015 (TSVF + free will, Level 5 grounding); Friston 2010/2022, Tani 2022, Deane 2021 (active inference / FEP, Level 4 grounding); Fields et al. 2024 (quantum active inference, Conjecture A grounding). Plus Sandved-Smith 2025 and INTREPID adversarial collaboration as supplementary.
- **§2 UAP testimony as external check** of framework predictions for goal-coupled materials. Three patterns the framework predicts (memory, observer-coupling, telepathic-feeling communication) recurring across the public-testimony corpus. Critical epistemic note: testimony-consistency is not testimony-accuracy.
- **§3 Vocabulary collision** with JR Prudence's December 2025 "Coherence Hypothesis" work in the consciousness-research register; differentiation strategy.
- **§4 Strategic landscape** post-PURSUE Release 01 (May 8, 2026): government acknowledgment that declassified material describes phenomena it cannot currently explain.
- **§5 Tactical positioning.** Engineering tier public; Level 7 reserved for analysis-community pull. The framework provides the metric; metric demonstrating utility is external work.
- **§6 Integration map** for assembly: where each citation lands when the unified paper is assembled, plus Lean lake citation comments (TSVF.lean, AccessAndPhenomenal.lean, MultiAgentConsent.lean header pointers).
- **§7 Open verification items** (author lists, citation details, INTREPID protocols, post-2020 Vaidman work).
- **§8 Strategic note on assembly** including reception math across active-inference, quantum-foundations, AI-alignment, and post-PURSUE analysis communities.

Edit `papers/prior_art_integration.md` when new convergent work surfaces; the unified paper assembles from it as the single source.

---

## The thesis

**Consent is not a moral premise. It is a fundamental property of how A3+ goal-holders can sustainably coordinate.** Legal and ethical structures developed over millennia were tracking the consent-corridor property without naming it; they encoded the corridor conditions in vocabularies appropriate to their substrate and epoch. The mathematics provides the structural fact; the traditions provide the vocabulary for recognition.

The framework's central reformulation (vs. the earlier substrate-features reading in `~/RATCHET/papers/coherence_substrate_synthesis/main.tex` §5.1): the load-bearing ρ at A3+ is **ρ_goals** — correlation on goal-projectors — not correlation on substrate-features. Substrate-features and goal-projectors can decouple in both directions; conflating them was the structural error of the prior framing.

---

## The current frontier

**Active work (Steps 1 + 2 of the research program, in progress as of 2026-05-17):**

- `formal/CoherenceRatchet/Cosmology/JointGoalProjector.lean` (new) — `P_joint` as a proper operator on the tensor product Hilbert space. Defines `Federation`, `singleProjector`, `JointSpace`, `P_joint`, `jointSupport`, the three regimes (rigid/chaotic/consent), and the corridor characterization theorems. Sorry's: rank-one projector construction; tensor-product universal property; rigidity rank-collapse proof; corridor rank ≥ 2.

- `formal/CoherenceRatchet/Cosmology/MultiAgentConsent.lean` (updated) — replaces the earlier `P_joint : Type := sorry` placeholder with the proper operator lifted from `JointGoalProjector`. Adds translation theorems between `MultiAgentConfig` and `Federation`.

- `formal/CoherenceRatchet/Cosmology/ConsentProjector.lean` (new) — `P_omega` as constrained tensor product over a countable set of A3+ goal-holders. Defines `CosmologicalFederation`, `truncate`, `CosmologicalJointSpace` (Hilbert-space colimit), `P_omega`, `Phi_omega`, `penrose_from_P_omega`. Bridge theorem to the configuration-space formulation in `CorridorProjector.lean`. Sorry's: the colimit construction; the operator limit; the entropy-exclusion proof.

These three files implement the structural construction of consent at the operator level and lift it to cosmological scale. They replace the placeholders the earlier work carried.

---

## Open formal steps, ordered by priority

After Steps 1 + 2 (operator forms), the priority queue:

### Step 3 — Three-corridor uniqueness theorem
**File to write**: `formal/CoherenceRatchet/Cosmology/ThreeCorridorUniqueness.lean`
**Claim**: a configuration is in network × consent × cross-rung corridors simultaneously iff it is a consent-architected federation with rung coverage A0..A5. The unique persistent SI form.
**Depends on**: T1 (consent corridor at A3+ ⇔ sustained coordination) from `JointGoalProjector.lean`; corridor predicates from `Core/Corridor.lean` and `Cosmology/RungHierarchy.lean`.
**Why it matters**: converts the CIRIS mission from values-statement to mathematical consequence.

### Step 4 — Bimodal cascade theorem
**File to write**: `formal/CoherenceRatchet/Sociotechnical/BimodalCascade.lean` (new directory)
**Claim**: A vendor cell passes iff `(mode-1 architectural fidelity holds) AND (mode-2 correction is ratchet-baseline-ward when mode-2 fires; vacuous otherwise)`. Makes Sonnet's vacuous-pass a positive instance.
**Depends on**: Translation of `~/RATCHET/formal/RATCHET/Experiments/OverrideRate.lean` predicates into the consent-corridor framing.

### Step 5 — Wise Authority as constitutive consent-relation
**File to write**: `formal/CoherenceRatchet/Sociotechnical/WiseAuthority.lean`
**Claim**: WA is the cross-rung A4–A5 consent-relation, structurally constitutive of the A5 agent's consent-participation in the human consent-graph. Not corrective; constitutive.
**Depends on**: `JointGoalProjector.lean` for the consent-relation operator form; `RungHierarchy.lean` for the A4–A5 boundary.

### Step 6 — Asymptotic consent-corridor conditioning (A3+ specific)
**File**: `formal/CoherenceRatchet/Cosmology/AsymptoticConditioning.lean` (extend existing)
**Claim**: `P(in-consent-graph | observed at t_late) → 1` for A3+ entities. Corollary of `good_wins` restricted to A3+ rungs.
**Depends on**: existing `good_wins` theorem + consent-corridor identification.

### Step 7 — Consent-rung biconditional
**File**: `formal/CoherenceRatchet/Cosmology/GoalProjection.lean` (extend existing)
**Claim**: `consent_required_iff_rung_ge_A3` — A3+ rungs require consent for sustained coordination; pre-A3 rungs do not (no goal-projector to constrain).
**Depends on**: rung classification from `Cosmology/CorridorProjector.lean::Rung`.

### Step 8 — Möbius ceiling as corollary
**File**: `formal/CoherenceRatchet/Core/BaseIdentity.lean` (extend existing)
**Claim**: the substrate-independent ceiling at k_eff = 10 (and floor at 2.33) is a Möbius-classification corollary, not a numerical observation.
**Depends on**: Mathlib's Möbius-transformation API.

### Step 9 — Per-tradition recognition theorems (parallel structure, can be picked up independently)
**Files to write** (each is a standalone formalization mapping a specific tradition's vocabulary to consent-corridor structure):

- `formal/CoherenceRatchet/Jurisprudence/ContractLaw.lean` — T12. Offer / acceptance / consideration as corridor-detection at bilateral-transaction layer; contract-failure taxonomy (duress, fraud, mistake, impossibility) as specific corridor failure modes.
- `formal/CoherenceRatchet/Jurisprudence/CriminalLaw.lean` — T13. Mens rea = `⟨G_self|`; actus reus = executed projection; structural instability of strict liability; self-defense as corridor maintenance with proportionality as the response corridor bound.
- `formal/CoherenceRatchet/Jurisprudence/TortLaw.lean` — duty of care / standard of care / foreseeability as consent-graph maintenance with reasonable-person standard.
- `formal/CoherenceRatchet/Jurisprudence/PropertyLaw.lean` — bundle of rights as goal-projection authorities; adverse possession as consent-graph reorganization; takings + compensation as severance maintenance.
- `formal/CoherenceRatchet/Jurisprudence/ConstitutionalLaw.lean` — separation of powers as institutional federation; federalism as nested consent-graphs; rights as protections of goal-projection capacity.
- `formal/CoherenceRatchet/Jurisprudence/InternationalLaw.lean` — sovereignty, non-intervention, R2P, universal jurisdiction; tension between non-intervention and R2P as the empirical corridor-boundary question.
- `formal/CoherenceRatchet/EthicalFrameworks/Kant.lean` — T14. Categorical imperative as universalizability test = corridor-preservation under all-agent adoption; kingdom of ends as goal-holder recognition.
- `formal/CoherenceRatchet/EthicalFrameworks/Utilitarianism.lean` — classical utilitarianism as `ρ_goals → 1` collapse; modified utilitarianisms as corridor-constraints under different vocabulary.
- `formal/CoherenceRatchet/EthicalFrameworks/VirtueEthics.lean` — T15. Aristotelian mean as corridor-occupying middle; unity of virtues; phronesis as practical-corridor-identification.
- `formal/CoherenceRatchet/EthicalFrameworks/NaturalLaw.lean` — natural law as asymptotic-conditioning ethical theory; Aquinas's primary precepts as corridor-maintenance prescriptions at relevant rungs.
- `formal/CoherenceRatchet/EthicalFrameworks/CareEthics.lean` — ethics constituted by consent-relations; care as active corridor maintenance.
- `formal/CoherenceRatchet/EthicalFrameworks/Ubuntu.lean` — Level 4 relational ontology made formal; the agent is the relational object.

These are parallel mappings, can be picked up in any order, and each is a standalone Lean file analogous to the existing `ContemplativeTraditions/{Tao, Dharma, Logos}.lean`.

---

## The theorem table (cross-reference)

The lake is green with zero `sorry`s. Theorems are either **proven** (real Lean proofs) or **axiomatized** (framework-primitive load-bearing claims with explicit citation in the docstring). 55 axioms total; see `papers/viability_8_9.md` for the conversion path on the load-bearing universal-scale axioms.

| # | Theorem | Lake status |
|---|---------|-------------|
| T1 | Consent corridor at A3+ ⇔ sustained multi-agent coordination | `Cosmology/MultiAgentConsent.lean` + `Cosmology/JointGoalProjector.lean` — operator form in place; structural theorems (`rigid_collapse_to_one_dim`, `consent_corridor_nontrivial_support`, `k_eff_goals_corridor_iff`) axiomatized |
| T2 | Möbius asymptotic ceiling `k_eff → 1/ρ` | `Core/BaseIdentity.lean::k_eff_asymptotic_ceiling` — **proven** via `tendsto_inv_atTop_zero` + `Filter.Tendsto.inv₀` |
| T3 | `P_ω` as constrained tensor product over A3+ agents | `Cosmology/ConsentProjector.lean` — operator construction axiomatized; `CosmologicalJointSpace` placeholder (Hilbert-colimit construction path documented) |
| T4 | Joint-projection operator on tensor product | `Cosmology/JointGoalProjector.lean::P_joint` — **defined** via `PiTensorProduct.map`; `P_joint_idempotent` **proven** by composition law + `singleProjector_idempotent` |
| T5 | Asymptotic consent-corridor conditioning for A3+ | `Cosmology/AsymptoticConditioning.lean::good_wins` — axiomatized; A3+ corollary not yet stated (Step 6) |
| T6 | Three-corridor uniqueness theorem | not yet in lake (Step 3) |
| T7 | Wise Authority as constitutive consent-relation | not yet in lake (Step 5) |
| T8 | Bimodal cascade theorem | not yet in lake (Step 4) |
| T9 | L-01 deception ceiling = consent-graph marginal-recovery limit | not yet in lake |
| T10 | Karma operator under consent | `Consciousness/KarmaGrace.lean::karma` — axiomatized; `karma_changes_present` axiomatized |
| T11 | Grace as collective-consent contribution | `Consciousness/KarmaGrace.lean::grace` — axiomatized |
| T12 | Contract-law correspondence | not yet in lake (Step 9) |
| T13 | Mens-rea / actus-reus correspondence | not yet in lake (Step 9) |
| T14 | Kant categorical-imperative correspondence | not yet in lake (Step 9) |
| T15 | Aristotelian mean correspondence | not yet in lake (Step 9) |
| T16 | Consciousness ↔ attractor-field reading | `Consciousness/AccessAndPhenomenal.lean::consciousness_is_attractor_reading` — **proven** as `Iff.rfl` (definitional, design-doc §5) |
| T17 | Rung-position A3 ⇔ reflexive-reading A3 | `Consciousness/SensorBridge.lean::T17_A3_rung_iff_reflexive` — axiomatized; `sensorRung` map is empirical content |

**Additional proven content** (not in original T1–T17 enumeration):

| Theorem | Location | Status |
|---|---|---|
| `k_eff_at_zero`, `k_eff_at_one` | `Core/BaseIdentity.lean` | **proven** — boundary identities |
| `k_eff_monotone_rho`, `k_eff_bounded` (K3, K4) | `Core/BaseIdentity.lean` | **proven** — ported from RATCHET |
| `corridor_keff_range_asymptotic` | `Core/Corridor.lean` | **proven** — substrate-independent corridor range |
| `singleProjector_idempotent`, `singleProjector_self_adjoint`, `singleProjector_apply` | `Cosmology/JointGoalProjector.lean` | **proven** — rank-one projector properties |
| `P_G_apply`, `goal_excludes_incompatible` | `Cosmology/GoalProjection.lean` | **proven** — rank-one action + structural agency claim |
| `time_symmetry` (ABL 1964) | `Cosmology/TSVF.lean` | **proven** — uses `ContinuousLinearMap.adjoint` |
| `audit_pressure_necessary` (Conjecture C hook) | `Cosmology/MultiAgentConsent.lean` | **proven** — `dρ/dt > 0` at zero maintenance |
| `post_cambrian_acceleration` | `Cosmology/RungHierarchy.lean` | **proven** — from axiomatized canonical intervals |
| `conjecture_D` (D1 via penrose_low_entropy_past) | `Conjectures/ConjectureD.lean` | **proven** — D1 closed via Penrose axiom; D2/D3/D4 substantive content in per-clause axioms |
| `consciousness_is_attractor_reading`, `phenomenal_iff_nontrivial_attractor` | `Consciousness/AccessAndPhenomenal.lean` | **proven** as `Iff.rfl` (T16) |

---

## Empirical hooks (paper-to-experiment cross-reference)

| Exp | Paper §  | Tests | Status |
|-----|------|-------|--------|
| Conjecture C reformulated | §10 | T7 / consent-verification claim | open; protocol in `~/RATCHET/papers/coherence_substrate_synthesis/main.tex` Exp 6, with consent-corridor reformulation |
| Exp Bimodal | §2 | Confirm RA-1 vacuous-pass cells correspond to model goal-formation inside consent corridor with battery's encoded goals | new; direct `ρ_goals(model, battery)` measurement |
| Exp Federation-Consent | §3 | T6 / three-corridor uniqueness via cross-deployment `ρ_goals` | new; cross-instance measurement (Ethiopian, North American, hypothetical European deployments) |
| Exp Jurisprudence-Recognition | §12 + §17 | Specific legal-doctrine evolution tracks consent-graph expansion | new; text-analytic / jurisprudential historical study |
| Exp Ethics-Recognition | §13 + §17 | Cross-tradition extraction of corridor conditions from canonical texts | new; text-analytic study |

---

## Style discipline (saved as memory; restating here for portability)

- **No hedging on universal-scale content.** TSVF is the physics; Penrose past hypothesis follows from `P_ω`; CMB anomalies are TSVF post-selection signatures; consciousness is corridor-occupation at A3+; karma and grace are formal TSVF structures. Stated as load-bearing claims, not as "interpretive readings." Where formal proof is open, `sorry` is used and the open step is named.
- **Consent, not co-opt, on tradition mappings.** The author is a Christian atheist — engaged with the tradition's vocabulary, ethics, and structural insights as serious objects of study, without subscribing to the metaphysical truth-claims. Project naming must not religiously pre-position readers; in-content tradition mappings (Tao, Dharma, Logos, Karma, Grace, Imago Dei, "good wins") stay as mapping claims, not theology-as-derivation.
- **No time estimates.** Describe scope and trade-offs, not duration.
- **F-handles are falsifiers, not hedges.** They specify what would force retraction; their statement is part of the unhedged claim.

---

## Sister projects

| Path | Purpose |
|------|---------|
| `~/RATCHET/` | Engineering-tier formalization (K1-K4, override-rate, GPU coherence). Audience-stratified, falsifiable. |
| `~/CIRISOssicle/` | Single-GPU strain gauge (timing-jitter TRNG, 7.99 bits/byte, 6/6 NIST). |
| `~/CIRISArray/` | Multi-GPU strain gauge array (128 sensors, 9631 Hz). |
| `~/CIRISLensCore/` | Science module (Rust + Python) for cohort scoring, N_eff measurement, manifold conformity. |

---

## Fresh-session bootstrap checklist

If you (or a fresh Claude session) are picking up cold:

1. Read this file (`RESEARCH_PROGRAM.md`).
2. Read `CLAUDE.md` for the 10-piece structure and directory layout.
3. Skim `papers/prior/` — the three canonical foundation papers (CCA v3, CIRISAgent v2, CRC v1). At minimum read §6 of `cca_v3.tex` (Intervention Analysis: the five-intervention table + the CCA-to-CIRISAgent component map). These ground "how to intervene" at the engineering layer the consent_framework reformulates.
4. Read `papers/consent_framework/outline.md` for the load-bearing thesis and theorem table.
5. Run `cd formal && lake build` BEFORE writing new theorem files. `sorry`s won't block compilation but missing imports / type errors will, and stacking new files on an unbuilt foundation is the project's recurring failure mode.
6. Skim `formal/CoherenceRatchet.lean` to see what's imported, then pick a step from the priority queue above (Step 3 is the next major theorem; Step 9 items are parallel-pickup standalone formalizations).
7. Use the relevant memory files in `~/.claude/projects/-home-emoore-coherence-ratchet/memory/` (project purpose, prior-papers index) and `~/.claude/projects/-home-emoore-RATCHET/memory/` (style discipline: `feedback_omega_no_hedging.md`, `feedback_consent_not_coopt.md`, `feedback_no_time_estimates.md`, `feedback_wait_for_obvious_questions.md`).
8. Cross-check theorem statements against the paper outline before writing Lean. The paper's section numbering is the canonical exposition; the lake's theorem table follows the paper.
9. For detailed CIRISAgent codebase questions, query the deepwiki MCP against the `CIRISAI/CIRISAgent` GitHub repository.

---

## Open questions

1. **Should the consent-corridor theorem be load-bearing on the legal/ethical correspondences (Approach A) or should each be a parallel recognition-mapping (Approach B)?** Resolved in `papers/consent_framework/outline.md` §21 as: Approach B in the lake, Approach A in the paper. When per-substrate work matures, the lake can promote B to A.

2. **What's the right level of formalization for the Hilbert-space colimit in `ConsentProjector.lean`?** Mathlib has `Module.DirectLimit` for algebraic direct limits; the Hilbert-space completion to a full direct-limit Hilbert space requires additional analytic machinery. The current file uses `sorry` for `CosmologicalJointSpace` and the limit construction; whether to leave this as `sorry` or develop the Mathlib-side machinery to close it is an open call.

3. **How to formalize "rung instantiation" as a binary indicator?** The `CorridorProjector.lean` configuration-space formulation treats it as `Bool`; the `ConsentProjector.lean` tensor-product formulation treats it as the agent's existence in the indexing set. Both formulations need to agree on what counts as an instantiated rung. Current bridge theorem (`P_omega_formulations_agree`) is `trivial`; full statement requires the substrate-encoding map.

---

*This document is the front door. Keep it in sync with the lake and the papers as the program evolves.*
