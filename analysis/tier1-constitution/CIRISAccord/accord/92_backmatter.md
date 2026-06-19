# Backmatter

> Call for adversarial review and the change record.

---

## Call for Adversarial Review

We invite safety labs, independent researchers, and civil-society organisations to stress-test CIRIS 1.3-RC2.

Submit issues at **https://github.com/CIRISAI/CIRISAccord/issues**.

Priority topics:
* metric-Goodhart scenarios (including σ-attestation circumvention — see Book IX §5.2)
* board-capture pathways and the Book VIII Ch 9 stewardship arrangement
* escalation failures across the Annex F incident workflows
* the Book IX assumptions named in §9.2 (truth-inclusion, LLM correlation floor, non-adaptive adversary)
* the red-team exercise owed under RC requirement 4 (a simulated misaligned optimizer attempting to pass all CIRIS checkpoints)

Bounties are available for validated critical findings (Bug-Bounty Ledger, Book I Ch 2).

---

## Change Record

The authoritative, cryptographically-hashed history of this document is the git commit history of **https://github.com/CIRISAI/CIRISAccord** (every revision is content-addressed by commit SHA), summarized in [CHANGELOG.md](../CHANGELOG.md).

Version summary:

* **2025-04-16 — 1.0-β** initial release; risk-limited; 24-month sunset.
* **1.2-Beta** — working-draft line carrying the full Book IX geometric framework; consolidated from three drifted renderings into the canonical repository (import commit `000ba57`).
* **2026-06-10 — 1.3-RC2** — backwards-pass release: J/C formula correction (CCA-validated form), σ attestation requirement, truth-inclusion assumption, OMV side-constraint restatement, forward citations to formal artifacts, Book VIII Ch 9 stewardship & renewal, Addendum 1 (ASI-readiness binding), Annexes F–I operationalized, expiry extended to 2027-06-10. License: AGPL-3.0.

---

End of Specification
