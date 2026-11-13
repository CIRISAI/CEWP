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
// The tint drawn over the street detail's ground fades with the detail disc's own edge (tiling.js TLV: 0.85 to 1 of its
// radius), so the leaves' faded rim does not take the tint twice: DM_RING is (centre x, y, radius) in device pixels for that
// pass, 0 otherwise. ringed() wraps a fragment shader's main so every branch is scaled.
const DM_RING = [0, 0, 0];
const ringed = (src) => src.replace("void main(){", "void main0(){") + "\nuniform vec3 uRing;float rk(){return uRing.z>0.0?1.0-smoothstep(0.85*uRing.z,uRing.z,length(gl_FragCoord.xy-uRing.xy)):1.0;}void main(){main0();gl_FragColor.a*=rk();}";
const ICF = `precision mediump float;uniform sampler2D T;uniform sampler2D C;uniform sampler2D Rp;uniform float uA;uniform float uMode;uniform float uAwake;uniform float uCurve;uniform float uRate;uniform vec4 uT;uniform float uDE0;uniform vec4 uDEd;uniform float uUS0;uniform vec4 uUSd;varying vec2 vUv;varying float vz;
vec3 ramp(float row,float t){return texture2D(Rp,vec2(clamp(t,0.0,1.0)*0.992+0.004,row)).rgb;}
void main(){if(vz<0.0)discard;float v=texture2D(T,vUv).r;vec4 a=texture2D(C,vUv);
float hatch=step(0.5,fract((gl_FragCoord.x+gl_FragCoord.y)/6.0));
bool us=a.r>0.2&&a.r<0.6;bool de=a.r>0.6;float cls=floor(a.g*255.0/50.0+0.5);
if(uMode<0.75){if(v<0.02)discard;vec3 c=mix(vec3(0.95,0.78,0.42),vec3(1.0,0.93,0.72),smoothstep(0.55,1.0,v));gl_FragColor=vec4(c,uA*(0.10+0.55*v*v)*(1.0-hatch*0.18));return;}
if(uMode<1.5){if(v<0.02)discard;gl_FragColor=vec4(ramp(0.0625,0.12+0.88*v)*(1.0-hatch*0.10),uA*(0.80+0.20*v));return;}
if(uMode>5.5){if(v<0.02)discard;float msg=pow(10.0,2.0+2.5*v)*uRate*(us?uAwake:uCurve);float t=(log(msg)/2.302585-2.5)/3.2;gl_FragColor=vec4(ramp(0.5625,0.06+0.94*t),uA*(0.78+0.22*clamp(t,0.0,1.0))*(us?1.0:0.40+0.60*hatch));return;}
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
// 7.5 an hour), read from the kernel, times who is awake at each block's local solar hour: in the United States the PRESENCE-0
// awake share (each person's day leads sending, as on the street, micro.rs send_by_curve 0), elsewhere the engine's assumed
// day curve (workload.rs model.diurnal.v1, kernel clock_diurnal), hatched: sending by hour has no reference, two assumptions
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
// Far from the ground (beyond the in-cell blocks, which take over from 3,000 to 1,200 m a pixel): one world texture of the same
// quantity per 1/8-degree cell, people per km2 from the population grid the page already holds (GEODATA.pop[1], the grid the
// street tier is conditioned on), log-scaled exactly as the blocks scale their ~1 km pixels, so the two meet on one ramp in one
// unit and cross-fade across the band. Built once in idle slices (a few rows a slice) after the grid loads, uploaded row by
// row, drawn only when whole. A second texture marks the United States and Germany (painted from the Natural Earth admin-0
// mask, as Power is); a 720-column strip holds each half-degree of longitude's local-hour values (PRESENCE-0's awake share, the
// assumed day curve), refreshed as the clock moves. Each map reads its own ramp and branch, as in the blocks' shader (ICF):
// People, Awake (United States only, hatched elsewhere), Checked?, Households (the cell's class from its density, the same
// lower bounds), Data (people x rate x who is awake, or the curve, hatched). A peopled cell under the ramp's first value (100
// people per km2: most land at this scale, where a cell's people are spread over its ~200 km2) is drawn at the ramp's bottom,
// faint: under the scale, not nobody.
const WIDE = { tex: null, row: 0, done: false, idle: false, mask: null, lon: null, lonData: new Uint8Array(720 * 4), lonAt: -1e9, mesh: null, longest: 0 };
const WDF = `precision mediump float;uniform sampler2D T;uniform sampler2D Cm;uniform sampler2D L;uniform sampler2D Rp;uniform float uA;uniform float uMode;uniform float uRate;uniform vec4 uT;uniform float uDE0;uniform vec4 uDEd;uniform float uUS0;uniform vec4 uUSd;varying vec2 vLL;varying float vz;
vec3 ramp(float row,float t){return texture2D(Rp,vec2(clamp(t,0.0,1.0)*0.992+0.004,row)).rgb;}
void main(){if(vz<0.0)discard;vec2 uv=vec2((vLL.y+180.0)/360.0,(90.0-vLL.x)/180.0);vec4 tv=texture2D(T,uv);float v=tv.r;bool peopled=tv.a>0.5;
float cc=texture2D(Cm,uv).r;bool us=cc>0.2&&cc<0.6;bool de=cc>0.6;vec4 lw=texture2D(L,vec2(uv.x,0.5));float uAwake=lw.r;float uCurve=lw.g*2.0;
float hatch=step(0.5,fract((gl_FragCoord.x+gl_FragCoord.y)/6.0));float cls=1.0+dot(step(uT,vec4(v)),vec4(1.0));
if(v<0.02&&!peopled)discard;
if(uMode<1.5){if(v<0.02){gl_FragColor=vec4(ramp(0.0625,0.12),uA*0.60);return;}gl_FragColor=vec4(ramp(0.0625,0.12+0.88*v)*(1.0-hatch*0.10),uA*(0.80+0.20*v));return;}
if(uMode>5.5){if(v<0.02){gl_FragColor=vec4(ramp(0.5625,0.16),uA*0.85*(us?1.0:0.40+0.60*hatch));return;}float msg=pow(10.0,2.0+2.5*v)*uRate*(us?uAwake:uCurve);float t=(log(msg)/2.302585-2.5)/3.2;gl_FragColor=vec4(ramp(0.5625,0.06+0.94*t),uA*(0.78+0.22*clamp(t,0.0,1.0))*(us?1.0:0.40+0.60*hatch));return;}
if(!peopled)discard;
if(uMode<2.5){
 if(us){gl_FragColor=vec4(ramp(0.1875,uAwake)*(1.0-hatch*0.08),uA*(0.40+0.55*max(v,0.1)));}
 else{gl_FragColor=vec4(0.36,0.38,0.44,uA*0.55*hatch);}
 return;}
if(uMode>3.5){
 float m=us?uUS0+dot(step(uT,vec4(v)),uUSd):uDE0+dot(step(uT,vec4(v)),uDEd);
 vec3 c=ramp(0.3125,(m-1.6)/1.0);
 gl_FragColor=(us||de)?vec4(c*(1.0-hatch*0.14),uA*(0.55+0.40*v)):vec4(c,uA*(0.30+0.40*v)*(0.25+0.75*hatch));return;}
vec3 teal=vec3(0.16,0.78,0.80);
if(de&&cls<1.5){gl_FragColor=vec4(teal,uA*(0.70+0.25*v));}
else if(us||de){gl_FragColor=vec4(teal*0.62,uA*(0.45+0.40*v)*(0.20+0.80*hatch));}
else{gl_FragColor=vec4(0.36,0.38,0.44,uA*0.55*hatch);}}`;
const AWV = `attribute vec3 p;attribute vec2 ll;uniform mat3 R;uniform float S;uniform vec2 asp;varying vec2 vLL;varying float vz;void main(){vec3 q=R*p;vz=q.z;vLL=ll;gl_Position=vec4(q.x*S*asp.x,q.y*S*asp.y,0.9,1.0);}`;
let pWide = null;
// a whole-sphere mesh in 3-degree quads with each vertex's latitude and longitude (shared by the world layer, Power and Climate)
function sphereLL(){ if (WIDE.mesh) return WIDE.mesh; const V = [];
  for (let la = -90; la < 90; la += 3) for (let lo = -180; lo < 180; lo += 3){ for (const [y, x] of [[la, lo], [la, lo + 3], [la + 3, lo + 3], [la, lo], [la + 3, lo + 3], [la + 3, lo]]) V.push(...xyz(y, x), y, x); }
  const b = gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER, b); gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(V), gl.STATIC_DRAW); return (WIDE.mesh = { b, n: V.length / 5 }); }
function wideSchedule(){ if (WIDE.idle || WIDE.done) return; WIDE.idle = true; (window.requestIdleCallback || ((f) => setTimeout(() => f({ timeRemaining: () => 8 }), 16)))(wideSlice, { timeout: 200 }); }
function wideSlice(dl){
  WIDE.idle = false; const P = typeof tlPop === "function" ? tlPop() : null; if (!P || typeof gl === "undefined" || !gl){ setTimeout(wideSchedule, 500); return; }
  const NX = 2880, NY = 1440, t0 = performance.now();
  if (!WIDE.tex){ WIDE.tex = gl.createTexture(); gl.bindTexture(gl.TEXTURE_2D, WIDE.tex); gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.LUMINANCE_ALPHA, NX, NY, 0, gl.LUMINANCE_ALPHA, gl.UNSIGNED_BYTE, null);
    for (const [k, v] of [[gl.TEXTURE_MIN_FILTER, gl.LINEAR], [gl.TEXTURE_MAG_FILTER, gl.LINEAR], [gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE], [gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE]]) gl.texParameteri(gl.TEXTURE_2D, k, v); }
  // rows while the slice allows (1.5 ms left), at least one row; a row is 2,880 cells (about 0.1 ms, 0.4 ms at 4x throttle)
  const buf = new Uint8Array(NX * 2 * 24); let n = 0;
  while (WIDE.row + n < NY && n < 24 && (n === 0 || dl.timeRemaining() > 1.5 || performance.now() - t0 < 2)){
    const r = WIDE.row + n, km2 = pcCellKm2(r);
    for (let c = 0; c < NX; c++){ const q = P[r * NX + c]; if (!(q > 0)) continue; const v = (Math.log10(q / km2) - 2) / 2.5, o = (n * NX + c) * 2;
      buf[o] = v <= 0 ? 0 : v >= 1 ? 255 : Math.round(v * 255); buf[o + 1] = 255; }
    n++; }
  gl.bindTexture(gl.TEXTURE_2D, WIDE.tex); gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
  gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, WIDE.row, NX, n, gl.LUMINANCE_ALPHA, gl.UNSIGNED_BYTE, buf.subarray(0, n * NX * 2));
  WIDE.row += n; WIDE.longest = Math.max(WIDE.longest, performance.now() - t0);
  if (WIDE.row < NY) wideSchedule(); else { WIDE.done = true; if (typeof gfxDirty === "function") gfxDirty(300); }
}
// the United States and Germany, painted from the admin-0 mask once it is in (an idle slice; two countries' rings)
function wideMask(){
  if (WIDE.mask || typeof HOLT === "undefined" || !HOLT.countries) return WIDE.mask;
  const W = 2048, H = 1024, cv2 = document.createElement("canvas"); cv2.width = W; cv2.height = H; const c = cv2.getContext("2d"); c.imageSmoothingEnabled = false;
  const X = (lo) => (lo / 100 + 180) / 360 * W, Y = (la) => (90 - la / 100) / 180 * H;
  for (const [iso, v] of [[840, 100], [276, 200]]){ c.fillStyle = `rgb(${v},0,0)`; c.beginPath();
    for (const r of HOLT.countries.rings){ if (r.iso !== iso) continue; const Pp = r.P; c.moveTo(X(Pp[1]), Y(Pp[0])); for (let i = 1; i < Pp.length / 2; i++) c.lineTo(X(Pp[2 * i + 1]), Y(Pp[2 * i])); c.closePath(); }
    c.fill("evenodd"); }
  const t = gl.createTexture(); gl.bindTexture(gl.TEXTURE_2D, t); gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, cv2);
  for (const [k, v] of [[gl.TEXTURE_MIN_FILTER, gl.NEAREST], [gl.TEXTURE_MAG_FILTER, gl.NEAREST], [gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE], [gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE]]) gl.texParameteri(gl.TEXTURE_2D, k, v);
  return (WIDE.mask = t);
}
// each half-degree of longitude's local-hour values, refreshed every half second of real time (the blocks read the same
// functions per block): r the awake share (PRESENCE-0), g the day curve over 2 (model.diurnal.v1 tops out at 1.6)
function wideLon(){
  const now = performance.now(); if (WIDE.lon && now - WIDE.lonAt < 500) return WIDE.lon; WIDE.lonAt = now;
  const D = WIDE.lonData; for (let x = 0; x < 720; x++){ const lo = -180 + (x + 0.5) * 0.5; D[4 * x] = Math.round(255 * Math.max(0, Math.min(1, incellAwake(lo)))); D[4 * x + 1] = Math.round(255 * Math.max(0, Math.min(1, dataDiurnal(lo) / 2))); D[4 * x + 3] = 255; }
  if (!WIDE.lon){ WIDE.lon = gl.createTexture(); gl.bindTexture(gl.TEXTURE_2D, WIDE.lon); gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, 720, 1, 0, gl.RGBA, gl.UNSIGNED_BYTE, D);
    for (const [k, v] of [[gl.TEXTURE_MIN_FILTER, gl.LINEAR], [gl.TEXTURE_MAG_FILTER, gl.LINEAR], [gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE], [gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE]]) gl.texParameteri(gl.TEXTURE_2D, k, v); }
  else { gl.bindTexture(gl.TEXTURE_2D, WIDE.lon); gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, 720, 1, gl.RGBA, gl.UNSIGNED_BYTE, D); }
  return WIDE.lon;
}
// called inside the land stencil; a: this layer's zoom fade (it gives way to the blocks from 1,200 to 3,000 m a pixel)
function wideDraw(R, S, asp, a){
  const k = Math.min(1, (performance.now() - DATAMAP.t0) / 400), mine = (m) => (m >= 1 && m <= 4) || m === 6;
  const passes = [[DATAMAP.active, k]]; if (k < 1) passes.unshift([DATAMAP.prev, 1 - k]);
  if (a <= 0.01 || !passes.some(([m, f]) => mine(m) && f > 0.01)) return;
  if (!WIDE.done){ wideSchedule(); return; } const mask = wideMask(); if (!mask) return;
  const hm = typeof hhMeans === "function" ? hhMeans() : null;
  if (!pWide) pWide = prog(AWV, WDF);
  gl.useProgram(pWide); const U = (n) => gl.getUniformLocation(pWide, n);
  gl.uniformMatrix3fv(U("R"), false, R); gl.uniform1f(U("S"), S); gl.uniform2fv(U("asp"), asp); gl.uniform1f(U("uRate"), dataRate());
  if (hm){ const lv = (x) => (Math.log10(x) - 2) / 2.5, d = (q) => [q[1] - q[0], q[2] - q[1], q[3] - q[2], q[4] - q[3]];
    gl.uniform4fv(U("uT"), hm.km2.slice(1).map(lv)); gl.uniform1f(U("uDE0"), hm.de[0]); gl.uniform4fv(U("uDEd"), d(hm.de)); gl.uniform1f(U("uUS0"), hm.us[0]); gl.uniform4fv(U("uUSd"), d(hm.us)); }
  gl.uniform1i(U("T"), 0); gl.uniform1i(U("Cm"), 1); gl.uniform1i(U("L"), 3); gl.uniform1i(U("Rp"), 2);
  gl.activeTexture(gl.TEXTURE3); gl.bindTexture(gl.TEXTURE_2D, wideLon()); gl.activeTexture(gl.TEXTURE2); gl.bindTexture(gl.TEXTURE_2D, incellRampTex());
  gl.activeTexture(gl.TEXTURE1); gl.bindTexture(gl.TEXTURE_2D, mask); gl.activeTexture(gl.TEXTURE0); gl.bindTexture(gl.TEXTURE_2D, WIDE.tex);
  const M = sphereLL(), ap = gl.getAttribLocation(pWide, "p"), al = gl.getAttribLocation(pWide, "ll"); gl.bindBuffer(gl.ARRAY_BUFFER, M.b); gl.enableVertexAttribArray(ap); gl.enableVertexAttribArray(al);
  gl.vertexAttribPointer(ap, 3, gl.FLOAT, false, 20, 0); gl.vertexAttribPointer(al, 2, gl.FLOAT, false, 20, 12);
  for (const [m, f] of passes){ if (!mine(m) || f <= 0.01 || (m === 4 && !hm)) continue; gl.uniform1f(U("uA"), a * f); gl.uniform1f(U("uMode"), m); gl.drawArrays(gl.TRIANGLES, 0, M.n); }
  gl.disableVertexAttribArray(al); gl.disableVertexAttribArray(ap);
  if (typeof gfxDirty === "function"){ if (k < 1) gfxDirty(100); if (DATAMAP.active === 2 || DATAMAP.active === 6) gfxDirty(1000); } // the hour moves
}
// One veil for every map at every zoom: while a map is shown, the land and relief under it go to a dim neutral base, faded with
// the map (never between two maps), so "a map on" never looks like Off
function dmVeil(){
  if (typeof DATAMAP === "undefined") return; const k = Math.min(1, (performance.now() - DATAMAP.t0) / 400);
  const on = (DATAMAP.active ? k : 0) + (DATAMAP.prev && k < 1 ? 1 - k : 0); incellVeil(0.82 * Math.min(1, on));
}
// Over the street tiling's mosaic (tiling.js calls this after the mosaic and its relief, before arterials, streets and
// buildings): the same maps again, as a tint over the holons, so a map stays visible down to one street. a: the mosaic's own
// opacity times the tint's share (buildings and streets are drawn after, so they stay readable).
function dmOverMosaic(R, S, asp, w, h, mpp, a){
  if (typeof DATAMAP === "undefined" || (!DATAMAP.active && !DATAMAP.prev) || a <= 0.01) return;
  incellDraw(R, S, asp, w, h, mpp, a, true); incellPower(R, S, asp, a); if (typeof climateDraw === "function") climateDraw(R, S, asp, a, true);
}
// Power (the data-map strip's GRID-0 map): one equirectangular RGBA texture of the chosen kind's country rows, painted on a
// 2D canvas from the Natural Earth admin-0 mask (each country's rings, even-odd): r the value over 0..1,000 g/kWh, g a row
// exists, b trust C (hatched). A whole-sphere mesh samples it inside the land stencil, over the veil; no row is a dark hatch.
const PWR = { tex: null, mesh: null, W: 2048, H: 1024 };
const PWF = `precision mediump float;uniform sampler2D T;uniform sampler2D Rp;uniform float uA;varying vec2 vLL;varying float vz;
void main(){if(vz<0.0)discard;vec4 a=texture2D(T,vec2((vLL.y+180.0)/360.0,(90.0-vLL.x)/180.0));float hatch=step(0.5,fract((gl_FragCoord.x+gl_FragCoord.y)/6.0));
if(a.g<0.5){gl_FragColor=vec4(0.36,0.38,0.44,uA*0.55*hatch);return;}
vec3 c=texture2D(Rp,vec2(a.r*0.992+0.004,0.4375)).rgb;float dots=step(0.6,fract(gl_FragCoord.x/4.0))*step(0.6,fract(gl_FragCoord.y/4.0));
gl_FragColor=a.b>0.75?vec4(c,uA*0.85*(0.30+0.70*hatch)):a.b>0.25?vec4(mix(c,vec3(0.05),dots*0.7),uA*0.88):vec4(c,uA*0.88);}`;
let pPwr = null;
function incellPowerReset(){ if (PWR.tex){ gl.deleteTexture(PWR.tex); PWR.tex = null; } }
function incellPowerTex(){
  if (PWR.tex) return PWR.tex; const g = typeof gridSel === "function" ? gridSel() : null; if (!g || typeof HOLT === "undefined" || !HOLT.countries) return null;
  const cv2 = document.createElement("canvas"); cv2.width = PWR.W; cv2.height = PWR.H; const c = cv2.getContext("2d"); c.imageSmoothingEnabled = false;
  const X = (lo) => (lo / 100 + 180) / 360 * PWR.W, Y = (la) => (90 - la / 100) / 180 * PWR.H;
  // r: the value over GRID_SCALE (900 g/kWh; above it, the top); b: 255 trust C (hatched), 128 a small system (dotted)
  for (const s of g.rows){ const v = Math.max(0, Math.min(255, Math.round(s.row.value / GRID_SCALE * 255)));
    c.fillStyle = `rgb(${v},255,${s.row.trust === "C" ? 255 : s.row.small_system ? 128 : 0})`; c.beginPath();
    for (const r of HOLT.countries.rings){ if (r.iso !== s.iso) continue; const P = r.P; c.moveTo(X(P[1]), Y(P[0])); for (let i = 1; i < P.length / 2; i++) c.lineTo(X(P[2 * i + 1]), Y(P[2 * i])); c.closePath(); }
    c.fill("evenodd"); }
  const t = gl.createTexture(); gl.bindTexture(gl.TEXTURE_2D, t); gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, cv2);
  for (const [k, v] of [[gl.TEXTURE_MIN_FILTER, gl.NEAREST], [gl.TEXTURE_MAG_FILTER, gl.NEAREST], [gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE], [gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE]]) gl.texParameteri(gl.TEXTURE_2D, k, v);
  PWR.tex = t; return t;
}
function incellPower(R, S, asp, a){
  const k = Math.min(1, (performance.now() - DATAMAP.t0) / 400), f = (DATAMAP.active === 5 ? k : 0) + (DATAMAP.prev === 5 && k < 1 ? 1 - k : 0);
  if (a <= 0.01 || f <= 0.01) return; const tex = incellPowerTex(); if (!tex) return;
  PWR.mesh = sphereLL(); // the veil is dmVeil's, shared by every map
  if (!pPwr) pPwr = prog(AWV, ringed(PWF));
  gl.useProgram(pPwr); gl.uniform3fv(gl.getUniformLocation(pPwr, "uRing"), DM_RING);
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
function incellDraw(R, S, asp, w, h, mpp, a, over){
  if (a <= 0.01 || typeof tlPop !== "function" || !tlPop() || !(typeof TL !== "undefined" && TL.l0 && TL.l0.size)) return;
  if (TL.incell === false) return; // the legacy even spread has no structure to draw
  const t0 = performance.now(), vis = incellVisible(w, h, mpp), keep = new Set();
  for (const v of vis) keep.add(v.k);
  INCELL.want = vis; if (vis.some((x) => !INCELL.blocks.has(x.k))) incellSchedule(); // built off the frame, in idle slices
  if (INCELL.blocks.size > INCELL.cap) for (const [k, b] of INCELL.blocks){ if (INCELL.blocks.size <= INCELL.cap) break; if (keep.has(k)) continue; if (b && b.tex){ gl.deleteTexture(b.tex); gl.deleteBuffer(b.mesh.buf); if (b.aux) gl.deleteTexture(b.aux); INCELL.bytes -= 180 * 180; } INCELL.blocks.delete(k); }
  if (!pIncell) pIncell = prog(ICV, ringed(ICF));
  gl.useProgram(pIncell); gl.uniform3fv(gl.getUniformLocation(pIncell, "uRing"), DM_RING);
  gl.uniformMatrix3fv(gl.getUniformLocation(pIncell, "R"), false, R); gl.uniform1f(gl.getUniformLocation(pIncell, "S"), S); gl.uniform2fv(gl.getUniformLocation(pIncell, "asp"), asp);
  gl.uniform1i(gl.getUniformLocation(pIncell, "T"), 0); gl.uniform1i(gl.getUniformLocation(pIncell, "C"), 1);
  gl.uniform1i(gl.getUniformLocation(pIncell, "Rp"), 2); gl.activeTexture(gl.TEXTURE2); gl.bindTexture(gl.TEXTURE_2D, incellRampTex()); gl.activeTexture(gl.TEXTURE0);
  const ap = gl.getAttribLocation(pIncell, "p"), au = gl.getAttribLocation(pIncell, "uv"), uA = gl.getUniformLocation(pIncell, "uA"), uM = gl.getUniformLocation(pIncell, "uMode"), uW = gl.getUniformLocation(pIncell, "uAwake"), uC = gl.getUniformLocation(pIncell, "uCurve");
  gl.enableVertexAttribArray(ap); gl.enableVertexAttribArray(au);
  // the active map and, for 400 ms after a switch, the one fading out
  const k = Math.min(1, (performance.now() - DATAMAP.t0) / 400), passes = [[DATAMAP.active, k]];
  if (k < 1) passes.unshift([DATAMAP.prev, 1 - k]); // None (0) is the plain view: the faint warm in-cell tint, no dimming
  const hm = typeof hhMeans === "function" ? hhMeans() : null;
  if (hm){ const lv = (x) => (Math.log10(x) - 2) / 2.5, d = (a) => [a[1] - a[0], a[2] - a[1], a[3] - a[2], a[4] - a[3]];
    gl.uniform4fv(gl.getUniformLocation(pIncell, "uT"), hm.km2.slice(1).map(lv)); gl.uniform1f(gl.getUniformLocation(pIncell, "uDE0"), hm.de[0]); gl.uniform4fv(gl.getUniformLocation(pIncell, "uDEd"), d(hm.de));
    gl.uniform1f(gl.getUniformLocation(pIncell, "uUS0"), hm.us[0]); gl.uniform4fv(gl.getUniformLocation(pIncell, "uUSd"), d(hm.us)); }
  gl.uniform1f(gl.getUniformLocation(pIncell, "uRate"), dataRate());
  for (const [mode, f] of passes){ if (f <= 0.01 || mode === 5 || mode === 7 || (mode === 4 && !hm) || (over && mode === 0)) continue; // Power and Climate draw their own; None's faint tint stays under the mosaic
    gl.uniform1f(uA, a * f); gl.uniform1f(uM, mode === 0 ? 0.5 : mode);
    for (const v of vis){ const b = INCELL.blocks.get(v.k); if (!b || !b.tex) continue;
      let aux = null; if ((mode > 1 && mode < 5) || mode === 6){ aux = incellAux(b); if (!aux) continue; } // Data reads the country too
      if (mode === 2) gl.uniform1f(uW, incellAwake(-180 + (b.bc + 0.5) * 1.5));
      if (mode === 6){ const lo = -180 + (b.bc + 0.5) * 1.5; gl.uniform1f(uW, incellAwake(lo)); gl.uniform1f(uC, dataDiurnal(lo)); } // Data: who is awake in the United States, the assumed curve elsewhere
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
(window.requestIdleCallback || ((f) => setTimeout(f, 3000)))(() => { try { if (!pIncell) pIncell = prog(ICV, ringed(ICF)); if (!pWide) pWide = prog(AWV, WDF); incellRampTex(); wideSchedule(); } catch (e) { console.warn("incell prewarm", e); } }, { timeout: 6000 });
