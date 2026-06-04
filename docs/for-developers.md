# For developers

CEWP is the platform identity for seven repos. If you want to dig
into the technical substrate, every load-bearing FSD lives in one
of those repos. This doc is the pointer map.

## The authoritative FSDs

### Platform-level

- **[FSD/CEWP.md](https://github.com/CIRISAI/CIRISNodeCore/blob/main/FSD/CEWP.md)**
  (in CIRISNodeCore) — the platform-identity FSD. Three load-bearing
  claims, the seven-repo architecture, the datacenter-elimination
  claim, the superalignment positioning, what CEWP is NOT.
- **[FSD/FEDERATION_SCALING_MODEL.md](https://github.com/CIRISAI/CIRISNodeCore/blob/main/FSD/FEDERATION_SCALING_MODEL.md)**
  (in CIRISNodeCore) — the quantitative model. CEG-organic replication
  discipline (trust × capacity intake; popularity × freshness
  eviction). L0/L1 tier model. Scaling scenarios up to 5B users.
  Identity-aware-storage thesis. Prior-art comparison.
- **[FSD/ANONYMOUS_TIER.md](https://github.com/CIRISAI/CIRISNodeCore/blob/main/FSD/ANONYMOUS_TIER.md)**
  (in CIRISNodeCore) — v2 deniability path for totalitarian-threat
  contexts. Parallel anonymous publication tier sketched. Not v1
  scope; design committed.

### Wire format

- **[CEG 0.10 PWD](https://github.com/CIRISAI/CIRISRegistry/tree/main/FSD/CEG)**
  (in CIRISRegistry) — the 18-section authoritative wire-format
  spec. 1+4 primitive lockdown (held through ten minor versions).
  §5 dimension namespace. §9 humanity_accord. The 0.10 delivery axis
  (`delivery_mode` / streaming multicast, §10.5) is the latest
  addition. Implementers pin against the 0.x series.

### Ethical framework

- **[The Accord](https://ciris.ai/ciris_accord.pdf)** — the ethical
  commitment the substrate enforces. M-1 meta-goal grounding.

### Empirical bet

- **[ciris.ai/research-status](https://ciris.ai/research-status/)** —
  the research synthesis. Reasoning-shape measurement claim. Corridor
  metaphor. k_eff math. The trace commons.

## Per-repo entry points

### CIRISVerify — crypto + transparency

- [docs/BENCHMARKS.md](https://github.com/CIRISAI/CIRISVerify/blob/main/docs/BENCHMARKS.md) — measured costs (hybrid sign 466 µs, hybrid verify 276 µs, AES-GCM 5.45 GiB/s, Merkle ops)
- [docs/THREAT_MODEL.md](https://github.com/CIRISAI/CIRISVerify/blob/main/docs/THREAT_MODEL.md) — assumed adversary capabilities

### CIRISPersist — storage substrate

- [CHANGELOG.md](https://github.com/CIRISAI/CIRISPersist/blob/main/CHANGELOG.md) — release notes (v3.5.0 just shipped the identity-aware-storage seam: `list_held_by` + `evict_actor`)
- [docs/PUBLIC_SCHEMA_CONTRACT.md](https://github.com/CIRISAI/CIRISPersist/blob/main/docs/PUBLIC_SCHEMA_CONTRACT.md) — the schema downstream consumers can rely on
- [docs/COHABITATION.md](https://github.com/CIRISAI/CIRISPersist/blob/main/docs/COHABITATION.md) — how the cohabitation triple works
- [docs/INTEGRATION_LENS.md](https://github.com/CIRISAI/CIRISPersist/blob/main/docs/INTEGRATION_LENS.md) — the H3ERE trace event integration (~14 KB per agent decision)

### CIRISEdge — transport + dispatch

- [docs/BENCHMARKS.md](https://github.com/CIRISAI/CIRISEdge/blob/main/docs/BENCHMARKS.md) — dispatch_inbound, outbound_enqueue, content_fetch_roundtrip, inline_text_pipeline costs
- [docs/STANDARDS_COMPARISON.md](https://github.com/CIRISAI/CIRISEdge/blob/main/docs/STANDARDS_COMPARISON.md) — comparison to libp2p, NATS, iroh
- [docs/THREAT_MODEL.md](https://github.com/CIRISAI/CIRISEdge/blob/main/docs/THREAT_MODEL.md)

### CIRISNodeCore — federation consensus

- [MISSION.md](https://github.com/CIRISAI/CIRISNodeCore/blob/main/MISSION.md) — the crate's six functions
- [CIRIS_FEDERATION.md](https://github.com/CIRISAI/CIRISNodeCore/blob/main/CIRIS_FEDERATION.md) — layer architecture companion
- [examples/scale_model.rs](https://github.com/CIRISAI/CIRISNodeCore/blob/main/examples/scale_model.rs) — the Rust toy that the JS toy in this repo derives from

### CIRISLensCore — detection + science

- [README.md](https://github.com/CIRISAI/CIRISLensCore) — overall positioning + status
- The F-3 detector family + CEG §5.5 slice are in active design; pin against the issues.

### CIRISRegistry — CEG + identity

- [FSD/CEG/](https://github.com/CIRISAI/CIRISRegistry/tree/main/FSD/CEG) — the wire format spec (18 sections)
- [docs/CEG_EXPLORATION_PAGE_PRIMER.md](https://github.com/CIRISAI/CIRISRegistry/blob/main/docs/CEG_EXPLORATION_PAGE_PRIMER.md) — the consumer surface for the wire format

### CIRISAgent — agent runtime AND unified client (seventh repo, both roles)

- [README.md](https://github.com/CIRISAI/CIRISAgent) — H3ERE pipeline, mode selector, the runtime humans + LLMs cohabit, the UI users interact with
- [CIRIS Architecture paper](https://doi.org/10.5281/zenodo.18137161)
- [Coherence Ratchet paper](https://doi.org/10.5281/zenodo.18142668)

## Where to engage

- **High-level positioning** — CIRISAgent#839 (the Agent 3.0 / CEWP umbrella tracker)
- **Quantitative model questions** — CIRISNodeCore#23 (the substrate alignment audit) or comments on FEDERATION_SCALING_MODEL.md
- **Wire-format proposals** — CIRISRegistry repo (CEG 0.10 PWD; §11.2 amendment process)
- **Implementation work** — the appropriate substrate / fabric repo
- **Conformance test reports** — CIRISConformance repo

## How to read this stack if you're new

If you want one paragraph: read this README + [`docs/overview.md`](overview.md).

If you want one hour:
1. CEWP overview (this doc)
2. [CIRIS_FEDERATION.md](https://github.com/CIRISAI/CIRISNodeCore/blob/main/CIRIS_FEDERATION.md) (the layer architecture)
3. [The Accord](https://ciris.ai/ciris_accord.pdf) (the ethical framework)
4. Play with the [interactive toy](../toy/index.html) to feel the scale

If you want one day:
1. CEG 0.10 spec (the wire format authority)
2. [FSD/FEDERATION_SCALING_MODEL.md](https://github.com/CIRISAI/CIRISNodeCore/blob/main/FSD/FEDERATION_SCALING_MODEL.md) (the quantitative model)
3. [FSD/CEWP.md](https://github.com/CIRISAI/CIRISNodeCore/blob/main/FSD/CEWP.md) (the platform identity)
4. The substrate-sister orientation issues (the per-repo positioning)
5. The research synthesis paper

If you want to contribute: pick a substrate / fabric repo + read its
current open issues. The substrate sisters are moving fast; the
fabric sisters set the pace.
