# The seven-repo Agent 3.0 architecture

CEWP is the platform identity for seven repos that compose into one
stack. Three substrate sisters host the bytes + crypto + transport.
Three fabric sisters host the federation-level semantics + detection
+ spec authority. The seventh repo is the agent runtime + unified
client (CIRISAgent — one repo, both roles), where users interact
and agents reason.

All seven **cohabit in one process** at CIRIS 3.0 deployments — one
PyO3 ABI, one persist Engine, one edge runtime, one tokio runtime.

```
                          ┌─────────────────────────────┐
                          │      CIRISAgent             │  ← users interact here;
                          │  (agent runtime + unified   │     agents reason here
                          │   client; H3ERE pipeline    │     (single repo, both roles)
                          │   + UI)                     │
                          └────────────┬────────────────┘
                                       │
            ┌──────────────────────────┼──────────────────────────┐
            │                          │                          │
   ┌────────▼─────────┐      ┌─────────▼────────┐      ┌──────────▼─────────┐
   │  CIRISNodeCore   │      │  CIRISLensCore   │      │   CIRISRegistry    │
   │  (consensus)     │      │  (detection)     │      │  (CEG + identity)  │
   └────────┬─────────┘      └─────────┬────────┘      └──────────┬─────────┘
            │                          │                          │
            └──────────────────────────┼──────────────────────────┘
                                       │
            ┌──────────────────────────┼──────────────────────────┐
            │                          │                          │
   ┌────────▼─────────┐      ┌─────────▼────────┐      ┌──────────▼─────────┐
   │   CIRISEdge      │      │  CIRISPersist    │      │   CIRISVerify      │
   │  (transport)     │      │  (storage)       │      │   (crypto)         │
   └──────────────────┘      └──────────────────┘      └────────────────────┘
                              │ federation_attestations
                              │ federation_blobs
                              │ federation_keys
                              │ audit chain
```

## Substrate sisters

### [CIRISVerify](https://github.com/CIRISAI/CIRISVerify) — crypto + transparency

Hybrid Ed25519 + ML-DSA-65 sign/verify. Merkle transparency log.
HardwareSigner trait family (TPM / Android Keystore / iOS Secure
Enclave / SoftwareOnly). AES-GCM at 5.45 GiB/s for encryption at rest.

**What it makes true:** "cryptographic accountability" is real, not
a slogan. Identity is rooted in cryptography, not in corporate
databases.

### [CIRISPersist](https://github.com/CIRISAI/CIRISPersist) — storage substrate

federation_keys + federation_attestations + federation_blobs +
audit chain. TrustScoring trait + AdmissionGate at every write path
+ EvictionSweeper (popularity × freshness) + list_held_by +
evict_actor (the identity-aware-storage seam).

**What it makes true:** trust is a substrate property, not an oracle.
Per-actor eviction is possible because every byte knows whose key
admitted it.

### [CIRISEdge](https://github.com/CIRISAI/CIRISEdge) — transport + dispatch

Reticulum (mesh, primary) + HTTPS (fallback). MessageType registry +
dispatch_inbound + outbound_enqueue. inline_text_pipeline (classify +
scrub + AES-GCM). ContentFetch / ContentBody for content addressing.

**What it makes true:** switching cost approaches zero. Federation
interop happens at the wire format level, not at any application
above it.

## Fabric sisters

### [CIRISNodeCore](https://github.com/CIRISAI/CIRISNodeCore) — federation consensus

The 11 federation-consensus primitives (P1-P11): deferral routing,
voting + weighted aggregation, expertise consensus, moderation,
slashing, reconsideration, external_content ingest, etc. Trust-depth
admission oracle. Article quality compose surfaces. Decision-
hierarchy typing.

**What it makes true:** wise authority deferral works as a first-
class wire protocol with audit trail and reversibility.

### [CIRISLensCore](https://github.com/CIRISAI/CIRISLensCore) — detection + science

F-3 detector family (emergent_deception, structural_injustice,
capacity-score regression, coherence-ratchet decay). Counter-RII
(Recursive Identity Injection counter-detection). Cohort/distributive
readings. Scoring oracle. Alert subscription delivery. RATCHET
integration.

**What it makes true:** the empirical bet — that reasoning shape is
measurable as everything else scales — is operationally testable.
LensCore is where the corridor metrics + k_eff math from the
[research synthesis](https://ciris.ai/research-status/) run.

### [CIRISRegistry](https://github.com/CIRISAI/CIRISRegistry) — CEG spec + identity bootstrap

The 18-section CEG 0.2 wire-format spec. The 1+4 primitive lockdown
(scores + delegates_to + supersedes + withdraws + recants). The
§5 dimension namespace governance. §9 humanity_accord. The steward
triple. The agent_files canonical attestation.

**What it makes true:** the wire format is authoritative; substrate
sisters implement against one spec; the federation is structurally
prevented from being captured by itself via the §9 humanity_accord.

## Agent runtime + unified client (one repo, both roles)

### [CIRISAgent](https://github.com/CIRISAI/CIRISAgent) — the agent runtime AND the unified client

The H3ERE pipeline — DMA → CSDMA → DSDMA → ASPDMA → conscience →
action. Policy adapters. The runtime humans + LLMs cohabit. The
`AgentMode` enum (client/proxy/server) that propagates tier posture
to all 22 services. The user interface + API runtime — iPhone,
Android, desktop, pip install.

**What it makes true:** agents reason inside CEWP; agents have the
same federation identity shape as humans; alignment is a property
of the federation's runtime, not the agent's pre-training. And: "a
free ChatGPT alternative you can actually check, in your language,
on your phone" (the [ciris.ai](https://ciris.ai/) public-facing
positioning).

CIRISAgent is the seventh repo — both the substrate's user-facing
surface AND the agent's reasoning runtime, in one cohabited
codebase.

## Why the cohabitation matters

CEWP runs the seven repos in one process. Same PyO3 ABI. Same persist
engine handle. Same edge runtime. Same tokio runtime.

This means:

- **Zero IPC overhead** between substrate components
- **One source of truth** for the federation state at any moment
- **Atomic transactions** across substrate sisters (the put_blob_signing
  call atomically commits bytes + holder attestation + signature)
- **Cohabited identity** — verify, persist, edge all share one
  local_signer via persist's PyCapsule pattern

The cost is that the seven repos have to stay in sync. The benefit
is that the platform runs on consumer-class hardware (your phone,
your laptop, your home server) instead of requiring datacenters.

That's the "we don't need big tech" claim made concrete.
