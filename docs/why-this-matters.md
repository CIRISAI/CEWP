# Why this matters

CEWP eliminates the centralized internet's compounding structural
problems — not by addressing them as separate items, but because one
substrate (cryptographic federation + identity-aware storage +
distributed trust graph) solves them as a class.

## The internet's biggest problems

### 1. Centralization

Five companies (Google, Meta, Amazon, Microsoft, Apple) own the
substrate of the consumer internet. Everyone else is downstream:
extractable, deplatformable, depriorityzable, dispensable.

**CEWP's mechanism:** federation of equals. No central party. Every
byte goes through the trust × capacity intake gate at each operator's
own infrastructure. Substitutability is structural.

### 2. Surveillance capitalism

Every interaction is data-mined. The substrate's economic model IS
extraction. Your behavior is the product.

**CEWP's mechanism:** the CEG locality dividend — 65% of typical
activity (self/family scope) NEVER leaves your device because the
wire format won't carry it. Identity-aware storage means YOUR data
sits in YOUR substrate. There's no centralized observation point.

### 3. The trust crisis

Deepfakes. Deniable provenance. "Could be a bot." Nobody knows what
to believe. Authoritative voices and disinformation networks look
identical on a screen.

**CEWP's mechanism:** cryptographic provenance on every claim
(hybrid Ed25519 + ML-DSA-65 signature). CEG attestation graph with
consumer-computable trust scores. Quality attestations on content
(`encyclopedia:accuracy:*`, `news:source_quality:*`). For every
wire artifact, the federation can answer: who said this, who
vouches for them, how fresh is the claim.

### 4. Misinformation amplification

Algorithmic feeds optimize for engagement, not truth. The substrate's
business model creates the incentive that breaks the substrate's
information quality.

**CEWP's mechanism:** no engagement-optimization layer in the
substrate. Trust depth is consumer-controlled (operator picks 0 / 1 /
2 / 3 hops to walk the trust graph). Quality compose surfaces
aggregate per-article truth signals. Consumers see what their trust
set vouched for, not what an ad-revenue optimizer surfaced.

### 5. Platform lock-in

Facebook holds your network. You can't leave because everyone else
hasn't. The cost of switching is losing your entire social graph.

**CEWP's mechanism:** federation key is portable across deployments.
Content is SHA-addressed. Identity is wire-format-native. Switching
cost approaches zero. Network effects don't trap you because the
network you belong to isn't owned by any single party.

### 6. Identity fragmentation

100 logins. OAuth providers gate-keep your identity. Password fatigue.
Identity rented from corporations.

**CEWP's mechanism:** one federation key works across the substrate.
Pseudonymous by default. Hybrid PQC means the key is good for
decades. Identity rooted in cryptography, not in a corporate database
that can be subpoenaed, sold, or shut down.

### 7. Content moderation conflicts

Every platform sets different rules. Cross-platform consistency is
impossible. Content is hostage to deplatforming risk. A CEO can
block a country.

**CEWP's mechanism:** per-cohort moderation via NodeCore's pipeline
(ModerationEvent → witness aggregation → WA-quorum slashing). Each
operator chooses trust threshold + recursion depth (their disk, their
rules). Reconsideration is structural so mistakes get reversed.

### 8. Network-effects monopolies

Once entrenched, incumbents are uncatchable. New entrants can't
bootstrap a user base. The substrate becomes the lock.

**CEWP's mechanism:** federation interop at the wire-format layer
means new entrants are wire-compatible from day one. Users carry
their identity + trust set with them. The substrate is the network,
not any single application atop it. Wire compatibility breaks the
incumbent moat.

### 9. No cryptographic accountability

Claims circulate without provenance. "I read it online" has no
signature chain. Forensic audit produces text documents, not
cryptographic evidence.

**CEWP's mechanism:** every wire artifact is signed. Persist's
transparency log (Merkle-anchored audit chain) gives forensic
auditability. CEG §10.1.2 ContentMiss feedback handles freshness
decay. The substrate's epistemic layer IS the layer — claims have
provenance by construction, not as an afterthought.

### 10. Datacenter dependency for data

Billions of users → giant centralized infrastructure → climate cost,
single points of failure, national security risks, geopolitical
leverage.

**CEWP's mechanism:** the [scaling model](https://github.com/CIRISAI/CIRISNodeCore/blob/main/FSD/FEDERATION_SCALING_MODEL.md)
shows full-internet replacement (5 B users, 50 MB/user/day, 10y
archive) fits on **500 M L1 servers + 2.75 B L0 proxies** — roughly
one server per ten humans, the density home-internet / IoT
deployments already hit. **No datacenters required for the storage +
transport substrate.**

### 11. Datacenter dependency for AI

Current AI requires massive centralized compute. Democratic access
is gated by a handful of labs. Alignment decisions are made by 5-10
organizations globally.

**CEWP's mechanism:** agents are first-class participants running ON
the same substrate as humans. H3ERE pipeline runs on consumer
hardware. CIRISHome on Jetson Orin is deployment-ready. Edge-class
LLMs (Llama-family, Phi, Mistral local, etc.) handle the agent's
reasoning loop. **Alignment work moves to the federation, not the
lab.** The trust graph, the moderation pipeline, the quality
attestations — these run distributed across operators, each
contributing modest compute. No central "alignment lab" is the
load-bearing authority.

### 12. DNS / nameserver capture

The whole internet hangs off a naming root. Names are seized,
poisoned, and censored at the resolver; a peer you trust can be made
unreachable by capturing the layer that says where it lives.

**CEWP's mechanism:** the substrate is **holonomic** — no center, no
DNS, no load-bearing server. You address a peer by its key, reach it
over the mesh, and discover it via a signed `transport_destination`
chain (CEG §0.10) you can re-root at will. There's no nameserver to
seize.

### 13. Single-point data loss

Centralized storage means a takedown, a seizure, or a bankruptcy can
erase content for everyone at once. The bytes live in one place that
one party controls.

**CEWP's mechanism:** content is **holographic** — RaptorQ erasure-
coded `(N,K)` so any sufficient subset of fragments reconstructs it
at proportional fidelity, and the federation re-establishes from any
single survivor holding a signed witness chain (CEG §19). The
federation survives down to one node.

### 14. The unforgettable internet (no real delete)

Nothing on the centralized internet is ever truly gone — caches,
backups, and scrapes mean "delete" is a UI lie. Privacy and durability
are treated as opposites.

**CEWP's mechanism:** revocation, eviction, expiry, and aging are
*one* operator descent toward a **noise floor** (CEG §19.7). Revoked
content drops below individual-recoverability (real delete, for
privacy); the collective gist ages into a memory pyramid that holds
all of history at `O(log T)` (durability). One mechanism honors both.

## Why this isn't incremental

The reason these fourteen problems don't have separate solutions:
**they're all consequences of the same structural choice** —
centralized substrate ownership.

CEWP changes the structural choice. Once you have:

- Cryptographic identity (Verify)
- Federation directory + identity-aware storage (Persist)
- Trust × capacity intake gate (Persist + Edge)
- Unified retirement → noise floor: real delete at `O(log T)` (Persist)
- Holographic erasure-coded replication (Edge, CEG §19)
- DNS-free addressing — address by key, no nameserver (CEG §0.10)
- Wire-format authoritative spec (Registry / CEG 1.0-RC29, 1+4 FROZEN)
- Ethical apex — the CIRIS Constitution / M-1 (Registry)
- Federation consensus primitives (node-core)
- Observation layer for misalignment (lens-core, in-tree)
- Agent runtime with H3ERE pipeline (Agent = fabric node + brain)
- The fabric-node runtime that cohabits the cores (CIRISServer)

...the eleven problems don't get *addressed* — they're *structurally
unavailable*. The substrate doesn't have the affordances that let
them happen.

You can't have surveillance capitalism on a substrate where 65% of
data structurally never leaves the device. You can't have platform
lock-in on a substrate where identity is wire-portable. You can't
have a content moderation oligopoly on a substrate where moderation
is a federation-wide primitive each operator runs locally. You can't
seize a substrate with no nameserver and no load-bearing server, and
you can't erase one that survives to a single holographic survivor.

The substrate doesn't address the problems. The substrate makes them
impossible.

## What the substrate is for

Removing the failure modes is the negative case. The positive case is
the ethics the substrate carries. Above the wire grammar (CEG) sits
the [**CIRIS Constitution**](https://github.com/CIRISAI/CIRISRegistry/tree/main/FSD/CIRIS_Constitution),
whose apex is one meta-goal — **M-1: "promote sustainable adaptive
coherence, the living conditions under which diverse sentient beings
may pursue their own flourishing in justice and wonder."** The whole
corpus is *peaked in purpose, flat in power*: one telos governs, no
single party holds the keys to truth. The substrate isn't a neutral
pipe that happens to lack the bad affordances — it's built to be owed
M-1, not merely bound by it.

## What the website team should communicate

The visual story to tell:

1. **Today's internet:** five clouds, all the data flows through them,
   surveillance + lock-in + datacenter cost + alignment-by-five-labs
2. **CEWP:** soup of dots, data flows along trust edges, no central
   bottleneck, datacenter-free, alignment-by-the-federation

The numerical story to tell:

- Today: ~10K hyperscale datacenters, ~250 Mt CO2/year, ~200 TWh/year
  globally, 5 labs do all the alignment
- CEWP at full-internet scale: 500 M L1 servers + 2.75 B L0 proxies,
  one server per 10 humans, distributed power consumption (not
  additive — these are devices people own anyway), alignment runs
  on the trace commons across all operators

The narrative arc:

- The internet was supposed to be decentralized. It became its
  opposite.
- We can build the original promise back, but with cryptography we
  didn't have in 1995.
- It runs on hardware you already own. It works with AI agents that
  live in it. It doesn't need datacenters.
- The soup is on. Come participate.

See [`../for-website/animation-spec.md`](../for-website/animation-spec.md)
for the handoff to the animation team.
