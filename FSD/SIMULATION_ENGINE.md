# FSD: CEWP Simulation Engine

**Status:** v0.1 design.
**Scope:** A modular, performant Rust simulation engine that models
the centralized internet AND the CEWP substrate on the same
real-world topology, at resolutions from one-thousand to
five-billion nodes, producing snapshot streams the website team
consumes via their existing Three.js / WebGL visualization stack
(`ciris-website/src/app/grammar/explore/AlephScene.tsx`).
**Audience:** sim engine implementers (this repo) + website team
(consumers of snapshot output).
**Goal:** make the bet visible. Run the same workload over both
topologies; show the centralized internet routing every byte
through ~10K hyperscale facilities, then show CEWP routing the same
workload through 500 M home-server-class nodes with no datacenters.
The math comes from
[CIRISNodeCore/FSD/FEDERATION_SCALING_MODEL.md](https://github.com/CIRISAI/CIRISNodeCore/blob/main/FSD/FEDERATION_SCALING_MODEL.md);
the topology comes from public datasets; the engine connects them.

---

## 0. Premise restated

Per [CEWP.md](https://github.com/CIRISAI/CIRISNodeCore/blob/main/FSD/CEWP.md):

> CEWP's premise: big tech is not necessary.
> CEWP's bet: cryptographic substrate + standardized ethical tracing
> prove it.

The simulation engine's job is to make the "big tech is not
necessary" claim **animatable**. Side-by-side, on a globe, with real
metros + real submarine cables + real IXPs. The same 5B users
generating the same workload — flowing through the centralized
internet (5 hyperscalers, ~10K datacenters, observable bottlenecks)
vs flowing through CEWP (federation of equals, no central party,
edges along the trust graph). The engine produces the numbers + the
snapshot stream; the website team renders.

---

## 1. Non-goals

- **Not a network simulator at packet level.** ns-3 / OMNeT++ /
  Mininet exist for that. The engine models aggregate flows, not
  packets.
- **Not a router simulator.** No BGP convergence, no IGP, no MPLS.
  We model the existing AS-level topology as a static graph and
  evolve demand on top of it.
- **Not a UI.** Website team owns the visualization layer (see §10).
- **Not a benchmark tool for any one substrate.** It compares two
  topologies under identical workload; the substrate sisters'
  CI benches are the ground truth for per-op costs (see §2 inputs).

---

## 2. Empirical inputs (measured + sourced)

### 2.1 Per-op compute costs (from substrate-sister benchmarks)

These are the load-bearing numbers the engine uses for per-node CPU
accounting. They come from the substrate sisters' published
benchmark suites:

| Op | Cost | Source |
|---|---|---|
| `hybrid_sign` (Ed25519 + ML-DSA-65) | 466 µs | [CIRISVerify v2.8.0](https://github.com/CIRISAI/CIRISVerify/blob/main/docs/BENCHMARKS.md) |
| `hybrid_verify` | 276 µs | same |
| AES-GCM @ 64 KiB | 5.45 GiB/s encrypt, 5.91 GiB/s decrypt | same |
| Persist SQLite per-row write | ~1.5 ms | [CIRISPersist](https://github.com/CIRISAI/CIRISPersist) v3.x storage_floor bench |
| Edge dispatch_inbound (256 B) | < 400 µs | [CIRISEdge v0.10.0+](https://github.com/CIRISAI/CIRISEdge/blob/main/docs/BENCHMARKS.md) |
| H3ERE trace per agent decision | ~14 KB | [CIRISPersist INTEGRATION_LENS.md](https://github.com/CIRISAI/CIRISPersist/blob/main/docs/INTEGRATION_LENS.md) |
| Scrub regex pass | 5–10 ns/byte | Edge inline_text_pipeline |

### 2.2 Topology inputs (real-world data)

The engine needs a globe-shaped backbone. Data sources:

| Dataset | What it provides | Format |
|---|---|---|
| **[GeoNames cities1000](https://download.geonames.org/export/dump/)** | ~150 K cities ≥ 1000 pop with lat/lon + population. The metros we drop CEWP nodes into. | Tab-separated; free; ~30 MB |
| **[PeeringDB](https://www.peeringdb.com/)** | ~1.3 K Internet Exchange Points + ~5 K interconnection facilities with metro location, capacity, member count. The IXPs centralized internet traffic flows through. | REST JSON API |
| **[CAIDA ITDK (2025-03)](https://www.caida.org/catalog/datasets/internet-topology-data-kit/release-2025-03/)** | AS-level topology from traceroutes (197 Ark monitors in 51 countries, March 2025). The peering graph between ~120 K ASNs. | TSV; account-gated but free |
| **[TeleGeography Submarine Cable Map (2025)](https://www.submarinecablemap.com/)** | 597 active/planned cable systems with 1712 landings, capacity per major route. KML for routes + Points for landings. | KML; free (full geocoded dataset is licensed) |
| **[Hurricane Electric BGP Toolkit](https://bgp.he.net/)** | AS relationships (provider / customer / peer). Refines the CAIDA peering graph. | Web; free |

### 2.3 Demand inputs (per the scaling model)

The per-user activity model is the same as
[FEDERATION_SCALING_MODEL.md §3](https://github.com/CIRISAI/CIRISNodeCore/blob/main/FSD/FEDERATION_SCALING_MODEL.md):

| Workload class | Avg envelope | Daily activity / user | Cohort distribution |
|---|---|---|---|
| `bootstrap` | 1.5 KB | 20 KB | default (65% local) |
| `dunbar_steady` | 1.5 KB | 50 KB | default |
| `media_heavy` | 8 KB | 500 KB | default |
| `twitter_scale` | 0.5 KB | 5 KB | default |
| `news_replacement` | 5 KB | 100 KB | default |
| `full_internet_v1` | 50 KB | 50 MB | default |
| `local_heavy` / `global_heavy` | varies | varies | local-heavy / global-heavy |

The engine accepts these as scenarios; each one specifies workload
class + trust topology + cohort distribution + tier mix. Users can
add custom scenarios.

---

## 3. The two topologies, modeled

The engine runs both topologies **simultaneously over the same
metro graph**, on the same workload, in the same time-stepping
loop. This is what makes the side-by-side comparison legible.

### 3.1 Centralized internet topology

| Element | Modeled as | Source |
|---|---|---|
| Hyperscale datacenters | ~10 K facilities, weighted by published capacity. Top 5 operators (AWS, Azure, GCP, Meta, Apple) hold ~70% of capacity. | PeeringDB facility data + public hyperscaler footprint |
| IXPs | ~1.3 K nodes at metro locations, capacity per member | PeeringDB IXP data |
| Submarine cables | 597 cable systems, 1712 landings, capacity per route | TeleGeography 2025 dataset |
| AS-level peering | ~120 K ASNs, link weights from CAIDA topology data | CAIDA ITDK 2025-03 |
| Terrestrial fiber | Major backbone routes inferred from BGP toolkit + ASN-to-metro adjacency | Hurricane Electric BGP toolkit |
| End users | Aggregated to metro nodes; per-metro user population from GeoNames + ITU broadband penetration estimates | GeoNames + ITU stats |

**Flow model:** every byte every user produces or consumes is routed
through their nearest hyperscaler facility's CDN, then through the
AS-level peering graph + submarine cables to its destination. Most
flows fan out from a small number of hyperscale origin/sink nodes —
that's the bottleneck the visualization makes visible.

### 3.2 CEWP topology

| Element | Modeled as | Source |
|---|---|---|
| Client nodes | Phones + tablets. Distributed by metro population. | GeoNames + ITU smartphone penetration |
| L0 proxy nodes (256 GB) | Laptops + small home servers. Distributed by metro population. | GeoNames + estimated laptop penetration |
| L1 server nodes (1 TB) | Home servers + small VPSes. Distributed by broadband-internet density. | GeoNames + ITU broadband stats |
| Trust topology | Small-world graph per Watts-Strogatz, R direct trust ≈ 150 (Dunbar), depth=1 default per scaling model | Synthetic; calibrated against scaling model |
| Transport links | Reticulum mesh (peer-to-peer over consumer ISPs) + HTTPS fallback for unreachable peers | Modeled as consumer broadband uplinks |
| Internet backbone reused | CEWP rides on the same submarine cables + terrestrial fiber, but at the consumer-ISP edge, not the hyperscale CDN edge | TeleGeography + ASN-to-metro |

**Flow model:** every byte is admitted at the recipient node's
trust × capacity gate, then propagates along the trust graph
according to cohort_scope (the locality dividend: 65% of traffic
never crosses metros). Bytes that DO cross metros traverse the
same submarine cables and terrestrial fiber as in the centralized
case — but originate and terminate at consumer endpoints, not
hyperscale facilities. The animation contrast: same fiber, no
datacenters in the middle.

---

## 4. The demand model

Per time step (default 1 sim-second = 1 wall-clock frame), each user
agent in the simulation:

1. Generates new content with probability `D / 86400` per second
   (where D = daily_bytes per user)
2. Each generated envelope has cohort_scope drawn from the scenario's
   distribution (default: 65% self/family, 15% community, 10% affiliations, 5% species, 3% planet, 2% federation)
3. Each envelope at `cohort_scope ≥ community` flows along the trust
   graph (fanout from author to admitting peers)
4. With probability `daily_fetch / 86400` per second, the user requests
   content from a random previously-generated envelope (weighted by
   popularity)
5. The two topologies route the flow differently (§3.1 vs §3.2)
6. Per-node bytes-held / bandwidth / CPU update accordingly

Agent decisions (the H3ERE pipeline) generate trace data at
~14 KB/decision, with `agent_decisions_per_day` per scenario. These
flow the same way as user-content, with the addition that LensCore
detectors run on aggregate trace streams per cohort.

---

## 5. Resolution gradient (single engine, many scales)

The same engine code runs at multiple resolutions; quantization is
a configuration knob, not a separate codebase.

| Resolution | Target | Hardware | Use case |
|---|---|---|---|
| **R0 (1:1 native)** | 5 B agents at 1:1 | GPU cluster (4-8 × A100/H100 80GB), or single H100 with batched streaming | Full-scale verification of the bet |
| **R1 (1:100 sampled)** | 50 M agents at 1:1 | Single A100 / RTX 4090 | Single-node deep simulation |
| **R2 (1:1000 sampled)** | 5 M agents at 1:1 | 64-core workstation CPU + rayon | Researcher reproducibility |
| **R3 (1:5000 sampled)** | 1 M agents at 1:1 | 16-core CPU | CI test loop |
| **R4 (animation grade)** | 10 K-100 K agents | Browser (WASM) | Website team's animation source |
| **R5 (illustrative)** | 1 K agents | Browser (WASM) | Public-facing interactive |

**Quantization is sampling, not aggregation.** R1 samples 1% of
users at 1:1; the metros they live in are real; their behavior is
individual. This preserves the visual distinctness the website
team needs (each dot is an agent, not an average).

**The website team consumes R4 or R5 snapshots** for the
visualization. The engine emits snapshots in the format §6
specifies; the team's Three.js InstancedMesh setup (already in
`AlephScene.tsx`) consumes them directly.

---

## 6. Snapshot output format

The engine emits a snapshot stream — one snapshot per simulated
time step — that the visualization consumes. Format chosen for
direct compatibility with the existing
`AlephScene.tsx` rendering (InstancedMesh per group + LineSegments
for arcs).

### 6.1 Per-snapshot binary layout (R4 / R5 — WASM-friendly)

Little-endian; aligned to 4-byte boundaries; suitable for direct
ArrayBuffer view in JS.

```
SnapshotHeader (16 bytes)
  u32 magic = 0xCE7C0501          // "CEWP snap, v1"
  u32 sim_time_seconds            // current sim time
  u32 node_count                  // N
  u32 edge_count                  // E

NodeArray (N × 32 bytes each)
  u32 node_id
  f32 lat, f32 lon                // metro location
  u8  topology = 0 (internet) | 1 (cewp)
  u8  tier     = 0 (client) | 1 (proxy/L0) | 2 (server/L1) | 3 (hyperscale_dc) | 4 (ixp)
  u8  state    = 0 (idle) | 1 (sending) | 2 (receiving) | 3 (signing) | 4 (verifying)
  u8  reserved
  u32 bytes_held                  // current storage occupancy (saturated to u32 max for R0)
  f32 bandwidth_in_bps            // rolling avg
  f32 bandwidth_out_bps
  f32 cpu_util_fraction           // 0..1
  u32 trust_set_size              // for CEWP nodes; 0 for internet nodes

EdgeArray (E × 24 bytes each)     // dynamic flows this step
  u32 src_node_id, u32 dst_node_id
  u8  topology = 0 (internet) | 1 (cewp)
  u8  flow_class = 0 (content) | 1 (attestation) | 2 (fetch) | 3 (trace)
  u16 reserved
  f32 bytes_per_second_this_step
  u32 cumulative_bytes_so_far
  f32 latency_ms
```

### 6.2 R0/R1/R2 native format

Same shape, but written to a memory-mapped Apache Arrow file for
random access + columnar compression. Node + edge arrays are
columnar; downstream tooling can query via DataFusion.

### 6.3 Snapshot cadence

Default: 1 snapshot per simulated minute → 1440 snapshots per
simulated day. At animation grade (R4 / R5), this is ~10 MB total
for a 1-day playback — easily streamable.

---

## 7. Workspace structure

Rust workspace at this repo root. Each crate is independently
testable + benchable + usable.

```
CEWP/
  Cargo.toml                       # workspace root
  crates/
    cewp-model/                    # pure math: scaling model formulas
      src/lib.rs                   #   - per-actor cost equations
                                   #   - cohort distribution + trust depth
                                   #   - benchmark constants
                                   # No I/O. No allocations on hot path.
                                   # Mirrors CIRISNodeCore/examples/scale_model.rs.

    cewp-topology/                 # real-world topology ingest
      src/lib.rs                   #   - GeoNames cities loader
      src/sources/peeringdb.rs     #   - PeeringDB API client + cache
      src/sources/caida.rs         #   - CAIDA ITDK loader
      src/sources/telegeography.rs #   - submarine cable KML loader
      src/graph.rs                 #   - metro/IXP/cable graph
      assets/                      #   - cached snapshots (cities1000.txt etc.)

    cewp-sim/                      # the simulation engine
      src/lib.rs
      src/agent.rs                 #   - Agent struct (16 bytes hot path)
      src/scheduler.rs             #   - time-stepping loop
      src/topology_router/         #   - centralized vs CEWP routing
      src/cpu/                     #   - rayon-parallel R1/R2/R3
      src/gpu/                     #   - wgpu compute shaders for R0/R1 [future]
      src/snapshot.rs              #   - §6 binary writer

    cewp-cli/                      # scenario runner
      src/main.rs                  #   $ cewp-cli run --scenario full_internet_v1 --resolution R3 --out snap.bin

    cewp-wasm/                     # WASM target for browser embedding [follow-up]
      src/lib.rs                   #   Same cewp-sim, compiled to wasm32-unknown-unknown
      Cargo.toml                   #   wasm-bindgen + js-sys

  scenarios/                       # YAML / TOML scenario presets
    full_internet_v1.toml
    dunbar_steady.toml
    twitter_scale.toml
    ...

  FSD/
    SIMULATION_ENGINE.md           # this doc

  docs/                            # platform-identity docs (already shipped)
    overview.md
    seven-repos.md
    ...
```

### 7.1 Workspace dependencies

```toml
[workspace.dependencies]
rayon       = "1.10"
serde       = { version = "1", features = ["derive"] }
serde_json  = "1"
toml        = "0.8"
csv         = "1"
quick-xml   = "0.36"        # for KML
reqwest     = { version = "0.12", features = ["json"] }
geo         = "0.28"        # for haversine + lat/lon math
petgraph    = "0.6"         # for topology graph
arrow       = "53"          # columnar snapshots (R0-R2)
bytemuck    = "1"           # zero-copy binary snapshots
thiserror   = "1"
tracing     = "0.1"
ulid        = "1"
chrono      = "0.4"
sha2        = "0.10"
ed25519-dalek = "2"         # for realistic per-sign cost in CPU accounting
```

### 7.2 Optional / feature-gated

- `wgpu = "22"` — for GPU compute (R0 / R1 target); behind `gpu` feature
- `wasm-bindgen = "0.2"` + `js-sys = "0.3"` — for WASM target; behind `wasm` feature
- `criterion = "0.5"` — for benches; dev-dep

---

## 8. Performance targets

| Resolution | Target wall-clock per simulated day | Hardware |
|---|---|---|
| R0 (5 B agents, 1:1) | < 1 minute on 4×H100 80GB | GPU cluster |
| R1 (50 M agents) | < 30 seconds on single A100 | GPU |
| R2 (5 M agents) | < 30 seconds on 64-core CPU | CPU + rayon |
| R3 (1 M agents) | < 10 seconds on 16-core CPU | CI loop |
| R4 (100 K agents) | < 5 seconds in browser | WASM |
| R5 (1 K agents) | 60 fps in browser | WASM |

### 8.1 What makes these achievable

- **Agent struct: 16 bytes hot path**. The full state per agent
  (current_held_bytes, last_action_time, trust_set_seed, demand_state)
  packs into 16 bytes. At 5 B × 16 = 80 GB, single H100 holds it.
- **Time step is per-second granularity**, not per-millisecond.
  86400 steps per simulated day; each step is a parallel reduce
  over all agents.
- **Trust graph is small-world, not stored per-edge**. Each agent's
  R direct trust is generated from a deterministic seed (Watts-
  Strogatz with metro-locality bias). No O(N²) edge storage.
- **Topology is sparse + static**. Metro graph has ~5 K nodes; AS
  peering graph ~120 K nodes. Both fit in memory; routing
  precomputed.
- **Snapshot emission is the I/O bottleneck**. At R0 we only emit
  R4-grade quantized snapshots for visualization — the full state
  is in GPU memory and never serialized.

---

## 9. Scenario configuration

Scenarios are TOML files in `scenarios/`. Schema:

```toml
[scenario]
name = "full_internet_v1"
description = "5B users, 50 MB/user/day, default cohort, depth-1 server"

[population]
n_users = 5_000_000_000
metro_distribution = "geonames_cities500"
broadband_penetration = "itu_2024"

[tier_mix]
client = 0.35
proxy_l0 = 0.55
server_l1 = 0.10

[workload]
daily_bytes_per_user = 52_428_800        # 50 MB
avg_envelope_bytes = 51_200              # 50 KB
agent_decisions_per_day = 200
daily_fetch_bytes = 1_073_741_824        # 1 GB

[cohort]
self = 0.50
family = 0.15
community = 0.15
affiliations = 0.10
species = 0.05
planet = 0.03
federation = 0.02

[trust]
direct_radius = 250
server_recursion_depth = 1
trace_publishable_fraction = 0.10

[disk_budgets]
client_bytes = 274_877_906_944           # 256 GB
proxy_l0_bytes = 274_877_906_944         # 256 GB (L0 default)
server_l1_bytes = 1_099_511_627_776      # 1 TB

[output]
resolution = "R4"                        # snapshot grade
snapshot_cadence_seconds = 60            # 1 per simulated minute
output_path = "out/full_internet_v1.snap.bin"
```

CLI:

```bash
cewp-cli run --scenario scenarios/full_internet_v1.toml
cewp-cli run --scenario scenarios/full_internet_v1.toml --resolution R3   # override
cewp-cli compare --baseline scenarios/internet_today.toml --candidate scenarios/full_internet_v1.toml
```

---

## 10. Output contract with the website team

The website team's existing rendering stack
(`ciris-website/src/app/grammar/explore/AlephScene.tsx`) uses React
Three Fiber + Three.js with `InstancedMesh` per node group +
`LineSegments` for edges. The snapshot format §6 maps directly:

| Snapshot field | RTF/Three.js consumer |
|---|---|
| `NodeArray.lat, lon` | InstancedMesh position via globe projection |
| `NodeArray.tier` | InstancedMesh group selection (one mesh per tier) |
| `NodeArray.state` | Color / material variant |
| `NodeArray.cpu_util_fraction` | Pulse / brightness |
| `EdgeArray.src/dst` | LineSegments endpoints |
| `EdgeArray.bytes_per_second` | LineSegments thickness / opacity |
| `EdgeArray.topology` | Internet edges in one LineSegments mesh; CEWP edges in another (separate animation layers) |

The website team:

* Loads the snapshot binary as an `ArrayBuffer`
* Decodes via DataView (each snapshot is a fixed-size header + two flat arrays)
* Updates InstancedMesh `instanceMatrix` + per-instance attributes
* `invalidate()` triggers a re-render
* Frame-loop = "demand" so the GPU isn't spinning when paused

No per-frame allocations on the JS side. The Rust engine produces
the data; the website team renders it. Clean separation.

---

## 11. Calibration to the scaling model

The engine must reproduce the scaling-model toy's per-actor numbers
exactly. The `cewp-model` crate is a Rust port of
[`CIRISNodeCore/examples/scale_model.rs`](https://github.com/CIRISAI/CIRISNodeCore/blob/main/examples/scale_model.rs)
with the same constants + the same formula structure.

The calibration test: for each scaling-model scenario (bootstrap,
dunbar_steady, media_heavy, twitter_scale, news_replacement,
full_internet_v1, full_internet_local_heavy, full_internet_global_
heavy, village_dense), running `cewp-sim` at R3 with the corresponding
TOML scenario produces aggregate per-tier numbers within 5% of the
toy's analytic output. This is the unit test that says "the
simulation is the model, evolved over time."

---

## 12. Roadmap

| Phase | Deliverable | Target |
|---|---|---|
| **0** | This FSD | done |
| **1** | `cewp-model` + `cewp-topology` skeleton + first scenario at R3 (1 M agents, CPU) | week 1-2 |
| **2** | `cewp-sim` time-stepping + snapshot writer + the centralized internet baseline routing | week 3-4 |
| **3** | CEWP routing (trust-graph + cohort_scope) + side-by-side comparison output | week 5 |
| **4** | R3/R4 reproduction of all 9 scaling-model scenarios + calibration test | week 6 |
| **5** | Website team integration: snapshot stream consumed by `AlephScene.tsx` variant | coordinated |
| **6** | `cewp-wasm` build + R4/R5 in-browser | follow-up |
| **7** | `gpu` feature for R1/R0 via wgpu | follow-up |

---

## 13. Open questions

These get resolved during phase 1 implementation:

1. **Metro count**. 5 K (top-3% by population) vs 50 K (cities ≥ 5K
   pop) vs 150 K (cities ≥ 1K pop). The right cut depends on what
   the website team's visualization can carry without losing
   legibility.
2. **Submarine cable capacity model**. TeleGeography publishes
   total capacity per route; modeling per-cable utilization
   requires assumptions. Initial pass: equal share among the cable's
   landing countries.
3. **Trust topology calibration**. The Watts-Strogatz parameters
   (`R = 150`, rewiring probability) reproduce Dunbar but the
   metro-locality bias parameter affects how many cross-metro
   edges appear. Initial pass: 30% of trust edges within own
   metro, 50% within same time zone, 20% globally.
4. **Diurnal patterns**. Default scenario assumes uniform 24h
   activity. Realistic patterns (8 AM-11 PM local) shift the
   bandwidth peaks. Initial pass: uniform; refinement: diurnal in
   phase 4.
5. **AS-level peering reduction**. CAIDA's 120 K ASN graph is
   overkill for visual purposes. Initial reduction: collapse
   ASNs to their parent organization (~10 K), then to their
   primary metro presence.

---

## 14. References

### Internal

- [CEWP.md (CIRISNodeCore)](https://github.com/CIRISAI/CIRISNodeCore/blob/main/FSD/CEWP.md) — the platform-identity FSD
- [FEDERATION_SCALING_MODEL.md (CIRISNodeCore)](https://github.com/CIRISAI/CIRISNodeCore/blob/main/FSD/FEDERATION_SCALING_MODEL.md) — the analytic model the simulation reproduces
- [`examples/scale_model.rs` (CIRISNodeCore)](https://github.com/CIRISAI/CIRISNodeCore/blob/main/examples/scale_model.rs) — the analytic toy this engine generalizes
- [`AlephScene.tsx` (ciris-website)](https://github.com/CIRISAI/ciris-website/blob/main/src/app/grammar/explore) — the rendering layer this engine feeds

### External — topology data sources

- [PeeringDB](https://www.peeringdb.com/) — IXPs + facilities ([API docs](https://docs.peeringdb.com/))
- [CAIDA ITDK 2025-03](https://www.caida.org/catalog/datasets/internet-topology-data-kit/release-2025-03/) — AS-level topology
- [TeleGeography Submarine Cable Map](https://www.submarinecablemap.com/) — 597 cable systems, 1712 landings
- [Hurricane Electric BGP Toolkit](https://bgp.he.net/) — AS relationships
- [GeoNames](https://download.geonames.org/export/dump/) — cities + population

### External — substrate benchmarks (cost inputs)

- [CIRISVerify BENCHMARKS.md](https://github.com/CIRISAI/CIRISVerify/blob/main/docs/BENCHMARKS.md)
- [CIRISEdge BENCHMARKS.md](https://github.com/CIRISAI/CIRISEdge/blob/main/docs/BENCHMARKS.md)
- [CIRISPersist INTEGRATION_LENS.md](https://github.com/CIRISAI/CIRISPersist/blob/main/docs/INTEGRATION_LENS.md)
