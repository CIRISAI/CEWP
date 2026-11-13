// The built-form identity set (crates/cewp-sim/src/builtform.rs): synthetic leaves at every form class, laid out by a
// kernel, one line each: packed id, the 33-number leaf context, the layout digest (low, high). The fixture
// crates/cewp-sim/tests/form_identity.txt was written by the kernel BEFORE the per-class numbers moved into the table
// (docs/kernel/kernel_scale.wasm at 6a784c6); crates/cewp-sim/tests/layout_xcheck.rs and docs/geo/layout_xcheck.test.js
// lay the same leaves out with no table loaded and must give every digest. Synthetic inputs only (no real place).
// Run: node tools/holon/form_identity.mjs [kernel.wasm] > crates/cewp-sim/tests/form_identity.txt
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
const root = new URL("../../docs/", import.meta.url).pathname;
const H = createRequire(import.meta.url)(root + "holon.js");
const wasmPath = process.argv[2] || root + "kernel/kernel_scale.wasm";
const { instance } = await WebAssembly.instantiate(readFileSync(wasmPath), {});
const w = instance.exports, mem = w.memory;
w.micro_params_reset();
// splitmix64 on BigInt, seeded: the same set every run
let s = 0x5EEDn; const M = (1n << 64n) - 1n;
const rnd = () => { s = (s + 0x9E3779B97F4A7C15n) & M; let z = s; z = ((z ^ (z >> 30n)) * 0xBF58476D1CE4E5B9n) & M; z = ((z ^ (z >> 27n)) * 0x94D049BB133111EBn) & M; z ^= z >> 31n; return Number(z >> 11n) / 2 ** 53; };
const out = ["# synthetic leaves at every form class: packed id, 33-number context, digest low, digest high (tools/holon/form_identity.mjs)"];
for (let n = 0; n < 600; n++){
  const level = 4 + Math.floor(rnd() * 4), lat = -55 + rnd() * 120, lon = -180 + rnd() * 360, c = H.at(level, lat, lon);
  const b = H.boundsDeg(c), la = (b[0] + b[2]) / 2, km2 = (b[2] - b[0]) * 111.32 * (b[3] - b[1]) * 111.32 * Math.cos(la * Math.PI / 180);
  const dens = Math.pow(10, 0.5 + rnd() * 4.2), hh = Math.max(1, Math.round(dens * km2 / 2.5));
  const ctx = new Array(33).fill(0); ctx[0] = hh;
  for (let k = 0; k < 4; k++){ ctx[1 + k] = Math.round(hh * 40 * (0.5 + rnd())); ctx[5 + 4 * k] = level; ctx[6 + 4 * k] = hh; ctx[7 + 4 * k] = Math.round(hh * (0.5 + rnd())); ctx[8 + 4 * k] = 1; }
  for (let l = 2; l <= level; l++) ctx[21 + l - 2] = Math.round(hh * Math.pow(4, level - l) * (0.8 + 0.4 * rnd()));
  ctx[32] = Math.round(hh * (2 + rnd()));
  const cp = w.holon_ctx_ptr() >>> 0; new Float64Array(mem.buffer, cp, 33).set(ctx);
  const id = H.pack(c); if (w.holon_layout(id) < 0) throw new Error("bad leaf " + id);
  const mp = w.holon_buf_ptr(4) >>> 0, m = new Float64Array(mem.buffer, mp, 7);
  out.push([id, ...ctx, m[5], m[6]].join(" "));
}
console.log(out.join("\n"));
