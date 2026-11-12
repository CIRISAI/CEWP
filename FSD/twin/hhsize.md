# Household size on the street leaf (hhsize)

What the twin's focal street leaf does to make a household's size, read from the code, and the
histogram that code produces (`docs/geo/household_hist.json`, written by
`crates/cewp-sim/examples/household_hist.rs`, kept current by `crates/cewp-sim/tests/household_hist.rs`).
Every number here is `MODEL`: no household size in the twin comes from a survey.

## The rule (file:line at 943ab2d)

1. The page builds the leaf's world with `micro_build_leaf(id)` (`docs/globe.js:2032-2033`), after
   setting context slot 32 to the leaf's people from `pop_of` (`docs/tiling.js:53`).
   `kernel-wasm/src/lib.rs:782-793` lays the leaf out (`LeafLayout::build`, people from the context,
   `crates/cewp-sim/src/tiling.rs:1027-1034`) and calls `MicroWorld::from_leaf`
   (`crates/cewp-sim/src/micro.rs:959-969`), which sets `seed = leaf.id.seed()` and
   `target_people = leaf.people`. `micro_population` (`kernel-wasm/src/lib.rs:849-859`) exports
   each household's `persons`.
2. Seed: `HolonId::seed() = mix64(HOLON_SEED ^ pack(id) * 0xD6E8_FEB8_6659_FD93)`
   (`tiling.rs:240-242`).
3. Draw (`micro.rs:1327-1337`): for dwelling `hi = 0..dwellings` (dwellings = the sum of the
   leaf's building units = its household budget), a fresh splitmix64 stream
   `Rng::new(seed ^ (hi + 1) * 0xA24B_AED4_963E_E407)` (`micro.rs:234-236`); one uniform is
   skipped (it is the adoption uniform, `micro.rs:1349`), then
   `size = round(2.5 + 1.1 * gauss).clamp(1, 6)`, with `gauss` Box-Muller on two uniforms
   (`micro.rs:263-266`) and `round` half away from zero. 2.5 is `P::PersonsPerHousehold`
   (`micro.rs:139`, MODEL, "no CIRIS source"); the page never sets it (`docs/globe.js:2002-2015`
   resets the block and writes other slots). 1.1 is a literal in the line, unlisted in `PARAM_SPECS`.
4. Apportionment (`micro.rs:1338`, `popchain.rs:220-222`, `tiling.rs:289-304`): when the leaf
   has people, `sizes = apportion(people, drawn)`: largest remainder, each household gets
   `floor(people * drawn_i / sum(drawn))`, the leftover persons go one each to the largest
   remainders `people * drawn_i mod sum(drawn)`, ties to the lower dwelling index. A leaf with no
   people (`target_people == 0`) keeps the drawn sizes.
5. Household `hi` gets `persons = sizes[hi].min(255)` (`micro.rs:1351`). No clamp after step 4.

So the final size is `floor` or `floor + 1` of `drawn_i * people / sum(drawn)`: it moves by at
most one person from the proportional share, never back to the 1..6 clamp. A size above 6 comes
out when a leaf's draws sum to fewer than its people (a drawn 6 scaled up); a size of 0 needs a
drawn 1 whose share is under one person (draws summing to more than the people) and which loses
the remainder race. The first occurs (below); the second did not occur in 1,000,020 households
and is not excluded by the code.

What conditions the size: the leaf seed (from its holon id), the dwelling's index, the leaf's
people and its dwelling count. Not the density class, the region, the building type or the
layout: the draw reads none of them, and the apportionment reads only people and the draws.
Through the seed, the id selects which pseudo-random draws a leaf gets, not their distribution.

## The histogram (`docs/geo/household_hist.json`)

16,667 leaves of 60 households and 150 people (1,000,020 households, 2,500,050 persons), synthetic
ids `HolonId { level: 6, i: 0, j: k }`, `k = 0..16666`, default parameters, built by
`MicroWorld::from_leaf` on a one-building layout (the layout does not reach the size). The drawn
sizes are the same call with the leaf's people set to 0.

| size | drawn | final |
|---:|---:|---:|
| 1 | 182,423 | 182,423 |
| 2 | 317,191 | 321,136 |
| 3 | 318,740 | 354,758 |
| 4 | 147,015 | 106,254 |
| 5 | 31,401 | 27,337 |
| 6 | 3,250 | 7,404 |
| 7 | 0 | 708 |

Mean drawn 2,537,590 / 1,000,020 = 253759/100002 (2.537539); final exactly 5/2. W1(drawn,
final) = 50276/1000020 = 12569/250005 (0.050275). 113,066 households change (75,303 by -1,
37,763 by +1). Regenerate with
`cargo run --release -p cewp-sim --example household_hist -- --write`; `--check` compares.

## Validity envelope

The leaf shape (60 households, 150 people, 2.5 per household) is the twin's own default ratio, so
the apportionment here only corrects each leaf's draw sum back to 150. A leaf whose people per
household differ from 2.5 (real leaves inherit people and households down the popchain, both
rounded) rescales the draws further: more sizes above 6 when people per household is higher, and
the possibility of size 0 when it is lower. This file does not cover those leaves.

## The real leaves (`docs/geo/household_hist_real.json`)

`node docs/geo/household_hist_real.js --write` walks the page's own path at twelve probe places
(Zurich, Cairo, Lagos, the Maryland suburb, the Cher village, Dhaka, Mumbai, Karachi, Kowloon,
Manhattan, Tokyo, Paris; coordinates as in `docs/geo/popcond.test.js`): the shipped GHS-POP assets
through `geodata.js` `geoPopFine` into the shipped kernel (`docs/kernel/kernel_scale.wasm`,
`pop_load`), the page's tree (`HOLON.tree` with `tiling.js`'s `tlRoot` / `tlSplit` callbacks),
every leaf under the place's L3 street tile, the leaf's context with its people in slot 32
(`tlContext`), then `holon_layout` (buildings) and `micro_build_leaf` + `micro_population`
(persons). Per place, per density class (`docs/geo/classes.json`, from the layout's own density
and class) and per units-per-building band (1, 2-4, 5+), it gives leaves, households, persons,
counts by size, the mean as an exact rational, people per household over the leaves (min, lower
median, max) and the households at 0 or above 6. The file records the sha256 of every input.
`docs/geo/household_hist_real.test.js` regenerates it and compares byte for byte (`--check` does
the same from the command line).

Cross-check against the library source: `--sample F` writes every leaf's id, context, persons and
building units; `cargo run --release -p cewp-sim --example household_hist -- --crosscheck F`
rebuilds each leaf natively with `LeafLayout::build` and `MicroWorld::from_leaf`. On 16,041 leaves
(356,507 households) every household's persons agree. The building split (how a leaf's dwellings
fall into buildings) differs on 18 leaves (17 in the Manhattan tile, 1 in Karachi's), with the same
household count; the cause is not established (a shipped wasm older than the source, or the
layout's transcendental functions rounding differently in wasm and native). The json follows the
shipped kernel, which is what the page runs.

## Where building type is available to the size rule

The sizes are drawn over the flat list of the leaf's dwellings (`micro.rs:1330-1337`) and
apportioned (`micro.rs:1338`) before the building loop (`micro.rs:1344-1351`), which is the first
place a household meets its building (`b = self.buildings[bi]`, `b.units`, `b.floors`; the
household keeps `Household::building`, `micro.rs:408-410`). The draw reads no building today. A
rule conditioned on building type needs the dwelling's building beside its index at the draw: the
units per dwelling are `buildings.flat_map(|b| repeat(b.units).take(b.units))` in the same order
as the dwellings, available from `self.buildings` before the draw (the leaf's layout is fixed by
then: `from_leaf` copies it, `micro.rs:965-966`). Two cautions for such a rule: in dense fabric
(`tiling.rs:1161-1172`) a building's units are the leaf's households shared evenly over its plots,
`floor` or `ceil(households / plots)`, a by-product of the plot count and not a building type; and
apportionment afterwards rescales every size by the leaf's people over its draw sum, whatever the
draw was conditioned on.
