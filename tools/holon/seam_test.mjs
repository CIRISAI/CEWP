// The page's tree (docs/holon.js HOLON.tree) driving the kernel (docs/kernel/kernel_scale.wasm):
// neighbouring leaves must list the same street crossings on their shared side, as the Rust
// witness neighbours_meet_at_the_same_crossings checks for tiling::HolonTree. Synthetic inputs
// only (a made-up town; no real place). Run: node tools/holon/seam_test.mjs
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
const root = new URL("../../docs/", import.meta.url).pathname;
const H = createRequire(import.meta.url)(root + "holon.js");
const { instance } = await WebAssembly.instantiate(readFileSync(root + "kernel/kernel_scale.wasm"), {});
const w = instance.exports, mem = w.memory;
w.micro_params_reset();
const centre = (c) => { const b = H.boundsDeg(c); return [(b[0] + b[2]) / 2, (b[1] + b[3]) / 2]; };
const rootF = (c) => { const [la, lo] = centre(c); return Math.floor(Math.exp(-((la - 10) ** 2 + (lo - 20) ** 2) / 2) * 4e6 + 2e4); };
const extF = (c) => { const [la, lo] = centre(c); const d = Math.hypot(la - 10, lo - 20); return Math.floor(1000 * Math.exp(-d * 20) + 50); };
const split = (p, pb, ext) => {
  const ptr = w.holon_ext_ptr(); new Uint32Array(mem.buffer, ptr, 25).set(ext);
  const n = w.holon_split(H.pack(p), pb); const o = w.holon_buf_ptr(0); return Array.from(new Float64Array(mem.buffer, o, n));
};
const T = H.tree(rootF, extF, split, 60);
const layout = (leaf) => {
  const ctx = T.context(leaf); const ptr = w.holon_ctx_ptr(); new Float64Array(mem.buffer, ptr, 32).set(ctx);
  w.holon_layout(H.pack(leaf));
  const pp = w.holon_buf_ptr(3), pn = w.holon_buf_len(3), P = new Float64Array(mem.buffer, pp, pn), ports = [];
  for (let k = 0; k < pn; k += 3) ports.push({ d: P[k], pos: P[k + 1], main: P[k + 2] });
  const mp = w.holon_buf_ptr(4); const meta = Array.from(new Float64Array(mem.buffer, mp, w.holon_buf_len(4)));
  return { ports, meta };
};
const UPD = H.UPD; let pairs = 0, mixed = 0, nports = 0, bad = 0;
const units = (c) => { const s = H.STEP[c.level]; return [c.i * s, c.j * s, (c.i + 1) * s, (c.j + 1) * s]; };
for (const [lat, lon0, lon1] of [[10.011, 19.95, 20.08], [10.311, 20.1, 20.45]]){
  for (let lon = lon0; lon < lon1;){
    const a = T.leafAt(lat, lon), la = layout(a), [y0, x0, y1, x1] = units(a);
    for (let d = 0; d < 4; d++){
      const seen = new Set();
      for (let k = 0; k < 16; k++){
        const f = (k + 0.5) / 16;
        const [u, v] = d === 0 ? [x0 + f * (x1 - x0), y1 + 0.5] : d === 2 ? [x0 + f * (x1 - x0), y0 - 0.5] : d === 1 ? [x1 + 0.5, y0 + f * (y1 - y0)] : [x0 - 0.5, y0 + f * (y1 - y0)];
        const b = T.leafAt(v / UPD, u / UPD), kb = H.pack(b); if (seen.has(kb)) continue; seen.add(kb);
        const lb = layout(b), [by0, bx0, by1, bx1] = units(b);
        const [lo, hi] = d % 2 === 0 ? [Math.max(x0, bx0), Math.min(x1, bx1)] : [Math.max(y0, by0), Math.min(y1, by1)];
        const mine = la.ports.filter(p => p.d === d && p.pos > lo && p.pos < hi).map(p => p.pos + ":" + p.main).join(",");
        const theirs = lb.ports.filter(p => p.d === (d + 2) % 4 && p.pos > lo && p.pos < hi).map(p => p.pos + ":" + p.main).join(",");
        if (mine !== theirs){ bad++; console.log("SEAM", JSON.stringify(a), d, JSON.stringify(b), mine, "|", theirs); }
        pairs++; nports += mine ? mine.split(",").length : 0; if (a.level !== b.level) mixed++;
      }
    }
    lon = x1 / UPD + 1e-7;
  }
}
// determinism: evict everything below L1 and rebuild one leaf: identical digest
const leaf = T.leafAt(10.004, 20.006), d1 = layout(leaf).meta.slice(5, 7).join(":");
T.evict(1); const d2 = layout(T.leafAt(10.004, 20.006)).meta.slice(5, 7).join(":");
if (d1 !== d2){ bad++; console.log("EVICT", d1, d2); }
const ok = !bad && pairs > 100 && mixed > 10 && nports > 50;
console.log(`${ok ? "ok" : "FAIL"} seams: ${pairs} pairs, ${mixed} of mixed depth, ${nports} crossings, ${bad} mismatches`);
process.exit(ok ? 0 : 1);
