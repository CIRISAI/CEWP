# holon-draw

`holon_draw.wasm` is the street leaf builder of CIRISHolon's `holon-draw` crate
(`engine/crates/holon-draw`, AGPL-3.0, as this repository), built for the browser with no imports and plain
`extern "C"` exports. The twin hands it a leaf's layout buffers from its own kernel and the look of a leaf as one
array (`TL_STYLE` in `docs/tiling.js`), and reads back the leaf's vertex stream. It is the same stream as the
twin's JS builder (`tlBuildLeaf`), float for float; the JS builder stays, as the fallback and as the comparator.

| | |
|---|---|
| source | CIRISHolon commit `9fef253`, `engine/crates/holon-draw` |
| build | `sh engine/crates/holon-draw/build-web.sh <outdir>` (release, LTO, one codegen unit, `panic=abort`, stripped) |
| size | 55,975 bytes |
| sha256 | `4b84b422a2d4c43eaa417dfea8ce5583604a0961b9a30722fbf6d42a2769280a` |

Exports used: `hd_in(which, len)` (input buffers: 0 roads, 1 buildings, 2 meta, 3 street ends, 4 roof tones,
5 the style), `hd_style()`, `hd_style_len()` (97), `hd_build_leaf(id, ox, oy, sx, pph, road r, g, b, has_tones)` and
`hd_out()`.

Checked, both ways:

- `node engine/crates/holon-draw/tools/twin_leaf_check.mjs docs/tiling.js holon_draw.wasm 2000` (CIRISHolon) runs
  this repository's `tlBuildLeaf` over synthetic leaves and compares the two streams bit for bit;
- `tools/page/holon_leaf/run.py` (this repository) builds real leaves from the kernel in the page, both ways, with
  `?hdcheck=1`, and compares them bit for bit.

`?hd=0` turns the wasm builder off (the JS builder draws).
