# For developers

CEWP is the platform identity for the CIRIS fabric node
(**`agent = fabric node + brain`**). If you want to dig into the
technical substrate, every load-bearing FSD lives in one of the
underlying repos — the substrate trio (Verify · Persist · Edge), the
fabric cores that cohabit via CIRISServer, and the agent. This doc is
the pointer map.

## The authoritative FSDs

### Platform-level

- **[FSD/CEWP.md](https://github.com/CIRISAI/CIRISNodeCore/blob/main/FSD/CEWP.md)**
  (in CIRISNodeCore) — the platform-identity FSD. Three load-bearing
  claims, the fabric-node architecture, the holonomic substrate, the
  datacenter-elimination claim, the superalignment positioning, what
  CEWP is NOT.
- **[FSD/FEDERATION_SCALING_MODEL.md](https://github.com/CIRISAI/CIRISNodeCore/blob/main/FSD/FEDERATION_SCALING_MODEL.md)**
  (in CIRISNodeCore) — the quantitative model. CEG-organic replication
  discipline (trust × capacity intake; popularity × freshness
  eviction). L0/L1 tier model. Scaling scenarios up to 5B users.
  Identity-aware-storage thesis. Prior-art comparison.
- **[FSD/ANONYMOUS_TIER.md](https://github.com/CIRISAI/CIRISNodeCore/blob/main/FSD/ANONYMOUS_TIER.md)**
  (in CIRISNodeCore) — the opt-in GPA-unobservability (Sphinx onion)
  tier for totalitarian-threat contexts. Cohort-scoped anonymity is
  already the default (CC 1.13.3.4); this is the residual opt-in, with
  the default-promotion cost (bandwidth-free, latency-bound) being
  measured.

### Wire format

- **[CEG 1.0-RC29](https://github.com/CIRISAI/CIRISRegistry/tree/main/FSD/CEG)**
  (in CIRISRegistry) — the 20-section (§0–§19) authoritative wire-
  format spec. The **1+4 surface is FROZEN** as of RC1 (scores +
  delegates_to + supersedes + withdraws + recants). §5 dimension
  namespace. §9 humanity_accord. New §18 interop ("speak CEG inside,
  standards at the edge"; C2PA via `evidence_refs[]`) and §19
  holonomic (RaptorQ fountain / ALM / noise-floor retirement,
  normative). Implementers pin against RC29; after 1.0 SemVer binds
  strictly (MAJOR only for wire-incompatible change).

### Ethical framework

- **[CIRIS Constitution (CC 0.4)](https://github.com/CIRISAI/CIRISRegistry/tree/main/FSD/CIRIS_Constitution)**
  (in CIRISRegistry) — the **top-of-stack canonical document**: the
  CIRIS Accord 1.3-RC2 (ethics) + CEG 1.0-RC29 (grammar) woven into
  one version line. The first complete cut (all 10 annexes migrated,
  references resolved). M-1 apex (sustainable adaptive coherence);
  *peaked in purpose, flat in power*.
- **[The Accord](https://ciris.ai/ciris_accord.pdf)** — the ethical
  commitment the substrate enforces (the ethics component of the
  Constitution). M-1 meta-goal grounding.

### Empirical bet

- **[ciris.ai/research-status](https://ciris.ai/research-status/)** —
  the research synthesis. Reasoning-shape measurement claim. Corridor
  metaphor. k_eff math. The trace commons.

## Per-repo entry points

### CIRISServer — the fabric-node runtime

- [README.md](https://github.com/CIRISAI/CIRISServer) — the headless
  cohabitation runtime (+ PyO3 wheel) that binds the three fabric
  cores over one persist Engine + one edge identity. Current **v0.5.30**
  (config-as-CEG + registry; the v6.3.0 substrate unlock wires the
  holonomic swarm into the live node); roadmap 0.1 lens-only → 0.3
  +auth/one-wheel → 0.4 +peering → 0.5 +config-as-CEG/registry → 1.0 +node.
- Adds hardware-rooted identity (YubiKey/TPM → `CIRIS-V2-` fedcode),
  NodeCode (`CIRIS-V1-…` QR add-by-code), `infra:*` owner-binding,
  serve-only-until-owned, `consent:replication`.

### CIRISVerify-family v6.11.0 — crypto + transparency

- [docs/BENCHMARKS.md](https://github.com/CIRISAI/CIRISVerify/blob/main/docs/BENCHMARKS.md) — measured costs (hybrid sign 466 µs, hybrid verify 276 µs, AES-GCM 5.45 GiB/s, Merkle ops)
- [docs/THREAT_MODEL.md](https://github.com/CIRISAI/CIRISVerify/blob/main/docs/THREAT_MODEL.md) — assumed adversary capabilities

### CIRISPersist v9.10.0 — storage substrate

- [CHANGELOG.md](https://github.com/CIRISAI/CIRISPersist/blob/main/CHANGELOG.md) — release notes (the identity-aware-storage seam `list_held_by` + `evict_actor`, plus the holonomic retirement tiers — noise-floor descent / `AggregationMetaV1` / `EjectionVerdict`, CEG §19.7)
- [docs/PUBLIC_SCHEMA_CONTRACT.md](https://github.com/CIRISAI/CIRISPersist/blob/main/docs/PUBLIC_SCHEMA_CONTRACT.md) — the schema downstream consumers can rely on
- [docs/COHABITATION.md](https://github.com/CIRISAI/CIRISPersist/blob/main/docs/COHABITATION.md) — how the cohabitation triple works
- [docs/INTEGRATION_LENS.md](https://github.com/CIRISAI/CIRISPersist/blob/main/docs/INTEGRATION_LENS.md) — the H3ERE trace event integration (~14 KB per agent decision)

### CIRISEdge v6.3.0 — transport + dispatch + holonomic swarm

- [docs/BENCHMARKS.md](https://github.com/CIRISAI/CIRISEdge/blob/main/docs/BENCHMARKS.md) — dispatch_inbound, outbound_enqueue, content_fetch_roundtrip, inline_text_pipeline costs
- [docs/STANDARDS_COMPARISON.md](https://github.com/CIRISAI/CIRISEdge/blob/main/docs/STANDARDS_COMPARISON.md) — comparison to libp2p, NATS, iroh
- realtime A/V chunk wire (SFrame + MLS) + the §19 RaptorQ fountain / ALM substrate, with the live `FountainSwarmRuntime` (publisher + converger) driving swarm-rarity convergence, repair, and revoke→hard-delete for holographic replication
- [docs/THREAT_MODEL.md](https://github.com/CIRISAI/CIRISEdge/blob/main/docs/THREAT_MODEL.md)

### CIRISNodeCore — consensus core (ciris-node-core; folds in at Server 1.0)

- [MISSION.md](https://github.com/CIRISAI/CIRISNodeCore/blob/main/MISSION.md) — the crate's six functions
- [CIRIS_FEDERATION.md](https://github.com/CIRISAI/CIRISNodeCore/blob/main/CIRIS_FEDERATION.md) — layer architecture companion
- [examples/scale_model.rs](https://github.com/CIRISAI/CIRISNodeCore/blob/main/examples/scale_model.rs) — the Rust toy that the JS toy in this repo derives from

### ciris-lens-core — observation (absorbed in-tree as a CIRISServer crate)

- The standalone CIRISLens deployment is retired; lens-core now lives
  in-tree. F-3 detector family + Coherence Ratchet / Capacity Score
  (validated, not adjudicated). Pin against the issues.

### CIRISRegistry — CEG + Constitution spec authority (ciris-registry-core)

- [FSD/CEG/](https://github.com/CIRISAI/CIRISRegistry/tree/main/FSD/CEG) — the wire format spec (CEG 1.0-RC29; 20 sections, 1+4 FROZEN)
- [FSD/CIRIS_Constitution/](https://github.com/CIRISAI/CIRISRegistry/tree/main/FSD/CIRIS_Constitution) — the top-of-stack canonical document (CC 0.4)
- [docs/CEG_EXPLORATION_PAGE_PRIMER.md](https://github.com/CIRISAI/CIRISRegistry/blob/main/docs/CEG_EXPLORATION_PAGE_PRIMER.md) — the consumer surface for the wire format

### CIRISAgent — `fabric node + brain` (agent runtime + unified client)

- [README.md](https://github.com/CIRISAI/CIRISAgent) — H3ERE pipeline, the runtime humans + LLMs cohabit, the UI users interact with; consumes the CIRISServer PyO3 wheel rather than composing the cores itself
- [CIRIS Architecture paper](https://doi.org/10.5281/zenodo.18137161)
- [Coherence Ratchet paper](https://doi.org/10.5281/zenodo.18142668)

## Where to engage

- **High-level positioning** — CIRISAgent#839 (the Agent 3.0 / CEWP umbrella tracker)
- **Quantitative model questions** — CIRISNodeCore#23 (the substrate alignment audit) or comments on FEDERATION_SCALING_MODEL.md
- **Wire-format proposals** — CIRISRegistry repo (CEG 1.0-RC29; §11.2 amendment process)
- **Implementation work** — the appropriate substrate / fabric repo
- **Conformance test reports** — CIRISConformance repo

## How to read this stack if you're new

If you want one paragraph: read this README + [`docs/overview.md`](overview.md).

If you want one hour:
1. CEWP overview (this doc)
2. [CIRIS_FEDERATION.md](https://github.com/CIRISAI/CIRISNodeCore/blob/main/CIRIS_FEDERATION.md) (the layer architecture)
3. [The CIRIS Constitution](https://github.com/CIRISAI/CIRISRegistry/tree/main/FSD/CIRIS_Constitution) (the ethical framework + grammar, one version line)
4. Play with the [interactive toy](../toy/index.html) to feel the scale

If you want one day:
1. CEG 1.0-RC29 spec (the wire format authority)
2. [FSD/FEDERATION_SCALING_MODEL.md](https://github.com/CIRISAI/CIRISNodeCore/blob/main/FSD/FEDERATION_SCALING_MODEL.md) (the quantitative model)
3. [FSD/CEWP.md](https://github.com/CIRISAI/CIRISNodeCore/blob/main/FSD/CEWP.md) (the platform identity)
4. The substrate-sister orientation issues (the per-repo positioning)
5. The research synthesis paper

If you want to contribute: pick a substrate / fabric repo + read its
current open issues. The substrate trio is moving fast; the fabric
cores (now cohabiting via CIRISServer) set the pace.
