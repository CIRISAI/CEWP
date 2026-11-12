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
