# The premise + the bet

*Reconciled to the CIRIS Constitution 1.0-rc9 (tag
[`v1.0-rc9`](https://github.com/CIRISAI/CIRISConstitution/tree/v1.0-rc9)).
References are written "CC <section>"; where this page and the
Constitution disagree, the Constitution wins.*

## The premise

**Big tech is not necessary.**

Not "should be regulated." Not "is harmful." Not "can be improved."
**Not necessary.**

This is the load-bearing claim. Everything in CEWP — the fabric-node
architecture, the cryptographic substrate, the federation discipline,
the H3ERE pipeline, the consented trace sharing — exists to test this
claim.

## The bet

**Cryptographic substrate + standardized ethical tracing prove it.**

The Constitution states the AI half of this bet in its own terms:
there are two roads to safe powerful AI, train values you trust, or
assume the model may be wrong and make its consequential acts
checkable by parties it does not control. CIRIS takes the second road
(CC Executive Summary, *The bet*). Its one measurement so far is
narrow: on a mental-health test whose design was fixed in writing
before it was run, the accord running inside the agent's pipeline
produced hard safety failures on 5.8% of turns, against 37.3% when the
same text was given to the model as instructions. That is one test,
one agent version, one provider, and nothing is claimed beyond that
domain (CC Executive Summary).

Two structural pieces:

### Piece 1 — cryptographic substrate

Every row in CEWP is signed, and there are five kinds of row and no
others (CC 2.4). Every record is signed Ed25519 + ML-DSA-65, every
content key is wrapped X25519 + ML-KEM-768, and every byte over a link
rides a hybrid transit seal, with no classical lane (CC 5.3.3.5 E1).
Trust is computed by each reader from the attestation graph under a
policy it chooses (CC 4.4.3.8). Withdrawal is a signed `withdraws` row
(CC 2.4.1.1).

This means:

- Identity is rooted in cryptography, not corporate databases
- Trust is computed locally, not asserted by oracles
- Audit chains are mathematical objects, not text reports
- Provenance is structural, not editorial

The substrate works at consumer-hardware cost: see the measured
numbers in the README's
[benchmarks section](../README.md#measured-performance--ciris-fabric-benchmarks).
The [scaling model](https://github.com/CIRISAI/CIRISNodeCore/blob/main/FSD/FEDERATION_SCALING_MODEL.md)
estimates one server per ten humans at full-internet scale; see the
[interactive toy](../toy/index.html) for the playable version.

The substrate's shape is part of the case:

- **No datacenters**, in the model. Full-internet scale (5 B users)
  fits on ~1 server per 10 humans, the density home-internet / IoT
  deployments already deliver. This is a model output, not a
  measurement.
- **No DNS, no load-bearing server.** You address a peer by its key
  and choose your trust root, which you can leave in one step
  (CC 3.2). CEWP has no steward, no root and no load-bearing instance
  (CC 8.1.1).
- **Survival from one holder.** An item its owner marks durable is
  held, chunk by chunk, by a chosen number of cohort nodes and repaired
  when that number falls, so the witnessed corpus recovers from any one
  holder (CC 6.1, CC 6.1.5).
- **Delete that means something.** Revocation, eviction, expiry, and
  aging are *one* descent toward a noise floor; a revoked item must
  fall below individual recoverability at every retained tier, while
  the collective gist can persist as a memory pyramid (CC 6.1.2). The
  claim is made against a defined default adversary; aggregation is
  not anonymization (CC 6.1.2).

If big tech were necessary, these would not hold at commodity scale.
The scaling model says they do; the benchmarks measure the
cryptographic and transport costs that model rests on.

### Piece 2 — standardized ethical tracing

The H3ERE pipeline produces typed decision-making verdicts
(`dma:pdma:*`, `dma:csdma:*`, `dma:dsdma:*`, `dma:idma:*`; CC 3.1.5.1)
and a reasoning trace per agent act. These traces are:

- **Anchored on the act** — a durable trace is anchored on the record
  of what the agent did; a deliberation that never reaches an action
  is ephemeral by design and is purged (CC 1.13.6)
- **Consented** — a node's traces are captured, shipped and scored
  only under three separate consent rows (CC 3.3.1)
- **Standardized** — trace components are rows in the same grammar as
  everything else; a trace names its run, and a mock trace is refused
  (promise 2)
- **Distributed** — traces flow to whichever observation nodes their
  owner consented to, not to a central corpus owned by one lab

<!-- rc9-sync: unverified, see CC ? — removed "~14 KB of typed trace components per agent decision" and "scrubbed via the inline_text_pipeline at ~10 ns/byte"; neither figure is in CC rc9 (component measurements, see CIRISPersist / CIRISEdge) -->

## How the bet pays out

The published research synthesis at
[ciris.ai/research-status](https://ciris.ai/research-status/)
articulates the empirical claim:

> "Reasoning has a shape we can measure as everything else scales."

And specifies HOW the shape is measured:

- The **corridor metaphor** — systems operate in a bounded region
  between rigidity (`ρ → 1`, single-voice collapse) and chaos
  (`ρ → 0`, vacuous dispersal)
- **Effective diversity** — `k_eff = k / (1 + ρ(k−1)) → 1` as
  constraints correlate
- **Distinct field structures** — completion corridors, refusal
  boundaries, hesitation zones

These metrics are meant to run in **ciris-lens-core**, the observation
layer of the fabric node (a CIRISServer crate). Its outputs are
`detection:*` rows, which are validated, not adjudicated, and are
never sole evidence for an authority action (CC 3.4.7.1). The
detectors that test what agents said against what they did are, at
rc9, a rule with no code yet (CC Executive Summary, promise 2).

If the detectors run and the metrics distinguish aligned from
misaligned behavior at meaningful resolutions, the bet pays out:

- **AI alignment** becomes a federation property (measured + governed
  in real time), not a training-time property owned by a lab
- **Datacenter dependency** for alignment work disappears — the
  measurement substrate runs on whatever the federation runs on
- **Benchmark asymmetry** disappears — the lab no longer controls
  the eval publication cycle because consented traces give every
  operator the same evaluative capacity

## What this is NOT

- **Not a guarantee** that alignment is solved. The substrate is
  necessary; it's not sufficient. Bad actors will still try;
  misalignment will still occur. The federation's response is what
  makes the difference.
- **Not a claim** that capability + governance scale at the same
  rate forever. A capability discontinuity could outpace the
  substrate's governance signal. The Constitution's own larger claim
  is conditional: an AI far beyond human ability could be governed
  through a mesh like this only as one of several under independent
  control, never alone (CC Executive Summary). Part 8 names what it
  would take to lose the bet.
- **Not a claim** that the federation is automatically trustworthy.
  The federation is only as trustworthy as the cross-attestations
  it accumulates. The wire format carries exactly one asymmetry:
  humanity's right to halt the system, held by named accord holders
  and reaching only those who chose to trust that root (CC 4.2). Above
  the wire grammar, the
  [CIRIS Constitution](https://github.com/CIRISAI/CIRISConstitution/tree/v1.0-rc9)
  names the apex the whole substrate answers to: M-1, sustainable
  adaptive coherence (CC 1.1).

## Why this is worth the bet

If the bet wins:

- The internet is decentralized, and private content stays inside its
  cohort at the substrate level (CC 5.2)
- AI alignment becomes a property of the federation's runtime, not
  any single lab's training run
- Datacenter dependency is removed at both the data and AI layers
- Network-effects monopolies weaken because identity is a key you hold
  and a root is something you can leave
- The structural pressures of the centralized internet
  (surveillance capitalism, trust collapse, platform lock-in,
  content-quality regression) are addressed at the substrate level

If the bet loses:

- We still have an alignment-research-program-worth-of substrate
  built and shipping
- We still have a decentralized communication network worth running
- We still have consented, signed trace data that other research
  programs can use
- The substrate works whether the bet pays out or not; the bet is
  about whether it's *enough*

## How you can engage with the bet

- Read [ciris.ai/research-status](https://ciris.ai/research-status/)
  for the synthesis paper
- Read the Constitution's
  [Executive Summary](https://github.com/CIRISAI/CIRISConstitution/blob/v1.0-rc9/constitution/EXECUTIVE_SUMMARY.md)
  for the eleven promises and their status
- Play with the [interactive toy](../toy/index.html) to see the
  scaling numbers
- Read the [scaling model FSD](https://github.com/CIRISAI/CIRISNodeCore/blob/main/FSD/FEDERATION_SCALING_MODEL.md)
  for the quantitative derivation
- Read the [CEWP platform FSD](https://github.com/CIRISAI/CIRISNodeCore/blob/main/FSD/CEWP.md)
  for the architectural commitment
- Try the [free CIRIS agent app](https://ciris.ai/) on your phone
  and choose whether to share traces

The substrate is shipping. The bet is open. The soup is on.
