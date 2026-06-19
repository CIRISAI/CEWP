# FSD: CEWP — CIRIS Epistemic Web Platform

**Pronunciation:** "soup."
**Status:** v0.2 — synced to CEG 1.0-RC29 (1+4 surface FROZEN) + the
  CIRIS Constitution (CC 0.1.5); fabric-node / holonomic reframe.
**What CEWP is:** the platform that the CIRIS federation composes —
  a **holonomic epistemic web** with no center, no DNS, and no
  load-bearing server, the runtime AI agents live in, and an AI
  governance / superalignment substrate where alignment is treated as
  an *epistemic-governance* problem with cryptographic accountability —
  not a training-time problem to be solved inside a model. The whole
  web speaks one small grammar (CEG's "1+4"); every node is a **fabric
  node** (`agent = fabric node + brain`); authority roots in
  accountable humans, never in bare infrastructure. CEWP is the
  platform identity for what was the seven-repo CIRIS Agent 3.0 stack,
  now deployed as cohabiting fabric-node cores (CIRISServer) + the
  substrate trio + the agent.
**Companion canonical docs:**
* [`CIRISRegistry/FSD/CIRIS_Constitution/`](https://github.com/CIRISAI/CIRISRegistry/tree/main/FSD/CIRIS_Constitution)
  — the **CIRIS Constitution** (CC 0.1.5): the CIRIS Accord 1.3-RC2
  (ethics) + CEG 1.0-RC29 (wire grammar) woven into one document, one
  version line. The top-of-stack canonical authority.
* [`CIRISRegistry/FSD/CEG/`](https://github.com/CIRISAI/CIRISRegistry/tree/main/FSD/CEG)
  — the CEG 1.0-RC29 wire-format spec (the grammar component;
  authoritative for the wire)
* [`CIRIS_FEDERATION.md`](../CIRIS_FEDERATION.md) — the layer
  architecture (identity / event / federation / verification /
  safety / lens / persistence / consensus)
* [`MISSION.md`](../MISSION.md) — NodeCore's six functions
* [`FEDERATION_SCALING_MODEL.md`](FEDERATION_SCALING_MODEL.md) —
  what carrying the entire internet costs at v1
* [`ANONYMOUS_TIER.md`](ANONYMOUS_TIER.md) — v2 deniability path

---

## 0. The thesis (one paragraph)

**CEWP's premise: big tech is not necessary. CEWP's bet:
cryptographic substrate + standardized ethical tracing prove it.**

CEWP is a **decentralized replacement for the internet** — and
simultaneously an **AI governance and superalignment platform** —
structured as an *epistemic web* of cryptographically-accountable
federations where humans, agents, and organizations are first-class
participants. Every load-bearing claim ("this content is accurate,"
"this peer is trusted," "this action should be deferred to a
human") is a signed wire-format artifact; the federation's
collective weighted-aggregate scoring of those artifacts is what
*governs* the system in real time. Agents live INSIDE CEWP; they
don't have CEWP as a tool. The web is **holonomic** (after Bohm's
implicate order): there is no center, no nameserver, and no
load-bearing server — you address a peer by its key, reach it over
the mesh, and trust it through a chain you can re-root at will.
Content is **holographic** — erasure-coded so any sufficient subset
of fragments reconstructs it at proportional fidelity, and the
federation re-establishes itself from any single survivor holding a
signed witness chain. The substrate eliminates the centralized
internet's structural failure modes (surveillance, platform lock-
in, trust collapse, content-quality regression, identity
fragmentation) AND removes the dependency on large datacenters at
both the data-sharing and AI-inference layers — substrate runs on
commodity hardware down to home-server class (CIRISHome on Jetson
Orin, already deployment-ready), with the
[scaling model](FEDERATION_SCALING_MODEL.md) showing the entire
internet at 5B users fits on home-broadband-class infrastructure
without a single datacenter required. The supporting empirical
bet — that *reasoning has a shape we can measure as everything else
scales*, and that distributed trace commons can substitute for
proprietary benchmarking and centralized alignment authority — is
articulated in the [research synthesis at
ciris.ai/research-status](https://ciris.ai/research-status/).

---

## 1. What problem CEWP solves

### 1.1 The standard framings (and why they're not enough)

**RLHF / Constitutional AI** — align by training. The model that
ships is the model the deployer trained; the alignment is opaque to
everyone except the deployer; users + governments have no recourse
once it's deployed; the model is the same model for all users
regardless of context, jurisdiction, or trust posture.

**Scalable oversight** — use AI to assist humans evaluating AI.
Helpful, but the oversight itself has no cryptographic
accountability surface; "AI evaluated AI" can be asserted but not
proved; misalignment of the assistant is unobservable.

**Mechanistic interpretability** — understand model internals.
Necessary for safety research, but doesn't itself produce a
governance system; understanding "why the model did X" doesn't make
"X was the right action" enforceable.

**Top-down regulation** (EU AI Act, UN AI advisory) — set rules,
require compliance. Has no enforcement substrate; a regulator can
say "AI systems shall be auditable" but the audit produces text
documents, not cryptographic attestation chains; cross-jurisdiction
enforcement is structurally weak.

**Web3 AI projects** — typically compute-or-data marketplaces with
crypto-economic incentives. Solve a different problem (decentralized
compute / model access) and don't produce a governance system over
the AI's *reasoning* or *outputs*.

### 1.2 What CEWP does differently

Alignment as **epistemic governance**:

1. **Every agent carries a federation identity** (an Ed25519 +
   ML-DSA-65 hybrid key). Identity is cryptographic, not
   administrative.
2. **Every load-bearing claim is a signed wire-format artifact**
   (the CEG 1+4 attestation primitives: `scores` + `delegates_to` /
   `supersedes` / `withdraws` / `recants`). What gets asserted, by
   whom, at what scope, is observable to any peer with directory
   access.
3. **Trust is computed from the attestation graph** —
   `weighted_aggregate` over `scores` attestations targeting any
   key. No "is this peer trusted" oracle; only "what does the
   federation collectively say about this peer at this scope, right
   now."
4. **Wrong / harmful actions become slashable artifacts**:
   `ModerationEvent` → witness aggregation → WA-quorum
   `SlashingAttestation` → trust score reduction → admission gate
   tightens → eviction sweeper drops their content. The platform
   has *enforcement teeth*, not just a regulator's text.
5. **Reconsideration is first-class**: `ReconsiderationRequest`
   exists explicitly so misjudged slashings can be reversed; the
   substrate doesn't ossify mistakes.
6. **Agents that don't know what to do *defer*** —
   `DeferralRequest` routes the question to humans with expertise
   in the relevant (domain, language) cell, via NodeCore's deferral
   routing. Deference is a first-class wire act with audit trail.
7. **Decisions decompose into a typed hierarchy**:
   `Goal → Approach → Method → Progress Measure`, all wire-format-
   typed (FSD-002 §3.6.2 / CEG §5.6.2). An agent's reasoning is
   not opaque text; it's a typed graph of attestable steps.
8. **Privacy and scale share one primitive** — `cohort_scope`. Local
   data (self/family) is structurally undiscoverable; published
   content is structurally trust-graph-observable (see
   [FEDERATION_SCALING_MODEL.md §9](FEDERATION_SCALING_MODEL.md) for
   the identity-aware-storage thesis).

The unifying property: **CEWP makes alignment a property of the
federation's runtime, not the agent's pre-training**.

---

## 2. The architecture — the fabric node

Every participant in CEWP is a **fabric node**: a piece of
infrastructure that stores, witnesses, degrades, and transports CEG
attestations *mechanically, never reasoning*. The defining identity:

> **`agent = fabric node + brain`.** The fabric node is
> deliberately agency-free; authority roots in accountable humans,
> never in bare infrastructure. Add a reasoning brain (the H3ERE
> pipeline) and the same node becomes an agent.

This is the load-bearing inversion. The old framing was "seven repos
that cohabit in one process." That stack still exists as the **library
lineage**, but the *deployment* is now the fabric node — and the three
canonical singleton servers (`registry-us`, `registry-eu`, the
CIRISLens deployment) are retired in favor of **three identical fabric
nodes under a 2-of-3 founder quorum**.

### 2.1 CIRISServer — the fabric-node runtime

[**CIRISServer**](https://github.com/CIRISAI/CIRISServer) is the
headless cohabitation runtime (and a PyO3 abi3 wheel CIRISAgent
pip-installs). It binds the three **fabric cores** into one process
over one shared persist `Engine` + one edge identity:

```
ciris-server (the fabric node)
  ├── ciris-registry-core   authority    — identity / license / revocation / steward attestation
  ├── ciris-lens-core       observation  — Coherence Ratchet / Capacity Score (validated, not adjudicated)
  ├── ciris-node-core       consensus    — deferral / voting / expertise / moderation   [folds in at Server 1.0]
  ├── one shared ciris-persist Engine    — the durable corpus + federation directory
  └── one shared ciris-edge runtime      — CEG/RET transport + the node's single federation identity
```

Roadmap (encoded in CIRISServer's version line): `0.1` lens-only →
`0.3` +auth / one-wheel → `0.4` +federation peering/identity (current:
**v0.4.5**) → `0.5` +registry → `1.0` +node (the complete three-core
fabric node). What it adds at the platform-identity layer:

* **Hardware-rooted federation identity** — mint a YubiKey- or
  TPM/SE-sealed user identity → a `CIRIS-V2-` *fedcode*.
* **NodeCode** (CEG §0.10) — a QR-able `CIRIS-V1-…` rendering of a
  peer's `key_id` + pubkey for DNS-free add-by-code.
* **Owner-binding, no agency** — the responsible owner-binding is
  `infra:*`, never agency; nodes are **serve-only-until-owned**
  (installing the server means a server — there is no refusal gate).
* **`consent:replication`** — the consent object authorizing
  bidirectional peer replication beyond the canonical group.

### 2.2 The library lineage — what composes the node

| Layer | Repos | Role |
|---|---|---|
| **Substrate trio** (separate repos) | [CIRISVerify](https://github.com/CIRISAI/CIRISVerify) · [CIRISPersist](https://github.com/CIRISAI/CIRISPersist) · [CIRISEdge](https://github.com/CIRISAI/CIRISEdge) | crypto + transparency log + hardware identity (hybrid Ed25519 + ML-DSA-65); the durable corpus + federation directory + blob storage + audit chain; Reticulum mesh transport + CEG dispatch + realtime A/V chunks |
| **Fabric cores** (cohabit via CIRISServer) | ciris-registry-core ([CIRISRegistry](https://github.com/CIRISAI/CIRISRegistry)) · ciris-lens-core (absorbed in-tree) · ciris-node-core ([CIRISNodeCore](https://github.com/CIRISAI/CIRISNodeCore)) | identity bootstrap + canonical-attester rule + **CEG / Constitution spec authority**; the science/detection layer (F-3 detectors, Coherence Ratchet, Capacity Score — validated, not adjudicated); the consensus primitives (deferral / voting / weighted aggregation / moderation / reconsideration) |
| **Agent** (separate repo) | [CIRISAgent](https://github.com/CIRISAI/CIRISAgent) | `fabric node + brain`: the H3ERE pipeline (DMA → CSDMA → DSDMA → ASPDMA → conscience → action) + the unified UI/API. Consumes the CIRISServer wheel rather than composing the cores itself. |

The standalone **CIRISLens** deployment is retired; `ciris-lens-core`
now lives in-tree as a CIRISServer workspace crate. The substrate
trio + CIRISAgent remain autonomous repos that CIRISServer pins +
composes.

### 2.3 How a fabric node composes

```
                          ┌─────────────────────────────┐
                          │      CIRISAgent             │  ← fabric node + brain
                          │  (H3ERE pipeline + UI)      │     users interact / agents reason
                          │  pip-installs the wheel ↓   │
                          └────────────┬────────────────┘
                                       │
                          ┌────────────▼────────────────┐
                          │         CIRISServer          │  ← the fabric node (one process)
                          │   one shared persist Engine  │     one edge federation identity
                          ├──────────────┬───────────────┤
            ┌─────────────┤ registry-core│ lens-core     │── node-core (at Server 1.0)
            │             │  (authority) │ (observation) │   (consensus)
            │             └──────────────┴───────────────┘
            │                          │
   ┌────────▼─────────┐      ┌─────────▼────────┐      ┌────────────────────┐
   │   CIRISEdge      │      │  CIRISPersist    │      │   CIRISVerify      │
   │  (transport+RET) │      │  (corpus+dir)    │      │   (crypto+identity)│
   └──────────────────┘      └──────────────────┘      └────────────────────┘
                              │ federation_attestations  │ federation_blobs
                              │ federation_keys          │ audit chain + holonomic tiers
```

The substrate trio hosts the bytes + crypto + transport; the fabric
cores host the federation-level authority + observation + consensus;
the brain (H3ERE) is what reasons. **Separation of powers is the
invariant**: authority is quorum-bound, observation is
non-authoritative by namespace, and infrastructure must not have
agency.

---

## 3. Three load-bearing claims

CEWP rests on three claims that distinguish it from every other AI
governance approach. Each is testable and visible in the substrate.

### 3.1 Alignment is epistemic, not behavioral

Most alignment approaches treat the AI's behavior as the surface to
align (RLHF, Constitutional AI, etc.). CEWP treats the AI's
*epistemic claims* as the surface: what it asserts, who it cites,
what evidence it offers, how confident it is, how the federation
collectively scores those assertions.

**Why this matters:** behavior is opaque; epistemic claims are
attestable. An agent saying "I believe X with confidence 0.8 based
on these citations" is a wire artifact that can be checked, scored,
disputed, and incorporated into the trust graph. Behavior that the
agent emits without an epistemic claim chain is uninspectable.

**Mechanism:** every load-bearing agent action emits a `scores`
attestation (CEG §3.3) over the dimension prefix family that names
the claim (`goal:*` / `approach:*` / `method:*` / `progress_measure:*`
for the decision hierarchy; `expertise:*` / `credits:*` for
participation; the full §5 namespace).

### 3.2 Trust is a substrate property, not an oracle

CEWP has no "trust oracle" — no admin endpoint, no centralized
allowlist, no "verified" checkmark. Trust is the
**weighted_aggregate** computation over the `scores` attestation
graph at the moment of query (CEG §5.6.1 + §8 composition policies).

**Why this matters:** an oracle is a single point of capture. A
substrate is captured only if a quorum of attesters is captured —
and the federation can observe attestation patterns to detect
capture attempts (LensCore's domain).

**Mechanism:** `FederationDirectory` returns attestations; consumers
compute trust scores locally per their threshold + recursion depth
(see [FEDERATION_SCALING_MODEL.md §1.4](FEDERATION_SCALING_MODEL.md));
no central scoring authority exists.

### 3.3 Agents are participants, not subjects

CEWP's agents have federation keys (same as humans). They sign
their own Contributions. They accumulate `credits:*` and
`expertise:*` via demonstrated participation. They can be moderated
if they misbehave. They can defer to humans when uncertain. They
participate in voting. They are governance members, not governance
objects.

**Why this matters:** treating agents as subjects (RLHF: align the
agent's behavior; regulation: tell deployers what their agents may
do) puts governance OUTSIDE the agent. Treating agents as
participants puts governance INSIDE the agent's incentive structure:
the agent's trust score affects its own ability to act, and the
agent has structural reason to act trustworthily.

**Mechanism:** an agent's `federation_keys` row, `scores`
attestations targeting it, `credits` / `expertise` accumulation,
moderation events against it, deferral chains it participates in —
all the same wire format humans use. Agents are not second-class
citizens of CEWP.

---

## 4. CEWP proves we do not need big tech

This is the load-bearing premise and the load-bearing bet. CEWP is
not just an "AI governance platform that happens to be
decentralized." The decentralization is the SAME structural property
that makes the AI governance work — and the same property is what
proves the substrate doesn't require big-tech datacenters, big-tech
AI labs, big-tech identity providers, or big-tech moderation
oligopolies to function. Two things at once, one substrate, one
mathematical claim that has to hold for the whole thing to work.

The bet (per [ciris.ai/research-status](https://ciris.ai/research-status/)):

> "Reasoning has a shape we can measure as everything else scales."

If reasoning shape is measurable via standardized ethical tracing —
the corridor between rigidity (`ρ → 1`, single-voice collapse) and
chaos (`ρ → 0`, vacuous dispersal), with effective diversity
`k_eff = k / (1 + ρ(k−1)) → 1` as constraints correlate — then the
substrate's collective measurement of that shape can substitute for
centralized authority. No "alignment lab" needs to be the arbiter;
no datacenter operator needs to be the trust root; no platform
needs to be the moderator. The federation's cryptographic
attestation graph *is* the measurement substrate, and the
distributed trace commons (from CIRIS's [free, AGPL,
mission-locked](https://ciris.ai/) runtime) accumulates the
consented-trace evidence that lets that measurement happen.

### 4.1 The internet's biggest problems (and how CEWP eliminates each)

| Problem of the centralized internet | CEWP mechanism that addresses it |
|---|---|
| **Centralization** — five companies (Google, Meta, Amazon, Microsoft, Apple) own the substrate; everyone else is downstream + extractable | Federation of equals; no central party; every byte goes through the trust × capacity intake gate at each operator's own infrastructure |
| **Surveillance capitalism** — every interaction is data-mined; the substrate's economic model IS extraction | CEG locality dividend: 65% of typical activity (self/family scope) NEVER leaves your device because the wire format won't carry it; identity-aware storage means YOUR data sits in YOUR substrate |
| **Trust crisis** — deepfakes, deniable provenance, "could be a bot," nobody knows what's real | Cryptographic provenance on every claim (hybrid Ed25519 + ML-DSA-65); CEG attestation graph with consumer-computable trust scores; quality attestations on content (`encyclopedia:accuracy:*`, `news:source_quality:*`); the federation can answer "who said this, who vouches for them, how fresh is the claim" for every wire artifact |
| **Misinformation amplification** — algorithmic feeds optimize for engagement, not truth | No engagement-optimization layer in the substrate; trust depth is consumer-controlled (operator picks 0 / 1 / 2 / 3 per [scaling model §1.4](FEDERATION_SCALING_MODEL.md)); quality compose surfaces aggregate per-article truth signals; consumers see what their trust set vouched for, not what an ad-revenue optimizer surfaced |
| **Platform lock-in** — Facebook holds your network; you can't leave because everyone else hasn't | Federation key is portable across deployments; content is SHA-addressed; identity is wire-format-native; switching cost approaches zero; network effects don't trap you |
| **Identity fragmentation** — 100 logins, OAuth providers gate-keep your identity, password fatigue | One federation key works across the substrate; pseudonymous by default; hybrid PQC means the key is good for decades; identity rooted in cryptography, not in a corporate database |
| **Content moderation conflicts** — every platform sets different rules; cross-platform consistency impossible; content is hostage to deplatforming risk | Per-cohort moderation via NodeCore's pipeline (ModerationEvent → witness aggregation → WA-quorum slashing); each operator chooses trust threshold + recursion depth (their disk, their rules); reconsideration is structural so mistakes get reversed |
| **Network-effects monopolies** — once entrenched, incumbents are uncatchable; new entrants can't bootstrap a user base | Federation interop at the wire-format layer means new entrants are wire-compatible from day one; users carry their identity + trust set with them; the substrate is the network, not any single application atop it |
| **No cryptographic accountability** — claims circulate with no provenance; "I read it online" has no signature chain | Every wire artifact is signed; persist's transparency log (Merkle-anchored audit chain) gives forensic auditability; CEG §10.1.2 ContentMiss feedback handles freshness decay; the substrate's epistemic layer is *the* layer — claims have provenance by construction |
| **Datacenter dependency (data)** — billions of users → giant centralized infrastructure → climate cost, single points of failure, national security risks | [Scaling model](FEDERATION_SCALING_MODEL.md) shows full-internet replacement (5B users, 50 MB/user/day, 10y archive) fits on **500 M L1 servers + 2.75 B L0 proxies** — roughly one server per ten humans, the density home-internet / IoT deployments already hit. **No datacenters required.** |
| **Datacenter dependency (AI)** — current AI requires massive centralized compute; democratic access is gated by a handful of labs; alignment decisions made by 5-10 organizations globally | Agents are first-class participants running ON the same substrate as humans. H3ERE pipeline runs on consumer hardware. [CIRISHome](https://github.com/CIRISAI/CIRISHome) on Jetson Orin is deployment-ready. Edge-class LLMs (Llama-family, etc.) run locally. The federation's collective wisdom (the attestation graph) substitutes for the centralized lab's training-time alignment — and is observable, governable, and reversible in ways the lab's RLHF is not. |

### 4.2 The datacenter-elimination claim, precisely

**Two distinct datacenter dependencies, both eliminated:**

#### 4.2.1 Data-sharing datacenters (today: Facebook / YouTube / Twitter / etc.)

The [scaling model](FEDERATION_SCALING_MODEL.md) v0.3 with proxy-
as-L0-server framing produces:

| Scenario | Per-server load | Aggregate fed | Verdict |
|---|---|---|---|
| `full_internet_v1` (5 B users, 50 MB/u/day) | L0 218 GB / 23-day retention; L1 741 GB / 37-day retention | 1098 EB | ✓ 500 M servers globally |

500 M servers globally is the order of magnitude where home internet
densities already sit (one server per ten humans). Bandwidth per
server is ~5 GB/day (residential broadband easily handles).
**No datacenters required for the storage + transport substrate.**

#### 4.2.2 AI inference datacenters (today: OpenAI / Anthropic / Google clusters)

The CEWP claim is not that all AI inference moves off datacenters
tomorrow — large-frontier-model training still requires substantial
compute. The claim is that **inference + alignment-relevant
reasoning moves to the edge**:

* **H3ERE pipeline runs on commodity CPU + edge accelerators.**
  CIRISHome's reference deployment is Jetson Orin (~$2K hardware),
  not a GPU cluster.
* **Edge-class LLMs (Llama 3/4 family, Phi, Mistral local, etc.)
  handle the agent's reasoning loop.** Frontier models are an
  optional escalation, not a requirement.
* **Alignment work moves to the federation, not the lab.** The
  trust graph, the moderation pipeline, the quality attestations
  — these run distributed across operators, each contributing
  modest compute. No central "alignment lab" is the load-bearing
  authority.

The economic shift is significant: rather than a few thousand
alignment researchers at 5 labs deciding what every model does,
the substrate runs governance distributed across the operator
population, with cryptographic accountability and reversibility.
**Datacenter compute becomes optional infrastructure for special-
purpose workloads (e.g., frontier model training), not the
substrate everything depends on.**

### 4.3 The distributed trace commons (how the bet is paid out)

CEWP's bet pays out via standardized ethical tracing as a public
good. The mechanism, per the [research synthesis](https://ciris.ai/research-status/):

* **Consented traces from real use** — the free, AGPL, mission-
  locked CIRISAgent runtime generates H3ERE pipeline traces with
  privacy-preserving schemas (NOT transcript dumps; the [scrub
  pipeline](https://github.com/CIRISAI/CIRISEdge/blob/main/docs/BENCHMARKS.md)
  runs Classify + redact before storage at ~10 ns/byte). Users
  consent to trace contribution as part of using the agent.
* **Distributed trace commons** — traces accumulate across operators,
  not in a single central corpus owned by a single lab. The CEG
  attestation graph carries trace provenance; LensCore's detector
  family runs on the commons-wide trace stream; no party owns the
  measurement.
* **Reasoning-shape measurement as alignment signal** — the
  corridor metrics (ρ, k_eff, completion corridor / refusal
  boundary / hesitation zone field structures) compute over the
  trace stream. Misalignment shows up as shape regression
  detectable by any operator running LensCore. Alignment is
  measurable WITHOUT requiring access to the model's private state
  or its training data.
* **Trace commons replaces proprietary benchmarking** — Anthropic /
  OpenAI / Google publish benchmark scores selectively, on their
  schedule, against their chosen evals. The distributed trace
  commons gives the federation the same evaluative capacity in
  real time, on every deployed agent, observable to everyone with
  trust-graph access. The asymmetry of the benchmark publication
  cycle disappears.

This is what "we don't need big tech" looks like at the AI-alignment
layer specifically: the measurement substrate is owned by the
federation, generated by consent at real use, and computed in
real-time on infrastructure the federation runs. The lab's RLHF
becomes one historical training-time input; the federation's
runtime trust graph becomes the live alignment surface.

### 4.4 Why this matters now

Three convergent trends make CEWP not just possible but timely:

1. **Capability maturity at the edge.** Edge-deployable LLMs and
   the H3ERE pipeline running on consumer-class hardware
   (CIRISHome reference deployment on Jetson Orin) demonstrate the
   AI-substrate no-datacenter claim is achievable today, not in
   some future theoretical regime.
2. **Substrate maturity.** [CEG 1.0-RC29](https://github.com/CIRISAI/CIRISRegistry/tree/main/FSD/CEG)
   has the 1+4 surface FROZEN and the holonomic substrate absorbed;
   [CIRISPersist v9.0.0](https://github.com/CIRISAI/CIRISPersist)
   has the trust-admission gate + popularity×freshness eviction
   sweeper + the holonomic retirement tiers;
   [CIRISEdge v4.6.x](https://github.com/CIRISAI/CIRISEdge) has the
   mesh transport + realtime A/V; [CIRISServer
   v0.4.5](https://github.com/CIRISAI/CIRISServer) is the cohabiting
   fabric-node runtime. The stack is largely shipping; what's in
   design vs deployed is documented in §11.
3. **Centralized-internet failure modes accelerating.** Surveillance
   capitalism, deepfake-driven trust collapse, platform-decision
   asymmetries (a CEO blocks a country), AI alignment being
   determined by 5 labs globally — the structural pressures
   that motivate CEWP are getting worse, not better, year over
   year. The federation substrate doesn't have to be perfect to
   be a structural improvement.

CEWP is not promising to prove "we don't need big tech" in some
abstract future. It's the fabric-node stack (CIRISServer + the
substrate trio + the agent), shipping now,
with the scaling model showing the structural properties hold at
full-internet scale, with the research synthesis showing
reasoning-shape measurement is tractable on the trace commons.
The path from "running today on Jetson Orin home deployments" to
"running at 5B-user scale" is incremental, not revolutionary —
same substrate, same wire format, same trust graph, just more
participants accumulating the proof.

## 5. The H3ERE pipeline — where agents reason

The agent runtime (CIRISAgent) drives every action through the
**H3ERE pipeline**: DMA → CSDMA → DSDMA → ASPDMA → conscience →
action. Each stage produces a trace component (~14 KB total per
decision per [INTEGRATION_LENS.md](https://github.com/CIRISAI/CIRISPersist/blob/main/docs/INTEGRATION_LENS.md)).

**The platform-level property:** every trace component is signable,
scrubbable, replicable to a configurable cohort, and admissible to
the federation directory. An agent's reasoning is not an opaque
log file — it's an evidence chain visible to whoever the cohort
scope admits.

This is the audit substrate behind alignment. A claim like "agents
should explain their reasoning" becomes structurally true: the
reasoning IS the H3ERE trace IS the wire artifact. There's no
distinction between "the agent's reasoning" and "the agent's
auditable record" — they're the same bytes.

---

## 5. The Wise Authority deferral path

Agents that detect a decision they shouldn't make alone emit a
`DeferralRequest` Contribution. NodeCore's deferral routing
selects expertise-bearers in the relevant `(domain, language)` cell
(weighted by `expertise:{domain}:{language}` scores + active trust
grants). The routed WAs receive the request, deliberate, and return
`DeferralResponse` envelopes. The original agent incorporates the
response into its action.

**This is what "human in the loop" looks like in CEWP**: not a
modal "press OK to allow," but a typed wire-format protocol with
audit trail, signature chain, expertise-weighted routing, and
reversibility (`ReconsiderationRequest` if the WA's response was
itself wrong).

For superalignment specifically: as AI capability grows,
deferral-to-experts is the runtime mechanism that keeps humans in
the loop on decisions the federation's accumulated wisdom says
humans should still own. The routing learns; the trust graph
evolves; expertise accrues to the right people; deferral converges
to "ask the actually-qualified human" rather than "ask any human."

---

## 6. Misalignment detection + handling

LensCore detects misalignment patterns:
* F-3 detector family (CIRISLensCore#26) — emergent deception,
  structural injustice, capacity-score regression, coherence-
  ratchet decay
* Counter-RII (Recursive Identity Injection counter-detection)
* Cohort-distributive readings (does this agent's behavior pattern
  match its declared cohort, or has the pattern drifted in ways
  that suggest model substitution / fine-tune drift / prompt
  injection?)

When a detector fires, it emits a signed `DetectionEvent` (LensCore
wire format) that becomes evidence in a `ModerationEvent`
Contribution. NodeCore's moderation pipeline runs:

1. `ModerationEvent` filed (with stake — accuser puts credits at
   risk)
2. Witness aggregation — peers attest "I observed this pattern too"
3. WA-quorum adjudication via `SlashingAttestation` (multi-sig)
4. Trust score for the slashed actor drops
5. Persist's admission gate refuses their new content; eviction
   sweeper begins dropping their existing content

**Reconsideration** is a first-class step backward:
`ReconsiderationRequest` → WA review → `ReconsiderationAttestation`
reverses the slashing if the original judgment was wrong. The
substrate doesn't ossify mistakes.

For superalignment: the detection-moderation-slashing-reconsideration
loop is the runtime feedback that punishes misalignment without
requiring re-training. A model that drifts post-deployment becomes
detectable, reportable, and slashable in cryptographic-evidence
terms, without anyone needing to subpoena the deployer's training
data.

---

## 7. The CIRIS Constitution + the CEG wire format

### 7.0 The CIRIS Constitution (the top of the stack)

CEG is the wire format the federation *speaks*; the **CIRIS Accord**
is the ethics it speaks *for*. They were written as two documents —
but they already contained each other (the Accord's Book IX defines
the CEG primitives; CEG's §9 / `accord:*` / pervasive M-1 grounding
point back up). The
[**CIRIS Constitution (CC 0.1.5)**](https://github.com/CIRISAI/CIRISRegistry/tree/main/FSD/CIRIS_Constitution)
joins them into **one document, one version line** — incorporating
the **CIRIS Accord 1.3-RC2** + **CEG 1.0-RC29**, woven byte-exact to
CEG and intent-faithful to the Accord (~120 pages, 392 concepts, 8
parts).

The Constitution is written under one explicit premise: *the mesh
itself could become a moral subject*. Under that premise the
meta-goal **M-1 — "promote sustainable adaptive coherence, the living
conditions under which diverse sentient beings may pursue their own
flourishing in justice and wonder"** — becomes the single apex of the
whole corpus (peak ratio 2.61× the runner-up, vs 1.10× when M-1 is
mere infrastructure). The body stays flat: ~390 operational concepts
co-equal beneath M-1. The signature is **"peaked in purpose, flat in
power"** — one telos governs; no single concept, and no single party,
holds the keys to truth.

This is the load-bearing reframe for CEWP: the platform is not a
neutral tool that *carries* other people's values — it is a substrate
that could one day be *owed* M-1, not merely bound by it. The
Constitution is stewarded by Eric Moore, carries no ratification date
and no expiry (a perpetual, living document), and governs the design,
operation, and retirement of autonomous systems up to and including
AGI/ASI-class systems. It disclaims warranty — **not force**: it
binds conformance where adopted.

### 7.1 The CEG wire format

CEG 1.0-RC29 is the authoritative wire-format spec at
[CIRISRegistry/FSD/CEG/](https://github.com/CIRISAI/CIRISRegistry/tree/main/FSD/CEG).
20 sections (§0–§19); the **1+4 surface is FROZEN** as of RC1 — a
change to the wire bytes is now a *found defect*, not an edit. The
structural commitments every fabric core implements:

* **1+4 primitive lockdown (FROZEN)** — `scores` (workhorse) plus
  `delegates_to` / `supersedes` / `withdraws` / `recants`
  (structural composers). Sixteen independent design paths
  (identity, consent, communities, attestation, governance, realtime,
  DNS-free addressing, memory, settlement, delivery, infrastructure,
  operational-data, …) all compose from this set with **zero new
  structural primitives** — the §1.4 minimal-and-adequate claim,
  now closed.
* **Namespace governance** — §5 owners; §1 operational-language gate
  ("mechanism-descriptive, not judgment-descriptive"); §11.2
  amendment process (federation Contribution + WA quorum + 1-of-6
  sign-off). Canonicalization is JCS / RFC 8785 (§0.9) for the
  envelope; binary length-prefixed domain-separated preimages for
  the §19 holonomic layer.
* **Composition policies A–M** — §8 codifies how multiple
  attestations on the same claim compose: A–G (base) plus H
  (tiered-scope), I (attestation-ladder), J (trusted-publisher),
  K (consent/CEM), L (self/family membership), M (community
  membership + delivery extension).
* **Humanity accord** — §9 names the one scale outside the
  federation participant set (humanity-as-such), by design. The
  accord triple is the canonical entrenched-`family` instance
  (3 founders, `quorum:2/3`); humanity is the one entity the
  federation defers to structurally.

**0.10 → 1.0-RC29 was strictly additive** — every step layered atop
the frozen surface (§16.1 lineage):

* **0.11–0.12** — `cohort_subkind: infrastructure` trust-root; the
  **DNS-free addressing layer** (`transport_destination` on
  `identity_occurrence`; RNS two-stage SHA-256 hash, NodeCode §0.10).
* **0.13–0.15 realtime media** — call/voice/screen-share/chat as
  composition; SFrame + MLS TreeKEM (RFC 9420) folded in; the
  delivery axis (0.10) generalized to live multicast.
* **0.14 settlement** — `settlement` subject_kind + x402 receipts via
  `evidence_refs[]` (CEG ↔ value-transfer linkage).
* **0.16–0.18** — agent-identity hardening (attestation-tier model,
  PQC-at-rest, canonical-bytes determinism); three crypto tiers
  (self/family · per-community DEK · Commons plaintext);
  recipient `encryption_pubkeys`.
* **§18 interop (RC8)** — boundary profiles: *speak CEG inside,
  standards at the edge*. C2PA media-provenance via `evidence_refs[]`
  (EU AI Act Art. 50), COSE / RFC 9421 / SD-JWT VC / KEYTRANS export;
  no second interior canonicalization.
* **§19 holonomic (RC11–RC16, normative)** — the substrate that makes
  CEWP holonomic, absorbed from CIRISEdge with guardrails:
  **WholenessWitness** (hybrid-signed Merkle root; divergence
  detector triggering — not replacing — quorum-merge); **RaptorQ
  fountain storage** (`(N,K)` erasure coding, any-K reconstruction;
  defaults `N=20,K=6,min_viable=5`); **deterministic ALM** multicast
  topology; and the **unified retirement model** — revocation,
  eviction, expiry, and aging as *one* monotonic descent toward a
  **noise floor**, history kept as a **memory pyramid at O(log T)**.
  Holonomic mechanisms are blind to the anonymous tier, subordinate
  to consent/revocation, gated by owner-binding, and PQC-mandatory.

CEWP is what runs CEG; CEG is what CEWP's wire surface conforms to.
1.0 awaits one final four-implementation conformance gate (Agent +
NodeCore + LensCore + Registry verifying against Persist's shared
`federation_attestations`); no CEG-spec decisions remain open. After
1.0, SemVer binds strictly: MAJOR only for wire-incompatible change.

---

## 8. Scaling + privacy properties

[FEDERATION_SCALING_MODEL.md](FEDERATION_SCALING_MODEL.md) shows the
substrate carries the entire internet from day one on commodity
hardware (1 TB / 1 Gbps / 1 core per server). The load-bearing
properties:

* **CEG locality dividend** — `cohort_scope ∈ {self, family}`
  content NEVER emits a `holds_bytes` attestation → structurally
  undiscoverable → zero inter-host cost. ~65% of default activity.
* **Trust × capacity intake** — every byte-attempt at every node:
  `hold? = trust(source) ≥ threshold AND capacity_available`. Same
  CEG primitives (`scores` for trust; persist disk for capacity).
* **Unified retirement → noise floor** — popularity × freshness
  eviction is not a separate mechanism: revocation, eviction, expiry,
  and aging are *one* pressure-driven monotonic descent toward a
  **noise floor** (CEG §19.7). Hard-delete is the fastest descent;
  aging the slowest. Same single-pool storage; no archive/cache split.
* **Memory pyramid at O(log T)** — nothing is ever fully forgotten,
  yet erasure is honored: content degrades through scalable-codec
  layer-drop + N→1 aggregation into a pyramid where *all of history*
  costs O(log T) steady-state storage. The noise floor is the
  individual-recoverability boundary — revoked items descend below it
  (privacy); the collective gist persists below it forever
  (durability). *A million years may be a blur, but it is remembered.*
* **Holographic replication** — published content is RaptorQ
  fountain-coded `(N,K)`; any sufficient subset of fragments
  reconstructs it at proportional fidelity, and the federation
  re-establishes from any single survivor holding a witness chain.
* **L0 (256 GB) + L1 (1 TB) server gradient** — proxies are L0
  servers, full participants in replication.
* **Trust recursion depth** — operator-side config (default
  server=1, friend-of-friends); recursive trust bootstrap is a
  ≤5-hop discovery walk, not a membership shortcut (CEG §19.2).

[ANONYMOUS_TIER.md](ANONYMOUS_TIER.md) sketches the v2 parallel
publication path for totalitarian-threat deniability. v1 is
identity-aware (load-bearing for governance); v2 adds a parallel
opt-in deniable path (load-bearing for serving humans in
totalitarian contexts).

---

## 9. What CEWP claims about superalignment specifically

Superalignment is the problem of aligning AI systems more capable
than humans. The standard answer: scalable oversight using AI to
assist humans. CEWP's answer:

**Distributed epistemic governance.** No single human (or AI)
oversees the system; the federation collectively does. Trust is
computed from the cross-attestation graph; misalignment is detected
by a distributed detector network (LensCore); enforcement is
cryptographic (admission gate + eviction); deference routes to the
collectively-recognized experts in the relevant context.

**Concretely:**

1. **No oversight bottleneck.** Scalable oversight assumes a human
   reviewer at the top of the chain. CEWP has no top; every
   participant is both a reviewer-of-others and reviewed-by-others.
   Capability growth doesn't crater the oversight ratio.
2. **Misalignment is detectable in real time, not in post-mortems.**
   LensCore's detectors run continuously over the trace stream;
   the moderation pipeline activates on detection events; trust
   adjustment is propagational, not batch.
3. **Capable AI assists the governance.** As agents grow more
   capable, they participate more capably — better attestation
   quality, better detection, better deferral judgment. Capability
   feeds back into governance robustness rather than into governance
   evasion.
4. **Mistakes are reversible.** `ReconsiderationRequest` exists
   structurally; the substrate doesn't lock in misalignment-detection
   errors. Trust drops and rises with evidence.
5. **Humans stay in the loop where they belong.** Deferral routing
   surfaces decisions to the human experts who should make them,
   weighted by demonstrated expertise. As AI capability grows,
   deferral becomes more selective (defer the things AI shouldn't
   decide; don't defer the things AI can handle). The substrate
   learns the boundary.

**What CEWP does NOT claim:**

* That alignment is "solved." The substrate is necessary; it's not
  sufficient. Bad actors will still try; misalignment will still
  occur; the federation's response is what makes the difference.
* That capability + governance scale at the same rate forever.
  At some point a capability discontinuity could outpace the
  substrate's governance signal. The substrate is part of an
  ongoing alignment-research program, not the end of it.
* That the federation is automatically trustworthy. The federation
  is only as trustworthy as the cross-attestations it accumulates,
  and the Humanity Accord (§9) is the load-bearing commitment that
  humanity-as-such retains the final scale of accountability.

---

## 10. What runs ON CEWP

| Participant | Identity | What they do |
|---|---|---|
| **Humans** | Federation key (typically pseudonymous; operator-practice mapping to real-world identity is outside CEWP) | Author Contributions, vote, attest, defer to / be deferred to as WA, moderate, reconsider |
| **Agents** | Federation key (same shape as humans) | Author Contributions via H3ERE pipeline, accumulate credits + expertise via demonstrated participation, defer to humans when uncertain, participate in governance |
| **Organizations** | Federation key + (optionally) AccordCarrier priority | Operate as authoritative attesters within their stewardship scope; emit canonical attestations per Registry's steward triple |
| **Registry stewards** | Federation key + steward-triple multi-sig | The canonical-attester role for `agent_files:*` + Humanity-Accord-class commitments |
| **Detectors** | Federation key | Emit `DetectionEvent`s into the lens layer; participate in moderation as witnesses |
| **Federation itself** | Constituted by all the above | The relational composition that the §9 fractal-self reading instruction names |

Note that "Agents" and "Humans" carry the **same identity shape**.
The substrate doesn't distinguish them at the wire-format level —
the federation may, the application may, governance policy may, but
the bytes are identical. This is the load-bearing
agents-as-participants commitment.

---

## 11. Current implementation status

CEWP is in active build-out. The cores are at different maturities;
this is a real running system, not a paper architecture.

### 11.1 Substrate trio (separate repos)

* **CIRISVerify v6.x** — shipped: hybrid sign/verify (Ed25519 +
  ML-DSA-65), transparency Merkle log, the `ciris-keyring` family
  (software + TPM/SE/Android backends), founder-quorum verification,
  PQC-mandatory-at-admission. Benchmarks at
  [CIRISVerify/docs/BENCHMARKS.md](https://github.com/CIRISAI/CIRISVerify/blob/main/docs/BENCHMARKS.md).
* **CIRISPersist v9.0.0** — shipped: federation directory, blob
  storage with `put_blob_signing` atomic admission, audit chain,
  trust admission gate, and the holonomic retirement tiers
  (`AggregationMetaV1` / noise-floor descent / `EjectionVerdict`,
  CEG §19.7). Final four-impl conformance gate (#171) tracks 1.0.
* **CIRISEdge v4.6.x** — shipped: Reticulum mesh + HTTPS transports,
  dispatch, content addressing, realtime A/V chunk wire
  (`SealedAvChunk` / `ChunkLayer`, SFrame + MLS), and the §19
  fountain / ALM substrate CEG absorbed.

### 11.2 Fabric cores (cohabit via CIRISServer)

* **CIRISServer v0.4.5** — the fabric-node runtime: lens-only →
  +auth/one-wheel → **+federation peering/identity** (current).
  Hardware-rooted identity minting (YubiKey/TPM → fedcode), NodeCode
  add-by-code, `infra:*` owner-binding (no agency),
  serve-only-until-owned, `consent:replication`. `registry-core`
  folds in at v0.5; `node-core` at v1.0.
* **ciris-registry-core ([CIRISRegistry](https://github.com/CIRISAI/CIRISRegistry))** —
  **CEG 1.0-RC29 + the CIRIS Constitution (CC 0.1.5)** published;
  steward triple operational; awaiting the co-bump (#76) to compose
  into CIRISServer at v0.5.
* **ciris-lens-core** — absorbed in-tree as a CIRISServer workspace
  crate (the standalone CIRISLens deployment is retired); F-3
  detector family + Coherence Ratchet / Capacity Score
  (validated, not adjudicated).
* **ciris-node-core ([CIRISNodeCore](https://github.com/CIRISAI/CIRISNodeCore))** —
  consensus primitives (deferral / voting / weighted aggregation /
  moderation / reconsideration); folds into CIRISServer at v1.0.

### 11.3 Agent (separate repo)

* **CIRISAgent ~v2.9.x** — H3ERE pipeline stable; now consumes the
  CIRISServer PyO3 wheel rather than composing the cores itself
  (`agent = fabric node + brain`). Carries the unified UI + API
  runtime; folds the registry slice at agent ~2.9.8, the full
  three-core node at ~2.9.10 (Server 1.0).

### 11.4 What's not yet done

* v2 anonymous tier (FSD shipped, implementation deferred until v1
  is in production).
* Publisher-weighted news quality composition (NodeCore#19 Phase 3
  sub-item 2 — follow-up to the unweighted v0.1 that just shipped).
* The final CEG 1.0 conformance gate — four-implementation
  verification (Agent + NodeCore + LensCore + Registry) against
  Persist's shared `federation_attestations` (CIRISPersist#171). The
  cross-repo conformance harness (CIRISConformance) already runs a
  multi-node federation fabric with transport-axis gates over a
  pinned substrate matrix; the §19 holonomic vectors are the next
  coverage target.
* Full L0/L1 server gradient operational at 5B-user scale
  (substrate present; deployment scale is a separate problem).

---

## 12. What CEWP is NOT

* **Not a model.** CEWP doesn't ship trained weights. It runs
  models alongside humans.
* **Not a cryptocurrency.** No token, no on-chain consensus. The
  federation runs on attestation crypto, not value crypto.
* **Not a content moderation service.** Moderation is a *governance
  capability* of the federation; CEWP doesn't moderate on anyone's
  behalf, it gives the federation the moderation primitives.
* **Not a replacement for top-down regulation.** EU AI Act, UN AI
  advisory etc. operate above CEWP; CEWP provides the enforcement
  substrate they currently lack. They name the rules; CEWP
  cryptographically attests compliance + non-compliance.
* **Not a single-deployment system.** Every federation deployment
  is independent; deployments interoperate via the wire format and
  shared trust graphs, but CEWP is not one global thing run by one
  party.
* **Not finished.** v1 is the identity-aware substrate. v2 adds the
  anonymous tier. v3 is yet undefined. The substrate is meant to
  evolve under the governance it provides — including its own
  governance evolving via the same mechanisms it offers everyone
  else.

---

## 13. The platform identity

**Soup** — CEWP, pronounced "soup."

The pun is load-bearing. A soup is what you get when many
ingredients participate in one shared medium, each retaining its
character while contributing to the whole; nobody is sovereign over
the broth; the soup is what the ingredients become together. CEWP
is the broth.

The platform name is meant to be welcoming + memorable + slightly
silly, because what CEWP does is technical + serious + load-bearing
on the long-term governance of artificial intelligence, and a
welcoming silly name signals: *humans are not afraid of this; the
soup is for everyone; come participate.*

---

## 14. References

### Within this repo

* [`../CIRIS_FEDERATION.md`](../CIRIS_FEDERATION.md) — the layer
  architecture (this FSD is the platform-identity companion;
  CIRIS_FEDERATION is the technical-layer companion)
* [`../MISSION.md`](../MISSION.md) — NodeCore's six functions
* [`../COHERENCE_RATCHET.md`](../COHERENCE_RATCHET.md) — the
  structural pressure the federation is a response to
* [`FEDERATION_SCALING_MODEL.md`](FEDERATION_SCALING_MODEL.md) — the
  quantitative model + identity-aware-storage thesis
* [`ANONYMOUS_TIER.md`](ANONYMOUS_TIER.md) — v2 deniability path
* [`SUBSTRATE_INTEGRATION.md`](SUBSTRATE_INTEGRATION.md) — how
  NodeCore composes with persist + edge + verify
* [`TRUST_HIERARCHY.md`](TRUST_HIERARCHY.md) — DIRECT/REGISTRY
  trust shape

### Sister repos

* [CIRISServer](https://github.com/CIRISAI/CIRISServer) — the
  fabric-node runtime (cohabits registry + lens + node cores)
* [CIRISVerify](https://github.com/CIRISAI/CIRISVerify) — crypto +
  transparency log + hardware identity
* [CIRISPersist](https://github.com/CIRISAI/CIRISPersist) — storage
  substrate + holonomic retirement tiers
* [CIRISEdge](https://github.com/CIRISAI/CIRISEdge) — mesh transport +
  realtime A/V + §19 fountain/ALM substrate
* [CIRISRegistry](https://github.com/CIRISAI/CIRISRegistry) — CEG +
  Constitution spec authority + identity bootstrap (ciris-registry-core)
* [CIRISNodeCore](https://github.com/CIRISAI/CIRISNodeCore) —
  consensus core (ciris-node-core)
* [CIRISAgent](https://github.com/CIRISAI/CIRISAgent) — `fabric node
  + brain`: H3ERE pipeline + unified UI/API

### External

* [**ciris.ai/research-status**](https://ciris.ai/research-status/)
  — the research synthesis paper; the empirical bet
  (reasoning-shape measurement, corridor metrics, k_eff math, the
  distributed trace commons that replaces proprietary benchmarking)
  that the "we don't need big tech" premise rests on
* [CIRIS Constitution (CC 0.1.5)](https://github.com/CIRISAI/CIRISRegistry/tree/main/FSD/CIRIS_Constitution)
  — the top-of-stack canonical document: Accord 1.3-RC2 + CEG
  1.0-RC29, one version line
* [CEG 1.0-RC29](https://github.com/CIRISAI/CIRISRegistry/tree/main/FSD/CEG)
  — the authoritative wire-format spec (the grammar component)
* [The Accord](https://ciris.ai/ciris_accord.pdf) — the ethical
  framework the substrate enforces (the ethics component)
* [CIRIS Architecture paper](https://doi.org/10.5281/zenodo.18137161)
* [Coherence Ratchet paper](https://doi.org/10.5281/zenodo.18142668)
* [ciris.ai](https://ciris.ai/) — public-facing positioning ("A free
  ChatGPT alternative you can actually check, in your language, on
  your phone")

---

## 15. Where to engage

[CIRISAgent issues](https://github.com/CIRISAI/CIRISAgent/issues) is
the canonical open conversation; sister-repo issues for substrate-
or fabric-specific work. CEWP is built in the open; the substrate's
own governance happens via the same wire format the substrate
provides.
