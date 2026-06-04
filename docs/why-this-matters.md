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

## Why this isn't incremental

The reason these eleven problems don't have separate solutions:
**they're all consequences of the same structural choice** —
centralized substrate ownership.

CEWP changes the structural choice. Once you have:

- Cryptographic identity (Verify)
- Federation directory + identity-aware storage (Persist)
- Trust × capacity intake gate (Persist + Edge)
- Popularity × freshness eviction (Persist)
- Wire-format authoritative spec (Registry / CEG 0.10)
- Federation consensus primitives (NodeCore)
- Detection layer for misalignment (LensCore)
- Agent runtime with H3ERE pipeline (Agent)
- Unified client (GUI)

...the eleven problems don't get *addressed* — they're *structurally
unavailable*. The substrate doesn't have the affordances that let
them happen.

You can't have surveillance capitalism on a substrate where 65% of
data structurally never leaves the device. You can't have platform
lock-in on a substrate where identity is wire-portable. You can't
have a content moderation oligopoly on a substrate where moderation
is a federation-wide primitive each operator runs locally.

The substrate doesn't address the problems. The substrate makes them
impossible.

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
