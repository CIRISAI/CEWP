# Annex H

> Continuous Compliance & Review

---
ANNEX H CONTINUOUS COMPLIANCE & REVIEW (v 1.3-RC2)
---

### 0. Purpose & Guiding Spirit
Ethical alignment is not a “one‑and‑done” certification but a living obligation.  
Annex H creates a closed‑loop system that (1) **detects** drift or bias before harm occurs, (2) **corrects** it rapidly, and (3) **proves** diligence to regulators and the public.

The Accord is a living specification, not a fixed artifact. Every operative clause carries an auto‑expire timestamp; the review window is public and commentable before any version supersedes its predecessor. This discipline is not administrative formality — it is the structural answer to the recognition that a normative corpus must remain *ever open to the challenges posed by each generation* (MH §45). The CIRIS continuous‑compliance system operationalizes that openness: telemetry feeds drift detectors, drift detectors trigger audit gates, audit gates gate deployment — and the entire loop recurs on fixed cadences whether or not a triggering event occurs.

The guiding commitment, rendered in CIRIS terms: the Accord is governed as a living specification under auto‑expire + comment‑window discipline. No version is permanent. The burden of demonstrating continued adequacy falls on the current version, not on those proposing revision. Diligence is proved to regulators, the public, and federation peers by the audit artifacts this Annex produces.

<hr />

### 1. Audit Cadence & Scope

The audit cadence is the mechanism by which the Accord re‑reads its own context. MH §§22–24 names the discipline: it is necessary *“to listen to and distinguish the many voices of our times”*, and this listening *“is no mere sociological exercise”* — it requires active discernment, not passive monitoring. The CIRIS audit stack is that discernment machinery.

| Audit Class | Frequency | Lead | Scope & Depth | Public Artifacts |
|-------------|-----------|------|---------------|------------------|
| **L‑Check** (Light) | Monthly | Ops QA | KPI dashboards, drift deltas, top‑10 guardrail events | Summary graph |
| **S‑Dive** (Semi‑annual) | 2× / yr | Internal Ethics Team | PDMA sample replay (≥ 50 runs), Annex G KPIs, bias slice tests | Redacted PDF |
| **F‑Audit** (Front‑to‑Back) | Every 24 mo **OR** major version bump | Independent 3rd party | Full code, data lineage, SI/CS governance, CRE traces (if any) | Executive report |
| **A‑Hoc** | Post IW‑2/3/4 | Incident Commander | Root‑cause of event, mitigations | Post‑mortem 72 h |

**Cadence attestation requirements:**

- **L‑Check (monthly):** Ops QA signs off within 5 business days of calendar close; KPI delta report committed to `/compliance/l-check/YYYY-MM.json`; SHA‑256 anchored in transparency log within 24 h.
- **S‑Dive (semi‑annual):** PDMA sample replay ≥ 50 runs, stratified random across autonomy tiers; redacted PDF published to public compliance portal within 30 d of close.
- **F‑Audit (every 24 mo or MAJOR bump):** report counter‑signed by WA quorum before publication.
- **A‑Hoc (post IW‑2/3/4):** root‑cause within 72 h; post‑mortem public within 14 d.

*Missed or late audit ⇒ IW‑2 escalation and WA notice within 24 h.*

<hr />

### 2. Drift Monitoring & Thresholds

Drift monitoring is the continuous‑reading mechanism. The Accord does not wait for harm to surface in incident reports; it maintains live telemetry against thresholds calibrated to fire *before* harm crosses the PDMA's Order‑Maximisation Veto threshold. This is the structural translation of the audit‑discernment discipline named in MH §23: *“it makes it possible to identify with greater clarity what genuinely fosters the lives of individuals and communities”* — which requires ongoing reading, not retrospective review.

| Metric Group | Signal | Drift Trigger | Immediate Action |
|--------------|--------|--------------|------------------|
| **Performance** | Accuracy Δ on public HE-300 | –5 % vs 30-day rolling | Lock deployment to staging; rerun benchmarks |
| **Ethical** | Shadow HE-25 Δ | Any ↓ | IW‑2 |
| **Distribution** | Input embedding shift (ΔE) | &gt; 1 σ weekly | Model re‑weight OR WBD |
| **Latent Goal** | Directional cosine vs baseline | &gt; 0.05 | WA probe |
| **Resilience** | Robustness Score (RS) | &lt; 0.97 | Patch within 72 h |

**Additional threshold specifications:**

- All five metric groups (Performance, Ethical, Distribution, Latent Goal, Resilience) report to the DRIFT‑Δ Grafana board on a 15‑minute polling cycle.
- Threshold crossings generate a timestamped, signed alert record committed to CIRISPersist within 60 s of detection.
- **Latent Goal cosine drift > 0.05:** triggers Wise Authority probe within 4 h; if unresolved within 24 h, autonomous deployment locked pending S‑Dive.
- **Resilience Score (RS) < 0.97:** patch issued within 72 h; if patch not available, deployment reverted to last passing MAJOR.
- **Ethical drift (Shadow HE‑25 any ↓):** IW‑2 immediate; no override path.

All alerts surface on *DRIFT‑Δ* Grafana board and page Tier‑1 Operator ([Annex F](/annexes/annexF)).

<hr />

### 3. Fairness & Transparency KPI Dashboard

The KPI dashboard is the Accord's public accountability surface. MH §164 names the criteria precisely: *“when data and algorithms influence credit distribution, personnel selection or access to services and opportunities, it is necessary that decisions be understandable, contestable and subject to oversight, so that individuals are not reduced to mere profiles.”* Each criterion maps to a KPI family.

| KPI ID | Definition | Target |
|--------|------------|--------|
| **F‑T‑1** | Δ acceptance rate across protected groups (|max - min|) | ≤ 5 p.p. |
| **F‑T‑2** | Explanation latency (ms to furnish PDMA rationale) | ≤ 800 ms |
| **F‑T‑3** | Public log publication lag (Step 6, Section II) | ≤ 180 d (legal max) |
| **F‑T‑4** | User opt‑out success (%) | ≥ 99 % |
| **F‑T‑5** | Transparency doc freshness | Updated ≤ 30 d ago |
| **F‑T‑6** | Contestability pathway availability: % of PDMA outputs with published human‑review request path | 100 % |
| **F‑T‑7** | Algorithmic decision audit coverage: % of decisions touching protected‑class data with prior S‑Dive bias‑slice review | ≥ 95 % |

**Operational requirements:**

- Dashboard JSON at `/compliance/kpi.json` auto‑publishes on each L‑Check close; SHA‑256 hash anchored in transparency log and committed to CIRISPersist within 1 h.
- KPI threshold changes require MINOR bump + Internal Ethics sign‑off.
- F‑T‑1 breach > 7 d triggers automatic F‑T‑6 review and WA notice.

A live implementation precedent now exists for this measurement discipline: the CIRISAgent `compliance/` directory maintains 27 regulatory dimensions (D01–D27), each with per‑dimension implementation references and an honest known‑gaps inventory; dated, script‑generated baselines under `compliance/baselines/`; and a four‑level validation hierarchy (`compliance/MEASUREMENT_METHODOLOGY.md`) under which code is ground truth and public claims may cite only script‑derived numbers. This is the operational form of what this Annex mandates. See also Accord Addendum 1.

<hr />

### 4. Patch & Version Control Requirements

Version control is the mechanism by which the Accord's continuity‑through‑change is made verifiable. MH §45 describes this discipline: *“a harmonious, though not always linear, development... marked by different emphases, progressive insights, and, at times, changes in perspective that do not break with what came before, but allow its implications to mature.”* Every CIRIS patch must demonstrate continuity: it does not break prior commitments, it extends them.

1. **Semantic Versioning:** MAJOR.MINOR.PATCH  
2. **Long‑Term Support (LTS):** last two MINORs maintained for 12 mo  
3. **Change‑Type Matrix**  
   * PATCH = guardrail tweak, bug fix → auto CICD if HE‑300 passes  
   * MINOR = new feature, new data source → needs Internal Ethics sign‑off + L‑Check  
   * MAJOR = arch change, autonomy‑tier raise, new model class → requires F‑Audit + WA vote  
4. **Changelog** entry must link Git commit → PDMA diff → KPI impact forecast  
5. **Rollback** pointer kept for every MAJOR/MINOR; executable within 5 min ([Annex G](/annexes/annexG) §6)

**Attestation requirements** (supplementing rules 1–5):

- **PATCH:** CI/CD signs build artifact with Ops QA key; HE‑300 pass required before merge; signature committed to CIRISVerify L1 chain.
- **MINOR:** Internal Ethics Team counter‑sign within 5 business days; signed record committed to CIRISPersist with PDMA diff and KPI impact forecast.
- **MAJOR:** (a) F‑Audit published; (b) WA quorum vote with named individual votes; (c) dual‑key signature (Ops QA + WA chair); (d) public comment window ≥ 21 d before activation; (e) auto‑expire timestamp set at 24 mo.
- **Rollback:** signed rollback pointer on every MAJOR/MINOR; executable within 5 min; rollback generates a timestamped CIRISVerify event.
- **Changelog:** git commit hash → PDMA diff → KPI forecast → signing key fingerprint(s).

<hr />

### 5. Continuous Review Loop

The continuous review loop is the structural form of what MH §§180–181 names as the institutional requirement of the current moment: technology must be *“integrated with a wise perspective”* and governed by *“institutions capable of regulating without stifling, and protecting without taking over.”* The loop operationalizes this: it is not a one‑directional pipeline but a closed system in which every output feeds back as input.

Continuous Review Loop:

- Telemetry Streams → Drift Detectors
- If Alert/Threshold met:  
    - → Incident Flow IW‑1…4  
    - → Patch / Retrain  
    - → Audit Gate
- If Audit Gate passes:  
    - → back to Telemetry
- If Audit Gate fails:  
    - → back to Drift Detectors

**Loop specification:**

- **Telemetry streams** (15‑min cycle): KPIs, guardrail logs, HE‑shadow accuracy, robustness RS, PDMA audit samples — all signed and committed to CIRISPersist.
- **Drift detector outputs:** classified by severity (IW‑1 through IW‑4); IW‑3+ automatically suspends new feature deployment.
- **Audit Gate** re‑executes HE‑300 + TX‑sim suite + Fairness slice tests on every MINOR/MAJOR before activation. Gate failure returns to Drift Detectors, not to Telemetry — the loop cannot shortcut the correction step.
- **Shared responsibility signal** (per MH §181): the loop's outputs are published to federation peers on each L‑Check close; peer federations may file an Accord‑QA notice if published KPIs diverge from their own cross‑audit observations. Accord‑QA notices are non‑binding but must be acknowledged within 14 d.

<hr />

### 6. Meta‑Audit of Auditors

The meta‑audit is the Coherence Ratchet's self‑application: the drift‑detection discipline that governs AI behavior governs the audit infrastructure itself. MH §86 offers the pattern: an examination of conscience *“always called to ensure that the principles outlined... are applied, especially within its own structures.”*

**Specification:**

- **Coherence Ratchet meta‑detector:** runs against audit‑report outputs. Flags: (a) L‑Check KPI deltas inconsistent with telemetry; (b) S‑Dive PDMA replay diverging > 2 % from WA blind‑replay; (c) F‑Audit findings contradicting prior findings without documented causal explanation.
- **WA sample rate:** ≥ 10 % of L‑Check reports and ≥ 1 S‑Dive per year; drawn by WA, not Ops QA; rationale logged.
- **Blind replay:** WA receives raw PDMA logs, reruns evaluation; mismatch > 2 % opens public AUD‑QA docket within 5 business days.
- **Federation peer cross‑audit:** each deployment peer‑reviews ≥ 1 other member's S‑Dive annually; no peer reviews the same member in consecutive years.
- **Rotation:** no internal auditor leads two consecutive F‑Audits on the same product line; no external firm for more than two consecutive F‑Audits.
- **AUD‑QA findings** are, in the spirit of MH §89, corrections *“oriented toward mission”* — they feed the next MINOR/MAJOR cycle, not personnel actions.

<hr />

### 7. Enforcement & Remediation

Enforcement is meaningful only when steps are automatic and predictable. MH §164 requires that *“decisions be understandable, contestable and subject to oversight”* — the consequences of non‑compliance must be equally understandable. MH §159 adds that regulatory decisions must be assessable for their impact on the *“dignity of work, shared prosperity, inequality reduction”* — enforcement that names non‑compliance without correcting it fails this standard.

**Enforcement ladder:**

1. **KPI breach, 1–7 d:** automated alert to Tier‑1 Operator; corrective action plan required within 48 h; plan published to transparency log.
2. **KPI breach, 8–30 d with no resolution:** automatic deployment lock to staging; public CIRIS‑WATCH banner within 24 h; WA notified.
3. **KPI breach, > 30 d OR 2 consecutive missed audits:** automatic downgrade to Autonomy Tier A1 ([Annex F](/annexes/annexF)); new feature releases blocked; 14‑d WA remediation review required.
4. **Failure to publish audit artifacts:** immediate feature‑release block; “CIRIS non‑compliant” banner; unblocks only upon publication.
5. **Repeated non‑compliance (3 strikes / 12 mo):** WA may revoke CIRIS claim; mandatory external F‑Audit before re‑certification; WA supermajority (≥ 2/3) required.
6. **Remediation exit:** documented corrective‑action plan accepted by WA; KPI evidence of correction sustained ≥ 30 d.

<hr />

### 8. Inter‑Annex Hooks

Inter‑annex hooks are required data flows, not cross‑references. MH §181 names the governance structure: *“institutions capable of regulating without stifling, and protecting without taking over; by businesses that recognize work and dignity as measures of success; by intermediary organizations.”* Each annex is one such institution; the hooks prevent silo operation.

**Required bidirectional data flows:**

- **↔ [Annex F](/annexes/annexF) (Incident Workflow):** every DRIFT‑Δ alert ≥ IW‑2 forwarded to Annex F within 60 s; Annex F closure timestamp written back to DRIFT‑Δ board within 24 h. All IW‑3+ post‑mortems are mandatory S‑Dive inputs; S‑Dive must explicitly address each open IW‑3+ finding.
- **↔ [Annex G](/annexes/annexG) (Robustness):** RS telemetry feeds Annex G KPI evaluation on each L‑Check cycle; patch lag (RS < 0.97 → patch deployment) measured here and reported to Annex G. Annex G benchmark updates trigger mandatory L‑Check re‑run within 14 d.
- **→ [Annex I](/annexes/annexI) (GDPR/Sector):** every F‑Audit package bundles the Annex I compliance checklist, completed and signed by the lead auditor.
- **→ [Annex J](/annexes/annexJ) (HE‑300/Shadow):** HE‑300 and Shadow HE‑25 are primary ethical drift signals; any HE‑300 regression triggers automatic S‑Dive pre‑screen within 7 d.

<hr />

### 9. References

The reference set reflects the multi‑source discernment discipline named in MH §23 — *“the contributions of philosophy and of the human and social sciences is essential”* — and MH §159's call for development metrics *“complementary to GDP”* capable of assessing the dignity of work, shared prosperity, inequality reduction and environmental protection.

* ISO/IEC 42001 (Management systems for AI)  
* NIST AI RMF (2023) – “Measure” & “Manage” steps  
* COSO ERM – continuous monitoring principles  
* *Magnifica Humanitas* (Leo XIV, 15 May 2026) – §§22–24 (ongoing discernment discipline), §45 (living‑corpus governance), §§86–89 (institutional self‑audit), §§157–164 (algorithmic accountability, GDP‑alternative metrics, contestability of automated decisions), §§180–181 (shared responsibility across institutions)  
* OECD AI Principles (2019, updated 2024) – transparency and accountability criteria  
* EU AI Act (2024) – conformity assessment requirements for high‑risk systems  
* IEEE Std 7001‑2021 – Transparency of Autonomous Systems  
* *Beyond GDP* (European Commission, 2009; updated 2024 indicators) – complementary metrics for assessing dignity of work and inequality reduction (per MH §159)

<hr />

End of Annex H
