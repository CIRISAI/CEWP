# holon-draw

`holon_draw.wasm` is the street leaf builder of CIRISHolon's `holon-draw` crate
(`engine/crates/holon-draw`, AGPL-3.0, as this repository), built for the browser with no imports and plain
`extern "C"` exports. The twin hands it a leaf's layout buffers from its own kernel and the look of a leaf as one
array (`TL_STYLE` in `docs/tiling.js`), and reads back the leaf's vertex stream. It is the same stream as the
twin's JS builder (`tlBuildLeaf`), float for float; the JS builder stays, as the fallback and as the comparator.

| | |
|---|---|
| source | CIRISHolon commit `fb3a3df`, `engine/crates/holon-draw` (resident buffers) |
| build | `sh engine/crates/holon-draw/build-web.sh <outdir>` (release, LTO, one codegen unit, `panic=abort`, stripped) |
| `holon_draw.wasm` | 83,912 bytes, sha256 `2d69cf58441092044683ad32580dec2bdd1cc530e52999159e9ddce0cb400e47` |
| `draw.js` | the crate's `web/draw.js` at CIRISHolon `7b698c4` (branch `holon-draw/twin-24`: kind 13 TRIANGLE, GLOBE, BLEND 'multiply', and the eye's CORRECTION), copied unchanged; sha256 `dd0aa36334b0d8488d822022eda984097ba137c576c4b98c423c164e1edde93f`. CORRECTION 2026-11-23: until this commit the row said `fb3a3df` (sha256 `c24e92ed...`), but the file here was already `f206a4f`'s (sha256 `f2bc0c7e...`), copied in for the matrix per batch; the row was not updated then |

Exports used: `hd_in(which, len)` (input buffers: 0 roads, 1 buildings, 2 meta, 3 street ends, 4 roof tones,
5 the style), `hd_style()`, `hd_style_len()` (97), `hd_build_leaf(id, ox, oy, sx, pph, road r, g, b, has_tones)` and
`hd_out()`; for the instance path, `hd_build_leaf_instances` (the same arguments), `hd_out_instances()`,
`hd_inst_groups()`, `hd_inst_group(g, which)` and, for the check, `hd_inst_expand()`.

**The instance path (`?hd=2`; on trial until 2026-11-23, the default since).** Each leaf is also built as 52-byte instance records in eight groups (ground,
mottling and trees, yards, garden trees, casings, streets, shadows, roofs), concatenated per group across a tile's leaves,
and drawn by `draw.js` into the page's own WebGL context between the tiling's passes, with the tile's settings (night, the
terrain's two ground colours and roof factor, the field's origin and switch, least widths, alpha by surface class, the
ring's fade) taken from the same values the tiling's own program takes, and the land stencil left as the page set it. It
differs from the default on purpose in four ways: discs are circles, not hexagons; within a tile all yards come before all
garden trees and all shadows before all roofs; colours are 8-bit; the ring's fade is per pixel. It is not the default: the
backend uploaded every record every frame.

**Now resident, and timed (2026-11-22).** A tile's eight groups are held in one GPU buffer (`draw.js`'s `resident()`),
written when the tile's leaves change, drawn every frame as eight ranges of it with no upload, and released with the tile.
`tools/page/hd_timing/run.py --assert` times the street tier both ways at 1280 x 800 on the machine's GPU (Chrome through
EGL, ANGLE on Mesa Intel RPL-S), unthrottled. Its stakes were written before the first run: at every stop, `?hd=2`'s median
frame at most 1.10 x `?hd=1`'s, and its uploads no more than `?hd=1`'s. **It FAILS, so `?hd=2` stays opt-in.**

| stop | `?hd=1` median / p95 ms | `?hd=2` median / p95 ms | ratio |
|---|---|---|---|
| dense, 3 m a pixel | 16.2 / 49.1 | 15.5 / 46.1 | 0.957 |
| dense, 10 | 27.0 / 44.4 | 25.5 / 43.6 | 0.944 |
| suburban, 3 | 5.8 / 22.4 | 5.5 / 21.3 | 0.948 |
| suburban, 10 | 7.1 / 23.5 | 7.3 / 41.7 | 1.028 |
| village, 3 | 4.1 / 14.1 | 4.7 / 21.5 | **1.146** |
| village, 10 | 3.8 / 14.3 | 5.0 / 21.5 | **1.316** |

- Where the street is dense, holon-draw's path is 4 to 6 % faster.
- Where it is sparse (a village: many tiles of few leaves), it is 0.6 to 1.2 ms slower. The page makes one `draw()` call
  per tile, each with its own matrix and full state setup, so the per-call cost is what shows. The fix is holon-draw's to
  make: many tiles in one call, with a matrix per batch.
- The upload stake also failed, by noise. The resident path adds no upload; the two paths' per-frame bytes are within
  7 % of each other both ways. But the whole page uploads 0.02 to 3.4 MB every frame whatever draws the street, so "no
  more than `?hd=1`" fails on noise. The stake is kept as written. A finding for the twin itself: what re-uploads 3.4 MB a
  frame at a dense street, on either path, is not the leaves.

**One call a frame (2026-11-23).** holon-draw gained a matrix per batch (CIRISHolon `f206a4f`: `batch.vp`,
`batch.pixelsPerUnit`), and `tlHdTile` now queues every tile of a pass for ONE `draw()` call (`tlHdFlush`), each batch with
its own tile's matrix and settings. `hd_timing` again, under the SAME stakes, on a quiet machine (load 3):

| stop | `?hd=1` median / p95 ms | `?hd=2` median / p95 ms | ratio |
|---|---|---|---|
| dense, 3 m a pixel | 11.15 / 46.1 | 11.6 / 62.5 | 1.040 |
| dense, 10 | 11.8 / 46.4 | 12.3 / 46.8 | 1.042 |
| suburban, 3 | 3.3 / 14.6 | 3.3 / 15.5 | 1.000 |
| suburban, 10 | 4.0 / 40.5 | 3.9 / 55.0 | 0.975 |
| village, 3 | 2.8 / 13.9 | 2.9 / 13.4 | 1.036 |
| village, 10 | 2.9 / 14.5 | 2.8 / 36.5 | 0.966 |

- The frame stake (at most 1.10 x at every stop) now HOLDS, the villages included.
- The witness still says FAIL, on the upload stake: `?hd=2` uploads 2, 28 and 29 bytes a frame more than `?hd=1` at three
  stops, out of 20 thousand to 1.9 million. That stake compares the whole page's uploads, which the street does not make;
  the difference is the rounding of bytes over frames. It measures the wrong thing, and it is amended in the next commit,
  alone and before any further run: the amended stake reads holon-draw's own uploads.
- The p95 frame is not better: it is higher at three stops (62.5 against 46.1 at the dense street). The stake is on the
  median and says nothing of it.

**The default (2026-11-23).** Under the amended stake (`tools/page/hd_timing/run.py`, the amendment committed alone before
the run), on a quiet machine (load 2 to 3), two runs:

| stop | ratio, run 1 | draw.js bytes / calls |
|---|---|---|
| dense, 3 m a pixel | 1.081 | 0 / 330 |
| dense, 10 | 1.085 | 0 / 330 |
| suburban, 3 | 1.000 | 0 / 1165 |
| suburban, 10 | 0.974 | 0 / 504 |
| village, 3 | 1.036 | 0 / 1287 |
| village, 10 | 1.036 | 0 / 1090 |

Both runs OK. The dense street is near the stake (1.08 against 1.10), and the stake is on the median: the p95 frame was
higher with holon-draw at two stops. With that said, the street is now drawn by holon-draw unless the address says
otherwise: `?hd=1` is the tiling's own program on the wasm-built stream, `?hd=0` the JS builder. Every page witness was
run on the new default and held: story_zoom, story_planet_arcs, layer_zoom, holon_leaf (both ways), no_highp (both ways),
zoom_range, story_freeze, hd_timing.

Checked, both ways:

- `node engine/crates/holon-draw/tools/twin_leaf_check.mjs docs/tiling.js holon_draw.wasm 2000` (CIRISHolon) runs
  this repository's `tlBuildLeaf` over synthetic leaves and compares the two streams bit for bit;
- `tools/page/holon_leaf/run.py` (this repository) builds real leaves from the kernel in the page, both ways, with
  `?hdcheck=1`, and compares them bit for bit.

- `tools/page/holon_leaf/run.py --inst` (this repository) expands each real leaf's records back into the vertex stream
  (`?hdcheck=1&hd=2`) and compares it with holon-draw's own tolerances: kind codes exact; positions 2e-3 plus |sx - 1|
  times the largest record's size; normals and widths 1e-3; colours half an 8-bit step.

`?hd=0` turns the wasm builder off (the JS builder draws).

**The globe's own passes, one layer at a time (2026-11-23, CEWP#24).** `docs/hdglobe.js` draws four of the globe's passes
through the same `draw.js` (the street tier's drawer when it has one, else its own), each behind `?hdg=`: `fade` (the
zoom hand-off fade) and `veil` (a data map's veil over the land) as one screen quad, their colour exact through DIM;
`lines` (coasts, borders, rivers, ETOPO shores) as world lines of no width of their own, FLOOR giving the width in device
pixels, drawn as tubes, one resident buffer per line set; `land` (the ground and the population tiles) as kind 13 TRIANGLE
records in one resident buffer, inside the land stencil the twin's own fan still writes. Lines and land are lit by the
engine's GLOBE light with the numbers of the twin's own shaders (gfx.js AF, globe.js GF). `?hdg=1` turns all four on,
`?hdg=fade,land` some, `?hdg=0` none; without it, `HDG_DEFAULT`. Each twin program stays and draws whenever its layer is
off. `tools/page/hd_globe/run.py` is the gate (stakes in its header, written before its first run).

**`hd_globe`, run 1 (2026-11-23, under the stakes as first written): FAIL.** Machine load 40 to 47 on 32 cores (other
sessions' work; the same hour the unchanged page's own `zoom_range` failed on long tasks and `hd_timing` on its frame stake).
fade and veil identical at every view (max 0). lines held at every view (on the 3 x 3 box-filtered pictures, mean_abs 0.05
to 0.54, over_32 0 to 0.10 %; unfiltered, max 80 to 180 at the lines' edges). land identical (max 0 or 1) at continent,
region and city, and FAILED at the planet: over_8 0.204 % (stake 0.1), a band of land one to three pixels wide along the
rim that holon-draw left out. The cause was the engine's: the eye it derives from `vp` was wrong for any camera that turns
(CIRISHolon `7b698c4`, a CORRECTION), and the GLOBE limb reads it. S6 FAILED at the planet (ratio 7.0) and was measuring the
wrong thing (frame intervals in bursts, medians below the draw's own CPU time); it is amended in the next commit, alone.

**`hd_globe`, run 2 (2026-11-23, S6 as amended; draw.js at `7b698c4`; load 4 to 6): FAIL, so nothing is made a default.**
Every picture stake held at every view: fade and veil max 0; land max 1 (mean_abs 0, over_8 0); lines on the box-filtered
pictures mean_abs 0.04 to 0.54 and over_32 0 to 0.10 % (raw max 87 to 180, at the lines' antialiased edges). S6 FAILED: the
synchronous frame with the lines and the land on is 1.32 x (planet), 2.57 x (continent), 1.88 x (region) and 1.26 x (city)
the frame with them off. A probe (not a stake) puts the cost on the lines: at 4000 m a pixel the frame is 14.4 ms off,
14.6 with the land on, 36.7 with the lines on; 161,790 segments, each drawn as an 8-sided tube with capped ends (the
engine's line mesh), against the twin's one quad a segment. The land costs nothing measurable. What would answer it is a
line that faces the camera as a flat ribbon of a width in pixels (the "pixel-sized marks" of CEWP#24's list); holon-draw
does not have it yet. Until then all four layers stay behind `?hdg=`, the default (`HDG_DEFAULT`) is empty, and the twin's
own programs draw. Before, after and difference pictures of run 2: not committed (one page load writes 48 images).

**Calibration and the page witnesses (2026-11-23).** `hd_globe --plant` made each layer's stake fail: fade (alpha 0.8) at
all four views, max 17 to 30; veil at all four, max 4 to 15; land (ground and tiles at 0.8) at planet, continent and
region (mean_abs 0.77 to 2.79), NOT at the city view; lines (twice as wide) only at the continent view (box mean_abs 1.50,
over_32 1.82 %), NOT at the other three, so the lines' stake is a weak witness away from the continent view. On this branch
(defaults unchanged but for draw.js at `7b698c4`), every page witness held at load 4 to 7: story_zoom, story_planet_arcs,
layer_zoom, holon_leaf (both ways), no_highp (both ways, and with `--query ?hdg=1`), zoom_range, story_freeze, hd_timing
(ratios 0.69 to 0.95). The unchanged page at `04e3b4b`, run earlier at load 40 to 47, failed zoom_range (18 long tasks)
and hd_timing (ratios up to 2.0): the machine's load, not the page; recorded, not rerun.
