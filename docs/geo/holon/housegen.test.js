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
