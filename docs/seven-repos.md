# The fabric-node architecture

CEWP is the platform identity for the CIRIS **fabric node**. The
defining identity is **`agent = fabric node + brain`**: a fabric node
is a piece of infrastructure that stores, witnesses, degrades, and
transports CEG attestations *mechanically, never reasoning* — and
authority roots in accountable humans, never in bare infrastructure.
Add a reasoning brain (the H3ERE pipeline) and the same node becomes
an agent.

What was the "seven-repo Agent 3.0 stack" still exists as the
**library lineage**, but the *deployment* is now the fabric node. The
three fabric cores cohabit inside one runtime ([**CIRISServer**](https://github.com/CIRISAI/CIRISServer));
the substrate trio (Verify · Persist · Edge) and the agent stay
autonomous repos. The three canonical singleton servers (`registry-us`,
`registry-eu`, the standalone CIRISLens deployment) are **retired** in
favor of three identical fabric nodes under a 2-of-3 founder quorum.

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

### [CIRISServer](https://github.com/CIRISAI/CIRISServer) — the fabric-node runtime

The headless cohabitation runtime (and a PyO3 abi3 wheel CIRISAgent
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
fabric node). At the platform-identity layer it adds hardware-rooted
federation identity (YubiKey/TPM → `CIRIS-V2-` fedcode), NodeCode
(`CIRIS-V1-…` QR add-by-code, DNS-free), `infra:*` owner-binding (no
agency), serve-only-until-owned, and `consent:replication`.

**What it makes true:** the deployment is a fabric node, not a process
full of cohabiting servers. **Separation of powers is the invariant** —
authority is quorum-bound, observation is non-authoritative by
namespace, and infrastructure must not have agency.

## Substrate trio (separate repos)

The substrate trio hosts the bytes + crypto + transport. These remain
autonomous repos that CIRISServer pins + composes.

### [CIRISVerify](https://github.com/CIRISAI/CIRISVerify) v6.x — crypto + transparency

Hybrid Ed25519 + ML-DSA-65 sign/verify. Merkle transparency log.
HardwareSigner trait family (TPM / Android Keystore / iOS Secure
Enclave / SoftwareOnly), founder-quorum verification, PQC-mandatory-
at-admission.

**What it makes true:** "cryptographic accountability" is real, not
a slogan. Identity is rooted in cryptography, not in corporate
databases.

### [CIRISPersist](https://github.com/CIRISAI/CIRISPersist) v9.0.0 — storage substrate

federation_keys + federation_attestations + federation_blobs +
audit chain. TrustScoring trait + AdmissionGate at every write path,
and the **holonomic retirement tiers** — revocation, eviction, expiry,
and aging as *one* monotonic descent toward a **noise floor**, history
kept as a memory pyramid at O(log T) (CEG §19.7).

**What it makes true:** trust is a substrate property, not an oracle —
and memory is forever-but-still-forgets. Per-actor eviction is
possible because every byte knows whose key admitted it.

### [CIRISEdge](https://github.com/CIRISAI/CIRISEdge) v4.6.x — transport + dispatch

Reticulum (mesh, primary) + HTTPS (fallback). MessageType registry +
dispatch. inline_text_pipeline (classify + scrub + AES-GCM). Realtime
A/V chunk wire (SFrame + MLS), and the §19 RaptorQ fountain / ALM
substrate CEG absorbed for holographic replication.

**What it makes true:** switching cost approaches zero, and content
survives to one holder — published content is erasure-coded so any
sufficient fragment subset reconstructs it.

## Fabric cores (cohabit via CIRISServer)

### [CIRISNodeCore](https://github.com/CIRISAI/CIRISNodeCore) — consensus core (ciris-node-core)

The federation-consensus primitives: deferral routing, voting +
weighted aggregation, expertise consensus, moderation, slashing,
reconsideration, external_content ingest. Folds into CIRISServer at
v1.0.

**What it makes true:** wise authority deferral works as a first-
class wire protocol with audit trail and reversibility.

### ciris-lens-core — detection + science (absorbed in-tree)

F-3 detector family (emergent_deception, structural_injustice,
capacity-score regression, coherence-ratchet decay). Counter-RII
(Recursive Identity Injection counter-detection). Cohort/distributive
readings. Coherence Ratchet / Capacity Score — **validated, not
adjudicated**. The standalone **CIRISLens deployment is retired**;
this is now a CIRISServer workspace crate.

**What it makes true:** the empirical bet — that reasoning shape is
measurable as everything else scales — is operationally testable.
This is where the corridor metrics + k_eff math from the
[research synthesis](https://ciris.ai/research-status/) run.

### [CIRISRegistry](https://github.com/CIRISAI/CIRISRegistry) — CEG + Constitution spec authority (ciris-registry-core)

The **20-section CEG 1.0-RC29** wire-format spec (§0–§19), with the
**1+4 surface FROZEN** as of RC1 (`scores` + delegates_to + supersedes
+ withdraws + recants) — a change to the wire bytes is now a *found
defect*, not an edit. New **§18 interop** ("speak CEG inside, standards
at the edge"; C2PA via `evidence_refs[]`) and **§19 holonomic**
(normative). The §5 dimension namespace governance. §9 humanity_accord.
The steward triple. Also the publication home of the
[**CIRIS Constitution (CC 0.1.5)**](https://github.com/CIRISAI/CIRISRegistry/tree/main/FSD/CIRIS_Constitution)
— the Accord 1.3-RC2 (ethics) + CEG 1.0-RC29 (grammar) woven into one
document, one version line; the top-of-stack canonical authority.

**What it makes true:** the wire format is authoritative; fabric cores
implement against one spec; the federation is structurally prevented
from being captured by itself via the §9 humanity_accord; and the
Constitution names what the substrate is owed (M-1), not merely bound
by.

## Agent (separate repo)

### [CIRISAgent](https://github.com/CIRISAI/CIRISAgent) — `fabric node + brain`

The H3ERE pipeline — DMA → CSDMA → DSDMA → ASPDMA → conscience →
action — plus the unified UI + API runtime (iPhone, Android, desktop,
pip install). Now **consumes the CIRISServer PyO3 wheel** rather than
composing the cores itself: the brain that turns a fabric node into an
agent.

**What it makes true:** agents reason inside CEWP; agents have the
same federation identity shape as humans; alignment is a property
of the federation's runtime, not the agent's pre-training. And: "a
free ChatGPT alternative you can actually check, in your language,
on your phone" (the [ciris.ai](https://ciris.ai/) public-facing
positioning).

## Why the fabric node matters

The fabric node runs the three cores in one process. Same PyO3 ABI.
Same persist engine handle. Same edge runtime. Same tokio runtime.

This means:

- **Zero IPC overhead** between cores
- **One source of truth** for the federation state at any moment
- **Atomic transactions** across the substrate trio (the
  put_blob_signing call atomically commits bytes + holder attestation
  + signature)
- **Cohabited identity** — verify, persist, edge all share one
  local_signer via persist's PyCapsule pattern

The benefit is that the platform runs on consumer-class hardware (your
phone, your laptop, your home server) instead of requiring datacenters,
and the three singleton servers collapse into identical peers under a
2-of-3 founder quorum — no center, no DNS, no load-bearing server.

That's the "we don't need big tech" claim made concrete.
