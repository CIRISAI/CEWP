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

## Dwellings per building (`docs/geo/building_hist.json`)

The rule, in `crates/cewp-sim/src/tiling.rs` at 943ab2d (`LeafLayout::build`, 1027-1108). The
layout is a pure function of (leaf id, `LeafContext`, `MicroParams`); one `Rng::new(id.seed())`
stream (1033) feeds placement and fitting in a fixed order.

**Inputs.** The leaf's households `ctx.budget` (the popchain's apportioned households on the page);
its area `w x h` from `HolonId::size_m` (side lengths at the centre latitude, the full cell, water
included; 1028-1030); density = budget x `P::PersonsPerHousehold` (2.5, MODEL) / area (1031); class
by `DensityClass::of` (micro.rs:306-316) against `P::SuburbanDensity` 250, `P::UrbanDensity` 1,500,
`P::UrbanCoreDensity` 6,000 (micro.rs:163-165, PARAM_SPECS MODEL), and `DENSE_FABRIC_KM2` 12,000
(870, MODEL in its doc, no PARAM_SPECS entry) splitting apartment blocks from dense fabric (1101).
The streets: lines inherited from ancestors plus local ones at `street_spacing_m(hh/km2) =
clamp(3400 / (hh/km2)^0.4, 60, 3000)` (491, "MODEL, a visual rule") per ancestor level, at most
ceil(budget/3) per level (563-603, with `ctx.chain`), and side crossings from the neighbours'
budgets (611ff). Street width by class (1052-1057): main 12 m (`MAIN_ROAD_M`), dense lanes 4 m
(`DENSE_LANE_M`, MODEL), apartment and terraced 8, suburban 6, village 5 (literals, unlabelled).

**Street-frontage classes** (village, suburban, terraced, apartment blocks; `place_buildings`,
1194-1244). Along each road, each side, starting 2-10 m in (1208), slot after slot: draw a building
(1210-1231), place it `width/2 + setback + d/2` off the road's axis (1234), keep it if `fits`
(1176-1192: corners at least `SIDE_INSET` = 7 m inside the leaf, the footprint 1 m clear of every
road, 0.6 m clear of every building already kept; first come, first kept), then advance `w + gap`
whether or not it was kept. The draws are literals (MODEL by the classes.json text, unlabelled):

| class | frontage w (m) | depth d (m) | floors | dwellings | gap (m) | setback (m) |
|---|---|---|---|---|---|---|
| apartment blocks | 16-28 | 11-15 | 4-7 | round(w/9) x floors = 8..21 | 0.5-3 | 1.5 |
| terraced | 6.5-9 | 8.5-11 | 2-3 | 1 or 2 | 0.4 | 3 |
| suburban | 9.5-13 | 8.5-12 | 1-2 | 1 | 9-15 | 7-11 |
| village house (82%) | 8.5-13 | 8-11 | 1-2 | 1 | 14-50 | 6-13 |
| village barn (18%) | 14-22 | 10-14 | 1 | 0 | 12-40 | 8-14 |

**Dense fabric** (`place_dense_fabric`, 1114-1174). Rows from the south edge up, plots 8-13 m wide
by 8-12 m deep (a plot or row that does not fit retries at 8 m), 1-2.5 m apart, 0.5 m clear of
lanes and 0.6 m of each other, inset 7 m on a side a main road crosses, else 2.5 m (1120). Then the
households are shared evenly: `each = budget / plots`, the first `budget mod plots` plots get one
more (1162-1170); when there are more plots than households only the first `budget` plots in
placement order are kept, one dwelling each (1164-1167). Floors = max(ceil(units / round(w/6)), 3),
at most 60 (1171-1172; `DENSE_MIN_FLOORS` 3, MODEL). Dwellings per building here are a by-product
of plot count against budget, not a drawn building type.

**Fitting to the budget** (`fit_households`, 1278-1323), every class:
* Too many dwellings: pick a random dwelling building (uniform over them, 1285); in apartment and
  terraced leaves take one dwelling off it if it has more than one, else (and always for suburban
  and village houses) remove the building (1286-1292). Floors are never lowered. Barns are never
  picked (units 0).
* Too few: if no building holds a dwelling, one block on the first free spot nearest the centre,
  side `clamp(sqrt(budget) x 6, 10, min(w, h) - 4)` shrinking by 0.7 to 6 m (`free_spot`,
  1248-1275); if none fits, `unplaced` = true and a 6 x 5 m block at the centre regardless
  (1300-1306). Then dwellings are added one at a time, round robin over the dwelling buildings in
  descending footprint order (ties to the lower index), each time raising floors to at least
  ceil(units / round(w/9)), at most 60 (1308-1321).

So the drawn types survive only where the frontage holds about the budget. Where it holds more,
apartment blocks are whittled down one flat at a time at random and houses are deleted at random;
where it holds less, the largest buildings are stacked with extra dwellings and storeys,
whatever the class.

**What the code can produce that looks wrong** (verified in the file below unless marked):
1. Village barns are never trimmed. Houses are removed down to the budget, barns stay at frontage
   density: synthetic village leaves hold 79,549 barns beside 65,827 dwelling buildings (at 5
   people/km2: 12,084 barns, 2,666 homes). The doc comment at 1276-1277 says houses beyond the
   budget "become empty lots"; the code removes them (1291).
2. Apartment blocks whittled to one or two flats on 4-7 storeys (floors are never lowered): 1,098 of
   11,788 synthetic apartment buildings hold 1 or 2 dwellings.
3. Top-up stacks detached houses: suburban houses (w < 13.5 m, so one flat a floor) reach 28
   dwellings and 10 storeys, terraced houses 31 dwellings and 29 storeys, a village house 18.
4. Dense fabric keeps at least 3 storeys for a one-dwelling plot (44,136 of 149,531 real
   dense-fabric buildings hold 1 dwelling on 3 or more storeys), and a dense leaf with more plots
   than households keeps the plots of its southern rows only (1164-1167; inferred from the row
   order, not measured).
5. Flats per floor are round(w/6) in dense fabric (1171) but round(w/9) in the top-up (1317).
6. The unplaced fallback puts every dwelling of the leaf in one 6 x 5 m block at the centre
   (native count of unplaced leaves: 15 synthetic dense fabric, 76 real dense fabric).
7. Units are capped at 65,535 and floors at 60; neither binds in a leaf of at most 60 households
   (inferred), but an L12 leaf may hold more.

**The file.** `node docs/geo/building_hist.js --write`; `docs/geo/building_hist.test.js` regenerates
and compares. Two sets: `synthetic` (per class six rungs of people per km2 across its range, each a
uniform world walked by the page's tree, 400 leaves a rung, ids from the L3 tiles east of 51.0 N
10.0 E; the class is the leaf's own, so a rung near a threshold can land some leaves in the
neighbouring class) and `real` (every leaf of the L3 tile at the twelve probe places plus Berlin
centre, a Munich suburb (Unterhaching), Gelsenkirchen and a Mecklenburg village (Krakow am See)).
Per class: buildings by exact dwellings and by bands 1, 2, 3-6, 7-12, 13+, dwellings by band,
dwelling buildings and dwellings per km2 of leaf (aggregate and per leaf), floors, footprint in 10
m2 bins, and non-dwelling buildings. `--sample F` with the example's `--crosscheck-layout F`
rebuilds every layout natively and counts differences against the shipped kernel.
