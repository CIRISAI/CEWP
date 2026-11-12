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

## At scale: billions of people and devices, one rulebook in WebAssembly

[`../globe.html`](../globe.html) models up to **8.1 billion people and 18.1 billion devices** (`5.00 B` people / `11.25 B` devices default at `2.25` devices per person's `self` roster), placed on a land-bounded world population density grid (`2,961` populated land cells across `181` localities and `9` GSMA/UN regions, stitched to continent coastlines with zero ocean nodes), and signs random acts every tick: 65% family-scope, 30% in a community room, 4% consent-free rows, 1% in a public room. Each node's local door evaluates the same constitutional rulebook (`transitions.json` + `governance_policy.json`) at the signer's device and at every device of the row's audience. The loop is [`kernel-wasm/src/lib.rs`](../../kernel-wasm/src/lib.rs), compiled to `kernel_scale.wasm` (about 170 KB with the micro tier, no dependencies, `kernel-wasm/build.sh`), with the same loop in JavaScript on the page so the two engines can be timed on one world.

What it models and measures (per **CC RC17 §4.5.13 / §5.3.3.2 / §5.4.3 / §5.4.6 / §6.1.6** and **CIRISEdge v45.0.0**, with v44 as the comparison):
- **The locality dividend (CC 5.2)**: the share of rows that never leave the signer's locality (~90% with community rooms drawn 90% local: set by the room mix, an input, MODEL; by delivery it is far lower, see `FSD/twin/locality.md`).
- **CC §5.4.3 budget classes + CC §6.1.6 `deterministic-alm` member relay tree**:
  - **Device-tier 1-hop shaped (`L ≤ 8`, CC §5.4.3)**: `server` (`7,000 B/s` = `605 MB/day`), `home` (`2,800 B/s` = `242 MB/day`), `phone-wifi` (`1,400 B/s` = `121 MB/day`), and `phone-cellular` (`350 B/s` = `30 MB/day`), each at cover fraction $\rho = 0.5$ (`W = 600 s`, or `3,600 s` on cellular; channel payload MDU `≈ 400 B` at the Reticulum `500 B` base MTU or `≈ 16 KB` / `16,272 B` over TCP). **A shaped link is 1 direct hop** between its two ends, never forwarded through a third node; a peer with no direct path gets no shaped link.
  - **Serve-tier shaped (`seat`, CC §5.4.3)**: `33,000 B/s` (`2.85 GB/day/slot`, $\rho = 0.5$, `W = 600 s`) canonical serve seats (`serve_max = 8`).
  - **Ordinary links + CC §6.1.6 `deterministic-alm` member relay tree**: Reticulum direct IPv6 / UDP-punched links + $O(\log_k N)$ **CC §6.1.6 `deterministic-alm` member relay tree** (`k = 8`) over sealed chunks so a 5,000-member room post costs member relay tree fan-out (`~43` hops) rather than `5,000` direct unicasts (distinct from **CC §5.3.3.2**, where the realtime profile has **no selective-forwarding relay role**).
- **Cover-replaced X-Wing onion routing (`CONP`, `15,296 B`, CC §5.4.6)**: 3-hop hybrid X25519 + ML-KEM-768 onion packets (`3 × 1,264 B` headers + `11,488 B` payload) replace scheduled chaff frames on TCP shaped links (**`+0 B` extra wire bytes**).
- **Reticulum IPv6 / NAT traversal vs. $k=2$ homed leaves (`reach_home_share`)**: [`formal/transport_tiers/Tiers.lean`](https://github.com/CIRISAI/CIRISConstitution) proves the 8-link bound (`L ≤ 8`), displacement, carriage/referral reach, and **`reach ≠ capacity`** (it has no reachability term); directly reachable home share (`reach_home_share`) is an explicit scenario input (`leviculum#100` first empirical NAT-mix data; `leviculum#101` notes neither PCP/NAT-PMP nor UPnP is implemented). Peers with no direct path attach as $k=2$ homed leaves (`cap = 32` $k=2$ leaves on `server`, `8` on directly reachable `home`) and carry identity, consent, chat, and small attachments on ordinary links.
- **Collapsible `▸ Advanced` opt-in & sustainability model**: exposes device tier shares (`% server`, `% home`, `% phone`), phone Wi-Fi vs. cellular + charging share, directly reachable home share (`reach_home_share`), no-direct-path share, and the 8 Constitutional opt-in choices with explicit **Benefit (`+`)** and **Downside (`−`)** alongside live **No-direct-path leaf load** and **1-hop shaped onion cover capacity** (`each_home_adds_seven` when reachable vs. `unowned_only_strands_the_73rd`).
- **Door checks per second**: ~15 M/s in WASM against ~6 M/s in JavaScript on one core at 1 M nodes. It runs the admission predicate, not the policy folds.

## Down to one street: the R5 micro tier

Zoom the globe past regional scale and the page tiles the land with neighbourhoods (leaf holons of the street tiling, out to an acuity ring set by the frame rate) and builds a **focal 1:1 neighbourhood** on the leaf under the camera. That world is [`crates/cewp-sim/src/micro.rs`](../../crates/cewp-sim/src/micro.rs), unit-tested in `cewp-sim` and compiled into the same wasm (`micro_*` exports). The page draws only what it exports: streets, houses, every device, every one-hop link with its shaped class, pending punches, and each row along the path it actually took.

- **Host kinds:**
  - **phone:** client-only; its class follows Wi-Fi or cellular; it suspends and resumes.
  - **home node:** always on; the member-owned mailbox.
  - **laptop:** runs as `home` by platform default and relays with no battery gate, yet sleeps.
  - **hosted server:** `server` class; shapes every identity link by default.
  - **serve seat:** `seat` to seats, `server` to everyone else; facilitator, federation and last-resort mailbox.
  - **CI runner:** ephemeral, sometimes on an older edge that never proposes a direct path.
- **Paths:**
  - **direct:** a LAN, one end's public listener, or a facilitated punch.
  - **with no direct path:** a row crosses a forwarder sealed and unshaped.
  - **at the bound:** a ninth shaped link runs unshaped.
  - **to a sleeping phone:** a row waits at the household's home node or server, else at a seat.
- **Provenance:** every number without a source in CIRISEdge or the constitution is a named parameter tagged `MODEL` or `FRONTIER`. The table is in [`FSD/SIMULATION_ENGINE.md` §4.6](../../FSD/SIMULATION_ENGINE.md). Notable FRONTIER defaults:
  - `reach_home_share` 0.40;
  - `cellular_punchable_share` 0.15;
  - `punch_success_share` 0.90 (leviculum#100 has no fleet data yet).
- **Places:** streets and homes are procedurally generated per leaf from coarse inputs only: the population grid, the Natural Earth 50m land mask and populated-place weights (apartment blocks, terraced streets, suburbs, villages). They are not map data: real geography stops at city and region scale, and no simulated device sits on a real street or building. The ground colour under them is a display rule (MODEL, `docs/tiling.js` `tlTerrain`): latitude and ETOPO elevation give a warmth that picks one of five ground bands, ground above 2,200 m blends to rock, and Natural Earth's shaded relief lights it; no land-cover or climate data is loaded, so desert and wet cities at one latitude and height look alike. A populated place with no population-grid cell within 1.6° brings its Natural Earth population to its L0 cell (MODEL, `tlRoot`), which is why Reykjavik and Manaus have streets. Land with no households keeps its ground and is not divided.
- **Who opens a link, and who gets served:** every link has an initiator. A phone always dials; a seat never dials a device and sends only on a link the device opened. A device has a seat as replication peer only through consent, which needs an owner. The world defaults to the v45 shape: bundle bootstrap (edge #881) on, so an unclaimed device rounds with the bundle's seats by required-set Pull. The chip's switch names the alternatives: v44 (off: an unclaimed device announces but no seat serves it) and whole-plane (on with the whole-plane diff, never shipped). The chip shows the switch, and a device's card shows its rounds (the model's reading).
- **Seat load on the node:** a serve seat's marker and card show its CPU and read rate from [`crates/cewp-sim/src/capacity.rs`](../../crates/cewp-sim/src/capacity.rs), the cost ledger calibrated to the production seat (FSD §4.7): for this patch's devices, and at 1,000, 10,000 and 100,000 devices, for edge v45.0.0 with the read fix (the default; PROVISIONAL in the two terms the card names) and for the alternatives the chip's switch names (MODEL). The figures include today's measured peer load at the seat as a fixed baseline (`today_peers_booked`).
- **Story:** acts 1 and 2 descend from orbit to such a neighbourhood near Zurich, tiled among its neighbours like any other. Alice's phone and home node, Bob down the street, their seats in distant data centres: the narration fires the actions, and the rules decide how each one goes. Then the camera pulls back to the planetary acts.
