# The fabric-node architecture

*Reconciled to the CIRIS Constitution 1.0-rc9 (tag
[`v1.0-rc9`](https://github.com/CIRISAI/CIRISConstitution/tree/v1.0-rc9)).
References are written "CC <section>"; where this page and the
Constitution disagree, the Constitution wins.*

CEWP is the platform identity for the CIRIS **fabric node**. The
defining identity is **`agent = fabric node + brain`**: a fabric node
attests, stores, observes, reaches consensus and transports, but does
not reason or act (CC 8.1.1). Authority stays with human-held keys
under quorum, never with bare infrastructure (CC 3.4.7.1). Add a
reasoning brain (the H3ERE pipeline) and the same node becomes an
agent.

What was the "seven-repo Agent 3.0 stack" still exists as the
**library lineage**, but the *deployment* is now the fabric node. The
three fabric cores cohabit inside one runtime ([**CIRISServer**](https://github.com/CIRISAI/CIRISServer));
the substrate trio (Verify · Persist · Edge) and the agent stay
autonomous repos. The normative text for all of them is the
[CIRIS Constitution](https://github.com/CIRISAI/CIRISConstitution/tree/v1.0-rc9).

The default trust root, `ciris-canonical`, has the three accord
holders as its founders, under a 2-of-3 quorum; the canonical installs
`ciris-canonical-{1,2,3}` are members with standing and no vote
(CC 3.2). Its genesis bundle is at version 4, re-minted by a 2-of-3
amend on 2026-10-20 (CC 5.3.4). It is the shipped default, not a
privileged root: any node may follow another root, and anyone may
found one (CC 4.2.1, promise 4).

```
                          ┌─────────────────────────────┐
                          │      CIRISAgent             │  ← fabric node + brain
                          │  (H3ERE pipeline + UI)      │     users interact / agents reason
                          │  pip-installs the wheel ↓   │
                          └────────────┬────────────────┘
                                       │
                          ┌────────────▼────────────────┐
                          │         CIRISServer          │  ← the fabric node (one process)
                          │  registry · lens · node cores│     one persist Engine · one edge identity
                          └────────────┬────────────────┘
                                       │
            ┌──────────────────────────┼──────────────────────────┐
   ┌────────▼─────────┐      ┌─────────▼────────┐      ┌──────────▼─────────┐
   │   CIRISEdge      │      │  CIRISPersist    │      │   CIRISVerify      │
   │ (mesh transport) │      │ (corpus + tiers) │      │ (crypto + identity)│
   └──────────────────┘      └──────────────────┘      └────────────────────┘
                              │ federation_attestations
                              │ federation_blobs
                              │ federation_keys
                              │ audit chain + holonomic tiers
```

## The fabric node — CIRISServer

### [CIRISServer](https://github.com/CIRISAI/CIRISServer) v0.5.232 — the fabric-node runtime

The headless cohabitation runtime (and a PyO3 abi3 wheel CIRISAgent
pip-installs). It composes registry authority, lens observation and
node consensus over one substrate (CC 3.4.7.1), with one shared
persist `Engine` + one edge identity:

```
ciris-server (the fabric node)
  ├── ciris-registry-core   authority    — identity / build / licence / partner records
  ├── ciris-lens-core       observation  — detection:* rows (validated, not adjudicated)
  ├── ciris-node-core       consensus    — deferral / voting / expertise / moderation
  ├── one shared ciris-persist Engine    — the durable corpus + federation directory
  └── one shared ciris-edge runtime      — transport + the node's single federation identity
```

See its repository for the version roadmap. At the identity layer it
carries FedCode, the QR-able kind-tagged identity shorthand
(`CIRIS-V2-` / `V3-`; CC 2.6.8, CC 8.1.1); `infra:*` owner-binding,
which cannot carry agency (CC 4.4.3.4.3); a new node that trusts and
serves only until owned (CC 3.2); and `consent:replication`
(CC 3.3.1).

**What it makes true:** the deployment is a fabric node, not a process
full of cohabiting servers. **Separation of powers is the invariant**,
held cryptographically rather than procedurally: authority is
quorum-bound, observation is non-authoritative by namespace, and a
co-located node gains neither a vote nor a verdict (CC 3.4.7.1).

## Substrate trio (separate repos)

The substrate trio hosts the bytes + crypto + transport. These remain
autonomous repos that CIRISServer pins + composes.

### [CIRISVerify](https://github.com/CIRISAI/CIRISVerify) — crypto + transparency

Hybrid Ed25519 + ML-DSA-65 sign/verify. Merkle transparency log with
STH cosigning and consistency proofs (CC 5.3.1). Hardware-backed keys
(the hardware-class taxonomy, CC 4.2.2), founder-quorum verification,
and the PQC half mandatory at admission (CC 5.3.2.4.3.1). Version: see
its repository.

**What it makes true:** "cryptographic accountability" is real, not
a slogan. Identity is rooted in cryptography, not in corporate
databases.

### [CIRISPersist](https://github.com/CIRISAI/CIRISPersist) v59.1.0 — storage substrate

federation_keys + federation_attestations + federation_blobs +
audit chain. The admission gate at every write path refuses any row
that is not one of the five types (CC 2.4). It also carries the
**retirement tiers**: revocation, eviction, expiry, and aging as *one*
monotonic descent toward a **noise floor**, with the collective gist
kept as a memory pyramid (CC 6.1.2). That pyramid is Θ(T) if every
level is kept and sublinear only under bounded retention per level
(CC 6.1.2).

**What it makes true:** trust is a substrate property, not an oracle,
and memory is forever-but-still-forgets. Per-actor eviction is
possible because every byte knows whose key admitted it.

### [CIRISEdge](https://github.com/CIRISAI/CIRISEdge) v43.1.1 — transport + dispatch + holonomic swarm

Reticulum mesh transport (CC 8.1.1). Every byte over a link rides a
hybrid X25519 + ML-KEM-768 transit seal; there is no classical lane
(CC 5.3.3.5 E1). Realtime A/V chunk wire (SFrame + MLS; CC 5.3.3.2),
and the replication / ALM substrate: an item its owner marks durable
is replicated, sealed chunk by sealed chunk, to its target holder
count across the live cohort, rarest chunk first, repaired when
holders fall, with durability never overriding a withdrawal
(CC 6.1.5); RaptorQ serves the realtime A/V plane only (CC 5.4.3). The ALM relay tree is a pure function every
member computes the same way (CC 6.1.6).

Edge also carries the two planes (CC 5.4.6 *Position*;
[`docs/LIGHTNET_DARKNET.md`](https://github.com/CIRISAI/CIRISEdge/blob/main/docs/LIGHTNET_DARKNET.md)):
the **lightnet**, the public identity plane, announced and
attributable, with no anonymity claim; and the **darknet**, the
derived group plane, whose destinations are derived by per-group HKDF
over the cached directory and never announced, so outsiders cannot see
that a group exists. Members relay for members.

**What it makes true:** content survives churn, and the witnessed
corpus recovers from any one holder (CC 6.1, CC 6.1.5).
Private groups emit nothing an outsider can observe on the addressing
plane (CC 5.4.6); traffic analysis by an on-path observer is not
covered (CC 1.13.3.1).

<!-- rc9-sync: unverified, see CC ? — removed "HTTPS (fallback)", the MessageType registry / inline_text_pipeline description, and the `FountainSwarmRuntime` / TrustScoring / AdmissionGate names; not in CC rc9 -->

## Fabric cores (cohabit via CIRISServer)

### [CIRISNodeCore](https://github.com/CIRISAI/CIRISNodeCore) — consensus core (ciris-node-core)

The federation-consensus primitives: deferral routing, voting +
weighted aggregation, expertise consensus, moderation, slashing,
reconsideration, external_content ingest. A `slashing:*` outcome needs
a Wise Authority quorum and never rests on detector flags alone
(CC 3.1.6).

**What it makes true:** wise authority deferral works as a first-
class wire protocol with audit trail and reversibility.

### ciris-lens-core — detection + science (absorbed in-tree)

Detector rows under `detection:*`, named for the mechanism they
measure, not a judgment (`detection:correlated_action:*`, not
`detection:emergent_deception:*`; CC 1.2). Counter-RII (Recursive
Identity Injection counter-detection, CC 3.4.7.2). Coherence Ratchet /
Capacity Score — **validated, not adjudicated** (CC 3.4.7.1). The
standalone **CIRISLens deployment is retired**; this is now a
CIRISServer workspace crate. Traces reach it only under consent
(CC 3.3.1). The detectors that test what agents said against what
they did are, at rc9, a rule with no code yet (promise 2).

**What it makes true:** the empirical bet — that reasoning shape is
measurable as everything else scales — is operationally testable.
This is where the corridor metrics + k_eff math from the
[research synthesis](https://ciris.ai/research-status/) are meant to
run.

### [CIRISRegistry](https://github.com/CIRISAI/CIRISRegistry) — identity, build, licence, partner (ciris-registry-core)

Carries identity, build, licence and partner records (CC 3.1.1). It
is a conferred slice of a canonical node, not a root (CC 5.3.4).
Licence authority is not a roster: anyone may be a licensing authority
under their own key, and a reader decides whom to believe (CC
2.4.1.2.1).

The wire grammar and the ethics are no longer published here. They
are one document, the
[**CIRIS Constitution 1.0-rc9**](https://github.com/CIRISAI/CIRISConstitution/tree/v1.0-rc9):
CEG is its Parts 2 to 5, the five row types are a closed slot changed
only by a release of the Constitution (CC 2.4), and the accord's halt
is the one asymmetry the wire carries (CC 4.2).

**What it makes true:** the wire format is authoritative, fabric cores
implement against one text, and the Constitution names what the
substrate serves (M-1, CC 1.1).

## Agent (separate repo)

### [CIRISAgent](https://github.com/CIRISAI/CIRISAgent) — `fabric node + brain`

The H3ERE pipeline, which produces typed decision-making verdicts
(`dma:pdma:*`, `dma:csdma:*`, `dma:dsdma:*`, `dma:idma:*`;
CC 3.1.5.1), plus the unified UI + API runtime (iPhone, Android,
desktop, pip install). Now **consumes the CIRISServer PyO3 wheel**
rather than composing the cores itself: the brain that turns a fabric
node into an agent.

**What it makes true:** agents reason inside CEWP, under the same
promises as humans (CC Executive Summary), with a bound human steward
(CC 3.2) and no vote (promise 6). Their consequential acts are
checkable by parties they do not control, and named humans can pause
them (promise 4). And: "a free ChatGPT alternative you can actually
check, in your language, on your phone" (the [ciris.ai](https://ciris.ai/)
public-facing positioning).

## Why the fabric node matters

The fabric node runs the three cores in one process. Same PyO3 ABI.
Same persist engine handle. Same edge runtime. Same tokio runtime.

This means:

- **Zero IPC overhead** between cores
- **One source of truth** for the federation state at any moment
- **Atomic transactions** across the substrate trio
- **Cohabited identity** — the full role set in one key and process,
  which is co-location of custody, not consolidation of authority
  (CC 3.4.7.1)

<!-- rc9-sync: unverified, see CC ? — removed the `put_blob_signing` and PyCapsule implementation details; not in CC rc9 -->

The benefit is that the platform runs on the phones and computers
people already own (CC Executive Summary) instead of requiring
datacenters, with no center, no DNS and no load-bearing server
(CC 8.1.1).

That's the "we don't need big tech" claim made concrete.
