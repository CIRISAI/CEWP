// The shipped wasm runs CIRISHolon's presence generator (PRESENCEGEN-2) bit for bit: its digest equals the one Holon
// published for native and wasm32 (PRESENCEGEN2_RESULTS.md), committed in presencegen_digest.txt and checked natively
// by street_rhythm's test the_presence_digest_is_holons. The street's day (micro_rhythm) keeps every person in every
// hour: the continuity law with source zero holds, which the kernel reports as its violation count.
"use strict";
const test = require("node:test"), assert = require("node:assert"), fs = require("node:fs"), path = require("node:path");
test("wasm gives Holon's presence digest", async () => {
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(path.join(__dirname, "../../kernel/kernel_scale.wasm")), {});
  const w = instance.exports; w.pop_presence_digest(); const o = new Float64Array(w.memory.buffer, w.holon_buf_ptr(0) >>> 0, 2);
  const got = (BigInt(o[1]) << 32n) | BigInt(o[0]), want = BigInt(fs.readFileSync(path.join(__dirname, "presencegen_digest.txt"), "utf8").trim());
  assert.strictEqual(got, want);
});
