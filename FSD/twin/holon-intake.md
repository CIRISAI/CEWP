# Twin package: CIRISHolon intake (consume Holon's statistics, draw them the same day)

The project's instruction (2026-11-11): consume CIRISHolon's statistics as they become available and draw them as
beautifully and efficiently as possible. This is the design; the first instance is UNEVEN-0
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

## 6. Built (status)

- **Intake** (tools/holon/import.py, docs/geo/holon/, intake.test.js): UNEVEN-0, HOUSEHOLD-0 US and
  HOUSEHOLD-0 DE at CIRISHolon 7783c24, with ledger entries carrying the notices. Assets are small
  (UNEVEN-0 15 kB, each household table 5 kB as JSON), so they ship as JSON parsed on demand rather
  than binary; they load at street zoom or with a world-model layer, never at first paint.
- **In-cell seam** (crates/cewp-sim/src/popchain.rs `incell`, on by default; kernel `pop_incell`,
  `pop_cell_pixels`, `pop_anchors`): a cell's people over its 15 x 15 pixels by the regional UNEVEN-0
  row where published (renormalised import, crates/cewp-sim/src/holon_tables.rs), else Holon's own
  world constants, with Holon's apportionment and seed rule; it matches `world::incell::divide`
  class by class on the world table (popchain test). The region of a cell is compiled in
  (crates/cewp-sim/src/regions_table.rs from regions.json, nearest listed parent for unlisted cells).
  Placement (MODEL): the 1/8-degree people field interpolated at the pixel plus a pull toward the
  cell's Natural Earth place. Exact sums at every split; check_sums rc 0.
- **Drawing**: docs/incell.js, one 180 x 180 R8 texture per 1.5-degree block, built once (about 7 ms a
  block in wasm, 32 kB of texture), drawn inside the land stencil at region and city zoom, hatched
  (unresolved); the world-model layers carry the regional chart and the household verdicts.
- **Owed**: the cell grid still shows (classes are quantised per cell and the table does not place
  towns); a rhythm Series renderer; envelope greying needs the country test (Natural Earth admin-0
  polygons, approval asked).
- **Under 4x CPU throttle (390 x 844)**: in-cell blocks build off the frame, cell by cell, in idle slices
  (at least 2 ms a slice when the thread is never idle); the longest slice measured 2.9 ms. The tree's
  splits now read memoised cell pixels (kernel), which took the camera jump's longest task from 2.1 s
  to 0.36 s at the Ruhr (both seams off: 0.15 s); max frame 67 ms (was 2,083 ms). Blocks fill
  gradually: about 6 s per block on a busy throttled thread.
- **Household seam** (popchain::households, kernel pop_households / pop_household_seam, on by default):
  `holon_scene::household::generate` once per L4 tile with the tile's people, its class (explicit
  map 1..5 at 0 / 250 / 1,500 / 6,000 / 12,000 per km2, tested) and the reference rule (MODEL: German
  2022 rows everywhere, in_country false until the country test exists, so every call is unresolved
  "outside the country read"); whole households to the tile's leaves by cumulative midpoint; the
  layout and the focal world both read the leaf's share, so they agree. The leaf rule is unchanged:
  the tree's households above the tile are the chain's people / 2.5, so "at most 60 households" is
  "at most about 150 people". One digest over a fixed grid of generator calls is the same natively and
  in wasm (3604435168484842135, docs/geo/holon/housegen_digest.txt). Wasm 1,219,716 bytes (+80 kB
  with holon-scene, the tables and the region table). A cold tile costs about 27 ms once (Lyon, 136
  leaves) and is cached; steady frames are unchanged (16.7 ms p50 at 1440 and 390, on and off).
  check_sums holds with the tile lines (households and people at the tile over its leaves).
- **Device seam** (micro.rs `DeviceRule` / `DeviceSeam`, kernel `micro_device_rule(on, where)` /
  `micro_device_seam` / `pop_device_digest`, on by default; layer "Device rule: CIRISHolon / legacy"):
  `holon_scene::devices::generate` (DEVICEGEN-1, CIRISHolon 33dc99f) once per household with its
  persons, its building type (1 unit house, 5 or more apartment, dense fabric or 2-4 units unknown) and
  in_country = the tile's Natural Earth country is the United States (840); elsewhere every call is
  unresolved and still generates. Phones by person group and laptops are drawn as nodes; desktops are
  counted in the device population, not drawn. The household's sizes are never changed. The legacy
  rule's random stream is still read, so its other draws keep their order (the histograms are
  unchanged; only their wasm hash moved). One digest over a fixed grid of calls is the same natively
  and in wasm (11302191051290176534, docs/geo/holon/devicegen_digest.txt). Wasm 1,390,075 bytes
  (+70 kB over 1,320,017).
- **Where a home node runs** is a MODEL scenario on the page (DEVICEGEN-1 declares a gateway with a
  fixed line, whether it can host unknown, and a desktop, whether it is on all day unknown); a home
  node is drawn at the legacy 0.6 share among the households the scenario lets host. Shares
  (crates/cewp-sim/examples/devicegen_shares.rs: 162,879 US households from HOUSEGEN-1, class 2,
  buildings in equal thirds; MODEL):

  | scenario | households that can host | home nodes |
  |---|---|---|
  | on the gateway (fixed line), default | 71.6 % | 43.0 % |
  | on an existing computer (desktop) | 38.6 % | 23.2 % |
  | on a new box (any internet at home) | 92.4 % | 55.5 % |
  | none (phones only) | 0 % | 0 % |
  | legacy rule (no generator) | all | 60 % |

  9.2 % of households have nothing to host on (no computer and no fixed line), certified in the country.
- **Data maps** (the strip None / People / Awake / Verdict / Households; incell.js shader modes, region and
  city zoom). Households: each ~0.9 km pixel is tinted by the mean of the HOUSEGEN-1 row the generator
  draws from at that pixel's density class (the pixel standing for its ~1 km tile; kernel
  `pop_household_means`, `popchain::households::row_mean`): US 2024 rows in the United States, German
  2022 rows elsewhere, hatched more outside Germany. That is the expectation, not one tile's draw: tiles
  give it pooled (400 tiles per row within 0.01, popchain test), one tile's mean wanders by about
  1.2 / sqrt(its households), and the chip gives the focal tile's own generated mean beside its row's.
  housegen.test.js checks the wasm's means against the rows at the pinned rev. The US rows run only
  2.27 to 2.51 across classes, so US cities and suburbs read nearly alike; the German rows run 1.71 to
  2.27.
- **Data maps after the review (2026-11-12).** None is the default and the plain view (the faint warm in-cell
  tint). Any other map lays a veil over the land (a dim neutral base inside the land stencil, faded with the
  map) and draws its quantity through its own sequential ramp texture: People inferno, Awake navy to amber,
  Households viridis, Verdict teal with hatching (incell.js DM_RAMPS; the legend bars use the same stops). The
  night glow is off where a map draws. Legends are two lines (name and ramp with ticks and units, or swatches;
  the table's badge), the prose behind "where this comes from". The strip and legend are one block with a
  backdrop (under the tier pill on a phone); the live-mesh banner closes to one line while a map is on; the chip
  ends clear of the zoom column on a phone. Awake also draws at continental zoom: one mesh over North America,
  each 0.5-degree longitude column at its local solar hour (lon / 15, not civil time zones), the United States
  from the Natural Earth mask, so the day's edge crosses the country; no people weighting at that zoom. At
  street zoom the chip's Awake line counts the street's adults from their own generated days (PRESENCEGEN-2)
  beside the diaries' share.
- **No Devices map.** The share of households with nothing to host on (DEVICEGEN-1's certified reading) by
  class, 4,000 US households per class drawn by HOUSEGEN-1's rows, building type unknown:

  | class | 1 | 2 | 3 | 4 | 5 |
  |---|---|---|---|---|---|
  | nothing to host on | 8.5 % | 8.6 % | 8.9 % | 9.9 % | 10.6 % |

  The standard error is about 0.45 points, so only class 1 against class 5 is real; a map would be one colour
  over the United States, and home nodes per km2 is the People map times about 0.18. Holon certifies the
  clustering by state, which the twin cannot place. Not built.
- **Land under the street tier (fix).** Natural Earth 10 m tiles are clipped to 10-degree boxes; a box edge drawn
  as one chord bows poleward by up to a quarter degree, so the land stencil dropped a strip along every tile's
  southern edge and streets there (Columbus, 40.06 N) floated on an empty field. Ring edges are now split every
  0.1 degree (globe.js geoUpload), at most a few metres off the parallel.
- **Street day: the rhythm Series as it is** (crates/cewp-sim/src/street_rhythm.rs `street_day`, kernel
  `micro_rhythm` / `pop_presence_digest`, on by default at street zoom; layer "Street day: on / off"; ledger
  holon-presencegen2). holon-scene is pinned at CIRISHolon f07a5ee, which adds PRESENCEGEN-2 and leaves
  household.rs and devices.rs as they were, so the HOUSEGEN and DEVICEGEN digests do not move. Each adult
  of the street, as DEVICEGEN-1 drew the household (18 to 64, over 64), gets one generated day from
  `presence::day(Generator::Chain, …)`: the chain, CERTIFIED in the United States on the 2024 diaries,
  unresolved elsewhere. Employment is drawn per adult from `presence::employed_ppm` (MODEL: 18 to 64 read
  at the 25-to-64 share). Children get no day: diaries start at 15 and the device generator's children
  are under 18. The days become one `rhythm::Series`: cells are the homes, then work or school,
  elsewhere and travelling; counts are 24 slots from 4 a.m.; flows are every change of place per hour,
  summed by (from, to); the source is zero, and `check_continuity` finds nothing (test, and the chip
  line). Drawn from the Series and nothing else: each home a ring (amber awake at home, blue asleep at
  home, by the hour, interpolated); each hour's moves as dots along arcs to three away discs off the
  patch's east side (display only). The day follows the street's own clock (the chip's local time),
  dated from the sim clock. Holon's published digest 2016341816221085133 matches natively and in wasm
  (presencegen.test.js). Wasm 1,508,022 bytes (+117 kB: the presence tables). Building a street's
  day is 0.05 to 0.15 ms in the page, once per generated day.
  Example (a Columbus suburb, Monday to Thursday, 40 adults and 8 children): 03:30, 39 at home; 08:30,
  18 at home, 11 at work or school, 7 travelling; 13:00, 13 at work or school; 17:30, 23 at home.
- **Presence tint** (micro.rs `set_presence` / `person_awake`, kernel `micro_presence`; the page hands the generated
  day whenever it changes). Inside the United States under CIRISHolon's device rule, the street's phones and
  laptops follow their people: a device belongs to its household's person of the same index (adults first, as
  DEVICEGEN-1 lists phones); while that person's generated state is awake (states 1, 2, 3, 5) it signs at the
  stand-in's rate divided by the hour's PRESENCE-0 awake share (so the street's total stays the stand-in's), and
  while they sleep it does not sign and shows asleep. A device with no person's day (a child's) is awake with
  PRESENCE-0's share (twin/theory's presence0.rs, merged at 24c6fe0) by a fixed draw. Elsewhere, and under the
  legacy device draw, the stand-in (model.diurnal.v1) runs alone. Test: at 3 a.m. at least 70 % of the street's
  phones are asleep, at 1 p.m. at most 20 %; outside the country none change. Wasm 1,517,600 bytes.
