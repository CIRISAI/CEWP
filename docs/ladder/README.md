# holon-draw's scale ladder

These files draw past the twin's own band, finer than the street and wider than the globe, once the zoom switch
("zoom: street to globe", in More and Controls) is off. They come from CIRISHolon's `holon-draw-ladder` crate
(`engine/crates/holon-draw-ladder`, AGPL-3.0, as this repository). `globe.js` loads them at idle only once the switch is
off (`zrLadder`), prepares them at idle (`prepare`), and calls `window.HOLON_LADDER.draw(gl, view, slot)` past each edge
(`zrHook`).

| | |
|---|---|
| source | CIRISHolon `e822856` (the adapter's `prepare`) over `fb3a3df` (the rest), `engine/crates/holon-draw-ladder` |
| build | `sh engine/crates/holon-draw-ladder/build-web.sh` (release, LTO, one codegen unit, `panic=abort`, stripped) |
| `holon_draw_ladder.wasm` | 1,236,342 bytes, sha256 `14c3e66fa72cdee293ffe5161243dff0a5b6c25aaba5036cda9e3b3a9ed1c1ec` |
| `holon-ladder.js` | the crate's `web/holon-ladder.js` at `e822856` |
| `draw.js`, `text.js` | holon-draw's `web/draw.js` at `7b698c4` and `web/text.js` at `f206a4f` (unchanged since); `draw.js` is the same file as `../holon/draw.js` |

Every rung says on screen what it is (computed, measured, catalogue, declared, illustration). The twin's own data and
simulation stop at its band's edges, and the pill says so past them.

Checked by `tools/page/zoom_range/run.py --assert` and `tools/page/no_highp/run.py --zoom-off --assert`.
