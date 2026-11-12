# Twin package: globe (rendering, layers, kernel-wasm)

What the public globe draws, where each mark comes from, and what it cannot say. Branch `twin/globe`.

## Rule the page keeps

A layer draws only what a kernel export produced. Illustration that is not data (ocean colour,
sun glint, the atmosphere) is named as illustration on the page (the layer dock's footnote). Every
layer carries its class as text (FSD §8.2), never by colour alone; glyph shape or dash pattern
carries category.

## Renderer (docs/gfx.js)

* **Context:** WebGL1 with `ANGLE_instanced_arrays` and `OES_standard_derivatives` (both on every
  current iOS Safari and Android Chrome). Every shader on the page is GLSL ES 1.00, shared with
  relief.js and tiling.js. A WebGL2 context would buy nothing the two extensions do not, and it would
  change extension behaviour under those shared shaders. Without instancing, each path falls back to
  the f73b918 drawing.
* **Sun and night side:** `clock_sun(t)` from the kernel on the simulation's clock. The clock runs
  only while the sim plays; the story sets its own hours. Sun: circular orbit, 23.44° tilt, no
  equation of time. Day of year `SIM_DOY = 314` (MODEL scenario parameter).
* **Who signs when, and who lights:** the clock's diurnal switch is ON by default (changed in
  twin/globe after f9cc739; Eric: "looks identical in day and night"). `tick` keeps a drawn signer
  with probability `diurnal_multiplier(local solar time) / 2.30` and redraws otherwise (at most 16
  times); `acts` is unchanged and where the acts land follows the hour (workload.rs, FSD §4.1, MODEL).
  New: a receiving node's mark in the activity buffer is scaled by the same multiplier (`rx_mark`),
  so a device in its pre-dawn trough barely lights (MODEL: it takes the rows later, from its mailbox
  or its next round); display only, no out slot depends on it. The page always passed diurnal = 1, so
  what changed on the page is the receivers' marks; the kernel default matters to callers that never
  call `clock_set`. `clock_set(.., 0)` restores the f73b918 tick exactly. No test pinned the old
  slots; `diurnal_is_on_by_default_and_night_receivers_mark_less` pins the new default.
  The daily curve (workload.rs `diurnal_multiplier`, a MODEL stand-in with no reference) is
  `model.diurnal.v1` since twin/globe after 322b63c (Eric: "0.2 is too low and the current high is too
  high"): trough `DIURNAL_TROUGH` 0.50x around 03:10, peak `DIURNAL_PEAK` 1.60x at
  `DIURNAL_PEAK_HOUR` 19:30, 0.89x to 1.21x from 09:00 to 16:00, integrating to 1.0. Hourly values
  00-23 h: 0.89 0.69 0.56 0.50 0.53 0.61 0.72 0.83 0.91 0.94 0.94 0.91 0.89 0.89 0.95 1.06 1.21 1.37
  1.51 1.59 1.59 1.50 1.33 1.11. `model.diurnal.v0` was 1.57 1.23 0.90 0.62 0.42 0.30 0.23 0.21 0.20
  0.20 0.21 0.23 0.30 0.42 0.62 0.90 1.23 1.57 1.90 2.15 2.28 2.28 2.15 1.90 (peak/trough 11.5, the low
  at 08-09 local, not before dawn as the text below said). Reason: people's AI agents, scheduled
  activity and other always-on use give the mesh a floor human-only traffic lacks, and messaging is a
  waking-hours plateau. Two harmonics cannot hold a full 1.0 plateau while hitting the trough, peak and
  hour exactly; the plateau's morning sits at 0.89x to 0.95x. One tick at 03.1 h UTC, acts by the
  locality's local hour 17-24 / 0-7 / 7-17 h: v0 935 / 554 / 511, v1 376 / 452 / 1,172; sunlit / dark
  side v0 514 / 1,486, v1 1,175 / 825 (MODEL). Under v0 below, the measured tick read as follows.
  Note the v0 rhythm: it peaked at 20:30 local, so the busy
  side was the evening side, not the sunlit side. One measured tick (MODEL, the page at 21.98 h UTC):
  acts signed in localities whose local hour is 17-24 h / 0-7 h / 7-17 h = 977 / 816 / 207 with the
  switch on, 386 / 1,277 / 337 with it off (ten ticks: 9,552 / 8,718 / 1,730 against 3,191 / 11,715 /
  5,094); sunlit / dark side 300 / 1,700 (on) against 620 / 1,380 (off). The dark side holds most of
  the world's people at that hour (Asia before dawn), which is why it still signs most. Cost: tick p50 18.4 → 18.6 ms
  (1 M nodes, 10 k acts, Intel UHD, wasm). This replaced two JavaScript stand-ins: a
  `Math.random()` diurnal filter on arcs, and a hash-driven flicker in the point shader.
* **Night glow:** the population grid the sim places people on (MODEL input), as a 1024 × 512
  equirectangular image, blurred 3 px, scaled so the 99th percentile is full brightness. It is lit
  where the kernel's sun is below the horizon and fades out by 500 m a pixel. It is not satellite
  night lights.
* **Relief:** ETOPO 2022 elevation gives a normal from four neighbouring texels, lit by the
  kernel's sun by day. Vertical exaggeration is 1 + 2·√(texel / 1.85 km): 3× on the 1/60° tiles,
  about 7.5× on the globe image (display only). By night the baked Natural Earth relief keeps the
  ground legible, as before. Over the tiling's holon mosaic the same relief is multiplied
  continuously. Two per-holon effects are gone: a flat relief step per holon, and a ±4 % random
  lightness jitter per holon (decoration that read as data).
* **Lines:** every gl.LINES segment becomes an instanced screen-space quad with one pixel of
  coverage falloff. Endpoints are read in place from the existing buffers (stride 24 B), so no line
  data is duplicated.
* **Arcs:** instanced ribbons, one instance per arc. The great circle and its lift are computed on
  the GPU. A point is hidden exactly when the sphere is between it and the eye (orthographic:
  `q.z < 0 && |q.xy| < 1`). Tier by dash pattern as well as colour: shaped solid, seat long dash,
  relay tree beads, onion double rule.
* **Budget:** 16.7 ms a frame. The resolution governor lowers the backing-store scale 15 % after
  20 slow frames (EMA > 1.25 × budget) and raises it 10 % after 120 fast ones. Frames that ran a
  sim tick are excluded from the governor's timing. Device pixel ratio is capped at 2.
* **Idle:** when the sim is paused, the camera is still, nothing is fading and nothing is loading,
  the loop skips the frame: no draw call, no GPU work. Any texture or buffer upload, and any input,
  asks for frames again.
* **Reduced motion:** with `prefers-reduced-motion` there is no auto-rotate and arcs are drawn still
  (no flowing heads). Auto-rotate also stops while the sim is paused.
* **Nothing flashes (WCAG 2.3.1; Eric: "seizure flashy").** A dot's drawn activity is a display
  envelope following the tick's pulse at a bounded slew (`ACT_RAMP_MS` 450 up and down; the pulse
  fades linearly over `ACT_HOLD_MS` 900), and the point shader maps it continuously (colour, size,
  the visibility of idle dots: no thresholds, no hash flicker; the `floor(uTime*2.5)` term is gone).
  At orbit zoom at most `ARC_ADD_ORBIT` = 2 arcs are added a tick (a sample; the arcs card says so),
  each fading in over at least 0.6 s and out over at least 1.4 s; heads are capped and arcs blend
  with ordinary alpha, so crossings never sum to white. Measured headless at first load, 10 s, every
  animation frame, 160 × 160 over the disc (`flicker.py`; 30-59 fps on the test box, so per frame,
  not per 1/60 s), before (eefac8c) → after (this change): globe.html at 1440 × 900, largest
  frame-to-frame mean-luminance change over the disc 5.23 → 0.20 (of 255), luminance reversals per
  second in the busiest 10 % of 8 × 8 blocks 10.85 → 2.43, largest single-block frame change
  76.8 → 17.4; at 390 × 844, 4.08 → 0.26, 10.40 → 3.14, 67.1 → 14.2. index.html opens on the
  Story, which dives from orbit to a street at about 5 s; the dive moves whole blocks by camera
  motion (the block maximum stays high: 73 at 1440, 134 at 390, measured through the zoom), not
  by flashing; its mean-luminance step went 7.0 → 2.4 (1440) and 13.6 → 3.2 (390) and its reversal
  rate 7.43 → 1.09 and 6.35 → 1.81. The lit hour slews at most `SUN_SLEW_H_PER_S` = 3 h a second,
  so a Story act change (whose scripted hours jump) or leaving the Story moves the terminator over
  seconds instead of relighting the disc in one frame.
* **The idle turn:** one turn in six minutes (`ROT_DEG_PER_S` = 1), time-based (the same on a 120 Hz
  screen), after 8 s with no input (`ROT_IDLE_MS`), stopped for the rest of the visit by any pointer,
  wheel, touch or key input, off under reduced motion; the Story's last act keeps its own slow turn
  (`STORY_ROT_RAD_PER_S`, now time-based). The first view faces the visitor's own longitude, read
  from their time zone (15° an hour; no location is asked for), and the sim clock opens at the real
  UTC time of day, so that side is lit as it is now.

## Export contract (kernel-wasm/src/globe.rs; buffers via `globe_buf_ptr(which)` / `globe_buf_len(which)`)

| export | buffer | layout | cadence | class |
|---|---|---|---|---|
| `clock_set(t_s, span_s, doy, diurnal)` | – | sets the clock the next `tick` uses | every tick | MODEL |
| `clock_sun(t_s)` | 0 sun, f64 × 8 | subsolar lat°, lon°, UTC hour, day, diurnal at subsolar point, diurnal peak, 0, 0 | every drawn frame | MODEL |
| `clock_diurnal_bins(t_s, bins)` | 1, f64 × bins | diurnal multiplier per longitude bin | on demand | MODEL |
| `locality_centroids(lat, lon, metro, n)` | 2, f64 × 3 × 181 | lat°, lon°, nodes | when the world is rebuilt | DERIVED |
| `tick` (existing; now also fills) | 10 flows, u32 181 × 181 | row = from locality, col = to; Σ = out[6] + out[7] | every tick (120 ms) | DERIVED |
| `tick` | 11, u32 × 181 | acts signed per locality | every tick | DERIVED |
| `tick` | 12, u32 × 2 | diurnal redraws, 181 | every tick | DERIVED |
| `alm_room(lat, lon, room_off, room_mem, room, publisher, max, seed)` | 3 members u32; 4 edges u32 × 3 (parent ix, child ix, depth); 5 unrooted u32; 6 member f64 × 4 (class, uplink Mbit/s, depth or -1, children) | `cewp_sim::alm::compute_alm_topology_verified_with_publisher` | when shown or when the room changes | DERIVED from MODEL inputs |
| `alm_bounds(0/1)` | – | fan-out cap 8, RTT ceiling 400 ms (alm.rs consts) | on demand | DERIVED |
| `custody_room(lat, lon, metro, room_off, room_mem, room, present_pm, seed)` | 7 f64 × 9: members, present, target 20, min viable 6, state 0/1/2, Kish n_eff, localities, mean km, max km; 8 u32 × 2: holder, reachable | `chunk_custody::classify`, `effective_source_count` | when shown or when the slider moves | DERIVED; reachability draw MODEL |
| `cap_curve(d0, d1, n, shape, read, seats, select, cand)` | 9 f64 × 4: devices, ρ cpu, ρ permits, round latency (-1 unbounded) | `capacity::run` over n fleet sizes; no knee reported | when shown or when the bootstrap setting changes | as capacity.rs COST_SPECS |

MODEL inputs to `alm_room`:
* Device class shares 30/30/15/25 % (phone cellular, phone Wi-Fi, laptop, home node), with uplinks
  of 2/8/12/20 Mbit/s and max subscribers of 1/2/4/8.
* RTT = 10 ms + 2 × 1.6 × great-circle km / 200 km/ms.
* Every member grants every other at trust depth 1.

These are round figures, not measurements; topology/nat/media replace them in Phase 2.

## Layers (docs/layers/layers.js; registry public as `window.CEWP_LAYERS`)

Registry contract (agreed with the evidence package): each shown layer registers at load as the literal
`window.CEWP_LAYERS["<id>"] = layer({ id: "<id>", title: ..., cls: "<CLASS>", ...` with `cls` exactly one
of MEASURED, DERIVED, MODEL, PROJECTED, FRONTIER; the evidence scan needs a ledger entry `layer-<id>` of the
same class. `last` is a flat name -> number map of what the layer drew; names and lists go in `info`. The class
chip is `<button data-layer-claim="<id>">` with the two-letter code; the evidence package handles its click and
the dock never stops that event. Phase 2 contracts are not in `CEWP_LAYERS` until their export exists.

| id | draws | export | class |
|---|---|---|---|
| illustration | ocean colour, sun glint, atmosphere limb: decoration, draws no data; off falls back to the f73b918 flat disc | none (shader in gfx.js, lit by `clock_sun`) | MODEL |
| glow | night side lit where people are | population grid + `clock_sun` | MODEL |
| arcs | a sample of the tick's deliveries | `tick` arcs buffer | DERIVED (sample) |
| flows | top 60 locality pairs over the last 25 ticks, width ∝ log count, counts on the top 5 | buffers 10, 2 | DERIVED |
| alm | one room's member relay tree: publisher triangle, members by depth, unrooted as crosses | buffers 3–6 | MODEL (one seeded run; the view codes its fields DERIVED, the engine is stochastic, so the page labels it MODEL per twin-evidence) |
| custody | a community room's holders: filled = reachable, struck = not; state, n_eff, spread | buffers 7–8 | DERIVED (MODEL reachability) |
| reach | street homes by router class: disc public, ring punchable NAT, hatched square hard NAT/CGNAT | micro buffer 6 | MODEL (micro.rs PARAM_SPECS) |
| seat | ρ CPU and ρ permits against fleet size; this street's seat marked | buffer 9, `cap_seat` | as capacity.rs |

Phase 2 views come through one door, `kernel-wasm/src/views.rs`: `view_topology(seed)`,
`view_topology_path(src, dst)`, `view_nat(seed)`, `view_people(grid_ptr, len, seed)`, `view_holon(seed)`
each call the package's own `*_view_*` functions once and park the vectors in slots read with
`view_ptr(slot)` / `view_len(slot)` / `view_is_u32(slot)`. The door computes nothing of its own; its native
test checks the slots equal the package functions' output (topology seed 7 on the GHS-POP 2020 grid: 1,622 points, 6,161 links).

| id | draws | slots | class |
|---|---|---|---|
| topology | generated AS / IXP / cable graph over the 181 real localities; glyph per role (dot fixed access, triangle mobile, square transit, diamond content, large diamond tier-1, plus IXP, ring metro); provider-customer, terrestrial (solid) and submarine (dashed) by default, peering and IXP membership on a toggle; one route between fixed access ASes with AS hops and long-haul km | 0, 1, 2, 15 | MODEL (all fields, `view_classes`) |
| nat | a generated street (metres, no place) in a dock panel: devices by access class (square, disc, triangle), behaviour by mark (white centre public, ring behind CGN), pairs by outcome (solid direct, dotted punched, dashed relayed, dash-dot after a failed punch) | 3–6 | MODEL |
| people | localities: ring = population weight, disc = week multiplier at the sim's hour of week; 40 strongest cross-locality ties dotted; a live event as an expanding ring with its onset multiplier | 7–10 | MODEL (FRONTIER event inputs) |
| holon | the R5 tier's cells on the generated street patch: certified filled, refused crossed, not evaluated dashed; a marked square with the counts while the patch is under 5 px. Certified means the coarse tier held within budget over the twin's MODEL world, not the real mesh | 11–14, 16 | DERIVED (MODEL world) |
| calibration | dock panel: one v45.0.0 seat's ρ outer band (500 to 100,000 devices, log axis) with the central model line dotted, the ρ 0.70 ceiling, the knee as a lower-to-upper bracket only (no central tick), and predicted / observed per seat-1 counter (filled square in-sample, ring held out); a slider picks key_publish_set growth g = 0, 0.25, 0.5, 0.75, 1 (FRONTIER) | 17 curve, 18 knee, 19 residuals (`view_calibration_curve(g)`, `view_calibration_knee(g)`, `view_calibration_residuals()`) | MODEL, interval (twin-calibration c13cee5 class field 2) |
| media | twin-media's churn room (4 homes, 6 phones, a home relay leaving with a signed end at 30 s), one second at a time: members at a generated scatter of about a degree around one metro anchor (square home, disc phone; hollow while out of the call; a ring where a parent left this second); one line per (parent, child) pair with its streams summed, solid when both video layers flow, dashed for audio + base, width by wire rate, a cross where copies were lost. Dock panel: one row per receiver over the 60 s, coloured by the lowest layer it was shown that second across the streams it receives (both layers, base, audio only, nothing), with a dark bar where any of them showed no video frame, departures as ticks, a cursor at the shown second. Controls: second slider, play (400 ms a second), zoom to the call, jump to the relay leaving. While the scatter is under 24 px tall the layer draws a dashed box with a label instead | 20 members, 21 tree, 22 edges per step, 23 rx per step, 24 events, 25 dims (`view_media(1)`, the scenario run once) | MODEL (one seeded run; the view codes its fields DERIVED, the engine is stochastic, so the page labels it MODEL per twin-evidence) |

People and holon run their scenario after the click (one shot, status says so): the week takes about
1.6 s and the tier demonstration about 4 s of main-thread time in Chrome on the test box (measured once,
not a budget). Hour of week = sim hours since 00:00 UTC of day 314 + 24, because day 314 of 2026 is a
Tuesday and the hook's hour 0 is Monday 00:00 UTC.

The seat card's knee line reads `view_calibration_knee(0)` and `(4)` and prints both intervals in one
sentence that names what it hinges on: 4,882 to 13,723 devices if the server's publish set grows with the
fleet (g 1, binds on CPU), 14,253 to 75,879 if it stays at today's size (g 0, binds on sweep permits at the
low end, CPU at the high end), MODEL with its interval (outer bounds over the fit's input intervals, not a
confidence level), and that which of the two holds is not known (FRONTIER). The native test asserts those
four numbers. cap_seat's single-point knee, the v44 and whole-plane knees and the single lock knee are no
longer printed (calibration covers v45.0.0 only).

Failures (twin-failures 89b1e4c, merged at 17a9a87) comes through the same door: `view_failures(s)` calls
`cewp_sim::failure_view::failure_view(s)` once per scenario (the page keeps each copy) and fills slots 26 to 31.
`failure_view.rs` adds no failure logic: it builds one of three named events on `DomainWorld::build(2026)` with
2,000 nodes, evaluates `failure::evaluate` at 43 hours (two before onset, 41 from onset to 1.25 × the duration, the
same hours as `view_event_steps`), and places the generated world for display. Its native test checks every
post-onset step against `view_event_steps` and `view_event_components`.

| id | draws | slots | class |
|---|---|---|---|
| failures | the generated world's nodes by state at the chosen hour: disc on the mainland (largest component), diamond cut off (one hue per island, by component label), cross down because of the event, faint dot asleep (not up with nothing failed), square withholding; a soft red / amber glow under what went dark or was cut off; one label at the event with the hour and the counts. Dock panel: stacked mainland / cut off / down against hours, onset dashed, cursor at the hour. Controls: hour slider, play (320 ms an hour step), next scenario (national shutdown of the second most populous generated country for shutdown.median_h 48 h with cellular; the busiest fixed ISP of the most populous for isp.median_h 4 h; view_demo's 12 h regional power failure), zoom to the outage | 26 meta (10), 27 nodes (lat, lon, kind, country), 28 metros (lat, lon, country, nodes), 29 steps (hour, up, mainland, cut off, components, down by the event, withholding), 30 state u32, 31 component u32 | MODEL |

Placement (display only): the failure world's countries and metros are indices with no coordinates. Generated
country c is drawn in page region (c + 6) mod 9; its anchor is that region's heaviest free locality and its other metros are spread across the
region (each the free locality farthest from those already placed); nodes sit on a sunflower spiral of 0.09° × √k around their metro. Lines join each country's metros to its anchor and, for every international link that lands in two generated countries (slot 33), the two anchors; a link is drawn red and dashed while either end has under a fifth of its nodes on the mainland. A generated
country therefore covers several real ones and is none of them, and the page says so on the status line.

The page's media figures are sums, counts and medians over the slices (legs attached, wire rate, sessions with no video frame, median resume gaps); nothing else is computed page-side, and they stay MODEL: one seeded run, so a median is over that run's sessions, not a rate. People, holon and media run their scenario once after the click.

Build: `sh kernel-wasm/build.sh` runs `cargo build --profile wasm --target wasm32-unknown-unknown`. `[profile.wasm]` sits at the workspace root (inherits release; opt-level 3, lto, codegen-units 1, panic abort), because a member crate's own `[profile.release]` is ignored inside a workspace. `kernel-wasm/Cargo.toml` carried such a block from f73b918 on, so earlier binaries were built without it. The workspace `[profile.release]` is untouched, so the native CLI keeps unwinding panics. Size: 1,034,318 bytes before, 1,005,909 after. The media view's digest is unchanged, and `view_media(1)` in node took 222 to 235 ms with both binaries (three runs each, measured once).

Cross-platform check (done): `views_media_bits_match_the_wasm_build` asserts that an FNV-1a digest over the bit patterns of slots 20 to 25 after a native `view_media(1)` equals the digest the built `kernel_scale.wasm` gave in node (0x7175cd5dd795aa63; 10 members, 60 steps, 172 tree edges). The two platforms draw the same call bit for bit. In node the wasm run took 234 ms (measured once, not a budget).
twin-media's basis (a source read): integer µs/bytes engine, exact `Rng::f`, no hash-order
iteration; the one transcendental (`f64::ln` in `BackgroundModel::next_gap_us`) runs only with household
contention on, which the view scenario leaves off. A view with contention on may differ by an ulp in wasm.

## What the globe cannot say

* The world the tick runs on is built in JavaScript (`buildWorld`) from the population grid. The
  kernel computes everything drawn from it, but the node positions themselves are a page-side MODEL
  input.
* Flows are counts of model deliveries, not traffic measurements.
* The relay tree is alm.rs's real algorithm on MODEL inputs. Its shape says what the algorithm
  does, not what a deployed call looks like.
* Custody reachability is a uniform per-holder draw; the failures layer shows correlated outages on its own generated world, which is not the custody layer's world.
* The failures layer is one event at its median duration on a generated world, connectivity only (no capacity loss on surviving links).
* The seat curve layer is cap_seat's capacity model at the street chip's setting and marks no knee. The
  knee exists only as an interval (calibration layer, seat card), and it hinges on g, which nothing
  measures: across g 0..1 it spans 4,882 to 75,879 devices.
* The topology is an invented graph on real cities: no AS, IXP or cable there exists. Route lengths are
  the model's, not the Internet's.
* The NAT street, the people's week and the holon cells are generated scenarios run once with a fixed
  seed; they show what the model does, not what a mesh measured.
* Between 30 and 26 m a pixel neither real data nor generated streets show, only the holon
  mosaic. This follows the page's rule that real and generated layers never share a scale.

## Where streets are built (population-conditioned street tier)

Eric, on the live page: neighbourhoods in the middle of the Urals. Cause: the tiling's root budget came
from the nearest 1.5° grid cell within 1.6° (`tlCellNear`), and each child's weight was its land
samples × (40 + a populated-place kernel), so every land holon got a share of its 1.5° cell's people
however empty it was. A place fallback added a city's Natural Earth population where the old grid had
no cell near it.

Now (crates/cewp-sim/src/popchain.rs, called by the page through kernel-wasm `pop_load`, `pop_root`,
`pop_split`, `pop_of` from docs/tiling.js tlRoot / tlSplit; MODEL rules over GHS-POP):
* One total, two cuts. The cohort tier keeps the 4,559 cells of 1.5° holding at least 20,000 people
  (7,830,639,988 people). The street tier takes every 1/8° cell at or above 1 person per km² of cell
  area (`popchain::FLOOR_KM2`, MODEL): 355,434 cells holding 7,834,012,940 people, each rounded to
  whole people, then apportions the cohort total over them, so both tiers hold 7,830,639,988 people
  and nobody is dropped or counted twice. Under the floor (forest, bare ground, snow and ice,
  wetland, water) a cell holds nobody: no streets, no devices; the focal chip says "No one lives
  here in the model" with the people GHS-POP reads there, its land-cover class and the floor.
* People are apportioned at every split (largest remainder, ties to the lower index, zero weight
  gets zero: the rule of CIRISHolon's `apportion.rs`), with the children's overlap of each 1/8° cell
  as weights. Households are round(people / 2.5) for the world, apportioned over the L0 holons by
  their people and at every split by the children's people, so both sum exactly at every level.
* At the leaf, every dwelling draws a size round(2.5 + 1.1 N) clamped 1..6 on its own stream; the
  leaf's people are then apportioned over the dwellings with those sizes as weights
  (`popchain::persons_per_household`), so the leaf's persons are exactly its people. Devices per
  person are drawn (MODEL); the leaf's total, adopting or not, is its **device population**
  (`MicroWorld::device_population`, per household in `household_devices`).
* **The 1,000-node cap is a display sample.** The focal world draws at most `R5_MAX_NODES` = 1,000
  nodes (adopting households' devices in building order, plus the seat, hosted servers and 6 CI
  slots); it never changes the people, households or device population the holon reports, which the
  chip states beside the sample.
* Density class follows the tile's people per km² through `micro.rs` thresholds (docs/geo/classes.json).
* Inside a 1/8° cell people are spread evenly by area: real data stops at that cell (in-cell
  land-cover weighting needs a fractional built-up layer; the shipped LC100 asset has one class per
  cell).

Check (`node --test docs/geo/popcond.test.js`, the first instance of CIRISHolon#3's conditional
consistency): at 27 places (the ten probe places and 17 dense ones), every split from the 1.5° root
to the street tile sums exactly in people and households; each holon's people are its GHS overlap
scaled to the cohort total within the stated rounding bound (cross-checked against popcond.js in
JavaScript); at the leaf, persons sum to people and household devices to the device population; the
empty places hold nobody. The test writes CIRISHolon's sums format (one line per holon and kind,
seed 0: the twin's tree takes no seed) to /tmp/cewp-popchain-sums.jsonl, which
`cargo run -p holon-scene --example check_sums` accepts; a sample is docs/geo/sums.sample.jsonl.

Probe (headless, both widths, street and block zoom; devices = the focal patch's devices including
its off-patch seat and servers):

| place | GHS-POP 1/8° cell | street tile households | devices | class |
|---|---|---|---|---|
| mid-Urals 60.0 N 59.5 E | 1.2 people, 0.013/km² | 0 | 0 | none: "no one lives here" |
| central Sahara 23.5 N 12.0 E | 0 | 0 | 0 | none |
| Greenland interior 72 N 40 W | 0 | 0 | 0 | none |
| Amazon interior 3.5 S 67.5 W | 0 | 0 | 0 | none |
| open Atlantic 0 N 30 W | 0 | 0 | 0 | none |
| Zurich | 2,484/km² | 52 | 166-167 | terraced streets |
| Cairo | 37,082/km² | 16 | 43-44 | apartment blocks |
| Lagos | 5,318/km² | 40 | 133-134 | terraced streets |
| Maryland suburb 39.2 N 77.25 W | 1,114/km² | 27 | 75-76 | suburban streets |
| rural Cher village 46.3 N 2.3 E | 13/km² | 18 | 45 | village |

The cohort tier and the night glow draw the 1.5° grid at its own grain (cohorts scattered inside
their cell), which is the coarse tier, not generated streets.

## The dense extreme (Eric: "the most populated places on earth")

What the data says: the densest 1/8° cells in GHS-POP 2020 by people per km² of land (land share from
the Natural Earth 50 m mask, 10 × 10 samples a cell) are Karachi (24.94 N 67.06 E, 41,964), Karachi's
coastal cell (41,448), two Mumbai cells (40,098 and 37,228), Giza/Cairo (37,082), Dhaka (34,850),
Mumbai (33,976), Kowloon (33,867), Delhi (32,344), Shanghai (30,818), Bengaluru, Cairo, upper
Manhattan, Surat, Wuhan, Ho Chi Minh City, Shanghai, Lahore, Delhi and Istanbul (25,150). Nothing at
1/8° exceeds 42,000 per km²: the cell average hides wards of 100,000 and more.

What the street tier does there (MODEL):
* Above 12,000 per km² (`DENSE_FABRIC_KM2`) an apartment-block leaf is dense fabric: 4 m lanes,
  plots filling the leaf, the leaf's households shared over the buildings, storeys = flats per
  building / flats per floor (at least 3). The leaf still holds at most 60 households (the tiling's
  target), so at 40,000 per km² a leaf is about 60 m across; the block zoom shows hundreds of such
  leaves side by side, which is where the density reads.
* Below it, 6,000 to 12,000 per km² keep apartment blocks of 4 to 7 storeys on street frontage.
* No trees or garden lots at 1,500 per km² and above; shadows follow storeys.
* The chip names dense fabric, its mean storeys, and that the density is a 1/8° cell average with
  denser wards inside.

What it does not capture:
* Towers. Storeys follow households per footprint at the cell average, so Kowloon and Manhattan draw
  as dense mid-rise, not 40-storey towers; no building-height data is approved.
* Wards denser than the cell (Cairo's and Dhaka's exceed 100,000 per km²).
* Household size. 2.5 persons a household everywhere (micro.rs PersonsPerHousehold, MODEL): too few
  for Dhaka, Karachi and Lagos and too many for Tokyo and Paris (the direction is general knowledge,
  not read from a dataset here). Households,
  devices per person and devices per seat are off by that ratio there. No approved dataset carries
  household size. Approval request: UN DESA Population Division, "Database on Household Size and
  Composition 2022" (https://www.un.org/development/desa/pd/data/household-size-and-composition),
  mean household size by country from censuses and surveys; licence and size to be read from its
  page before approval; it would replace the single 2.5 with a per-country value at the
  locality's country. Not fetched.

Probe (headless, street zoom 0.6 m/px and block zoom 4 m/px, both widths, a fresh page per place;
devices are the focal patch's own plus its off-patch seat and servers; households, devices and
classes are identical at 1440 and 390; frame time at 390 × 844 without CPU throttle):

| place | GHS-POP 1/8° cell, people / per km² | class drawn | patch | households | devices (phone/home/laptop/hosted/seat) | buildings, mean storeys | share needing a relay | devices rounding with the seat | tile sums | frame p50/p95 ms, 390, street / block |
|---|---|---|---|---|---|---|---|---|---|---|
| dhaka | 6,165,561 / 34,850 | dense fabric | 35 m | 15 | 36 (21/2/10/2/1) | 1, 8 | 0.82 | 24 | holds | [16.7, 16.8] / [16.7, 33.4] |
| manila | 1,369,421 / 7,324 | apartment blocks | 139 m | 55 | 150 (80/20/41/8/1) | 10, 5.7 | 0.59 | 104 | holds | [16.7, 16.7] / [16.7, 16.7] |
| mumbai | 3,288,120 / 18,007 | dense fabric | 70 m | 33 | 96 (50/15/27/3/1) | 7, 3.6 | 0.68 | 74 | holds | [16.7, 16.8] / [16.7, 33.3] |
| kolkata | 4,374,412 / 24,519 | dense fabric | 70 m | 44 | 135 (69/14/37/14/1) | 17, 3 | 0.63 | 104 | holds | [16.7, 16.8] / [16.7, 33.4] |
| karachi | 4,433,619 / 25,283 | dense fabric | 70 m | 44 | 147 (80/18/42/6/1) | 15, 3 | 0.62 | 113 | holds | [16.7, 16.8] / [16.7, 16.8] |
| delhi | 4,413,550 / 26,042 | dense fabric | 70 m | 44 | 130 (67/17/36/9/1) | 15, 3 | 0.6 | 96 | holds | [16.7, 16.7] / [16.7, 33.3] |
| kowloon | 3,813,322 / 21,336 | dense fabric | 70 m | 38 | 129 (68/18/34/8/1) | 13, 3 | 0.78 | 94 | holds | [16.7, 16.8] / [16.7, 16.8] |
| macau | 1,044,436 / 5,839 | terraced streets | 139 m | 42 | 98 (50/16/24/7/1) | 23, 2.6 | 0.53 | 74 | holds | [16.7, 16.7] / [16.7, 16.8] |
| lagos-island | 405,015 / 2,110 | none: not on the 10 m land | - | 16 | 0 (0/0/0/0/0) | -, - | - | - | holds | [16.7, 16.7] / [16.7, 16.7] |
| cairo | 6,200,206 / 37,082 | dense fabric | 35 m | 16 | 43 (25/3/13/1/1) | 3, 3 | 0.46 | 36 | holds | [16.7, 16.8] / [16.7, 16.8] |
| tokyo | 2,626,666 / 16,740 | dense fabric | 70 m | 26 | 103 (56/8/30/7/1) | 16, 3 | 0.56 | 74 | holds | [16.7, 16.7] / [16.7, 16.8] |
| paris | 1,830,895 / 14,391 | dense fabric | 70 m | 18 | 35 (17/9/5/3/1) | 15, 3 | 0.68 | 32 | holds | [16.7, 16.8] / [16.7, 33.3] |
| manhattan | 2,271,707 / 15,536 | dense fabric | 70 m | 23 | 77 (41/11/17/7/1) | 19, 3 | 0.88 | 61 | holds | [16.7, 16.8] / [16.7, 33.3] |
| top01-karachi | 7,351,335 / 41,964 | dense fabric | 35 m | 19 | 80 (44/7/22/5/1) | 4, 3 | 0.59 | 59 | holds | [16.7, 16.7] / [16.7, 16.7] |
| top02-karachi | 4,433,619 / 25,283 | dense fabric | 70 m | 45 | 180 (95/21/48/15/1) | 12, 3.3 | 0.5 | 137 | holds | [16.7, 16.7] / [16.7, 33.3] |
| top03-mumbai | 4,389,730 / 24,059 | dense fabric | 70 m | 44 | 146 (75/20/36/14/1) | 10, 3.4 | 0.6 | 110 | holds | [16.7, 16.7] / [16.7, 33.3] |
| top04-mumbai | 3,058,969 / 16,752 | none: not on the 10 m land | - | 31 | 0 (0/0/0/0/0) | -, - | - | - | holds | [16.7, 16.7] / [16.7, 16.7] |
| top05-giza | 6,200,206 / 37,082 | dense fabric | 35 m | 16 | 34 (15/5/11/2/1) | 3, 4.3 | 0.84 | 22 | holds | [16.7, 16.8] / [16.7, 33.3] |
| top06-dhaka | 6,165,561 / 34,850 | dense fabric | 35 m | 16 | 45 (25/3/13/3/1) | 1, 8 | 0.78 | 31 | holds | [16.7, 16.7] / [16.7, 33.4] |
| top07-mumbai | 3,288,120 / 18,007 | dense fabric | 70 m | 33 | 116 (62/16/27/10/1) | 23, 3 | 0.44 | 85 | holds | [16.7, 16.8] / [16.7, 16.8] |
| top08-hong-kong | 3,813,322 / 21,336 | dense fabric | 70 m | 39 | 108 (58/14/27/8/1) | 4, 5 | 0.68 | 76 | holds | [16.7, 16.8] / [16.7, 16.7] |
| top09-delhi | 5,481,580 / 32,344 | dense fabric | 70 m | 55 | 175 (93/20/47/14/1) | 10, 3.8 | 0.68 | 124 | holds | [16.7, 16.7] / [16.7, 33.3] |
| top10-shanghai | 5,093,367 / 30,818 | dense fabric | 70 m | 51 | 134 (73/15/38/7/1) | 12, 3.1 | 0.6 | 99 | holds | [16.7, 16.8] / [16.7, 33.3] |
| top11-bengaluru | 5,349,957 / 28,428 | dense fabric | 70 m | 54 | 140 (76/17/41/5/1) | 17, 3.1 | 0.57 | 102 | holds | [16.7, 16.7] / [16.7, 33.3] |
| top12-cairo | 4,644,254 / 27,776 | dense fabric | 70 m | 47 | 95 (50/13/24/7/1) | 13, 3.2 | 0.56 | 63 | holds | [16.7, 16.7] / [16.7, 16.7] |
| top13-new-york | 2,271,707 / 15,536 | dense fabric | 70 m | 23 | 50 (30/5/13/1/1) | 17, 3 | 0.54 | 34 | holds | [16.7, 16.7] / [16.7, 16.7] |
| top14-surat | 4,879,417 / 27,088 | dense fabric | 70 m | 49 | 134 (79/18/30/6/1) | 20, 3 | 0.6 | 100 | holds | [16.7, 16.7] / [16.7, 33.4] |
| top15-wuhan | 3,526,333 / 21,198 | dense fabric | 70 m | 36 | 101 (56/11/24/9/1) | 10, 3.1 | 0.71 | 69 | holds | [16.7, 16.8] / [16.7, 33.3] |
| top16-ho-chi-minh-city | 5,069,351 / 26,714 | dense fabric | 70 m | 51 | 152 (75/20/44/12/1) | 11, 3.2 | 0.78 | 112 | holds | [16.7, 16.7] / [16.7, 33.4] |
| top17-shanghai | 4,408,185 / 26,708 | dense fabric | 70 m | 45 | 105 (52/17/30/5/1) | 21, 3 | 0.64 | 76 | holds | [16.7, 16.8] / [16.7, 33.4] |
| top18-lahore | 4,319,303 / 26,239 | dense fabric | 70 m | 43 | 100 (57/13/24/5/1) | 9, 3.2 | 0.68 | 74 | holds | [16.7, 16.7] / [16.7, 33.3] |
| top19-delhi | 4,413,550 / 26,042 | dense fabric | 70 m | 44 | 81 (46/9/18/7/1) | 13, 3.1 | 0.77 | 60 | holds | [16.7, 16.7] / [16.7, 33.3] |
| top20-istanbul | 3,260,521 / 22,384 | dense fabric | 70 m | 33 | 91 (48/12/25/5/1) | 17, 3 | 0.73 | 68 | holds | [16.7, 16.7] / [16.7, 33.4] |

The densest cell (Karachi, 41,964 per km²) at 390: frame p50 16.7 ms street and block; JS heap
121 MB; wasm memory 31.9 MB. The tile-sums-to-parent check holds at every probed place (|b - E| <=
level + 1 from the 1.5° root to the street tile). Mesh readouts at this density: a seat serves 22 to
137 of the patch's devices, and 44 to 88 % of devices need a relay (hard NAT shares from micro.rs,
MODEL); both are per patch of at most 60 households, so the seat's load at a city's density is the
seat card's fleet rows, not this patch's.

Lagos Island and Mumbai's eastern cell (19.06 N 72.94 E) still show no street: the Natural Earth
10 m coast puts that point in the water (lagoon and creek), and the street tier is built only on land.

## The Zurich path without loading (Eric: "pre-cache all the Zurich tiles")

`docs/preload.js`, at idle after first paint, asks once for what the Story's and the default zoom's
path to Zurich draws, through the page's own loaders so everything is decoded and on the GPU: the 50 m
coast, lakes and places, GHS-POP and land cover at 1/8°, relief levels 1 and 2 over the Zurich
corridor (1.6 × the view's half-width at level 2, because the camera is still settling when it starts
to draw) and the ne10 tiles geoSets will ask for from 2,400 m a pixel. Then, in idle slices under
8 ms, it builds the Zurich street tiling (leaf layouts and arterials within 0.018° of the centre;
240 leaves, about 13 MB) and runs the Story's street search (the leaf whose two homes a punch can
join) one candidate world per idle callback, dropping the world afterwards. The Story now finishes
turning to Zurich before it dives (no zoom in while more than about 3° off), so the dive comes
straight down and needs only the Zurich tiles. `docs/sw.js` serves versioned and geo/kernel files
from its cache on a return visit (return visit measured: 0 network responses, 78 from the service
worker). A precomputed Zurich asset (option c) was not needed.

Measured from first load on a cold cache, 10 Mbit/s and 40 ms, following the Story (times from the
start of the dive):

| | region tier complete | city tiling complete | street complete | requests during the dive | long tasks during the dive |
|---|---|---|---|---|---|
| 1440, before | +3.25 s (relief and coast still loading) | +1.5 s | +3.25 s | 43 (3.05 MB) | 1 (70 ms) |
| 1440, after | +1.5 s (on arrival) | +1.75 s | +3.0 s (street reached at +2.75 s) | 0 | 1 (68 ms) |
| 390 with 4x CPU, before | +2.68 s | +1.95 s | +3.68 s | 25 (1.83 MB) | 7 (393 ms) |
| 390 with 4x CPU, after | +1.5 s (on arrival) | +1.78 s | +3.0 s (on arrival) | 0 | 2 (106 ms) |

What remains: during the half second the camera passes block zoom (15 to 4 m a pixel), the leaves of
that wider view (several hundred to a few thousand) are laid out at 5 ms a frame; until each is built
its holon shows the density mosaic, not a blank. Bytes: the preload moves about 2.3 MB (390) to 4.3 MB
(1440) ahead of the dive, all but 0.35 MB (390) and 1.22 MB (1440) of which the path loaded anyway.

## Street classes, the floor and the region rule (one table; machine-readable: docs/geo/classes.json)

Density is people per km² of a street leaf: its households × 2.5 / its area, its households being its
share of GHS-POP 2020 (popcond.js). All MODEL.

| class | from (people per km²) | what it generates |
|---|---|---|
| (none) | below the floor | a 1/8° GHS-POP cell under 1 person per km² of cell area contributes no people: no households, streets or devices |
| village | 0 | houses of 1-2 storeys, a fifth of plots empty, 5 m streets, 14-50 m apart, yards and trees |
| suburban streets | 250 | detached 1-2 storeys, 6 m streets, 9-15 m gaps, yards and trees |
| terraced streets | 1,500 | terraces of 2-3 storeys, 8 m streets; no yards |
| apartment blocks | 6,000 | 4-7 storeys on street frontage, 8 m streets; no trees |
| dense fabric | 12,000 | 4 m lanes, plots filling the leaf, storeys from flats (at least 3); no trees |

Region rule: a 1/8° cell belongs to the region of the 1.5° GHS-POP grid cell holding it; each grid cell
belongs to one of 181 localities and each locality to one of 9 regions, a partition inherited from the
old grid by nearest old cell (MODEL).
