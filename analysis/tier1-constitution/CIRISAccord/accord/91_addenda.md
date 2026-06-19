# Addenda

> Operational addenda binding the Accord text to live implementation evidence. Addenda are renewed with the Accord version; each is dated and cites reproducible artifacts.

---

## Addendum 1 — ASI-Readiness Status & Regulatory Cross-Walk Binding

**Adopted**: 2026-06-10 (1.3-RC2) · **Steward**: founder (per Book VIII Ch 9)

### 1.1 Purpose

The Scope section makes a conditional claim: this framework *explores* a candidate alignment protocol for ASI, gated on the four Release Candidate requirements in the Introduction. This addendum binds that claim to the operational evidence that now exists, and states plainly what remains. ASI **readiness** is not achieved alignment; it is the state in which the verification scaffolding that Book IX's mathematics requires — independent constraint manifolds, attestation surfaces, a conscience layer that cannot be normalized away, kill-switch surfaces, and the Annex D Catastrophic-Risk Evaluation — exists in implemented, auditable form with its gaps named.

### 1.2 The regulatory cross-walk is now operational

The working cross-walk is the **CIRISAgent compliance directory** (`CIRISAgent/compliance/`, agent release 2.9.6-stable), which maps the framework's implementation to four senior governance frameworks at paragraph grain (Annex C carries the two-layer structure: this operational cross-walk plus the informative statutory table pending legal verification):

* **Sources cross-walked**: *Magnifica Humanitas* (religious magisterium, 2026) · EU HLEG Ethics Guidelines for Trustworthy AI (governmental advisory, 2019) · IEEE Ethically Aligned Design 1st ed. (technical society, 2019) · ASEAN Guide on AI Governance and Ethics (multilateral political, 2024). Four institutionally-distinct shapes — the convergence is structural evidence that the framework's principle scaffolding is not an artifact of any one tradition.
* **Structure**: 27 stable dimensions (D01–D27, from `SEED_DIMENSIONS.yaml` v1.0; 16 attested by all four sources, 11 by three). Each dimension document carries an auto-rendered regulatory top (per-source citations, wire form, convergence notes) and a human-authored implementation section with `file:line` references that must grep-verify against current main.
* **Evidence discipline**: every numeric claim is reproducible from dated, script-generated baselines (`compliance/baselines/`); prose may cite baselines but never embed unverifiable numbers. A four-level validation hierarchy runs from implementation files (ground truth) up through public claims (`compliance/MEASUREMENT_METHODOLOGY.md`).
* **Honesty discipline**: each dimension carries a "Known gaps" inventory; substrate-gated items are flagged with the owning substrate rather than claimed. Ten cross-cutting findings are inventoried in the compliance README (e.g., typed wire envelopes not yet emitted; the LensCore detector family not yet shipped; reverse-axis reconsideration gap).

Annex C remains in the text as the future home of statutory mappings (EU AI Act articles, NIST AI RMF, ISO/IEC 42001) once legal review completes; the compliance directory is the **live, evidence-bearing** cross-walk until then and feeds it.

### 1.3 Wire format: CEG supersedes FSD-002

The federation wire format for the CIRIS 3.0 agent line is **CEG (CIRIS Epistemic Grammar)**, maintained at `CIRISRegistry/FSD/CEG/`. The grammar is locked at **exactly five wire-format primitives** — one workhorse (`scores`) plus four structural composers (`delegates_to` / `supersedes` / `withdraws` / `recants`) — over an open, mechanism-descriptive dimension namespace. The lockdown has held across the entire 0.x line; every increment has been composition over already-locked primitives.

Two consequences matter for ASI readiness:

1. **The CEG-native turn**: the 3.0 agent does not *map* its internal state onto the wire format — its internal state mutations **are** self-level CEG attestations under the agent's own key, promoted to federation visibility per the tier model. The evidence an operator audits is the same envelope the federation reads. This is the implemented form of Book IX's requirement that constraint surfaces be externally referenceable rather than self-reported.
2. **Four implementations gate CEG 1.0**: CIRISAgent (runtime), CIRISNodeCore (consensus), CIRISLensCore (detection — the external witness the agent never self-emits, the anti-Goodhart separation), and CIRISRegistry (authority). No single party — including the agent — can declare the verification fabric complete.

### 1.4 RC-requirement status at 1.3-RC2

| RC requirement | Status | Evidence / outstanding |
|---|---|---|
| 1. Annex F–I operationalization | **Text operationalized at 1.3-RC2; live-cycle validation outstanding** | Annexes F–I carry full procedures, thresholds, and validation mechanisms (completed from the May 2026 *Magnifica Humanitas*-grounded drafts); they have not yet been exercised against a live deployment cycle |
| 2. Book IX mathematical validation | **Partially satisfied** | Collapse dynamics formalized in Lean 4 (CCA preprint v3, DOI 10.5281/zenodo.18217688); corridor empirics provide retrospective cross-substrate fits. Outstanding: one pre-registered out-of-sample prediction in a new substrate; external adversarial review independent of the author |
| 3. Threshold justification | **Partially satisfied** | OMV ratio retired (restated as side-constraint, Book II). Still provisional: 10²⁶ FLOP CRE trigger, 5% sentience heuristic, < 20% Echo Density target |
| 4. Red-team exercise | **Not yet performed** | The 1.3 σ-attestation requirement (Book IX §5.2) preemptively closes the first known attack vector (σ-pumping via free gratitude signals); a full adversarial cycle against all checkpoints remains owed |

### 1.5 The conscience layer as the ASI-critical control

The Annex D CRE protocol governs the *threshold crossing*; the conscience layer governs *every action before and after it*. As implemented (agent 2.9.6): every thought passes a priority-ordered stack of checks — four LLM-judged faculties (entropy, coherence, optimization-veto, epistemic humility) plus a deterministic safety floor (thought-depth guardrail, action-sequence check) that runs even in benchmark mode and even for otherwise-exempt actions. Thresholds are code-level constants, not learned weights — the architectural invariant that ethics cannot be normalized away by repeated exposure. Multiple independent stop surfaces exist: optimization-veto fail-safe-to-abort, the shutdown service, and a cryptographically signed remote Accord Invocation triggering full prohibition lockdown. Every check writes a tagged trace row, so an auditor can replay the full per-thought reasoning chain.

Known limitation, stated rather than hidden: the conscience refactor (3 shards + deterministic gate) is partially landed; the typed federation emission of conscience verdicts awaits the substrate Contribution envelope. Per-dimension detail: `compliance/D12_conscience.md`.

### 1.6 Substrate dependency trajectory

ASI-grade verification requires the full federation fabric. The substrate trajectory is **Persist → Edge → LensCore → NodeCore**; roughly a third of the 27 dimensions are gated on LensCore's external detector family (the witness the agent must never supply for itself). Until those substrates ship, the corresponding dimension claims remain *implementation-side ready, federation-side pending* — and are flagged as such per dimension.

### 1.7 What this addendum does and does not change

* It does **not** upgrade the Scope's ASI claim — that remains conditional on all four RC requirements.
* It does **not** replace Annex D — the CRE protocol stands unmodified as the threshold-crossing gate.
* It **does** make the evidence chain auditable end-to-end for the first time: regulatory paragraph → dimension (D01–D27) → implementation `file:line` → dated baseline → (as substrates land) external federation witness.
* It **does** commit the Accord to renewing this addendum at each version with the then-current agent release and baseline, so the readiness claim can never silently outdate.

---

*Prior addenda: none. This is the first.*
