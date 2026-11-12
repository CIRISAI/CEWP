# Twin package: CIRISHolon intake (consume Holon's statistics, draw them the same day)

Eric, 2026-11-11: "we need to consume those stats from holon as they come available and draw them as
beautifully and efficiently as possible". This is the design; the first instance is UNEVEN-0
(CIRISHolon 90ef9d0, `conformance/scene/tables/uneven0/`), built after the current candidate.

## 1. Intake: one path for every table

- **Where.** `docs/geo/holon/index.json` plus one directory per table (`docs/geo/holon/<ref>/`).
  An index entry: registry rows it serves (names from `docs/geo/couplings.json`), Holon's reference id,
  the pinned Holon commit, version, sha256 and bytes of the source file and of the converted asset,
  the licence notice text verbatim from the sidecar, class (`MODEL`, dataset = the external source
  "via CIRISHolon"), envelope (regions and classes covered), and Holon's verdict per row
  (`certified` | `refused` | `unresolved` with the reason).
- **Importer.** `tools/holon/import.py <table.json> <sidecar.json>`, input read from the local
  CIRISHolon checkout at a pinned commit (`git show <commit>:<path>`); the twin fetches nothing.
  Steps: check the schema and that every count is an integer; refuse any key that carries
  coordinates, ids, names or tags of real features (statistics only, RULES); where a row has a law,
  write it in the sums format and run Holon's `check_sums`; apply and record the sidecar's stated
  fixes (UNEVEN-0: renormalise share vectors to 1,000,000 within its tolerance of 300, and log which
  vectors moved and by how much); convert to the compact asset (little-endian typed arrays, gzip,
  tiled by region when over 256 kB); write the index entry and the ledger entry
  (`crates/cewp-evidence/src/page_ledger.rs`, with the notice).
- **Test.** `docs/geo/holon/intake.test.js` fails when a table is in the index without a ledger
  entry and attribution, or a ledger entry names a table not in the index; when an asset's sha256
  differs from its index entry; or when a converted asset no longer reproduces the source table.

## 2. Consumption: one seam

`cewp_sim::popchain` gains the seam: a `Tables` value the kernel holds (loaded through one door,
`holon_table_load(kind, ptr, len)`), and every generator step that a registry row covers asks it
first. Where a table exists for the row, the generator uses it; where not, it uses today's MODEL
rule, and the row's verdict reads `unresolved: no table`. The verdict travels with the number to
the chip and the ledger.

First instance, UNEVEN-0: inside a 1/8-degree cell of class k, the cell's tiles get classes and
people by `world::incell::divide(cell_class, people, n_tiles, seed)`'s rule (grain 1, each cell
once): each tile class's share of tiles and of people from the table, apportioned exactly
(largest remainder, ties to the lower index, the rule popchain already shares with Holon), so
tiles sum to the cell for every seed. Implemented in the twin, matching Holon's apportionment and
seed so the two agree; any difference is stated. Holon's crate is not a dependency.

**Placement is the twin's display rule, MODEL**: the table says how many tiles of each class, not
which. Dense tiles go nearest the cell's Natural Earth populated place if it has one, else toward
the neighbouring cell with the most people, growing contiguously outward; the per-class counts
equal `divide()`'s. Labelled "which tile is the town is not in the table".

## 3. Drawing: every table visible the day it lands

- **A "world model" group in the layer dock**: one row per consumed table with its verdict badge
  and envelope; outside the envelope the layer is greyed (a US-only time-use table lights only
  where it applies).
- **In-cell structure as a GPU texture** at region and city zoom: population concentration at about
  1 to 4 km from the transition table, one R8 texture per region tile built once per table and
  sampled in the ground shader, blended with relief and land cover, so a metro reads as a core with
  fingers and a rural cell as scattered settlements. This replaces the flat olive region fill.
- **Rhythm tables drawn from Holon's Series as it is**: cells filling and emptying per bin, flows as
  moving arcs, a kind selector; never converted to another shape first.
- **Uncertified or out-of-envelope rows look different**: hatched or desaturated, with the reason
  on hover and tap, so the picture never looks more certain than its verdict.

**Efficiency rules.** Per-cell data goes to the GPU as textures or instanced buffers built once per
table; no per-frame JS loop over cells; tables load lazily by view and by layer, never at first
paint; the per-commit service-worker cache holds them. Every table states its budget, measured in
the report: bytes on the wire, decode ms, and frame cost at 390 x 844 with 4x CPU throttle.

## 4. Report per table

What changed on the globe (before and after at 1440 and 390), the frame cost, the sums check rc
through Holon's checker, and what the verdicts say.

## 5. The generator seam (Holon's first generator: household sizes in a tile)

Holon's interface, as stated (version one): the twin passes a 64-bit seed, the tile's people (the
integer popchain apportioned), the tile's density class 0-4 (classes.json) and its region code
(regions.json); no household count and no hint. The generator returns the tile's households as
sizes, summing to the tile's people exactly for every seed, and one verdict line (certified |
refused | unresolved, with the envelope), so a tile outside the envelope is still generated and
marked. Version one returns no building type (neither reference publishes household size jointly
with building type). Call shape `children(id, population, class, seed, want = households)`.

**The tile is L4.** The generator's references are a 1 km census tile (Germany) and a tract (US), so
it is valid at the tile tier, not at a leaf of about 40 to 150 people. L4 is the twin level nearest
1 km on a side: 0.01 degree, about 1.1 km north-south and 0.75 km east-west at 47 N, against L3's
2.2 km; so the twin calls the generator once per L4 holon, with its seed `HolonId::seed()`, its
people, its own class (people per km2 of the L4 holon through classes.json, not the leaf's) and its
region. Where a leaf is coarser than L4 (an L3 village leaf), the leaf is its own tile. Holon's
leaf-size floor (below which a direct call returns unresolved) is never hit by per-tile calls.

**One seam.** `cewp_sim::popchain::TileHouseholds` is a trait with one method,
`households(seed, people, class, region) -> (Vec<u8 size>, Verdict)`. Two implementations: `Legacy`
(today's rule: round(people / 2.5) households, sizes drawn round(2.5 + 1.1 N) clamped 1..6 then
apportioned to the people; verdict "no certificate") and, when it lands, `Holon` (a thin call into
`holon_scene`). kernel-wasm picks one in one line; the legacy rule is kept as the labelled fallback
and never tuned.

**Households to leaves, whole.** The tile's households are split over its leaves by an exact rule:
the households in the generator's order, a running sum of their sizes, and each household goes to
the leaf whose cumulative-people interval (the leaves in `children()` order, each spanning its
apportioned people) holds that household's midpoint. Every household lands whole in exactly one
leaf, so households sum exactly at the tile and at the leaves; each leaf's people then FOLLOW the
households assigned to it (the sum of their sizes), departing from the leaf's apportioned people by
at most half the largest household on each of its two interval ends, while the tile total stays
exact. The sums file carries people and households at the tile, and the leaves' people and
households as the generator's output split. A leaf's mean household size varies around the tile's
instead of being pinned.

**People are the exact quantity down to the tile.** popchain carries people from the root to the
tile as today; households stop being apportioned at every level (above the tile a household count is
a derived, labelled estimate for display and the leaf rule's legacy form).

**The leaf rule becomes people-based**: split while a holon holds more than 150 people (the bound
today's 60 households at 2.5 persons gives), at L3 or deeper. It depends only on the exact
quantity and keeps today's leaf sizes at the mean; with Germany's 2.04 persons per household a leaf
holds about 74 households, with a US-like 2.53 about 59; the R5 node budget (1,000) covers either.

**Buildings stay legacy in version one.** The layout keeps making buildings its own way, labelled
"no certificate". Building type will come from a separate generator fitted on the German building
table (shares of buildings and dwellings by type and size per class,
`conformance/scene/tables/household0_de/household_table.json`, "buildings"); then the layout takes
households by kind (single house, 2-4 units, 5+) as an input, and dense fabric's "households shared
evenly over plots" goes.

**Linking.** When the generator exists, kernel-wasm depends on `holon-scene` (AGPL, zero
dependencies, integers only); the seam makes it a one-line swap of the `TileHouseholds`
implementation. Expected cost: compiled-in tables of a few kB each, so the wasm grows by roughly
20-60 kB over today's 1.09 MB; per tile a few apportionments over at most a few thousand people,
microseconds, once per tile build and never per frame, so no frame cost. Both to be measured when built.

**What the legacy rule is, recorded so the verdicts stay true:** HOUSEHOLD-0 US certified its mean
(2.500 vs 2.532) and refused its composition (one-person share 0.182 vs 0.286); HOUSEHOLD-0 DE
refused both (2.500 vs 2.042; 0.182 vs 0.434, from 0.32 in village tiles to 0.62 in dense fabric):
the family is wrong, not its two numbers, so it is not retuned. The size-7 households come from the
missing clamp after apportionment (micro.rs populate) and are fixed with the new rule, not before.
