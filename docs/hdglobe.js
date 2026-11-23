// The globe's own passes drawn by holon-draw (CIRISHolon engine/crates/holon-draw, vendored as docs/holon/draw.js), one
// layer at a time: CEWP#24, "the twin drawn exclusively by holon-draw". Each layer here replaces one of the twin's own WebGL
// programs; the program stays, and draws whenever its layer is off or holon-draw is not loaded yet.
//
//   fade   the zoom hand-off fade (globe.js zrFade): a screen quad over the frame, the page's background colour at 1 - w
//   veil   a data map's veil over the land (incell.js incellVeil): a screen quad inside the land stencil
//   lines  coasts, borders, rivers and ETOPO shores (gfx.js gfxLines): world lines, a width in pixels, the GLOBE light
//   land   the land's ground and the population tiles (globe.js pG's two colour passes): TRIANGLE records, the GLOBE light,
//          inside the land stencil, which the twin's own fan still writes
//
// ?hdg= picks them: a comma list of those names, ?hdg=1 every one, ?hdg=0 none. Without it, HDG_DEFAULT: the layers whose
// page gate (tools/page/hd_globe/run.py) held under stakes written before its run. What is data and what is decoration does
// not change: each layer draws what its own program drew, from the same numbers.
const HDG_ALL = ["fade", "veil", "lines", "land"];
const HDG_DEFAULT = [];
const HDG = (() => {
  const m = typeof location !== "undefined" && /[?&]hdg=([^&]*)/.exec(location.search);
  let on = HDG_DEFAULT;
  if (m) on = m[1] === "1" ? HDG_ALL : m[1] === "0" ? [] : decodeURIComponent(m[1]).split(",").filter((x) => HDG_ALL.includes(x));
  return { on: new Set(on), drawer: null, mod: null, loading: false, err: null, drawn: { fade: 0, veil: 0, lines: 0, land: 0 },
    vp: new Float32Array(16), sq: null, sqP: new Float32Array(32), sets: new Map(), land: null, landP: [new Float32Array(32), new Float32Array(32)] };
})();
// The drawer: the street tier's own if it has one by the first use here (tiling.js TL_HD), else one made here; then always
// that one, since the resident buffers made here are that drawer's.
function hdgDrawer(){
  if (HDG.drawer) return HDG.drawer;
  if (typeof TL_HD !== "undefined" && TL_HD.drawer){ HDG.mod = TL_HD.mod; HDG.drawer = TL_HD.drawer; return HDG.drawer; }
  if (!HDG.loading && !HDG.err){ HDG.loading = true;
    import("./holon/draw.js?v=4").then((mod) => { HDG.mod = mod; HDG.drawer = mod.createDrawer(gl, {}); if (typeof gfxDirty === "function") gfxDirty(600); })
      .catch((e) => { HDG.err = String(e && e.message || e); console.warn("holon-draw (globe): " + HDG.err + "; the twin's own programs draw"); }); }
  return null;
}
function hdgUse(layer){ return HDG.on.has(layer) ? hdgDrawer() : null; }
// draw.js leaves the depth test and culling on and the blend as its GL STATE says; the globe's passes run with neither
function hdgRestore(){ gl.disable(gl.DEPTH_TEST); gl.disable(gl.CULL_FACE); gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA); }
// one 52-byte record (draw.js's THE RECORD), into a DataView at byte o
function hdgRec(dv, o, kind, pos, size, aux, angle, r, g, b, a, variant){
  dv.setFloat32(o, pos[0], true); dv.setFloat32(o + 4, pos[1], true); dv.setFloat32(o + 8, pos[2], true);
  dv.setFloat32(o + 12, size[0], true); dv.setFloat32(o + 16, size[1], true); dv.setFloat32(o + 20, size[2], true);
  dv.setFloat32(o + 24, aux[0], true); dv.setFloat32(o + 28, aux[1], true); dv.setFloat32(o + 32, aux[2], true);
  dv.setFloat32(o + 36, angle, true); dv.setUint8(o + 40, r); dv.setUint8(o + 41, g); dv.setUint8(o + 42, b); dv.setUint8(o + 43, a);
  dv.setUint32(o + 44, 0, true); dv.setUint8(o + 48, kind); dv.setUint8(o + 49, 0); dv.setUint8(o + 50, 0); dv.setUint8(o + 51, variant || 0);
}
// the globe's camera as one matrix: clip = (S asp.x (R p).x, S asp.y (R p).y, -0.5 (R p).z, 1), column-major (R as globe.js
// hands it to its shaders, uniformMatrix3fv without transpose)
function hdgVP(R, S, asp){
  const V = HDG.vp, ax = S * asp[0], ay = S * asp[1]; V.fill(0);
  V[0] = ax * R[0]; V[1] = ay * R[1]; V[2] = -0.5 * R[2];
  V[4] = ax * R[3]; V[5] = ay * R[4]; V[6] = -0.5 * R[5];
  V[8] = ax * R[6]; V[9] = ay * R[7]; V[10] = -0.5 * R[8]; V[15] = 1;
  return V;
}
const hdgOpts = (o) => Object.assign({ viewport: [0, 0, gl.drawingBufferWidth, gl.drawingBufferHeight], depthTest: false }, o);
// ---- fade and veil: one screen quad over the frame, its colour exact (DIM, a float) and its alpha FADE
function hdgScreenQuad(D, rgb, a){
  const w = gl.drawingBufferWidth, h = gl.drawingBufferHeight;
  if (!HDG.sq){ HDG.sq = { bytes: new Uint8Array(52), w: 0, h: 0 }; }
  const Q = HDG.sq;
  if (Q.w !== w || Q.h !== h){ hdgRec(new DataView(Q.bytes.buffer), 0, 1, [w / 2, h / 2, 0], [w + 2, h + 2, 0], [0, 0, 0], 0, 255, 255, 255, 255, 0); Q.w = w; Q.h = h; }
  const P = HDG.sqP; P.set(HDG.mod.PARAMS_NEUTRAL); P[0] = rgb[0]; P[1] = rgb[1]; P[2] = rgb[2]; P[3] = a;
  D.draw([{ kind: 1, screen: true, count: 1, bytes: Q.bytes, params: P }], null, hdgOpts());
  hdgRestore();
}
function hdgFade(a){ const D = hdgUse("fade"); if (!D) return false; hdgScreenQuad(D, [0.043, 0.051, 0.07], Math.min(1, a)); HDG.drawn.fade++; return true; }
function hdgVeil(a){ const D = hdgUse("veil"); if (!D) return false; hdgScreenQuad(D, [0.050, 0.057, 0.072], a); HDG.drawn.veil++; return true; }
// ---- lines: each line set's segments (gfx.js gfxLines reads the same buffers) as world lines in ONE resident buffer per set,
// made on the set's first draw and released with the set (geoFree). Colour and opacity are the kind's DIM and FADE; the
// width is FLOOR, in device pixels, on a line of no width of its own; drawn as tubes (a round tube's width on screen is its
// diameter whichever way it runs) with no depth test. The GLOBE light is the twin's line shader's (gfx.js AF): day by
// smoothstep(-0.10, 0.12) of the sun's height, colour x mix(0.48, 1, day), alpha x mix(0.6, 1, day), the limb
// smoothstep(-0.02, 0.12).
function hdgSet(D, G){
  let H = HDG.sets.get(G); if (H) return H;
  const lists = [G.lines[0], G.lines[1], G.lines[2], G.shoreK]; let n = 0;
  for (const L of lists) if (L && L.d) n += L.d.length / 6;
  const bytes = new Uint8Array(Math.max(1, n) * 52), dv = new DataView(bytes.buffer); let at = 0; const spans = [];
  const z3 = [0, 0, 0], a = [0, 0, 0], b = [0, 0, 0];
  for (const L of lists){ const first = at; if (L && L.d){ const d = L.d;
      for (let i = 0; i + 5 < d.length; i += 6){ a[0] = d[i]; a[1] = d[i + 1]; a[2] = d[i + 2]; b[0] = d[i + 3]; b[1] = d[i + 4]; b[2] = d[i + 5]; hdgRec(dv, at * 52, 2, a, z3, b, 0, 255, 255, 255, 255, 0); at++; } }
    spans.push([first, at - first]); }
  H = { h: n ? D.resident(bytes) : 0, spans, D, params: [0, 1, 2, 3].map(() => new Float32Array(32)) };
  HDG.sets.set(G, H); return H;
}
function hdgFreeSet(G){ const H = HDG.sets.get(G); if (!H) return; if (H.h) H.D.release(H.h); HDG.sets.delete(G); }
function hdgLines(R, S, asp, gsets, LS, nk, widthsPx, sun){
  const D = hdgUse("lines"); if (!D) return false;
  for (const G of gsets) if (!G.lines[0].d) return false; // a set uploaded before the switch was read keeps no source: its own program draws
  const P0 = HDG.mod.PARAMS_NEUTRAL, PM = HDG.mod.PARAM, B = [], ppu = S * asp[1] * gl.drawingBufferHeight / 2;
  const globe = { sun, day: [-0.10, 0.12], rgb: [0.48, 1], alpha: [0.6, 1], limb: [-0.02, 0.12] };
  for (let k = 0; k < nk; k++) for (const G of gsets){ const H = hdgSet(D, G), [first, count] = H.spans[k]; if (!count) continue;
    const P = H.params[k]; P.set(P0); P[0] = LS[k][0][0]; P[1] = LS[k][0][1]; P[2] = LS[k][0][2]; P[3] = LS[k][1] * GEO.rf;
    P[PM.FLOOR] = widthsPx[k] * GFX.k / 2;
    B.push({ kind: 2, handle: H.h, first, count, params: P, globe, pixelsPerUnit: ppu }); }
  if (B.length){ D.draw(B, hdgVP(R, S, asp), hdgOpts({ flatLines: false })); hdgRestore(); HDG.drawn.lines++; }
  return true;
}
// ---- land: the ground (the whole sphere in 3-degree quads) and the population grid's tiles (brighter where more people
// live) as TRIANGLE records in one resident buffer, made once; the tiles' density tone is RAMP t (the triangle's angle),
// between globe.js GF's two colours. The GLOBE light is GF's: colour x (0.56 + 0.84 smoothstep(-0.10, 0.10, s) (0.45 + 0.55
// max(s, 0))), i.e. rgb [0.56, 1.40] with a Lambert share of 0.55; the limb smoothstep(0, 0.06). Inside the land stencil,
// which the caller has set.
function hdgLand(R, S, asp, sun, groundA, tilesA){
  const D = hdgUse("land"); if (!D || !LAND_SRC) return false;
  if (!HDG.land){ const g = LAND_SRC.ground, p = LAND_SRC.tiles, t = LAND_SRC.t, ng = g.length / 9, np = p.length / 9;
    const bytes = new Uint8Array((ng + np) * 52), dv = new DataView(bytes.buffer), A = [0, 0, 0], Bv = [0, 0, 0], C = [0, 0, 0];
    const put = (src, i, at, tt) => { A[0] = src[i]; A[1] = src[i + 1]; A[2] = src[i + 2]; Bv[0] = src[i + 3]; Bv[1] = src[i + 4]; Bv[2] = src[i + 5]; C[0] = src[i + 6]; C[1] = src[i + 7]; C[2] = src[i + 8];
      hdgRec(dv, at * 52, 13, A, Bv, C, tt, 255, 255, 255, 255, 0x40); };
    for (let k = 0; k < ng; k++) put(g, 9 * k, k, 0);
    for (let k = 0; k < np; k++) put(p, 9 * k, ng + k, t[3 * k]);
    HDG.land = { h: D.resident(bytes), ng, np }; }
  const L = HDG.land, PM = HDG.mod.PARAM, ppu = S * asp[1] * gl.drawingBufferHeight / 2;
  const globe = { sun, day: [-0.10, 0.10], rgb: [0.56, 1.40], lambert: 0.55, limb: [0, 0.06] };
  const mk = (P, a) => { P.set(HDG.mod.PARAMS_NEUTRAL); P[PM.FADE] = a; P.set([0.22, 0.24, 0.19], PM.RAMP0); P.set([0.31, 0.33, 0.26], PM.RAMP1); return P; };
  D.draw([{ kind: 13, handle: L.h, first: 0, count: L.ng, params: mk(HDG.landP[0], groundA), globe, pixelsPerUnit: ppu },
          { kind: 13, handle: L.h, first: L.ng, count: L.np, params: mk(HDG.landP[1], tilesA), globe, pixelsPerUnit: ppu }], hdgVP(R, S, asp), hdgOpts());
  hdgRestore(); HDG.drawn.land++; return true;
}
// the land's sources, kept by globe.js initLand only when the land layer is on (its own program needs only the GL buffers)
let LAND_SRC = null;
