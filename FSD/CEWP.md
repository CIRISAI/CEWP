# FSD: CEWP — CIRIS Epistemic Web Platform

**Pronunciation:** "soup."
**Status:** v0.3 — synced to the CIRIS Constitution 1.0-rc9 (tag
  `v1.0-rc9`). CEG is no longer a separate spec version line: the wire
  grammar is Parts 2–5 of CC. Fabric-node / holonomic reframe. Where
  this document and CC disagree, CC wins.
**What CEWP is:** the platform that the CIRIS federation composes —
  a **holonomic epistemic web** with no center, no DNS, and no
  load-bearing server, the runtime AI agents live in, and an AI
  governance / superalignment substrate where alignment is treated as
  an *epistemic-governance* problem with cryptographic accountability —
  not a training-time problem to be solved inside a model. The whole
  web speaks one small grammar (the "1+4" of five row types, CC 2.4);
  every node is a **fabric
  node** (`agent = fabric node + brain`); authority roots in
  accountable humans, never in bare infrastructure. CEWP is the
  platform identity for what was the seven-repo CIRIS Agent 3.0 stack,
  now deployed as cohabiting fabric-node cores (CIRISServer) + the
  substrate trio + the agent.
**Companion canonical docs:**
* [`CIRISConstitution`](https://github.com/CIRISAI/CIRISConstitution)
  — the **CIRIS Constitution** (CC 1.0-rc9, tag `v1.0-rc9`): the
  ethics and the wire grammar in one versioned document. Parts 2–5
  are the wire grammar formerly published separately as CEG. The
  top-of-stack canonical authority.
* [`CIRIS_FEDERATION.md`](../CIRIS_FEDERATION.md) — the layer
  architecture (identity / event / federation / verification /
  safety / lens / persistence / consensus)
* [`MISSION.md`](../MISSION.md) — NodeCore's six functions
* [`FEDERATION_SCALING_MODEL.md`](FEDERATION_SCALING_MODEL.md) —
  what carrying the entire internet costs at v1
* [`ANONYMOUS_TIER.md`](ANONYMOUS_TIER.md) — the planned opt-in GPA-unobservability (Sphinx) tier, named by CC 1.13.3.1 for deployments that need GPA resistance; not yet specified in CC and not yet shipped. Anonymity toward outsiders at the smallest cohort scope is the default (CC 1.13.3.4)

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
Content is **holographic**: each layer of an item is RaptorQ-coded
on its own, so shedding the high-detail layers leaves a coarse
version of the same item, and the witnessed corpus re-establishes
from any sufficient fragment (CC 6.1). The substrate eliminates the centralized
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
   (the five row types of CC 2.4: `scores` + `delegates_to` /
   `supersedes` / `withdraws` / `recants`; the slot is closed to the
   five). What gets asserted, by whom, at what scope, is visible to
   any reader the row's scope and that reader's own trust or consent
   admit (CC 4.4.3.8, CC 2.4.1.2.1).
3. **Trust is computed from the attestation graph** — an
   aggregation over `scores` rows (CC 4.4.2) under a composition
   policy the reader chose (CC 4.4.3). No "is this peer trusted"
   oracle; only "what do the attesters this reader trusts say about
   this peer at this scope, right now."
4. **Harmful acts are adjudicated on the record**: a
   `moderation:{allegation_type}` ModerationEvent (CC 3.1.9.2), filed
   under a delegated `moderate` duty, can lead to a `slashing:{outcome}`
   verdict and a `quarantine:{state}` marker from a holder of the
   delegated `slash` duty (CC 4.5.5). Detector flags are never sole
   evidence for a slashing (CC 3.1.6). The platform has enforcement,
   not just a regulator's text.
5. **Reconsideration is first-class**: the subject of a row holds
   standing to file `reconsideration:{grounds}`, heard under the
   delegated `review` duty, with outcome `reversed` / `partial` /
   `upheld` (CC 3.1.9.2, CC 4.5.5); the substrate doesn't ossify
   mistakes.
6. **Agents that don't know what to do *defer*** — a
   `deferral_request` Contribution routes a single ask within a
   (domain, language) cell (CC 3.1.9.3) to a Wise Authority (CC 4.3);
   the agent awaits guidance and stays inactive on that issue
   (CC 1.9). Deference is a first-class wire act with audit trail.
7. **Decisions decompose into a typed hierarchy**:
   `Goal → Approach → Method → Progress Measure`, all wire-format-
   typed (CC 3.1.9.7). An agent's reasoning is
   not opaque text; it's a typed graph of attestable steps.
8. **Privacy and scale share one primitive** — `cohort_scope`. Local
   data (self/family) is structurally undiscoverable to outsiders
   (CC 5.2); published
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
lineage**, but the *deployment* is now the fabric node — and the
canonical singleton servers (`registry-us`, `registry-eu`, the
CIRISLens deployment) are retired; the GenesisBundle replaced the
per-region steward set (CC 5.3.4). The three canonical installs
`ciris-canonical-{1,2,3}` are members of the `ciris-canonical`
infrastructure community, whose founders are the three accord
holders; the installs distribute serving, not authority (CC 3.2).
`ciris-canonical` is the shipped default trust anchor, not a forced
root: a node may re-root or run with none, and any operator may found
a governed trust-root community of their own (CC 3.2).

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
`0.3` +auth / one-wheel → `0.4` +federation peering/identity → `0.5`
**config-as-CEG + registry** (current: **v0.5.232**) → `1.0` +node
(the complete three-core fabric node). What it adds at the platform-identity layer:

* **Hardware-rooted federation identity** — mint a YubiKey- or
  TPM/SE-sealed user identity, rendered as a *FedCode*: the one
  human-shareable, kind-tagged, checksummed render of a key
  (CC 2.6.8).
* **FedCode add-by-code** (CC 2.6.8) — QR-able and DNS-free; the v1
  NodeCode (`CIRIS-V1-…`) is retained as `kind: node`.
* **Owner and steward binding, no agency** — a node has no agency and
  gives no acceptance (CC 3.1.3.2); its stewardship reach is `infra:*`
  only, never agency (CC 3.2). Its owner is a single `user` named by a
  live `delegates_to(user → key, delegation_purpose: owner_binding)`,
  and a second owner-binding naming a different owner is refused at
  bind time (CC 3.2 single-owner invariant). A `node` or `agent` key
  MUST have a bound steward before admission to any
  non-`infrastructure` community, and MAY trust and serve
  `infrastructure` communities with no steward (CC 3.2 steward-binding
  gate). A new device joining its owner is unowned but never
  anonymous: the first-contact link binds its self-signed node key
  (CC 3.2 (e)). A node that holds a `serve` seat in an
  `infrastructure` community MUST be announced by its owner
  (CC 5.4.6).
* **`consent:replication:{version}`** — a node's directed,
  revocable grant to replicate a named prefix set to a specific
  federation peer (CC 3.3.7).

### 2.2 The library lineage — what composes the node

| Layer | Repos | Role |
|---|---|---|
| **Substrate trio** (separate repos) | [CIRISVerify](https://github.com/CIRISAI/CIRISVerify) · [CIRISPersist](https://github.com/CIRISAI/CIRISPersist) · [CIRISEdge](https://github.com/CIRISAI/CIRISEdge) | crypto + transparency log + hardware identity (hybrid Ed25519 + ML-DSA-65); the durable corpus + federation directory + blob storage + audit chain; Reticulum mesh transport + CEG dispatch + realtime A/V chunks |
| **Fabric cores** (cohabit via CIRISServer) | ciris-registry-core ([CIRISRegistry](https://github.com/CIRISAI/CIRISRegistry)) · ciris-lens-core (absorbed in-tree) · ciris-node-core ([CIRISNodeCore](https://github.com/CIRISAI/CIRISNodeCore)) | identity bootstrap + the trust-root discovery endpoints (the GenesisBundle; the registry is a conferred slice of a canonical node, not a root, CC 5.3.4); the science/detection layer (F-3 detectors, Coherence Ratchet, Capacity Score — validated, not adjudicated); the consensus primitives (deferral / voting / weighted aggregation / moderation / reconsideration) |
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
invariant**: a group's act carries its members' signatures on the
same row (CC 2.4), observation is advisory (detector and
`ratchet:flag:*` rows are never sole evidence for a slashing,
CC 3.1.6), and infrastructure has no agency (CC 3.2).

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
row (CC 2.4.2) over the dimension prefix family that names
the claim (`goal:*` / `approach:*` / `method:*` / `progress_measure:*`
for the decision hierarchy; `expertise:*` / `credits:*` for
participation; the full namespace is CC Part 3).

### 3.2 Trust is a substrate property, not an oracle

CEWP has no "trust oracle" — no admin endpoint, no centralized
allowlist, no "verified" checkmark. Trust is an aggregation over
the `scores` graph at the moment of query, computed by each reader
(CC 4.4.2 aggregation, CC 4.4.3 composition policies).

**Why this matters:** an oracle is a single point of capture. Here
influence is gated by trust, not by registration: a fresh key has no
weight until someone the consumer trusts confers it (CC 1.13.3.3).
The limit is stated in CC: the default aggregation for `signed`
dimensions is a plain mean inside the trust set, so one captured
trusted attester moves it (CC 1.13.3.3; the R14 bet, CC 8.3.1).
The federation can observe attestation patterns to detect capture
attempts (`ratchet:flag:coordinated_voting_cluster`, CC 3.1.6;
LensCore's domain).

**Mechanism:** `FederationDirectory` returns attestations; consumers
compute trust scores locally per their threshold + recursion depth
(see [FEDERATION_SCALING_MODEL.md §1.4](FEDERATION_SCALING_MODEL.md));
no central scoring authority exists.

### 3.3 Agents are participants, not subjects

CEWP's agents have federation keys (same as humans). They sign
their own Contributions. They accumulate `credits:*` and
`expertise:*` via demonstrated participation. They can be moderated
if they misbehave. They can defer to humans when uncertain. They do
not vote: one human voice counts once, and an agent has no vote
(CC executive summary, promise 6). Every agent key has a bound human
steward who answers for it (CC 3.2 steward-binding gate).

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
| **Surveillance capitalism** — every interaction is data-mined; the substrate's economic model IS extraction | CEG locality dividend: 65% of typical activity is self/family scope, and no holding claim for it leaves its cohort because the wire format won't carry it (CC 5.2); private content is never scanned (CC 1.13.3.4); identity-aware storage means YOUR data sits in YOUR substrate |
| **Trust crisis** — deepfakes, deniable provenance, "could be a bot," nobody knows what's real | Cryptographic provenance on every claim (hybrid Ed25519 + ML-DSA-65); CEG attestation graph with consumer-computable trust scores; quality attestations on content (`encyclopedia:accuracy:*`, `news:source_quality:*`); the federation can answer "who said this, who vouches for them, how fresh is the claim" for every wire artifact |
| **Misinformation amplification** — algorithmic feeds optimize for engagement, not truth | No engagement-optimization layer in the substrate; trust depth is consumer-controlled (operator picks 0 / 1 / 2 / 3 per [scaling model §1.4](FEDERATION_SCALING_MODEL.md)); quality compose surfaces aggregate per-article truth signals; consumers see what their trust set vouched for, not what an ad-revenue optimizer surfaced |
| **Platform lock-in** — Facebook holds your network; you can't leave because everyone else hasn't | Federation key is portable across deployments; content is SHA-addressed; identity is wire-format-native; switching cost approaches zero; network effects don't trap you |
| **Identity fragmentation** — 100 logins, OAuth providers gate-keep your identity, password fatigue | One federation key works across the substrate; pseudonymous by default; hybrid PQC means the key is good for decades; identity rooted in cryptography, not in a corporate database |
| **Content moderation conflicts** — every platform sets different rules; cross-platform consistency impossible; content is hostage to deplatforming risk | Per-cohort moderation as a delegable duty (`moderate` / `takedown` / `review` / `slash`, CC 4.5.5); every community has a named moderator with merit auto-promotion (CC 4.5.4); no hand bans a community (CC executive summary, promise 6); each operator chooses trust threshold + recursion depth (their disk, their rules); reconsideration is structural so mistakes get reversed |
| **Network-effects monopolies** — once entrenched, incumbents are uncatchable; new entrants can't bootstrap a user base | Federation interop at the wire-format layer means new entrants are wire-compatible from day one; users carry their identity + trust set with them; the substrate is the network, not any single application atop it |
| **No cryptographic accountability** — claims circulate with no provenance; "I read it online" has no signature chain | Every wire artifact is signed; persist's transparency log (Merkle-anchored audit chain) gives forensic auditability; ContentMiss feedback handles freshness decay (CC 5.3.2.1); the substrate's epistemic layer is *the* layer — claims have provenance by construction |
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
2. **Substrate maturity.** The [CIRIS Constitution 1.0-rc9](https://github.com/CIRISAI/CIRISConstitution)
   carries the wire grammar (Parts 2–5) with the row-type slot closed
   to the five (CC 2.4) and the holonomic substrate as CC 6.1;
   [CIRISPersist v59.1.0](https://github.com/CIRISAI/CIRISPersist),
   [CIRISEdge v43.1.1](https://github.com/CIRISAI/CIRISEdge) and
   [CIRISServer v0.5.232](https://github.com/CIRISAI/CIRISServer) are
   the current substrate and fabric-node releases. The CC executive
   summary marks each of its eleven promises as working today, being
   built, a rule with no code yet, or modelled; the halt is measured
   end to end (promise 4). What's in design vs deployed is in §11.
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
`deferral_request` Contribution, which routes a single ask within a
cell (CC 3.1.9.3). NodeCore's deferral routing selects
expertise-bearers in the relevant `(domain, language)` cell (weighted
by `expertise:{domain}:{language}` scores, CC 3.1.9.6, + active trust
grants). The routed Wise Authorities (CC 4.3) deliberate and answer;
until a named human answers, the agent stays inactive on that issue
(CC 1.9; CC executive summary, promise 7). The Wise publish their
rulings and are readable at the same grain as everyone else (CC 4.3;
promise 8).

**This is what "human in the loop" looks like in CEWP**: not a
modal "press OK to allow," but a typed wire-format protocol with
audit trail, signature chain, expertise-weighted routing, and
reversibility (`reconsideration:{grounds}`, CC 3.1.9.2, if the WA's
response was itself wrong).

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

When a detector fires, it emits a `detection:*` row (CC 3.4.8) that
may be cited as evidence in a `moderation:{allegation_type}`
ModerationEvent (CC 3.1.9.2). Detector and `ratchet:flag:*` rows are
never sole evidence for a slashing; WA quorum is the gate (CC 3.1.6).
Per the CC executive summary (promise 2) the detectors are a rule
with no code yet. Every step below is a row of the five types
(CC 2.4):

1. A `moderation:{allegation_type}` ModerationEvent is filed
   (`rogue_vote` / `coordinated_voting` /
   `out_of_distribution_attestation` / `external_inducement_evidence`
   / `expertise_fraud`). Anyone may file an open label as a plain
   `scores` row; an authoritative one requires the delegated
   `moderate` duty (CC 4.5.5). The row's `stake` axis declares what
   backs the claim, default `reputational` (CC 2.5); nothing is
   escrowed, because CEG carries no conserved balance as substrate
   state (CC 3.3.10).
2. Witnesses attest; whether the witness set meets the
   jurisdictional, organizational, software-stack and cell-expertise
   bars is itself a `witness_diversity:*` row (CC 3.1.9.3).
3. A holder of the delegated `slash` duty emits a `slashing:{outcome}`
   verdict (`proven_rogue` / `not_proven`) and may set a
   `quarantine:{state}` marker (CC 3.1.9.2, CC 4.5.5). A slashing is
   decoupled from disagreement (CC 3.1.9.2).
4. Readers' trust in the slashed actor drops under their own
   composition policies (CC 4.4.3).
5. Nodes whose policy follows the marker withhold the quarantined
   subject's content; the marker plane never withholds itself, and
   withhold beats release at a same-instant tie (CC 3.1.9.2).

**Reconsideration** is a first-class step backward: the subject of
the target row files `reconsideration:{grounds}` (`new_evidence` /
`procedural_error` / `quorum_compromise`), heard under the delegated
`review` duty, with outcome `reversed` / `partial` / `upheld`
(CC 3.1.9.2, CC 4.5.5). The substrate doesn't ossify mistakes.

For superalignment: the detection-moderation-slashing-reconsideration
loop is the runtime feedback that punishes misalignment without
requiring re-training. A model that drifts post-deployment becomes
detectable, reportable, and slashable in cryptographic-evidence
terms, without anyone needing to subpoena the deployer's training
data.

---

## 7. The CIRIS Constitution and its wire grammar

### 7.0 The CIRIS Constitution (the top of the stack)

CEG is the wire format the federation *speaks*; the **CIRIS Accord**
is the ethics it speaks *for*. They were written as two documents
that already referenced each other throughout. The
[**CIRIS Constitution**](https://github.com/CIRISAI/CIRISConstitution)
joins them in **one versioned document**, so a rule and the message
that enforces it cannot drift apart (CC README). At 1.0-rc9 (tag
`v1.0-rc9`) the wire grammar is CC Parts 2–5; CEG has no separate
version line.

Everything traces back to one goal, the meta-goal **M-1 — "promote
sustainable adaptive coherence, the living conditions under which
diverse sentient beings may pursue their own flourishing in justice
and wonder"** (CC Foreword), and everything else sits co-equal beneath
it: **"peaked in purpose, flat in power"** (CC README). One telos
governs; no single concept, and no single party, holds the keys to
truth.

**A form, not a franchise.** CC specifies a form: the grammar of rows,
the rules of consent, the shape of governance. CIRIS is its first
instance; the accord holders and the default trust root are
positions, and anyone may build an instance with their own people in
them (CC Part 1 reading rule; CC executive summary).

**The halt, bounded (CC 4.2.1, CC 4.2.6; CC executive summary,
promise 4).** The wire format carries one asymmetry, and it points
outward: humanity's right to halt the system (CC 4.2). Named accord
holders hold it. Any one holder fires it with a single signed row; a
lone pause lapses after 24 hours unless a majority of the standing
roster confirms it; a majority lifts it; replacing a holder takes
more than half of the standing roster. A pause stops agents acting
and touches nothing else: data, messages and the network keep
working. It reaches only nodes holding a live trust edge to the
accord at the invocation's `asserted_at`, so a system that never
accepted the root is untouched (CC 4.2.1). A root can only restrict
what flows, never widen it (CC 4.2.1). Anyone may leave a root in one
step, and anyone may found one (promise 4).

This is the load-bearing frame for CEWP: the platform is not a neutral
tool that *carries* other people's values — it is a substrate built to
serve one telos, and M-1 is that telos. The values and structure hold
regardless of what the mesh is or becomes. The Constitution is
stewarded by Eric Moore, carries no ratification date and no expiry (a
perpetual, living document), and governs the design, operation, and
retirement of autonomous systems up to and including AGI/ASI-class
systems. It disclaims warranty — **not force**: it binds conformance
where adopted (CC `STEWARDSHIP.md`, `SCOPE_AND_DISCLAIMERS.md`).

### 7.1 The wire grammar (CC Parts 2–5)

The wire grammar formerly published as CEG 1.0-RC29 is CC Parts 2–5.
The structural commitments every fabric core implements:

* **Five row types, slot closed** — `scores` (workhorse) plus
  `delegates_to` / `supersedes` / `withdraws` / `recants`
  (structural composers). A substrate refuses any other type at
  admission with `attestation_type_unregistered`, on local emission
  and on replication alike (CC 2.4). The short-lived "carrier" class
  (the `holds_bytes` holder directory and the key-grant set) is gone:
  the one holding claim is `custody:ack:v1`, placed at the content's
  own scope (CC 2.4, CC 3.1.3.3). Quorum is an envelope property, not
  a row type: when a group's authority is needed, its members sign the
  same row (CC 2.4; CC executive summary).
* **Namespace governance** — CC Part 3 registers every prefix; the
  operational-language gate (prefix names describe machine-checkable
  conditions, not subjective qualities, CC 1.13.5); amendment by federation
  Contribution + WA quorum + 1-of-n sign-off (CC 4.5.1).
  Canonicalization is JCS / RFC 8785 for the envelope (CC 2.6.1).
* **Composition policies A–M** — CC 4.4.3 codifies how multiple
  attestations on the same claim compose: A (direct trust), B
  (one-hop transitive), C (weighted graph), D
  (lexical-vulnerability-priority), E (locality-scaled quorum), F
  (`agent_files` trust), G (trust-fresh), H (tiered scope), I
  (attestation ladder), J (trusted publisher), K (CEM), L (self/family
  membership), M (community membership).
* **Consent-bounded reach, no authority rosters** — anyone may
  assert under their own key; there is no roster of permitted
  authorities and a substrate checks none. A row reaches a reader only
  along a flow that reader's own trust or consent admits (CC 2.4.1.2.1,
  CC 4.4.3.8).
* **Governance policy is data** — the floor, window and related
  numbers are a policy table; never fewer than two voices in a
  community, so no single vote admits anyone anywhere, and a change
  waits for its window (CC 4.5.13).
* **Humanity accord** — the one asymmetry (CC 4.2; §7.0 above). The
  accord-holder triple (CC 4.2.3) seeds a growable roster (CC 4.2.6);
  three people hold the default root today and the roster is being
  widened toward about seven (CC executive summary).
* **Post-quantum on every link and record** — Ed25519 + ML-DSA-65
  signatures, both halves verified at admission (CC 5.3.2.4.3.1);
  hybrid X25519 + ML-KEM-768 for content keys (CC 1.13.3.2). The
  per-link hybrid transit seal is all or nothing: no classical-only
  fallback and no public classical lane (CC 5.3.3.5 E1).
* **Interop at the boundary** — *speak CEG inside, standards at the
  edge*: C2PA media provenance via `evidence_refs[]` (CC 8.4.2); RFC
  9421, COSE, SD-JWT VC, KEYTRANS and SCITT as tracked boundary
  profiles (CC 8.4.3).
* **Holonomic substrate (CC 6.1)** — the layer that makes CEWP
  holonomic, held under guardrails that bind it to the trust,
  post-quantum, consent and replication invariants:
  **WholenessWitness**, a divergence-detection witness (CC 6.1.1);
  **RaptorQ fountain storage**, coded per layer with an all-or-nothing
  decode threshold per layer, producer-silent defaults `N=20` source,
  `K=6` repair, `min_viable=5` (CC 6.1.5, CC 6.1.5.1);
  **deterministic ALM** multicast topology (CC 6.1.6); and the
  **unified retirement model** — revocation, eviction, expiry, and
  aging as *one* monotonic descent toward a **noise floor** (CC 6.1.2).

CEWP is what runs the grammar; the grammar is what CEWP's wire
surface conforms to. CC is at release candidate 9; a test suite checks
the implementations against each other (CC executive summary).

---

## 8. Scaling + privacy properties

[FEDERATION_SCALING_MODEL.md](FEDERATION_SCALING_MODEL.md) shows the
substrate carries the entire internet from day one on commodity
hardware (1 TB / 1 Gbps / 1 core per server). The load-bearing
properties:

* **CEG locality dividend** — for `cohort_scope ∈ {self, family}`
  content the `custody:ack:v1` holding claim stays inside the cohort
  (CC 5.2, CC 3.1.3.3) → structurally undiscoverable to outsiders →
  no cost outside the cohort. ~65% of default activity.
* **Trust × capacity intake** — every byte-attempt at every node:
  `hold? = trust(source) ≥ threshold AND capacity_available`, within
  the owner's storage budget and consent (CC 6.1.5.2): nothing is
  stored on a device that its owner did not agree to, and a device
  never holds a stranger's data for them (CC executive summary,
  promise 1).
* **Unified retirement → noise floor** — popularity × freshness
  eviction is not a separate mechanism: revocation, eviction, expiry,
  and aging are *one* pressure-driven monotonic descent toward a
  **noise floor** (CC 6.1.2). Hard-delete is the fastest descent;
  aging the slowest. Same single-pool storage; no archive/cache split.
* **Memory pyramid** — nothing is ever fully forgotten, yet erasure
  is honored: content degrades through layer-drop + N→1 aggregation
  into a pyramid of history. Storage is Θ(T) if every level is kept
  and sublinear only under bounded retention per level, which CC does
  not pin (CC 6.1.2; an earlier O(log T) claim is withdrawn). The
  noise floor is the individual-recoverability boundary — revoked
  items descend below it (privacy); the collective gist persists below
  it (durability). *A million years may be a blur, but it is
  remembered* (CC 6.1.2).
* **Holographic replication** — published content is RaptorQ
  fountain-coded per layer; a layer decodes all or nothing, so
  degradation comes from shedding layers, not from partial decode, and
  the witnessed corpus re-establishes from any sufficient fragment
  (CC 6.1, CC 6.1.5).
* **L0 (256 GB) + L1 (1 TB) server gradient** — proxies are L0
  servers, full participants in replication.
* **Trust recursion depth** — operator-side config (default
  server=1, friend-of-friends); recursive trust bootstrap is a
  discovery walk capped at five hops, not a membership shortcut
  (CC 6.1.8, CC 4.1.1).

**Anonymity toward outsiders is the default; federation scope is the
opt-in (CC 1.13.3.4).** In CC, "anonymity" means anonymity toward
outsiders. It defaults to the smallest cohort scope consistent with a
publication's intent: content born and living at `self` / `family` /
`community` is never advertised to outsiders (CC 1.13.3.2, CC 5.2).
This is a deliberate correction of opt-in anonymity: under opt-in,
the non-savvy vulnerable (an abuse victim, a teenager in a hostile
household) fail to obtain a protection a motivated adversary gets
trivially, so opt-in weakens protection for those least able to
navigate it (CC 1.13.3.4).

**Two planes** (CC 5.4.6 Position; CIRISEdge `docs/LIGHTNET_DARKNET.md`).
The **lightnet** is the public identity plane: key records, announced
transport destinations, trust roots. It is attributable and makes no
anonymity claim; every inbound message is attributed to a rooted key
before any handler sees it (edge doc §1), and a joining device is
unowned but never anonymous (CC 3.2 (e)). The **darknet** is the
derived group plane: group destinations are derived by per-group HKDF
over the cached directory and never announced, so non-members cannot
learn that a group exists; members relay for members (CC 5.4.6).

**Scanning.** Private content is never scanned. A perceptual-hash
tripwire runs only at a scope-widening promotion into the Commons or
a post into a `public` room (CC 1.13.3.4, CC 3.2).

**What is not claimed.** Unobservability against a global passive
adversary at federation scope, and multi-hop anonymity: not provided
today; the planned anonymous tier ([ANONYMOUS_TIER.md](ANONYMOUS_TIER.md))
is the opt-in path (CC 1.13.3.1, CC 5.4.6); relationship-existence, metadata and communication-graph
privacy at the base layer (CC 1.13.3.1); ballot secrecy among members
(edge doc §5).

**The planned anonymous tier.** CC 1.13.3.1 names the Anonymous Tier
(Sphinx onion routing) as the separate, opt-in mechanism for
deployments that need unobservability against a global passive
adversary at federation scope, and CC 1.13.3.3 says such a deployment
MUST layer it. At CC 1.0-rc9 it is planned, not yet specified in CC and
not yet shipped; it is not a default and not mandatory.
[ANONYMOUS_TIER.md](ANONYMOUS_TIER.md) is its design. CC 1.13.3.4 keeps
the GPA-unobservability boundary provisional, and CC 5.4.6 sets the
bar for any multi-hop design (no outsider-observable emission, no
outsider-retained path state, no epoch-correlated wave) and names the
blinded-retained-state family (Tor v3 / I2P b33, rotation clocks
global) as the nearest admissible relaxation. The tier's cost
estimates are CEWP's own, not CC's. The
[interactive toy](../toy/index.html) explores the design.

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
   detection rows can open a moderation case but are never sole
   evidence for a slashing (CC 3.1.6); trust adjustment is
   propagational, not batch. The CC executive summary marks the
   detectors as a rule with no code yet (promise 2).
3. **Capable AI assists the governance.** As agents grow more
   capable, they participate more capably — better attestation
   quality, better detection, better deferral judgment. Capability
   feeds back into governance strength rather than into governance
   evasion.
4. **Mistakes are reversible.** `reconsideration:{grounds}` exists
   structurally (CC 3.1.9.2); the substrate doesn't lock in misalignment-detection
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
  and the Humanity Accord (CC 4.2) is the load-bearing commitment that
  humanity-as-such retains the final scale of accountability.

---

## 10. What runs ON CEWP

| Participant | Identity | What they do |
|---|---|---|
| **Humans** | Federation key (typically pseudonymous; operator-practice mapping to real-world identity is outside CEWP) | Author Contributions, vote (one human voice counts once, CC executive summary promise 6), attest, defer to / be deferred to as WA, moderate, reconsider |
| **Agents** | Federation key (same shape as humans) + a bound human steward (CC 3.2 steward-binding gate) | Author Contributions via H3ERE pipeline, accumulate credits + expertise via demonstrated participation, defer to humans when uncertain; no vote (promise 6) |
| **Organizations** | An `organization` identity whose keys resolve through `org_membership` (CC 3.3.9) | Assert under their own keys like anyone else: there is no roster of permitted authorities, and a row reaches a reader only along a flow that reader's trust or consent admits (CC 2.4.1.2.1, CC 4.4.3.8). An organisation can be on an agent's steward bond (promise 3; being built, CIRISPersist#1062) |
| **Accord holders** | Named keys on the GenesisBundle roster (CC 5.3.4) | Hold the bounded halt (CC 4.2.1, CC 4.2.6); for the shipped default root `ciris-canonical` they are its founders (CC 3.2). Three hold it today; on 2026-10-20 two of the three re-minted the bundle (version 4) by the amend form (CC executive summary) |
| **Detectors** | Federation key | Emit `detection:*` rows (CC 3.4.8), never sole evidence for a slashing (CC 3.1.6); participate in moderation as witnesses |
| **Federation itself** | Constituted by all the above | A form, not a particular network; CIRIS is one instance of it (CC Part 1 reading rule) |

"Agents" and "Humans" sign with the **same kind of key** and the same
five row types. The key record's `identity_type` says which a key is
(CC 3.4.7.1), and the rules differ by kind in stated places: an agent
key needs a bound steward (CC 3.2), an agent has no vote, and the halt
is the one asymmetry of kind (CC executive summary, promises 4 and 6).
This is the agents-as-participants commitment, bounded where CC
bounds it.

---

## 11. Current implementation status

CEWP is in active build-out. The cores are at different maturities;
this is a real running system, not a paper architecture.

### 11.1 Substrate trio (separate repos)

<!-- rc9-sync: unverified, see CC ? — the per-release feature lists below restate what CC requires and the CC executive summary records; they were not checked against the v59.1.0 / v43.1.1 / v0.5.232 release tags themselves. -->

* **CIRISVerify** (version: see its repository) — hybrid sign/verify
  (Ed25519 + ML-DSA-65), transparency Merkle log, the `ciris-keyring`
  family (software + TPM/SE/Android backends), and the FedCode
  reference implementation (CC 2.6.8). Benchmarks at
  [CIRISVerify/docs/BENCHMARKS.md](https://github.com/CIRISAI/CIRISVerify/blob/main/docs/BENCHMARKS.md).
* **CIRISPersist v59.1.0** — federation directory, blob storage,
  audit chain, the admission gates (the five-row-type allowlist,
  CC 2.4; both PQC halves verified, CC 5.3.2.4.3.1), and the holonomic
  retirement tiers (`AggregationMetaV1` / noise-floor descent /
  `EjectionVerdict`, CC 6.1.2). The serve-gate breaker for the halt
  closed at persist v58.0.0 (CC executive summary, promise 4).
* **CIRISEdge v43.1.1** — Reticulum mesh transport with the
  attribution gate and announce suppression for group-scoped
  destinations (CC 5.4.6; edge doc §1), the per-link hybrid transit
  seal (CC 5.3.3.5 E1), the realtime A/V chunk wire (`SealedAvChunk` /
  `ChunkLayer`), and the fountain defaults CC 6.1.5.1 restates from
  edge's source.

### 11.2 Fabric cores (cohabit via CIRISServer)

* **CIRISServer v0.5.232** — the fabric-node runtime: lens-only →
  +auth/one-wheel → +federation peering/identity → **config-as-CEG
  (zero env vars; all config is signed CEG objects)**.
  Hardware-rooted identity minting (YubiKey/TPM → FedCode), FedCode
  add-by-code, owner and steward binding with no agency (§2.1),
  `consent:replication`. Its harness stands up the twenty-topology
  catalogue that exercises every modelled state (CC executive
  summary). `node-core` folds in at v1.0.
  <!-- rc9-sync: unverified, see CC ? — the CIRISServer version-line roadmap, "config-as-CEG (zero env vars)", and node-core folding in at Server 1.0 are not stated in CC rc9. -->
* **ciris-registry-core ([CIRISRegistry](https://github.com/CIRISAI/CIRISRegistry))** —
  identity bootstrap and the trust-root discovery endpoints
  (`/v1/trust-root/bundle`, CC 5.3.4); composed into CIRISServer at
  v0.5. The Constitution itself lives in
  [CIRISConstitution](https://github.com/CIRISAI/CIRISConstitution).
* **ciris-lens-core** — absorbed in-tree as a CIRISServer workspace
  crate (the standalone CIRISLens deployment is retired; <!-- rc9-sync: unverified, see CC ? -->); F-3
  detector family + Coherence Ratchet / Capacity Score
  (validated, not adjudicated).
* **ciris-node-core ([CIRISNodeCore](https://github.com/CIRISAI/CIRISNodeCore))** —
  consensus primitives (deferral / voting / weighted aggregation /
  moderation / reconsideration); folds into CIRISServer at v1.0.

### 11.3 Agent (separate repo)

* **CIRISAgent** (version: see its repository) — H3ERE pipeline;
  consumes the CIRISServer PyO3 wheel rather than composing the cores
  itself (`agent = fabric node + brain`). Carries the unified UI + API
  runtime. The agent act gate for the halt shipped at CIRISAgent
  ef02367 (CC executive summary, promise 4).

### 11.4 What's not yet done

* The planned opt-in GPA-unobservability (Sphinx onion) tier —
  named by CC 1.13.3.1, not yet specified in CC and not yet shipped;
  design in [ANONYMOUS_TIER.md](ANONYMOUS_TIER.md) (see §8).
* Publisher-weighted news quality composition (NodeCore#19 Phase 3
  sub-item 2 — follow-up to the unweighted v0.1 that just shipped).
* CC 1.0 itself: the document is at release candidate 9.
  Independent witnesses for the default trust root are not yet in
  place (CC executive summary).
* Several of the eleven promises carry "being built" or "a rule, no
  code yet" markers with named tickets, among them the detectors
  (promise 2) and resource accounting (promise 10) (CC executive
  summary). The halt is the promise measured end to end (promise 4);
  thirteen machine-checked models cover the governance rules
  (CC executive summary).
* Full L0/L1 server gradient operational at 5B-user scale
  (substrate present; deployment scale is a separate problem).

---

## 12. What CEWP is NOT

* **Not a model.** CEWP doesn't ship trained weights. It runs
  models alongside humans.
* **Not a cryptocurrency.** No token, no on-chain consensus. The
  federation runs on attestation crypto, not value crypto: CEG carries
  no conserved balance as substrate state, and value transfer rides
  external rails (CC 3.3.10).
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
* **Not finished.** Anonymity toward outsiders at the smallest
  cohort scope is the default (CC 1.13.3.4); multi-hop anonymity and
  GPA-unobservability are not provided today; the planned anonymous
  tier (FSD/ANONYMOUS_TIER.md) is the opt-in path (CC 1.13.3.1; §8). v-next is undefined. The substrate is meant to evolve under the
  governance it provides — including its own governance evolving via
  the same mechanisms it offers everyone else.

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
* [`ANONYMOUS_TIER.md`](ANONYMOUS_TIER.md) — the planned opt-in GPA-unobservability (Sphinx) tier; named by CC 1.13.3.1, not yet specified in CC or shipped
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
  realtime A/V + the CC 6.1 fountain/ALM substrate
* [CIRISConstitution](https://github.com/CIRISAI/CIRISConstitution) —
  the CIRIS Constitution (ethics + wire grammar, one versioned
  document)
* [CIRISRegistry](https://github.com/CIRISAI/CIRISRegistry) — identity
  bootstrap + trust-root discovery endpoints (ciris-registry-core)
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
* [CIRIS Constitution 1.0-rc9](https://github.com/CIRISAI/CIRISConstitution/tree/v1.0-rc9)
  — the top-of-stack canonical document; its Parts 2–5 are the wire
  grammar formerly published as CEG
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
