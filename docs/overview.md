# What is CEWP?

CEWP is the **CIRIS Epistemic Web Platform**, pronounced **"soup."**

*Reconciled to the CIRIS Constitution 1.0-rc9 (tag
[`v1.0-rc9`](https://github.com/CIRISAI/CIRISConstitution/tree/v1.0-rc9)).
References are written "CC <section>"; where this page and the
Constitution disagree, the Constitution wins.*

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
the cryptographic substrate. Nobody owns the soup. In the
Constitution's words, CEWP is "the emergent peer-to-peer web of
CEG-speaking nodes", with no steward, no root and no load-bearing
instance (CC 8.1.1).

## No center, no DNS, no load-bearing server

There's no center, no nameserver, and no server the whole thing leans
on. You address a peer by its key, reach it over the mesh, and trust
it through a root you chose. You can leave that root in one step, and
anyone may found another (CC 3.2; promise 4 of the
[Executive Summary](https://github.com/CIRISAI/CIRISConstitution/blob/v1.0-rc9/constitution/EXECUTIVE_SUMMARY.md)).

The soup has two layers (CC 5.4.6 *Position*; CIRISEdge
[`docs/LIGHTNET_DARKNET.md`](https://github.com/CIRISAI/CIRISEdge/blob/main/docs/LIGHTNET_DARKNET.md)):

- The **lightnet** is the public identity plane: who has which key,
  which devices are announced, which trust roots exist. It is
  attributable on purpose and makes no anonymity claim. Every message
  is tied to a key before anything handles it.
- The **darknet** is the group plane. Each member computes the group's
  address from what the members already hold, so nothing is announced
  and outsiders cannot see that the group exists. Members relay for
  members.

What this does not hide: an observer of network traffic can still
infer who talks to whom, roughly how big a group is and when it
changes. CC claims content confidentiality and cohort-scoped
visibility, not metadata or traffic-analysis privacy (CC 1.13.3).

An item its owner marks durable is held, chunk by chunk, by a chosen
number of nodes in its own cohort and repaired when that number falls,
so the witnessed corpus recovers from any one holder (CC 6.1,
CC 6.1.5). Big items are stored in layers, and losing the detailed
layers leaves a coarse version of the same item (CC 6.1).

And the soup has a memory that's **forever but still forgets**.
Revocation, eviction, expiry, and aging aren't four mechanisms;
they're *one* descent toward a **noise floor** (CC 6.1.2). A revoked
item must drop below the line where it can be individually recovered;
the collective gist can age on below it. A million years may be a
blur, but it is remembered. Two limits apply: "below the floor" is
claimed against a defined default adversary, not against one who
already knows the other members of the aggregate, and aggregation is
not anonymization (CC 6.1.2). A purge leaves a tombstone (promise 1).

## Agents live in the soup — they don't hold it

Every node in CEWP is a **fabric node**: it attests, stores, observes,
reaches consensus and transports, but does not reason or act. Add a
reasoning brain and the same node becomes an agent:
**`agent = fabric node + brain`** (CC 8.1.1). Authority stays with
human-held keys under quorum; a node gains neither a vote nor a
verdict by running the authority software (CC 3.4.7.1). The broth is
not allowed to govern itself.

## The ethics the soup is for

The substrate carries a grammar (CEG, Parts 2 to 5 of the
Constitution); the
[**CIRIS Constitution**](https://github.com/CIRISAI/CIRISConstitution/tree/v1.0-rc9)
carries the ethics it speaks for, in the same document. At the apex is
one meta-goal, **M-1: "Promote sustainable adaptive coherence — the
living conditions under which diverse sentient beings may pursue their
own flourishing in justice and wonder"** (CC 1.1). Six principles sit
beneath it, and no single principle grants licence to violate another
(CC 1.1).

## What the substrate does

Every claim in CEWP — "this content is accurate," "this peer is
trusted," "this article cites these sources" — is a signed row, and
there are only five kinds of row: a claim (`scores`), a delegation, a
replacement, a withdrawal and a recantation (CC 2.4). Consent, keys,
group rosters, moderation and the halt itself are all rows of these
five kinds (CC Executive Summary).

There's no central trust authority and no "verified" checkmark. Each
reader decides whom to believe under a policy it chooses, and a new key
has no weight until someone the reader trusts confers it (CC 1.13.3.3,
CC 4.4.3.8). If you are named in a claim, you can contest it on the
record, and the contest travels with the claim (promise 2).

## Why this is also AI governance

The same promises hold for human and AI participants alike (CC
Executive Summary). Agents sign their acts like everyone else, but
they are not on the network alone: an agent key must have a named
human steward bound by a signed row (CC 3.2), and an agent has no vote
(promise 6).

The Constitution's bet: there are two roads to safe powerful AI. Train
values you trust, or assume the model may be wrong and make its
consequential acts checkable by parties it does not control. CIRIS
takes the second road (CC Executive Summary, *The bet*).

Two pieces make that concrete:

- **The halt.** Named accord holders can pause every agent under their
  root. Any one holder fires it with a single signed row; a lone pause
  lapses after 24 hours unless a majority confirms it; a majority
  lifts it. A pause stops agents acting and touches nothing else, and
  it reaches only nodes that accepted the root. A root can only tell
  you to share less, never more (CC 4.2.1, CC 4.2.6; promise 4). This
  is the one promise measured end to end on real nodes.
- **Traces.** An agent's durable reasoning trace is anchored on what
  it actually did (CC 1.13.6). Traces are shared with the observation
  layer only by consent: capture, shipping and scoring are each a
  separate consent row (CC 3.3.1). Agents do not train on the people
  they serve (promise 1).

The research bet on top of that: we can measure reasoning shape from
consented agent traces and use those measurements as an alignment
signal, without access to model weights and without a central
alignment lab. See [`the-bet.md`](the-bet.md) for the empirical
articulation.

## Why this matters for the centralized internet

The same properties also address some of the centralized internet's
structural problems:

- **Surveillance** is structurally limited for private content: no
  holding claim for self or family content leaves that cohort, and the
  scaling model puts about 65% of typical activity there (CC 5.2).
  Private content is never scanned (CC 1.13.3.4).
- **Trust crisis** eases because every claim has a signature and a
  signer, signed hybrid post-quantum (CC 5.3.3.5 E1).
- **Platform lock-in** weakens because your identity is your key, not
  an account, and leaving a root is one row (CC 3.2).
- **Datacenter dependency** is the target at both the data layer and
  the AI layer: the software runs on the phones and computers people
  already own (CC Executive Summary).

See [`why-this-matters.md`](why-this-matters.md) for the full
problem-to-mechanism mapping, with the limits.

## How big is "big enough"

The quantitative answer from this repo's scaling model: CEWP at
full-internet scale (5 billion users, 50 MB of activity per user per
day) fits on **500 million home-server-class machines + 2.75 billion
laptop-class proxies**, roughly one server per ten humans, the density
home-internet / IoT deployments already deliver today. That is a model
output, not a measurement.

In that model, no datacenters are required.

See the [interactive toy](../toy/index.html) to play with the numbers.

## What's NOT in scope

CEWP is not:

- A trained model (it runs models alongside humans; it doesn't ship weights)
- A cryptocurrency (no token of its own; value transfer is not a grammar primitive and rides external rails, CC 3.3.10)
- A content moderation service (it gives the federation moderation primitives; every community has a named moderator, CC 4.5.4)
- A replacement for law (it creates no legal claim where the law recognises none; it supplies the signed record a remedy can reach, promise 8)
- A single-deployment system (every federation is independent; deployments interoperate via wire format, CC 2.2)
- Finished. Anonymity toward outsiders at the smallest cohort scope is the default (CC 1.13.3.4). Unobservability against a global passive adversary at federation scope is the job of a planned opt-in tier (Sphinx onion routing, [`FSD/ANONYMOUS_TIER.md`](../FSD/ANONYMOUS_TIER.md)) that CC 1.13.3.1 names for deployments needing it; it is not yet specified in CC and not yet shipped, and where that boundary sits is provisional (CC 1.13.3.4). Multi-hop anonymity is not provided today; the planned anonymous tier is the opt-in path (CC 5.4.6). Independent witnesses for the default trust root are not yet in place (CC Executive Summary).
