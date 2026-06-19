# Annex C

> Regulatory Cross-Walk

---
ANNEX C REGULATORY CROSS‑WALK   (v 1.3-RC2)
---

### Purpose

Map CIRIS clauses to major external standards to simplify dual compliance. This annex carries two layers:

1. **The operational cross-walk (live, evidence-bearing)** — the CIRISAgent `compliance/` directory: 27 stable dimensions (D01–D27) cross-walked at paragraph grain against four institutionally-distinct senior frameworks (*Magnifica Humanitas* · EU HLEG Trustworthy AI Guidelines · IEEE Ethically Aligned Design · ASEAN AI Governance Guide), with per-dimension implementation references and dated, script-generated baselines. See **Addendum 1** for the binding and the evidence discipline.
2. **The statutory mapping (informative)** — the table below, mapping CIRIS structures to binding-law and standards frameworks. These rows are *informative engineering correspondences*, not legal opinions; statutory rows graduate to "verified" status only upon legal review. Annex I carries the operative legal-alignment procedures (data-protection mapping, sector overlays, liability matrix, escalation feeds).

### Statutory and standards mapping (informative; pending legal verification)

| External Framework | Relevant Articles / Clauses | CIRIS Mapping (Book / Annex §) | Status |
|---|---|---|---|
| EU AI Act (2024) | Art 9 Risk Management | Book II §II (PDMA); Annex D CRE | Informative |
| | Art 12 Record-Keeping | Book IV Ch 3 audit trails; Annex F §4 | Informative |
| | Art 13 Transparency | Book II §II Step 6; Book IV Ch 3 | Informative |
| | Art 14 Human Oversight (incl. 14(4)) | Book II §III (WBD); Annex F authority lattice + autonomy tiers | Informative |
| | Art 61 Post-Market Monitoring | Annex H drift controls + continuous audit | Informative |
| NIST AI RMF 1.0 | Govern → Map → Measure → Manage | Govern: Books I, VI; Map: Book II §II Steps 1–2; Measure: Annex A metrics + Annex H baselines; Manage: Annex F/H workflows | Informative |
| ISO/IEC 42001 | Cl 6.2 Risk Assessment | Book II §II | Informative |
| | Annex A controls | CIRISAgent `compliance/` dimension docs (per-control correspondence pending) | Informative |
| OSHA Robotics Guidelines | Sec 5.E Safety Audits | Annex D CRE | Partial |
| EU HLEG Trustworthy AI (2019) | 7 Requirements | Operational cross-walk, dimensions D01–D27 | Evidence-bearing |
| IEEE EAD 1st ed. (2019) | 8 General Principles | Operational cross-walk, dimensions D01–D27 | Evidence-bearing |
| ASEAN AI Governance Guide (2024) | 7 Principles; HITL/HOTL/HOOTL | Operational cross-walk; Annex F autonomy tiers | Evidence-bearing |
| *Magnifica Humanitas* (2026) | 245 ¶¶ (paragraph-grain mapping) | Operational cross-walk, dimensions D01–D27 | Evidence-bearing |

### Graduation rule

A statutory row moves from *Informative* to *Verified* when qualified legal review confirms the correspondence for a named jurisdiction and the review artifact is linked from this table. Until then, rows in this annex support engineering alignment and gap analysis, not compliance claims; per the Introduction's Liability section, compliance claims are void where prohibited by applicable law.

(Additional frameworks — UNESCO Recommendation on AI Ethics, OECD AI Principles, Council of Europe Framework Convention — are queued for the operational cross-walk per the compliance directory roadmap.)

---
End of Annex C
