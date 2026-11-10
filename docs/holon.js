// Holon lattice ids: the same integer arithmetic as crates/cewp-sim/src/tiling.rs (HolonId).
// A holon is a lat/lon cell: L0 1.5°, L1 0.3°, L2 0.06°, L3 0.02° (the street tile), then halves
// down to L12. Its id is level·2^47 + (i + 2^22)·2^24 + (j + 2^23), below 2^51, so a JavaScript
// number holds it exactly. Checked against the Rust vectors by tools/holon/id_test.mjs.
"use strict";
const HOLON = (() => {
  const UPD = 25600, STEP = [38400, 7680, 1536, 512, 256, 128, 64, 32, 16, 8, 4, 2, 1];
  const RATIO = [1, 5, 5, 3, 2, 2, 2, 2, 2, 2, 2, 2, 2];
  const LON_UNITS = 360 * UPD, LAT_HALF = 90 * UPD, MAX_LEVEL = 12, L3 = 3;
  const P47 = 140737488355328, P24 = 16777216, I_OFF = 4194304, J_OFF = 8388608;
  const fdiv = (a, b) => Math.floor(a / b); // exact: |a| < 2^24, b a positive integer
  const cols = (l) => LON_UNITS / STEP[l], rowsHalf = (l) => LAT_HALF / STEP[l];
  function wrapJ(l, j){ const n = cols(l), h = n / 2; return ((((j + h) % n) + n) % n) - h; }
  function make(level, i, j){
    if (level < 0 || level > MAX_LEVEL) return null;
    const h = rowsHalf(level); if (i < -h || i >= h) return null;
    return { level, i, j: wrapJ(level, j) };
  }
  function at(level, lat, lon){
    const s = STEP[level], y = Math.floor(lat * UPD), x = Math.floor(lon * UPD), h = rowsHalf(level);
    return { level, i: Math.min(h - 1, Math.max(-h, fdiv(y, s))), j: wrapJ(level, fdiv(x, s)) };
  }
  const pack = (c) => c.level * P47 + (c.i + I_OFF) * P24 + (c.j + J_OFF);
  function unpack(v){
    const level = Math.floor(v / P47), r = v - level * P47;
    const i = Math.floor(r / P24) - I_OFF, j = (r % P24) - J_OFF;
    const c = make(level, i, j); return c && pack(c) === v ? c : null;
  }
  function parent(c){ if (c.level === 0) return null; const r = RATIO[c.level]; return { level: c.level - 1, i: fdiv(c.i, r), j: fdiv(c.j, r) }; }
  function ancestor(c, level){ while (c.level > level) c = parent(c); return c; }
  function children(c){
    if (c.level >= MAX_LEVEL) return [];
    const r = RATIO[c.level + 1], out = [];
    for (let a = 0; a < r; a++) for (let b = 0; b < r; b++) out.push({ level: c.level + 1, i: c.i * r + a, j: c.j * r + b });
    return out;
  }
  const DIRS = [[1, 0], [0, 1], [-1, 0], [0, -1]]; // N, E, S, W (tiling::Dir order)
  function neighbour(c, d){ return make(c.level, c.i + DIRS[d][0], c.j + DIRS[d][1]); }
  function boundsDeg(c){ const s = STEP[c.level]; return [c.i * s / UPD, c.j * s / UPD, (c.i + 1) * s / UPD, (c.j + 1) * s / UPD]; }
  return { UPD, STEP, RATIO, MAX_LEVEL, L3, at, make, pack, unpack, parent, ancestor, children, neighbour, boundsDeg };
})();
if (typeof module !== "undefined") module.exports = HOLON;
