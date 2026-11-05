# The kernel

The state processor of the CIRIS Constitution, 4.5.13, as one file with no dependencies: [`kernel.js`](kernel.js).

A node holds rows, each with the instant it first held it, and nothing else. Every verdict is a fold over that set, never stored. One step appends and never rewrites. The queue a person sees, the preview of an act and the door that admits it are one function:

| call | what it is |
|---|---|
| `fold(held, tables, now)` | every verdict: each object's state (five states), the rooms' rosters and moderators, who is registered, the trust root's holders, the halt; the policy folds (two-path, mirror, cosigned, ranked) over the voices on an object |
| `standing(fold, person, object)` | the role tokens a person holds toward an object, folded from held rows |
| `enabledFor(tables, family, state, roles)` | the transitions of `manifests/transitions.json` enabled from this state for these roles |
| `step(held, tables, row, now)` | the door before it commits: a plan or a refusal by name |
| `commit(held, plan)` | append |
| `door(held, tables, row, now)` | step then commit |
| `queue(held, tables, person, now)` | the door run forwards: one item per enabled transition, plus what the person may start |
| `preview(held, tables, rows, now)` | the same step over a copy; the impact is the difference of the folds |
| `Mesh` | nodes as cells, rows as signals: one hop per tick, offered only to a node whose owner is in the row's audience as the sender folds it; a stranger carries only the consent-free set |

The tables are the constitution's, vendored under [`tables/`](tables/) at the commit named in `tables/VERSION`: `transitions.json` (191 families, 474 transitions), `governance_policy.json` (the five numbers per policy row), and `mesh_story_trace.json` (the constitution's own executable story).

**What it proves by running.** `node --test docs/kernel/kernel.test.js` replays the constitution's fourteen-step story and checks that this kernel offers the same queue and refuses the same acts at every step, then runs a mesh: keys spread through a seat, a stranger is never offered a room's row, no single vote admits, the invitee's yes seats her, two nodes holding the same rows count the same voices, and the preview of a post equals its execution.

**What it leaves out, on purpose.** Signatures (a held row is already verified), transport below "one hop per tick", other nodes' holdings, projections, text, a clock (`now` is an input), and any reader's weighting of what it reads. That list is the constitution's own: it is what keeps the kernel small enough that every question about it is decidable.

[`../mesh.html`](../mesh.html) runs it in the browser as a mesh of people and their nodes.

## At scale: the door in WebAssembly

[`../globe.html`](../globe.html) puts a few million nodes on a globe, placed by where people live, and signs random acts every tick: 65% family-scope, 30% in a community room, 4% consent-free rows, 1% in a public room. The door runs at the signer's node and at every node of the row's audience. The loop is [`kernel-wasm/src/lib.rs`](../../kernel-wasm/src/lib.rs), compiled to `kernel_scale.wasm` (23 KB, no dependencies, `kernel-wasm/build.sh`), with the same loop in JavaScript on the page so the two engines can be timed on one world. What it shows: the share of rows that never leave the signer's metro (the locality dividend of CC 5.2; about nine in ten with rooms drawn 90% within a metro), the bytes moved against a broadcast network and a datacenter in the middle, and door checks per second (about 15 M/s in WASM against 6 M/s in JavaScript on one core at 1 M nodes). It runs the admission predicate, not the policy folds.
