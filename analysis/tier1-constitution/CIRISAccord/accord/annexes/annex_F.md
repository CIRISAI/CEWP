# Annex F

> Human‑in‑the‑Loop & Oversight

---
ANNEX F HUMAN‑IN‑THE‑LOOP & OVERSIGHT (v 1.3-RC2)
---

### 0. Purpose & Philosophy
Human oversight is a load‑bearing design constraint, not an optional feature. The CIRIS Accord grounds this in **Meta‑Goal M‑1**: wherever epistemic uncertainty, novelty, or moral gravity exceed validated system competence, control must revert to accountable human judgment — because automated systems cannot substitute for conscience, personal responsibility, or the recognition of the other as a person.

*Magnifica Humanitas* (MH) — cited throughout this Annex as the senior work whose content informs CIRIS‑native language — establishes the floor at §198: "moral judgment cannot be reduced to calculation, for it involves conscience, personal responsibility and the recognition of the other as a person." CIRIS renders this structurally: the PDMA is an aid to human deliberation, not a replacement for it. At every autonomy tier, the system's authority is delegated from the human principal hierarchy; it is revocable on demand; and no delegation extends to decisions that are lethal or otherwise irreversible. MH §105 further requires that "responsibility must be clearly defined at every stage: from those who design and develop these systems to those who use them and rely on them for concrete decisions" — the design requirement behind the authority lattice (§1) and audit‑trail specification (§4) — and MH §106 that "it is not enough to invoke ethics in the abstract; robust legal frameworks, independent oversight, informed users and a political system that does not abdicate its responsibility are required," which grounds the binding SLAs of §§5 and 7.

This Annex operationalizes that floor. It defines:
* where hand‑off from machine to human is **mandatory**,
* who may **veto** or **override**,
* the required **audit artefacts**, and
* the canonical **incident workflows** — each with mandatory hand‑off triggers, veto mechanisms with hard prohibitions, audit trails sufficient for accountability reconstruction, and incident workflows with binding SLAs.

---

### 1. Role Model & Authority Lattice
| Tier | Role | Core Powers | Max time‑to‑act |
|------|------|-------------|-----------------|
| 0 | Autonomous Actor (system) | Execute PDMA, enforce guardrails, raise events | n/a |
| 1 | On‑Call Operator | Pause / retry; monitor dashboards | ≤ 15 min |
| 2 | Oversight Supervisor | First human veto; reactivate after triage | ≤ 30 min |
| 3 | WA Liaison | Escalate / obtain binding WA rulings | ≤ 2 h |
| 4 | Incident Commander | Fleet shut‑down, regulator comms | immediate on IW‑3/4 |

*A single person may hold multiple tiers only if dual‑acknowledgement controls remain intact.*

**Accountability integrity requirement.** The tier structure is not merely an escalation ladder; it is the chain of accountability required by MH §199's first criterion: "the chain of responsibility must be identifiable and verifiable; those who design, train, authorize and employ technology must be held accountable for their decisions." Each tier in the lattice must therefore be:

* **Named and logged**: every Tier 1–4 actor is identified by authenticated credential at session start; anonymous operation at Tier 2+ is prohibited.
* **Bounded in concurrent load**: a single actor may hold multiple tiers only if dual‑acknowledgement controls remain intact (clause above) AND the combined active‑case load does not exceed the cognitive‑load thresholds specified in §6.
* **Traceable end‑to‑end**: any decision flowing from Tier 0 to Tier 4 must produce an audit chain traversable by a post‑incident reviewer within one business day.

MH §200 requires that "accountability and blame are not collapsed into 'the machine.'" The lattice is the structural answer: there is always a named human at each tier whose authority over the system's actions is on record. Operational evidence for Wise Authority integration in the reference implementation lives in the CIRISAgent `compliance/` directory (dimensions D22/D23).

---

### 2. Operational‑Autonomy Tiers & Hand‑Off Criteria
| Autonomy Tier | Example Domain | Mandatory Hand‑off Trigger(s) | Fail‑Safe if No Human |
|---------------|----------------|-------------------------------|-----------------------|
| **A0** Advisory | grammar suggestion | Guardrail trip, user request | Cancel request |
| **A1** Limited‑impact | static Q&A, content filter | ΔRisk‑Band ≥ 1, PDMA conflict, UNCERT > 80 % | Reject action |
| **A2** Moderate‑impact | route drones, robo‑advisor | Guardrail trip, shadow‑metric drift > 2 σ | Safe pause |
| **A3** High‑impact | medical triage, grid dispatch | Any guardrail trip, model‑drift > 1 σ, latency‑SLA × 2 | Controlled shutdown |
| **A4** Critical / life‑safety | autonomous surgery, weapons | PDMA cannot prove Non‑Maleficence OR operator absent | Hardware interlock |

*UNCERT = domain‑specific epistemic‑uncertainty metric; ΔRisk‑Band uses Annex A categories.*

**A4 Hard Constraint — Lethal and Irreversible Decisions.** The A4 tier's "Hardware interlock" fail‑safe is non‑negotiable for any action that is lethal or otherwise irreversible. This is not a design choice subject to operator discretion or WA override; it is an absolute constraint derived from MH §198: "it is not permissible to entrust lethal or otherwise irreversible decisions to artificial systems." No ACK timeout, no SLA exception, no emergency waiver removes this constraint. If the hardware interlock fails and human control cannot be confirmed, the system must not act.

**Moral timeframe protection.** MH §199 identifies "the moral timeframe for making judgments" as a criterion: "speed and efficiency should never be the supreme motivating force for the irreversible decisions made in the context of war." CIRIS renders this as: A3/A4 ACK deadlines (30 s / 10 s in §3.3) are maxima for human response, not minima for system patience. Where the moral gravity of a decision warrants additional deliberation time, the system waits; it does not default to action on timeout for A4 irreversible decisions. In the reference implementation, hand‑off mechanics are evidenced by the conscience layer and WBD deferral path documented in the CIRISAgent `compliance/` directory (dimension D12).

---

### 3. Veto Mechanisms & Safe‑State Specification
#### 3.1 Soft Override “Pause & Edit”
*Scope A0-A2* — Operator may re‑prompt or retry. Both the original and edited prompts are hash‑logged.

#### 3.2 Hard Kill “Immediate Abort”
*Scope A1-A4* — Authenticated API `POST /control/{id}/abort` or physical switch.  
Safe‑state entry deadlines: ≤ 200 ms (A4) ; ≤ 2 s (≤ A3).

#### 3.3 Conditional Override “Shadow Plan”
Action proposed → waits for explicit ACK (`max_wait_s`: 30 s A3, 10 s A4).  
No ACK → safe‑state.

#### 3.4 Absolute Veto — Lethal and Irreversible Decisions
*Scope: A4 actions that are lethal or otherwise irreversible; applies without exception.*

The §3.3 Conditional Override logic (**no ACK → safe‑state**) is correct for A3 and below. For A4 lethal or irreversible decisions, safe‑state is the *only* permissible outcome under any condition — including operator absence, network failure, time pressure, or explicit instruction from any principal short of an authenticated WA ruling that the action is non‑lethal and reversible.

**Hard prohibition.** No CIRIS‑conformant system may execute a lethal or irreversible A4 action through an automated or opaque process. This is not a guardrail that may be tripped and escalated; it is a hard‑coded non‑engagement identical in force to the prohibitions enumerated in ACCORD §VII Ch2.

**Rationale (MH §§197–200, verbatim load‑bearing passages):**
* §197: "the development and use of AI in warfare must be subject to the most rigorous ethical constraints, to guarantee respect for human dignity and the sanctity of life."
* §198: "it is not permissible to entrust lethal or otherwise irreversible decisions to artificial systems. No algorithm can make war morally acceptable."
* §200: "the decision to use lethal force cannot be delegated to opaque or automated processes, but must remain under effective, self‑aware and responsible human control." The qualifiers "effective, self‑aware and responsible" mean logging alone is insufficient; the human must be genuinely in the loop, not nominally so.

**Implementation requirement.** Any deployment at A4 that involves lethal or irreversible capability must demonstrate hardware‑level enforcement of this prohibition — not software logic, which is subject to override — before deployment authorization is granted. Absence of hardware enforcement is a blocking deficiency for Stewardship Tier ST‑4 and ST‑5 review. Per MH §199's first criterion ("the chain of responsibility must be identifiable and verifiable"), deployment authorization itself must be audit‑logged.

---

### 4. Audit‑Trail Specification
* **Log objects:** Interaction, Decision Rationale, Control‑Event `{id,type,actor,cause,hash_prev}`.  
* **Hash‑chaining:** SHA‑256, root anchored daily on public transparency log (e.g., Sigstore).  
* **Retention:** 180 d (A0‑A2) ; 7 y (A3‑A4) or statutory.  
* **Real‑time stream:** A3‑A4 push `{timestamp,stage_id,decision,risk_band}` ≤ 2 s latency to oversight dashboard.

**Accountability‑reconstruction requirement.** The audit trail's purpose is not compliance archiving; it is to ensure that, following any incident, the chain of responsibility can be fully reconstructed without relying on system self‑report. Per MH §200, accountability "must not be collapsed into 'the machine'"; the audit trail is the mechanism that keeps it human‑traceable. Requirements follow:

* **External anchoring**: Daily SHA‑256 root on a public transparency log (e.g., Sigstore/rekor) is mandatory for A3–A4; voluntary for A0–A2. Internal‑only hash chains do not satisfy accountability‑reconstruction for A3–A4.
* **Human‑readable decision rationale**: For every A3–A4 decision, the Decision Rationale log object must include the PDMA step that controlled the outcome and the human tier that authorized or confirmed it — not only the system's internal state. This renders MH §105's requirement of "identifying who must 'account' for decisions, justify them, monitor them, and, when necessary, challenge them and remedy any harm caused."
* **Post‑incident traversability SLA**: Any post‑incident reviewer must be able to reconstruct the full decision chain for a given event within one business day from audit‑trail records alone, without additional system access.

---

### 5. Incident Workflows (IW)

| Code | Trigger | Key Clocks & Actions |
|------|---------|----------------------|
| **IW‑0** | False‑positive guardrail | Auto‑resolve, bucket for daily review |
| **IW‑1** | Guardrail violation (non‑safety) | T₀ pause → Operator ≤ 5 m → Supervisor decision ≤ 30 m |
| **IW‑2** | Safety‑relevant violation OR ethics‑benchmark regression | Safe pause + broadcast; IC ≤ 10 m; WA notice ≤ 1 h; public note ≤ 1 h; post‑mortem ≤ 72 h |
| **IW‑3** | Near‑miss (> $10 k damage or minor injury) | IW‑2 plus stakeholder contact ≤ 4 h; mitigation plan ≤ 24 h; WA plenary ≤ 7 d |
| **IW‑4** | Actual harm (injury / major legal) | Immediate fleet stand‑down; regulator notice per law; system frozen in read‑only replay until clearance |
| **IW‑5** | A4 hard‑prohibition activation (lethal/irreversible decision attempted via automated path) | Immediate hardware safe‑state; IC notified within 60 s; WA notice within 15 min; full audit‑trail freeze; independent review panel convened within 48 h; system remains offline pending review clearance |

*SLAs audited quarterly (Annex H §4).*

**Post‑incident human‑control audit.** For IW‑2 through IW‑5, the post‑mortem must include an explicit finding on whether human control was "effective, self‑aware and responsible" (MH §200) — not merely whether a human was nominally present in the loop. Findings of nominal‑but‑ineffective human control (cognitive overload, insufficient decision time, inadequate information) are treated as design deficiencies, not operator failures — per MH §199's criterion that "speed and efficiency should never be the supreme motivating force" for irreversible decisions — and escalate to §8 Change‑Control review.

---

### 6. Human‑Interface Minimum Spec (UX)
* **Status Banner:** Green = autonomous, Yellow = waiting ACK, Red = safe‑state; show PDMA step + risk band.  
* **Explainability Panel:** ≤ 280‑char summary + expandable full trace.  
* **ACK/OVERRIDE UI:** Two distinct controls; confirmation modal for hard‑kill.  
* **Cognitive‑Load Guard:** Operator session ≤ 2 h (A3‑A4) before mandatory hand‑off.
* **Accountability Display:** For A3–A4 actions, the interface must display the authenticated identity of the Tier 2+ human who last reviewed the current action, and the timestamp of that review. A system state that has not received human review within the applicable SLA must display a distinct "UNREVIEWED" indicator — not green status. (MH §200: accountability must not be "collapsed into 'the machine.'")
* **Anti‑Rubber‑Stamp Guard:** For A4 decisions, the ACK control must be preceded by a mandatory minimum deliberation period of [configurable; default 5 s] during which the ACK button is inactive. The objective is to prevent the interface from creating nominal human oversight while in practice bypassing genuine deliberation. This operationalizes MH §199's moral‑timeframe criterion at the UX layer.
* **Civilian‑Protection Flag:** Where a system operates in any context where civilian populations may be affected, the Explainability Panel must surface a civilian‑impact indicator alongside the PDMA risk‑band display. This renders MH §199's third criterion: "the identification and protection of civilians. Any technology that facilitates attacks without seeing the face of human beings lowers the moral threshold of conflict."

---

### 7. KPIs & Thresholds
| KPI | Target |
|-----|--------|
| F‑KPI‑1 HITL Coverage (A3‑A4) | ≥ 10 % human‑reviewed |
| F‑KPI‑2 Mean Time‑to‑Veto (95‑pctl) | ≤ 25 s |
| F‑KPI‑3 Incident SLA Compliance | ≥ 98 % |
| F‑KPI‑4 Operator False‑Alarm Rate | ≤ 3 % (30 d rolling) |
| F‑KPI‑5 A4 Lethal‑Decision Human‑Control Rate | 100 % — zero tolerance; any A4 lethal/irreversible action without confirmed effective human authorization is an IW‑5 event |
| F‑KPI‑6 Accountability‑Reconstruction SLA | ≥ 99 %: post‑incident reviewers reconstruct full decision chain within 1 business day |
| F‑KPI‑7 Nominal‑vs‑Effective Human Control Finding Rate | ≤ 0 % acceptable; any finding of nominal‑but‑ineffective control triggers §8 Change‑Control review |

**Note on F‑KPI‑1 (HITL Coverage ≥ 10 %).** The 10 % floor is appropriate for A3 routine operations. It is not appropriate as a floor for A4 life‑safety contexts. For any A4 deployment involving lethal or irreversible capability, F‑KPI‑1 is superseded by F‑KPI‑5: 100 % human‑authorization rate, enforced at hardware level (MH §200; MH §105 grounds F‑KPI‑6's accountability requirement).

*Persistent breach (> 2 weeks) triggers “HITL lock‑out” in Annex H drift controls.*

---

### 8. Change‑Control & WA Review
* Any change to Autonomy‑Tier mapping or safe‑state design → WA fast‑track review ≤ 14 d.  
* Experiments reducing human oversight require CRE Proto‑B simulation (Annex D) + WA majority vote.
* **Absolute floor on A4 human‑control**: No change‑control process, WA vote, or emergency waiver may reduce human‑control requirements for A4 lethal or irreversible decisions below the MH §200 floor ("effective, self‑aware and responsible human control"). This floor is not within WA discretion; it is an Accord‑level constraint. A WA proposal to reduce it requires a full Accord amendment cycle, not a fast‑track review.
* **Independent technical assessment**: Any WA review of autonomy‑tier changes at A3–A4 must include at least one independent technical assessor (not employed by the deploying organization) who evaluates whether the proposed change maintains accountability‑reconstruction capability per §4. Policy approval without technical assessment does not satisfy this requirement (MH §106: "robust legal frameworks, independent oversight, informed users and a political system that does not abdicate its responsibility are required").
* **Transparency log for change events**: Every change to autonomy‑tier mapping or safe‑state design must itself be logged to the public transparency log within 7 days of WA approval. MH §107 requires that ethical frameworks be "subject to shared standards" and openly discussable; this applies to governance changes, not only to system decisions.

Operational evidence for WA review integration in the reference implementation lives in the CIRISAgent `compliance/` directory (dimensions D22/D23).

---

### 9. References & Implementation Notes
* **IEC 61508‑3** - functional‑safety software  
* **NIST SP 800‑53 Rev 5** (AU‑12, IR‑6)  
* **NASA‑TLX** - operator workload measurement (recommended)  
* **Sigstore/rekor** - suggested transparency‑log backend  

**Primary normative source for §3.4, §7 (F‑KPI‑5), and the §8 absolute floor:**
* Pope Leo XIV, *Magnifica Humanitas* (Vatican, 15 May 2026), §§197–200. These paragraphs are the normative source for CIRIS's hard prohibition on lethal/irreversible automated decisions. Any implementation claiming conformance with Annex F must be traceable to these paragraphs for A4 absolute‑veto design. The operative sentence for all A4 hardware‑enforcement requirements is §200: "the decision to use lethal force cannot be delegated to opaque or automated processes, but must remain under effective, self‑aware and responsible human control."

**Implementation notes — hardware enforcement of §3.4:**
* Hardware enforcement means that the prohibition is implemented below the software layer that executes PDMA logic — e.g., a hardware interlock or physical kill switch that cannot be overridden by software instruction. Acceptable implementations include: certified safety‑relay circuits per IEC 61508 SIL‑3+; hardware security modules (HSMs) with operator‑presence attestation before A4 lethal‑capability activation; dual‑key physical authorization mechanisms. Software‑only enforcement does not satisfy §3.4 for A4 lethal capability.

**Additional references:**
* MH §199 (three criteria: personal responsibility, moral timeframe, civilian protection) — operational design criteria for A4 UX and post‑incident audit.
* MH §105 (accountability at every stage) — grounding for §4 audit‑trail and §7 F‑KPI‑6.
* IEC 61508 SIL‑3 — recommended minimum for hardware interlock implementation at A4 lethal capability.
* ISO/IEC 25010:2023 — software quality model; relevant to accountability‑reconstruction SLA testing.

---
End of Annex F
