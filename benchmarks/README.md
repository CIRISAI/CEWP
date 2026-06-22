# Benchmarks & conformance evidence (vendored)

Raw artifacts behind the README's **Validation & evidence** section. Vendored
copies — the home repo stays canonical; re-pull before relying on a number.

| File | Source | What it is |
|---|---|---|
| [`ciris-server-bench.json`](ciris-server-bench.json) | `CIRISServer/bench-site/data.json` | The CIRISServer scoreboard export (schema `ciris-server/bench/2`) — crypto handshake (KEX), AV frame seal/open, fan-out, membership rekey, mesh room-cost, and replication-ingest figures. Drives <https://cirisai.github.io/CIRISServer/>. |
| [`ciris-server-fabric-conformance.json`](ciris-server-fabric-conformance.json) | `CIRISServer/qa/fabric_results.json` | The fabric conformance run — per-endpoint PASS / N-A / SKIP verdicts across 10 modules. Latest: **13 PASS / 0 FAIL** (30 N-A, 1 SKIP) over 44 checks. |

## Provenance & honesty notes

- **Single-core.** Every `bench/2` figure is one thread on one core; real
  deployments fan out across cores. Receivers are charged **open-only**.
- **100% hybrid PQC, hard cut** — Ed25519+ML-DSA-65 signatures,
  X25519+ML-KEM-768 KEM, no classical-only path.
- **Labels matter.** The scoreboard tags each metric `MEASURED` / `MODEL` /
  `PROJECTED` / `FRONTIER`. The README table preserves those labels. Notably,
  membership-change rekey is **PROJECTED** from the real hybrid-KEM primitive
  (the substrate does not implement join/leave rekey yet — CIRISEdge#129), and
  the 50-person-room receive cost is a **MODEL** over a stated GOP.
- **The `bench/2` export is a subset.** It does not carry the ALM-relay and
  fountain-reconstruction rows shown in the README; those come from the same
  CIRISServer suite (`benches/alm_chain.rs` and the `FountainPolicy` reference
  `N=20 / K=6 / H=30` in `src/benchmarks/scoreboard.rs`).
- The committed `data.json` carried `commit: "localtest"` / `date: 2026-06-16`
  — a local scoreboard run, not a tagged CI SHA. Re-pull from CIRISServer for
  the current published board.
