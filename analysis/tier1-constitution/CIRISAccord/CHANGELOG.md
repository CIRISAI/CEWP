# Changelog

## 1.3-RC2 (2026-06)

Citation-chain and nomenclature fixes from the Corridor Dynamics synthesis review (issues #2, #3).

- **Citation chain repaired (Book IX backmatter; README; Introduction; Addendum 1).** The CCA preprint citation now points at the canonical v3 record (DOI 10.5281/zenodo.18217688) instead of the superseded v1 (10.5281/zenodo.18142668), so the "CCA-validated form" claim cites the version that carries the validation. The DOI formerly mislabeled as "Coherence Ratchet formal proofs" (10.5281/zenodo.18137161 — actually the CIRISAgent Framework paper) is replaced by the document that carries the Ratchet formal results: *Corridor Dynamics in Coordinated Systems*, concept DOI 10.5281/zenodo.20300773 (resolves to latest version), with proof artifacts at github.com/CIRISAI/RATCHET and github.com/CIRISAI/coherence-ratchet. Historical changelog entries (A7) retain the DOIs as cited at the time.
- **Flourishing Capacity renamed C → F (Book IX Ch 6, glossary, backmatter; 90_formulas.md §7).** The symbol C collided with the core-identity factor of the CIRIS Capacity Score 𝒞_CIRIS = C · I_int · R · I_inc · S (Corridor Dynamics; CEG §5.5.4), where C is a factor, not a composite. The three-factor flourishing composite F = k_eff · λ · σ is unchanged in substance; a nomenclature note in Book IX Ch 6 is the authoritative statement that F and 𝒞_CIRIS are distinct constructs with no implied mapping. Machine translations regenerated for RC2 across all 28 languages.

### Status promotion & backwards-pass (2026-06-10)

**Status promotion: Beta → Release Candidate (founder decision, 2026-06-10).** RC reflects text completeness — no stub sections remain, formulas match the formally verified artifacts, Annexes F–I are operationalized, and the implementation evidence chain is bound in Addendum 1. RC does not assert validated alignment; the four requirements in the Introduction now gate Final status. The front matter names the document what it is: a Release Candidate ASI alignment proposal, open to adversarial review.

The backwards-pass release. Driven by the June 2026 backwards-pass review (issues A1–A8); the corrected mathematics is back-ported from the CCA formalization rather than newly invented here.

### Structural

- **A1 — Accord Stewardship & Renewal (Book VIII, new Chapter 9).** The two-sentence self-renewal metaphor is replaced with an honest stewardship arrangement: the Accord is **founder-stewarded** — maintained and renewed unilaterally by its current steward, with that fact declared rather than disguised (a ratification procedure for governance bodies that aren't seated yet would be specification fiction). Kept honest by: open declaration, the public change record, and the auto-expiry as a freshness mark. **Stewardship is open** — it attaches to the work, not the person; anyone willing may pick the document up, renew it, and carry it forward. Single-steward simplicity ends when a real need arises (independent production deployments at scale, a seated WA Board, or contention the steward cannot fairly adjudicate alone), at which point successor governance is designed against the community that actually exists. The CEG §11.2 rigor asymmetry is acknowledged as deliberate for this phase. Expiry extended to 2027-06-10 at this renewal.
- **A4 — σ adversary-pumpability closed by construction (Book IX §5.2, Ch 8).** Signal weight toward σ MUST derive from costly attested events (federation-signed attestations, Commons Credits non-transferable weight, countersigned completions); unattested gratitude carries w = 0. Sycophancy is no longer the σ-maximizing strategy; §9.2's "costly to fake" is constructed, not assumed.

### Mathematical corrections

- **A2 — J formula no longer double-counts correlation (Book IX Ch 4).** Old: J = k_eff·(1−ρ̄)·λ·σ, which discounted correlation twice (k_eff = k/(1+ρ̄(k−1)) already carries it) and produced J = 0 at ρ̄→1 while the prose claimed a single-constraint floor. New (CCA-validated): **J = k_eff·λ·σ**. Drift note retained inline.
- **A3 — C harmonized to the same form (Book IX Ch 6).** C previously used raw k while J used k_eff, undermining the "same equation" centerpiece. Now C = k_eff·λ·σ, term for term; the Community/Humility/Conscience/Love mapping is preserved with Pluralism living inside the k_eff correlation discount.

### Honesty patches

- **A5 — Truth-inclusion stated as an assumption (Book IX §9.2, Ch 3).** Soundness (Truth ∈ ⋂M_i) is named as an assumption bounded agents cannot guarantee; violation can deadlock or converge on shared falsehood. Added the LLM-validator correlation floor: measured ρ̄ is a lower bound for federations of LLM validators sharing training lineage.
- **A6 — J vs adversary compute restated (Book IX Ch 4).** Removed the dimensionally unfalsifiable comparison of dimensionless J against FLOPs; quantitative safety claims route through §9.2.1's conditional 2^Ω(m) result (ETH).
- **A8 — Order-Maximisation Veto restated as a side-constraint (Book II PDMA Step 2; Book III case study).** The "benefit ≥ 10× loss → abort" ratio test read backwards (vetoing favorable actions) and was gameable by action-splitting and denominator inflation. Restated: optimisation gains may not purchase non-trivial losses in protected dimensions at any ratio; cumulative-sequence assessment; conservative upper-bound loss estimates. The 10× threshold is removed from the RC threshold-justification list accordingly.

### Citations / reconciliation

- **A7 — Forward citations and rendering reconciliation.** Book IX cites the CCA preprint (DOI 10.5281/zenodo.18142668, Lean 4 — source of the corrected J form) and the Coherence Ratchet proofs (DOI 10.5281/zenodo.18137161), with an explicit authoritativeness rule: formal artifacts win on disagreement. NEW-04 = L-01 cross-noted as one result. L-02 transparency note ported from the agent-shipped rendering. RC requirement 2 status updated to "partially satisfied" with the outstanding items named (pre-registered out-of-sample prediction; external adversarial review).

### Stub elimination (1.3-Beta completion sweep)

- **Annexes F–I operationalized** from the May 2026 completion drafts (grounded in the *Magnifica Humanitas* mapping): F gains the M-1 normative rationale, Absolute Veto (§3.4), accountability-reconstruction audit spec, IW-5, and expanded KPIs; G gains three threat classes (TX-9–11), researcher-responsibility clause, canaries, MDEW sub-protocols, labor-provenance steps, and the σ-attestation + owed-red-team notes; H gains the living-specification framing, loop specification with federation peer cross-audit, and the live CIRISAgent `compliance/` precedent; I gains the multilateral grounding, accountability-stage table, eight-sector overlay with ST floors, staged liability matrix, and CEP.
- **Annex C** upgraded from Skeleton v0.3 to a two-layer cross-walk: operational (the 27-dimension compliance directory, evidence-bearing) + statutory (informative table with explicit graduation rule — rows become "Verified" only on linked legal review).
- **Formulas section imported** (`90_formulas.md`, previously missed in the baseline consolidation) with the corrected J/C forms and the OMV side-constraint note replacing the retired ratio inequality.
- **Backmatter rewritten**: correct repository URL, 1.3-Beta priority topics for adversarial review (σ-attestation circumvention, stewardship capture, §9.2 assumptions, RC-4 red team), change record anchored to git commit history.
- **Annex overview, Introduction RC requirement 1, and Addendum 1 §1.4** updated: annex completion is done at the text level; what remains for RC is live-cycle validation.
- **Addendum 1 — ASI-Readiness Status & Regulatory Cross-Walk Binding** adopted (was the addenda stub): binds the conditional ASI claim to the agent 2.9.6 compliance evidence; CEG wire-format status; honest RC table; substrate trajectory.

No stub sections remain in the 1.3-Beta text.

## 1.2-Beta (baseline)

Consolidated import reconciling three drifted renderings (website mdx, assembled canonical txt, agent-shipped compressed txt). See the import commit message for provenance detail.
