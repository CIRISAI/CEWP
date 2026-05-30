# CEWP

**CIRIS Epistemic Web Platform.** Pronounced **"soup."**

A decentralized replacement for the internet — and simultaneously
an AI governance / superalignment platform — built on the bet that
**big tech is not necessary**.

## What CEWP is

CEWP is the platform identity for the seven-repo CIRIS Agent 3.0
stack. It's an *epistemic web*: a cryptographically-accountable
federation where humans, AI agents, and organizations are first-
class participants with the same identity shape. Every load-bearing
claim ("this content is accurate," "this peer is trusted," "this
action should be deferred to a human") is a signed wire-format
artifact; the federation's collective weighted-aggregate scoring of
those artifacts is what *governs* the system in real time.

Agents live **inside** CEWP; they don't have CEWP as a tool. The
substrate eliminates the centralized internet's structural failure
modes (surveillance, platform lock-in, trust collapse, content-
quality regression, identity fragmentation) AND removes the
dependency on large datacenters at both the data-sharing and AI-
inference layers.

```
                          ┌─────────────────────────────┐
                          │      CIRISAgent             │  ← agent runtime + unified client
                          │  (H3ERE pipeline + UI)      │     users interact here; agents reason here
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
```

**7 repos total**: 3 substrate sisters (bottom row) + 3 fabric sisters (middle row) + 1 agent runtime + unified client (CIRISAgent).

All seven repos cohabit in one process at CIRIS 3.0 deployments.
The substrate runs on commodity hardware (down to CIRISHome on
Jetson Orin — already deployment-ready) and scales to the full
internet (5 billion users) on ~1 server per 10 humans — the density
home-internet / IoT deployments already deliver. **No datacenters
required.**

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

## What's in this repo

This repo is the **platform-identity + simulation home** for CEWP.
The technical substrate lives in the seven sister repos (linked
below); this repo is what you read to understand the platform as a
whole and what you fork to run simulations of it.

| Path | Purpose |
|---|---|
| [`FSD/SIMULATION_ENGINE.md`](FSD/SIMULATION_ENGINE.md) | **The Rust simulation engine spec.** Modular workspace; scalable from 1 K-agent browser playback to 5 B-agent GPU sim at 1:1. Uses real-world topology data (PeeringDB, CAIDA, TeleGeography submarine cables, GeoNames). Side-by-side comparison of centralized internet vs CEWP on the same workload. Snapshots feed the website team's existing Three.js viz. |
| [`docs/overview.md`](docs/overview.md) | Plain-language platform explanation |
| [`docs/seven-repos.md`](docs/seven-repos.md) | Agent 3.0 architecture: who does what across the seven repos |
| [`docs/the-bet.md`](docs/the-bet.md) | The premise + the empirical bet from the research synthesis |
| [`docs/why-this-matters.md`](docs/why-this-matters.md) | The internet's 11 structural problems + CEWP's mechanisms that eliminate them |
| [`docs/for-developers.md`](docs/for-developers.md) | Pointers to the canonical FSDs in the substrate-sister repos |

## What CEWP eliminates

| Problem | Mechanism |
|---|---|
| Centralization (5 companies own the substrate) | Federation of equals; no central party |
| Surveillance capitalism | CEG locality dividend — 65% of typical activity never leaves your device |
| Trust crisis (deepfakes, deniable provenance) | Cryptographic provenance on every claim |
| Misinformation amplification | No engagement-optimization layer; consumer-controlled trust depth |
| Platform lock-in | Federation key portable across deployments; content SHA-addressed |
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

## The seven repos

CEWP is the platform identity for the seven repos of CIRIS Agent
3.0. Each has a specific role; the platform is what they become
together.

- **Substrate sisters** (bytes + crypto + transport):
  [CIRISVerify](https://github.com/CIRISAI/CIRISVerify) +
  [CIRISPersist](https://github.com/CIRISAI/CIRISPersist) +
  [CIRISEdge](https://github.com/CIRISAI/CIRISEdge)
- **Fabric sisters** (federation semantics + detection + spec):
  [CIRISNodeCore](https://github.com/CIRISAI/CIRISNodeCore) +
  [CIRISLensCore](https://github.com/CIRISAI/CIRISLensCore) +
  [CIRISRegistry](https://github.com/CIRISAI/CIRISRegistry)
- **Agent runtime + unified client**:
  [CIRISAgent](https://github.com/CIRISAI/CIRISAgent) (the H3ERE pipeline + the UI users interact with — single repo, both roles)

Each sister has a CEWP-orientation issue explaining their specific
role + the v1 posture they implement against. See `docs/for-developers.md`
for the pointer map.

## Canonical references

- [Platform identity FSD](https://github.com/CIRISAI/CIRISNodeCore/blob/main/FSD/CEWP.md) — the load-bearing claims (in CIRISNodeCore)
- [Scaling model FSD](https://github.com/CIRISAI/CIRISNodeCore/blob/main/FSD/FEDERATION_SCALING_MODEL.md) — the quantitative bet
- [Anonymous tier v2 FSD](https://github.com/CIRISAI/CIRISNodeCore/blob/main/FSD/ANONYMOUS_TIER.md) — totalitarian-threat deniability path
- [CEG 0.2 wire-format spec](https://github.com/CIRISAI/CIRISRegistry/tree/main/FSD/CEG) — the authoritative wire-format spec (in CIRISRegistry)
- [The Accord](https://ciris.ai/ciris_accord.pdf) — ethical framework the substrate enforces
- [Research synthesis](https://ciris.ai/research-status/) — the empirical bet

## License

[AGPL-3.0-or-later](LICENSE) — same as the rest of the CIRIS stack.
The substrate is mission-locked: you can use it, you can modify it,
you cannot use it to capture the substrate.

## Engaging

- High-level positioning / discussion: [CIRISAgent#839](https://github.com/CIRISAI/CIRISAgent/issues/839) (Agent 3.0 / CEWP umbrella tracker)
- Simulation-engine implementation: issues in this repo (once it lands)
- Substrate-sister implementation: appropriate sister repo
- Wire-format proposals: CIRISRegistry (CEG 0.2 PWD; §4.9.2 amendment process)

The soup is on. Come participate.
