// The shipped wasm runs CIRISHolon's device generator (DEVICEGEN-1) bit for bit as native Rust does: its own digest over
// 1,000 fixed calls, committed in devicegen_digest.txt and checked natively by the popchain test
// the_device_generator_digest_is_the_committed_one.
"use strict";
const test = require("node:test"), assert = require("node:assert"), fs = require("node:fs"), path = require("node:path");
test("wasm and native give one device-generator digest", async () => {
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(path.join(__dirname, "../../kernel/kernel_scale.wasm")), {});
  const w = instance.exports; w.pop_device_digest(); const p = w.holon_buf_ptr(0) >>> 0, o = new Float64Array(w.memory.buffer, p, 2);
  assert.strictEqual((BigInt(o[1]) << 32n) | BigInt(o[0]), BigInt(fs.readFileSync(path.join(__dirname, "devicegen_digest.txt"), "utf8").trim()));
});
