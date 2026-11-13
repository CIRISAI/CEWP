# Twin package: per-building appearance and the ground texture (the two-layer seam)

A per-building draw (roof material and roof tone, later roof form) and, later, a few kilobytes of learned texture
weights may come from a sister project. None exists. This note fixes the seam they will plug into and the rival a
learned texture must beat.

## 1. Layer one: per-building appearance (a typed integer draw)

- **Where it travels.** The kernel's leaf layout (`crates/cewp-sim/src/tiling.rs` `LeafLayout::tones`) carries one
  roof tone per building, at the building's index; the page reads it from holon buffer 8 (`kernel-wasm` `holon_layout`)
  beside the 8-float building record (cx, cy, w, d, angle, floors, units, street), which is unchanged, so every reader of
  that record and every histogram over it stays byte for byte as it was.
- **Where it comes from.** The table door's built-form generator (`builtform::FormGenerator::draw`, `BuildingDraw`
  `roof_tone`). No generator ships, so every building is `Neutral`.
- **The values.** `builtform::RoofTone`: neutral, red_tile, brown_tile, weathered_wood, grey_slate, dark_shingle,
  metal_light, concrete_flat, with the approximate sRGB CIRISHolon's BUILDGEN-1 prereg records as ASSUMED ("what the
  material usually looks like from above, nothing more"; no verdict but `assumed` may be shown for a roof's colour).
  Neutral is the renderer's present roof colour.
- **Drawing.** The page writes a non-neutral tone into the roof's vertex colours (its lit and shaded halves, as today);
  neutral keeps the present colour. The street panel says "roof colour: the built-in default" until a tone is drawn.
- **Identity with no generator.** Every layout's numbers are unchanged (`layout_xcheck`: 2,000 or more real leaves,
  natively and in the wasm; 600 synthetic leaves at every form class against the previous kernel's digests), every tone
  is neutral (tested), and the page draws a neutral roof exactly as before. A test generator that returns red tile for
  detached houses carries its tone into the record (`tests/door.rs`).

## 2. Layer two: a per-fragment texture over ground, canopy and roof (the contract)

Evaluated in the twin's own WebGL 1 street shader (`docs/tiling.js` `TLF`), per fragment, once a surface is drawn. A
texture is a function of these inputs only:

| input | units, range | where the shader has it |
|---|---|---|
| position | metres, on a world-anchored grid: x = (lon + 180) x 111,320 x cos(the 1.5-degree latitude band's centre), y = (lat + 90) x 111,320, both modulo 4,096 m | `vN`, from a per-tile origin (`tlNoiseOrigin`) and the tile frame |
| metres per pixel | the drawn scale, 1 / `uPpm` | `uPpm` |
| surface kind | ground (kind 0), canopy (1.25: a tree's crown, drawn as kind 1), roof (3) | `vK` |
| mean colour | the land-cover class's ground colour (or the terrain band's), a tree's colour, the roof's colour or tone | the vertex colour |
| density tone | 0 (sparse) to 1 (dense), the leaf's | `vD` (ground only) |
| per-leaf seed, land-cover class, climate class, canopy share | the leaf's seed; LC100 class; APPEARANCE-0 class; a share 0..1 | named for a learned texture; the rival reads none of them beyond the mean colour |

**Valid resolution band.** Street scale: the street tier draws from about 26 m a pixel down; a texture adds detail
only where its wavelength spans at least 2 pixels.

**What the tests mean.**
- *Seamless:* the same world point gives the same value from every leaf and every tile that draws it (the grid is
  world-anchored, not leaf-anchored); the field may step only at a latitude band's edge, every 1.5 degrees. A per-leaf
  seed may only change what lies wholly inside a leaf (a roof), never the ground or canopy field across a leaf's edge.
- *Stable under zoom:* no component whose wavelength is under 2 pixels is drawn, and a component fades in and out
  continuously with scale (no pop), so zooming changes the picture smoothly and a still camera never shimmers.
- *Readable under the maps:* the texture modulates luminance only, by at most about a fifth, halved while a data map is
  shown, so a map's colours keep their order.
- *Free:* evaluated in the existing pass, no texture fetched, no long task, no new buffer.

## 3. The rival (hand-written, procedural), the thing a learned texture must beat

Band-limited value noise on the world-anchored grid, its lattice periodic in 4,096 m so the modulo above is seamless.
Each octave of cell size c metres is weighted 1 - smoothstep(0.25, 0.5, mpp / c): fully on at 4 or more pixels a cell,
gone at 2. The luminance is multiplied by 1 + the sum:

| surface | octaves (cell, amplitude) |
|---|---|
| ground | 48 m 0.10, 12 m 0.07, 3 m 0.05, each times (1 - density tone / 2) |
| canopy | 6 m 0.16, 1.5 m 0.10 |
| roof | 4 m 0.05, 1 m 0.04 |

The mean colour comes from what the twin already holds: the land-cover class's ground colour (or the terrain band's)
for the ground, the tree's own colour for canopy, the roof's colour or tone for roofs. It is off by default, behind the
Advanced switch "ground texture: flat / procedural".
