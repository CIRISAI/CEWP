# holon-draw

`holon_draw.wasm` is the street leaf builder of CIRISHolon's `holon-draw` crate
(`engine/crates/holon-draw`, AGPL-3.0, as this repository), built for the browser with no imports and plain
`extern "C"` exports. The twin hands it a leaf's layout buffers from its own kernel and the look of a leaf as one
array (`TL_STYLE` in `docs/tiling.js`), and reads back the leaf's vertex stream. It is the same stream as the
twin's JS builder (`tlBuildLeaf`), float for float; the JS builder stays, as the fallback and as the comparator.

| | |
|---|---|
| source | CIRISHolon commit `842fe76`, `engine/crates/holon-draw` |
| build | `sh engine/crates/holon-draw/build-web.sh <outdir>` (release, LTO, one codegen unit, `panic=abort`, stripped) |
| `holon_draw.wasm` | 86,077 bytes, sha256 `8acbdf79cc71c3a7e4d7ea7579e8d6eecca720fcc1ea77fc92992b6f9a362842` |
| `draw.js` | the crate's `web/draw.js` at CIRISHolon `05f70d4`, copied unchanged (the same file as at `842fe76` but for one line of its header comment); sha256 `ad1b725ff3f6186752cce248766f09578b3cc88edb78cf66f5e0f60ef79e7b64` |

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
backend still uploads every record every frame, and it is not timed until a tile's records can stay on the GPU.

Checked, both ways:

- `node engine/crates/holon-draw/tools/twin_leaf_check.mjs docs/tiling.js holon_draw.wasm 2000` (CIRISHolon) runs
  this repository's `tlBuildLeaf` over synthetic leaves and compares the two streams bit for bit;
- `tools/page/holon_leaf/run.py` (this repository) builds real leaves from the kernel in the page, both ways, with
  `?hdcheck=1`, and compares them bit for bit.

- `tools/page/holon_leaf/run.py --inst` (this repository) expands each real leaf's records back into the vertex stream
  (`?hdcheck=1&hd=2`) and compares it with holon-draw's own tolerances: kind codes exact; positions 2e-3 plus |sx - 1|
  times the largest record's size; normals and widths 1e-3; colours half an 8-bit step.

`?hd=0` turns the wasm builder off (the JS builder draws).
