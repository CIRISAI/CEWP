// Wasm half of the layout crosscheck: the shipped kernel lays out every leaf of layout_xcheck.txt to
// the digest cewp-sim computes natively (crates/cewp-sim/tests/layout_xcheck.rs); zero differences.
"use strict";
const test = require("node:test"), assert = require("node:assert"), fs = require("node:fs"), path = require("node:path");
test("the shipped wasm lays out every crosscheck leaf as native does", async () => {
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(path.join(__dirname, "../kernel/kernel_scale.wasm")), {});
  const w = instance.exports; let n = 0; const bad = [];
  for (const line of fs.readFileSync(path.join(__dirname, "layout_xcheck.txt"), "utf8").split("\n")){
    if (!line || line.startsWith("#")) continue;
    const v = line.split(" ").map(Number), cp = w.holon_ctx_ptr() >>> 0;
    new Float64Array(w.memory.buffer, cp, 33).set(v.slice(1, 34));
    assert.ok(w.holon_layout(v[0]) >= 0);
    const p = w.holon_buf_ptr(4) >>> 0, m = new Float64Array(w.memory.buffer, p, 7);
    if (m[5] !== v[34] || m[6] !== v[35]) bad.push(v[0]);
    n++;
  }
  assert.ok(n >= 2000, `${n} leaves`);
  assert.deepStrictEqual(bad.slice(0, 5), [], `${bad.length} of ${n} leaves differ`);
});
