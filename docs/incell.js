// In-cell structure on the globe (FSD/twin/holon-intake.md section 3): inside every peopled 1/8-degree
// cell, the people of each 30 arc-second pixel (about 0.9 km) as the kernel's in-cell seam divides them
// (popchain::incell, CIRISHolon's UNEVEN-0, regional row where published, else world; which pixel is the
// town is the twin's display rule). One R8 texture per 1.5-degree block (180 x 180 pixels = 12 x 12 cells),
// built once from pop_cell_pixels and kept, drawn as a tint inside the land stencil at region and city zoom,
// blended with the land and relief already drawn. No per-frame loop over cells: a frame binds textures and
// draws one mesh per visible block. Blocks are built off the frame, cell by cell, in idle slices
// (requestIdleCallback), stopping while 1.5 ms of the slice remain (2 ms at least when the thread is never idle),
// so a slow phone never hitches.
// Unresolved data looks unresolved: a fine diagonal hatch runs through the tint (the verdict of every row).
const INCELL = { blocks: new Map(), cap: 48, budgetMs: 6, built: 0, buildMs: 0, bytes: 0, frameMs: 0, gen: 0 };
const ICV = `attribute vec3 p;attribute vec2 uv;uniform mat3 R;uniform float S;uniform vec2 asp;varying vec2 vUv;varying float vz;void main(){vec3 q=R*p;vz=q.z;vUv=uv;gl_Position=vec4(q.x*S*asp.x,q.y*S*asp.y,0.9,1.0);}`;
// modes (the data-map strip, globe.js): 1 People (UNEVEN-0 through the seam), 2 Awake now (PRESENCE-0, US only, uAwake per block at
// its local hour), 3 Verdict (HOUSEGEN-1 by country and class), 4 Households (HOUSEGEN-1's row mean at the pixel's class: uT the
// classes' lower bounds 2..5 in the texture's log scale, uDE0/uUS0 the class-1 means and uDEd/uUSd the steps between classes). C is the block's 12 x 12 cell texture: r country (100/255 United
// States, 200/255 Germany, 0 elsewhere), g cell class x 50 / 255, b peopled. No reference: a dark hatch, never an invented value.
const ICF = `precision mediump float;uniform sampler2D T;uniform sampler2D C;uniform float uA;uniform float uMode;uniform float uAwake;uniform vec4 uT;uniform float uDE0;uniform vec4 uDEd;uniform float uUS0;uniform vec4 uUSd;varying vec2 vUv;varying float vz;
void main(){if(vz<0.0)discard;float v=texture2D(T,vUv).r;vec4 a=texture2D(C,vUv);
float hatch=step(0.5,fract((gl_FragCoord.x+gl_FragCoord.y)/6.0));
bool us=a.r>0.2&&a.r<0.6;bool de=a.r>0.6;float cls=floor(a.g*255.0/50.0+0.5);
if(uMode<1.5){if(v<0.02)discard;vec3 c=mix(vec3(0.95,0.78,0.42),vec3(1.0,0.93,0.72),smoothstep(0.55,1.0,v));gl_FragColor=vec4(c,uA*(0.10+0.55*v*v)*(1.0-hatch*0.18));return;}
if(a.b<0.5)discard;
if(uMode<2.5){
 if(us){vec3 c=mix(vec3(0.17,0.22,0.52),vec3(1.0,0.80,0.40),uAwake);gl_FragColor=vec4(c,uA*(0.30+0.45*max(v,0.2))*(1.0-hatch*0.15));}
 else{gl_FragColor=vec4(0.42,0.44,0.50,uA*0.40*hatch);}
 return;}
if(uMode>3.5){if(v<0.02)discard;
 float m=us?uUS0+dot(step(uT,vec4(v)),uUSd):uDE0+dot(step(uT,vec4(v)),uDEd);
 vec3 c=mix(vec3(0.36,0.45,0.90),vec3(1.0,0.74,0.32),clamp((m-1.6)/1.0,0.0,1.0));
 gl_FragColor=(us||de)?vec4(c,uA*0.60*(1.0-hatch*0.22)):vec4(c,uA*0.45*(0.30+0.70*hatch));return;}
if(us){gl_FragColor=vec4(0.70,0.74,0.86,uA*0.50*(0.35+0.65*hatch));}
else if(de&&cls<1.5){gl_FragColor=vec4(0.20,0.72,0.86,uA*0.62);}
else if(de){gl_FragColor=vec4(0.70,0.74,0.86,uA*0.50*(0.35+0.65*hatch));}
else{gl_FragColor=vec4(0.30,0.31,0.36,uA*0.45*hatch);}}`;
// DATAMAP (globe.js): which data map is drawn, and the one fading out (a switch cross-fades over 400 ms; 0 = none)
let pIncell = null;
// a block's mesh on the sphere: 1.5 degrees in 0.25-degree steps, texture across it
function incellMesh(la1, lo0){
  const n = 6, deg = 1.5, V = [];
  const at = (i, j) => { const la = la1 - deg * i / n, lo = lo0 + deg * j / n; return [...xyz(la, lo), j / n, i / n]; };
  for (let i = 0; i < n; i++) for (let j = 0; j < n; j++){ const a = at(i, j), b = at(i, j + 1), c = at(i + 1, j + 1), d = at(i + 1, j); for (const q of [a, b, c, a, c, d]) V.push(...q); }
  const buf = gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER, buf); gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(V), gl.STATIC_DRAW);
  return { buf, n: V.length / 5 };
}
// the blocks the view wants, nearest first, built in idle slices; `job` is the block in progress
INCELL.want = []; INCELL.job = null; INCELL.idle = false; INCELL.slices = 0; INCELL.longest = 0;
function incellCell(job){
  const k = job.next++, cr = Math.floor(k / 12), cc = k % 12, r = job.br * 12 + cr, c = job.bc * 12 + cc, N = 180;
  if (!wasm.pop_cell_pixels(r, c)) return false;
  const p = wasm.holon_buf_ptr(0) >>> 0, o = new Float64Array(wasm.memory.buffer, p, 452);
  job.cls[k] = o[451]; if (o[451] === 0) return true;
  const lat = 90 - (r + 0.5) * 0.125, km2 = 0.86 * 0.86 * Math.cos(lat * Math.PI / 180);
  for (let i = 0; i < 225; i++){ const q = o[i]; if (!q) continue; job.any = true;
    const v = Math.max(0, Math.min(1, (Math.log10(q / km2) - 2) / 2.5)); job.img[(cr * 15 + Math.floor(i / 15)) * N + cc * 15 + (i % 15)] = Math.round(v * 255); }
  return true;
}
function incellIdle(dl){
  INCELL.idle = false; const t0 = performance.now(); INCELL.slices++;
  // as much as the idle slice allows (leaving 1.5 ms), and at least 2 ms when the thread is never idle (a cell is
  // about 0.05 ms, 0.2 ms at 4x throttle): a starved slow phone still fills its blocks without a visible hitch
  while (dl.timeRemaining() > 1.5 || performance.now() - t0 < 2){
    if (!INCELL.job){ const v = INCELL.want.find((x) => !INCELL.blocks.has(x.k)); if (!v) break; INCELL.job = { k: v.k, br: v.br, bc: v.bc, next: 0, any: false, img: new Uint8Array(180 * 180), cls: new Uint8Array(144), gen: INCELL.gen }; }
    const j = INCELL.job; if (j.gen !== INCELL.gen){ INCELL.job = null; continue; }
    if (!incellCell(j)){ INCELL.job = null; break; }
    if (j.next === 144){ INCELL.blocks.set(j.k, incellUpload(j)); INCELL.built++; INCELL.job = null; if (typeof gfxDirty === "function") gfxDirty(300); }
  }
  const dt = performance.now() - t0; INCELL.buildMs += dt; INCELL.longest = Math.max(INCELL.longest, dt);
  if (INCELL.job || INCELL.want.some((x) => !INCELL.blocks.has(x.k))) incellSchedule();
}
function incellSchedule(){ if (INCELL.idle) return; INCELL.idle = true; (window.requestIdleCallback || ((f) => setTimeout(() => f({ timeRemaining: () => 8 }), 16)))(incellIdle, { timeout: 100 }); }
function incellUpload(j){
  if (!j.any) return { empty: true };
  const N = 180, tex = gl.createTexture(); gl.bindTexture(gl.TEXTURE_2D, tex);
  gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
  gl.texImage2D(gl.TEXTURE_2D, 0, gl.LUMINANCE, N, N, 0, gl.LUMINANCE, gl.UNSIGNED_BYTE, j.img);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  INCELL.bytes += N * N;
  return { tex, mesh: incellMesh(90 - j.br * 1.5, -180 + j.bc * 1.5), br: j.br, bc: j.bc, cls: j.cls, aux: null };
}
// the block's cell texture (country, class, peopled), once the countries mask is in (globe.js holCountries)
function incellAux(b){
  if (b.aux || typeof HOLT === "undefined" || !HOLT.countries) return b.aux;
  const D = new Uint8Array(144 * 4);
  for (let k = 0; k < 144; k++){ if (!b.cls[k]) continue; const la = 90 - (b.br * 12 + Math.floor(k / 12) + 0.5) * 0.125, lo = -180 + (b.bc * 12 + (k % 12) + 0.5) * 0.125;
    const c = countryNear(HOLT.countries, la, lo); D[4 * k] = c === 840 ? 100 : c === 276 ? 200 : 0; D[4 * k + 1] = b.cls[k] * 50; D[4 * k + 2] = 255; D[4 * k + 3] = 255; }
  const t = gl.createTexture(); gl.bindTexture(gl.TEXTURE_2D, t); gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
  gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, 12, 12, 0, gl.RGBA, gl.UNSIGNED_BYTE, D);
  for (const [k, v] of [[gl.TEXTURE_MIN_FILTER, gl.NEAREST], [gl.TEXTURE_MAG_FILTER, gl.NEAREST], [gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE], [gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE]]) gl.texParameteri(gl.TEXTURE_2D, k, v);
  b.aux = t; return t;
}
// PRESENCE-0's awake share at a block's local time (states 1, 2, 3 and 5: 1 - home asleep - elsewhere asleep), interpolated
// between clock hours; day types Mon-Thu, Fri, Sat, Sun; the sim's clock (SIM.t seconds from 10 November 2026, a Tuesday)
function incellAwake(lon){
  const t = typeof HOLT !== "undefined" && HOLT.t.presence0_us; if (!t || typeof SIM === "undefined") return 0;
  const hl = SIM.t / 3600 + lon / 15, day = ((Math.floor(hl / 24) + 1) % 7 + 7) % 7, h = ((hl % 24) + 24) % 24; // day: 0 Monday
  const dt = (d) => d <= 3 ? 0 : d === 4 ? 1 : d === 5 ? 2 : 3, aw = (d, hh) => { const r = t.data.table[dt(d)][hh]; return 1 - r[0] - r[4]; };
  const h0 = Math.floor(h), f = h - h0, d1 = h0 === 23 ? (day + 1) % 7 : day;
  return aw(day, h0) * (1 - f) + aw(d1, (h0 + 1) % 24) * f;
}
// every block the view can see, nearest the centre first
function incellVisible(w, h, mpp){
  const la = pitch * 180 / Math.PI, lo = ((yaw * 180 / Math.PI + 540) % 360) - 180;
  const ext = Math.min(30, 0.6 * Math.max(w, h) * mpp / 111000) + 0.8, lx = Math.min(180, ext / Math.max(0.15, Math.cos(la * Math.PI / 180)));
  const out = [];
  for (let a = la - ext; a <= la + ext; a += 1.5) for (let o = lo - lx; o <= lo + lx; o += 1.5){
    const br = Math.min(119, Math.max(0, Math.floor((90 - a) / 1.5))), bc = ((Math.floor((o + 180) / 1.5)) % 240 + 240) % 240, k = br * 240 + bc;
    if (!out.some((x) => x.k === k)) out.push({ k, br, bc, d: (a - la) ** 2 + (o - lo) ** 2 }); }
  return out.sort((x, y) => x.d - y.d);
}
// the seam switch or a new population chain invalidates every block
function incellReset(){ for (const b of INCELL.blocks.values()) if (b && b.tex){ gl.deleteTexture(b.tex); gl.deleteBuffer(b.mesh.buf); if (b.aux) gl.deleteTexture(b.aux); } INCELL.blocks.clear(); INCELL.bytes = 0; INCELL.gen++; INCELL.job = null; }
// Called inside the land stencil, after the relief. a: the layer's opacity (zoom fade).
function incellDraw(R, S, asp, w, h, mpp, a){
  if (a <= 0.01 || typeof tlPop !== "function" || !tlPop() || !(typeof TL !== "undefined" && TL.l0 && TL.l0.size)) return;
  if (TL.incell === false) return; // the legacy even spread has no structure to draw
  const t0 = performance.now(), vis = incellVisible(w, h, mpp), keep = new Set();
  for (const v of vis) keep.add(v.k);
  INCELL.want = vis; if (vis.some((x) => !INCELL.blocks.has(x.k))) incellSchedule(); // built off the frame, in idle slices
  if (INCELL.blocks.size > INCELL.cap) for (const [k, b] of INCELL.blocks){ if (INCELL.blocks.size <= INCELL.cap) break; if (keep.has(k)) continue; if (b && b.tex){ gl.deleteTexture(b.tex); gl.deleteBuffer(b.mesh.buf); if (b.aux) gl.deleteTexture(b.aux); INCELL.bytes -= 180 * 180; } INCELL.blocks.delete(k); }
  if (!pIncell) pIncell = prog(ICV, ICF);
  gl.useProgram(pIncell);
  gl.uniformMatrix3fv(gl.getUniformLocation(pIncell, "R"), false, R); gl.uniform1f(gl.getUniformLocation(pIncell, "S"), S); gl.uniform2fv(gl.getUniformLocation(pIncell, "asp"), asp);
  gl.uniform1i(gl.getUniformLocation(pIncell, "T"), 0); gl.uniform1i(gl.getUniformLocation(pIncell, "C"), 1);
  const ap = gl.getAttribLocation(pIncell, "p"), au = gl.getAttribLocation(pIncell, "uv"), uA = gl.getUniformLocation(pIncell, "uA"), uM = gl.getUniformLocation(pIncell, "uMode"), uW = gl.getUniformLocation(pIncell, "uAwake");
  gl.enableVertexAttribArray(ap); gl.enableVertexAttribArray(au);
  // the active map and, for 400 ms after a switch, the one fading out
  const k = Math.min(1, (performance.now() - DATAMAP.t0) / 400), passes = [[DATAMAP.active, k]];
  if (k < 1 && DATAMAP.prev) passes.unshift([DATAMAP.prev, 1 - k]);
  const hm = typeof hhMeans === "function" ? hhMeans() : null;
  if (hm){ const lv = (x) => (Math.log10(x) - 2) / 2.5, d = (a) => [a[1] - a[0], a[2] - a[1], a[3] - a[2], a[4] - a[3]];
    gl.uniform4fv(gl.getUniformLocation(pIncell, "uT"), hm.km2.slice(1).map(lv)); gl.uniform1f(gl.getUniformLocation(pIncell, "uDE0"), hm.de[0]); gl.uniform4fv(gl.getUniformLocation(pIncell, "uDEd"), d(hm.de));
    gl.uniform1f(gl.getUniformLocation(pIncell, "uUS0"), hm.us[0]); gl.uniform4fv(gl.getUniformLocation(pIncell, "uUSd"), d(hm.us)); }
  for (const [mode, f] of passes){ if (!mode || f <= 0.01 || (mode === 4 && !hm)) continue;
    gl.uniform1f(uA, a * f); gl.uniform1f(uM, mode);
    for (const v of vis){ const b = INCELL.blocks.get(v.k); if (!b || !b.tex) continue;
      let aux = null; if (mode > 1){ aux = incellAux(b); if (!aux) continue; }
      if (mode === 2) gl.uniform1f(uW, incellAwake(-180 + (b.bc + 0.5) * 1.5));
      gl.activeTexture(gl.TEXTURE1); gl.bindTexture(gl.TEXTURE_2D, aux || b.tex); gl.activeTexture(gl.TEXTURE0);
      gl.bindTexture(gl.TEXTURE_2D, b.tex); gl.bindBuffer(gl.ARRAY_BUFFER, b.mesh.buf);
      gl.vertexAttribPointer(ap, 3, gl.FLOAT, false, 20, 0); gl.vertexAttribPointer(au, 2, gl.FLOAT, false, 20, 12);
      gl.drawArrays(gl.TRIANGLES, 0, b.mesh.n); } }
  if (k < 1 && typeof gfxDirty === "function") gfxDirty(100);
  if (DATAMAP.active === 2 && typeof gfxDirty === "function") gfxDirty(1000); // the awake share follows the clock
  gl.disableVertexAttribArray(au);
  INCELL.frameMs = performance.now() - t0;
}
