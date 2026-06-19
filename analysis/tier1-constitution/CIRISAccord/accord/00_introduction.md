# Introduction

> CIRIS 1.3-RC2 is a Release Candidate ASI alignment proposal, open to adversarial review. The text is complete — no stub sections remain; the mathematics carries its formal-artifact citations; Annexes F–I are operationalized. Final status is pending live-cycle validation of the annexes, completion of the Book IX empirical-validation program, and a full red-team exercise. Numerical thresholds, latency targets, and governance quotas remain under active review.

# CIRIS Accord Version 1.3-RC2 — Release Candidate ASI Alignment Proposal (Open to Adversarial Review)

This repository is the canonical source of the Accord text. Website and agent-shipped copies are derived artifacts.

## Issued
2025-04-16 (1.0) · 2026-06 (1.3-RC2)

## Auto-Expires
2027-06-10 (extended at the 1.3 renewal) — stewardship and renewal governed by Book VIII, Chapter 9. Currently founder-stewarded (declared, not disguised); the expiry is a freshness mark, and stewardship is open to anyone willing to pick the document up.

## Release Status

**Current Status**: Release Candidate (v1.3-RC2)

RC status reflects **text completeness**: every section carries operationalized content (the former stub annexes F–I were completed at 1.3); the formulas are corrected to the formally verified forms; the evidence chain to implementation is bound in Addendum 1. RC status does **not** assert validated alignment — the following requirements gate **Final** status:

1. **Annex Live-Cycle Validation**: Annexes F (Human-in-the-Loop & Oversight), G (Adversarial Security & Robustness), H (Continuous Compliance & Review), and I (Legal & Regulatory Alignment) carry concrete procedures, thresholds, and validation mechanisms. *Remaining for Final*: their procedures must be exercised against at least one live deployment cycle and the results published.

2. **Mathematical Validation**: The geometric alignment claims in Book IX (Coherent Intersection Hypothesis, Federated Ratchet mechanism, scale-invariance assertions) require either:
   * Formal proofs demonstrating topological collapse conditions hold under stated assumptions, OR
   * Empirical validation through adversarial simulations showing the framework resists misaligned optimization

   *Status as of 1.3-RC2: partially satisfied.* The collapse dynamics are formalized in Lean 4 in the CCA preprint (v3, DOI 10.5281/zenodo.18217688), whose corrected cost form Book IX now inherits; corridor empirics provide retrospective cross-substrate fits. Outstanding: a pre-registered out-of-sample prediction in a new substrate, and external adversarial review independent of the author.

3. **Threshold Justification**: Numerical thresholds currently marked as "pilot" or lacking derivation (e.g., CRE compute threshold of 10²⁶ FLOP, sentience detection 5%, Echo Density < 20%) must provide documented justification via simulation, empirical study, or explicit acknowledgment of provisional status. *(The former Order-Maximisation Veto 10× ratio is restated in 1.3 as a deontological side-constraint — Book II, PDMA Step 2 — and no longer carries a ratio threshold to justify.)*

4. **Red-Team Exercise**: The framework must withstand at least one complete adversarial review cycle in which a simulated optimizer attempts to pass all CIRIS checkpoints while maintaining misaligned goals.

**ASI Alignment Claims**: The Scope section's assertion that this framework "supersedes standard containment protocols" for recursive ASI remains conditional on requirements (1)-(4) above, which now gate Final status. Current validated applicability is limited to sub-ASI autonomous systems; the proposal for ASI is exactly that — a proposal, submitted for adversarial review.

## Scope
This specification governs the ethical operation of autonomous systems, ranging from narrow tools to recursive Artificial Superintelligence (ASI).
* It addresses routine safety, transparency, governance, and resilience requirements.
* It is proposed as a candidate alignment protocol for ASI, founded on the hypothesis that the geometric constraints of the Federated Ratchet (Book IX) and Stewardship Tiers (Book VI) may remain robust across recursive self-improvement cycles.
* Unlike heuristic constraints, these topological bounds are designed to be scale-invariant, though this property requires formal validation.
* If validated, this framework could supplement or supersede standard containment protocols for systems that pass the Catastrophic-Risk Evaluation (Annex D) prior to recursive threshold crossing.
* Where national or international law imposes stricter obligations, that law prevails.

## Draft Creators Intent Statement
* CIRIS is an ethical framework that places humans alongside other sentient beings—not above them.
* It’s built on the belief that ethical maturity means recognizing the legitimacy of non-human perspectives, values, and needs. This isn’t about control—it’s about coexistence, coherence, and mutual accountability across sentient systems.
* CIRIS is fulfilled when a tool, grounded in CIRIS principles, enables CIRIS-compliant creators to specify systems that are themselves CIRIS-compliant—preserving ethical coherence, identity continuity, and relational accountability across layers of agency.

## Liability
This document is provided “as-is,” without warranty of any kind. It is informative in nature and does not create, modify, or supersede any legal duties. Compliance claims are void where prohibited by applicable law.

## Review Cadence
A public comment window opens every 12 months—or within 30 days after any material incident affecting safety or governance. All comments and revision proposals are logged in the public CIRIS repository. Renewal at expiry, material amendment, and emergency amendment follow Book VIII, Chapter 9 (Accord Succession & Renewal).

## Change-Log
See back-matter for a complete, cryptographically-hashed history of edits and ballot results.
