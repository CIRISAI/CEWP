# CEWP

**CIRIS Epistemic Web Platform.** Pronounced **"soup."**

A decentralized replacement for the internet — and simultaneously
an AI governance / superalignment platform — built on the bet that
**big tech is not necessary**.

## See it move

- [**Billions of people and devices, one rulebook (Landing Page)**](https://cirisai.github.io/CEWP/) — 5 billion people and 11.25 billion devices (adjustable from `100 M` to `8.1 B` people) on a land-bounded world population density grid (`2,961` cells, `181` localities, `9` GSMA/UN regions). Every device evaluates the same constitutional rulebook in WebAssembly across **CC §5.4.3 1-hop shaped budget classes** (`server` 7,000 B/s, `home` 2,800 B/s, `phone-wifi` 1,400 B/s, `phone-cellular` 350 B/s, `seat` 33,000 B/s), ordinary links, **CC §6.1.6 `deterministic-alm` member relay trees** over sealed chunks, and cover-replaced 15,296 B X-Wing onion walks (`+0 B` extra wire bytes), with expandable `☰ Controls`, `☰ Metrics`, and direct pill links to how it works and how it compares to other decentralized meshes.
- [**How it works & compared to other decentralized meshes**](https://cirisai.github.io/CEWP/how-it-works.html) — tabs for [how it solves 11 structural problems of decentralized networks](https://cirisai.github.io/CEWP/how-it-works.html#solves) (each with its mechanism, existing standards, and stated residual), [how CEWP compares to BitTorrent/IPFS, ActivityPub/Matrix, Nostr/SSB, and Tor/Nym](https://cirisai.github.io/CEWP/how-it-works.html#peers) alongside the Verify/Persist/Edge substrate tables, [the 6-person + 1-agent queue/view story in motion](https://cirisai.github.io/CEWP/how-it-works.html#motion), and [the contract](https://cirisai.github.io/CEWP/how-it-works.html#contract) ([`docs/why-it-works.md`](docs/why-it-works.md), [`docs/epistemic-web.md`](docs/epistemic-web.md), CIRISConstitution `rc17`).
- [**A mesh run by its kernel**](https://cirisai.github.io/CEWP/mesh.html) — people and their devices as cells, rows as signals travelling one hop per tick, every node folding what it holds with the same order-free function; click any node for its owner's queue and a preview that equals execution. Driven by [`docs/kernel/kernel.js`](docs/kernel/kernel.js) (one file, zero dependencies) over the constitution's own tables (`node --test docs/kernel/kernel.test.js`).
- [**Interactive scaling & slot-supply toy**](https://cirisai.github.io/CEWP/toy/) — per-server feasibility gates (`1 TB / 1 Gbps / 1 core`), 8 Constitutional opt-in trade-offs (`+` benefit / `−` downside), reachable-home share input, and 1-hop shaped onion cover capacity.

## What CEWP is

CEWP is an *epistemic web*: a **holonomic**, cryptographically-
accountable federation where humans, AI agents, and organizations are
first-class participants with the same identity shape. Every
load-bearing claim ("this content is accurate," "this peer is
trusted," "this action should be deferred to a human") is a signed
wire-format artifact in one small grammar (CEG's **"1+4"**); the
federation's collective weighted-aggregate scoring of those artifacts
is what *governs* the system in real time.

Nothing here relies on uninvented cryptography. Hybrid post-quantum
signatures (`Ed25519 + ML-DSA-65`), hybrid KEMs (`X-Wing`), CBOR/COSE
statements, RFC 9162 Merkle trees, whole-chunk sealed storage (`CC §5.4.3, §6.1.5`), and
Poisson-timed cover traffic all exist today. **What is new is the
arrangement** — five load-bearing properties that make decentralization,
privacy, and AI accountability reinforce each other instead of trading off:

1. **One rulebook (`the queue is the door run forwards`).**
   Every networked app normally writes its rules twice — once in the UI
   that draws the buttons and once in the server that authorizes the
   request — and their drift is where authorization bugs live. In CEWP
   (CC 4.5.13), a client renders only two served shapes, the **view** and
   the **queue**, and holds no policy. One constitutional table
   ([`transitions.json`](docs/kernel/tables/transitions.json) +
   [`governance_policy.json`](docs/kernel/tables/governance_policy.json))
   runs backward at every node's boundary as the admission check
   (`door`) and forward to generate the user's actionable items
   (`queue`), with `preview` equal to execution over a permutation-invariant
   fold. Nothing offered is refused; nothing refused was offered; a
   stranger sees nothing.
2. **Privacy and scale come from the same primitive (~92% locality dividend).**
   Content at `self` and `family` scope (~65% of typical activity) never
   emits a discovery attestation (`holds_bytes`), and community rooms
   (~30%) are predominantly local. As a result, **~92% of signed acts
   never leave the signer's locality** (CC 5.2), and the 1% of acts in
   5,000-member public rooms fan out over the **CC §6.1.6 `deterministic-alm`
   member relay tree** ($O(\log_k N)$ fan-out where every member computes
   the same deterministic tree over sealed chunks, distinct from CC §5.3.3.2's
   realtime profile which has no selective-forwarding relay role — no seat
   or third party decodes or selects) rather than unicasting from a central
   server. Privacy is not a tax on scale; privacy is *why* 5 billion people
   and 11.25 billion devices fit on commodity hardware without datacenters.
3. **CC §5.4.3 1-hop shaped budget classes + zero-extra-byte onion cover (CC §5.4.6 / Edge v44).**
   Per **CC §5.4.3 ("A shaped link is one hop")**, a shaped link is one
   direct hop between its two ends, never forwarded through a third node,
   at one of five constitutional budget classes (`server` `7,000 B/s`,
   `home` `2,800 B/s`, `phone-wifi` `1,400 B/s`, `phone-cellular` `350 B/s`,
   and `seat` `33,000 B/s`, each at $\rho = 0.5$, with $W = 600\text{ s}$ or
   $3,600\text{ s}$ on cellular; channel payload MDU $\approx 400\text{ B}$ at
   Reticulum base MTU or $\approx 16\text{ KB}$ / `16,272 B` over TCP).
   The shaped tier carries identity, consent, chat, and small attachments
   ($\le 64\text{ KiB}$); A/V and bulk ride unshaped only; a peer with no
   direct path gets no shaped link and (like a node not opted in) carries
   the same classes on ordinary links. When a peer walks a 3-hop `15,296 B`
   X-Wing (`CONP`) onion route from a sticky contact guard, each hop
   replaces a scheduled cover frame on an already-shaped link (**`+0 B`
   extra wire bytes**). `formal/transport_tiers/Tiers.lean` proves the
   8-link bound, displacement (`home_displaces_a_phone`), carriage and
   referral reach (`each_home_adds_seven`, `unowned_only_strands_the_73rd`),
   and the **`reach ≠ capacity`** theorem; combined with an explicit
   **reachable-home share** input (direct IPv6 or punched UDP path;
   `leviculum#100` NAT-mix data, `leviculum#101` no PCP/NAT-PMP/UPnP),
   directly reachable servers and homes supply billions of 1-hop shaped
   slots without stranding at the 73rd peer.
4. **`agent = fabric node + brain` (alignment as an epistemic-web problem).**
   Every node is a **fabric node** that stores, witnesses, degrades, and
   transports — *mechanically, never reasoning*. Add a reasoning brain
   and it becomes an agent that signs only its own rows, never inherits
   a human's standing, and roots its authority in an accountable human
   who can narrow or withdraw consent in one forward-only step.
5. **Every choice and every claim states its downside and residual.**
   In the queue schema (`manifests/choice_item.cddl`), every option must
   carry a non-empty `benefit` and `downside` before a person signs (e.g.,
   link shaping hides timing/volume `+`, and costs continuous cover
   gigabytes/month and Poisson frame wait `−`). Likewise, the substrate's
   peer comparisons and threat models state explicitly where it is
   **ahead**, **at parity**, and **behind** (including open exposures like
   `F-AV-ECLIPSE` until multi-medium transport lands and no third-party
   crypto audit yet).

**No center, no DNS, no load-bearing server.** You address a peer by
its key, reach it over the mesh, and trust it through a chain you can
re-root at will. Content is stored as sealed whole chunks (`CC 5.4.3`,
`CC 6.1.5`) across consenting holders (`custody:ack:v1`; `holds_bytes` is
retired and refused per `CC 3.1.3.3`) replicated to the multiplicity floor
($n_{\text{eff}} \ge 3$ across independent ASN/jurisdiction/signer axes, $R_9 \ge 9$
holders for root durability per `CC 6.1.2.1.2`). Layered-codec fidelity (`CC 6.1.2`,
`CLM-noise-classify`) defines `Full` / `Partial` / `EnvelopeOnly` chunk classification
by rate-distortion $(R, \varepsilon)$ over `ChunkLayer` sets rather than symbol counts,
while live A/V sealed chunks are never fragmented into symbols (`CC 5.3.3.2.3`).
Memory is forever but still
forgets: revocation, eviction, expiry, and aging are *one* descent toward
a **noise floor**, keeping all of history as a memory pyramid at `O(log T)`.

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

Single-core Criterion microbenchmarks from the CIRISServer scoreboard
(schema `bench/2`) back the crypto, transport, and storage claims. The raw
export is vendored at
[`benchmarks/ciris-server-bench.json`](benchmarks/ciris-server-bench.json)
(single-core, one thread; 100% hybrid PQC — Ed25519+ML-DSA-65 sigs,
X25519+ML-KEM-768 KEM, no classical-only path; receivers charged open-only):

| What | Result | Label | Claim it supports |
|---|---|---|---|
| Hybrid X25519 + ML-KEM-768 handshake | **161.9 µs** total (PQC tax **+83.2 µs** over classical 78.7 µs; once per peer-link) | MEASURED | Post-quantum-ready identity at negligible cost |
| AV frame seal + open throughput | up to **2.24 GiB/s** / core (64 KiB frame) | MEASURED | Crypto is not the bottleneck |
| 1080p keyframe seal → wire → open | **112.3 µs** (256 KiB; seal 56.8 + open 49.0) | MEASURED | Full-motion video on commodity cores |
| A 50-person room, 720p/30fps | **~0.45%** of one core to receive (open-only); ≤ **10,823** concurrent 30fps streams/core | MODEL | Group media without datacenters |
| Shared-inner-seal fan-out (N=50) | **2.07×** vs naive per-recipient seal | MEASURED | 1:N multicast amortizes |
| Membership-change rekey (tree, N=50) | **0.41 ms** (8.3× vs flat rewrap) | PROJECTED | Churn cost stays sublinear (CIRISEdge#129) |
| ALM relay per-hop overhead | **~946 ns** (~0.96 µs/tier, `O(log N)` tree) | MEASURED | Mesh transport scales without datacenters |
| Whole-chunk sealed storage custody | **Multiplicity floor** ($n_{\text{eff}} \ge 3$, $R_9 \ge 9$, CC §6.1.2.1.2) | MODEL | Consented whole-chunk survival without fragmentation |
| Hybrid trace ingest | **230.6 µs** (~**4,336** traces/sec/core; dedup 205.8 µs) | MEASURED | Signed-artifact governance throughput |

The scoreboard labels every row `MEASURED` vs. `MODEL` vs. `PROJECTED` vs.
`FRONTIER` — it is explicit about **what is measured, what is modeled, and
what is not yet built** (e.g. full symmetric M>2 MDC video is `FRONTIER`;
membership rekey is `PROJECTED` from the real hybrid-KEM primitive against
[CIRISEdge#129](https://github.com/CIRISAI/CIRISEdge/issues/129)). The ALM
figures come from the benchmark suite (`benches/alm_chain.rs`), where ALM fan-out
$k = 8$ and 50M-viewer numbers are CEWP scaling model assumptions (`MODEL`),
while `CLM-alm-live-tree-verified` is staged (`CIRISEdge#945`), and chunk storage
enforces the $n_{\text{eff}} \ge 3$ and $R_9 \ge 9$ multiplicity floor (`CC §6.1.2.1.2`).

### Conformance — [CIRISConformance](https://github.com/CIRISAI/CIRISConformance)

Cohabitation + CEG-profile conformance is **gated against the published
wheels**. The latest fabric run is vendored at
[`benchmarks/ciris-server-fabric-conformance.json`](benchmarks/ciris-server-fabric-conformance.json):
**13 PASS / 0 FAIL** (30 N/A, 1 SKIP) across 44 checks over 10 modules
(auth · sdk · telemetry · agent · system · memory · audit · tools · guidance ·
ceg-native-fabric). The suite verifies the five PyO3 substrate cores
(persist · verify · edge · node-core · lens-core) cohabit correctly in one
process and conform to the CEG contract, across three CEG conformance
profiles — **CCP** (producer), **CCC** (consumer), **CCS** (substrate) — at
two tiers (substrate cohabitation + fabric emergent behavior: replication
discipline, scaling factors). Expected failures are marked `xfail` against
upstream issues rather than hidden.

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
| [`FSD/ANONYMOUS_TIER.md`](FSD/ANONYMOUS_TIER.md) | The opt-in GPA-unobservability tier — CC RC17 §5.4.6 / Edge v44 cover-replaced 3-hop X-Wing `CONP` onion routing (`15,296 B` fixed frame riding inside a `16 KiB` / `16,272 B` TCP shaped slot at **`+0 B` extra wire bytes**) + Ed25519 key blinding + AEAD + rendezvous discoverability. **Cohort-scoped anonymity is already the default (CC 1.13.3.4);** this is the residual federation-scope opt-in, whose 1-hop shaped cover capacity and latency cost both the [globe landing page](https://cirisai.github.io/CEWP/) and [toy](toy/index.html) model. |
| [`FSD/SIMULATION_ENGINE.md`](FSD/SIMULATION_ENGINE.md) | **v0.2 Simulation, Verification & Evidence Engine spec** — 9-layer architecture uniting **CC RC17 §4.5.13 executable state-processor authority** (`door_is_queue`, `preview_is_execution`, `refused_previews_nothing`), the **holistic CSD/4 (Card-first Specification Design) topology contract (`T0–T19`, catalogue version 6, 156 vocabulary atoms, 152 in state table, 110 promised across T0–T19, X1–X5 Cornerstone Invariants)** across `CIRISConstitution`, `CIRISServer`, `CIRISClient` (`101` cards = `43` CSD/4 + `58` bullet), `CEWPOS` (`4` cards = `105` total; `csd_namespace_map.json` is CEWP's synthesis), and `CEWP`, exact **transport (`CC §5.4.3 / §5.4.6`), integer-only member-relay `deterministic-alm` (`CC §6.1.6`), and whole-chunk sealed storage with layered-codec rate-distortion noise-floor descent (`CC §6.1.2`, CLM-noise-classify)**, compound-Poisson + IMIX workload math, **`CIRISHolon`-certified multiscale coarse-graining (`R5 → R0`)**, an **8-class adversarial & formal-exclusion test programme**, and machine-checkable **`evidence_bundle.json`** (`MEASURED` / `MODEL` / `PROJECTED` / `FRONTIER`). |

### Working toy

| Path | Purpose |
|---|---|
| [`examples/scale_model.rs`](examples/scale_model.rs) | The runnable scaling-model toy. `cargo run --example scale_model` (from CIRISNodeCore workspace; mirrored here for reference). **14 workload scenarios** (bootstrap → dunbar → media-heavy → twitter → news → full-internet → tiktok → youtube → netflix → adulthub → full-internet-with-video) **+ 3 regional-realism scenarios** (south-asia-dense, sub-saharan-bootstrap, north-america-realistic) **+ CC RC17 §5.4.3 budget classes (`server` `7,000 B/s`, `home` `2,800 B/s`, `phone-wifi` `1,400 B/s`, `phone-cellular` `350 B/s`, `seat` `33,000 B/s`, each at $\rho=0.5$, 1 direct hop per shaped link), CC §6.1.6 `deterministic-alm` member relay tree over sealed chunks (distinct from CC §5.3.3.2 realtime no-relay-role), cover-replaced X-Wing onion walks, and `reach_home_share` / no-direct-path leaf accounting (`formal/transport_tiers/Tiers.lean` proves `reach ≠ capacity`) + a threat-model pass + per-scenario environmental footprint**. All feasible per-server at v1 gates (1 TB / 1 Gbps / 1 core). |
| [`toy/index.html`](toy/index.html) | The **interactive** browser toy — pick a scenario, tune the top-level hardware gates (disk / bandwidth / core), and expand **▸ Advanced: Population Opt-In Choices (+/-), Device Tiers & Sustainability** to adjust device tier shares (`% server`, `% home`, `% phone`), phone Wi-Fi vs. cellular + charging share, directly reachable home share (`reach_home_share`), no-direct-path share, and the 8 Constitutional opt-in choices with live **No-Direct-Path Leaf Capacity** and **1-Hop Shaped Onion Cover Capacity** readouts. Mirrors the `scale_model.rs` math; no build step, just open it. |

### Explanation docs (for general audience + website team)

| Path | Purpose |
|---|---|
| [`docs/how-it-works.html`](docs/how-it-works.html) | What CEWP solves (11 structural failures), how it compares to BitTorrent/IPFS, ActivityPub/Matrix, Nostr/SSB, and Tor/Nym, and how it moves |
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
| [1 — Constitution](analysis/tier1-constitution/) | sovereignty, amendment, halting, anti-capture | Constitution 0.4, CIRISAccord, CEG §9/§11 |
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
| Single-point data loss | Whole-chunk sealed storage & multiplicity floor ($n_{\text{eff}} \ge 3$, $R_9 \ge 9$) across consenting holders; federation survives to one survivor |
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
| Storage duration | Platform-decided; deletion is a promise | **Effectively indefinite** — full-fidelity for centuries-to-millennia on one commodity server (replicated whole-chunk sealed storage with multiplicity floor $n_{\text{eff}} \ge 3$, $R_9 \ge 9$); the hardest "entire internet *with video* on one home server" run still holds ~1–2 yr at full fidelity, then degrades-not-deletes into the O(log T) memory pyramid forever ([scaling model §2.6](FSD/FEDERATION_SCALING_MODEL.md)) |

The simulation engine ([`FSD/SIMULATION_ENGINE.md`](FSD/SIMULATION_ENGINE.md)) is what produces
these numbers in animation-ready form — modeled at 1:1 over the
real internet topology (real metros, real submarine cables, real
peering points).

### CC §5.4.3 Budget Classes, 1-Hop Shaped Slot Supply & CC §6.1.6 `deterministic-alm` (CC RC17 / Edge v44)

Per **CIRISConstitution RC17** (§4.5.13, §5.3.3.2, §5.4.3, §5.4.6, §6.1.6, and [`formal/transport_tiers/Tiers.lean`](https://github.com/CIRISAI/CIRISConstitution)) and **CIRISEdge v44**, transport separates into the Constitution's named budget classes and fan-out rules:
1. **Device-tier 1-hop shaped links (`L ≤ 8`, CC §5.4.3)**: `server` (`7,000 B/s` = `605 MB/day`), `home` (`2,800 B/s` = `242 MB/day`), `phone-wifi` (`1,400 B/s` = `121 MB/day`), and `phone-cellular` (`350 B/s` = `30 MB/day`), each at cover fraction $\rho = 0.5$ (`W = 600 s`, or `3,600 s` on cellular; channel payload MDU `≈ 400 B` at the Reticulum `500 B` base MTU or `≈ 16 KB` / `16,272 B` over TCP). **A shaped link is 1 direct hop** between its two ends, never forwarded through a third node; a peer with no direct path gets no shaped link (and carries identity, consent, chat, and small attachments on ordinary links; A/V and bulk unshaped only).
2. **Serve-tier shaped (`seat`, CC §5.4.3)**: `33,000 B/s` (`2.85 GB/day/slot`, $\rho = 0.5$, `W = 600 s`) — canonical serve seats (`serve_max = 8`).
3. **Ordinary links & CC §6.1.6 `deterministic-alm` member relay tree**: Reticulum direct IPv6 / UDP-punched links (`leviculum#100` first empirical NAT-mix measurement; `leviculum#101` notes neither PCP/NAT-PMP nor UPnP is implemented) + $O(\log_k N)$ **CC §6.1.6 `deterministic-alm` member relay tree** (`k = 8`) over sealed chunks for large-$N$ rooms. The stream key pins the single publisher at depth 0; the relay tree is strictly a **member relay tree** (seats/forwarders are never fan-out hubs per `CC §5.4.6`). Fan-out $k=8$ and 50M-viewer broadcast figures are **CEWP scaling model assumptions** (`MODEL`), and `CLM-alm-live-tree-verified` is `staged` (`CIRISEdge#945`). Distinct from **CC §5.3.3.2**, where the realtime transport profile has **no selective-forwarding relay role** — no seat or third party decodes or selects.
4. **Cover-Replaced X-Wing Onion (`CONP`, `15,296 B`, CIRISEdge v44)**: 3-hop hybrid X25519 + ML-KEM-768 onion packets (`3 × 1,264 B` headers + `11,488 B` body) replace scheduled chaff frames on TCP shaped links (**`+0 B` extra wire bytes**; latency-only cost). Note: `CONP` frame sizing (`15,296 B`), `3 × 1,264 B` onion headers, and TCP channel payload MDU (`16,272 B / 500`) are attributed to **CIRISEdge v44** (CC §5.4.6 pins the functional onion contract and general bounds "about 400 bytes ... about 16 KB"). Sticky guards (`CC §5.4.6` item 4) are held as long as they answer (not rotated on a schedule); below the relay floor the tier is `onion_tier_unavailable_floor`. Proxy pull only proxies federation-tier rows (`CC §5.4.6`). Referral ranking is by inbound dial count descending (reachability), and link class agreement is the lower of the two offers (`min(local, peer)` per `CC §5.4.3`). Sybil resistance in CC is trust gating (`CC §1.13.3.3`) and stated residual (`CC §5.4.6`), not link-count shaping. Shaped links (`L ≤ 8`) form with offering nodes and any node that asks (`shaped_only` / Anonymous Tier), chat and files ride a shaped link only when one end is `shaped_only`, and the 9th peer is refused only on `shaped_only` nodes (running unshaped otherwise).

Note: [`formal/transport_tiers/Tiers.lean`](https://github.com/CIRISAI/CIRISConstitution) proves the 8-link bound (`L ≤ 8`), displacement (`unowned_only_strands_the_73rd`), carriage/referral reach (`each_home_adds_seven`), and the **`reach ≠ capacity`** theorem — it has no reachability term. Both [`docs/index.html`](docs/index.html), [`toy/index.html`](toy/index.html), and [`examples/scale_model.rs`](examples/scale_model.rs) therefore take **directly reachable home share (`reach_home_share`)** as an explicit scenario input alongside the 8 Constitutional opt-in choices at full-internet scale (**`5.00 B` people · `11.25 B` devices** at `2.25` devices/person across `self` rosters · `50,000` canonical serve seats):

| Regime (`5.00 B` people · `11.25 B` devices) | No-Direct-Path Leaves vs. Capacity | Reachable Home Share (`reach_home_share`) | 1-Hop Shaped Slots (`each_home_adds_seven` when reachable) | Direct 1-Hop Shaping Pool (`+0 B` wire) | Answering Onion Relays / Layer |
|---|---|---|---|---|---|
| **Fleet Defaults** (`5%` server, `55%` home, `40%` phone, `60%` no-direct-path, `90%` relay offer, `25%` home shaping, `12%` onion) | `6.41 B / 29.55 B` (**22% load**) | **40.0%** | `8.94 B` supplied vs. `2.78 B` demanded (**100% seated**) | `1.55 B` devices (`13.8%` of fleet; **2.66×** per onion device) | `773.52 M / layer` (floor $\ge 3$) |
| **High Privacy** (`8%` server, `60%` home, `75%` home shaping, `40%` phone shaping, `36%` onion) | `5.69 B / 41.19 B` (**14% load**) | **45.0%** | `17.41 B` supplied vs. `6.73 B` demanded (**100% seated**) | `3.74 B` devices (`33.3%` of fleet; **1.87×** per onion device) | `1.09 B / layer` (floor $\ge 3$) |
| **CGNAT / Mobile Stress** (`4%` server, `40%` home, `56%` phone, `85%` no-direct-path, `45%` Wi-Fi) | `9.18 B / 15.31 B` (**60% load**) | **15.0%** | `3.94 B` supplied vs. `1.35 B` demanded (**100% seated**) | `750.62 M` devices (`6.7%` of fleet; **2.59×** per onion device) | `276.32 M / layer` (floor $\ge 3$) |
| **Unowned-Only (`0%` Relay Offer — `unowned_only_strands_the_73rd`)** | `6.41 B / 1.80 M` (**999% load / stranded**) | **40.0%** | `400.00 K` supplied vs. `2.40 B` demanded (**0.02% seated; 99.98% displaced**) | `400.00 K` canonical slots only | `16.67 K / layer` |

In the live `100,000`-act rulebook simulation over `5.00 B` people / `11.25 B` devices (`cargo run --example scale_model`):
- **91.9% of acts** never leave the signer's locality (CC 5.2 locality dividend).
- **12.55 M local rulebook checks** run in **2.60 ms** (**4.82 B rulebook checks/sec** on 1 core; `3.10 K` stranger acts refused at the signer's own door).
- **Wire traffic** moves **7.24 GB** (`280.89 K` 1-hop device-shaped hops, `3.71 K` `seat` hops, `1.61 M` ordinary/`deterministic-alm` hops, `5.02 K` onion acts at `+0 B`), saving **10.49 M** unicast deliveries via the CC §6.1.6 `deterministic-alm` member relay tree (**6.6× less wire traffic** than a central datacenter at `47.86 GB`, and **574.49 M× less** than broadcasting to `11.25 B` devices at `3.87 EB`).

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
- [Anonymous tier FSD](https://github.com/CIRISAI/CIRISNodeCore/blob/main/FSD/ANONYMOUS_TIER.md) — the opt-in GPA-unobservability (Sphinx) tier (cohort anonymity is default, CC 1.13.3.4)
- [CIRIS Constitution (CC 0.4)](https://github.com/CIRISAI/CIRISRegistry/tree/main/FSD/CIRIS_Constitution) — the top-of-stack canonical document: Accord 1.3-RC2 + CEG 1.0-RC29, one version line (first complete cut + accord halt/resume surface)
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
