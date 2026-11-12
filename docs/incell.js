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
// Each map reads its quantity through its own sequential ramp (DM_RAMPS, one row of the ramp texture Rp), dark to bright, over a
// dimmed neutral land (the veil; None, mode 0, is the plain view: a faint warm in-cell tint and no veil): People inferno, Awake navy to amber, Households viridis, Verdict teal and hatching.
// Colour-blind safe: no ramp separates by red against green alone. uR is the ramp's row (texture v).
const ICF = `precision mediump float;uniform sampler2D T;uniform sampler2D C;uniform sampler2D Rp;uniform float uA;uniform float uMode;uniform float uAwake;uniform float uRate;uniform vec4 uT;uniform float uDE0;uniform vec4 uDEd;uniform float uUS0;uniform vec4 uUSd;varying vec2 vUv;varying float vz;
vec3 ramp(float row,float t){return texture2D(Rp,vec2(clamp(t,0.0,1.0)*0.992+0.004,row)).rgb;}
void main(){if(vz<0.0)discard;float v=texture2D(T,vUv).r;vec4 a=texture2D(C,vUv);
float hatch=step(0.5,fract((gl_FragCoord.x+gl_FragCoord.y)/6.0));
bool us=a.r>0.2&&a.r<0.6;bool de=a.r>0.6;float cls=floor(a.g*255.0/50.0+0.5);
if(uMode<0.75){if(v<0.02)discard;vec3 c=mix(vec3(0.95,0.78,0.42),vec3(1.0,0.93,0.72),smoothstep(0.55,1.0,v));gl_FragColor=vec4(c,uA*(0.10+0.55*v*v)*(1.0-hatch*0.18));return;}
if(uMode<1.5){if(v<0.02)discard;gl_FragColor=vec4(ramp(0.0625,0.12+0.88*v)*(1.0-hatch*0.10),uA*(0.80+0.20*v));return;}
if(uMode>5.5){if(v<0.02)discard;float msg=pow(10.0,2.0+2.5*v)*uRate*uAwake;float t=(log(msg)/2.302585-2.5)/3.2;gl_FragColor=vec4(ramp(0.5625,0.06+0.94*t),uA*(0.78+0.22*clamp(t,0.0,1.0)));return;}
if(a.b<0.5)discard;
if(uMode<2.5){
 if(us){gl_FragColor=vec4(ramp(0.1875,uAwake)*(1.0-hatch*0.08),uA*(0.40+0.55*max(v,0.1)));}
 else{gl_FragColor=vec4(0.36,0.38,0.44,uA*0.55*hatch);}
 return;}
if(uMode>3.5){if(v<0.02)discard;
 float m=us?uUS0+dot(step(uT,vec4(v)),uUSd):uDE0+dot(step(uT,vec4(v)),uDEd);
 vec3 c=ramp(0.3125,(m-1.6)/1.0);
 gl_FragColor=(us||de)?vec4(c*(1.0-hatch*0.14),uA*(0.55+0.40*v)):vec4(c,uA*(0.30+0.40*v)*(0.25+0.75*hatch));return;}
vec3 teal=vec3(0.16,0.78,0.80);
if(de&&cls<1.5){gl_FragColor=vec4(teal,uA*(0.70+0.25*v));}
else if(us||de){gl_FragColor=vec4(teal*0.62,uA*(0.45+0.40*v)*(0.20+0.80*hatch));}
else{gl_FragColor=vec4(0.36,0.38,0.44,uA*0.55*hatch);}}`;
// the ramps, as sRGB stops evenly spaced (People: matplotlib's inferno; Households: viridis; Awake: navy to amber with lightness
// rising throughout); the legend's bars (globe.js) are drawn from the same stops
const DM_RAMPS = {
  people: ["#000004", "#1b0c41", "#4a0c6b", "#781c6d", "#a52c60", "#cf4446", "#ed6925", "#fb9b06", "#f7d13d", "#fcffa4"],
  awake: ["#0c1a4a", "#1d3383", "#3a4fa6", "#6b65a6", "#9c7a95", "#c98f74", "#eaa64c", "#ffc35a", "#ffe09a"],
  households: ["#440154", "#482878", "#3e4a89", "#31688e", "#26828e", "#1f9e89", "#35b779", "#6ece58", "#b5de2b", "#fde725"],
  data: ["#0b0405", "#2e1e3c", "#413d7b", "#37659e", "#348fa7", "#40b7ad", "#8ad9b1", "#def5e5"], // mako
  power: ["#042333", "#1b2f6b", "#2c3395", "#533f97", "#744992", "#93548a", "#b15f82", "#d06c6e", "#eb7958", "#fbb43d", "#e8fa5b"], // cmocean thermal
};
// a stop list sampled at t in [0, 1] (linear between stops), as [r, g, b] 0..255
function dmRampAt(stops, t){
  const x = Math.max(0, Math.min(1, t)) * (stops.length - 1), i = Math.min(stops.length - 2, Math.floor(x)), f = x - i;
  const c = (h) => [1, 3, 5].map((k) => parseInt(h.slice(k, k + 2), 16)), a = c(stops[i]), b = c(stops[i + 1]);
  return a.map((v, k) => Math.round(v + (b[k] - v) * f));
}
let rampTex = null;
function incellRampTex(){
  if (rampTex) return rampTex;
  // eight rows (v = (row + 0.5) / 8): people, awake, households, power, data, three spare
  const W = 256, rows = [DM_RAMPS.people, DM_RAMPS.awake, DM_RAMPS.households, DM_RAMPS.power, DM_RAMPS.data, DM_RAMPS.data, DM_RAMPS.data, DM_RAMPS.data], D = new Uint8Array(W * 8 * 4);
  rows.forEach((st, r) => { for (let x = 0; x < W; x++){ const c = dmRampAt(st, x / (W - 1)); D.set([c[0], c[1], c[2], 255], (r * W + x) * 4); } });
  rampTex = gl.createTexture(); gl.bindTexture(gl.TEXTURE_2D, rampTex); gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
  gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, W, 8, 0, gl.RGBA, gl.UNSIGNED_BYTE, D);
  for (const [k, v] of [[gl.TEXTURE_MIN_FILTER, gl.LINEAR], [gl.TEXTURE_MAG_FILTER, gl.LINEAR], [gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE], [gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE]]) gl.texParameteri(gl.TEXTURE_2D, k, v);
  return rampTex;
}
// the veil: while a data map is shown, the land and relief under it go to a dim neutral base (a near-opaque dark grey over the
// land stencil), so the map's ramp is the only colour on the land; it fades in and out with the map, never between two maps
const VLV = `attribute vec2 p;void main(){gl_Position=vec4(p,0.95,1.0);}`, VLF = `precision mediump float;uniform float uA;void main(){gl_FragColor=vec4(0.050,0.057,0.072,uA);}`;
let pVeil = null, veilBuf = null;
function incellVeil(a){
  if (a <= 0.004) return;
  if (!pVeil){ pVeil = prog(VLV, VLF); veilBuf = gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER, veilBuf); gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 1, -1, 1, 1, -1, -1, 1, 1, -1, 1]), gl.STATIC_DRAW); }
  gl.useProgram(pVeil); gl.uniform1f(gl.getUniformLocation(pVeil, "uA"), a);
  const vp = gl.getAttribLocation(pVeil, "p"); gl.bindBuffer(gl.ARRAY_BUFFER, veilBuf); gl.enableVertexAttribArray(vp); gl.vertexAttribPointer(vp, 2, gl.FLOAT, false, 0, 0);
  gl.drawArrays(gl.TRIANGLES, 0, 6);
}
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
// Data: the engine's message rate (micro.rs msg_mean_interval_s: a person's awake device signs a row every 480 s on average,
// 7.5 an hour) and its day curve (workload.rs model.diurnal.v1), both read from the kernel; under the presence tint the street's
// total is this same product, so the map is what the engine moves, per km2 and per hour, at each block's local solar hour
let DATA_RATE = null;
function dataRate(){ if (DATA_RATE === null && typeof wasm !== "undefined" && wasm && wasm.micro_msg_interval){ DATA_RATE = 3600 / Math.max(1, wasm.micro_msg_interval()); } return DATA_RATE || 0; }
function dataHour(lon){ return typeof SIM === "undefined" ? 12 : (((SIM.t / 3600 + lon / 15) % 24) + 24) % 24; }
function dataDiurnal(lon){ return (typeof wasm !== "undefined" && wasm && wasm.clock_diurnal) ? wasm.clock_diurnal(dataHour(lon)) : 1; }
// PRESENCE-0's awake share at a block's local time (states 1, 2, 3 and 5: 1 - home asleep - elsewhere asleep), interpolated
// between clock hours; day types Mon-Thu, Fri, Sat, Sun; the sim's clock (SIM.t seconds from 10 November 2026, a Tuesday)
function incellAwake(lon){
  const t = typeof HOLT !== "undefined" && HOLT.t.presence0_us; if (!t || typeof SIM === "undefined") return 0;
  const hl = SIM.t / 3600 + lon / 15, day = ((Math.floor(hl / 24) + 1) % 7 + 7) % 7, h = ((hl % 24) + 24) % 24; // day: 0 Monday
  const dt = (d) => d <= 3 ? 0 : d === 4 ? 1 : d === 5 ? 2 : 3, aw = (d, hh) => { const r = t.data.table[dt(d)][hh]; return 1 - r[0] - r[4]; };
  const h0 = Math.floor(h), f = h - h0, d1 = h0 === 23 ? (day + 1) % 7 : day;
  return aw(day, h0) * (1 - f) + aw(d1, (h0 + 1) % 24) * f;
}
// Awake now at continental zoom (beyond the in-cell blocks, which stop at region zoom): one mesh over North America with each
// fragment's local solar hour from its longitude, so the day's edge shows across the time zones. The United States from the
// Natural Earth mask (a 0.5-degree texture built once), the awake share per 0.5-degree longitude column (PRESENCE-0, the same
// incellAwake as the blocks) updated each frame; no people weighting at this zoom; elsewhere hatched, no reference.
const AWW = { mesh: null, mask: null, col: null, colData: new Uint8Array(240), box: [15, -170, 72, -50] };
const AWV = `attribute vec3 p;attribute vec2 ll;uniform mat3 R;uniform float S;uniform vec2 asp;varying vec2 vLL;varying float vz;void main(){vec3 q=R*p;vz=q.z;vLL=ll;gl_Position=vec4(q.x*S*asp.x,q.y*S*asp.y,0.9,1.0);}`;
const AWF = `precision mediump float;uniform sampler2D M;uniform sampler2D W;uniform sampler2D Rp;uniform float uA;varying vec2 vLL;varying float vz;
void main(){if(vz<0.0)discard;vec2 uv=vec2((vLL.y+170.0)/120.0,(vLL.x-15.0)/57.0);float hatch=step(0.5,fract((gl_FragCoord.x+gl_FragCoord.y)/6.0));
if(texture2D(M,uv).r>0.5){float aw=texture2D(W,vec2(uv.x,0.5)).r;gl_FragColor=vec4(texture2D(Rp,vec2(aw*0.992+0.004,0.1875)).rgb,uA*0.85);}
else{gl_FragColor=vec4(0.36,0.38,0.44,uA*0.5*hatch);}}`;
let pAww = null;
function awakeWideBuild(){
  if (AWW.mask || typeof HOLT === "undefined" || !HOLT.countries) return !!AWW.mask;
  // the mask is 27,000 country lookups (about 1 s on a phone at 4x CPU throttle): a few rows a frame, at most 3 ms each,
  // and the layer draws once it is whole
  const [a0, o0, a1, o1] = AWW.box, NX = 240, NY = 114;
  if (!AWW.D){ AWW.D = new Uint8Array(NX * NY); AWW.row = 0; }
  const D = AWW.D, t0 = performance.now();
  while (AWW.row < NY && performance.now() - t0 < 3){ const y = AWW.row++; for (let x = 0; x < NX; x++){ const la = a0 + (y + 0.5) * 0.5, lo = o0 + (x + 0.5) * 0.5; D[y * NX + x] = countryNear(HOLT.countries, la, lo) === 840 ? 255 : 0; } }
  if (AWW.row < NY){ if (typeof gfxDirty === "function") gfxDirty(100); return false; }
  const tex = (w, h, data, filt) => { const t = gl.createTexture(); gl.bindTexture(gl.TEXTURE_2D, t); gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1); gl.texImage2D(gl.TEXTURE_2D, 0, gl.LUMINANCE, w, h, 0, gl.LUMINANCE, gl.UNSIGNED_BYTE, data);
    for (const [k, v] of [[gl.TEXTURE_MIN_FILTER, filt], [gl.TEXTURE_MAG_FILTER, filt], [gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE], [gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE]]) gl.texParameteri(gl.TEXTURE_2D, k, v); return t; };
  AWW.mask = tex(NX, NY, D, gl.NEAREST); AWW.col = tex(NX, 1, AWW.colData, gl.LINEAR);
  const V = []; for (let la = a0; la < a1; la++) for (let lo = o0; lo < o1; lo++){ const q = [[la, lo], [la, lo + 1], [la + 1, lo + 1], [la, lo], [la + 1, lo + 1], [la + 1, lo]]; for (const [y, x] of q) V.push(...xyz(y, x), y, x); }
  const b = gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER, b); gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(V), gl.STATIC_DRAW); AWW.mesh = { b, n: V.length / 5 };
  return true;
}
// called inside the land stencil after incellDraw; a: this layer's zoom fade (it takes over as the blocks fade out)
function incellAwakeWide(R, S, asp, a){
  const k = Math.min(1, (performance.now() - DATAMAP.t0) / 400), f = (DATAMAP.active === 2 ? k : 0) + (DATAMAP.prev === 2 && k < 1 ? 1 - k : 0);
  if (a <= 0.01 || f <= 0.01 || !awakeWideBuild()) return;
  incellVeil(a * 0.82 * Math.min(1, f)); // the same veil as the blocks (only Awake draws at this zoom)
  for (let x = 0; x < 240; x++) AWW.colData[x] = Math.round(255 * incellAwake(AWW.box[1] + (x + 0.5) * 0.5));
  gl.bindTexture(gl.TEXTURE_2D, AWW.col); gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, 240, 1, gl.LUMINANCE, gl.UNSIGNED_BYTE, AWW.colData);
  if (!pAww) pAww = prog(AWV, AWF);
  gl.useProgram(pAww);
  gl.uniformMatrix3fv(gl.getUniformLocation(pAww, "R"), false, R); gl.uniform1f(gl.getUniformLocation(pAww, "S"), S); gl.uniform2fv(gl.getUniformLocation(pAww, "asp"), asp); gl.uniform1f(gl.getUniformLocation(pAww, "uA"), a * f);
  gl.uniform1i(gl.getUniformLocation(pAww, "M"), 0); gl.uniform1i(gl.getUniformLocation(pAww, "W"), 1); gl.uniform1i(gl.getUniformLocation(pAww, "Rp"), 2);
  gl.activeTexture(gl.TEXTURE2); gl.bindTexture(gl.TEXTURE_2D, incellRampTex()); gl.activeTexture(gl.TEXTURE1); gl.bindTexture(gl.TEXTURE_2D, AWW.col); gl.activeTexture(gl.TEXTURE0); gl.bindTexture(gl.TEXTURE_2D, AWW.mask);
  const ap = gl.getAttribLocation(pAww, "p"), al = gl.getAttribLocation(pAww, "ll"); gl.bindBuffer(gl.ARRAY_BUFFER, AWW.mesh.b); gl.enableVertexAttribArray(ap); gl.enableVertexAttribArray(al);
  gl.vertexAttribPointer(ap, 3, gl.FLOAT, false, 20, 0); gl.vertexAttribPointer(al, 2, gl.FLOAT, false, 20, 12); gl.drawArrays(gl.TRIANGLES, 0, AWW.mesh.n); gl.disableVertexAttribArray(al); gl.disableVertexAttribArray(ap);
  if (typeof gfxDirty === "function") gfxDirty(1000); // the share follows the clock
}
// Power (the data-map strip's GRID-0 map): one equirectangular RGBA texture of the chosen kind's country rows, painted on a
// 2D canvas from the Natural Earth admin-0 mask (each country's rings, even-odd): r the value over 0..1,000 g/kWh, g a row
// exists, b trust C (hatched). A whole-sphere mesh samples it inside the land stencil, over the veil; no row is a dark hatch.
const PWR = { tex: null, mesh: null, W: 2048, H: 1024 };
const PWF = `precision mediump float;uniform sampler2D T;uniform sampler2D Rp;uniform float uA;varying vec2 vLL;varying float vz;
void main(){if(vz<0.0)discard;vec4 a=texture2D(T,vec2((vLL.y+180.0)/360.0,(90.0-vLL.x)/180.0));float hatch=step(0.5,fract((gl_FragCoord.x+gl_FragCoord.y)/6.0));
if(a.g<0.5){gl_FragColor=vec4(0.36,0.38,0.44,uA*0.55*hatch);return;}
vec3 c=texture2D(Rp,vec2(a.r*0.992+0.004,0.4375)).rgb;gl_FragColor=a.b>0.5?vec4(c,uA*0.85*(0.30+0.70*hatch)):vec4(c,uA*0.88);}`;
let pPwr = null;
function incellPowerReset(){ if (PWR.tex){ gl.deleteTexture(PWR.tex); PWR.tex = null; } }
function incellPowerTex(){
  if (PWR.tex) return PWR.tex; const g = typeof gridSel === "function" ? gridSel() : null; if (!g || typeof HOLT === "undefined" || !HOLT.countries) return null;
  const cv2 = document.createElement("canvas"); cv2.width = PWR.W; cv2.height = PWR.H; const c = cv2.getContext("2d"); c.imageSmoothingEnabled = false;
  const X = (lo) => (lo / 100 + 180) / 360 * PWR.W, Y = (la) => (90 - la / 100) / 180 * PWR.H;
  for (const s of g.rows){ const v = Math.max(0, Math.min(255, Math.round(s.row.value / 1000 * 255)));
    c.fillStyle = `rgb(${v},255,${s.row.trust === "C" ? 255 : 0})`; c.beginPath();
    for (const r of HOLT.countries.rings){ if (r.iso !== s.iso) continue; const P = r.P; c.moveTo(X(P[1]), Y(P[0])); for (let i = 1; i < P.length / 2; i++) c.lineTo(X(P[2 * i + 1]), Y(P[2 * i])); c.closePath(); }
    c.fill("evenodd"); }
  const t = gl.createTexture(); gl.bindTexture(gl.TEXTURE_2D, t); gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, cv2);
  for (const [k, v] of [[gl.TEXTURE_MIN_FILTER, gl.NEAREST], [gl.TEXTURE_MAG_FILTER, gl.NEAREST], [gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE], [gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE]]) gl.texParameteri(gl.TEXTURE_2D, k, v);
  PWR.tex = t; return t;
}
function incellPower(R, S, asp, a){
  const k = Math.min(1, (performance.now() - DATAMAP.t0) / 400), f = (DATAMAP.active === 5 ? k : 0) + (DATAMAP.prev === 5 && k < 1 ? 1 - k : 0);
  if (a <= 0.01 || f <= 0.01) return; const tex = incellPowerTex(); if (!tex) return;
  incellVeil(a * 0.82 * Math.min(1, f));
  if (!PWR.mesh){ const V = []; for (let la = -90; la < 90; la += 3) for (let lo = -180; lo < 180; lo += 3){ for (const [y, x] of [[la, lo], [la, lo + 3], [la + 3, lo + 3], [la, lo], [la + 3, lo + 3], [la + 3, lo]]) V.push(...xyz(y, x), y, x); }
    const b = gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER, b); gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(V), gl.STATIC_DRAW); PWR.mesh = { b, n: V.length / 5 }; }
  if (!pPwr) pPwr = prog(AWV, PWF);
  gl.useProgram(pPwr);
  gl.uniformMatrix3fv(gl.getUniformLocation(pPwr, "R"), false, R); gl.uniform1f(gl.getUniformLocation(pPwr, "S"), S); gl.uniform2fv(gl.getUniformLocation(pPwr, "asp"), asp); gl.uniform1f(gl.getUniformLocation(pPwr, "uA"), a * f);
  gl.uniform1i(gl.getUniformLocation(pPwr, "T"), 0); gl.uniform1i(gl.getUniformLocation(pPwr, "Rp"), 2);
  gl.activeTexture(gl.TEXTURE2); gl.bindTexture(gl.TEXTURE_2D, incellRampTex()); gl.activeTexture(gl.TEXTURE0); gl.bindTexture(gl.TEXTURE_2D, tex);
  const ap = gl.getAttribLocation(pPwr, "p"), al = gl.getAttribLocation(pPwr, "ll"); gl.bindBuffer(gl.ARRAY_BUFFER, PWR.mesh.b); gl.enableVertexAttribArray(ap); gl.enableVertexAttribArray(al);
  gl.vertexAttribPointer(ap, 3, gl.FLOAT, false, 20, 0); gl.vertexAttribPointer(al, 2, gl.FLOAT, false, 20, 12); gl.drawArrays(gl.TRIANGLES, 0, PWR.mesh.n); gl.disableVertexAttribArray(al); gl.disableVertexAttribArray(ap);
  if (k < 1 && typeof gfxDirty === "function") gfxDirty(100);
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
  { const k0 = Math.min(1, (performance.now() - DATAMAP.t0) / 400), blk = (m) => (m >= 1 && m <= 4) || m === 6, on = (blk(DATAMAP.active) ? k0 : 0) + (blk(DATAMAP.prev) && k0 < 1 ? 1 - k0 : 0); incellVeil(a * 0.82 * Math.min(1, on)); } // one veil, faded with the map; None (0) has none
  if (!pIncell) pIncell = prog(ICV, ICF);
  gl.useProgram(pIncell);
  gl.uniformMatrix3fv(gl.getUniformLocation(pIncell, "R"), false, R); gl.uniform1f(gl.getUniformLocation(pIncell, "S"), S); gl.uniform2fv(gl.getUniformLocation(pIncell, "asp"), asp);
  gl.uniform1i(gl.getUniformLocation(pIncell, "T"), 0); gl.uniform1i(gl.getUniformLocation(pIncell, "C"), 1);
  gl.uniform1i(gl.getUniformLocation(pIncell, "Rp"), 2); gl.activeTexture(gl.TEXTURE2); gl.bindTexture(gl.TEXTURE_2D, incellRampTex()); gl.activeTexture(gl.TEXTURE0);
  const ap = gl.getAttribLocation(pIncell, "p"), au = gl.getAttribLocation(pIncell, "uv"), uA = gl.getUniformLocation(pIncell, "uA"), uM = gl.getUniformLocation(pIncell, "uMode"), uW = gl.getUniformLocation(pIncell, "uAwake");
  gl.enableVertexAttribArray(ap); gl.enableVertexAttribArray(au);
  // the active map and, for 400 ms after a switch, the one fading out
  const k = Math.min(1, (performance.now() - DATAMAP.t0) / 400), passes = [[DATAMAP.active, k]];
  if (k < 1) passes.unshift([DATAMAP.prev, 1 - k]); // None (0) is the plain view: the faint warm in-cell tint, no dimming
  const hm = typeof hhMeans === "function" ? hhMeans() : null;
  if (hm){ const lv = (x) => (Math.log10(x) - 2) / 2.5, d = (a) => [a[1] - a[0], a[2] - a[1], a[3] - a[2], a[4] - a[3]];
    gl.uniform4fv(gl.getUniformLocation(pIncell, "uT"), hm.km2.slice(1).map(lv)); gl.uniform1f(gl.getUniformLocation(pIncell, "uDE0"), hm.de[0]); gl.uniform4fv(gl.getUniformLocation(pIncell, "uDEd"), d(hm.de));
    gl.uniform1f(gl.getUniformLocation(pIncell, "uUS0"), hm.us[0]); gl.uniform4fv(gl.getUniformLocation(pIncell, "uUSd"), d(hm.us)); }
  gl.uniform1f(gl.getUniformLocation(pIncell, "uRate"), dataRate());
  for (const [mode, f] of passes){ if (f <= 0.01 || mode === 5 || (mode === 4 && !hm)) continue;
    gl.uniform1f(uA, a * f); gl.uniform1f(uM, mode === 0 ? 0.5 : mode);
    for (const v of vis){ const b = INCELL.blocks.get(v.k); if (!b || !b.tex) continue;
      let aux = null; if (mode > 1 && mode < 5){ aux = incellAux(b); if (!aux) continue; }
      if (mode === 2) gl.uniform1f(uW, incellAwake(-180 + (b.bc + 0.5) * 1.5));
      if (mode === 6) gl.uniform1f(uW, dataDiurnal(-180 + (b.bc + 0.5) * 1.5)); // Data: the engine's day curve at the block's local hour
      gl.activeTexture(gl.TEXTURE1); gl.bindTexture(gl.TEXTURE_2D, aux || b.tex); gl.activeTexture(gl.TEXTURE0);
      gl.bindTexture(gl.TEXTURE_2D, b.tex); gl.bindBuffer(gl.ARRAY_BUFFER, b.mesh.buf);
      gl.vertexAttribPointer(ap, 3, gl.FLOAT, false, 20, 0); gl.vertexAttribPointer(au, 2, gl.FLOAT, false, 20, 12);
      gl.drawArrays(gl.TRIANGLES, 0, b.mesh.n); } }
  if (k < 1 && typeof gfxDirty === "function") gfxDirty(100);
  if ((DATAMAP.active === 2 || DATAMAP.active === 6) && typeof gfxDirty === "function") gfxDirty(1000); // the awake share and the activity follow the clock
  gl.disableVertexAttribArray(au);
  INCELL.frameMs = performance.now() - t0;
}
// the data maps' programs compiled ahead, in an idle slice after load (a first use compiled them inside a frame: a 140 ms
// task on a throttled phone when the Story first reached region zoom)
(window.requestIdleCallback || ((f) => setTimeout(f, 3000)))(() => { try { if (!pIncell) pIncell = prog(ICV, ICF); incellRampTex(); } catch (e) { console.warn("incell prewarm", e); } }, { timeout: 6000 });
