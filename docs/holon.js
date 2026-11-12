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
  // The tree over caller inputs, the same walk as tiling::HolonTree: root(l0) -> households,
  // ext(child) -> weight, split(parent, households, weights) -> child households (the kernel's
  // largest-remainder apportionment). Budgets are memoised in B; evicting them is always safe.
  function tree(root, ext, split, target){
    const B = new Map();
    const T = {
      B, target,
      budget(c){
        if (c.level === 0) return root(c);
        const k = pack(c); const b = B.get(k); if (b !== undefined) return b;
        const p = parent(c), pb = T.budget(p), kids = children(p), out = split(p, pb, kids.map(ext));
        for (let q = 0; q < kids.length; q++) B.set(pack(kids[q]), out[q]);
        return B.get(k);
      },
      isLeaf(c){ return c.level >= L3 && (T.budget(c) <= target || c.level >= MAX_LEVEL); },
      leafOver(c){ for (let l = L3; l <= c.level; l++){ const a = ancestor(c, l); if (T.isLeaf(a)) return a; } return null; },
      leafAt(lat, lon){ let c = at(L3, lat, lon); while (!T.isLeaf(c)) c = at(c.level + 1, lat, lon); return c; },
      // LeafContext::to_f64: budget, 4 L2 neighbours, per side (level, own, other, present), then
      // the budgets from L2 down to the leaf
      context(leaf){
        const v = new Float64Array(33), a2 = ancestor(leaf, 2); // v[32]: the leaf's people, set by the caller (tlContext)
        v[0] = T.budget(leaf);
        for (let d = 0; d < 4; d++){
          const n2 = neighbour(a2, d); v[1 + d] = n2 ? T.budget(n2) : 0;
          const nb = neighbour(leaf, d); if (!nb) continue;
          const y = T.leafOver(nb), level = y ? y.level : leaf.level;
          const own = ancestor(leaf, level), other = neighbour(own, d);
          v[5 + 4 * d] = level; v[6 + 4 * d] = T.budget(own); v[7 + 4 * d] = T.budget(other); v[8 + 4 * d] = 1;
        }
        for (let l = 2; l <= leaf.level; l++) v[21 + l - 2] = T.budget(ancestor(leaf, l));
        return v;
      },
      // drop memoised budgets at `level` and below (they come back identical)
      evict(level){ for (const k of [...B.keys()]) if (Math.floor(k / P47) >= level) B.delete(k); },
    };
    return T;
  }
  // Frame-time governor for the acuity ring: an EMA of frame time; above 28 ms (under ~36 fps) the
  // ring shrinks, below 18 ms (a 60 Hz display's 16.7 ms frame included) it grows, in between it
  // holds (hysteresis), with a cool-down after each move.
  function governor(s, ms){
    ms = Math.min(100, Math.max(1, ms));
    s.ema = s.ema ? s.ema * 0.9 + ms * 0.1 : ms;
    s.ring = Math.min(s.max, Math.max(s.min, s.ring));
    if (s.cool > 0){ s.cool--; return s; }
    if (s.ema > 28 && s.ring > s.min){ s.ring = Math.max(s.min, s.ring * 0.85); s.cool = 12; }
    else if (s.ema < 18 && s.ring < s.max){ s.ring = Math.min(s.max, s.ring * 1.06); s.cool = 6; }
    return s;
  }
  return { UPD, STEP, RATIO, MAX_LEVEL, L3, at, make, pack, unpack, parent, ancestor, children, neighbour, boundsDeg, tree, governor };
})();
if (typeof module !== "undefined") module.exports = HOLON;
