// The acuity ring's frame-time governor on fixed frame-time series (docs/holon.js HOLON.governor).
// Run: node tools/holon/governor_test.mjs
import { createRequire } from "node:module";
const H = createRequire(import.meta.url)(new URL("../../docs/holon.js", import.meta.url).pathname);
let bad = 0; const ok = (c, m) => { if (!c){ bad++; console.log("FAIL", m); } };
const run = (s, series) => { for (const ms of series) H.governor(s, ms); return s; };
const fresh = () => ({ ring: 600, min: 150, max: 900, ema: 0, cool: 0 });
// slow frames (30 ms): the ring shrinks to its floor and stays there
let s = run(fresh(), Array(200).fill(30)); ok(s.ring === 150, "30 ms -> min, got " + s.ring);
// fast frames (10 ms) afterwards: it grows back to the ceiling
s = run(s, Array(400).fill(10)); ok(s.ring === 900, "10 ms -> max, got " + s.ring);
// inside the band (20-26 ms alternating): no movement at all
s = fresh(); s.ema = 23; const r0 = s.ring; run(s, Array(300).fill(0).map((_, k) => k % 2 ? 20 : 26)); ok(s.ring === r0, "band holds, got " + s.ring);
// a 60 Hz display (16.7 ms every frame) grows the ring to its ceiling
s = run(fresh(), Array(400).fill(16.7)); ok(s.ring === 900, "60 Hz -> max, got " + s.ring);
// one 100 ms spike in a fast series does not shrink the ring (EMA)
s = fresh(); s.ema = 10; run(s, [10, 10, 100, 10, 10]); ok(s.ring >= 600, "spike ignored, got " + s.ring);
// moves are rate-limited: from 30 ms the ring takes several cool-downs, not one frame
s = run(fresh(), Array(20).fill(30)); ok(s.ring > 150 && s.ring < 600, "gradual, got " + s.ring);
// the ceiling follows the viewport: a smaller max clamps at once
s = fresh(); s.max = 300; run(s, [10]); ok(s.ring === 300, "clamp to max, got " + s.ring);
console.log(bad ? `FAIL ${bad}` : "ok governor");
process.exit(bad ? 1 : 0);
