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
