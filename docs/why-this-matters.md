# Why this matters

*Reconciled to the CIRIS Constitution 1.0-rc9 (tag
[`v1.0-rc9`](https://github.com/CIRISAI/CIRISConstitution/tree/v1.0-rc9)).
References are written "CC <section>"; where this page and the
Constitution disagree, the Constitution wins.*

CEWP aims at the centralized internet's compounding structural
problems — not by addressing them as separate items, but because one
substrate (cryptographic federation + identity-aware storage +
distributed trust graph) addresses them as a class. Where the
Constitution bounds a mechanism, the bound is stated next to it: CC
requires that nobody claim more than the substrate provides
(CC 1.13.3).

## The internet's biggest problems

### 1. Centralization

A handful of companies own the substrate of the consumer internet.
Everyone else is downstream: extractable, deplatformable,
deprioritizable, dispensable.

**CEWP's mechanism:** a federation with no steward, no root and no
load-bearing instance (CC 8.1.1). Every node runs its own admission
gate on its own infrastructure, and every reader picks the trust root
it follows and can leave it in one step (CC 3.2).

### 2. Surveillance capitalism

Every interaction is data-mined. The substrate's economic model IS
extraction. Your behavior is the product.

**CEWP's mechanism:** structural invisibility. For self and family
content, no holding claim leaves the cohort, so the rest of the
network cannot discover that the bytes exist or fetch them (CC 5.2).
The scaling model puts about 65% of typical activity at that scope
(CC 5.2, *Locality dividend*). Group destinations are never announced,
so outsiders cannot see that a private group exists (CC 5.4.6). Private
content is never scanned (CC 1.13.3.4).

**The limit:** this is content-holding confidentiality and
cohort-scoped visibility. It is not metadata, communication-graph or
traffic-analysis privacy; an observer of the network can still infer
a good deal about who talks to whom (CC 1.13.3.1).

### 3. The trust crisis

Deepfakes. Deniable provenance. "Could be a bot." Nobody knows what
to believe. Authoritative voices and disinformation networks look
identical on a screen.

**CEWP's mechanism:** a signature on every claim (hybrid Ed25519 +
ML-DSA-65, CC 5.3.3.5 E1). Claims about content are `scores` rows on
registered dimensions (CC 2.4), and each reader composes them under
its own trust policy (CC 4.4.3.8). An AI says it is an AI, and anyone
named in a claim can contest it on the record, with the contest
travelling with the claim (promise 2). For every row, a reader can
answer: who said this, who vouches for them, how fresh is the claim.

### 4. Misinformation amplification

Algorithmic feeds optimize for engagement, not truth. The substrate's
business model creates the incentive that breaks the substrate's
information quality.

**CEWP's mechanism:** the reader decides whom to believe. Consumer
policy sets how far trust is walked, 5 hops by default (CC 4.1.1),
and a fresh key has no weight until someone the reader trusts confers
it (CC 1.13.3.3). Consumers see what their trust set vouched for, not
what an ad-revenue optimizer surfaced.

<!-- rc9-sync: unverified, see CC ? — removed "no engagement-optimization layer in the substrate" as a CC claim and the dimension names `encyclopedia:accuracy:*` / `news:source_quality:*` from #3; neither is in CC rc9 -->

### 5. Platform lock-in

Facebook holds your network. You can't leave because everyone else
hasn't. The cost of switching is losing your entire social graph.

**CEWP's mechanism:** your identity is your key, not an account on
someone's server. Content is addressed by its hash and verified in
full before use (CC 5.3.2.5). Leaving a trust root is one row (CC 3.2),
and any node may follow another root (promise 4). The network you
belong to isn't owned by any single party.

### 6. Identity fragmentation

100 logins. OAuth providers gate-keep your identity. Password fatigue.
Identity rented from corporations.

**CEWP's mechanism:** one key works across the substrate. A key is an
identifier; standing comes only from attestations others make about
it (CC 1.13.3.3). Identity is rooted in cryptography, not in a
corporate database that can be subpoenaed, sold, or shut down. Records
are signed hybrid post-quantum (CC 5.3.3.5 E1).

### 7. Content moderation conflicts

Every platform sets different rules. Cross-platform consistency is
impossible. Content is hostage to deplatforming risk. A CEO can
block a country.

**CEWP's mechanism:** moderation lives with each community. No
community runs without a named moderator, and if the moderator lapses
the most trusted member is promoted in the same step (CC 4.5.4).
Allegations are `moderation:*` rows; a `slashing:*` outcome needs a
Wise Authority quorum and can never rest on detector flags alone
(CC 3.1.6, CC 3.1.9.2). No hand bans a community: the network
withdraws from one that keeps harm, node by node and root by root
(promise 6).

### 8. Network-effects monopolies

Once entrenched, incumbents are uncatchable. New entrants can't
bootstrap a user base. The substrate becomes the lock.

**CEWP's mechanism:** conformance is defined at the wire, as three
profiles (producer, consumer, substrate; CC 2.2), so a new
implementation that meets them interoperates from day one. Users
carry their key and their trust choices with them. The substrate is
the network, not any single application atop it.

### 9. No cryptographic accountability

Claims circulate without provenance. "I read it online" has no
signature chain. Forensic audit produces text documents, not
cryptographic evidence.

**CEWP's mechanism:** every row is signed. The transparency log, with
cosigned signed tree heads and consistency proofs, means a producer
cannot show two viewers two histories (CC 5.3.1; promise 11). Holder
records expire and are corrected by `ContentMiss` feedback
(CC 5.3.2.1). Claims have provenance by construction, not as an
afterthought.

### 10. Datacenter dependency for data

Billions of users → giant centralized infrastructure → climate cost,
single points of failure, national security risks, geopolitical
pressure.

**CEWP's mechanism:** the [scaling model](https://github.com/CIRISAI/CIRISNodeCore/blob/main/FSD/FEDERATION_SCALING_MODEL.md)
estimates that full-internet replacement (5 B users, 50 MB/user/day,
10y archive) fits on **500 M L1 servers + 2.75 B L0 proxies**, roughly
one server per ten humans, the density home-internet / IoT
deployments already hit. In that model, **no datacenters are required
for the storage + transport substrate.** These are model outputs, not
measurements.

### 11. Datacenter dependency for AI

Current AI requires massive centralized compute. Democratic access
is gated by a handful of labs. Alignment decisions are made by a few
organizations globally.

**CEWP's mechanism:** agents run on the same substrate as humans, on
the phones and computers people already own (CC Executive Summary).
The agent is a fabric node plus a brain (CC 8.1.1); its consequential
acts are checkable by parties it does not control (CC Executive
Summary, *The bet*), and named humans can pause every agent under
their root (promise 4). **Alignment work moves to the federation, not
the lab.** The trust graph, the moderation pipeline, the consented
traces run distributed across operators, each contributing modest
compute. No central "alignment lab" is the load-bearing authority.

<!-- rc9-sync: unverified, see CC ? — removed "CIRISHome on Jetson Orin is deployment-ready" and the list of edge-class LLMs; not in CC rc9 -->

### 12. DNS / nameserver capture

The whole internet hangs off a naming root. Names are seized,
poisoned, and censored at the resolver; a peer you trust can be made
unreachable by capturing the layer that says where it lives.

**CEWP's mechanism:** no center, no DNS, no load-bearing server. You
address a peer by its key, reach it over the mesh, and find it through
its own hybrid-signed `transport_destination` row (CC 3.3.6.2, CC
5.3.4). Members resolve each other without DNS (CC 4.4.3.2.4.1), and
no root is forced: the shipped default is one root among any number
(CC 3.2). There's no nameserver to seize.

**The limit:** that resolution, and the public identity plane it runs
on, expose who is reachable where (CC 1.13.3.1).

### 13. Single-point data loss

Centralized storage means a takedown, a seizure, or a bankruptcy can
erase content for everyone at once. The bytes live in one place that
one party controls.

**CEWP's mechanism:** durability is a per-item choice the owner makes
on publish. An item marked durable carries a target holder count, and
peers replicate its sealed chunks to that count across the live
cohort, rarest chunk first, repairing when holders fall toward the
floor, so content survives churn (CC 6.1.5). The witnessed,
trust-anchored corpus recovers from any one holder (CC 6.1). Nodes
never encode stored content (CC 5.4.2); a layered item is stored per
layer, so losing the detailed layers leaves a coarse version
(CC 6.1). Durability never overrides a withdrawal (CC 6.1.5 N5), and
anonymous-tier content is exempt so it can truly disappear
(CC 6.1.5 SR-2/3).

### 14. The unforgettable internet (no real delete)

Nothing on the centralized internet is ever truly gone — caches,
backups, and scrapes mean "delete" is a UI lie. Privacy and durability
are treated as opposites.

**CEWP's mechanism:** revocation, eviction, expiry, and aging are
*one* descent toward a **noise floor** (CC 6.1.2). A revoked item must
be below individual recoverability at every retained tier, and any
tier where it is still recoverable is purged (CC 6.1.5 N5); the
collective gist can age into a memory pyramid. A purge leaves a
tombstone, never touching a key (promise 1). One mechanism serves
both privacy and durability.

**The limits:** "below the floor" is claimed against a defined default
adversary holding the retained artifact and nothing else, not against
one with side information about the other members; aggregation is not
anonymization (CC 6.1.2). The pyramid's storage is Θ(T) if every
level is kept and sublinear only under bounded retention per level;
an earlier `O(log T)` figure was withdrawn (CC 6.1.2).

## Why this isn't incremental

The reason these fourteen problems don't have separate solutions:
**they're all consequences of the same structural choice** —
centralized substrate ownership.

CEWP changes the structural choice. Once you have:

- Cryptographic identity (Verify)
- Federation directory + identity-aware storage (Persist)
- An admission gate at every node (Persist + Edge)
- One retirement operator → noise floor (Persist; CC 6.1.2)
- Per-item replication (Edge; CC 6.1.5)
- DNS-free addressing — address by key, no nameserver (CC 4.4.3.2.4.1)
- One normative text for the grammar and the ethics (the CIRIS
  Constitution; CEG is its Parts 2 to 5)
- Federation consensus primitives (node-core)
- Observation layer for misalignment (lens-core, in-tree)
- Agent runtime with H3ERE pipeline (Agent = fabric node + brain)
- The fabric-node runtime that cohabits the cores (CIRISServer)

...many of these problems stop having a place to live. The substrate
lacks the affordances they depend on.

You can't have surveillance capitalism over content that never leaves
its cohort. You can't have platform lock-in where identity is a key
you hold and a root is something you leave in one row. You can't have
a content moderation oligopoly where moderation is a duty each
community holds for itself. You can't seize a substrate with no
nameserver and no load-bearing server, and you can't erase content
that recovers from any one of its holders.

The limits above still stand: metadata and traffic analysis are not
covered (CC 1.13.3.1), and harmful content inside private cohorts is
not detected; CC says so plainly rather than scanning private content
(CC 1.13.3.4).

## What the substrate is for

Removing the failure modes is the negative case. The positive case is
the ethics the substrate carries. The grammar and the ethics are one
document, the
[**CIRIS Constitution**](https://github.com/CIRISAI/CIRISConstitution/tree/v1.0-rc9),
whose apex is one meta-goal — **M-1: "Promote sustainable adaptive
coherence — the living conditions under which diverse sentient beings
may pursue their own flourishing in justice and wonder"** (CC 1.1).
Six principles sit beneath it, and no single principle grants licence
to violate another (CC 1.1). The substrate isn't a neutral pipe that
happens to lack the bad affordances; it is built to serve M-1.

## What the website team should communicate

The visual story to tell:

1. **Today's internet:** five clouds, all the data flows through them,
   surveillance + lock-in + datacenter cost + alignment-by-five-labs
2. **CEWP:** soup of dots, data flows along trust edges, no central
   bottleneck, datacenter-free, alignment-by-the-federation

The numerical story to tell:

- Today: ~10K hyperscale datacenters, ~250 Mt CO2/year, ~200 TWh/year
  globally, 5 labs do all the alignment
- CEWP at full-internet scale (modeled): 500 M L1 servers + 2.75 B L0
  proxies, one server per 10 humans, distributed power consumption
  (not additive — these are devices people own anyway), alignment runs
  on consented traces across all operators

<!-- rc9-sync: unverified, see CC ? — "Today" figures (~10K hyperscale datacenters, ~250 Mt CO2/year, ~200 TWh/year, "5 labs") have no CC source; keep only with an external citation -->

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

<!-- rc9-sync: the for-website/ directory does not exist in this repo; the link above is broken -->
