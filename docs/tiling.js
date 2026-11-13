// Street tiling: the holon tree (cewp_sim::tiling, FSD §4.6.6) drawn over settled land out to the
// acuity ring. The page keeps the tree; the kernel splits households and lays out each leaf. Every
// street and home here is generated: the only real inputs are the coarse population grid, the 50m
// land mask and the populated-place weights, and real map layers have faded out before streets
// fade in (below 26 m a pixel). The ground under them is tinted by tlTerrain (MODEL: ETOPO elevation,
// latitude and shaded relief, and Copernicus LC100 2019 land cover where it answers).
"use strict";
const TL_TARGET = 60, TL_PPH = 2.5;           // P::TargetHouseholds, P::PersonsPerHousehold defaults
const TL_MF = (EARTH_M * Math.PI / 180) / 111320; // layout metres (111,320 m a degree) to globe metres
const TL_BYTE_CAP = 60e6, TL_BUDGET_CAP = 250000, TL_GEN_MS = 5;
const TL = {
  T: null, l0: new Map(), lay: new Map(), bytes: 0, queue: [], ring: { ring: 900, min: 140, max: 900, ema: 0, cool: 0 },
  lastT: 0, ringEff: 0, mains: new Map(), batch: new Map(), draws: 0, drawn: 0, agg: 0, pending: 0, genMs: 0, frameMs: 16.7, focal: -1, placeBins: null, on: false,
  stats(){ return { on: this.on, ring: Math.round(this.ringEff || this.ring.ring), governor: Math.round(this.ring.ring), ringMax: Math.round(this.ring.max), fps: Math.round(1000 / Math.max(1, this.ring.ema || 16.7)), leaves: this.drawn, draws: this.draws, aggregate: this.agg, pending: this.pending, cached: this.lay.size, mb: +(this.bytes / 1e6).toFixed(1), budgets: this.T ? this.T.B.size : 0, genMs: +this.genMs.toFixed(1) }; },
};
// The land test the tree reads is Natural Earth 50m (1:50 million), loaded once below 12 km a pixel:
// the 110m mask puts many coastal cities in the sea. The tree waits for it (and the place list),
// so every budget is computed from the same inputs whenever it is computed.
let TL_LAND = null;
const tlLand = () => TL_LAND || (GEO.l50 ? (TL_LAND = landTest(GEO.l50.rings)) : null);
// the tree also waits for GHS-POP at 1/8 degree: every budget is conditioned on it (popcond.js)
const tlPop = () => (typeof GEODATA !== "undefined" && GEODATA.pop[1]) || null;
// the kernel holds the population chain (cewp_sim::popchain): hand it the 1/8-degree grid once, with
// the cohort tier's GHS total (POP_GRID_PEOPLE), so the street tier apportions the same people
function tlPopKernel(){
  if (TL.popOk) return true; const P = tlPop(); if (!P || !wasm || !wasm.pop_load) return false;
  // the in-cell seam's anchors: each cell's first Natural Earth populated place (largest first), before any split
  if (wasm.pop_anchors && GEO.places){ const pl = GEO.places, ap = wasm.alloc(pl.length * 8) >>> 0, A = new Int32Array(mem.buffer, ap, pl.length * 2);
    pl.forEach((p, k) => { A[2 * k] = Math.round(p.la * 1e4); A[2 * k + 1] = Math.round(p.lo * 1e4); }); TL.anchors = wasm.pop_anchors(ap, pl.length); }
  const n = P.length, ptr = wasm.alloc(n * 4) >>> 0; new Float32Array(mem.buffer, ptr, n).set(P);
  wasm.pop_load(ptr, n, typeof POP_GRID_PEOPLE === "number" ? POP_GRID_PEOPLE : 0);
  TL.popTotals = Array.from(new Float64Array(mem.buffer, wasm.holon_buf_ptr(0) >>> 0, 4)); TL.popOk = true; return true;
}
const tlReady = () => !!(wasm && wasm.holon_split && wasm.holon_arterials && wasm.micro_build_leaf && GEO.places && tlLand() && tlPopKernel());

// ---- inputs: households per L0 cell, a weight per child
function tlCentre(c){ const b = HOLON.boundsDeg(c); return [(b[0] + b[2]) / 2, (b[1] + b[3]) / 2]; }
// people per holon, beside the tree's households (HOLON.tree memoises households only)
const tlPP = new Map();
const tlOut = (n) => new Float64Array(mem.buffer, wasm.holon_buf_ptr(0) >>> 0, n);
function tlRoot(c){ // a level-0 holon's households from the kernel's world apportionment (popchain::World)
  const key = HOLON.pack(c); let v = TL.l0.get(key); if (v !== undefined) return v;
  v = wasm.pop_root(key); tlPP.set(key, tlOut(2)[0]); TL.l0.set(key, v); return v;
}
function tlExt(c){ return 0; } // weights come from the kernel's own split (pop_split)
function tlSplit(p, pb){ // people and households of p's children, apportioned in the kernel (popchain::split)
  const k = HOLON.pack(p); let pp = tlPP.get(k);
  if (pp === undefined){ wasm.pop_of(k); pp = tlOut(2)[0]; tlPP.set(k, pp); }
  const n = wasm.pop_split(k, pp, pb), o = tlOut(2 * n), kids = HOLON.children(p), hh = [];
  for (let q = 0; q < n; q++){ tlPP.set(HOLON.pack(kids[q]), o[q]); hh.push(o[n + q]); }
  if (tlPP.size > 400000) tlPP.clear();
  return hh;
}
// a leaf's context with its people (the kernel apportions them over the leaf's households)
function tlContext(leaf){ const ctx = tlTree().context(leaf), k = HOLON.pack(leaf); let pp = tlPP.get(k);
  if (pp === undefined){ wasm.pop_of(k); pp = tlOut(2)[0]; } ctx[32] = pp; return ctx; }
function tlTree(){ if (!TL.T) TL.T = HOLON.tree(tlRoot, tlExt, tlSplit, TL_TARGET); return TL.T; }
// the in-cell seam's switch (kernel pop_incell): CIRISHolon's UNEVEN-0 inside a cell (on, the default) or the legacy
// even spread; every budget below the cell changes, so the tree, its people and the leaf geometry are rebuilt
function tlIncell(on){
  if (typeof wasm === "undefined" || !wasm || !wasm.pop_incell) return; MICRO_GEN++; wasm.pop_incell(on ? 1 : 0); TL.incell = !!on; tlRebuild();
}
// the household seam's switch (kernel pop_households): CIRISHolon's generator per L4 tile (on, the default) or the
// legacy rule ("no certificate"); every leaf's households and layout change, so the same rebuild
function tlHouseholds(on){
  if (typeof wasm === "undefined" || !wasm || !wasm.pop_households) return; MICRO_GEN++; wasm.pop_households(on ? 1 : 0); TL.housegen = !!on; tlRebuild();
}
// the device seam's switch (kernel micro_device_rule): CIRISHolon's DEVICEGEN-1 per household or the legacy draw
function tlDevices(on){
  if (typeof wasm === "undefined" || !wasm || !wasm.micro_device_rule) return; MICRO_GEN++; wasm.micro_device_rule(on ? 1 : 0, typeof homeWhere === "number" ? homeWhere : 0); TL.devicegen = !!on;
  if (typeof M !== "undefined") M = null; if (typeof gfxDirty === "function") gfxDirty(800);
}
// the runtime table door (kernel door_*): after a table arrives, a layout is kept exactly while every slot it read is at the
// version it read; the others (and their tiles' buffers, and the focal world on one) go and are laid out again
function tlDoorChanged(){
  if (typeof wasm === "undefined" || !wasm || !wasm.door_versions) return 0;
  const n = wasm.door_versions(), v = new Float64Array(wasm.memory.buffer, wasm.holon_buf_ptr(0) >>> 0, n); let gone = 0;
  for (const [k, e] of TL.lay){ const f = e.form; if (!f || [0, 1, 2, 3].every((s) => !(f.read & (1 << s)) || f.v[s] === v[s])) continue;
    TL.lay.delete(k); TL.bytes -= e.bytes; gone++; const bt = TL.batch.get(e.tile); if (bt){ gl.deleteBuffer(bt.vbo); TL.batch.delete(e.tile); }
    if (typeof M !== "undefined" && M && M.leaf === k){ M = null; MICRO_GEN++; } }
  if (gone && typeof gfxDirty === "function") gfxDirty(800);
  return gone;
}
function tlRebuild(){
  TL.T = null; TL.l0.clear(); tlPP.clear(); TL.lay.clear(); TL.bytes = 0;
  for (const [, bt] of TL.batch) gl.deleteBuffer(bt.vbo); TL.batch.clear();
  if (typeof M !== "undefined" && M && M.leaf !== undefined) M = null;
  if (typeof incellReset === "function") incellReset();
  if (typeof gfxDirty === "function") gfxDirty(1500);
}

// one density tone for the mosaic and the leaves' ground: households per km², 10 → 0, ~3,000 → 1.
// tlGround turns it into a colour between the terrain band's sparse and dense ground.
const tlTone = (d) => Math.max(0, Math.min(1, (Math.log10(Math.max(1, d)) - 1) / 2.5));
const TL_ROAD = [[0.47, 0.49, 0.53], [0.47, 0.49, 0.52], [0.45, 0.47, 0.44], [0.5, 0.48, 0.41]];
// ---- the ground under the generated streets follows the real terrain. MODEL, a visual rule.
// Real inputs, read at the holon: NOAA ETOPO 2022 elevation and Natural Earth shaded relief
// (relief.js), and the latitude. From them a stand-in for annual warmth, in degrees:
// 27 at the equator, 0.42 less per degree of latitude beyond 12, 6.5 less per km of height.
// It picks the ground's two tones (sparse and dense) from five bands, turns high ground to bare
// rock from 2,200 m, and the shaded relief lights or darkens the slope. Homes take a paler roof in
// the cold and the heat. Without land cover (below), a desert and a forest at one
// latitude and height look alike; the streets themselves are laid out the same way everywhere.
// Where Copernicus LC100 land cover answers (docs/geo/geodata.js), its class's ground colour replaces
// the band's two ground tones; the band rule stays for the roofs and where there is no land-cover data.
const TL_BANDS = [ // [warmth, sparse ground, dense ground, roof multiplier]
  [-4, [0.2, 0.225, 0.27], [0.44, 0.47, 0.52], [1.2, 1.22, 1.3]],     // snow and ice
  [5, [0.105, 0.14, 0.145], [0.255, 0.3, 0.27], [1.04, 1.05, 1.08]],  // cold: moss and grey
  [13, [0.095, 0.145, 0.11], [0.25, 0.315, 0.2], [1, 1, 1]],          // temperate: green
  [20, [0.12, 0.135, 0.105], [0.325, 0.305, 0.205], [1.06, 1.03, 0.97]], // warm: khaki
  [26.5, [0.135, 0.125, 0.095], [0.385, 0.315, 0.2], [1.2, 1.12, 0.98]], // hot: ochre
];
const TL_ROCK = [[0.13, 0.13, 0.137], [0.31, 0.3, 0.29]];
// Roof tones (builtform::RoofTone), the approximate sRGB CIRISHolon's BUILDGEN-1 prereg records as ASSUMED ("what the material
// usually looks like from above, nothing more"); index 0, neutral, keeps the renderer's present roof colour. No generator
// ships, so every roof is neutral today.
const TL_ROOF_TONES = [null, [178, 98, 72], [142, 122, 104], [128, 112, 94], [92, 100, 110], [84, 82, 80], [168, 172, 176], [150, 150, 146]].map((c) => c && c.map((v) => v / 255));
const tlMix3 = (a, b, t) => [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t];
// { g0, g1: the ground at density tone 0 and 1; roof: a multiplier on the homes; warm, m: what it read }
function tlTerrain(la, lo){
  const r = typeof reliefAt === "function" ? reliefAt(la, lo) : null, m = r ? r[0] : 0, sh = r ? r[1] : 1;
  const warm = 27 - 0.42 * Math.max(0, Math.abs(la) - 12) - 0.0065 * m;
  let k = 0; while (k < TL_BANDS.length - 2 && warm > TL_BANDS[k + 1][0]) k++;
  const A = TL_BANDS[k], B = TL_BANDS[k + 1], t = Math.max(0, Math.min(1, (warm - A[0]) / (B[0] - A[0])));
  let g0 = tlMix3(A[1], B[1], t), g1 = tlMix3(A[2], B[2], t); const roof = tlMix3(A[3], B[3], t);
  const rock = 0.65 * sstep(2200, 4200, m) * sstep(-4, 2, warm); // bare high ground, below the snow
  g0 = tlMix3(g0, TL_ROCK[0], rock); g1 = tlMix3(g1, TL_ROCK[1], rock);
  // real land cover where it answers (Copernicus LC100 2019, majority class per 1/8 or 1/2 degree, MEASURED):
  // the class's ground colour (a display choice, MODEL) replaces the latitude / elevation rule, which still
  // holds where there is no land-cover data (Antarctica, no input) and still gives the roofs
  const L = typeof geoLandcoverAt === "function" ? geoLandcoverAt(la, lo) : null;
  // the sparse end of the density tone is the cell's class colour; the dense end is built-up ground
  // (the urban class's colour), so a dense leaf in a cell whose majority class is forest (Kowloon's
  // hills) is not drawn on green
  if (L && L.ground){ const b = L.ground.map((v) => v / 255), u = ((GEO_LC_LEGEND[50] && GEO_LC_LEGEND[50][2]) || [128, 124, 120]).map((v) => v / 255); g0 = b.map((v) => v * 0.42); g1 = u.map((v) => v * 0.8); }
  const lit = Math.max(0.62, Math.min(1.32, 1 + (sh - 1) * 1.5));
  return { g0: g0.map(v => v * lit), g1: g1.map(v => v * lit), roof, warm, m, lit, z: r ? r[2] : -1, lc: L ? L.code : -1 };
}
const tlGround = (T, t) => tlMix3(T.g0, T.g1, t);
// The rival texture's switch (Advanced: "ground texture: flat / procedural"), off by default.
const TL_TEX = { on: typeof location !== "undefined" && /[?&]tex=1(&|$)/.test(location.search) }; // ?tex=1 starts it on (for the witnesses)
// The texture's world-anchored metre grid for an L3 tile's frame: x = (lon + 180) x 111,320 x cos(the 1.5-degree latitude band's
// centre), y = (lat + 90) x 111,320, both taken modulo the noise's 4,096 m period in double precision here, so the shader
// works in small numbers; a tile's frame metres map to it by x' = x0 + kx * x (kx = cos(band) / cos(tile)) and y' = y0 + y.
// The same point gives the same grid value from every tile of its band: the field is seamless across leaves and tiles,
// and steps only at a band's edge (every 1.5 degrees of latitude).
function tlNoiseOrigin(la, lo){
  const band = (Math.floor((la + 90) / 1.5) + 0.5) * 1.5 - 90, cb = Math.cos(band * Math.PI / 180), P = 4096, md = (v) => ((v % P) + P) % P;
  return [md((lo + 180) * 111320 * cb), md((la + 90) * 111320), cb / Math.cos(la * Math.PI / 180)];
}
// The land cover's ground colour at a point, blended bilinearly between the four nearest cells' class colours (the
// class is a 1/8 or 1/2 degree cell's majority, MEASURED; its colour a display choice), sea cells left out: the mosaic's
// corners take it, so the colour runs across a cell's edge instead of stepping there (a film strip of the Story's dive
// showed 14 km blocks of flat colour at 20 to 120 m a pixel). Cached by corner: corners are exact grid values.
const TL_LCG = new Map();
function tlLcGround(la, lo){
  const key = la.toFixed(6) + "," + lo.toFixed(6); let v = TL_LCG.get(key); if (v !== undefined) return v;
  v = null; const L = typeof geoLandcoverAt === "function" ? geoLandcoverAt(la, lo) : null;
  if (L){ const st = GEODATA.STEP[L.level], fx = (lo + 180) / st - 0.5, fy = (90 - la) / st - 0.5, c0 = Math.floor(fx), r0 = Math.floor(fy), ax = fx - c0, ay = fy - r0;
    const acc = [0, 0, 0]; let wsum = 0;
    for (const [dr, dc, w] of [[0, 0, (1 - ax) * (1 - ay)], [0, 1, ax * (1 - ay)], [1, 0, (1 - ax) * ay], [1, 1, ax * ay]]){
      if (w <= 0) continue; const e = geoLandcoverAt(90 - (r0 + dr + 0.5) * st, -180 + (c0 + dc + 0.5) * st, L.level);
      if (!e || !e.ground || e.code === 200) continue; for (let k = 0; k < 3; k++) acc[k] += w * e.ground[k] / 255; wsum += w; }
    if (wsum > 0) v = acc.map((x) => x / wsum); }
  if (TL_LCG.size > 60000) TL_LCG.clear(); TL_LCG.set(key, v); return v;
}
// A leaf's geometry, in the metres of its L3 tile's frame (one affine per tile, one draw per tile).
// Each vertex: position, normal, half width, colour, kind (0 ground, 1 casing, 2 road, 3 building).
function tlBuildLeaf(leaf){
  const T = tlTree(), ctx = tlContext(leaf);
  const cp = wasm.holon_ctx_ptr(); new Float64Array(mem.buffer, cp, 33).set(ctx);
  const id = HOLON.pack(leaf); wasm.holon_layout(id);
  const buf = (i) => { const p = wasm.holon_buf_ptr(i), n = wasm.holon_buf_len(i); return new Float64Array(mem.buffer, p, n).slice(); };
  const R = buf(1), B = buf(2), meta = buf(4), S = buf(6), w = meta[0], h = meta[1], cls = meta[3] | 0;
  const TN = wasm.holon_buf_len(8) ? buf(8) : null; // each building's roof tone (kernel buffer 8; 0 neutral until a generator draws one)
  const t = HOLON.ancestor(leaf, 3), [laL, loL] = tlCentre(leaf), [laT, loT] = tlCentre(t);
  const cT = Math.cos(laT * Math.PI / 180), sx = cT / Math.cos(laL * Math.PI / 180), ox = (loL - loT) * 111320 * cT, oy = (laL - laT) * 111320;
  const V = []; const vert = (x, y, nx, ny, wd, c, a, k) => V.push(ox + x * sx, oy + y, nx, ny, wd, c[0], c[1], c[2], a, k);
  const quad = (p, c, a, k) => { for (const q of [0, 1, 2, 0, 2, 3]) vert(p[q][0], p[q][1], 0, 0, 0, c, a, k); };
  // the ground carries its density tone in red; the tile's terrain turns it into a colour when drawn
  quad([[-w / 2, -h / 2], [w / 2, -h / 2], [w / 2, h / 2], [-w / 2, h / 2]], [tlTone(meta[2] / Math.max(1e-6, w * h / 1e6)), 0, 0], 0.9, 0);
  // street ends from the leaf's street graph (tiling::StreetGraph, holon buffer 6: arterial, start kind, end kind;
  // kinds 0 exit, 1 dead end, 2 tee, 3 cross, 4 corner)
  const road = (r, extra, c, a, k) => {
    let [x0, y0, x1, y1, wd] = [R[r], R[r + 1], R[r + 2], R[r + 3], R[r + 4] / 2 + extra];
    const L = Math.hypot(x1 - x0, y1 - y0); if (L < 1e-6) return; const ux = (x1 - x0) / L, uy = (y1 - y0) / L;
    // an exit meets the neighbour's street flush; every other end (a dead end, or a tee or corner on another street) gets a square cap
    const st = (r / 5) * 3;
    if (S[st + 1] !== 0){ x0 -= ux * wd; y0 -= uy * wd; } if (S[st + 2] !== 0){ x1 += ux * wd; y1 += uy * wd; }
    const nx = -uy, ny = ux;
    for (const [px, py, s] of [[x0, y0, -1], [x1, y1, -1], [x1, y1, 1], [x0, y0, -1], [x1, y1, 1], [x0, y0, 1]]) vert(px, py, nx * s, ny * s, wd, c, a, k);
  };
  // the ground between buildings (generated decoration, display only; drawn before the streets, which cover it): a paler lot around each footprint and a
  // few trees in it, placed by a hash of the leaf and the building, so the same street always looks the same
  const hsh = (a, b) => { let x = Math.imul(a | 0, 0x9e3779b1) ^ Math.imul(b | 0, 0x85ebca77); x ^= x >>> 15; x = Math.imul(x, 0x2c1b3c6d); x ^= x >>> 12; return (x >>> 0) / 4294967296; };
  // a hexagon (6 triangles): the street ground's decoration is kept cheap, about 2,000 vertices a leaf
  // kind 1.25: canopy (a tree's crown), drawn as kind 1 (the shader rounds the kind) and marked for the ground texture
  const disc = (x, y, r, c, a, kd = 1) => { for (let k = 0; k < 6; k++){ const a0 = k * Math.PI / 3, a1 = (k + 1) * Math.PI / 3; vert(x, y, 0, 0, 0, c, a, kd); vert(x + r * Math.cos(a0), y + r * Math.sin(a0), 0, 0, 0, c, a, kd); vert(x + r * Math.cos(a1), y + r * Math.sin(a1), 0, 0, 0, c, a, kd); } };
  const seed = (id % 2147483647) | 0;
  // over the whole leaf: soft mottling (meadow and wear) and scattered trees, fewer where it is dense;
  // buildings and streets drawn later cover whatever lands under them
  { const area = w * h, dens = meta[2] / Math.max(1e-6, area / 1e6), pkm = dens * TL_PPH, nT = pkm >= 6000 ? 0 : Math.min(70, Math.round(area / (pkm >= 1500 ? 1600 : 700))), nP = Math.min(14, Math.round(area / 9000)); // no trees where it is apartment blocks or denser
    for (let k = 0; k < nP; k++){ const x = (hsh(seed, 9000 + k) - 0.5) * w, y = (hsh(9000 + k, seed) - 0.5) * h, r = 8 + 18 * hsh(seed ^ k, 77);
      disc(x, y, r, hsh(k, seed) < 0.5 ? [0.5, 0.55, 0.36] : [0.04, 0.07, 0.04], 0.07); }
    for (let k = 0; k < nT; k++){ const x = (hsh(seed, k) - 0.5) * w, y = (hsh(k, seed + 1) - 0.5) * h, r = 1.4 + 2.2 * hsh(seed + 2, k), gk = hsh(k, 31);
      disc(x, y, r, [0.09 + 0.05 * gk, 0.17 + 0.08 * gk, 0.08], 0.8, 1.25); } }
  const lots = meta[2] * TL_PPH / Math.max(1e-6, w * h / 1e6) < 1500; // yards and garden trees only in suburban and village streets
  for (let b = 0; lots && b < B.length; b += 8){
    const cx = B[b], cy = B[b + 1], bw = B[b + 2] / 2, bd = B[b + 3] / 2, ca = Math.cos(B[b + 4]), sa = Math.sin(B[b + 4]);
    const lw = bw + 2 + 0.35 * bw, ld = bd + 2 + 0.35 * bd, at = (u, v) => [cx + ca * u - sa * v, cy + sa * u + ca * v];
    quad([[-lw, -ld], [lw, -ld], [lw, ld], [-lw, ld]].map(([u, v]) => at(u, v)), [0.42, 0.5, 0.33], 0.16, 1);
    const nt = Math.floor(hsh(seed, b) * 3);
    for (let t = 0; t < nt; t++){ const side = hsh(seed + t, b) * 4 | 0, f = hsh(b, seed + 7 * t) * 2 - 1, r = 1.6 + 1.8 * hsh(seed ^ t, b + 3);
      const [u, v] = side === 0 ? [f * lw, ld - r * 0.6] : side === 1 ? [f * lw, -ld + r * 0.6] : side === 2 ? [lw - r * 0.6, f * ld] : [-lw + r * 0.6, f * ld];
      const [x, y] = at(u, v); disc(x, y, r, [0.11, 0.2 + 0.06 * hsh(t, b), 0.1], 0.85, 1.25); }
  }
  // streets by width class (the kernel's own widths): arterial >= 11 m pale and warm on a dark casing,
  // collector >= 7 m light grey, local streets the class colour; local ones drawn first so wider ones cross over
  const rank = (r) => R[r + 4] >= 11 ? 2 : R[r + 4] >= 7 ? 1 : 0, order = []; for (let r = 0; r < R.length; r += 5) order.push(r); order.sort((a, b) => rank(a) - rank(b));
  for (const r of order) road(r, rank(r) === 2 ? 1.6 : 0.8, [0.07, 0.08, 0.095], 0.9, 1); // the casing stays inside the metre every building keeps clear
  for (const r of order) road(r, 0, rank(r) === 2 ? [0.86, 0.78, 0.56] : rank(r) === 1 ? [0.66, 0.66, 0.64] : TL_ROAD[cls], 1, 2);
  for (let b = 0; b < B.length; b += 8){
    const cx = B[b], cy = B[b + 1], bw = B[b + 2] / 2, bd = B[b + 3] / 2, ca = Math.cos(B[b + 4]), sa = Math.sin(B[b + 4]), units = B[b + 6];
    const at = (u, v, dx, dy) => [cx + dx + ca * u - sa * v, cy + dy + sa * u + ca * v];
    // a cast shadow (fixed light from the north-west, display only; +x east, +y north), then the roof in two
    // tones split along the long axis, so footprints read as pitched roofs and not flat blocks
    const sh = 0.5 + 1.0 * Math.max(1, B[b + 5]); // shadow length follows storeys (about 3 m a storey, low sun; display only)
    quad([[-bw, -bd], [bw, -bd], [bw, bd], [-bw, bd]].map(([u, v]) => at(u, v, sh, -sh)), [0.02, 0.025, 0.03], 0.5, 1);
    const tone = TN ? TL_ROOF_TONES[TN[b / 8] | 0] : null; // ASSUMED tone (BUILDGEN-1 prereg), null for neutral: the present colour
    const c0 = tone ? tone : units > 0 ? [0.44, 0.41, 0.36] : [0.3, 0.29, 0.27], lit = c0.map((v) => v * 1.22), dark = c0.map((v) => v * 0.74);
    if (bw >= bd){ quad([[-bw, 0], [bw, 0], [bw, bd], [-bw, bd]].map(([u, v]) => at(u, v, 0, 0)), lit, 1, 3); quad([[-bw, -bd], [bw, -bd], [bw, 0], [-bw, 0]].map(([u, v]) => at(u, v, 0, 0)), dark, 1, 3); }
    else { quad([[-bw, -bd], [0, -bd], [0, bd], [-bw, bd]].map(([u, v]) => at(u, v, 0, 0)), lit, 1, 3); quad([[0, -bd], [bw, -bd], [bw, bd], [0, bd]].map(([u, v]) => at(u, v, 0, 0)), dark, 1, 3); }
  }
  const data = new Float32Array(V);
  // the built form's record (kernel holon_layout meta 7..22): its source, climate class, region, the door slots read and their
  // versions, buildings a generator drew, the state used and why, the generator's pooling
  const form = meta.length >= 23 ? { source: meta[7], climate: meta[8], region: meta[9], read: meta[10], v: Array.from(meta.slice(11, 15)), generated: meta[15], state: meta[16], why: meta[17], pooling: meta[18] >= 0 ? Array.from(meta.slice(18, 22)) : null, mixed: meta[22] } : null;
  const tones = TN ? Array.from(TN).filter((x) => x > 0).length : 0;
  return { id, tile: HOLON.pack(t), t, data, bytes: data.byteLength, hh: meta[2], form, tones };
}
function tlFrameOf(la, lo){ const a = la * Math.PI / 180, o = lo * Math.PI / 180;
  return { c: [Math.cos(a) * Math.sin(o), Math.sin(a), Math.cos(a) * Math.cos(o)], E: [Math.cos(o), 0, -Math.sin(o)], N: [-Math.sin(a) * Math.sin(o), Math.cos(a), -Math.sin(a) * Math.cos(o)] }; }

// ---- the GL program: metres in, an affine per tile to CSS px, streets kept at least a pixel wide,
// alpha per kind, and a radial fade at the acuity ring's edge
const TLV = `attribute vec2 p;attribute vec2 n;attribute float wd;attribute vec4 c;attribute float k;uniform vec3 uX;uniform vec3 uY;uniform vec2 uView;uniform float uPpm;uniform vec4 uMinHalf;uniform vec4 uKindA;uniform vec3 uRing;uniform vec3 uG0;uniform vec3 uG1;uniform vec3 uRoof;uniform float uTerr;uniform vec4 uNO;varying vec4 vc;varying vec2 vN;varying float vK;varying float vD;
void main(){float kr=floor(k+0.5);vec4 sel=vec4(equal(vec4(kr),vec4(0.0,1.0,2.0,3.0)));vec2 q=p+n*max(wd,dot(uMinHalf,sel)/uPpm);float px=uX.x*q.x+uX.y*q.y+uX.z;float py=uY.x*q.x+uY.y*q.y+uY.z;
float f=uRing.z>0.0?1.0-smoothstep(0.85*uRing.z,uRing.z,length(vec2(px,py)-uRing.xy)):1.0;
gl_Position=vec4(px/uView.x*2.0-1.0,1.0-py/uView.y*2.0,0.0,1.0);vec3 col=mix(c.rgb,sel.x*mix(uG0,uG1,c.r)+(sel.y+sel.z)*c.rgb+sel.w*c.rgb*uRoof,uTerr);vc=vec4(col,c.a*dot(uKindA,sel)*f);
vN=vec2(uNO.x+uNO.z*q.x,uNO.y+uNO.w*q.y);vK=k;vD=sel.x*c.r;}`;
// The rival ground texture (FSD/twin/texture.md, the layer-two contract's first user; off unless the Advanced switch says
// "procedural"): band-limited value noise on a world-anchored metre grid (vN, seamless across leaves and tiles), its octaves
// faded out between 4 and 2 pixels a cycle (uPpm), modulating the luminance of ground, canopy and roof by surface kind.
const TLF = `#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif
uniform float uDim;uniform float uTex;uniform float uPpm;varying vec4 vc;varying vec2 vN;varying float vK;varying float vD;
float h2(vec2 i,float m){i=mod(i,m);return fract(sin(dot(i,vec2(12.9898,78.233)))*43758.5453);}
float vn(vec2 x,float cell){vec2 g=x/cell;vec2 i=floor(g);vec2 t=fract(g);t=t*t*(3.0-2.0*t);float m=4096.0/cell;
return mix(mix(h2(i,m),h2(i+vec2(1.0,0.0),m),t.x),mix(h2(i+vec2(0.0,1.0),m),h2(i+vec2(1.0,1.0),m),t.x),t.y)*2.0-1.0;}
float oct(vec2 x,float cell,float amp){float w=1.0-smoothstep(0.25,0.5,1.0/(uPpm*cell));return w>0.0?amp*w*vn(x,cell):0.0;}
void main(){if(vc.a<0.004)discard;vec3 col=vc.rgb;
if(uTex>0.5){float s=0.0;
 if(vK<0.5){float a=1.0-0.5*vD;s=oct(vN,48.0,0.10*a)+oct(vN,12.0,0.07*a)+oct(vN,3.0,0.05*a);}
 else if(vK>1.1&&vK<1.4){s=oct(vN,6.0,0.16)+oct(vN,1.5,0.10);}
 else if(vK>2.5){s=oct(vN,4.0,0.05)+oct(vN,1.0,0.04);}
 col=col*(1.0+s*uTex);}
gl_FragColor=vec4(col*uDim,vc.a);}`;
let tlProg = null, tlLoc = null, tlAggBuf = null;
function tlGL(){
  if (tlProg) return; tlProg = prog(TLV, TLF); tlAggBuf = gl.createBuffer();
  const u = (n) => gl.getUniformLocation(tlProg, n), a = (n) => gl.getAttribLocation(tlProg, n);
  tlLoc = { p: a("p"), n: a("n"), wd: a("wd"), c: a("c"), k: a("k"), X: u("uX"), Y: u("uY"), view: u("uView"), ppm: u("uPpm"), minHalf: u("uMinHalf"), kindA: u("uKindA"), ring: u("uRing"), dim: u("uDim"), g0: u("uG0"), g1: u("uG1"), roof: u("uRoof"), terr: u("uTerr"), no: u("uNO"), tex: u("uTex") };
}
function tlAttribs(){ const L = tlLoc, s = 40;
  gl.enableVertexAttribArray(L.p); gl.vertexAttribPointer(L.p, 2, gl.FLOAT, false, s, 0);
  gl.enableVertexAttribArray(L.n); gl.vertexAttribPointer(L.n, 2, gl.FLOAT, false, s, 8);
  gl.enableVertexAttribArray(L.wd); gl.vertexAttribPointer(L.wd, 1, gl.FLOAT, false, s, 16);
  gl.enableVertexAttribArray(L.c); gl.vertexAttribPointer(L.c, 4, gl.FLOAT, false, s, 20);
  gl.enableVertexAttribArray(L.k); gl.vertexAttribPointer(L.k, 1, gl.FLOAT, false, s, 36); }
function tlNoAttribs(){ const L = tlLoc; for (const k of [L.p, L.n, L.wd, L.c, L.k]) gl.disableVertexAttribArray(k); }
function tlAffine(F, e){
  const qc = camQ(...e.c), qE = camQ(...e.E), qN = camQ(...e.N), s = F.k / EARTH_M * TL_MF;
  return { a: s * qE[0], b: -s * qE[1], c: s * qN[0], d: -s * qN[1], e: F.w / 2 + F.k * qc[0], f: F.h / 2 - F.k * qc[1], front: qc[2], ppm: s };
}
// screen-space quads (mosaic, arterials): kind 0, no ring fade
function tlScreen(arr, alpha){
  if (!arr.length || alpha <= 0.01) return;
  gl.bindBuffer(gl.ARRAY_BUFFER, tlAggBuf); gl.bufferData(gl.ARRAY_BUFFER, arr, gl.STREAM_DRAW); tlAttribs();
  gl.uniform3f(tlLoc.X, 1, 0, 0); gl.uniform3f(tlLoc.Y, 0, 1, 0); gl.uniform1f(tlLoc.ppm, 1); gl.uniform4f(tlLoc.minHalf, 0, 0, 0, 0);
  gl.uniform4f(tlLoc.kindA, alpha, 0, 0, 0); gl.uniform3f(tlLoc.ring, 0, 0, 0); gl.uniform1f(tlLoc.dim, 1); gl.uniform1f(tlLoc.terr, 0); gl.uniform1f(tlLoc.tex, 0); // night and terrain are in the colours; the mosaic (in screen pixels) is never textured
  gl.drawArrays(gl.TRIANGLES, 0, arr.length / 10);
}

// ---- which holons to draw: detail inside the acuity ring, a household-density mosaic outside
function tlVisit(F, mpp, capDetail){
  const T = tlTree(), cx = F.w / 2, cy = F.h / 2, R = TL.ring.ring, detailZoom = mpp < 26;
  const clat = pitch * 180 / Math.PI, clon = ((yaw * 180 / Math.PI + 540) % 360) - 180;
  const mPerDeg = EARTH_M * Math.PI / 180, ppm = F.k / EARTH_M;
  const hy = (F.h / 2 + 40) / ppm / mPerDeg, hx = (F.w / 2 + 40) / ppm / (mPerDeg * Math.max(0.05, Math.cos(clat * Math.PI / 180)));
  let lvl = 3; while (lvl > 1 && ((2 * hy) / (HOLON.STEP[lvl] / HOLON.UPD)) * ((2 * hx) / (HOLON.STEP[lvl] / HOLON.UPD)) > 1600) lvl--;
  const a0 = HOLON.at(lvl, Math.max(-89.9, clat - hy), clon - hx), a1 = HOLON.at(lvl, Math.min(89.9, clat + hy), clon + hx);
  const detail = [], agg = [], tiles = new Map(), capAgg = 6000;
  const box = (c) => { const [la, lo] = tlCentre(c); const q = camQ(...xyz(la, lo)); if (q[2] <= 0.02) return null;
    const s = HOLON.STEP[c.level] / HOLON.UPD * mPerDeg * ppm, sx = s * Math.cos(la * Math.PI / 180);
    const x = cx + F.k * q[0], y = cy - F.k * q[1], dx = Math.max(0, Math.abs(x - cx) - sx / 2), dy = Math.max(0, Math.abs(y - cy) - s / 2);
    return { x, y, hw: sx / 2, hh: s / 2, dist: Math.hypot(dx, dy), la, lo }; };
  // best-first from the centre: the nearest holons are resolved first, so the detail stops at a
  // ring (the acuity ring) when the leaf cap or the governor's radius is reached
  const heap = [], push = (e) => { heap.push(e); let k = heap.length - 1; while (k){ const p = (k - 1) >> 1; if (heap[p].bx.dist <= e.bx.dist) break; heap[k] = heap[p]; k = p; } heap[k] = e; };
  const pop = () => { const top = heap[0], last = heap.pop(); if (heap.length){ let k = 0; for (;;){ let m = 2 * k + 1; if (m >= heap.length) break; if (m + 1 < heap.length && heap[m + 1].bx.dist < heap[m].bx.dist) m++; if (heap[m].bx.dist >= last.bx.dist) break; heap[k] = heap[m]; k = m; } heap[k] = last; } return top; };
  const offer = (c) => { const b = T.budget(c); const bx = box(c); if (!bx || bx.x + bx.hw < 0 || bx.x - bx.hw > F.w || bx.y + bx.hh < 0 || bx.y - bx.hh > F.h) return; if (!b){ agg.push({ c, bx, b: 0 }); return; } push({ c, bx, b }); }; // empty land keeps its ground, undivided
  const cols = 360 * HOLON.UPD / HOLON.STEP[lvl];
  let span = a1.j - a0.j; if (span < 0) span += cols; span = Math.min(span, cols - 1);
  for (let i = a0.i; i <= a1.i; i++) for (let q = 0; q <= span; q++){ const c = HOLON.make(lvl, i, a0.j + q); if (c) offer(c); }
  let ring = 0;
  while (heap.length){
    const e = pop(), { c, bx } = e, size = Math.min(bx.hw, bx.hh) * 2;
    if (c.level >= 2){ const t = HOLON.ancestor(c, 2), k = HOLON.pack(t); if (!tiles.has(k)) tiles.set(k, t); }
    const inRing = detailZoom && bx.dist <= R && detail.length < capDetail;
    if (T.isLeaf(c)){ if (inRing){ detail.push(e); ring = Math.max(ring, bx.dist); } else agg.push(e); continue; }
    if ((inRing || size >= 28) && agg.length + heap.length < capAgg && c.level < HOLON.MAX_LEVEL) for (const k of HOLON.children(c)) offer(k);
    else agg.push(e);
  }
  TL.ringEff = detail.length >= capDetail ? ring : R;
  return { detail, agg, tiles };
}
// the arterials of an L2 cell (its own lines; generated, they fade in below 30 m a pixel)
function tlArterials(c){
  const key = HOLON.pack(c); let v = TL.mains.get(key); if (v) return v;
  const n = wasm.holon_arterials(key, tlTree().budget(c)), p = wasm.holon_buf_ptr(5);
  v = Array.from(new Float64Array(mem.buffer, p, n * 2));
  if (TL.mains.size > 4000) TL.mains.clear();
  TL.mains.set(key, v); return v;
}
// ---- per frame
function tlFrame(now){
  const ms = TL.lastT ? now - TL.lastT : 16.7; TL.lastT = now; TL.frameMs = ms;
  const F = camFrame(); TL.ring.max = Math.hypot(F.w, F.h) / 2 + 20; HOLON.governor(TL.ring, ms);
}
// night: the land fill's own factor (globe.js GF: mix(0.62, 1, smoothstep(-0.10, 0.12, sun · p))),
// so the tiling, the land and the water darken together
function tlNight(sun, la, lo){ const a = la * Math.PI / 180, o = lo * Math.PI / 180;
  const d = sun[0] * Math.cos(a) * Math.sin(o) + sun[1] * Math.sin(a) + sun[2] * Math.cos(a) * Math.cos(o);
  return 0.62 + 0.38 * sstep(-0.10, 0.12, d); }
// a holon's own lightness, ±4% from its id, so neighbours read as patchwork rather than graph paper
function tlJitter(v){ let x = Math.imul((v % 4294967296) >>> 0, 0x9e3779b1) ^ Math.imul(Math.floor(v / 4294967296) | 0, 0x85ebca77);
  x ^= x >>> 15; x = Math.imul(x, 0x2c1b3c6d); x ^= x >>> 12; return 1 + 0.08 * (((x >>> 0) / 4294967296) - 0.5); }
function tlDraw(w, h, mpp, sun){
  sun = sun || [0, 0, 1];
  // the kernel's population chain (pop_load, a few hundred ms on a throttled phone) is loaded in an idle slice as soon as
  // the grid is in, never inside a frame: the frame only asks whether it is there
  if (!TL.popOk && !TL.popSoon && tlPop() && wasm && wasm.pop_load){ TL.popSoon = true;
    (window.requestIdleCallback || ((f) => setTimeout(f, 30)))(() => { tlPopKernel(); TL.popSoon = false; if (typeof gfxDirty === "function") gfxDirty(300); }, { timeout: 3000 }); }
  TL.on = !TL.disabled && mpp < 400 && zoom >= 8 && !!TL.popOk && tlReady() && !(microStory && !(M && M.leaf !== undefined)); // the story's street is a leaf, tiled like any other
  if (TL.on && !TL.lc1 && typeof geoLoad === "function"){ TL.lc1 = true; geoLoad("lc", 1); } // the 1/8-degree land cover, once streets are near
  if (!TL.on){ TL.drawn = TL.agg = TL.pending = 0; return; }
  tlGL(); const F = camFrame(), T = tlTree();
  if (T.B.size > TL_BUDGET_CAP) T.evict(4);
  const capDetail = F.w < 800 ? 2500 : 6000;
  const { detail, agg, tiles } = tlVisit(F, mpp, capDetail);
  const aggA = sstep(400, 120, mpp), detA = sstep(26, 12, mpp);
  gl.useProgram(tlProg); gl.uniform2f(tlLoc.view, F.w, F.h);
  if (landVerts){ gl.enable(gl.STENCIL_TEST); gl.stencilFunc(gl.EQUAL, 1, 1); gl.stencilOp(gl.KEEP, gl.KEEP, gl.KEEP); }
  // the mosaic: one quad per holon in view, toned by households per km². Under a leaf that is
  // fading in (or out at the ring's edge) the same tone shows through, so the two cross-fade.
  const relA = (typeof reliefDraw === "function") ? GEO.rf : 0;
  const quads = agg.concat(detail), Q = new Float32Array(quads.length * 60); let o = 0;
  for (const { c, bx, b } of quads){
    // a holon's tone is its households per km² (the tiling's own apportionment) over its climate's
    // ground; the terrain's light is not baked per holon (that drew each holon as a flat step) but
    // multiplied over the whole mosaic from ETOPO below, continuous across holon edges
    const km2 = (bx.hw * 2 / (F.k / EARTH_M) / 1000) * (bx.hh * 2 / (F.k / EARTH_M) / 1000), T0 = tlTerrain(bx.la, bx.lo), col0 = tlGround(T0, tlTone(b / Math.max(1e-6, km2)));
    // land with no one on it (budget 0) keeps its own ground readable at night: its class colour, lit by the
    // relief's shade, never darker than 0.85 of day (the ground is real data; only the people are absent)
    const empty = !b, f = empty ? Math.max(0.85, tlNight(sun, bx.la, bx.lo)) * (relA > 0.01 ? 1 : T0.lit) * 1.6 : tlNight(sun, bx.la, bx.lo) / (relA > 0.01 ? T0.lit : 1), col = [col0[0] * f, col0[1] * f, col0[2] * f];
    const ins = bx.hw >= 20 ? 0.3 : 0; // the tiling reads by tone; a fine seam only once a holon is 40 px wide
    const P = [[bx.x - bx.hw + ins, bx.y - bx.hh + ins], [bx.x + bx.hw - ins, bx.y - bx.hh + ins], [bx.x + bx.hw - ins, bx.y + bx.hh - ins], [bx.x - bx.hw + ins, bx.y + bx.hh - ins]];
    // where land cover answers, each corner takes the blended class colour there (north-west, north-east, south-east,
    // south-west, as P runs); the holon's own tone, lighting and night stay the holon's
    let C = null;
    if (T0.lc >= 0){ const bd = HOLON.boundsDeg(c), tn = tlTone(b / Math.max(1e-6, km2)), lit = relA > 0.01 ? 1 : T0.lit;
      C = [[bd[2], bd[1]], [bd[2], bd[3]], [bd[0], bd[3]], [bd[0], bd[1]]].map(([la, lo]) => { const g = tlLcGround(la, lo); if (!g) return col;
        const g0 = g.map((v) => v * 0.42 * T0.lit), m = tlMix3(g0, T0.g1, tn), q = empty ? Math.max(0.85, tlNight(sun, bx.la, bx.lo)) * lit * 1.6 : tlNight(sun, bx.la, bx.lo) / lit; return [m[0] * q, m[1] * q, m[2] * q]; }); }
    for (const k of [0, 1, 2, 0, 2, 3]){ const cc = C ? C[k] : col; Q[o++] = P[k][0]; Q[o++] = P[k][1]; Q[o++] = 0; Q[o++] = 0; Q[o++] = 0; Q[o++] = cc[0]; Q[o++] = cc[1]; Q[o++] = cc[2]; Q[o++] = 0.9; Q[o++] = 0; }
  }
  tlScreen(Q, aggA);
  // the terrain over the mosaic, as on the land below it (relief.js multiply, inside the land stencil)
  if (relA > 0.01 && aggA > 0.01 && landVerts){ const asp = w > h ? [h / w, 1] : [1, w / h]; tlNoAttribs();
    reliefDraw(rot(), 0.86 * zoom, asp, w, h, mpp, relA * aggA, sun); gl.useProgram(tlProg); gl.uniform2f(tlLoc.view, F.w, F.h); gl.enable(gl.STENCIL_TEST); gl.stencilFunc(gl.EQUAL, 1, 1); }
  // a data map over the mosaic, under arterials, streets and buildings (incell.js dmOverMosaic), so it shows to one street;
  // at street scale (26 to 12 m a pixel) it lightens from 0.8 to 0.5 of the map's own opacity, and over the leaves' ground
  // inside the detail disc it is 0.4, so the ground carries the map's colour and the streets, homes and devices stay the subject
  if (aggA > 0.01 && landVerts && typeof dmOverMosaic === "function" && typeof DATAMAP !== "undefined" && (DATAMAP.active || DATAMAP.prev)){ const asp = w > h ? [h / w, 1] : [1, w / h]; tlNoAttribs();
    dmOverMosaic(rot(), 0.86 * zoom, asp, w, h, mpp, aggA * (0.8 - 0.3 * sstep(26, 12, mpp))); gl.useProgram(tlProg); gl.uniform2f(tlLoc.view, F.w, F.h); gl.enable(gl.STENCIL_TEST); gl.stencilFunc(gl.EQUAL, 1, 1); gl.stencilOp(gl.KEEP, gl.KEEP, gl.KEEP); }
  // arterials over the mosaic: each L2 cell's own lines, at least a pixel wide
  const artA = sstep(30, 18, mpp);
  if (artA > 0.01 && tiles.size){
    const A = [], mPerU = 111320 / HOLON.UPD;
    for (const t of tiles.values()){
      const L = tlArterials(t); if (!L.length) continue;
      const [la, lo] = tlCentre(t), s = HOLON.STEP[2], af = tlAffine(F, tlFrameOf(la, lo)); if (af.front <= 0.02) continue;
      const xc = t.j * s + s / 2, yc = t.i * s + s / 2, cl = Math.cos(la * Math.PI / 180), hw = Math.max(0.6, 6 * af.ppm), nf = tlNight(sun, la, lo);
      const hx = s / 2 * mPerU * cl, hy = s / 2 * mPerU;
      for (let k = 0; k < L.length; k += 2){
        const v = L[k], ns = L[k + 1] === 1;
        const seg = ns ? [[(v - xc) * mPerU * cl, -hy], [(v - xc) * mPerU * cl, hy]] : [[-hx, (v - yc) * mPerU], [hx, (v - yc) * mPerU]];
        const p = seg.map(([xm, ym]) => [af.a * xm + af.c * ym + af.e, af.b * xm + af.d * ym + af.f]);
        const Ln = Math.hypot(p[1][0] - p[0][0], p[1][1] - p[0][1]); if (Ln < 0.5) continue;
        const nx = -(p[1][1] - p[0][1]) / Ln * hw, ny = (p[1][0] - p[0][0]) / Ln * hw;
        for (const [q, sg] of [[0, -1], [1, -1], [1, 1], [0, -1], [1, 1], [0, 1]]) A.push(p[q][0] + nx * sg, p[q][1] + ny * sg, 0, 0, 0, 0.62 * nf, 0.59 * nf, 0.5 * nf, 0.85, 0);
      }
    }
    tlScreen(new Float32Array(A), artA);
  }
  // generation off the render path: a few ms a frame, nearest the centre first (detail is in
  // distance order already)
  const t0 = performance.now(); let pending = 0;
  for (const d of detail){ const k = HOLON.pack(d.c); if (TL.lay.has(k)) continue; pending++;
    if (performance.now() - t0 > TL_GEN_MS) continue; const e = tlBuildLeaf(d.c); TL.lay.set(k, e); TL.bytes += e.bytes; }
  TL.genMs = performance.now() - t0;
  // the detail is a disc of laid-out leaves: it ends at the nearest leaf with no layout yet (detail is in distance
  // order), so a leaf built late appears at the disc's faded edge, never as a hole or a lone patch in the mosaic (a
  // film strip of the Story's dive showed the prebuilt square standing in the mosaic, then leaves popping in around it)
  let cut = detail.length; for (let i = 0; i < detail.length; i++) if (!TL.lay.has(HOLON.pack(detail[i].c))){ cut = i; break; }
  TL.cut = cut < detail.length ? Math.max(0.5, detail[cut].bx.dist) : 0;
  // detail: one buffer and one draw per L3 tile, rebuilt only when its set of live leaves changes
  // every laid-out leaf in view is kept (most recent in the cache, safe from eviction), drawn or not: a leaf beyond the disc's
  // edge that the cache dropped would pull the edge in and drop the next (a strip caught the disc collapsing to the centre once
  // the cache passed its byte cap, and the street's neighbourhood drawn as bare ground)
  const live = new Set(), keep = new Set(), byTile = new Map();
  for (let i = 0; i < detail.length; i++){ const k = HOLON.pack(detail[i].c), e = TL.lay.get(k); if (!e) continue;
    TL.lay.delete(k); TL.lay.set(k, e); keep.add(e);
    if (i < cut){ live.add(e); if (!byTile.has(e.tile)) byTile.set(e.tile, []); byTile.get(e.tile).push(e); } }
  TL.live = live; // the leaves this frame draws in detail (read by tools/page/story_zoom's painted-buildings check)
  const homePx = 10 * F.k / EARTH_M, dpr = window.devicePixelRatio || 1, mh = 0.55 / Math.min(2, dpr);
  const Re = TL.cut ? Math.min(TL.ringEff || TL.ring.ring, TL.cut) : (TL.ringEff || TL.ring.ring), edge = Re < Math.hypot(F.w, F.h) / 2;
  TL.re = edge ? Re : 0; // the disc's drawn radius this frame (0: the whole screen), read by tools/page/story_zoom
  if (detA > 0.01 && byTile.size){
    const kA = [detA, detA * sstep(1.2, 3, homePx * 0.25), detA, detA * sstep(0.6, 2, homePx)];
    gl.uniform4f(tlLoc.minHalf, 0, mh + 0.5, mh, 0); gl.uniform3f(tlLoc.ring, F.w / 2, F.h / 2, edge ? Re : 0);
    const pass = (ka) => { gl.uniform4f(tlLoc.kindA, ka[0], ka[1], ka[2], ka[3]);
    for (const [tk, list] of byTile){
      list.sort((a, b) => a.id - b.id); const key = list.map(e => e.id).join(",");
      let bt = TL.batch.get(tk);
      if (!bt){ const [la, lo] = tlCentre(list[0].t); bt = { vbo: gl.createBuffer(), key: "", n: 0, fr: tlFrameOf(la, lo) }; TL.batch.set(tk, bt); }
      if (bt.key !== key){
        let len = 0; for (const e of list) len += e.data.length; const all = new Float32Array(len); let at = 0;
        for (const e of list){ all.set(e.data, at); at += e.data.length; }
        gl.bindBuffer(gl.ARRAY_BUFFER, bt.vbo); gl.bufferData(gl.ARRAY_BUFFER, all, gl.DYNAMIC_DRAW); bt.key = key; bt.n = len / 10;
      }
      bt.used = frame;
      const A = tlAffine(F, bt.fr); if (A.front <= 0.02) continue;
      if (bt.night === undefined || frame % 30 === 0){ const [la, lo] = tlCentre(list[0].t); bt.night = tlNight(sun, la, lo); bt.terr = tlTerrain(la, lo); }
      gl.uniform1f(tlLoc.dim, bt.night); gl.uniform1f(tlLoc.terr, 1);
      { const [la, lo] = tlCentre(list[0].t), no = tlNoiseOrigin(la, lo); gl.uniform4f(tlLoc.no, no[0], no[1], no[2], 1);
        gl.uniform1f(tlLoc.tex, TL_TEX.on ? (typeof DATAMAP !== "undefined" && DATAMAP.active ? 0.5 : 1) : 0); } // halved under a data map
      gl.uniform3fv(tlLoc.g0, bt.terr.g0); gl.uniform3fv(tlLoc.g1, bt.terr.g1); gl.uniform3fv(tlLoc.roof, bt.terr.roof);
      gl.bindBuffer(gl.ARRAY_BUFFER, bt.vbo); tlAttribs();
      gl.uniform3f(tlLoc.X, A.a, A.c, A.e); gl.uniform3f(tlLoc.Y, A.b, A.d, A.f); gl.uniform1f(tlLoc.ppm, A.ppm);
      gl.drawArrays(gl.TRIANGLES, 0, bt.n);
    } };
    // with a data map on, the leaves' ground first (marking stencil bit 2 where it lands), the map's tint over that ground
    // (incell.js dmOverMosaic), then the streets, homes and roofs over the tint, so they stay readable at one street
    const mapOn = landVerts && typeof dmOverMosaic === "function" && typeof DATAMAP !== "undefined" && (DATAMAP.active || DATAMAP.prev);
    if (!mapOn) pass(kA);
    else {
      gl.stencilMask(2); gl.stencilFunc(gl.EQUAL, 3, 1); gl.stencilOp(gl.KEEP, gl.KEEP, gl.REPLACE);
      pass([kA[0], 0, 0, 0]);
      gl.stencilOp(gl.KEEP, gl.KEEP, gl.KEEP); gl.stencilFunc(gl.EQUAL, 3, 3); tlNoAttribs();
      if (typeof DM_RING !== "undefined"){ const d = gl.drawingBufferWidth / Math.max(1, F.w); DM_RING[0] = F.w / 2 * d; DM_RING[1] = F.h / 2 * d; DM_RING[2] = edge ? Re * d : 0; }
      dmOverMosaic(rot(), 0.86 * zoom, w > h ? [h / w, 1] : [1, w / h], w, h, mpp, detA * 0.4); // translucent: the street stays the subject
      if (typeof DM_RING !== "undefined") DM_RING[2] = 0;
      gl.useProgram(tlProg); gl.uniform2f(tlLoc.view, F.w, F.h); gl.uniform4f(tlLoc.minHalf, 0, mh + 0.5, mh, 0); gl.uniform3f(tlLoc.ring, F.w / 2, F.h / 2, edge ? Re : 0);
      gl.stencilFunc(gl.EQUAL, 1, 1);
      pass([0, kA[1], kA[2], kA[3]]);
      gl.clearStencil(0); gl.clear(gl.STENCIL_BUFFER_BIT); gl.stencilMask(1); // bit 2 back to 0 (the frame clears bit 1 only)
    }
  }
  tlNoAttribs(); gl.disable(gl.STENCIL_TEST);
  // tile buffers not drawn for a while go; leaves are an LRU by holon id under a byte budget
  for (const [tk, bt] of TL.batch) if (frame - (bt.used || 0) > 120){ gl.deleteBuffer(bt.vbo); TL.batch.delete(tk); }
  for (const [k, e] of TL.lay){ if (TL.bytes <= TL_BYTE_CAP) break; if (keep.has(e)) continue; TL.bytes -= e.bytes; TL.lay.delete(k); }
  TL.drawn = live.size; TL.agg = agg.length; TL.pending = pending; TL.draws = byTile.size;
}
function tlMetrics(){
  const el = document.getElementById("s_tiling"); if (!el || frame % 15) return;
  const s = TL.stats(); el.textContent = s.on ? `ring ${s.ring}/${s.ringMax} px · ${s.fps} fps · ${s.leaves} leaves drawn, ${s.aggregate} aggregate, ${s.pending} queued · ${s.cached} cached (${s.mb} MB)` : "off (zoom to region scale)";
}
// the 1/2-degree land cover at load (18 kB); the street ground reads it through tlTerrain
if (typeof geoLoad === "function"){ geoLoad("lc", 0); setTimeout(() => { geoLoad("pop", 0); geoLoad("pop", 1); }, 1500); } // people at 1/8 degree (about 0.8 MB) condition the street tier
