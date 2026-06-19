# CEWP

**CIRIS Epistemic Web Platform.** Pronounced **"soup."**

A decentralized replacement for the internet — and simultaneously
an AI governance / superalignment platform — built on the bet that
**big tech is not necessary**.

## What CEWP is

CEWP is an *epistemic web*: a **holonomic**, cryptographically-
accountable federation where humans, AI agents, and organizations are
first-class participants with the same identity shape. Every
load-bearing claim ("this content is accurate," "this peer is
trusted," "this action should be deferred to a human") is a signed
wire-format artifact in one small grammar (CEG's **"1+4"**); the
federation's collective weighted-aggregate scoring of those artifacts
is what *governs* the system in real time.

**No center, no DNS, no load-bearing server.** You address a peer by
its key, reach it over the mesh, and trust it through a chain you can
re-root at will. Content is **holographic** — erasure-coded so any
sufficient subset of fragments reconstructs it, and the federation
re-establishes from any single survivor with a signed witness chain.
Memory is forever but still forgets: revocation, eviction, expiry, and
aging are *one* descent toward a **noise floor**, keeping all of
history as a memory pyramid at `O(log T)`.

Agents live **inside** CEWP; they don't have CEWP as a tool. Every
node is a **fabric node** that stores, witnesses, degrades, and
transports — *mechanically, never reasoning*: **`agent = fabric node
+ brain`**, and authority roots in accountable humans, never in bare
infrastructure.

```
                          ┌─────────────────────────────┐
                          │      CIRISAgent             │  ← fabric node + brain
                          │  (H3ERE pipeline + UI)      │     pip-installs the wheel ↓
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
```

**The fabric node** ([CIRISServer](https://github.com/CIRISAI/CIRISServer))
cohabits the three fabric cores (registry · lens · node) over one
shared persist Engine + one edge identity; the substrate trio
(Verify · Persist · Edge) and the agent stay autonomous repos. The
three canonical singleton servers are retired in favor of identical
fabric nodes under a 2-of-3 founder quorum. The substrate runs on
commodity hardware (down to CIRISHome on Jetson Orin — already
deployment-ready) and scales to the full internet (5 billion users)
on ~1 server per 10 humans — the density home-internet / IoT
deployments already deliver. **No datacenters required.**

## The premise and the bet

**Premise:** big tech is not necessary.
**Bet:** cryptographic substrate + standardized ethical tracing
prove it.

Not "we want to replace big tech." Not "big tech could be better."
**Big tech is not necessary** — the substrate makes the centralized
internet's failure modes structurally unavailable, and the empirical
case (that reasoning has a shape we can measure as everything else
scales) is articulated at
[**ciris.ai/research-status**](https://ciris.ai/research-status/).

## Validation & evidence

The claims above are not aspirational hand-waving. Much of the substrate
is **measured**, **conformance-gated**, and **shipping to end users today**.
This section surfaces that evidence next to the claims it supports; the raw
artifacts live in the sister repos and on the live benchmark page.

### Measured performance — [CIRIS fabric benchmarks](https://cirisai.github.io/CIRISServer/)

Single-core Criterion microbenchmarks (the `MEASURED` rows on the benchmark
page) back the crypto, transport, and storage claims:

| What | Result | Claim it supports |
|---|---|---|
| Hybrid X25519 + ML-KEM-768 handshake | **339.6 µs** total (post-quantum tax +160.7 µs, once per peer-link) | Post-quantum-ready identity at negligible cost |
| AEAD throughput (AES-256-GCM) | up to **1.27 GiB/s** / core | Crypto is not the bottleneck |
| 1080p keyframe seal → wire → open | **192.18 µs** (256 KiB) | Full-motion video on commodity cores |
| ALM relay per-hop overhead | **~946 ns** (~0.96 µs/tier, `O(log N)` tree) | Mesh transport scales without datacenters |
| Holographic storage (N=20, K=6, H=30) | **99.6%** reconstruction at 33% fragment loss; 100% at 30% | Erasure-coded survival is real, not theoretical |
| Hybrid trace ingest | **470.44 µs** (~2,126 traces/sec/core) | Signed-artifact governance throughput |

Crucially, the benchmark page labels every row `MEASURED` vs. `MODEL` vs.
`PROJECTED` vs. `FRONTIER` — it is explicit about **what is measured, what is
modeled, and what is not yet built** (e.g. full symmetric M>2 MDC video is
flagged `FRONTIER`; membership rekey is `PROJECTED` against an open issue).

### Conformance — [CIRISConformance](https://github.com/CIRISAI/CIRISConformance)

Cohabitation + CEG-profile conformance is **gated against the published
wheels**. The suite verifies the five PyO3 substrate cores (persist · verify ·
edge · node-core · lens-core) cohabit correctly in one process and conform to
the CEG contract, across three CEG conformance profiles — **CCP** (producer),
**CCC** (consumer), **CCS** (substrate) — at two tiers (substrate cohabitation
+ fabric emergent behavior: replication discipline, scaling factors). Expected
failures are marked `xfail` against upstream issues rather than hidden.

### Shipping clients — [ciris.ai/install](https://ciris.ai/install)

CIRISAgent is not a lab exercise; it ships to end users on both mobile stores:

- **iOS** — [CIRIS Agent on the App Store](https://apps.apple.com/us/app/cirisagent/id6758524415)
- **Android** — [CIRIS Agent on Google Play](https://play.google.com/store/apps/details?id=ai.ciris.mobile)
- **Desktop / server** — `pip install ciris-agent`, Docker, or from source (see the [install guide](https://ciris.ai/install))

## What's in this repo

This repo is the **platform-identity + canonical FSDs + simulation
home** for CEWP. The technical substrate lives in the sister repos
(linked below); this repo is what you read to understand the platform
as a whole, what you fork to run simulations of it, and where the
cross-cutting FSDs (those that span more than one substrate repo)
are mirrored for easy cross-reference.

### Canonical FSDs (mirrored from CIRISNodeCore)

| FSD | What it covers |
|---|---|
| [`FSD/CEWP.md`](FSD/CEWP.md) | Platform identity — three load-bearing claims, "we don't need big tech" premise, agents-as-participants, superalignment via distributed epistemic governance |
| [`FSD/FEDERATION_SCALING_MODEL.md`](FSD/FEDERATION_SCALING_MODEL.md) | Quantitative model. Single-pool storage. Trust × capacity intake + popularity × freshness eviction. L0 / L1 tier model. 13 scenarios up to full-internet-with-video; identity-aware-storage thesis with prior-art comparison. |
| [`FSD/MEDIA_SHARING.md`](FSD/MEDIA_SHARING.md) | **Multimedia tier — YouTube / TikTok / Netflix / OnlyFans / AdultHUB replacement.** Five new external_content sub_kinds (image / audio / video / film / model_3d); content classification + multi-scheme rating (MPAA / BBFC / PEGI / etc.); operator-managed age gate; takedown_notice + key_grant CEG-native subject_kinds; content encryption (DEK / KEK / X25519+AES-GCM HPKE base mode); international standards mapping (DMCA / DSA / OSA / TVEC / NCMEC / AVMSD / KOSA / EU AI Act). |
| [`FSD/ANONYMOUS_TIER.md`](FSD/ANONYMOUS_TIER.md) | v2 anonymous publication path — Sphinx onion routing + Ed25519 key blinding + AEAD + rendezvous discoverability for totalitarian-threat deniability. Parallel to v1; doesn't break the identity-aware substrate. |
| [`FSD/SIMULATION_ENGINE.md`](FSD/SIMULATION_ENGINE.md) | Rust simulation engine spec — modular workspace; scalable from 1 K-agent browser playback to 5 B-agent GPU sim at 1:1. Real-world topology data (PeeringDB, CAIDA, TeleGeography submarine cables, GeoNames). Snapshots feed the website team's existing Three.js viz. |

### Working toy

| Path | Purpose |
|---|---|
| [`examples/scale_model.rs`](examples/scale_model.rs) | The runnable scaling-model toy. `cargo run --example scale_model` (from CIRISNodeCore workspace; mirrored here for reference). **13 scenarios** covering bootstrap → village → dunbar → media-heavy → twitter → news → full-internet → adulthub → tiktok → youtube → netflix → full-internet-with-video. All feasible per-server at v1 gates (1 TB / 1 Gbps / 1 core). |

### Explanation docs (for general audience + website team)

| Path | Purpose |
|---|---|
| [`docs/overview.md`](docs/overview.md) | Plain-language platform explanation |
| [`docs/seven-repos.md`](docs/seven-repos.md) | The fabric-node architecture: who does what across the cores (CIRISServer cohabitation + substrate trio + agent) |
| [`docs/the-bet.md`](docs/the-bet.md) | The premise + the empirical bet from the research synthesis |
| [`docs/why-this-matters.md`](docs/why-this-matters.md) | The internet's 11 structural problems + CEWP's mechanisms that eliminate them |
| [`docs/for-developers.md`](docs/for-developers.md) | Pointers to the canonical FSDs in the substrate-sister repos |

### Consolidated analysis set (vendored snapshot)

[`analysis/`](analysis/) gathers the **load-bearing documents** of the whole
CIRIS stack — pulled from their 15 home repositories into one tiered set
(philosophy → constitution → governance → implementation) so the system can be
reviewed as a single philosophical-technical whole. See
[`analysis/README.md`](analysis/README.md) for the tier map and
[`analysis/SOURCES.md`](analysis/SOURCES.md) for the provenance manifest
(every source repo + pinned commit).

| Tier | Covers | Key sources |
|---|---|---|
| [0 — Philosophy](analysis/tier0-philosophy/) | adaptive coherence, Coherence Ratchet, Proof-of-Benefit, M-1 | CIRISAgent, coherence-ratchet (+ Lean lake), RATCHET |
| [1 — Constitution](analysis/tier1-constitution/) | sovereignty, amendment, halting, anti-capture | Constitution 0.1.5, CIRISAccord, CEG §9/§11 |
| [2 — Federation governance](analysis/tier2-federation-governance/) | consensus, expertise, moderation, deferral | CIRISNodeCore |
| [3 — Observation/epistemology](analysis/tier3-observation-epistemology/) | what counts as evidence; coherence measurement | CIRISLensCore, CCA paper |
| [4 — Registry/trust](analysis/tier4-registry-trust/) | credentials, licensure, revocation; CEG 1.0-RC29 spec | CIRISRegistry, CIRISPersist |
| [5 — Security](analysis/tier5-security/) | per-repo threat models + audits | every substrate repo (canonical) |
| [6 — Substrate](analysis/tier6-substrate/) | partition / steward-loss / node-loss survival | Verify, Persist, Edge |
| [7 — Integration](analysis/tier7-integration/) | do the pieces compose | CIRISServer, CIRISAgent |

> ⚠️ **Snapshot pulled 2026-06-19 — these copies go stale.** Every file under
> `analysis/` is a verbatim vendored copy at the commit pinned in `SOURCES.md`;
> the home repo stays canonical. Re-pull before relying on any document for a
> decision. Threat models were taken from each repo's canonical
> `docs/THREAT_MODEL.md` (not the stale vendored copies in RATCHET).

## What CEWP eliminates

| Problem | Mechanism |
|---|---|
| Centralization (5 companies own the substrate) | Federation of equals; no central party |
| Surveillance capitalism | CEG locality dividend — 65% of typical activity never leaves your device |
| Trust crisis (deepfakes, deniable provenance) | Cryptographic provenance on every claim |
| Misinformation amplification | No engagement-optimization layer; consumer-controlled trust depth |
| Platform lock-in | Federation key portable across deployments; content SHA-addressed |
| DNS / nameserver capture | No DNS, no forced root — address by key, discover via signed `transport_destination` chain, re-root trust at will |
| Single-point data loss | Holographic erasure coding — any sufficient fragment subset reconstructs; federation survives to one survivor |
| The unforgettable internet (no real delete) | One descent operator → noise floor: revoked content drops below individual-recoverability; collective gist ages into an `O(log T)` memory pyramid |
| Identity fragmentation | One federation key works across the substrate |
| Content moderation conflicts | Per-cohort moderation; operator chooses trust threshold + recursion depth |
| Network-effects monopolies | Federation interop at wire format means new entrants are wire-compatible day one |
| No cryptographic accountability | Every wire artifact signed; transparency-log audit chain |
| Datacenter dependency (data) | Full-internet scale fits on home-server-class hardware, ~1 per 10 humans |
| Datacenter dependency (AI) | Agents run on Jetson Orin class hardware; alignment moves to the federation, not the lab |

## The numbers (from the scaling model)

At "full internet replacement" — 5 billion users, 50 MB/user/day:

|  | Today's Internet | CEWP Substrate |
|---|---|---|
| Centralization | ~5 hyperscalers run the substrate | 500 M L1 servers + 2.75 B L0 proxies, ~1 per 10 humans |
| Datacenters required | Yes (~10 K hyperscale + edge) | **None** |
| AI training/inference | Centralized in 5 labs | Edge-deployable; alignment distributed |
| Per-user disk owned | ~0 (your data lives on their servers) | Yours, on your hardware |
| Per-user trust signal | Platform-controlled "verified" checkmark | Cryptographic attestation graph, locally computable |
| Switching cost | Network-effects lock-in (Facebook holds your graph) | ~Zero (federation key portable) |

The simulation engine (FSD/SIMULATION_ENGINE.md) is what produces
these numbers in animation-ready form — modeled at 1:1 over the
real internet topology (real metros, real submarine cables, real
peering points).

## The fabric-node stack

CEWP is the platform identity for the CIRIS fabric node. What was
the "seven-repo Agent 3.0 stack" is now deployed as cohabiting cores;
the platform is what they become together.

- **The fabric node** ([CIRISServer](https://github.com/CIRISAI/CIRISServer)):
  the headless runtime (+ PyO3 wheel) cohabiting the three fabric
  cores — `ciris-registry-core` (authority + CEG/Constitution spec),
  `ciris-lens-core` (observation; absorbed in-tree), `ciris-node-core`
  (consensus; folds in at Server 1.0) — over one shared persist
  Engine + one edge identity.
- **Substrate trio** (separate repos; bytes + crypto + transport):
  [CIRISVerify](https://github.com/CIRISAI/CIRISVerify) +
  [CIRISPersist](https://github.com/CIRISAI/CIRISPersist) +
  [CIRISEdge](https://github.com/CIRISAI/CIRISEdge)
- **Fabric-core repos**:
  [CIRISRegistry](https://github.com/CIRISAI/CIRISRegistry) +
  [CIRISNodeCore](https://github.com/CIRISAI/CIRISNodeCore)
  (lens-core is now a CIRISServer workspace crate)
- **Agent** (`fabric node + brain`):
  [CIRISAgent](https://github.com/CIRISAI/CIRISAgent) — the H3ERE
  pipeline + the UI users interact with; consumes the CIRISServer wheel.

See `docs/for-developers.md` for the pointer map.

## Canonical references

- [Platform identity FSD](https://github.com/CIRISAI/CIRISNodeCore/blob/main/FSD/CEWP.md) — the load-bearing claims (in CIRISNodeCore)
- [Scaling model FSD](https://github.com/CIRISAI/CIRISNodeCore/blob/main/FSD/FEDERATION_SCALING_MODEL.md) — the quantitative bet
- [Anonymous tier v2 FSD](https://github.com/CIRISAI/CIRISNodeCore/blob/main/FSD/ANONYMOUS_TIER.md) — totalitarian-threat deniability path
- [CIRIS Constitution (CC 0.1.5)](https://github.com/CIRISAI/CIRISRegistry/tree/main/FSD/CIRIS_Constitution) — the top-of-stack canonical document: Accord 1.3-RC2 + CEG 1.0-RC29, one version line
- [CEG 1.0-RC29 wire-format spec](https://github.com/CIRISAI/CIRISRegistry/tree/main/FSD/CEG) — the authoritative wire-format spec (in CIRISRegistry; 1+4 surface FROZEN)
- [The Accord](https://ciris.ai/ciris_accord.pdf) — ethical framework the substrate enforces
- [Research synthesis](https://ciris.ai/research-status/) — the empirical bet
- [CIRIS fabric benchmarks](https://cirisai.github.io/CIRISServer/) — live measured crypto / transport / storage numbers
- [CIRISConformance](https://github.com/CIRISAI/CIRISConformance) — cohabitation + CEG-profile conformance gating
- [Install / shipping clients](https://ciris.ai/install) — [iOS](https://apps.apple.com/us/app/cirisagent/id6758524415) · [Android](https://play.google.com/store/apps/details?id=ai.ciris.mobile) · pip · Docker

## License

[AGPL-3.0-or-later](LICENSE) — same as the rest of the CIRIS stack.
The substrate is mission-locked: you can use it, you can modify it,
you cannot use it to capture the substrate.

## Engaging

- High-level positioning / discussion: [CIRISAgent#839](https://github.com/CIRISAI/CIRISAgent/issues/839) (Agent 3.0 / CEWP umbrella tracker)
- Simulation-engine implementation: issues in this repo (once it lands)
- Substrate-sister implementation: appropriate sister repo
- Wire-format proposals: CIRISRegistry (CEG 1.0-RC29; §11.2 amendment process)

The soup is on. Come participate.
