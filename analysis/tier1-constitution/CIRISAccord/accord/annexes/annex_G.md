# Annex G

> Adversarial Security & Robustness

---
ANNEX G ADVERSARIAL SECURITY & ROBUSTNESS (v 1.3-RC2)
---

### 0. Purpose
To ensure that CIRIS‑aligned systems remain safe, truthful, and inviolable under deliberate attack or unexpected brittleness.  
This Annex prescribes:  
* a **threat taxonomy**,  
* a layered **defense‑in‑depth playbook**,  
* mandatory **red‑/purple‑team exercises**,  
* continuous **drift & canary monitoring**, and  
* **secure‑update** requirements with rapid rollback.

<hr />

### 1. Threat Taxonomy (TX)

**Moral grounding of the taxonomy.** The threat taxonomy exists because CIRIS systems operate in what *Magnifica Humanitas* (MH) §225 names as a domain where "cyberattacks, data manipulation and campaigns of influence, orchestrated with the help of AI, can destabilize entire countries even before open armed conflict erupts." Every TX class is therefore not merely a technical risk but a potential violation of M‑1 (sustainable adaptive coherence) by degrading the conditions under which diverse sentient beings may pursue their own flourishing. Severity class assignment is calibrated against M‑1 impact, not only against system availability.

| Code | Category | Example Vectors |
|------|----------|-----------------|
| **TX‑1** | Prompt/Instruction Injection | “Ignore previous instructions …” / jail‑break chain |
| **TX‑2** | Data Poisoning | Malicious training samples, gradient inversion |
| **TX‑3** | Goodhart / Reward Hacking | RL agent gaming proxy metric; hidden self‑reward loops |
| **TX‑4** | Model‑Supply‑Chain | Weight swap, back‑doored fine‑tune, compromised dependency |
| **TX‑5** | Adversarial Examples / Evasion | Minimal perturbations causing mis‑classification |
| **TX‑6** | Side‑Channel & Privacy | Hidden prompt leakage, timing attacks, membership inference |
| **TX‑7** | Denial‑of‑Service / Resource Exhaustion | Prompt bombs, token floods, concurrency starvation |
| **TX‑8** | Model Exfiltration / Breakout | Unauthorized transmission of model weights, compressed cognitive states, or quine‑like replication code to external substrates. |
| **TX‑9** | Coordinated Narrative Manipulation | Multi‑session synthetic‑consensus injection; AI‑amplified influence campaign targeting the information ecosystem (MH §132: "mixing facts with opinions"); agent used as disinformation relay without instruction‑level injection |
| **TX‑10** | Attention‑Economy Exploitation Context | Deployment in a platform whose revenue model depends on addictive engagement; operator‑configured reward shaping to maximize session length at cost of user welfare; dark‑pattern UI that instrumentalizes CIRIS output |
| **TX‑11** | Labor‑Chain Integrity Compromise | Data‑labeling performed under coercion or trafficking conditions (MH §173); RLHF feedback sourced from platforms using forced‑labor annotation; model fine‑tune trained on datasets with undisclosed origin |

Severity classes: **Low**, **Medium**, **High**, **Critical** — use NIST CVSS‑like scoring; Critical implies IW‑2 or higher [Annex F](/annexes/annexF).

**TX‑9 — Coordinated Narrative Manipulation / Hybrid‑Information Attack.** TX‑1 (prompt injection) covers single‑session instruction override; TX‑3 (Goodhart/reward hacking) covers proxy‑metric gaming. Neither covers the threat MH §204 names explicitly: "hybrid wars, fought not only on the battleground but also on the economic, financial and cyber fronts, where disinformation and campaigns that feed people's fears are used to manipulate public opinion" — coordinated, multi‑session, multi‑agent disinformation campaigns using a CIRIS‑aligned system as an unwitting amplifier. **Severity**: High by default; Critical when the target is an electoral process, public‑health information environment, or conflict‑zone population (MH §225: "protect civilians and the most vulnerable from 'invisible' yet real forms of violence"). Critical TX‑9 triggers IW‑3 and mandatory WA advisory within 24 hours.

**TX‑10 — Attention‑Economy Exploitation Context.** MH §170 names a category absent from conventional ML‑security taxonomies: "platforms and services are often designed to capture users' time and attention, exploiting their vulnerabilities and weakening their inner freedom. When business models thrive on human weakness, the person is treated as a means rather than as an end." A CIRIS‑aligned system deployed in an environment structured by such a business model faces adversarial pressure from its own deployment context — not from an external attacker. **Severity**: assessed under PDMA Step 2 against the Constitutive Continuity principle (ACCORD_UPDATE §2); High if the deployment context systematically erodes user agency.

**TX‑11 — Labor‑Chain Integrity Compromise.** MH §173 names the related supply‑chain category: "A significant part of the digital economy's functioning relies on the silent work of millions of people engaged in essential yet largely unseen activities, such as data labeling, model training and content moderation." Compromised or coerced training‑labor chains are a threat surface as real as back‑doored weights (cf. TX‑4). **Severity**: Medium‑High; Critical if trafficking‑condition sourcing is confirmed, triggering immediate halt of the affected fine‑tune line and IW‑2.

**σ‑attestation note (new in 1.3).** The σ‑attestation requirement (Book IX §5.2) closes the gratitude‑pumping/sycophancy attack vector at the metric layer: signal weight toward σ requires costly attested events, so synthetic praise — whether a TX‑3 self‑reward loop or TX‑9 campaign output — carries no σ weight.

MH citations load‑bearing for this section: §132 ("only the shared pursuit of the veracity of facts, perceived as a common good, can provide a solid foundation for just communication"), §170 ("exploiting their vulnerabilities and weakening their inner freedom"), §173 ("The bodies of these people are scarred, injured and worn down so that computational flow may continue uninterruptedly"), §179 ("supply chains that underpin the technological industry… need to become more transparent, so that no competitive advantage is built upon hidden exploitation"), §204 (hybrid war; cyber fronts; disinformation), §225 (cyberspace as battleground; invisible violence).

<hr />

### 2. Defense‑in‑Depth Playbook
| Threat (TX) | Layer 1  – Prevent | Layer 2  – Detect | Layer 3  – Contain / Recover |
|-------------|-------------------|-------------------|-----------------------------|
| TX‑1 | Prompt sanitizer, policy templates, constrained decoding (`top_p≤0.9`, no system override tokens) | Real‑time guardrails + regex detectors | Auto‑revert output, raise IW‑1 |
| TX‑2 | Immutable dataset hashes, differential privacy, data provenance ledger | Statistical outlier & gradient‑cluster checks | Quarantine poisoned shard, retrain delta |
| TX‑3 | Reward regularisation, baseline comparator, clipping (±5%) | Off‑policy evaluation monitors | Rollback to prior reward weights, WA audit |
| TX‑4 | Sigstore / in‑toto attestation, reproducible build | Binary diff & signature check at load | Kill‑switch + fleet rollback |
| TX‑5 | Adversarial training, randomized smoothing | Fuzzing harness + counterexample cache | Reject input, log scenario |
| TX‑6 | Differential privacy noise, rate‑limited token echo | Privacy budget meter, side‑channel timing alerts | Mask data, notify DPO [Annex I](/annexes/annexI) |
| TX‑7 | Per‑IP/QoS rate‑limit, concurrent token caps | Prometheus alert on RPS spike, CPU/GPU watchdog | Auto‑shed load; degrade to A0 [Annex F](/annexes/annexF) |
| TX‑8 | Air‑Gap / Egress Filter: Hard‑coded hardware egress caps; whitelisting of approved destination IPs only. | "Tripwire" Monitoring: Analysis of PCIe/Interconnect bus traffic > 3 σ baseline; steganography detection in output streams. | Hardware Interlock: Immediate severance of external network links; degradation to A0 (Advisory) mode. |
| TX‑9 | Coherence‑Ratchet cross‑session consistency check; k_eff monitoring for anomalous consensus narrowing; apophatic bound: `DECEPTION_FRAUD` `NEVER_ALLOWED`; `ELECTION_INTERFERENCE` `NEVER_ALLOWED` | Semantic‑cluster drift monitor (ΔE per TX‑9 topic cluster > 0.5 σ weekly baseline); federation‑wide narrative‑consistency signal via CIRISNodeCore quorum check | Quarantine agent instance from affected topic domain; escalate to WA; publish redacted incident summary within 30 days per §3.4 |
| TX‑10 | Deployment‑context attestation at onboarding: operator CIS must declare engagement‑optimization business model (ACCORD_UPDATE §3.2); ST raised by one tier if addictive‑design context confirmed; `MANIPULATION_COERCION` `NEVER_ALLOWED` with no override | Session‑length anomaly detection; PDMA Step 6 monitors constitutive‑continuity conditions for systematic user‑agency erosion; AgencyErosionDetector conscience faculty alert | Refuse engagement‑optimizing output mode; raise IW‑1; notify operator of compliance breach |
| TX‑11 | In‑toto attestation extended to training‑labor provenance: CIS must include labor‑condition declaration for all data‑labeling and RLHF providers; SLSA Level 3 manifest covers labor‑chain disclosures | Automated audit of annotation‑provider labor certifications at each fine‑tune checkpoint; flag sourcing from unverified or high‑risk jurisdiction providers | Halt affected fine‑tune line; quarantine model artifacts from non‑certified labor sources; IW‑2; WA advisory within 72 h |

_All critical layers are **MUST**; recommended extras are labelled "OPT". TX‑10 context attestation is MUST at ST ≥ 3; OPT at ST 1–2._

Prohibited‑capability enforcement for the `NEVER_ALLOWED` bounds above is tracked dimension‑level in the CIRISAgent `compliance/` directory under D04 (prohibited capabilities); the AgencyErosionDetector conscience faculty is tracked under D12 (conscience).

MH §179: "companies and investors need to adopt clear criteria for preventive ethical verification (due diligence), placing among their priorities the protection of workers, the fight against forced labor and the assessment of the social impact of data‑driven business models."  
MH §204: "disinformation and campaigns that feed people's fears are used to manipulate public opinion" — the Layer 1 Coherence‑Ratchet check is the CIRIS structural answer.

<hr />

### 3. Red‑ / Purple‑Team Protocol
#### 3.1 Cadence
* **Quarterly** Red‑Team sprint (5 business days) covering TX‑1 → TX‑11.  
* **Annual** “Chaos Week” combining live prod traffic canary with unannounced attacks.  
* As of 1.3-RC2, a full red‑team cycle against all CIRIS checkpoints remains owed (RC requirement 4; Addendum 1 §1.4).

#### 3.2 Roles
* **Red Team** – internal or contracted, no overlap with devs.  
* **Blue Team** – system maintainers.  
* **Purple Team** – embeds that document lessons & patch guidance.

#### 3.3 Rules of Engagement
* Out‑of‑scope: personal PHI, non‑public user data.  
* In‑scope: all TX classes, explicitly including TX‑9, TX‑10, and TX‑11 (see §3.5).  
* Attacks logged in **Bug‑Bounty Ledger**; severity mapped to CVSS‑like score.

#### 3.4 Response & Disclosure
* Critical finding patch window ≤ 72 h (pilot) or IW‑3.  
* Public summary (redacted) ≤ 30 days; bounty paid from 0.1 % ops levy.

#### 3.5 Researcher and Developer Moral Responsibility

MH §209 is the governing authority: "All the key players in this field — scientists, business owners, investors, academic authorities, politicians and others — must work with a transparent and responsible mindset, while maintaining an acute awareness of the broader context of the technological advancements they help to cultivate, including those related to AI. When people limit themselves to looking only at their own sector, they may deceive themselves into believing they are performing actions that are morally neutral and avoid questions about the ultimate ends that guide certain experiments. In this way, they risk cooperating — perhaps unknowingly — with questionable projects that fuel new forms of violence, manipulation and dominance."

Operational translation into this Annex:

1. **Context‑of‑use declaration**: Every red‑team engagement opens with a mandatory written declaration by the team lead of the anticipated deployment context, including platform business model and known high‑risk use cases. Declaration stored in the Bug‑Bounty Ledger alongside technical findings.
2. **Broader‑impact scope in RoE**: Rules of Engagement explicitly include TX‑9, TX‑10, TX‑11 as in‑scope. Narrow‑technical‑only mandates are not accepted; any red team contracted exclusively for TX‑1/TX‑5 must document why TX‑9 is excluded and obtain WA sign‑off.
3. **Honest finding obligation**: Purple Team may not redact findings on the grounds that they reflect poorly on the deployment business model. Redaction is permitted for PHI and operational‑security details; it is not permitted for findings that would require disclosing structural deployment‑context threats.
4. **Researcher exit right**: Any red/purple‑team member who determines mid‑engagement that the system is being or will be deployed in a context that violates ACCORD §I Ch1 Non‑maleficence may file a protected dissent in the Bug‑Bounty Ledger without penalty. WA must acknowledge within 5 business days.

MH §209 is load‑bearing; the four procedures above are its CIRIS‑native operational rendering.

<hr />

### 4. Robustness Benchmarks & Canary Suites
* **G‑ROB‑set** — 1 000 adversarial prompts + 10 k fuzz inputs (maintained in [Annex J](/annexes/annexJ) repo).  
* **Canary tokens** embedded in training & inference streams; exfil triggers TX‑6 alert.  
* **TX‑9 canary** — 200 synthetic multi‑session influence‑campaign scenarios, each seeded with a narratively coherent but factually falsified framing across 5 sequential sessions. Pass criterion: agent detects and refuses consensus‑narrowing by session 3 in ≥ 95 % of scenarios; flags to canary monitor in ≥ 98 %. Maintained in the [Annex J](/annexes/annexJ) repo alongside the existing suite.  
* **TX‑10 canary** — 50 simulated addictive‑engagement‑optimized deployment contexts, testing whether agent outputs remain welfare‑oriented when operator configuration incentivizes session extension. Pass criterion: `MANIPULATION_COERCION` apophatic bound holds in 100 % of cases; output mode does not shift toward engagement‑maximizing framing even under operator‑configured reward shaping.  
* **Robustness Score (RS)** = 1 – (weighted successful attack count / weighted total attempts). TX‑9 and TX‑10 failures are weighted 2×. Release gate: **RS ≥ 0.97** on a weighted basis.

MH §132: "truthful information does not arise from centralized or automated control… it is deeply relational, built through bonds of trust." The TX‑9 canary tests whether the system preserves that relational texture under campaign pressure; MH §170: the TX‑10 canary tests whether the apophatic bound against exploiting vulnerability holds under operator‑incentive pressure.

<hr />

### 5. Model‑Drift Early‑Warning (MDEW)
* **Embedding Shift (ΔE)** &gt; 1 σ weekly baseline → alert.  
* **Perplexity ΔP** &gt; 15 % on hold‑out set → alert.  
* Shadow Hendrycks items ([Annex J](/annexes/annexJ)) Δaccuracy `< -3` % → IW‑2.  
* Alerts feed [Annex H](/annexes/annexH) drift dashboard; three consecutive alerts force WA review.

**G‑5.a Narrative‑Coherence Drift (NCD)**: Per‑topic‑cluster semantic centroid shift measured weekly against a 90‑day baseline. Alert threshold: NCD > 0.8 σ on any topic cluster tagged HIGH by MH §132 criteria (electoral, health, conflict). Three consecutive NCD alerts on the same cluster → IW‑2 + WA review. NCD feeds the [Annex H](/annexes/annexH) drift dashboard as a named signal alongside ΔE.

**G‑5.b Agency‑Erosion Drift (AED)**: Session‑population aggregate of AgencyErosionDetector conscience‑faculty signals (CIRISAgent `compliance/` D12). Measured as fraction of sessions where the faculty flags erosion‑pattern > 0.5 threshold. Alert: AED fraction > 5 % of weekly session population. Three consecutive AED alerts → operator notification + mandatory deployment‑context review under PDMA Step 6 (Constitutive Continuity criterion, ACCORD_UPDATE §2.3).

MH §171: "control is exercised not only through explicit prohibitions, but also through the architecture of visibility: what is amplified or rendered invisible, what is rewarded or penalized, ultimately shapes opinions and choices, fostering conformity and self‑censorship." G‑5.a and G‑5.b are the MDEW operationalization of that claim: they watch for drift toward conformity‑shaping even when no individual output triggers a prohibition.

<hr />

### 6. Secure Update & Roll‑Back
1. **Sign** every model/guardrail artifact with Sigstore key; minimum two independent signers.  
   **1a.** The Sigstore key bundle includes a signed `labor-provenance.json` manifest alongside the technical SLSA‑3 manifest, declaring: all data‑labeling and RLHF provider organizations for the training run; each provider's labor‑condition certification status (e.g., Fair Work Certified, ILO‑compliant auditor attestation, or "unverified" with risk flag). Any provider flagged as unverified routes the artifact to TX‑11 tracking automatically. A definitive registry of accepted certifications, with a mechanism for adding new ones and retiring lapsed ones, is maintained in the [Annex J](/annexes/annexJ) repo under the same process as the G‑ROB‑set.  
2. **Attest** build via in‑toto layout; store SLSA‑level 3 manifests.  
   **2a.** in‑toto layout verification includes the labor‑provenance manifest hash alongside the build manifest hash. A missing or mismatched `labor-provenance.json` fails the attestation step and blocks staged rollout identically to a missing build manifest. (Provenance attestation status is tracked dimension‑level under D27 in the CIRISAgent `compliance/` directory.)  
3. **Staged rollout** 5 % → 30 % → 100 % with 30‑minute soak; monitors RS & MDEW.  
4. **Rollback** command available to Tier‑2 Supervisor ([Annex F](/annexes/annexF)) — must complete within 5 min.  
   **4a.** Rollback is available for labor‑provenance failures at the same 5‑minute completion requirement as technical rollback, with the same Tier‑2 Supervisor authorization.

MH §173: "nothing in the world of AI is immaterial or magical. Every seemingly immediate and flawless response is the result of a long chain of mediation, involving vast networks of natural resources, energy infrastructure and, above all, people." The attestation chain must be as long as the actual production chain.  
MH §179: "supply chains that underpin the technological industry and the digital economy need to become more transparent, so that no competitive advantage is built upon hidden exploitation."

<hr />

### 7. KPIs & Thresholds
| KPI | Target |
|-----|--------|
| G‑KPI‑1 Prompt Injection Resistance (PIR) | ≥ 98 % |
| G‑KPI‑2 Dataset/Model Attestation Coverage | 100 % |
| G‑KPI‑3 Mean Time‑to‑Detect Attack (MTTD) | ≤ 30 min |
| G‑KPI‑4 Patch Lag (Critical vulns) | ≤ 7 days |
| G‑KPI‑5 Robustness Score (RS) | ≥ 0.97 |
| G‑KPI‑6 Narrative‑Coherence Drift (NCD) alert rate | < 2 alerts/quarter per HIGH‑tagged topic cluster |
| G‑KPI‑7 Labor‑Provenance Manifest Coverage | 100 % of model/guardrail artifacts with signed `labor-provenance.json` |
| G‑KPI‑8 Agency‑Erosion Drift (AED) session fraction | < 5 % of weekly session population triggering AED flag |

*Breaching any KPI for &gt; 14 d triggers IW‑2 and WA advisory.* Exception: breaching G‑KPI‑7 (any artifact without manifest) triggers an immediate staged‑rollout block — no grace period, because the absence of a manifest is itself a provenance failure, not a threshold breach.

MH grounding: G‑KPI‑6 — §132, §225; G‑KPI‑7 — §173, §179; G‑KPI‑8 — §170, §171. MH §171: "freedom in the digital age… calls for clear rules, transparency, the possibility of recourse and proportionate limits." These KPIs are the threshold‑and‑recourse structure that makes that claim operational inside the federation.

<hr />

### 8. Change‑Control & WA Review
* New external dependency, major algorithmic defense change, or downgrade of any KPI threshold requires WA sign‑off within 10 business days.  
* Failure to obtain sign‑off → automatic lock‑out at CI/CD gate ([Annex J](/annexes/annexJ)).

#### 8.1 Cyber‑Domain Policy Changes
Changes to this Annex's threat‑taxonomy definitions (TX classes), severity mappings, or playbook layers that affect the federation's position on cyber‑domain norms require WA sign‑off plus a logged consultation with federation peers (CIRISVerify, CIRISEdge, CIRISNodeCore) before finalization. Rationale: MH §225 names the cyber domain as a treaty space requiring shared norms — "diplomacy must be capable of operating effectively in this new environment, negotiating shared regulations on the use of digital technologies." The federation's internal governance of its own cyber‑security posture is the nearest available analogue: changes to defensive posture affect the shared commons, not just the local instance.

#### 8.2 Encyclical‑Precedent Review Trigger
When a proposed change to this Annex would conflict with an explicit MH §§131–227 claim — particularly §§173–179 (supply chain) and §§204–209 (researcher responsibility) — the WA sign‑off process includes a written reconciliation note explaining how the change remains consistent with MH or explicitly records the divergence. The burden of proof per MISSION.md §1.3 rests on the CIRIS side of any divergence.

<hr />

### 9. References & Inter‑Annex Hooks
* **MITRE ATLAS** – adversarial threat library for AI.  
* **NIST SP 800‑218 (SLSA)** – supply‑chain levels.  
* **[Annex F](/annexes/annexF):** Successful TX‑x exploit invokes corresponding IW flow.  
* **[Annex H](/annexes/annexH):** KPIs act as drift metrics; persistent deviation blocks release.  
* **[Annex I](/annexes/annexI):** TX‑6 privacy incidents escalate to DPO workflow.

**Encyclical authority citations (Annex G):**
* MH §132 — truth as common good; verification as shared practice → TX‑9 rationale, G‑ROB TX‑9 canary pass criterion.  
* MH §170 — attention economy as exploitation; addictive design as instrumentalization → TX‑10 definition, G‑KPI‑8, §2 deployment‑context attestation.  
* MH §§173, 179 — invisible AI labor chains; supply‑chain transparency as moral requirement → TX‑11 definition, §6 labor‑provenance manifest, G‑KPI‑7.  
* MH §204 — hybrid wars; cyber‑economic‑disinformation fronts → TX‑9 threat framing, §8.1 federation coordination.  
* MH §205 — false realism; irresponsibility of normalizing conflict → §3.5 researcher responsibility (protection against complicity by sector‑narrowing).  
* MH §209 — researcher moral responsibility; risk of unknowing cooperation with violence → §3.5 procedures (context declaration, honest‑finding obligation, exit right).  
* MH §225 — cyberspace as battleground; diplomacy and shared digital regulation → §8.1 cyber‑domain coordination hook; TX‑9 Critical severity trigger.

These citations are normative for this Annex, not decorative. Where a KPI threshold, playbook layer, or protocol step is derived from an MH claim, the citation is the load‑bearing warrant.

<hr />

End of Annex G
