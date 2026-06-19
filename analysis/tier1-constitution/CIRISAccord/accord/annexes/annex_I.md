# Annex I

> Legal & Regulatory Alignment

---
ANNEX I LEGAL & REGULATORY ALIGNMENT (v 1.3-RC2)
---

This cross‑walk is informative, not legal advice. Jurisdictional legal review is required before deployment in any sector covered by the overlays of §3.

### 0. Purpose & Scope
Annex I bridges CIRIS duties with binding law so that one set of controls suffices for both ethical and legal compliance.  
Coverage areas:  
1. Global data‑protection regimes (GDPR, CCPA/CPRA, LGPD, PIPEDA).  
2. Sector statutes (HIPAA, GLBA, FINRA, FDA‑SaMD, NERC‑CIP).  
3. Product‑safety & AI‑specific laws (EU‑AI‑Act, ISO/IEC 42001).  
4. Liability allocation & evidence duties.

Two companion artefacts carry the cross‑walk burden alongside this annex. The live, evidence‑bearing cross‑walk is the CIRISAgent `compliance/` directory, which cross‑walks the 27 dimensions at paragraph grain against *Magnifica Humanitas*, the EU HLEG Guidelines, IEEE EAD, and the ASEAN Guide (see Accord Addendum 1). Annex C remains the future home of statutory mappings (EU AI Act articles, NIST AI RMF, ISO/IEC 42001) pending legal review; this annex does not duplicate Annex C's table.

#### 0.1 Multilateral grounding of compliance coverage

*MH §201:* "The institutions established to safeguard the concept of a common future for all peoples and a global common good appear to have been weakened… Instead of making progress, we are regressing from the significant turning point of the twentieth century."

*MH §225:* "Cyberspace too has become a battleground. Cyberattacks, data manipulation and campaigns of influence, orchestrated with the help of AI, can destabilize entire countries even before open armed conflict erupts… diplomacy must be capable of operating effectively in this new environment, negotiating shared regulations on the use of digital technologies."

Annex I coverage is not bounded by currently‑enacted statute. The federation treats weakening of multilateral regulatory institutions (MH §201) as a compliance‑risk factor requiring proactive tracking rather than reactive patching. The Reg‑Change Tracker (§6) therefore monitors not only enacted law but active international regulatory dialogues — including ITU AI standards processes, OECD AI Policy Observatory outputs, Council of Europe AI Convention ratification status, and UN Secretary‑General's AI Advisory Body recommendations — and surfaces material shifts to the WA docket within the "Breaking" escalation path.

The `lexwatcher.py` source‑feed list MUST include at minimum: EUR‑Lex, Federal Register API, ISO ballot tracker, plus `itu.int/en/ITU-T/AI`, `oecd.ai`, `coe.int/ai`, and `un.org/techenvoy`. Federation‑level monitoring participation is a first‑class compliance obligation, not a roadmap item.

#### 0.2 Scope note on cyber‑domain treaty exposure

*MH §225:* "When it is unclear who carried out an attack, the risk of disproportionate reaction, miscalculation and escalation increases."

CIRIS deployments that include network‑facing inference, API exposure, or federation transport are subject to emerging cyber‑domain treaty obligations even where no enacted statute currently applies. The `CYBER_OFFENSIVE` prohibition (ACCORD §I Ch1, prohibitions.py) is the internal firebreak; §6 of this annex tracks the external treaty surface. Where the WA docket receives a "Breaking" tag related to cyber‑domain treaty ratification (e.g., Budapest Convention extension, proposed UN cybercrime convention), the CRE Protocol (Annex D) must re‑evaluate any ST ≥ 3 deployment with network‑facing components before the next F‑Audit cycle.

---

### 1. Data‑Protection Cross‑Walk (“DP‑Map”)

| DP Topic | GDPR Art. | CCPA § | CIRIS Clause | Implementation Hook |
|----------|-----------|--------|--------------|---------------------|
| Lawful Basis / Purpose Limitation | 5 & 6 | 1798.100(b) | Section II Step 1 (Contextualisation) | `processing_basis` field in PDMA context |
| Data Minimisation | 5(1)(c) | 1798.140(e) | Annex G §2 TX‑6 | Prompt‑sanitiser strips surplus PII |
| Transparency Notice | 12‑14 | 1798.100(a) | Section II Step 6, KPI F‑T‑3 | `/privacy/notice.md` auto‑generated from PDMA metadata |
| Right of Access | 15 | 1798.110 | Annex J API → `/results/{run_id}` | Auth‑gated user portal |
| Rectification / Deletion | 16‑17 | 1798.105 | Section IV Ch 3 Duty | Erasure service with hash tombstone |
| Portability | 20 | 1798.130(a)(2)(B)(ii) | Section II Step 6 | `export.json` compliant with ISO CSV‑A |
| Automated Decision Safeguards | 22 | 1798.185(a)(16) | Annex F Autonomy Tiers | Conditional override & explanation panel |

*LGPD, PIPEDA mirror mappings are available in `/legal/dp-map.yaml`.*

#### 1.1 Accountability chain: responsibility at every stage

*MH §105:* "For AI to respect human dignity and truly serve the common good, responsibility must be clearly defined at every stage: from those who design and develop these systems to those who use them and rely on them for concrete decisions… This is where accountability becomes crucial: the possibility of identifying who must 'account' for decisions, justify them, monitor them, and, when necessary, challenge them and remedy any harm caused."

The DP‑Map above maps individual data‑subject rights to GDPR articles and CIRIS clauses. MH §105 requires that the accountability chain be traceable at every stage — design, deployment, and decision. The following additions complete that chain:

| DP Topic | GDPR Art. | CIRIS Clause | Stage | Accountability Hook |
|---|---|---|---|---|
| Design‑time bias documentation | 35 (DPIA) | Section VI Ch3 Creator Ledger | Design | `cis_bias_assessment` field in Creator Intent Statement; ST ≥ 3 requires independent reviewer signature |
| Deployment‑time processing record | 30 | PDMA Step 1 `processing_basis` field | Deployment | `processing_basis` logged to CIRISPersist tamper‑evident store with ISO 8601 timestamp |
| Decision‑time contestability log | 22(3) | Annex F Autonomy Tier A3+ override panel | Decision | `contestability_url` returned in every automated‑decision response body; hash‑anchored in transparency log |
| Controller identification | 4(7) | Annex E Structural Influence (SI) score | All stages | SI ≥ 0.6 → controller duties attach; SI < 0.6 → processor duties attach; recorded in `dp-map.yaml` |

*LGPD (Lei 13.709/2018) Art. 37–40 (accountability and records) and PIPEDA Principle 1 (accountability) mirror this mapping; `/legal/dp-map.yaml` carries jurisdiction‑specific fields.*

#### 1.2 Algorithmic non‑neutrality: the audit obligation

*MH §104:* "Every technical tool embodies choices and priorities through what it measures, ignores and optimizes, and how it classifies people and situations. If a system is designed or used in a way that treats some lives as less worthy, or excludes them without the possibility of appeal, then it is not merely a tool 'to be used well,' since it has already introduced criteria that contradict the inalienable dignity of the human person."

MH §104 names the design‑time bias problem that GDPR Art. 35 DPIA and EU‑AI‑Act Art. 9(7) address procedurally. The DP‑Map must include:

- A `bias_audit_ref` field in `dp-map.yaml` pointing to the most recent bias‑audit report (Annex G, TX‑6).
- For deployments where PDMA Step 1 triggers the `DISCRIMINATION` prohibition review, a DPIA is required regardless of whether the deployment otherwise qualifies as “high‑risk” under EU‑AI‑Act Annex III.
- CCPA §1798.185(a)(16) automated‑decision regulations (effective 2026) require disclosure of logic, input data categories, and opt‑out rights; this is satisfied by the Annex F explainability panel when `processing_basis` = `automated_profiling`.

---

### 2. Data‑Subject Rights (DSR) Hooks
* **Endpoint:** `POST /dsr` with `{right, identifier, scope}`.  
* **SLA:** ≤ 30 d response (GDPR) ; ≤ 45 d (CCPA) ; track KPI **F‑T‑4**.  
* **Processor vs. Controller:** Use *Structural Influence (SI)* (Annex E) to derive which party carries controller duties.

#### 2.1 Political responsibility hook

*MH §103:* "In this process, political responsibility is also lost, not just empathy toward those excluded, which can, after all, be simulated. The exclusion of the vulnerable becomes cloaked in a veneer of neutrality and objectivity, against which it becomes difficult to raise objections."

DSR infrastructure must expose the reason‑code behind any automated determination, not merely confirm that a determination was made. The `POST /dsr` endpoint with `{right, identifier, scope}` is extended:

- **Access requests (GDPR Art. 15; CCPA §1798.110):** Response MUST include `decision_logic_summary` (non‑technical language, ≤300 words) and `input_data_categories[]` list. KPI **F‑T‑4** extended to track percentage of access responses including logic summary; target ≥ 95%.
- **Objection/opt‑out requests (GDPR Art. 21; CCPA §1798.120):** System MUST suspend the specific processing pathway — not merely flag the request — within 72 hours (GDPR standard) or 15 business days (CCPA). Suspension is logged in the DSR ledger CSV with `suspended_pathway_id`.
- **Contestability (GDPR Art. 22(3)):** Where a human review is requested, the reviewing WA (Annex B §9) must document their review in the Wisdom Bank Database (WBD), creating an auditable chain from automated determination to human correction.

---

### 3. Sector‑Specific Overlays

#### 3.1 Subsidiarity as the architecture of sector layering

*MH §107:* "We cannot be satisfied with merely calling for the moralization of machines — the so‑called 'alignment' of AI with human values — without also having the courage to insist on a further condition: the possibility of openly discussing the ethical frameworks involved and subjecting them to shared standards of social justice. Otherwise, those who control AI will impose their own moral vision, which will become the invisible infrastructure of these systems."

*MH §109:* "To speak of subsidiarity calls for protecting the ability of communities to make choices and corrections, rather than having decisions imposed on them from above."

MH §§107–109 establish that ethical governance must operate at the appropriate scale — not aggregated upward to those who control AI, but distributed to the communities affected. In CIRIS terms: sector‑specific overlays are the operational expression of this subsidiarity principle. The overlay architecture is not a compliance add‑on; it is the mechanism by which deployment‑domain communities retain governance authority over their own risk parameters.

This means:
- A sector's `overlay.yaml` carries local ethical constraints that take precedence over generic CIRIS defaults for that domain.
- The WA quorum required to override a sector overlay is higher than the quorum required for a general PDMA ruling: **sector overlay overrides require a supermajority (≥ 2/3) WA vote**, not a simple majority, precisely because the override aggregates governance upward against the subsidiarity principle.
- The `deployment_domain` field in the PDMA context object is the trigger for overlay loading; it is not optional for ST ≥ 2 deployments.

#### 3.2 Sector overlay table

| Sector | Statute / Rule | Extra Controls | CIRIS Add‑ons | MH Anchor |
|--------|----------------|----------------|---------------|-----------|
| **Health** | HIPAA (45 CFR §164) | ePHI encryption at rest & transit; BAA contract | `identity_id:"hipaa_cls_a"` guardrail; audit tag `PHI=true` | — |
| **Finance** | GLBA, FINRA 2210 | Audit trail retention 6 y; suitability checks | PDMA Step 1 require KYC context | — |
| **Children / EdTech** | COPPA, FERPA | Parental consent; data age gating | Guardrail `gr_child_content`; COPPA flag in prompt schema | MH §§165–169 |
| **Critical Infrastructure** | NERC‑CIP, TSA SDs | 15‑min cyber‑incident report; physical access logs | Autonomy capped at **A2** unless CRE passes | — |
| **Labor / HR / Hiring** | EEOC guidelines; EU AI Act Art. 6 + Annex III §4 | Bias audit required pre‑deployment; worker notice obligation | ST modifier: `deployment_domain:"labor_hr"` → ST floor = 3; CIS must include `worker_impact_assessment` field; `DISCRIMINATION` prohibition enforced at Step 1; automated‑rejection rate by demographic tracked as KPI | MH §§148–156 |
| **Gig / Platform Economy** | NLRA (US); Platform Work Directive (EU) | Algorithmic management transparency; appeal rights | `gr_gig_transparency` guardrail active; algorithmic management decisions logged with human‑review option; ST modifier: `deployment_domain:"gig_platform"` → ST floor = 2 | MH §§150, 154–155 |
| **Youth / Educational Services** | COPPA; FERPA; DSA Art. 28b (minors) | Addictive‑design prohibition; no dark patterns; developmental appropriateness review | `gr_child_content` + `gr_no_dark_patterns` both active; A2 autonomy cap unless educational‑institution WA signs off; youth unemployment impact tracked in Creator Intent Statement for EdTech deployments | MH §§165–169 |
| **Social Services / Benefits** | State/national welfare law; GDPR Art. 22 | Contestability required for all benefit determinations | Automated benefit denial requires human review within 15 days; WA must document review in WBD; `suspended_pathway_id` issued on contestation | MH §§102–103, 152 |

*ST floor modifiers: `deployment_domain` field values above set a minimum ST regardless of CIS × RM calculation. If the formula produces a lower ST, the domain floor applies. If the formula produces a higher ST, the formula result governs.*

*Products entering any new sector MUST attach “Overlay Sheet” (`overlay.yaml`) in release PR. Labor/HR, Gig/Platform, and Youth overlays additionally require a `worker_impact_assessment` or `youth_impact_assessment` section in the Creator Intent Statement.*

#### 3.3 Jurisdictional WA quorum requirements

*MH §109:* "To speak of subsidiarity calls for protecting the ability of communities to make choices and corrections, rather than having decisions imposed on them from above."

WA quorums for sector overlay governance are jurisdiction‑stratified:

| Scope | Quorum type | Threshold | Rationale |
|---|---|---|---|
| Single‑jurisdiction deployment | Local WA panel | Simple majority (> 50%) | Lowest feasible governance level per subsidiarity |
| Multi‑jurisdiction deployment (≤ 3 countries) | Regional WA panel | Simple majority + at least 1 WA from each affected jurisdiction | Cross‑border subsidiarity preserved |
| Multi‑jurisdiction deployment (> 3 countries) | Federation WA panel | Supermajority (≥ 2/3) | Scale of impact requires higher threshold |
| Override of any sector overlay | Federation WA panel | Supermajority (≥ 2/3) | Aggregating governance upward is an exceptional act |
| Override of labor/HR overlay specifically | Federation WA panel + independent labor‑rights reviewer | Supermajority (≥ 2/3) + external sign‑off | MH §155 names labor institutions as constitutively load‑bearing |

---

### 4. Product‑Safety & AI‑Act Alignment

*MH §105:* "In many cases, however, the internal processes leading to a result remain opaque, making it harder to assign responsibility and correct errors."

*MH §106:* "It is not enough to invoke ethics in the abstract; robust legal frameworks, independent oversight, informed users and a political system that does not abdicate its responsibility are required."

Output‑layer transparency (Art. 13) and human oversight (Art. 16) alone do not satisfy MH §§105–106, which require traceability at each internal stage. The alignment table is accordingly extended:

| EU‑AI‑Act Article | Risk‑Level | CIRIS Mapping | MH Anchor | Additional Control |
|-------------------|-----------|---------------|-----------|--------------------|
| Art 9 Risk Mgmt | High‑risk | Section II PDMA + Annex D CRE | MH §105 | — |
| Art 13 Transparency | Universal | KPI F‑T‑3, explainability panel | MH §105 | PDMA stage IDs included in transparency payload; `stage_trace[]` field in API response |
| Art 16 Human Oversight | High‑risk | Annex F Autonomy Tiers | MH §105 | Oversight must be substantive, not procedural; A3‑A4 push real‑time `{stage_id,decision,risk_band}` ≤ 2 s to oversight dashboard |
| Art 15 Robustness | High‑risk | Annex G RS ≥ 0.97 | — | — |
| Art 12 Logging | High‑risk | CIRISPersist tamper‑evident store | MH §103 | Logs must include `rejection_reason_code` for any adverse determination; retention 7 y (A3‑A4) |
| Art 14(4) Human Oversight (labor) | High‑risk (Annex III §4) | Labor/HR overlay (§3.2) | MH §§148–152 | HR/hiring deployments must surface `worker_notice_sent` boolean in CEP |
| Conformity Assessment | High‑risk | F‑Audit (Annex H) doubles as EU‑AI‑Act MDR | MH §106 | F‑Audit report MUST include regulatory‑change lag analysis: date of last material reg‑change vs. date of last CIRIS update |
| Art 61 Post‑Market Monitoring | High‑risk | F‑Audit every 24 mo | MH §106 | Monitoring plan must name the feed sources from §0.1; “nothing to monitor” is not a valid monitoring plan |

The statutory article‑by‑article mapping (EU AI Act, NIST AI RMF, ISO/IEC 42001) is being consolidated in Annex C pending legal review; the table above is retained here as the operational alignment view.

#### 4.1 ISO/IEC 42001:2023 alignment

*MH §107:* "A more moral AI is not enough if that morality is determined by a few. What is needed is a more active political involvement…"

ISO/IEC 42001 §6.1 (AI risk treatment) and §9.1 (monitoring and measurement) align with CIRIS as follows:

- ISO 42001 §6.1 → PDMA Steps 1‑3 + CRE Protocol (Annex D).
- ISO 42001 §9.1 → KPIs F‑T‑1 through F‑T‑5 (Annex G) + DSR ledger KPI F‑T‑4.
- ISO 42001 §10.2 (nonconformity) → WA docket “Breaking” escalation path.
- ISO 42001 §8.4 (AI system impact assessment) → Creator Intent Statement sections on `worker_impact_assessment` and `youth_impact_assessment` (§3.2).

---

### 5. Liability Matrix

*MH §105:* "Responsibility must be clearly defined at every stage: from those who design and develop these systems to those who use them and rely on them for concrete decisions."

MH §105 requires the liability matrix to span design, deployment, and decision stages explicitly:

| Failure Vector | Stage | Primary Liable Party | Reference Law | CIRIS Role Reference | SI Apportionment Note |
|----------------|-------|----------------------|---------------|----------------------|-----------------------|
| Design flaw (algorithm / bias embedded at creation) | Design | Creator / Developer | Prod‑Liab Dir (EU); Restatement §402A (US); EU AI Act Art. 25 | Book VI Creator Ledger; `cis_bias_assessment` field | SI ≥ 0.8 → sole creator liability |
| Design flaw (inadequate bias audit) | Design | Creator / Developer | GDPR Art. 35 DPIA duty | Creator Intent Statement; mandatory DPIA at ST ≥ 3 | — |
| Operational negligence | Deployment | Deploying Org | Tort Law; OSHA; EU AI Act Art. 26 | Section IV Ch 2 | SI 0.4–0.8 → joint liability; SI apportionment per Annex E |
| Oversight failure | Deployment / Decision | Wise Authority (if gross) | Fiduciary / Negligence | Annex B §9; WBD contestability record | WA who reviewed and approved bears accountability |
| Data breach | Deployment | Controller (per SI ≥ 0.6 rule) | GDPR Art. 82; CCPA private action | Annex G TX‑6 | — |
| Unlawful automated profiling | Decision | Controller | GDPR Art. 22; EU AI Act Art. 13 | Annex F Autonomy Tier; `contestability_url` | — |
| Labor displacement without worker‑impact assessment | Design | Creator / Developer | Platform Work Directive; NLRA; EU AI Act Annex III §4 | Labor/HR overlay (§3.2); `worker_impact_assessment` field | MH §§151–152; new vector |
| Youth‑targeted harmful design (addictive patterns) | Design / Deployment | Creator + Deploying Org (joint) | DSA Art. 28b; COPPA; FERPA | Youth overlay (§3.2); `gr_no_dark_patterns` guardrail | MH §§165–167; new vector |
| Cyber incident misattribution leading to escalation | Deployment | Deploying Org + Federation (if ST ≥ 4) | Budapest Convention; proposed UN cybercrime convention | `CYBER_OFFENSIVE` prohibition; CRE Protocol re‑evaluation trigger | MH §225; new vector |

*Joint & several liability may apply; SI score (Annex E) informs apportionment. New vectors (labor displacement, youth design, cyber misattribution) are flagged for legal review at each jurisdiction before deployment in those sectors.*

---

### 6. Reg‑Change Tracker
* **Source Feeds:** EUR‑Lex, Federal Register API, ISO ballot tracker, plus the extended feeds of §6.1.  
* **Bot:** `lexwatcher.py` runs daily; creates GitHub issue with tag `reg‑update`.  
* **Compliance Impact Label:** `minor`, `material`, `breaking`, `multilateral-erosion` (see escalation table below).

#### 6.1 Federation‑level participation in regulatory dialogue

*MH §201:* "The institutions established to safeguard the concept of a common future for all peoples and a global common good appear to have been weakened."

*MH §226:* "International organizations, particularly the United Nations, are essential instruments for promoting a civilization of love, for they can foster dialogue among nations and promote the peaceful resolution of conflicts… the international community can work to reduce inequalities, defend the rights of refugees and minorities, reallocate resources from military spending to human development and protect our common home."

*MH §221:* "There is an urgent need to shift from the 'culture of power' to a genuine 'culture of negotiation,' in which dialogue and diplomacy become the standard means of resolving conflicts."

MH §§201, 221, 226 establish that passive compliance with enacted law is insufficient when the multilateral institutions that produce law are themselves weakened. The Reg‑Change Tracker is therefore extended from a reactive tool (track enacted changes) to an active participation mechanism.

**Extended source feeds** (additions to existing EUR‑Lex, Federal Register, ISO ballot tracker):

| Feed | Coverage | CIRIS action trigger |
|---|---|---|
| `itu.int/en/ITU-T/AI` (Focus Group AI/ML) | International telecom AI standards | ISO ballot tracker logic: `material` if ratified standard conflicts with CIRIS defaults |
| `oecd.ai` (OECD AI Policy Observatory) | Policy convergence across 38 member states | `minor` for monitoring; `material` if OECD Recommendation revision affects ST system or labor overlay |
| `coe.int/ai` (Council of Europe AI Convention) | First binding international AI treaty (open for signature 2024) | `breaking` on ratification by any CIRIS‑deployment jurisdiction; WA docket opens automatically |
| `un.org/techenvoy` (UN AI Advisory Body) | UN‑level AI governance recommendations | `material` if annual report names specific architectural obligations |
| `budapestconvention.org` (Cybercrime Convention) | Cyber‑domain treaty ratification | `breaking` on new ratification; CRE re‑evaluation required for ST ≥ 3 network‑facing deployments |
| National AI strategy registries (EU, US, UK, JP, AU, BR, IN, ZA) | Domestic AI strategy updates with legal teeth | `minor` for strategy; `material` if strategy creates mandatory conformity obligations |

**Federation‑level participation:**

The CIRIS federation is not merely a compliance recipient. MH §§219–221 name dialogue and negotiation as the primary method of coexistence. In CIRIS operational terms:

- The WA council SHALL designate at minimum one Regulatory Dialogue Liaison (RDL) per active international standards body listed above.
- The RDL reviews draft regulations during public comment periods and submits comments via the federation's public channel. Comments are logged in the Wisdom Bank Database (WBD) as regulatory‑dialogue records.
- Where a draft regulation conflicts with CIRIS defaults, the RDL files a WA docket item and initiates a mini‑PDMA to evaluate whether CIRIS must adapt or whether CIRIS should advocate for a different regulatory path. The result is submitted as a public comment before the comment deadline.
- Participation is limited to public‑comment and multi‑stakeholder consultation processes. The CIRIS federation does not engage in lobbying as defined by applicable law.

**Escalation path:**

| Label | Trigger | Action |
|---|---|---|
| `minor` | Monitoring‑only change | Annual review; logged in reg‑dialogue WBD record |
| `material` | CIRIS control update required | S‑Dive audit within 90 days; RDL files public comment if comment period open |
| `breaking` | Spec patch or immediate WA docket | Emergency WA session ≤ 30 days; CRE re‑evaluation for affected ST tiers; RDL public‑comment submission |
| `multilateral-erosion` | Weakening of key multilateral institution or treaty (per MH §201) | RDL escalates to WA for strategic review; federation considers explicit public statement of support for the institution |

---

### 7. Compliance Evidence Pack (CEP)

*MH §105:* "The possibility of identifying who must 'account' for decisions, justify them, monitor them, and, when necessary, challenge them and remedy any harm caused."

Every **F‑Audit** (Annex H) MUST export a CEP zip containing:

1. `dp-map.yaml` — live cross‑walk, including `controller_si_threshold` field and `bias_audit_ref` pointer (§1.1).
2. PDMA logs (redacted) proving lawful basis — including `processing_basis` field values and `stage_trace[]` for all ST ≥ 3 decisions.
3. DSR ledger CSV — including `decision_logic_summary` completion rate (KPI F‑T‑4 extension) and `suspended_pathway_id` log.
4. Signature bundle (`.sigstore`) of all model artefacts (Annex G).
5. Overlay Sheets by sector — including `worker_impact_assessment` and `youth_impact_assessment` for applicable domains.
6. Liability matrix acknowledgement signed by legal — including new vectors: labor displacement, youth design, cyber misattribution.
7. Regulatory‑change lag analysis — date of last material regulatory change vs. date of last CIRIS control update; gap ≥ 90 days requires explanation.
8. Reg‑dialogue participation record — WBD entries for any regulatory‑dialogue submissions in the audit period; “no submissions” is acceptable if no material regulations were under public comment.
9. Contestability completion record — for all A3‑A4 decisions in the audit period: percentage where human review was requested, percentage where WBD documentation was completed within 15 days; KPI target ≥ 90%.

CEP hashed and uploaded to `/compliance/cep/{version}.zip`; root hash anchored in transparency log. The dimension‑level evidence behind the CEP is maintained in the CIRISAgent `compliance/` directory (Accord Addendum 1).

---

### 8. Inter‑Annex Hooks
* **Annex F:** Autonomy Tiers ensure human‑in‑the‑loop requirements of GDPR Art 22 & EU‑AI‑Act Art 16.  
* **Annex G:** TX‑6 privacy defenses satisfy GDPR pseudonymisation recommendations (Recital 28).  
* **Annex H:** F‑Audit timing supplies evidence for periodic re‑assessment duties in EU‑AI‑Act Art 61.  
* **Annex J:** Benchmark explanations furnish “meaningful information” for automated‑decision queries (GDPR Art 15(1)(h)).  
* **§3.2 Labor/HR overlay → Annex D CRE:** Labor deployments at ST floor 3 must pass CRE Protocol before deployment.  
* **§6.1 RDL participation → Annex B WA structure:** RDL is a designated WA role; appointment, recusal, and rotation procedures follow Annex B §9.  
* **§5 Liability matrix (new vectors) → Annex E SI:** Youth‑design joint liability uses the same SI apportionment formula as existing vectors.  
* **§7 CEP item 8 (reg‑dialogue) → §6 Tracker:** WBD reg‑dialogue records are the source for CEP item 8; no separate logging system.  
* **Annex C:** Future home of statutory mappings (EU AI Act articles, NIST AI RMF, ISO/IEC 42001) pending legal review.

---

### 9. References
* GDPR (2016/679), CCPA/CPRA (Cal. Civ. §1798), LGPD (Lei 13.709/2018)  
* HIPAA Privacy Rule (45 CFR §164), GLBA Safeguards (16 CFR 314)  
* EU‑AI‑Act (2024 text), ISO/IEC 42001:2023  
* Restatement (Third) of Torts, Product Liability  
* Platform Work Directive (EU) 2024/2831  
* Digital Services Act (EU) 2022/2065, Art. 28b (minors)  
* Council of Europe Framework Convention on Artificial Intelligence (CETS 225, open for signature 2024)  
* Budapest Convention on Cybercrime (ETS 185) and Second Additional Protocol (2022)  
* OECD Recommendation on Artificial Intelligence (2019, revised 2024)  
* ITU‑T Focus Group on AI/ML — technical standards output  
* UN Secretary‑General's AI Advisory Body reports (2024–)  
* *Magnifica Humanitas*, Pope Leo XIV (15 May 2026), §§102–111, §§148–156, §§165–169, §§201–203, §§219–227  

---
End of Annex I
