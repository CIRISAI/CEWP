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
* **Who signs when:** with the clock's diurnal switch on (the page turns it on), `tick` keeps a
  drawn signer with probability `diurnal_multiplier(local solar time) / 2.30` and redraws otherwise
  (at most 16 times). `acts` is unchanged; where the acts land follows the hour (workload.rs, FSD
  §4.1, MODEL). With the switch off (the default for any caller that never calls `clock_set`), no
  extra draw is made and every out slot matches the earlier build. Cost: tick p50 18.4 → 18.6 ms
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
| alm | one room's member relay tree: publisher triangle, members by depth, unrooted as crosses | buffers 3–6 | DERIVED (MODEL inputs) |
| custody | a community room's holders: filled = reachable, struck = not; state, n_eff, spread | buffers 7–8 | DERIVED (MODEL reachability) |
| reach | street homes by router class: disc public, ring punchable NAT, hatched square hard NAT/CGNAT | micro buffer 6 | MODEL (micro.rs PARAM_SPECS) |
| seat | ρ CPU and ρ permits against fleet size; this street's seat marked | buffer 9, `cap_seat` | as capacity.rs |

Phase 2 views come through one door, `kernel-wasm/src/views.rs`: `view_topology(seed)`,
`view_topology_path(src, dst)`, `view_nat(seed)`, `view_people(grid_ptr, len, seed)`, `view_holon(seed)`
each call the package's own `*_view_*` functions once and park the vectors in slots read with
`view_ptr(slot)` / `view_len(slot)` / `view_is_u32(slot)`. The door computes nothing of its own; its native
test checks the slots equal the package functions' output (topology seed 7: 1,115 points, 3,732 links).

| id | draws | slots | class |
|---|---|---|---|
| topology | generated AS / IXP / cable graph over the 181 real localities; glyph per role (dot fixed access, triangle mobile, square transit, diamond content, large diamond tier-1, plus IXP, ring metro); provider-customer, terrestrial (solid) and submarine (dashed) by default, peering and IXP membership on a toggle; one route between fixed access ASes with AS hops and long-haul km | 0, 1, 2, 15 | MODEL (all fields, `view_classes`) |
| nat | a generated street (metres, no place) in a dock panel: devices by access class (square, disc, triangle), behaviour by mark (white centre public, ring behind CGN), pairs by outcome (solid direct, dotted punched, dashed relayed, dash-dot after a failed punch) | 3–6 | MODEL |
| people | localities: ring = population weight, disc = week multiplier at the sim's hour of week; 40 strongest cross-locality ties dotted; a live event as an expanding ring with its onset multiplier | 7–10 | MODEL (FRONTIER event inputs) |
| holon | the R5 tier's cells on the generated street patch: certified filled, refused crossed, not evaluated dashed; a marked square with the counts while the patch is under 5 px. Certified means the coarse tier held within budget over the twin's MODEL world, not the real mesh | 11–14, 16 | DERIVED (MODEL world) |
| calibration | dock panel: one v45.0.0 seat's ρ outer band (500 to 100,000 devices, log axis) with the central model line dotted, the ρ 0.70 ceiling, the knee as a lower-to-upper bracket only (no central tick), and predicted / observed per seat-1 counter (filled square in-sample, ring held out); a slider picks key_publish_set growth g = 0, 0.25, 0.5, 0.75, 1 (FRONTIER) | 17 curve, 18 knee, 19 residuals (`view_calibration_curve(g)`, `view_calibration_knee(g)`, `view_calibration_residuals()`) | MODEL, interval (twin-calibration c13cee5 class field 2) |
| media | twin-media's churn room (4 homes, 6 phones, a home relay leaving with a signed end at 30 s), one second at a time: members at a generated scatter of about a degree around one metro anchor (square home, disc phone; hollow while out of the call; a ring where a parent left this second); one line per (parent, child) pair with its streams summed, solid when both video layers flow, dashed for audio + base, width by wire rate, a cross where copies were lost. Dock panel: one row per receiver over the 60 s, coloured by the lowest layer it was shown that second across the streams it receives (both layers, base, audio only, nothing), with a dark bar where any of them showed no video frame, departures as ticks, a cursor at the shown second. Controls: second slider, play (400 ms a second), zoom to the call, jump to the relay leaving. While the scatter is under 24 px tall the layer draws a dashed box with a label instead | 20 members, 21 tree, 22 edges per step, 23 rx per step, 24 events, 25 dims (`view_media(1)`, the scenario run once) | DERIVED (MODEL inputs) |

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

Still hidden: failures (no view functions on any branch when this was wired; its contract stays in `layers.js` and the dock does not list it).

The page's media figures are sums, counts and medians over the slices (legs attached, wire rate, sessions stalled, median resume gaps); nothing else is computed page-side. People, holon and media run their scenario once after the click.

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
* Custody reachability is a uniform per-holder draw until failures supplies correlated outages.
* The seat curve layer is cap_seat's capacity model at the street chip's setting and marks no knee. The
  knee exists only as an interval (calibration layer, seat card), and it hinges on g, which nothing
  measures: across g 0..1 it spans 4,882 to 75,879 devices.
* The topology is an invented graph on real cities: no AS, IXP or cable there exists. Route lengths are
  the model's, not the Internet's.
* The NAT street, the people's week and the holon cells are generated scenarios run once with a fixed
  seed; they show what the model does, not what a mesh measured.
* Between 30 and 26 m a pixel neither real data nor generated streets show, only the holon
  mosaic. This follows the page's rule that real and generated layers never share a scale.
