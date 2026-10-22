# CEWP

**CIRIS Epistemic Web Platform.** Pronounced **"soup."**

A decentralized replacement for the internet, and at the same time an
AI governance platform, built on the bet that **big tech is not
necessary**.

> **Pinned to the CIRIS Constitution 1.0-rc9** (tag
> [`v1.0-rc9`](https://github.com/CIRISAI/CIRISConstitution/tree/v1.0-rc9)).
> CEG, the wire grammar, is no longer a separate spec with its own
> version line: it is Parts 2 to 5 of the Constitution. Section
> references below are written "CC <section>". Where this repo and the
> Constitution disagree, the Constitution wins.

## What CEWP is

CEWP is an *epistemic web*: the network formed when nodes exchange
signed rows in the CIRIS grammar (CEG) over Edge/Reticulum transport.
It is not a product, a server or a central service; it has no steward,
no root and no load-bearing instance (CC 8.1.1).

Every load-bearing claim ("this content is accurate," "this peer is
trusted," "this action should be deferred to a human") is a signed row
of one of five kinds: `scores`, `delegates_to`, `supersedes`,
`withdraws`, `recants` (CEG's **"1+4"**). The list is closed; a
substrate refuses any sixth kind, and a group's authority is the
group's members signing the same row, not a separate record type
(CC 2.4). Each reader composes those rows under a trust policy it
chooses; there is no registry whose entry makes a key trusted, and a
fresh key has no weight until someone the reader trusts confers it
(CC 1.13.3.3, CC 4.4.3.8).

Humans, AI agents and organizations take part under the same eleven
promises, written once for human and AI alike
([CC Executive Summary](https://github.com/CIRISAI/CIRISConstitution/blob/v1.0-rc9/constitution/EXECUTIVE_SUMMARY.md)).
They are not interchangeable: an agent or node key must have a bound
human steward before it can join any non-infrastructure community
(CC 3.2 steward-binding gate), a child has a parent or guardian bound
by a signed row (CC 3.4.13), and one human voice counts once while an
agent has no vote (promise 6).

**No center, no DNS, no load-bearing server.** You address a peer by
its key, reach it over the mesh, and trust it through a root you chose
and can leave in one step; anyone may found another root (CC 3.2,
promise 4). The network runs on two planes (CC 5.4.6 *Position*;
CIRISEdge
[`docs/LIGHTNET_DARKNET.md`](https://github.com/CIRISAI/CIRISEdge/blob/main/docs/LIGHTNET_DARKNET.md)):

- **Lightnet**, the public identity plane: key records, announced
  transport destinations, trust roots. It is attributable and makes no
  anonymity claim. Every inbound message is attributed to a key before
  any handler sees it, and a first-contact device is "unowned but
  never anonymous" (CC 3.2 (e)).
- **Darknet**, the derived group plane: a group's destinations are
  derived by per-group HKDF over the cached directory and never
  announced, so the group's existence is invisible to non-members.
  Members relay for members.

An item its owner marks durable is held, chunk by chunk, by a chosen
number of nodes in its own cohort and repaired when that number falls,
so the witnessed, trust-anchored corpus recovers from any one holder;
anonymous-tier content is exempt and may truly disappear
(CC 6.1, CC 6.1.5). Revocation, eviction, expiry and aging are one
operation at different rates, a descent toward a **noise floor**: a
revoked item must fall below individual recoverability at every
retained tier, while the collective gist can survive as a memory
pyramid (CC 6.1.2). That pyramid costs Θ(T) if every level is kept and
is sublinear only under bounded retention per level; the earlier
`O(log T)` figure was withdrawn (CC 6.1.2).

Agents live **inside** CEWP; they don't have CEWP as a tool. A **fabric
node** attests, stores, observes, reaches consensus and transports, but
does not reason or act: **`agent = fabric node + brain`** (CC 8.1.1).
A node that co-locates authority, observation and consensus gains
neither a vote nor a verdict; authority stays with human-held keys
under quorum (CC 3.4.7.1).

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

**The fabric node** ([CIRISServer](https://github.com/CIRISAI/CIRISServer),
v0.5.232) is the headless cohabitation runtime that composes
registry authority, lens observation and node consensus over one
substrate (CC 3.4.7.1); the substrate trio (Verify · Persist · Edge)
and the agent stay separate repos. The default trust root,
`ciris-canonical`, has the three accord holders as its founders under a
2-of-3 quorum; the canonical installs are members with standing and no
vote (CC 3.2). Its genesis bundle is at version 4, re-minted by a
2-of-3 amend on 2026-10-20 (CC 5.3.4). Independent witnesses for that
root are not yet in place (CC Executive Summary).

The software runs on the phones and computers people already own
(CC Executive Summary). This repo's scaling model estimates that the
full internet (5 billion users) fits on about one server per ten
humans, the density home-internet and IoT deployments already reach;
in that model, **no datacenters are required**. These are model
outputs, not measurements
([`FSD/FEDERATION_SCALING_MODEL.md`](FSD/FEDERATION_SCALING_MODEL.md)).

## The premise and the bet

**Premise:** big tech is not necessary.
**Bet:** cryptographic substrate + standardized ethical tracing
prove it.

Not "we want to replace big tech." Not "big tech could be better."
**Big tech is not necessary.** The Constitution states its own bet in
the same direction: of the two roads to safe powerful AI, train values
you trust, or assume the model may be wrong and make its consequential
acts checkable by parties it does not control, CIRIS takes the second
(CC Executive Summary, *The bet*). The empirical case that reasoning
has a shape we can measure is articulated at
[**ciris.ai/research-status**](https://ciris.ai/research-status/).

## Validation & evidence

Much of the substrate is **measured**, **conformance-gated**, and
**shipping to end users today**. This section puts that evidence next
to the claims it supports; the raw artifacts live in the sister repos
and on the live benchmark page.

At the Constitution level: thirteen machine-checked models of the
governance rules, 152 properties over 157 states, and a twenty-entry
topology catalogue the server's harness stands up
(CIRISConstitution
[`formal/README.md`](https://github.com/CIRISAI/CIRISConstitution/blob/v1.0-rc9/formal/README.md)).
Of the eleven promises, only the halt is measured end to end on real
nodes (promise 4).

### Measured performance — [CIRIS fabric benchmarks](https://cirisai.github.io/CIRISServer/)

Single-core Criterion microbenchmarks from the CIRISServer scoreboard
(schema `bench/2`) back the crypto, transport, and storage claims. The raw
export is vendored at
[`benchmarks/ciris-server-bench.json`](benchmarks/ciris-server-bench.json)
(single-core, one thread; 100% hybrid PQC — Ed25519+ML-DSA-65 sigs,
X25519+ML-KEM-768 KEM, no classical-only path; receivers charged open-only).
CC requires the same posture: every record is signed Ed25519 +
ML-DSA-65, every content key is wrapped X25519 + ML-KEM-768, and every
byte over a link rides a hybrid transit seal; there is no classical
lane (CC 5.3.3.5 E1).

| What | Result | Label | Claim it supports |
|---|---|---|---|
| Hybrid X25519 + ML-KEM-768 handshake | **161.9 µs** total (PQC tax **+83.2 µs** over classical 78.7 µs; once per peer-link) | MEASURED | Post-quantum-ready identity at negligible cost |
| AV frame seal + open throughput | up to **2.24 GiB/s** / core (64 KiB frame) | MEASURED | Crypto is not the bottleneck |
| 1080p keyframe seal → wire → open | **112.3 µs** (256 KiB; seal 56.8 + open 49.0) | MEASURED | Full-motion video on commodity cores |
| A 50-person room, 720p/30fps | **~0.45%** of one core to receive (open-only); ≤ **10,823** concurrent 30fps streams/core | MODEL | Group media without datacenters |
| Shared-inner-seal fan-out (N=50) | **2.07×** vs naive per-recipient seal | MEASURED | 1:N multicast amortizes |
| Membership-change rekey (tree, N=50) | **0.41 ms** (8.3× vs flat rewrap) | PROJECTED | Churn cost stays sublinear (CIRISEdge#129) |
| ALM relay per-hop overhead | **~946 ns** (~0.96 µs/tier, `O(log N)` tree) | MEASURED | Mesh transport scales without datacenters |
| Durable storage (per-item replication, CC 6.1.5) | pending re-run under replication | MODEL | An item is recoverable while one of its holders is live (CC 6.1.5.1) |
| Hybrid trace ingest | **230.6 µs** (~**4,336** traces/sec/core; dedup 205.8 µs) | MEASURED | Signed-artifact governance throughput |

<!-- rc9-sync: superseded by replication; re-run needed — the durable-storage row's former figure (99.6% reconstruction at 33% fragment loss, N=20/K=6/H=30) modelled coded storage, which CC 6.1.5 no longer uses -->

The scoreboard labels every row `MEASURED` vs. `MODEL` vs. `PROJECTED` vs.
`FRONTIER`, so it says **what is measured, what is modeled, and
what is not yet built** (e.g. full symmetric M>2 MDC video is `FRONTIER`;
membership rekey is `PROJECTED` from the real hybrid-KEM primitive against
[CIRISEdge#129](https://github.com/CIRISAI/CIRISEdge/issues/129)). The ALM
figure comes from `benches/alm_chain.rs`. Stored content is never
coded: nodes hold and forward sealed chunks, and the rateless codec
serves realtime A/V only (CC 5.4.2, CC 6.1.5); a layered item sheds
whole layers, so degradation is a property of the layering (CC 6.1).

### Conformance — [CIRISConformance](https://github.com/CIRISAI/CIRISConformance)

Cohabitation + CEG-profile conformance is **gated against the published
wheels**. The latest fabric run is vendored at
[`benchmarks/ciris-server-fabric-conformance.json`](benchmarks/ciris-server-fabric-conformance.json):
**13 PASS / 0 FAIL** (30 N/A, 1 SKIP) across 44 checks over 10 modules
(auth · sdk · telemetry · agent · system · memory · audit · tools · guidance ·
ceg-native-fabric). The suite verifies the five PyO3 substrate cores
(persist · verify · edge · node-core · lens-core) cohabit correctly in one
process and conform to the CEG contract, across the three conformance
profiles CC 2.2 defines — **CCP** (producer), **CCC** (consumer), **CCS**
(substrate) — at two tiers (substrate cohabitation + fabric emergent
behavior: replication discipline, scaling factors). Expected failures
are marked `xfail` against upstream issues rather than hidden.

### Shipping clients — [ciris.ai/install](https://ciris.ai/install)

CIRISAgent is not a lab exercise; it ships to end users on both mobile stores:

- **iOS** — [CIRIS Agent on the App Store](https://apps.apple.com/us/app/cirisagent/id6758524415)
- **Android** — [CIRIS Agent on Google Play](https://play.google.com/store/apps/details?id=ai.ciris.mobile)
- **Desktop / server** — `pip install ciris-agent`, Docker, or from source (see the [install guide](https://ciris.ai/install))

## What's in this repo

This repo is the **platform-identity + FSD + simulation home** for
CEWP. The technical substrate lives in the sister repos (linked
below), and the normative text lives in the
[CIRIS Constitution](https://github.com/CIRISAI/CIRISConstitution/tree/v1.0-rc9).
This repo is what you read to understand the platform as a whole, what
you fork to run simulations of it, and where the cross-cutting FSDs
(those that span more than one substrate repo) are mirrored for easy
cross-reference.

### FSDs (mirrored from CIRISNodeCore)

These are design documents. Each carries a status line saying how it
stands against CC 1.0-rc9; where an FSD and CC disagree, CC wins.

| FSD | What it covers |
|---|---|
| [`FSD/CEWP.md`](FSD/CEWP.md) | Platform identity — three load-bearing claims, "we don't need big tech" premise, agents-as-participants, superalignment via distributed epistemic governance |
| [`FSD/FEDERATION_SCALING_MODEL.md`](FSD/FEDERATION_SCALING_MODEL.md) | Quantitative model. Single-pool storage. Trust × capacity intake + popularity × freshness eviction. L0 / L1 tier model. 13 scenarios up to full-internet-with-video; identity-aware-storage thesis with prior-art comparison. |
| [`FSD/MEDIA_SHARING.md`](FSD/MEDIA_SHARING.md) | **Multimedia tier.** Five new external_content sub_kinds (image / audio / video / film / model_3d); content classification + multi-scheme rating (MPAA / BBFC / PEGI / etc.); age assurance; `takedown_notice` and `key_grant`; content encryption; international standards mapping (DMCA / DSA / OSA / TVEC / NCMEC / AVMSD / KOSA / EU AI Act). Under CC, a key grant is one `scores` row per recipient on `key_grant:{axis}:v1` (CC 2.4), and content keys are wrapped hybrid X25519 + ML-KEM-768, never classical-only (CC 5.2, CC 5.3.3.5 E1). |
| [`FSD/SCOPE_PRIVACY.md`](FSD/SCOPE_PRIVACY.md) | Scope-native privacy: how cohort-scoped confidentiality and anonymity to outsiders are provided by default below federation scope (CC 1.13.3.4, CC 5.2, CC 5.4.6). |
| [`FSD/ANONYMOUS_TIER.md`](FSD/ANONYMOUS_TIER.md) | **Planned** opt-in tier for unobservability against a global passive adversary at federation scope (Sphinx onion routing). CC 1.13.3.1 names it as the mechanism for deployments that need that resistance; it is not yet specified normatively in CC and not yet shipped, and it is not a default or a requirement today. **Anonymity toward outsiders at the smallest cohort scope is already the default** (CC 1.13.3.4); where the opt-in boundary sits is provisional and may move as the tier's overhead is measured (CC 1.13.3.4). Multi-hop anonymity is not provided today; this planned tier is the opt-in path (CC 5.4.6). The [toy](toy/index.html) explores what a default would cost; that is a design study, not CC policy. |
| [`FSD/SIMULATION_ENGINE.md`](FSD/SIMULATION_ENGINE.md) | Rust simulation engine spec — modular workspace; scalable from 1 K-agent browser playback to 5 B-agent GPU sim at 1:1. Real-world topology data (PeeringDB, CAIDA, TeleGeography submarine cables, GeoNames). Snapshots feed the website team's existing Three.js viz. |

### Working toy

| Path | Purpose |
|---|---|
| [`examples/scale_model.rs`](examples/scale_model.rs) | The runnable scaling-model toy. `cargo run --example scale_model` (from CIRISNodeCore workspace; mirrored here for reference). **14 workload scenarios** (bootstrap → dunbar → media-heavy → twitter → news → full-internet → tiktok → youtube → netflix → adulthub → full-internet-with-video) **+ 3 regional-realism scenarios** (south-asia-dense, sub-saharan-bootstrap, north-america-realistic) **+ a threat-model pass + per-scenario environmental footprint**. All feasible per-server at v1 gates (1 TB / 1 Gbps / 1 core). |
| [`toy/index.html`](toy/index.html) | The **interactive** browser toy — pick a scenario, tune the gates (disk / bandwidth / core, cohort mix, trust depth), watch per-server feasibility update live. Mirrors the `scale_model.rs` math; no build step, just open it. |

### Explanation docs (for general audience + website team)

| Path | Purpose |
|---|---|
| [`docs/overview.md`](docs/overview.md) | Plain-language platform explanation |
| [`docs/seven-repos.md`](docs/seven-repos.md) | The fabric-node architecture: who does what across the cores (CIRISServer cohabitation + substrate trio + agent) |
| [`docs/the-bet.md`](docs/the-bet.md) | The premise + the empirical bet from the research synthesis |
| [`docs/why-this-matters.md`](docs/why-this-matters.md) | The internet's structural problems + the CEWP mechanisms aimed at them, with the limits CC states |
| [`docs/for-developers.md`](docs/for-developers.md) | Pointers to the Constitution and the per-repo entry points |

### Consolidated analysis set (vendored snapshot)

[`analysis/`](analysis/) gathers the **load-bearing documents** of the whole
CIRIS stack — pulled from their 15 home repositories into one tiered set
(philosophy → constitution → governance → implementation) so the system can be
reviewed as a single philosophical-technical whole. See
[`analysis/README.md`](analysis/README.md) for the tier map and
[`analysis/SOURCES.md`](analysis/SOURCES.md) for the provenance manifest
(every source repo + pinned commit).

| Tier | Covers | Key sources (as pulled) |
|---|---|---|
| [0 — Philosophy](analysis/tier0-philosophy/) | adaptive coherence, Coherence Ratchet, Proof-of-Benefit, M-1 | CIRISAgent, coherence-ratchet (+ Lean lake), RATCHET |
| [1 — Constitution](analysis/tier1-constitution/) | sovereignty, amendment, halting, anti-capture | Constitution 0.4, CIRISAccord, CEG §9/§11 (superseded: see CC at [`v1.0-rc9`](https://github.com/CIRISAI/CIRISConstitution/tree/v1.0-rc9)) |
| [2 — Federation governance](analysis/tier2-federation-governance/) | consensus, expertise, moderation, deferral | CIRISNodeCore |
| [3 — Observation/epistemology](analysis/tier3-observation-epistemology/) | what counts as evidence; coherence measurement | CIRISLensCore, CCA paper |
| [4 — Registry/trust](analysis/tier4-registry-trust/) | credentials, licensure, revocation; the CEG spec as pulled (now CC Parts 2–5) | CIRISRegistry, CIRISPersist |
| [5 — Security](analysis/tier5-security/) | per-repo threat models + audits | every substrate repo (canonical) |
| [6 — Substrate](analysis/tier6-substrate/) | partition / steward-loss / node-loss survival | Verify, Persist, Edge |
| [7 — Integration](analysis/tier7-integration/) | do the pieces compose | CIRISServer, CIRISAgent |

> ⚠️ **Snapshot pulled 2026-06-19; it predates CC 1.0-rc9.** Every file
> under `analysis/` is a verbatim vendored copy at the commit pinned in
> `SOURCES.md`, and the copies are not updated. The constitutional
> documents in it (Constitution 0.4, CEG as a separate spec) are
> superseded: the CIRIS Constitution at tag
> [`v1.0-rc9`](https://github.com/CIRISAI/CIRISConstitution/tree/v1.0-rc9)
> is canonical, and each home repo stays canonical for its own
> documents. Re-pull before relying on any document for a decision.
> Threat models were taken from each repo's canonical
> `docs/THREAT_MODEL.md` (not the stale vendored copies in RATCHET).

## What CEWP is built to remove

Each row names the mechanism and, where CC bounds it, the bound.
[`docs/why-this-matters.md`](docs/why-this-matters.md) has the long form.

| Problem | Mechanism |
|---|---|
| Centralization (a handful of companies own the substrate) | No steward, no root, no load-bearing instance (CC 8.1.1); trust roots are chosen and left by the reader (CC 3.2) |
| Surveillance capitalism | Structural invisibility: no holding claim leaves a bounded cohort (CC 5.2). The scaling model puts ~65% of typical activity at self/family scope, which never federates (CC 5.2, *Locality dividend*). Bound: this is content-holding confidentiality, not metadata or traffic-analysis privacy (CC 1.13.3.1) |
| Trust crisis (deepfakes, deniable provenance) | Every row signed hybrid post-quantum (CC 5.3.3.5 E1); anyone named in a claim can contest it on the record (promise 2) |
| Misinformation amplification | Each reader composes claims under its own trust policy (CC 4.4.3.8); trust is walked a bounded number of hops, 5 by default (CC 4.1.1) |
| Platform lock-in | Identity is your key, not an account on a server; leaving a root is one row (CC 3.2, promise 4); content is addressed by hash and verified in full before use (CC 5.3.2.5) |
| DNS / nameserver capture | No forced root (CC 3.2); members resolve each other without DNS (CC 4.4.3.2.4.1). Bound: that resolution exposes who is reachable where (CC 1.13.3.1) |
| Single-point data loss | Per-item replication: an item its owner marks durable is held by a chosen number of cohort nodes, repaired when that number falls, and recovers from any one holder (CC 6.1, CC 6.1.5) |
| The unforgettable internet (no real delete) | One descent operator toward the noise floor: a revoked item must fall below individual recoverability at every retained tier (CC 6.1.2, CC 6.1.5 N5). Bound: below-floor is claimed against the pinned default adversary, not against one holding side information; aggregation is not anonymization (CC 6.1.2) |
| Identity fragmentation | One key, used across the substrate; standing comes from attestations others make about it (CC 1.13.3.3) |
| Content moderation conflicts | Every community has a named moderator, with merit auto-promotion if the moderator lapses (CC 4.5.4); no hand bans a community (promise 6) |
| Network-effects monopolies | Conformance is defined at the wire (CC 2.2), so a new implementation that meets the profiles interoperates from day one |
| No cryptographic accountability | Every row signed; transparency log with STH cosigning and consistency proofs (CC 5.3.1) |
| Datacenter dependency (data) | The scaling model fits full-internet scale on home-server-class hardware, ~1 per 10 humans (model output) |
| Datacenter dependency (AI) | Agents run on the devices people own (CC Executive Summary); their consequential acts are checkable by parties they do not control (CC Executive Summary, *The bet*) |

## The numbers (from the scaling model)

At "full internet replacement" — 5 billion users, 50 MB/user/day.
These are outputs of this repo's scaling model, not measurements:

|  | Today's Internet | CEWP Substrate (modeled) |
|---|---|---|
| Centralization | ~5 hyperscalers run the substrate | 500 M L1 servers + 2.75 B L0 proxies, ~1 per 10 humans |
| Datacenters required | Yes (~10 K hyperscale + edge) | **None** |
| AI training/inference | Centralized in 5 labs | Edge-deployable; alignment distributed |
| Per-user disk owned | ~0 (your data lives on their servers) | Yours, on your hardware |
| Per-user trust signal | Platform-controlled "verified" checkmark | Signed rows composed under the reader's own trust policy (CC 4.4.3.8) |
| Switching cost | Network-effects lock-in (Facebook holds your graph) | Leaving a root is one row (CC 3.2) |
| Storage duration | Platform-decided; deletion is a promise | Full-fidelity retention on commodity storage for as long as the owner's per-item target is held (CC 6.1.5), then descent into the memory pyramid rather than deletion ([scaling model §2.6](FSD/FEDERATION_SCALING_MODEL.md), whose retention figures await re-run under replication). The pyramid is Θ(T) if every level is kept, sublinear only under bounded retention per level (CC 6.1.2) |

<!-- rc9-sync: superseded by replication; re-run needed — the storage-duration row's former "~1–2 yr at full fidelity" figure came from the scaling model's fountain `1/N` store -->
<!-- rc9-sync: unverified, see CC ? — the "Today's Internet" column (hyperscaler count, ~10 K datacenters, "5 labs") is external-world data with no CC source; kept as the scaling model's comparison baseline -->

The simulation engine (FSD/SIMULATION_ENGINE.md) is what produces
these numbers in animation-ready form — modeled at 1:1 over the
real internet topology (real metros, real submarine cables, real
peering points).

## The fabric-node stack

CEWP is the platform identity for the CIRIS fabric node. What was
the "seven-repo Agent 3.0 stack" is now deployed as cohabiting cores;
the platform is what they become together.

- **The fabric node** ([CIRISServer](https://github.com/CIRISAI/CIRISServer),
  v0.5.232): the headless runtime (+ PyO3 wheel) that composes
  registry authority, lens observation and node consensus over one
  substrate (CC 3.4.7.1), with one shared persist Engine + one edge
  identity. See its repository for the core-by-core roadmap.
- **Substrate trio** (separate repos; bytes + crypto + transport):
  [CIRISVerify](https://github.com/CIRISAI/CIRISVerify) +
  [CIRISPersist](https://github.com/CIRISAI/CIRISPersist) (v59.1.0) +
  [CIRISEdge](https://github.com/CIRISAI/CIRISEdge) (v43.1.1)
- **Fabric-core repos**:
  [CIRISRegistry](https://github.com/CIRISAI/CIRISRegistry) (identity,
  build, licence and partner records, CC 3.1.1) +
  [CIRISNodeCore](https://github.com/CIRISAI/CIRISNodeCore)
  (lens-core is now a CIRISServer workspace crate)
- **Agent** (`fabric node + brain`):
  [CIRISAgent](https://github.com/CIRISAI/CIRISAgent) — the H3ERE
  pipeline + the UI users interact with; consumes the CIRISServer wheel.
- **The normative text**:
  [CIRISConstitution](https://github.com/CIRISAI/CIRISConstitution/tree/v1.0-rc9)
  — the Accord's ethics and the CEG grammar in one document.

See `docs/for-developers.md` for the pointer map.

## Canonical references

- [CIRIS Constitution 1.0-rc9](https://github.com/CIRISAI/CIRISConstitution/tree/v1.0-rc9) — the top-of-stack canonical document. Part 1 is the foundation (M-1, the principles, the adversary model); Parts 2 to 5 are CEG, the wire grammar implementers conform to; Parts 6 to 8 are the coherence mathematics, lifecycle and stewardship, and appendices. Start with the [Executive Summary](https://github.com/CIRISAI/CIRISConstitution/blob/v1.0-rc9/constitution/EXECUTIVE_SUMMARY.md) (the eleven promises in plain words)
- [Platform identity FSD](https://github.com/CIRISAI/CIRISNodeCore/blob/main/FSD/CEWP.md) — the load-bearing claims (in CIRISNodeCore)
- [Scaling model FSD](https://github.com/CIRISAI/CIRISNodeCore/blob/main/FSD/FEDERATION_SCALING_MODEL.md) — the quantitative bet
- [Anonymous tier FSD](https://github.com/CIRISAI/CIRISNodeCore/blob/main/FSD/ANONYMOUS_TIER.md) — the planned opt-in tier for unobservability against a global passive adversary (named in CC 1.13.3.1; not yet specified in CC or shipped); cohort-scoped anonymity toward outsiders is the default (CC 1.13.3.4)
- [Lightnet / darknet](https://github.com/CIRISAI/CIRISEdge/blob/main/docs/LIGHTNET_DARKNET.md) — the two-plane split as Edge ships it, each claim labelled normative, shipped, measured or posited
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
- Wire-format and constitutional proposals: [CIRISConstitution](https://github.com/CIRISAI/CIRISConstitution) (amendment process, CC 4.5.1; a new row type is filed against CC 2.4 before it ships)

The soup is on. Come participate.
