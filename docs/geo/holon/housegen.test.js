// The shipped wasm runs CIRISHolon's household generator bit for bit as native Rust does: one digest over a fixed
// grid of calls (popchain::households::generator_digest), committed in housegen_digest.txt and checked natively by
// the popchain test the_generator_digest_is_the_committed_one.
"use strict";
const test = require("node:test"), assert = require("node:assert"), fs = require("node:fs"), path = require("node:path");
test("wasm and native give one generator digest", async () => {
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(path.join(__dirname, "../../kernel/kernel_scale.wasm")), {});
  const w = instance.exports; w.pop_generator_digest(); const p = w.holon_buf_ptr(0) >>> 0, o = new Float64Array(w.memory.buffer, p, 2);
  const got = (BigInt(o[1]) << 32n) | BigInt(o[0]), want = BigInt(fs.readFileSync(path.join(__dirname, "housegen_digest.txt"), "utf8").trim());
  assert.strictEqual(got, want);
});
// The Households data map draws pop_household_means: each HOUSEGEN-1 row's people per household. The rows here are
// copied from holon-scene household.rs at the pinned rev 33dc99f (DE_PPM / DE_SIZES, US_PPM / US_SIZES), so a
// changed pin that moves a row fails here until the map's numbers are re-read.
test("the Households map's means are HOUSEGEN-1's rows", async () => {
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(path.join(__dirname, "../../kernel/kernel_scale.wasm")), {});
  const w = instance.exports; assert.strictEqual(w.pop_household_means(), 15);
  const o = Array.from(new Float64Array(w.memory.buffer, w.holon_buf_ptr(0) >>> 0, 15));
  const DE = [[323365, 352902, 152510, 113690, 37753, 19780], [359185, 341645, 142815, 107542, 33115, 15698], [455845, 297434, 117192, 85793, 28513, 15223], [563116, 239882, 95242, 64333, 22981, 14446], [615464, 210298, 86439, 55289, 18435, 14075]];
  const US = [[255238, 377663, 149835, 124017, 57864, 21849, 13534], [285493, 340975, 154633, 128882, 57497, 20756, 11763], [299563, 306221, 158932, 129482, 62651, 25455, 17696], [344038, 283416, 148764, 113994, 60032, 26560, 23197], [381954, 296377, 141412, 97934, 45544, 19845, 16934]];
  const mean = (r, z) => r.reduce((a, x, i) => a + x * z[i], 0) / r.reduce((a, x) => a + x, 0);
  const want = [...DE.map((r) => mean(r, [1, 2, 3, 4, 5, 7])), ...US.map((r) => mean(r, [1, 2, 3, 4, 5, 6, 8])), 0, 250, 1500, 6000, 12000];
  for (let i = 0; i < 15; i++) assert.ok(Math.abs(o[i] - want[i]) < 1e-12, `value ${i}: ${o[i]} vs ${want[i]}`);
});
