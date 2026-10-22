# For developers

*Reconciled to the CIRIS Constitution 1.0-rc9 (tag
[`v1.0-rc9`](https://github.com/CIRISAI/CIRISConstitution/tree/v1.0-rc9)).
References are written "CC <section>"; where this page and the
Constitution disagree, the Constitution wins.*

CEWP is the platform identity for the CIRIS fabric node
(**`agent = fabric node + brain`**, CC 8.1.1). The normative text is
the CIRIS Constitution; the implementations live in the underlying
repos — the substrate trio (Verify · Persist · Edge), the fabric cores
that cohabit via CIRISServer, and the agent. This doc is the pointer
map.

## The authoritative documents

### The CIRIS Constitution (normative)

- **[CIRIS Constitution 1.0-rc9](https://github.com/CIRISAI/CIRISConstitution/tree/v1.0-rc9)**
  — the **top-of-stack canonical document**: the Accord's ethics and
  the CEG wire grammar in one document, one version line. CEG is no
  longer a separate spec: implementers conform to **Parts 2 to 5**
  (CC Executive Summary, *Who this is for*).
  - Part 1 — Foundation: M-1 (sustainable adaptive coherence, CC 1.1),
    the six principles, the adversary model and privacy non-goals
    (CC 1.13.3)
  - Part 2 — The grammar: the envelope (CC 2.1), the conformance
    profiles CCP / CCC / CCS (CC 2.2), the five row types
    (CC 2.4), the signed object (CC 2.6.1.7), versioning (CC 2.6.4)
  - Part 3 — The namespace: dimensions, communities and the trust
    root (CC 3.2), consent (CC 3.3.1), minors (CC 3.4.13)
  - Part 4 — Composition & governance: anti-patterns (CC 4.1), the
    accord and the halt (CC 4.2), composition policies (CC 4.4),
    amendment and governance discipline (CC 4.5)
  - Part 5 — Transport & substrate: structural invisibility (CC 5.2),
    endpoints and the trust-root bundle (CC 5.3), the scope-native
    privacy wire profile including announce suppression (CC 5.4)
  - Parts 6 to 8 — the coherence mathematics and the holonomic
    substrate (CC 6), lifecycle and stewardship (CC 7), appendices,
    glossary and interoperability profiles (CC 8)
- **The five row types**: `scores`, `delegates_to`, `supersedes`,
  `withdraws`, `recants`. The slot is closed; a substrate refuses any
  other type at admission with `attestation_type_unregistered`, and a
  component that needs a new type files it against CC 2.4 before it
  ships (CC 2.4). Quorum is an envelope property, not a row type
  (CC 2.4).
- **Interop**: "speak CEG inside, standards at the edge"; C2PA is the
  one boundary profile written in full (CC 8.4).
- **[Executive Summary](https://github.com/CIRISAI/CIRISConstitution/blob/v1.0-rc9/constitution/EXECUTIVE_SUMMARY.md)**
  — the eleven promises in plain words, each with its status
  (works today / being built / a rule, no code yet / modelled) and the
  sections and checks that carry it.
- **[formal/](https://github.com/CIRISAI/CIRISConstitution/tree/v1.0-rc9/formal)**
  — thirteen machine-checked models of the governance rules, with the
  state table and invariant list.
- **[The Accord](https://ciris.ai/ciris_accord.pdf)** — the ethical
  commitment the substrate enforces (the ethics component of the
  Constitution). M-1 meta-goal grounding.

### Platform-level FSDs (design documents)

- **[FSD/CEWP.md](https://github.com/CIRISAI/CIRISNodeCore/blob/main/FSD/CEWP.md)**
  (in CIRISNodeCore) — the platform-identity FSD. Three load-bearing
  claims, the fabric-node architecture, the holonomic substrate, the
  datacenter-elimination claim, the superalignment positioning, what
  CEWP is NOT.
- **[FSD/FEDERATION_SCALING_MODEL.md](https://github.com/CIRISAI/CIRISNodeCore/blob/main/FSD/FEDERATION_SCALING_MODEL.md)**
  (in CIRISNodeCore) — the quantitative model. Replication discipline
  (trust × capacity intake; popularity × freshness eviction). L0/L1
  tier model. Scaling scenarios up to 5B users. Identity-aware-storage
  thesis. Prior-art comparison. Its outputs are model results, not
  measurements.
- **[FSD/ANONYMOUS_TIER.md](https://github.com/CIRISAI/CIRISNodeCore/blob/main/FSD/ANONYMOUS_TIER.md)**
  (in CIRISNodeCore) — the **planned** opt-in tier (Sphinx onion
  routing) for unobservability against a global passive adversary at
  federation scope. CC 1.13.3.1 names it as the mechanism for
  deployments that need that resistance; it is not yet specified
  normatively in CC and not yet shipped, and it is not a default or a
  requirement. Anonymity toward outsiders at the smallest cohort scope
  is already the default, and the opt-in boundary is provisional
  (CC 1.13.3.4). Multi-hop anonymity is not provided today; this
  planned tier is the opt-in path.
- **[Lightnet / darknet](https://github.com/CIRISAI/CIRISEdge/blob/main/docs/LIGHTNET_DARKNET.md)**
  (in CIRISEdge) — the two-plane split (CC 5.4.6 *Position*) mapped
  onto Edge's shipped code, every claim labelled normative, shipped,
  measured or posited. It states what is not claimed: ballot secrecy
  among members, anonymity for the identity plane, and multi-hop
  anonymity (not provided today; the planned anonymous tier,
  FSD/ANONYMOUS_TIER.md, is the opt-in path).

### Empirical bet

- **[ciris.ai/research-status](https://ciris.ai/research-status/)** —
  the research synthesis. Reasoning-shape measurement claim. Corridor
  metaphor. k_eff math.

## Per-repo entry points

Version numbers below are the ones the Constitution's evidence is
pinned against at rc9. For anything newer, see each repository.

### CIRISServer v0.5.232 — the fabric-node runtime

- [README.md](https://github.com/CIRISAI/CIRISServer) — the headless
  cohabitation runtime (+ PyO3 wheel) that composes registry
  authority, lens observation and node consensus over one substrate
  (CC 3.4.7.1). See the repository for its roadmap.
- Identity surfaces it carries: FedCode, the QR-able kind-tagged
  identity shorthand (`CIRIS-V2-` / `V3-`; v1 `CIRIS-V1-` decodes as a
  node; CC 2.6.8, CC 8.1.1); `infra:*` owner-binding that cannot carry
  agency (CC 4.4.3.4.3); a new node serves and trusts only until
  owned (CC 3.2); `consent:replication` (CC 3.3.1).

### CIRISVerify — crypto + transparency

- [docs/BENCHMARKS.md](https://github.com/CIRISAI/CIRISVerify/blob/main/docs/BENCHMARKS.md) — measured costs (hybrid sign / verify, AEAD, Merkle ops)
- [docs/THREAT_MODEL.md](https://github.com/CIRISAI/CIRISVerify/blob/main/docs/THREAT_MODEL.md) — assumed adversary capabilities
- Version: see its repository.

### CIRISPersist v59.1.0 — storage substrate

- [CHANGELOG.md](https://github.com/CIRISAI/CIRISPersist/blob/main/CHANGELOG.md) — release notes, including the retirement tiers (noise-floor descent, `AggregationMetaV1`, `EjectionVerdict`; CC 6.1.2)
- [docs/PUBLIC_SCHEMA_CONTRACT.md](https://github.com/CIRISAI/CIRISPersist/blob/main/docs/PUBLIC_SCHEMA_CONTRACT.md) — the schema downstream consumers can rely on
- [docs/COHABITATION.md](https://github.com/CIRISAI/CIRISPersist/blob/main/docs/COHABITATION.md) — how the cohabitation triple works
- [docs/INTEGRATION_LENS.md](https://github.com/CIRISAI/CIRISPersist/blob/main/docs/INTEGRATION_LENS.md) — the H3ERE trace event integration

### CIRISEdge v43.1.1 — transport + dispatch + holonomic swarm

- [docs/LIGHTNET_DARKNET.md](https://github.com/CIRISAI/CIRISEdge/blob/main/docs/LIGHTNET_DARKNET.md) — the two planes, with claims and their proofs
- [docs/BENCHMARKS.md](https://github.com/CIRISAI/CIRISEdge/blob/main/docs/BENCHMARKS.md) — dispatch and content-fetch costs
- [docs/STANDARDS_COMPARISON.md](https://github.com/CIRISAI/CIRISEdge/blob/main/docs/STANDARDS_COMPARISON.md) — comparison to libp2p, NATS, iroh
- Realtime A/V chunk wire (SFrame + MLS, with RaptorQ loss recovery on that plane only; CC 5.3.3.2, CC 5.4.3) and the per-item replication / ALM substrate (CC 6.1.5, CC 6.1.6)
- [docs/THREAT_MODEL.md](https://github.com/CIRISAI/CIRISEdge/blob/main/docs/THREAT_MODEL.md)

<!-- rc9-sync: unverified, see CC ? — removed implementation names not in CC rc9 (`list_held_by`, `FountainSwarmRuntime`, the ~14 KB trace size, the inline_text_pipeline cost list) and the stale component versions (Verify v6.11.0, Persist v9.10.0, Edge v6.3.0, Server v0.5.30) -->

### CIRISNodeCore — consensus core (ciris-node-core)

- [MISSION.md](https://github.com/CIRISAI/CIRISNodeCore/blob/main/MISSION.md) — the crate's functions
- [CIRIS_FEDERATION.md](https://github.com/CIRISAI/CIRISNodeCore/blob/main/CIRIS_FEDERATION.md) — layer architecture companion
- [examples/scale_model.rs](https://github.com/CIRISAI/CIRISNodeCore/blob/main/examples/scale_model.rs) — the Rust toy that the JS toy in this repo derives from

### ciris-lens-core — observation (a CIRISServer crate)

- The standalone CIRISLens deployment is retired; lens-core lives
  in-tree. Its outputs are `detection:*` rows, validated, not
  adjudicated, and never sole evidence for an authority action
  (CC 3.4.7.1). The detectors that test what agents said against what
  they did are a rule with no code yet at rc9 (promise 2).

### CIRISRegistry — identity, build, licence, partner (ciris-registry-core)

- Carries identity, build, licence and partner records (CC 3.1.1). It
  is a conferred slice of a canonical node, not a root (CC 5.3.4).
- The CEG and Constitution text no longer live here; they are the
  [CIRIS Constitution](https://github.com/CIRISAI/CIRISConstitution/tree/v1.0-rc9).
- [docs/CEG_EXPLORATION_PAGE_PRIMER.md](https://github.com/CIRISAI/CIRISRegistry/blob/main/docs/CEG_EXPLORATION_PAGE_PRIMER.md) — the consumer surface for the wire format

### CIRISAgent — `fabric node + brain` (agent runtime + unified client)

- [README.md](https://github.com/CIRISAI/CIRISAgent) — H3ERE pipeline, the runtime humans + LLMs cohabit, the UI users interact with; consumes the CIRISServer PyO3 wheel rather than composing the cores itself
- [CIRIS Architecture paper](https://doi.org/10.5281/zenodo.18137161)
- [Coherence Ratchet paper](https://doi.org/10.5281/zenodo.18142668)

## Where to engage

- **High-level positioning** — CIRISAgent#839 (the Agent 3.0 / CEWP umbrella tracker)
- **Quantitative model questions** — CIRISNodeCore#23 (the substrate alignment audit) or comments on FEDERATION_SCALING_MODEL.md
- **Wire-format and constitutional proposals** — the [CIRISConstitution](https://github.com/CIRISAI/CIRISConstitution) repo (amendment process, CC 4.5.1; a new row type is filed against CC 2.4 before it ships)
- **Implementation work** — the appropriate substrate / fabric repo
- **Conformance test reports** — CIRISConformance repo

## How to read this stack if you're new

If you want one paragraph: read this README + [`docs/overview.md`](overview.md).

If you want one hour:
1. CEWP overview ([`overview.md`](overview.md))
2. The Constitution's [Executive Summary](https://github.com/CIRISAI/CIRISConstitution/blob/v1.0-rc9/constitution/EXECUTIVE_SUMMARY.md) (the eleven promises)
3. [CIRIS_FEDERATION.md](https://github.com/CIRISAI/CIRISNodeCore/blob/main/CIRIS_FEDERATION.md) (the layer architecture)
4. Play with the [interactive toy](../toy/index.html) to feel the scale

If you want one day:
1. [The CIRIS Constitution](https://github.com/CIRISAI/CIRISConstitution/tree/v1.0-rc9), Parts 1 to 5 (the foundation and the wire grammar)
2. [FSD/FEDERATION_SCALING_MODEL.md](https://github.com/CIRISAI/CIRISNodeCore/blob/main/FSD/FEDERATION_SCALING_MODEL.md) (the quantitative model)
3. [FSD/CEWP.md](https://github.com/CIRISAI/CIRISNodeCore/blob/main/FSD/CEWP.md) (the platform identity)
4. The substrate-sister orientation issues (the per-repo positioning)
5. The research synthesis paper

If you want to contribute: pick a substrate / fabric repo + read its
current open issues. The substrate trio is moving fast; the fabric
cores (now cohabiting via CIRISServer) set the pace.
