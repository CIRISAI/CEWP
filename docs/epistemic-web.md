# The epistemic web, as the Constitution now states it

*Pre-release: these shapes are on CIRISConstitution branch `rc17` and become the edition's text at its tag. Where this page and the Constitution disagree, the Constitution wins.*

## One sentence

A client of the epistemic web renders two served shapes, the **view** and the **queue**, holds the keys of its deployment and no policy, and the queue is the admission door run forwards.

## The view

For one object a person can see: its **rows** (the signed statements it rests on) and the **atoms** the node folded from them through the person's own trust set (CC 4.4.3.8), each atom carrying its provenance: the rows it read and the rule it ran. The record and the state are one thing seen twice. Schema: `manifests/view.cddl`.

## The queue

The items awaiting the person's hand. Each item: a context (circle, role, atoms in force), facts with provenance, a governance policy row, options with a benefit and a downside that are never empty, the rows each option would sign, and an impact **derived from the family model** for those rows (what moves, where it goes, widens or narrows, can it be undone). Four kinds and no more: choice, act, entry, notice. Consent is not a kind: a consent is a choice whose writes are consent rows. Schema: `manifests/choice_item.cddl`.

## How the view makes the queue

The queue is the door run forwards. For an object, the node holds its family, its lifecycle state, and the person's standing toward it (author, subject, member, moderator, founder, steward, holder, witness, folded from held rows). A family's lifecycle is a list of transitions (verb, from, to, roles, policy, widens), derived from the family model into `manifests/transitions.json`. The queue is exactly the transitions enabled for this person in this state, one item each. The admission door is the same predicate, so nothing offered is refused and nothing refused was offered; a stranger sees nothing; an answered item leaves the next view; a widening item is confirmed before it is signed. Proved in `formal/event_lifecycle`.

## The life of an event

view → queue → item → row → view. `tools/mesh_story.py` runs one scripted life of a mesh over the derived tables and checks every step; the trace it writes is what the page beside this one replays.

## Localization

Every string the client shows is a text reference the node serves for the requested locale: `{id, text, params, lang?}`. The client holds no copy. On the way there, a node serves ids with source-language text and a consumer may overlay its own bundle by id; the ids are one namespace, and the difference between served ids and held strings is the measured distance to the full model, closed one card at a time.

## The signed object, next edition

CBOR with COSE, aligned with SCITT (RFC 9943): one COSE_Sign1 per signer, issuer and subject claims, the composite ML-DSA-65 with Ed25519 construction, composite keys with thumbprint ids, deterministic encoding, content-addressed statement ids, receipts as COSE Receipts (RFC 9942) over RFC 9162 trees. Every row signed under the earlier object verifies as itself. CC 2.6.1.8; `manifests/signed_object_v2.cddl`.

## Where to read

- CIRISConstitution `rc17`: Part 4 §4.5.13 (the choice as data, the epistemic web client, how the view makes the queue), Part 2 §2.6.1.8, `manifests/{view,choice_item,transitions}.json|cddl`, `formal/event_lifecycle`, `formal/anonymous`, `formal/moderator_election`, `formal/transport_tiers`.
- CEWPOS: the twelve primitives render these two shapes and nothing else (CEWPOS#2).
