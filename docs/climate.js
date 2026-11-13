// Climate class (CIRISHolon APPEARANCE-0, docs/geo/holon/appearance0.json; FSD/twin/holon-intake.md section 7): the
// Köppen-Geiger class (Beck et al. 2023, 1991 to 2020, CC BY 4.0) of every 1/8-degree cell of the twin's grid, one byte a
// cell in the twin's own cell order (geodata.js geoCell, level 1), and the majority's share of the cell (in 255ths).
// A key, not an appearance: nothing on the globe changes with it yet; the Climate data map (Advanced) shows it so the
// intake can be seen and checked, and the kernel can key a street's built form on it through the runtime table door.
//
// A class is NOT land (a cell with one classed 1 km pixel in 225 has a class): the map draws inside the twin's own land
// stencil, the lookup reports the twin's land cover beside the class, and the kernel reads the class only for leaves,
// which exist only where the twin's population chain puts people.
//
// Lazy: nothing is fetched at first paint. The grid (114 kB packed) loads when the Climate map is opened, or when a
// built-form table that keys on climate is handed to the kernel; the share (350 kB) only with the map. Both are
// unpacked by the browser's stream decoder and checked against the sha256 the index declares (crypto.subtle) before any
// lookup answers. The kernel gets its own copy through the door in 64 kB slices in idle time, checks the sha256 itself
// and only then holds the table (kernel door_begin / door_feed / door_finish). The map's texture is uploaded in strips
// of rows in idle time. The licence asks for the credit wherever the class is shown: CLIMATE_CREDIT.
"use strict";
const CLIMATE = { want: false, share: false, state: "idle", cells: null, shareCells: null, err: null, k: { state: "idle", at: 0, version: 0, code: 0 },
  tex: null, texRow: 0, texStep: 1, pal: null, mesh: null, ms: { fetch: 0, check: 0, kernel: 0, upload: 0 }, slices: 0, longest: 0 };
const CLIMATE_FILE = "koppen_1991_2020_cell.bin.gz", CLIMATE_SHARE = "koppen_1991_2020_cell_share.bin.gz", CLIMATE_N = 2880 * 1440, CLIMATE_CHUNK = 65536;
// the main groups in plain words, with a colour each (Okabe-Ito hues: no two groups told apart by red against green alone);
// a class takes its group's hue, darker to lighter by its place in the group (a display choice, not the source map's colours)
const CLIMATE_GROUPS = {
  tropical: { plain: "tropical: hot all year", rgb: [0, 158, 115] },
  arid: { plain: "dry: desert and steppe", rgb: [230, 159, 0] },
  temperate: { plain: "temperate: mild winters", rgb: [240, 228, 66] },
  cold: { plain: "cold: snowy winters", rgb: [86, 180, 233] },
  polar: { plain: "polar: tundra and ice", rgb: [215, 218, 226] },
};
const CLIMATE_ORDER = { A: "tropical", B: "arid", C: "temperate", D: "cold", E: "polar" };
const CLIMATE_SYMBOLS = ["Af", "Am", "Aw", "BWh", "BWk", "BSh", "BSk", "Csa", "Csb", "Csc", "Cwa", "Cwb", "Cwc", "Cfa", "Cfb", "Cfc", "Dsa", "Dsb", "Dsc", "Dsd", "Dwa", "Dwb", "Dwc", "Dwd", "Dfa", "Dfb", "Dfc", "Dfd", "ET", "EF"];
// the credit, worded from the sidecar's source block (the licence's version is in the link)
const CLIMATE_CREDIT = `<span class="dm-credit">climate classes: Beck et al. (2023), high-resolution Köppen-Geiger maps, 1991 to 2020 (<a href="https://doi.org/10.6084/m9.figshare.21789074" target="_blank" rel="noopener">record</a>), <a href="https://creativecommons.org/licenses/by/4.0/" target="_blank" rel="noopener">CC BY 4.0</a>; reduced to one majority class per 1/8° cell by CIRISHolon; no endorsement implied</span>`;
const climateCellOf = typeof geoCell === "function" ? geoCell : require("./geo/geodata.js").geoCell;
// a class's colour [r, g, b] 0..255 (0: none)
function climateColour(code){
  const sym = CLIMATE_SYMBOLS[code - 1]; if (!sym) return null;
  const g = CLIMATE_ORDER[sym[0]], members = CLIMATE_SYMBOLS.filter((s) => s[0] === sym[0]), i = members.indexOf(sym), n = members.length;
  const t = n > 1 ? i / (n - 1) : 0.5, base = CLIMATE_GROUPS[g].rgb; // darker for the first of the group, lighter for the last
  // polar's darker end stays a mid grey, so tundra never reads as the dark hatch of a cell with no class
  const lo = g === "polar" ? 0.7 : 0.55;
  return base.map((v) => Math.round(t < 0.5 ? v * (lo + 2 * (1 - lo) * t) : v + (255 - v) * (t - 0.5) * 0.7));
}
// the class code of a cell grid at a point (the twin's cell, as geoCell): the page's lookup and the tests' share it
function climateCode(cells, lat, lon){ const [r, c] = climateCellOf(lat, lon, 1); return cells[r * 2880 + c]; }
// the twin's land cover says water: no land pixel in the cell (200) or permanent water its majority (80)
const climateWater = (code) => code === 200 || code === 80;
// the index entry and asset (holLoad brings them with every Holon table: 7 kB)
function climateMeta(){ return typeof HOLT !== "undefined" && HOLT.t && HOLT.t.appearance0 ? HOLT.t.appearance0 : null; }
// The climate class at a point: { code, symbol, name, group, plain, share, mixed, land, cell } once the grid is in and
// checked; null before. land is the twin's own land cover (Copernicus LC100 at 1/8 degree): false at sea or on a lake, null where
// it has no answer; the class never says land.
function climateAt(lat, lon){
  const C = CLIMATE.cells; if (!C) return null;
  const [r, c] = climateCellOf(lat, lon, 1), k = r * 2880 + c, code = C[k], m = climateMeta(), e = code && m ? m.data.legend[code] : null;
  const lc = typeof geoLandcoverAt === "function" ? geoLandcoverAt(lat, lon, 1) : null;
  const share = CLIMATE.shareCells ? CLIMATE.shareCells[k] / 255 : null;
  return { code, symbol: e ? e.symbol : null, name: e ? e.name : null, group: e ? e.group : null, plain: e ? CLIMATE_GROUPS[e.group].plain : null,
    share, mixed: !!code && share !== null && share < 0.8, land: lc ? !climateWater(lc.code) : null, cell: k };
}
async function climateFetch(name, ref = "appearance0"){
  const r = await fetch("geo/holon/" + ref + "/" + name + "?v=" + (typeof HOL_V === "number" ? HOL_V : 1)); if (!r.ok) throw new Error(name + " " + r.status);
  const buf = new Uint8Array(await r.arrayBuffer());
  if (buf[0] !== 0x1f || buf[1] !== 0x8b) return buf; // a host that already undid the gzip
  if (typeof DecompressionStream === "undefined") throw new Error("no DecompressionStream: the climate grid needs it");
  return new Uint8Array(await new Response(new Blob([buf]).stream().pipeThrough(new DecompressionStream("gzip"))).arrayBuffer());
}
const climateHex = (b) => Array.from(new Uint8Array(b), (x) => x.toString(16).padStart(2, "0")).join("");
// the unpacked bytes against the sidecar's sha256 (crypto.subtle, off the main thread's long tasks); without it (an insecure
// origin) the page waits for the kernel's own check, which reads every byte
async function climateCheck(bytes, want){
  if (bytes.length !== CLIMATE_N) throw new Error("climate grid: " + bytes.length + " bytes, not one per cell");
  if (typeof crypto === "undefined" || !crypto.subtle) return null;
  const h = climateHex(await crypto.subtle.digest("SHA-256", bytes)); if (h !== want) throw new Error("climate grid: sha256 differs from the index"); return true;
}
// load the grid (and the share with the map); idempotent
function climateLoad(share){
  if (share) CLIMATE.share = true;
  if (!CLIMATE.want){ CLIMATE.want = true; climateStart(); }
  if (CLIMATE.share && CLIMATE.state === "held" && !CLIMATE.shareCells && !CLIMATE.shareP) climateShare();
}
function climateStart(){
  const m = climateMeta(); if (!m){ if (typeof holLoad === "function") holLoad(); setTimeout(climateStart, 300); return; }
  CLIMATE.state = "fetching"; const t0 = performance.now(), f = m.data.files[CLIMATE_FILE];
  climateFetch(CLIMATE_FILE).then(async (b) => { CLIMATE.ms.fetch = performance.now() - t0; const t1 = performance.now(); const ok = await climateCheck(b, f.sha256_unpacked); CLIMATE.ms.check = performance.now() - t1;
    CLIMATE.raw = b; CLIMATE.checked = ok; climateKernelStart();
    if (ok){ CLIMATE.cells = b; CLIMATE.state = "held"; climateHeld(); } else CLIMATE.state = "waiting for the kernel's check"; })
    .catch((e) => { CLIMATE.state = "failed"; CLIMATE.err = e.message; console.warn(e.message); });
}
function climateHeld(){ if (CLIMATE.share) climateShare(); if (typeof dmRender === "function" && typeof DATAMAP !== "undefined" && DATAMAP.active === 7) dmRender(); if (typeof gfxDirty === "function") gfxDirty(500); }
function climateShare(){
  const m = climateMeta(), f = m.data.files[CLIMATE_SHARE];
  CLIMATE.shareP = climateFetch(CLIMATE_SHARE).then(async (b) => { if ((await climateCheck(b, f.sha256_unpacked)) === false) return; CLIMATE.shareCells = b; CLIMATE.texRow = 0; if (typeof gfxDirty === "function") gfxDirty(500); })
    .catch((e) => { CLIMATE.err = e.message; console.warn(e.message); });
}
// ---- the kernel's copies, through the runtime table door: one table at a time (the door holds one pending table), a 64 kB
// slice or more per idle callback; the kernel hashes every byte and holds the table only when the sha256 is the declared one
const DOOR_Q = { list: [], cur: null };
function climateIdle(f){ (window.requestIdleCallback || ((g) => setTimeout(() => g({ timeRemaining: () => 8 }), 16)))(f, { timeout: 200 }); }
function doorPut(name, ref, version, sha, bytes, done){
  if (typeof wasm === "undefined" || !wasm || !wasm.door_begin){ done({ state: "no kernel", code: 0 }); return; }
  DOOR_Q.list.push({ name, ref, version, sha, bytes, done, at: 0 }); if (!DOOR_Q.cur) doorNext();
}
function doorNext(){
  const t = DOOR_Q.cur = DOOR_Q.list.shift() || null; if (!t) return;
  const hdr = new TextEncoder().encode(`${t.name}\n${t.ref}\n${t.version}\n${t.sha}`), p = wasm.door_stage(Math.max(CLIMATE_CHUNK, hdr.length)) >>> 0;
  new Uint8Array(wasm.memory.buffer, p, hdr.length).set(hdr);
  const rc = wasm.door_begin(hdr.length); if (rc < 0){ t.done({ state: "refused", code: rc }); doorNext(); return; }
  climateIdle(doorSlice);
}
function doorSlice(dl){
  const t = DOOR_Q.cur, t0 = performance.now(); if (!t) return; CLIMATE.slices++;
  // as much as the idle slice allows (leaving 2 ms), and one 64 kB piece at least when the thread is never idle
  do { const len = Math.min(CLIMATE_CHUNK, t.bytes.length - t.at), p = wasm.door_stage(CLIMATE_CHUNK) >>> 0;
    new Uint8Array(wasm.memory.buffer, p, len).set(t.bytes.subarray(t.at, t.at + len)); const n = wasm.door_feed(len);
    if (n < 0){ t.done({ state: "refused", code: n }); doorNext(); return; } t.at += len;
  } while (t.at < t.bytes.length && dl.timeRemaining() > 2);
  const dt = () => { const d = performance.now() - t0; CLIMATE.ms.kernel += d; CLIMATE.longest = Math.max(CLIMATE.longest, d); };
  if (t.at < t.bytes.length){ dt(); climateIdle(doorSlice); return; }
  const s = wasm.door_finish(); dt();
  if (s >= 0){ const n = wasm.door_versions(), v = new Float64Array(wasm.memory.buffer, wasm.holon_buf_ptr(0) >>> 0, n)[s]; t.done({ state: "held", code: s, version: v });
    if (typeof tlDoorChanged === "function") tlDoorChanged(); }
  else t.done({ state: "refused", code: s });
  doorNext();
}
function climateKernelStart(){
  if (CLIMATE.k.state !== "idle") return; const m = climateMeta(), f = m.data.files[CLIMATE_FILE]; CLIMATE.k.state = "feeding";
  doorPut("climate", m.ref.split(" ")[0], m.version, f.sha256_unpacked, CLIMATE.raw, (r) => { Object.assign(CLIMATE.k, r);
    if (r.state === "held" && !CLIMATE.cells){ CLIMATE.cells = CLIMATE.raw; CLIMATE.state = "held"; climateHeld(); } // no crypto.subtle: the kernel's check stands
    if (r.state !== "held" && !CLIMATE.cells) CLIMATE.state = "refused by the kernel's sha256 check"; });
}
// ---- the Climate data map: one equirectangular texture (L the class code, A the majority's share; NEAREST) on a whole-sphere
// mesh inside the land stencil, over the veil; the palette is a 32 x 1 texture. Mixed cells (the majority under 0.8 of the
// cell, 0.089 of classed cells by CIRISHolon's count) are hatched; a cell with no class is a dark hatch.
const CLF = `#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif
uniform sampler2D T;uniform sampler2D P;uniform float uA;varying vec2 vLL;varying float vz;
void main(){if(vz<0.0)discard;vec4 t=texture2D(T,vec2((vLL.y+180.0)/360.0,(90.0-vLL.x)/180.0));float code=floor(t.r*255.0+0.5);float hatch=step(0.5,fract((gl_FragCoord.x+gl_FragCoord.y)/6.0));
if(code<0.5){gl_FragColor=vec4(0.36,0.38,0.44,uA*0.45*hatch);return;}
vec3 c=texture2D(P,vec2((code+0.5)/32.0,0.5)).rgb;float mixed=step(t.a,0.799);gl_FragColor=vec4(c*(1.0-mixed*hatch*0.5),uA*0.88);}`;
const CLV = `attribute vec3 p;attribute vec2 ll;uniform mat3 R;uniform float S;uniform vec2 asp;varying vec2 vLL;varying float vz;void main(){vec3 q=R*p;vz=q.z;vLL=ll;gl_Position=vec4(q.x*S*asp.x,q.y*S*asp.y,0.9,1.0);}`;
let pClim = null;
function climateTexSlice(dl){
  CLIMATE.texIdle = false; const t0 = performance.now(), st = CLIMATE.texStep, W = 2880 / st, H = 1440 / st, rows = 48, C = CLIMATE.cells, S = CLIMATE.shareCells;
  gl.bindTexture(gl.TEXTURE_2D, CLIMATE.tex); gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
  do { const y0 = CLIMATE.texRow, n = Math.min(rows, H - y0), D = new Uint8Array(W * n * 2);
    for (let y = 0; y < n; y++) for (let x = 0; x < W; x++){ const k = (y0 + y) * st * 2880 + x * st, o = 2 * (y * W + x); D[o] = C[k]; D[o + 1] = S ? S[k] : 255; }
    gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, y0, W, n, gl.LUMINANCE_ALPHA, gl.UNSIGNED_BYTE, D); CLIMATE.texRow += n;
  } while (CLIMATE.texRow < H && dl.timeRemaining() > 3);
  const dt = performance.now() - t0; CLIMATE.ms.upload += dt; CLIMATE.longest = Math.max(CLIMATE.longest, dt);
  if (CLIMATE.texRow < H){ CLIMATE.texIdle = true; climateIdle(climateTexSlice); } else if (typeof gfxDirty === "function") gfxDirty(400);
}
function climateTex(){
  if (!CLIMATE.cells) return null;
  if (!CLIMATE.tex){
    const max = gl.getParameter(gl.MAX_TEXTURE_SIZE); CLIMATE.texStep = max >= 2880 ? 1 : 2; // a GPU that cannot hold 2,880 columns gets every other cell
    CLIMATE.tex = gl.createTexture(); gl.bindTexture(gl.TEXTURE_2D, CLIMATE.tex);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.LUMINANCE_ALPHA, 2880 / CLIMATE.texStep, 1440 / CLIMATE.texStep, 0, gl.LUMINANCE_ALPHA, gl.UNSIGNED_BYTE, null);
    for (const [k, v] of [[gl.TEXTURE_MIN_FILTER, gl.NEAREST], [gl.TEXTURE_MAG_FILTER, gl.NEAREST], [gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE], [gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE]]) gl.texParameteri(gl.TEXTURE_2D, k, v);
    CLIMATE.texRow = 0; CLIMATE.texShare = !!CLIMATE.shareCells;
    const P = new Uint8Array(32 * 4); for (let c = 1; c <= 30; c++){ const q = climateColour(c); P.set([q[0], q[1], q[2], 255], 4 * c); }
    CLIMATE.pal = gl.createTexture(); gl.bindTexture(gl.TEXTURE_2D, CLIMATE.pal); gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1); gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, 32, 1, 0, gl.RGBA, gl.UNSIGNED_BYTE, P);
    for (const [k, v] of [[gl.TEXTURE_MIN_FILTER, gl.NEAREST], [gl.TEXTURE_MAG_FILTER, gl.NEAREST], [gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE], [gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE]]) gl.texParameteri(gl.TEXTURE_2D, k, v);
  }
  if (CLIMATE.shareCells && !CLIMATE.texShare){ CLIMATE.texShare = true; CLIMATE.texRow = 0; } // the share arrived: upload again with it
  const H = 1440 / CLIMATE.texStep; if (CLIMATE.texRow < H){ if (!CLIMATE.texIdle){ CLIMATE.texIdle = true; climateIdle(climateTexSlice); } return CLIMATE.texRow > 0 && CLIMATE.texShare === !!CLIMATE.shareCells ? CLIMATE.tex : null; }
  return CLIMATE.tex;
}
// called inside the land stencil (globe.js, beside the Power map); a: the zoom fade
function climateDraw(R, S, asp, a){
  if (typeof DATAMAP === "undefined") return;
  const k = Math.min(1, (performance.now() - DATAMAP.t0) / 400), f = (DATAMAP.active === 7 ? k : 0) + (DATAMAP.prev === 7 && k < 1 ? 1 - k : 0);
  if (DATAMAP.active === 7) climateLoad(true);
  if (a <= 0.01 || f <= 0.01) return; const tex = climateTex(); if (!tex) return;
  if (typeof incellVeil === "function") incellVeil(a * 0.82 * Math.min(1, f));
  if (!CLIMATE.mesh){ const V = []; for (let la = -90; la < 90; la += 3) for (let lo = -180; lo < 180; lo += 3){ for (const [y, x] of [[la, lo], [la, lo + 3], [la + 3, lo + 3], [la, lo], [la + 3, lo + 3], [la + 3, lo]]) V.push(...xyz(y, x), y, x); }
    const b = gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER, b); gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(V), gl.STATIC_DRAW); CLIMATE.mesh = { b, n: V.length / 5 }; }
  if (!pClim) pClim = prog(CLV, CLF);
  gl.useProgram(pClim);
  gl.uniformMatrix3fv(gl.getUniformLocation(pClim, "R"), false, R); gl.uniform1f(gl.getUniformLocation(pClim, "S"), S); gl.uniform2fv(gl.getUniformLocation(pClim, "asp"), asp); gl.uniform1f(gl.getUniformLocation(pClim, "uA"), a * f);
  gl.uniform1i(gl.getUniformLocation(pClim, "T"), 0); gl.uniform1i(gl.getUniformLocation(pClim, "P"), 1);
  gl.activeTexture(gl.TEXTURE1); gl.bindTexture(gl.TEXTURE_2D, CLIMATE.pal); gl.activeTexture(gl.TEXTURE0); gl.bindTexture(gl.TEXTURE_2D, tex);
  const ap = gl.getAttribLocation(pClim, "p"), al = gl.getAttribLocation(pClim, "ll"); gl.bindBuffer(gl.ARRAY_BUFFER, CLIMATE.mesh.b); gl.enableVertexAttribArray(ap); gl.enableVertexAttribArray(al);
  gl.vertexAttribPointer(ap, 3, gl.FLOAT, false, 20, 0); gl.vertexAttribPointer(al, 2, gl.FLOAT, false, 20, 12); gl.drawArrays(gl.TRIANGLES, 0, CLIMATE.mesh.n); gl.disableVertexAttribArray(al); gl.disableVertexAttribArray(ap);
  if ((k < 1 || CLIMATE.texRow < 1440 / CLIMATE.texStep) && typeof gfxDirty === "function") gfxDirty(100);
}
// ---- the legend and the chip line (globe.js dmLegend / dmValue, map id 7)
function climateLegend(){
  climateLoad(true); const m = climateMeta();
  const sw = Object.entries(CLIMATE_GROUPS).map(([, g]) => `<span class="vd" style="border-color:rgb(${g.rgb.join(",")});background:rgba(${g.rgb.join(",")},.35)">${g.plain}</span>`).join(" ");
  const head = `<div class="dm-l1 dm-wrap"><b>Climate</b> ${fig("holon-appearance0", CLIMATE.cells ? "the climate class of each 1/8° cell (Köppen-Geiger, 1991 to 2020): a key the street's built form may follow, not how a place looks" : "the climate class of each 1/8° cell: " + (CLIMATE.err ? "not loaded (" + CLIMATE.err + ")" : "loading…"))}</div>`;
  const s = m ? m.data.stakes : null, more = (html) => `<button type="button" class="dm-more-b" data-dm-more aria-expanded="${DM_MORE.open}">where this comes from ${DM_MORE.open ? "▾" : "▸"}</button>${DM_MORE.open ? `<div class="dm-more">${html} <span class="dm-gloss">${fig("holon-verdict-count", HOL_GLOSS + ".")}</span></div>` : ""}`;
  const k = CLIMATE.k, kern = k.state === "held" ? "the kernel holds it (version " + k.version + ", its own sha256 check passed)" : k.state === "feeding" ? "the kernel is taking it in" : k.state === "refused" ? "the kernel refused it (code " + k.code + ")" : "not handed to the kernel";
  return head + `<div class="dm-l2 dm-wrap">${fig("holon-appearance0", sw)} <span class="vd dm-noref">no class</span> <span class="vd vd-unres">hatched: mixed cell</span> ${CLIMATE_CREDIT} ${more("Table: " + holVd("unresolved", "CIRISHolon APPEARANCE-0 v" + (m ? m.version : "?") + ", unresolved: a reduction of a published map, nothing scored against observations") + ". " +
    fig("holon-appearance0", "Each cell shows the class holding most of its 1 km pixels (a tie to the lower code); the majority holds under 0.8 of the cell in " + (s ? s.S1["share_of_classed_cells_with_majority_under_0.8"] : "?") + " of classed cells, most in the temperate and cold groups where relief and coasts mix classes (hatched). 12 of 12 known places read the textbook's class. The classes are a model's output from gridded climate for 1991 to 2020, not observations at a place and not a year. A class is not land: a cell with one classed pixel in 225 has one; the colour is drawn only on the twin's own land. Nothing drawn elsewhere on the globe follows the class yet; a built-form table keyed on it can reach the street's layout through the kernel's table door, and none exists. The colours are the page's (one hue per main group), not the source map's. Grid " + (CLIMATE.cells ? "checked against its sha256" : "not loaded") + "; " + kern + "."))}</div>`;
}
function climateValue(lat, lon){
  climateLoad(true); const c = climateAt(lat, lon);
  if (!c) return "climate here: " + (CLIMATE.err ? "not loaded" : "loading the climate grid…");
  if (!c.code) return `climate here: no class for this cell ${CLIMATE_CREDIT}`;
  return `climate here: ${c.symbol}, ${c.name.toLowerCase()} (${c.plain}${c.share !== null ? "; the majority class of this 1/8° cell, " + Math.round(c.share * 100) + "% of it" + (c.mixed ? ", a mixed cell" : "") : ""}${c.land === false ? "; the twin's land cover reads sea here" : ""}) ${CLIMATE_CREDIT}`;
}
// ---- the US state of a cell (CIRISHolon REGION-0, docs/geo/holon/region0.json; Natural Earth 1:50m admin-1, public domain): the
// state's federal code per cell, the winner's share of the points in any state, and how many of the cell's 25 sample points
// lie in any state (8 to 11 kB each, packed). A key for a generator that does not ship yet; shown in the street's panel.
// Loaded lazily (the street panel), checked like the climate grid, and handed to the kernel's state slots.
// The twin's rules, each a hazard REGION-0 names: a code is not land and not "mostly this state", so a cell takes its state
// only with 13 or more of its 25 points in any state, inside the twin's own United States mask (Natural Earth admin-0), on
// the twin's land; lake water inside a state's outline carries its code, and the twin's land decides what is drawn; US land
// with no code (small islands, the westernmost Aleutians) and cells under the count fall back to the country's row, and
// say so. The winner's share of the WHOLE cell is its points over 25: share x count / 255 / 25.
const STATE = { want: false, state: "idle", code: null, share: null, count: null, err: null, k: { code: "idle", count: "idle" } };
const STATE_FILES = ["us_state_cell.bin.gz", "us_state_cell_share.bin.gz", "us_state_cell_classed.bin.gz"], STATE_MIN_POINTS = 13;
const STATE_WHY = ["its own", "no state table", "no state code here (small islands and the westernmost Aleutians are not on the map): the country's row", "fewer than 13 of 25 points in any state (coast or border): the country's row", "outside the twin's United States: no state", "water in the twin's land cover (a lake inside a state's outline carries its code): no state drawn"];
// the same reasons in plain words for the street panel
const STATE_PLAIN = ["", "the state map is not loaded", "this spot is not on the state map: small islands and the far Aleutians are left off it", "this spot is too close to a coast or border to say which state", "outside the United States", "this spot is water on the twin's map"];
function stateMeta(){ return typeof HOLT !== "undefined" && HOLT.t && HOLT.t.region0 ? HOLT.t.region0 : null; }
function stateLoad(){
  if (STATE.want) return; STATE.want = true;
  const go = () => { const m = stateMeta(); if (!m){ if (typeof holLoad === "function") holLoad(); setTimeout(go, 300); return; }
    STATE.state = "fetching";
    Promise.all(STATE_FILES.map(async (f) => { const b = await climateFetch(f, "region0"); if ((await climateCheck(b, m.data.files[f].sha256_unpacked)) === false) throw new Error(f); return b; }))
      .then(([c, sh, n]) => { STATE.code = c; STATE.share = sh; STATE.count = n; STATE.state = "held";
        doorPut("state", m.ref.split(" ")[0], m.version, m.data.files[STATE_FILES[0]].sha256_unpacked, c, (r) => { STATE.k.code = r.state; });
        doorPut("state_count", m.ref.split(" ")[0], m.version, m.data.files[STATE_FILES[2]].sha256_unpacked, n, (r) => { STATE.k.count = r.state; }); })
      .catch((e) => { STATE.state = "failed"; STATE.err = e.message; console.warn(e.message); }); };
  go();
}
// the state rule on a cell's published values: { used: code or 0, why: index into STATE_WHY } (the kernel's door::Door::state_of)
function stateRule(raw, points, inUs, land){
  if (!raw) return { used: 0, why: inUs ? 2 : 4 };
  if (points < STATE_MIN_POINTS) return { used: 0, why: 3 };
  if (!inUs) return { used: 0, why: 4 };
  if (land === false) return { used: 0, why: 5 };
  return { used: raw, why: 0 };
}
// The state at a point: { raw, postal, name, points, share, whole, used, why } once the grids are in; null before.
function stateAt(lat, lon){
  if (!STATE.code) return null; const [r, c] = climateCellOf(lat, lon, 1), k = r * 2880 + c, raw = STATE.code[k], points = STATE.count[k], m = stateMeta();
  const inUs = typeof HOLT !== "undefined" && HOLT.countries && typeof countryNear === "function" ? countryNear(HOLT.countries, lat, lon) === 840 : false;
  const lc = typeof geoLandcoverAt === "function" ? geoLandcoverAt(lat, lon, 1) : null, land = lc ? !climateWater(lc.code) : null;
  const e = raw && m ? m.data.states[raw] : null, rule = stateRule(raw, points, inUs, land);
  return { raw, postal: e ? e.postal : null, name: e ? e.name : null, points, share: STATE.share[k] / 255, whole: points ? Math.round(STATE.share[k] * points / 255) / 25 : 0, inUs, land, ...rule };
}
// ---- what a street's built form was read from (the focal leaf's layout record, tiling.js tlBuildLeaf e.form)
const FORM_SOURCE = ["the built-in default", "the loaded table's world row", "the loaded table's class × zone row"];
const POOL = ["the state's own rows", "its division's rows", "borrowed rows"];
function formLine(){
  if (typeof M === "undefined" || !M || M.leaf === undefined || typeof TL === "undefined") return "built form: –";
  const v = typeof wasm !== "undefined" && wasm && wasm.door_versions ? (wasm.door_versions(), Array.from(new Float64Array(wasm.memory.buffer, wasm.holon_buf_ptr(0) >>> 0, 5))) : [0, 0, 0, 0, 0];
  const e = TL.lay.get(M.leaf), f = e && e.form;
  // the state used for this street, by the twin's rules on REGION-0 (shown even though no generator reads it yet)
  if (M.lat0 > 15 && M.lat0 < 72 && (M.lon0 < -60 || M.lon0 > 170)) stateLoad();
  const st = stateAt(M.lat0, M.lon0);
  // plain words: the state's name, or "none" and why; the pooling words wait for a generator that returns them
  const stRaw = !st ? "" : st.used ? `; state for this street: ${st.name}` :
    st.inUs ? `; state for this street: none, the United States as a whole is used (${STATE_PLAIN[st.why]})` : "";
  const stLine = stRaw && typeof fig === "function" ? fig("holon-region0", stRaw) : stRaw;
  // with no form table in the kernel every street reads the built-in rows; with one, the street's own record says which
  if (!f) return (v[1] ? "built form: this street's layout is not in the cache yet" : "built form: the built-in default (MODEL, visual rules: widths, storeys, setbacks by street class; no table of built form exists)") + stLine + (st ? " (state lines: Natural Earth, public domain, via CIRISHolon REGION-0)" : "");
  const zone = f.climate ? CLIMATE_SYMBOLS[f.climate - 1] : null;
  return `built form: ${FORM_SOURCE[f.source] || "?"}${f.source === 0 ? " (MODEL, visual rules: widths, storeys, setbacks by street class; no table of built form exists)" : ""}${zone && f.source === 2 ? "; keyed on climate class " + zone : ""}; a built-form generator: ${v[4] ? "compiled in" : "none ships (CIRISHolon's first was refused by its own tests)"}${f.generated ? ", " + f.generated + " buildings drawn by it from " + (f.pooling ? ["footprint", "storeys", "dwellings", "spacing"].map((k, i) => k + " " + POOL[f.pooling[i]]).join(", ") + (f.mixed ? " (later buildings pooled differently)" : "") : "?") : ""}${stLine}${st ? " (state lines: Natural Earth, public domain, via CIRISHolon REGION-0)" : ""}`;
}
// the map's program compiled ahead in an idle slice after load (as incell.js does: a first use compiled inside a frame is a long task)
if (typeof window !== "undefined") (window.requestIdleCallback || ((f) => setTimeout(f, 3000)))(() => { try { if (!pClim && typeof gl !== "undefined" && gl) pClim = prog(CLV, CLF); } catch (e) { console.warn("climate prewarm", e); } }, { timeout: 7000 });
if (typeof module !== "undefined") module.exports = { CLIMATE_GROUPS, CLIMATE_SYMBOLS, climateColour, climateCode, CLIMATE_CREDIT, FORM_SOURCE, stateRule, STATE_MIN_POINTS, STATE_WHY, climateWater };
