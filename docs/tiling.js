// Street tiling: the holon tree (cewp_sim::tiling, FSD §4.6.6) drawn over settled land out to the
// acuity ring. The page keeps the tree; the kernel splits households and lays out each leaf. Every
// street and home here is generated: the only real inputs are the coarse population grid, the 50m
// land mask and the populated-place weights, and real map layers have faded out before streets
// fade in (below 26 m a pixel).
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
const tlReady = () => !!(wasm && wasm.holon_split && wasm.holon_arterials && wasm.micro_build_leaf && GEO.places && tlLand());

// ---- inputs: households per L0 cell, a weight per child
function tlCentre(c){ const b = HOLON.boundsDeg(c); return [(b[0] + b[2]) / 2, (b[1] + b[3]) / 2]; }
function tlLandK(c, n, kern){ // land samples on an n×n lattice inside the cell; with kern, the
  // place kernel summed over the land samples instead of a count
  const b = HOLON.boundsDeg(c); let k = 0;
  for (let a = 0; a < n; a++) for (let q = 0; q < n; q++){ const la = b[0] + (a + 0.5) / n * (b[2] - b[0]), lo = b[1] + (q + 0.5) / n * (b[3] - b[1]);
    if (TL_LAND.inside(la, lo)) k += kern ? 40 + tlKernel(la, lo) : 1; }
  return k;
}
// the grid cell holding a point, or else the nearest within 1.6° (the grid stops short of many coasts)
function tlCellNear(la, lo){
  const c = cellAt(la, lo); if (c >= 0) return c;
  const G = initPopGrid(), cl = Math.cos(la * Math.PI / 180); let best = -1, bd = 1.6 * 1.6;
  for (let k = 0; k < G.nCells; k++){ const dy = G.cellLat[k] - la, dx = (((G.cellLon[k] - lo + 540) % 360) - 180) * cl, d = dx * dx + dy * dy; if (d < bd){ bd = d; best = k; } }
  return best;
}
function tlRoot(c){ // households in an L0 cell: the population grid's density at each land sample × its share of the area
  const key = HOLON.pack(c); let v = TL.l0.get(key); if (v !== undefined) return v;
  const b = HOLON.boundsDeg(c), n = 6, [la] = tlCentre(c), km = 1.5 * 111.32, cellKm2 = km * km * Math.cos(la * Math.PI / 180) / (n * n);
  let people = 0;
  for (let a = 0; a < n; a++) for (let q = 0; q < n; q++){
    const y = b[0] + (a + 0.5) / n * (b[2] - b[0]), x = b[1] + (q + 0.5) / n * (b[3] - b[1]);
    if (!TL_LAND.inside(y, x)) continue; const g = tlCellNear(y, x); if (g >= 0) people += cellDensity(g) * cellKm2;
  }
  v = Math.round(people / TL_PPH); TL.l0.set(key, v); return v;
}
function tlPlaceBins(){
  if (TL.placeBins) return TL.placeBins;
  const m = new Map();
  for (const p of GEO.places){ const k = (Math.floor(p.la) + 90) * 360 + ((Math.floor(p.lo) + 360) % 360); if (!m.has(k)) m.set(k, []); m.get(k).push(p); }
  return (TL.placeBins = m);
}
// populated-place kernel: each place a Gaussian of its population, σ = 1.5 km + 4 km·√(pop / 1M)
function tlKernel(la, lo){
  const bins = tlPlaceBins(), cl = Math.cos(la * Math.PI / 180), dl = Math.ceil(1 / Math.max(0.1, cl)); let s = 0;
  for (let a = Math.floor(la) - 1; a <= Math.floor(la) + 1; a++) for (let o = Math.floor(lo) - dl; o <= Math.floor(lo) + dl; o++){
    const L = bins.get((a + 90) * 360 + ((o % 360) + 360) % 360); if (!L) continue;
    for (const p of L){ const dy = (p.la - la) * 111.32, dx = ((((p.lo - lo) % 360) + 540) % 360 - 180) * 111.32 * cl, sg = 1.5 + 4 * Math.sqrt(p.pop / 1e6);
      const d2 = dx * dx + dy * dy; if (d2 < 25 * sg * sg) s += p.pop / 1000 * Math.exp(-d2 / (2 * sg * sg)); }
  }
  return s;
}
function tlExt(c){ // Σ over land samples of (40 + place kernel), per sample; an integer (the kernel then multiplies by its noise)
  const n = c.level <= 2 ? 3 : c.level === 3 ? 2 : 1;
  return Math.min(4e9, Math.round(tlLandK(c, n, true) * 9 / (n * n)));
}
function tlSplit(p, pb, ext){
  const ep = wasm.holon_ext_ptr(); new Uint32Array(mem.buffer, ep, 25).set(ext);
  const n = wasm.holon_split(HOLON.pack(p), pb); const op = wasm.holon_buf_ptr(0);
  return Array.from(new Float64Array(mem.buffer, op, n));
}
function tlTree(){ if (!TL.T) TL.T = HOLON.tree(tlRoot, tlExt, tlSplit, TL_TARGET); return TL.T; }

// one tone ramp for the mosaic and the leaves' ground: households per km², 10 → 0, ~3,000 → 1
const tlTone = (d) => Math.max(0, Math.min(1, (Math.log10(Math.max(1, d)) - 1) / 2.5));
const tlRamp = (t) => [0.11 + 0.2 * t, 0.135 + 0.165 * t, 0.13 + 0.1 * t];
const TL_ROAD = [[0.47, 0.49, 0.53], [0.47, 0.49, 0.52], [0.45, 0.47, 0.44], [0.5, 0.48, 0.41]];
// A leaf's geometry, in the metres of its L3 tile's frame (one affine per tile, one draw per tile).
// Each vertex: position, normal, half width, colour, kind (0 ground, 1 casing, 2 road, 3 building).
function tlBuildLeaf(leaf){
  const T = tlTree(), ctx = T.context(leaf);
  const cp = wasm.holon_ctx_ptr(); new Float64Array(mem.buffer, cp, 32).set(ctx);
  const id = HOLON.pack(leaf); wasm.holon_layout(id);
  const buf = (i) => { const p = wasm.holon_buf_ptr(i), n = wasm.holon_buf_len(i); return new Float64Array(mem.buffer, p, n).slice(); };
  const R = buf(1), B = buf(2), meta = buf(4), w = meta[0], h = meta[1], cls = meta[3] | 0;
  const t = HOLON.ancestor(leaf, 3), [laL, loL] = tlCentre(leaf), [laT, loT] = tlCentre(t);
  const cT = Math.cos(laT * Math.PI / 180), sx = cT / Math.cos(laL * Math.PI / 180), ox = (loL - loT) * 111320 * cT, oy = (laL - laT) * 111320;
  const V = []; const vert = (x, y, nx, ny, wd, c, a, k) => V.push(ox + x * sx, oy + y, nx, ny, wd, c[0], c[1], c[2], a, k);
  const quad = (p, c, a, k) => { for (const q of [0, 1, 2, 0, 2, 3]) vert(p[q][0], p[q][1], 0, 0, 0, c, a, k); };
  quad([[-w / 2, -h / 2], [w / 2, -h / 2], [w / 2, h / 2], [-w / 2, h / 2]], tlRamp(tlTone(meta[2] / Math.max(1e-6, w * h / 1e6))), 0.9, 0);
  const onSide = (x, y) => Math.abs(Math.abs(x) - w / 2) < 1e-6 || Math.abs(Math.abs(y) - h / 2) < 1e-6;
  const road = (r, extra, c, a, k) => {
    let [x0, y0, x1, y1, wd] = [R[r], R[r + 1], R[r + 2], R[r + 3], R[r + 4] / 2 + extra];
    const L = Math.hypot(x1 - x0, y1 - y0); if (L < 1e-6) return; const ux = (x1 - x0) / L, uy = (y1 - y0) / L;
    // a street end inside the leaf gets a square cap; an end on a side meets the neighbour's street flush
    if (!onSide(x0, y0)){ x0 -= ux * wd; y0 -= uy * wd; } if (!onSide(x1, y1)){ x1 += ux * wd; y1 += uy * wd; }
    const nx = -uy, ny = ux;
    for (const [px, py, s] of [[x0, y0, -1], [x1, y1, -1], [x1, y1, 1], [x0, y0, -1], [x1, y1, 1], [x0, y0, 1]]) vert(px, py, nx * s, ny * s, wd, c, a, k);
  };
  for (let r = 0; r < R.length; r += 5) road(r, 0.8, [0.1, 0.114, 0.133], 0.85, 1); // the casing stays inside the metre every building keeps clear
  for (let r = 0; r < R.length; r += 5) road(r, 0, R[r + 4] >= 11 ? [0.62, 0.59, 0.5] : TL_ROAD[cls], 1, 2);
  for (let b = 0; b < B.length; b += 8){
    const cx = B[b], cy = B[b + 1], bw = B[b + 2] / 2, bd = B[b + 3] / 2, ca = Math.cos(B[b + 4]), sa = Math.sin(B[b + 4]), units = B[b + 6];
    const P = [[-bw, -bd], [bw, -bd], [bw, bd], [-bw, bd]].map(([u, v]) => [cx + ca * u - sa * v, cy + sa * u + ca * v]);
    quad(P, units > 0 ? [0.36, 0.34, 0.3] : [0.24, 0.235, 0.22], 1, 3);
  }
  const data = new Float32Array(V);
  return { id, tile: HOLON.pack(t), t, data, bytes: data.byteLength, hh: meta[2] };
}
function tlFrameOf(la, lo){ const a = la * Math.PI / 180, o = lo * Math.PI / 180;
  return { c: [Math.cos(a) * Math.sin(o), Math.sin(a), Math.cos(a) * Math.cos(o)], E: [Math.cos(o), 0, -Math.sin(o)], N: [-Math.sin(a) * Math.sin(o), Math.cos(a), -Math.sin(a) * Math.cos(o)] }; }

// ---- the GL program: metres in, an affine per tile to CSS px, streets kept at least a pixel wide,
// alpha per kind, and a radial fade at the acuity ring's edge
const TLV = `attribute vec2 p;attribute vec2 n;attribute float wd;attribute vec4 c;attribute float k;uniform vec3 uX;uniform vec3 uY;uniform vec2 uView;uniform float uPpm;uniform vec4 uMinHalf;uniform vec4 uKindA;uniform vec3 uRing;varying vec4 vc;
void main(){vec4 sel=vec4(equal(vec4(k),vec4(0.0,1.0,2.0,3.0)));vec2 q=p+n*max(wd,dot(uMinHalf,sel)/uPpm);float px=uX.x*q.x+uX.y*q.y+uX.z;float py=uY.x*q.x+uY.y*q.y+uY.z;
float f=uRing.z>0.0?1.0-smoothstep(0.85*uRing.z,uRing.z,length(vec2(px,py)-uRing.xy)):1.0;
gl_Position=vec4(px/uView.x*2.0-1.0,1.0-py/uView.y*2.0,0.0,1.0);vc=vec4(c.rgb,c.a*dot(uKindA,sel)*f);}`;
const TLF = `precision mediump float;uniform float uDim;varying vec4 vc;void main(){if(vc.a<0.004)discard;gl_FragColor=vec4(vc.rgb*uDim,vc.a);}`;
let tlProg = null, tlLoc = null, tlAggBuf = null;
function tlGL(){
  if (tlProg) return; tlProg = prog(TLV, TLF); tlAggBuf = gl.createBuffer();
  const u = (n) => gl.getUniformLocation(tlProg, n), a = (n) => gl.getAttribLocation(tlProg, n);
  tlLoc = { p: a("p"), n: a("n"), wd: a("wd"), c: a("c"), k: a("k"), X: u("uX"), Y: u("uY"), view: u("uView"), ppm: u("uPpm"), minHalf: u("uMinHalf"), kindA: u("uKindA"), ring: u("uRing"), dim: u("uDim") };
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
  gl.uniform4f(tlLoc.kindA, alpha, 0, 0, 0); gl.uniform3f(tlLoc.ring, 0, 0, 0); gl.uniform1f(tlLoc.dim, 1); // night is in the colours
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
  const offer = (c) => { const b = T.budget(c); if (!b) return; const bx = box(c); if (!bx || bx.x + bx.hw < 0 || bx.x - bx.hw > F.w || bx.y + bx.hh < 0 || bx.y - bx.hh > F.h) return; push({ c, bx, b }); };
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
  TL.on = !TL.disabled && tlReady() && !(microStory && !(M && M.leaf !== undefined)) && mpp < 400 && zoom >= 8; // the story's street is a leaf, tiled like any other
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
  const quads = agg.concat(detail), Q = new Float32Array(quads.length * 60); let o = 0;
  for (const { c, bx, b } of quads){
    const km2 = (bx.hw * 2 / (F.k / EARTH_M) / 1000) * (bx.hh * 2 / (F.k / EARTH_M) / 1000), col0 = tlRamp(tlTone(b / Math.max(1e-6, km2)));
    const f = tlJitter(HOLON.pack(c)) * tlNight(sun, bx.la, bx.lo), col = [col0[0] * f, col0[1] * f, col0[2] * f];
    const ins = bx.hw >= 20 ? 0.3 : 0; // the tiling reads by tone; a fine seam only once a holon is 40 px wide
    const P = [[bx.x - bx.hw + ins, bx.y - bx.hh + ins], [bx.x + bx.hw - ins, bx.y - bx.hh + ins], [bx.x + bx.hw - ins, bx.y + bx.hh - ins], [bx.x - bx.hw + ins, bx.y + bx.hh - ins]];
    for (const k of [0, 1, 2, 0, 2, 3]){ Q[o++] = P[k][0]; Q[o++] = P[k][1]; Q[o++] = 0; Q[o++] = 0; Q[o++] = 0; Q[o++] = col[0]; Q[o++] = col[1]; Q[o++] = col[2]; Q[o++] = 0.9; Q[o++] = 0; }
  }
  tlScreen(Q, aggA);
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
  // detail: one buffer and one draw per L3 tile, rebuilt only when its set of live leaves changes
  const live = new Set(), byTile = new Map();
  for (const d of detail){ const k = HOLON.pack(d.c), e = TL.lay.get(k); if (!e) continue;
    TL.lay.delete(k); TL.lay.set(k, e); live.add(e); if (!byTile.has(e.tile)) byTile.set(e.tile, []); byTile.get(e.tile).push(e); }
  const homePx = 10 * F.k / EARTH_M, dpr = window.devicePixelRatio || 1, mh = 0.55 / Math.min(2, dpr);
  const Re = TL.ringEff || TL.ring.ring, edge = Re < Math.hypot(F.w, F.h) / 2;
  if (detA > 0.01 && byTile.size){
    gl.uniform4f(tlLoc.kindA, detA, detA * sstep(1.2, 3, homePx * 0.25), detA, detA * sstep(0.6, 2, homePx));
    gl.uniform4f(tlLoc.minHalf, 0, mh + 0.5, mh, 0); gl.uniform3f(tlLoc.ring, F.w / 2, F.h / 2, edge ? Re : 0);
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
      if (bt.night === undefined || frame % 30 === 0){ const [la, lo] = tlCentre(list[0].t); bt.night = tlNight(sun, la, lo); }
      gl.uniform1f(tlLoc.dim, bt.night);
      gl.bindBuffer(gl.ARRAY_BUFFER, bt.vbo); tlAttribs();
      gl.uniform3f(tlLoc.X, A.a, A.c, A.e); gl.uniform3f(tlLoc.Y, A.b, A.d, A.f); gl.uniform1f(tlLoc.ppm, A.ppm);
      gl.drawArrays(gl.TRIANGLES, 0, bt.n);
    }
  }
  tlNoAttribs(); gl.disable(gl.STENCIL_TEST);
  // tile buffers not drawn for a while go; leaves are an LRU by holon id under a byte budget
  for (const [tk, bt] of TL.batch) if (frame - (bt.used || 0) > 120){ gl.deleteBuffer(bt.vbo); TL.batch.delete(tk); }
  for (const [k, e] of TL.lay){ if (TL.bytes <= TL_BYTE_CAP) break; if (live.has(e)) continue; TL.bytes -= e.bytes; TL.lay.delete(k); }
  TL.drawn = live.size; TL.agg = agg.length; TL.pending = pending; TL.draws = byTile.size;
}
function tlMetrics(){
  const el = document.getElementById("s_tiling"); if (!el || frame % 15) return;
  const s = TL.stats(); el.textContent = s.on ? `ring ${s.ring}/${s.ringMax} px · ${s.fps} fps · ${s.leaves} leaves drawn, ${s.aggregate} aggregate, ${s.pending} queued · ${s.cached} cached (${s.mb} MB)` : "off (zoom to region scale)";
}
