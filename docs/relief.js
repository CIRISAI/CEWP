// Terrain on the land: Natural Earth's shaded relief (public domain) and NOAA ETOPO 2022 elevation
// (public domain), built into JPEG tiles by tools/geo/build_dem.py. Each pixel: R = shaded relief
// (206 = flat ground), G = elevation code round(255·sqrt(clamp(z, 0, 6500) / 6500)).
// Levels: z0 the globe (2048 x 1024), z1 45° tiles (1024 px), z2 15° tiles (900 px, 1/60°).
// The relief is a 2x multiply on the land already drawn, inside the land stencil, so the land
// keeps its colours and the shading only darkens slopes away from the light and lifts the ones
// facing it. It fades out with the other real layers (by 30 m a pixel), before any generated
// street fades in: real geography stops at region scale.
const RELIEF = { idx: null, pIdx: false, tiles: new Map(), cap: { 1: 10, 2: 16 }, used: "-",
  stats(){ let n = 0; for (const t of this.tiles.values()) if (t.tex) n++; return { used: this.used, textures: n }; } };
const RV = `attribute vec3 p;attribute vec2 uv;uniform mat3 R;uniform float S;uniform vec2 asp;varying vec2 vUv;varying float vz;void main(){vec3 q=R*p;vz=q.z;vUv=uv;gl_Position=vec4(q.x*S*asp.x,q.y*S*asp.y,0.9,1.0);}`;
const RF = `precision mediump float;uniform sampler2D T;uniform float uA;varying vec2 vUv;varying float vz;
void main(){if(vz<0.0)discard;vec4 t=texture2D(T,vUv);float sh=clamp(1.0+(t.r/0.808-1.0)*1.5,0.3,1.5);float e=t.g;
vec3 tint=mix(vec3(0.97,1.03,0.96),vec3(1.04,1.0,0.94),smoothstep(0.15,0.45,e));tint=mix(tint,vec3(1.12,1.11,1.1),smoothstep(0.55,0.8,e));tint=mix(tint,vec3(1.3,1.3,1.32),smoothstep(0.85,0.95,e));
vec3 c=0.5*sh*tint;gl_FragColor=vec4(mix(vec3(0.5),c,uA*smoothstep(0.0,0.06,vz)),1.0);}`;
let pRelief = null;
function reliefKey(z, r, c){ return z + "/" + r + "_" + c; }
// a tile's mesh on the sphere: cells of at most 1.5° so it stays near the sphere, with its texture coordinates
function reliefMesh(la1, lo0, deg){
  const n = Math.max(1, Math.ceil(deg / 1.5)), V = [];
  const at = (i, j) => { const la = la1 - deg * i / n, lo = lo0 + deg * j / n; return [...xyz(la, lo), j / n, i / n]; };
  for (let i = 0; i < n; i++) for (let j = 0; j < n; j++){ const a = at(i, j), b = at(i, j + 1), c = at(i + 1, j + 1), d = at(i + 1, j); for (const q of [a, b, c, a, c, d]) V.push(...q); }
  const buf = gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER, buf); gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(V), gl.STATIC_DRAW);
  return { buf, n: V.length / 5 };
}
function reliefLoad(z, r, c){
  const k = reliefKey(z, r, c); if (RELIEF.tiles.has(k)) return;
  const deg = z === 0 ? 360 : z === 1 ? 45 : 15, la1 = 90 - r * deg, lo0 = -180 + c * deg;
  const e = { z, pending: true, tex: null, mesh: null, used: performance.now() };
  RELIEF.tiles.set(k, e);
  const img = new Image();
  img.onload = () => {
    if (RELIEF.tiles.get(k) !== e) return;
    const t = gl.createTexture(); gl.bindTexture(gl.TEXTURE_2D, t);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGB, gl.RGB, gl.UNSIGNED_BYTE, img);
    const pow2 = (img.width & (img.width - 1)) === 0 && (img.height & (img.height - 1)) === 0;
    if (pow2){ gl.generateMipmap(gl.TEXTURE_2D); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR_MIPMAP_LINEAR); }
    else gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    e.tex = t; e.mesh = z === 0 ? reliefGlobe() : reliefMesh(la1, lo0, deg); e.pending = false;
  };
  img.onerror = () => { console.warn("relief tile " + k + " failed"); };
  img.src = "geo/relief/" + k + ".jpg?v=1";
}
// the globe's mesh: 3° cells, texture across the whole image
function reliefGlobe(){
  const V = [], st = 3, at = (la, lo) => [...xyz(la, lo), (lo + 180) / 360, (90 - la) / 180];
  for (let la = 90; la > -90; la -= st) for (let lo = -180; lo < 180; lo += st){ const a = at(la, lo), b = at(la, lo + st), c = at(la - st, lo + st), d = at(la - st, lo); for (const q of [a, b, c, a, c, d]) V.push(...q); }
  const buf = gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER, buf); gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(V), gl.STATIC_DRAW);
  return { buf, n: V.length / 5 };
}
// the tiles of level z the view can see (centre = (pitch, yaw); half-extent from metres a pixel)
function reliefVisible(z, w, h, mpp){
  const deg = z === 1 ? 45 : 15, la = pitch * 180 / Math.PI, lo = ((yaw * 180 / Math.PI + 540) % 360) - 180;
  const ext = Math.min(90, 1.25 * Math.max(w, h) / 2 * mpp / 111000), lx = Math.min(180, ext / Math.max(0.15, Math.cos(la * Math.PI / 180)));
  const out = [], have = RELIEF.idx[z];
  for (let a = la - ext; a < la + ext + deg; a += deg){ const r = Math.floor((90 - Math.max(-89.999, Math.min(89.999, a))) / deg);
    for (let o = lo - lx; o < lo + lx + deg; o += deg){ const c = Math.floor((((o + 180) % 360 + 360) % 360) / deg); const k = r + "_" + c; if (have.has(k) && !out.includes(k)) out.push(k); } }
  return out.map(k => k.split("_").map(Number));
}
function reliefEvict(z, keep){
  const n = [...RELIEF.tiles.values()].filter(t => t.z === z).length; if (n <= RELIEF.cap[z]) return;
  const old = [...RELIEF.tiles.entries()].filter(([k, t]) => t.z === z && !keep.has(k) && !t.pending).sort((a, b) => a[1].used - b[1].used);
  for (let i = 0; i < n - RELIEF.cap[z] && i < old.length; i++){ const [k, t] = old[i]; gl.deleteTexture(t.tex); gl.deleteBuffer(t.mesh.buf); RELIEF.tiles.delete(k); }
}
// Called inside the land stencil (bit 1). Finest first; each pass marks bit 2, so a pixel is shaded once.
function reliefDraw(R, S, asp, w, h, mpp, a){
  if (a <= 0.01) return;
  if (!pRelief){ pRelief = prog(RV, RF); }
  if (!RELIEF.pIdx){ RELIEF.pIdx = true; reliefLoad(0, 0, 0);
    fetch("geo/relief/index.json?v=1").then(r => r.json()).then(j => { RELIEF.idx = { 1: new Set(j["1"].map(t => t[0] + "_" + t[1])), 2: new Set(j["2"].map(t => t[0] + "_" + t[1])) }; }).catch(e => console.warn(e.message)); }
  const passes = [];
  if (RELIEF.idx){
    for (const z of [2, 1]){
      if (z === 2 && mpp > 1500) continue; if (z === 1 && mpp > 6000) continue; // a texel stays above a pixel or so
      const vis = reliefVisible(z, w, h, mpp), keep = new Set(vis.map(([r, c]) => reliefKey(z, r, c)));
      for (const [r, c] of vis){ reliefLoad(z, r, c); const t = RELIEF.tiles.get(reliefKey(z, r, c)); t.used = performance.now(); if (t.tex) passes.push(t); }
      reliefEvict(z, keep);
    }
  }
  const g = RELIEF.tiles.get("0/0_0"); if (g && g.tex) passes.push(g);
  RELIEF.used = passes.length ? "z" + passes[0].z : "-";
  if (!passes.length) return;
  gl.useProgram(pRelief);
  gl.uniformMatrix3fv(gl.getUniformLocation(pRelief, "R"), false, R); gl.uniform1f(gl.getUniformLocation(pRelief, "S"), S); gl.uniform2fv(gl.getUniformLocation(pRelief, "asp"), asp);
  gl.uniform1f(gl.getUniformLocation(pRelief, "uA"), a); gl.uniform1i(gl.getUniformLocation(pRelief, "T"), 0); gl.activeTexture(gl.TEXTURE0);
  const ap = gl.getAttribLocation(pRelief, "p"), au = gl.getAttribLocation(pRelief, "uv");
  gl.enableVertexAttribArray(ap); gl.enableVertexAttribArray(au);
  gl.blendFunc(gl.DST_COLOR, gl.SRC_COLOR); // 2x multiply: 0.5 leaves the land as it is
  gl.stencilMask(2); gl.stencilFunc(gl.EQUAL, 1, 3); gl.stencilOp(gl.KEEP, gl.KEEP, gl.INVERT);
  for (const t of passes){
    gl.bindTexture(gl.TEXTURE_2D, t.tex); gl.bindBuffer(gl.ARRAY_BUFFER, t.mesh.buf);
    gl.vertexAttribPointer(ap, 3, gl.FLOAT, false, 20, 0); gl.vertexAttribPointer(au, 2, gl.FLOAT, false, 20, 12);
    gl.drawArrays(gl.TRIANGLES, 0, t.mesh.n);
  }
  gl.disableVertexAttribArray(au);
  gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
  // leave bit 2 clear again and the stencil as the land draw expects it
  gl.colorMask(false, false, false, false); gl.stencilFunc(gl.ALWAYS, 0, 3); gl.stencilOp(gl.KEEP, gl.KEEP, gl.ZERO);
  for (const t of passes){ gl.bindBuffer(gl.ARRAY_BUFFER, t.mesh.buf); gl.vertexAttribPointer(ap, 3, gl.FLOAT, false, 20, 0); gl.drawArrays(gl.TRIANGLES, 0, t.mesh.n); }
  gl.colorMask(true, true, true, true); gl.stencilMask(1); gl.stencilFunc(gl.EQUAL, 1, 1); gl.stencilOp(gl.KEEP, gl.KEEP, gl.KEEP);
}
