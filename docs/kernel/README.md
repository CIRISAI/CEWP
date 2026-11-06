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

[`../globe.html`](../globe.html) puts up to a few million nodes on a globe, placed on a land-bounded world population density grid (`2,961` populated land cells across `181` localities and `9` GSMA/UN regions, stitched to continent coastlines with zero ocean nodes), and signs random acts every tick: 65% family-scope, 30% in a community room, 4% consent-free rows, 1% in a public room. The door runs at the signer's node and at every node of the row's audience. The loop is [`kernel-wasm/src/lib.rs`](../../kernel-wasm/src/lib.rs), compiled to `kernel_scale.wasm` (34 KB, no dependencies, `kernel-wasm/build.sh`), with the same loop in JavaScript on the page so the two engines can be timed on one world.

What it models and measures (per **CC RC17 §4.5.13 / §5.4.3 / §5.4.6** and **CIRISEdge v44**):
- **The locality dividend (CC 5.2)**: the share of rows that never leave the signer's locality (~90% with community rooms drawn 90% local).
- **Three transport meshes + Seat/ALM relay tree**:
  - **Low-BW shaped (`base`)**: `16 KiB / 2 s` (`8 KB/s`, `691 MB/day/slot`) constant-rate padded/chaff links.
  - **High-BW shaped (`seat`)**: `16 KiB / 500 ms` (`33 KB/s`, `2.76 GB/day/slot`) canonical serve seats (`serve_max = 8`).
  - **Live / unshaped bulk (`live`)**: Reticulum IPv6 / UDP hole-punched links + $O(\log_k N)$ Seat/ALM relay trees (`k = 8` seat fan-out) so a 5,000-member public room post costs seat fan-out (`~43` hops) rather than `5,000` direct unicasts.
- **Cover-replaced X-Wing onion routing (`CONP`, `15,296 B`)**: 3-hop hybrid X25519 + ML-KEM-768 onion packets (`3 × 1,264 B` headers + `11,488 B` payload) replace scheduled `16 KiB` chaff frames on shaped links (**`+0 B` extra wire bytes**).
- **Reticulum IPv6 / NAT traversal vs. $k=2$ homed leaves**: nodes with IPv6 or hole-punched UDP connect directly; Hard-NAT / CGNAT peers attach as $k=2$ homed leaves (`cap = 36` on `server`, `4` on `home`).
- **Collapsible `▸ Advanced` opt-in & sustainability model**: exposes device tier shares (`% server`, `% home`, `% phone`), phone Wi-Fi vs. cellular + charging share, Hard-NAT share, and the 8 Constitutional opt-in choices with explicit **Benefit (`+`)** and **Downside (`−`)** alongside live **Hard-NAT leaf load** and **Shaped onion cover capacity** (`each_home_adds_seven` vs. `unowned_only_strands_the_73rd`).
- **Door checks per second**: ~15 M/s in WASM against ~6 M/s in JavaScript on one core at 1 M nodes. It runs the admission predicate, not the policy folds.
