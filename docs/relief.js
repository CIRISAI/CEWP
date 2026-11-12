// Terrain on the land: Natural Earth's shaded relief (public domain) and NOAA ETOPO 2022 elevation
// (public domain), built into JPEG tiles by tools/geo/build_dem.py. Each pixel: R = shaded relief
// (206 = flat ground), G = elevation code round(255·sqrt(clamp(z, 0, 6500) / 6500)).
// Levels: z0 the globe (2048 x 1024), z1 45° tiles (1024 px), z2 15° tiles (900 px, 1/60°).
// The relief is a 2x multiply on the land already drawn, inside the land stencil, so the land
// keeps its colours and the shading only darkens slopes away from the light and lifts the ones
// facing it. On the night side the land is darker, so the shading is stretched (2.6x against 1.5x
// by day) to stay as legible. It fades out with the other real layers (by 30 m a pixel), before any generated
// street fades in: real geography stops at region scale.
const RELIEF = { idx: null, pIdx: false, tiles: new Map(), cpu: new Map(), cpuGen: 0, cap: { 1: 10, 2: 16 }, used: "-",
  stats(){ let n = 0; for (const t of this.tiles.values()) if (t.tex) n++; return { used: this.used, textures: n }; } };
// By day the slopes are lit by the simulation's sun: a normal from ETOPO's elevation (four
// neighbouring texels, east-west scaled by cos latitude), vertical exaggeration 1 + 2·sqrt(texel /
// 1.85 km) (3× on the 1/60° tiles, about 7.5× on the globe image; display only), shaded against the
// flat ground's own light so a plain stays as it was. By night the baked Natural Earth relief
// (fixed north-west light) keeps the ground legible, as before.
const RV = `attribute vec3 p;attribute vec2 uv;uniform mat3 R;uniform float S;uniform vec2 asp;uniform vec3 uSunWorld;varying vec2 vUv;varying float vz;varying float vSun;varying vec3 vP;void main(){vec3 q=R*p;vz=q.z;vSun=dot(p,uSunWorld);vUv=uv;vP=p;gl_Position=vec4(q.x*S*asp.x,q.y*S*asp.y,0.9,1.0);}`;
const RF = `#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif
uniform sampler2D T;uniform float uA;uniform vec2 uTexel;uniform float uMpt;uniform vec3 uSunWorld;varying vec2 vUv;varying float vz;varying float vSun;varying vec3 vP;
float zc(vec2 u){float g=texture2D(T,u).g;return g*g*6500.0;}
void main(){if(vz<0.0)discard;vec4 t=texture2D(T,vUv);float day=smoothstep(-0.10,0.12,vSun);float baked=clamp(1.0+(t.r/0.808-1.0)*mix(2.6,1.5,day),0.25,1.7);
vec3 up=normalize(vP);vec3 east=normalize(vec3(up.z,0.0,-up.x)+vec3(1e-6,0.0,0.0));vec3 north=cross(up,east);float cl=max(0.08,length(up.xz));float ex=1.0+2.0*sqrt(uMpt/1850.0);
float dzx=(zc(vUv+vec2(uTexel.x,0.0))-zc(vUv-vec2(uTexel.x,0.0)))/(2.0*uMpt*cl)*ex;float dzy=(zc(vUv-vec2(0.0,uTexel.y))-zc(vUv+vec2(0.0,uTexel.y)))/(2.0*uMpt)*ex;
vec3 n=normalize(up-east*dzx-north*dzy);float ref=max(dot(up,uSunWorld),0.0);float lam=max(dot(n,uSunWorld),0.0);float dyn=clamp((lam+0.05)/(ref+0.05),0.3,1.75);
float sun=smoothstep(-0.02,0.20,vSun);float sh=mix(baked,mix(baked,dyn,0.78),sun);float e=t.g;
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
// ---- the same tiles read on the CPU: elevation and shade at a point, for the ground under the
// generated streets (tiling.js). The globe image (about 20 km a pixel) answers everywhere; the
// 1/60-degree tile under the point (about 1.9 km) answers once it has loaded.
function reliefKeepCpu(k, z, img, la1, lo0, deg){
  try {
    const cvs = document.createElement("canvas"); cvs.width = img.width; cvs.height = img.height;
    const cx = cvs.getContext("2d", { willReadFrequently: true }); cx.drawImage(img, 0, 0);
    const d = cx.getImageData(0, 0, img.width, img.height).data, n = img.width * img.height, sh = new Uint8Array(n), el = new Uint8Array(n);
    for (let i = 0; i < n; i++){ sh[i] = d[i * 4]; el[i] = d[i * 4 + 1]; }
    RELIEF.cpu.set(k, { z, w: img.width, h: img.height, sh, el, la1, lo0, dLon: z === 0 ? 360 : deg, dLat: z === 0 ? 180 : deg }); RELIEF.cpuGen++;
  } catch (err){ console.warn("relief cpu copy " + k + ": " + err.message); }
}
function reliefBoot(){
  if (RELIEF.pIdx) return; RELIEF.pIdx = true; reliefLoad(0, 0, 0);
  fetch("geo/relief/index.json?v=1").then(r => r.json()).then(j => { RELIEF.idx = { 1: new Set(j["1"].map(t => t[0] + "_" + t[1])), 2: new Set(j["2"].map(t => t[0] + "_" + t[1])) }; }).catch(e => console.warn(e.message));
}
// [metres above the sea, shade (1 = flat ground, below 1 = a slope away from the light), level read], or null before the globe image is in
function reliefAt(la, lo){
  if (typeof gl === "undefined" || !gl) return null; reliefBoot();
  let t = null; lo = ((lo + 180) % 360 + 360) % 360 - 180; la = Math.max(-89.999, Math.min(89.999, la));
  if (RELIEF.idx){ const r = Math.floor((90 - la) / 15), c = Math.floor((lo + 180) / 15), k = reliefKey(2, r, c);
    if (RELIEF.idx[2].has(r + "_" + c)){ reliefLoad(2, r, c); const e = RELIEF.tiles.get(k); if (e) e.used = performance.now(); t = RELIEF.cpu.get(k); } }
  if (!t) t = RELIEF.cpu.get("0/0_0"); if (!t) return null;
  const fx = Math.max(0, Math.min(t.w - 1.001, (lo - t.lo0) / t.dLon * t.w - 0.5)), fy = Math.max(0, Math.min(t.h - 1.001, (t.la1 - la) / t.dLat * t.h - 0.5));
  const x = Math.floor(fx), y = Math.floor(fy), ax = fx - x, ay = fy - y, i = y * t.w + x;
  const bil = (a) => (a[i] * (1 - ax) + a[i + 1] * ax) * (1 - ay) + (a[i + t.w] * (1 - ax) + a[i + t.w + 1] * ax) * ay;
  const code = bil(t.el) / 255; return [code * code * 6500, bil(t.sh) / 206, t.z];
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
    e.texel = [1 / img.width, 1 / img.height]; e.mpt = (z === 0 ? 180 : deg) / img.height * 111320;
    if (z !== 1) reliefKeepCpu(k, z, img, la1, lo0, deg);
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
// the tiles of level z a view centred at (la, lo) can see: every row and column the window (half-extent from metres a
// pixel) overlaps, and no other. Shared by the draw and the Zurich preload (preload.js), so the two cannot disagree.
// (It once stepped from the window's edge by whole tiles and could add a tile wholly outside the view: a film strip
// caught the Story's dive fetching such tiles on demand.)
function reliefWindow(z, w, h, mpp, la, lo){
  const deg = z === 1 ? 45 : 15, n = 360 / deg;
  const ext = Math.min(90, 1.25 * Math.max(w, h) / 2 * mpp / 111000), lx = Math.min(180, ext / Math.max(0.15, Math.cos(la * Math.PI / 180)));
  const r0 = Math.floor((90 - Math.min(89.999, la + ext)) / deg), r1 = Math.floor((90 - Math.max(-89.999, la - ext)) / deg);
  const c0 = Math.floor((lo - lx + 180) / deg), c1 = Math.min(Math.floor((lo + lx + 180) / deg), c0 + n - 1), out = [];
  for (let r = r0; r <= r1; r++) for (let c = c0; c <= c1; c++) out.push([r, ((c % n) + n) % n]);
  return out;
}
// the tiles of level z the view can see (centre = (pitch, yaw))
function reliefVisible(z, w, h, mpp){
  const have = RELIEF.idx[z];
  return reliefWindow(z, w, h, mpp, pitch * 180 / Math.PI, ((yaw * 180 / Math.PI + 540) % 360) - 180).filter(([r, c]) => have.has(r + "_" + c));
}
function reliefEvict(z, keep){
  const n = [...RELIEF.tiles.values()].filter(t => t.z === z).length; if (n <= RELIEF.cap[z]) return;
  const old = [...RELIEF.tiles.entries()].filter(([k, t]) => t.z === z && !keep.has(k) && !t.pending && !(typeof zpPinned === "function" && zpPinned(k))).sort((a, b) => a[1].used - b[1].used);
  for (let i = 0; i < n - RELIEF.cap[z] && i < old.length; i++){ const [k, t] = old[i]; gl.deleteTexture(t.tex); gl.deleteBuffer(t.mesh.buf); RELIEF.tiles.delete(k); RELIEF.cpu.delete(k); }
}
// Called inside the land stencil (bit 1). Finest first; each pass marks bit 2, so a pixel is shaded once.
function reliefDraw(R, S, asp, w, h, mpp, a, sun){
  if (a <= 0.01) return;
  if (!pRelief){ pRelief = prog(RV, RF); }
  reliefBoot();
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
  gl.uniformMatrix3fv(gl.getUniformLocation(pRelief, "R"), false, R); gl.uniform1f(gl.getUniformLocation(pRelief, "S"), S); gl.uniform2fv(gl.getUniformLocation(pRelief, "asp"), asp); gl.uniform3fv(gl.getUniformLocation(pRelief, "uSunWorld"), sun || [0, 0, 1]);
  gl.uniform1f(gl.getUniformLocation(pRelief, "uA"), a); gl.uniform1i(gl.getUniformLocation(pRelief, "T"), 0); gl.activeTexture(gl.TEXTURE0);
  const ap = gl.getAttribLocation(pRelief, "p"), au = gl.getAttribLocation(pRelief, "uv");
  gl.enableVertexAttribArray(ap); gl.enableVertexAttribArray(au);
  gl.blendFunc(gl.DST_COLOR, gl.SRC_COLOR); // 2x multiply: 0.5 leaves the land as it is
  gl.stencilMask(2); gl.stencilFunc(gl.EQUAL, 1, 3); gl.stencilOp(gl.KEEP, gl.KEEP, gl.INVERT);
  const uTex = gl.getUniformLocation(pRelief, "uTexel"), uMpt = gl.getUniformLocation(pRelief, "uMpt");
  for (const t of passes){
    gl.uniform2fv(uTex, t.texel || [1 / 2048, 1 / 1024]); gl.uniform1f(uMpt, t.mpt || 19568);
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
