# What is CEWP?

CEWP is the **CIRIS Epistemic Web Platform**, pronounced **"soup."**

It's a platform — like the internet is a platform — built to do two
things at once:

1. **Replace the centralized internet** with a federation of equals.
2. **Run AI agents accountably** alongside humans.

The bet is that those are the same problem.

## The soup analogy

A soup is what you get when many ingredients participate in one
shared medium. Each ingredient keeps its character. Nobody is
sovereign over the broth. The soup is what the ingredients become
together.

That's CEWP. The ingredients are humans, AI agents, organizations,
registry stewards, detectors, the federation itself. The broth is
the cryptographic substrate. Nobody owns the soup. The soup is
what we become together.

## No center, no DNS, no load-bearing server

CEWP is **holonomic** (after Bohm's implicate order): there's no
center, no nameserver, and no server the whole thing leans on. You
address a peer by its key, reach it over the mesh, and trust it
through a chain you can re-root at will. Nothing in the broth is the
broth's owner.

Content is **holographic** — erasure-coded so that any sufficient
subset of the fragments reconstructs it, and the federation
re-establishes itself from any single survivor holding a signed
witness chain. The soup survives down to one bowl.

And the soup has a memory that's **forever but still forgets**.
Revocation, eviction, expiry, and aging aren't four mechanisms —
they're *one* descent toward a **noise floor**: revoked things drop
below the line where any individual item can be recovered (privacy),
while the collective gist ages on below it forever (durability). All
of history is kept as a memory pyramid that costs `O(log T)` to hold.
A million years may be a blur, but it is remembered.

## Agents live in the soup — they don't hold it

Every node in CEWP is a **fabric node** — a piece of infrastructure
that stores, witnesses, degrades, and transports, *mechanically,
never reasoning*. Add a reasoning brain and the same node becomes an
agent: **`agent = fabric node + brain`**. Authority roots in
accountable humans, never in bare infrastructure — the broth is not
allowed to govern itself.

## The ethics the soup is for

The substrate carries a grammar (CEG); the
[**CIRIS Constitution**](https://github.com/CIRISAI/CIRISRegistry/tree/main/FSD/CIRIS_Constitution)
carries the ethics it speaks for. At the apex is one meta-goal,
**M-1 — "promote sustainable adaptive coherence, the living
conditions under which diverse sentient beings may pursue their own
flourishing in justice and wonder."** Everything else stays flat
beneath it: *peaked in purpose, flat in power* — one telos governs;
no single concept, and no single party, holds the keys to truth. The
soup is not a neutral pot; it's a substrate that could one day be
*owed* M-1, not merely bound by it.

## What the substrate does

Every claim in CEWP — "this content is accurate," "this peer is
trusted," "this article cites these sources" — is a cryptographically
signed wire-format artifact. The federation collectively scores
those artifacts via weighted aggregation. That scoring is what
*governs* the system in real time.

There's no central trust authority. There's no admin endpoint. There's
no "verified" checkmark. Trust is computed locally by each operator
from the attestation graph the federation maintains together.

## Why this is also AI governance

When humans interact, the wire artifacts carry their attestations.
When AI agents interact, the wire artifacts carry *their*
attestations. The agents have the same federation identity shape as
humans, sign the same Contributions, accumulate the same trust, can
be moderated by the same pipelines.

This means alignment becomes an **epistemic governance** problem, not
a training-time problem to be solved inside a model. The lab's RLHF
becomes a historical training-time input; the federation's runtime
trust graph becomes the live alignment surface.

The bet: we can measure reasoning shape from agent traces, accumulate
those measurements in a distributed trace commons (consented to by
real users), and use that measurement substrate as the alignment
authority — without requiring access to model weights, without
requiring a central alignment lab, and without requiring datacenters.

See [`the-bet.md`](the-bet.md) for the empirical articulation.

## Why this matters for the centralized internet

The same property that makes AI governance work — cryptographic
substrate + identity-aware storage + federation trust graph — also
addresses the centralized internet's compounding structural problems:

- **Surveillance** is structurally limited because 65% of typical
  activity (self/family scope) never leaves your device.
- **Trust crisis** dissolves because every claim has cryptographic
  provenance.
- **Platform lock-in** disappears because your identity + content
  are wire-portable.
- **Datacenter dependency** at both the data layer and the AI layer
  is eliminated — the substrate runs on commodity hardware (down to
  home-server class on a Jetson Orin).

See [`why-this-matters.md`](why-this-matters.md) for the full
problem-to-mechanism mapping.

## How big is "big enough"

The quantitative answer: CEWP at full-internet scale (5 billion
users, 50 MB of activity per user per day) fits on **500 million
home-server-class machines + 2.75 billion laptop-class proxies** —
roughly one server per ten humans, the density home-internet / IoT
deployments already deliver today.

No datacenters required.

See the [interactive toy](../toy/index.html) to play with the numbers.

## What's NOT in scope

CEWP is not:

- A trained model (it runs models alongside humans; it doesn't ship weights)
- A cryptocurrency (no token, no on-chain consensus)
- A content moderation service (it gives the federation moderation primitives)
- A replacement for top-down regulation (it provides the enforcement substrate regulation currently lacks)
- A single-deployment system (every federation is independent; deployments interoperate via wire format)
- Finished (cohort-scoped anonymity is the default per CC 1.13.3.4; the opt-in GPA-unobservability tier — Sphinx onion, for totalitarian-threat deniability — is being measured for default promotion; v-next is undefined)
