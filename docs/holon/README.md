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
| `draw.js` | the crate's `web/draw.js` at CIRISHolon `fb3a3df`, copied unchanged; sha256 `c24e92ed95c56a4d09652f757aac351148fbf0cd0e42900dba687a2a83744603` |

Exports used: `hd_in(which, len)` (input buffers: 0 roads, 1 buildings, 2 meta, 3 street ends, 4 roof tones,
5 the style), `hd_style()`, `hd_style_len()` (97), `hd_build_leaf(id, ox, oy, sx, pph, road r, g, b, has_tones)` and
`hd_out()`; for the instance path, `hd_build_leaf_instances` (the same arguments), `hd_out_instances()`,
`hd_inst_groups()`, `hd_inst_group(g, which)` and, for the check, `hd_inst_expand()`.

**The instance path, on trial (`?hd=2`).** Each leaf is also built as 52-byte instance records in eight groups (ground,
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

Checked, both ways:

- `node engine/crates/holon-draw/tools/twin_leaf_check.mjs docs/tiling.js holon_draw.wasm 2000` (CIRISHolon) runs
  this repository's `tlBuildLeaf` over synthetic leaves and compares the two streams bit for bit;
- `tools/page/holon_leaf/run.py` (this repository) builds real leaves from the kernel in the page, both ways, with
  `?hdcheck=1`, and compares them bit for bit.

- `tools/page/holon_leaf/run.py --inst` (this repository) expands each real leaf's records back into the vertex stream
  (`?hdcheck=1&hd=2`) and compares it with holon-draw's own tolerances: kind codes exact; positions 2e-3 plus |sx - 1|
  times the largest record's size; normals and widths 1e-3; colours half an 8-bit step.

`?hd=0` turns the wasm builder off (the JS builder draws).
