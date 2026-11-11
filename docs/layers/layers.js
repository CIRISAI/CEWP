// Layers: each draws only what a kernel export produced, and says which export, its buffer layout,
// how often it is read, and the class of what it draws (FSD §8.2: MEASURED, DERIVED, MODEL,
// PROJECTED, FRONTIER). The registry is public (window.CEWP_LAYERS) so the evidence panels can make
// any layer inspectable without touching this file; `last` holds the numbers drawn this frame.
//
// A layer whose export the loaded kernel does not have is not shown at all (`available()`); the
// contracts for the packages still being built are written down here so their views fit.
(() => {
const $id = (x) => document.getElementById(x);
const F64 = (which) => new Float64Array(mem.buffer, wasm.globe_buf_ptr(which) >>> 0, wasm.globe_buf_len(which) >>> 0);
const U32 = (which) => new Uint32Array(mem.buffer, wasm.globe_buf_ptr(which) >>> 0, wasm.globe_buf_len(which) >>> 0);
const has = (...names) => () => !!(wasm && names.every((n) => typeof wasm[n] === "function"));
const fmt = (x) => !isFinite(x) ? "–" : x >= 1e6 ? (x / 1e6).toFixed(2) + "M" : x >= 1e4 ? (x / 1e3).toFixed(1) + "k" : Math.round(x).toLocaleString();
const proj = (F, la, lo) => projLL(F, la, lo); // [x, y, z]; z < 0 is behind the globe
const KEEP = { on: new Set(["illustration", "glow", "arcs"]) };
try { const s = localStorage.getItem("cewp.layers"); if (s) KEEP.on = new Set(JSON.parse(s)); } catch (e) {}
const save = () => { try { localStorage.setItem("cewp.layers", JSON.stringify([...KEEP.on])); } catch (e) {} };

const REG = [];
// `last` is a flat name -> number map (the evidence card prints it); strings and lists go in `info`.
function layer(def){ def.last = {}; def.info = {}; REG.push(def); return def; }
window.CEWP_LAYERS = {};

// ------------------------------------------------------------------ illustration (not data)
window.CEWP_LAYERS["illustration"] = layer({ id: "illustration", title: "Ocean colour, glint and atmosphere", cls: "MODEL",
  source: "gfx.js gfxOcean (shader only; reads no kernel export)",
  contract: "Decoration. It draws no data: the ocean's colour, the sun's glint and the atmosphere's limb are a shading illustration lit by the sim sun (clock_sun) and nothing else. Off, the globe falls back to the plain ocean disc of f73b918 with no atmosphere.",
  available: () => true, status(){ this.last = {}; return "decoration, no data"; } });
window.CEWP_LAYERS["glow"] = layer({ id: "glow", title: "Night side lit by where people are", cls: "MODEL", decorative: false,
  source: "population grid (globe.js POP_GRID) · clock_sun",
  contract: "The population grid the sim places people on (MODEL input, 1.5° cells, scattered ±cellRad), blurred 3 px into a 1024×512 image; lit where the sun from clock_sun(t) is below the horizon. Not satellite night lights. Read once at load; the sun every frame.",
  available: () => true, status(){ this.last = { sun_lat_deg: +GFX.sunLL[0].toFixed(2), sun_lon_deg: +GFX.sunLL[1].toFixed(2), utc_hour: +GFX.utc.toFixed(3) }; return "sun over " + GFX.sunLL.map((v) => v.toFixed(1) + "°").join(", ") + " · " + String(Math.floor(GFX.utc)).padStart(2, "0") + ":" + String(Math.floor((GFX.utc % 1) * 60)).padStart(2, "0") + " UTC"; } });

// ------------------------------------------------------------------ the tick's sample arcs
window.CEWP_LAYERS["arcs"] = layer({ id: "arcs", title: "Deliveries crossing localities (a sample)", cls: "DERIVED",
  source: "tick → arcs buffer (u32 × 2: signer, target | tier << 28)",
  contract: "tick() writes up to 420 (signer, target, tier) pairs, itself a 1-in-3 to 1-in-48 sample of the deliveries it ran; the page draws at most 10 a tick. Tier: 0 shaped hop, 1 seat replication, 2 relay tree / unshaped, 3 onion. Read every tick (120 ms). The exact counts are on the flows layer.",
  available: () => true, status(){ this.last = { arcs_on_screen: GFX.arcsDrawn || 0 }; return this.last.arcs_on_screen + " arcs on screen"; } });

// ------------------------------------------------------------------ flows between localities (exact counts)
const FLOW = { cent: null, W: null, win: [], sum: null, arcs: [], top: [], checked: "", N: 0 };
const FLOW_WINDOW = 25; // ticks summed (3 s of sim at 120 ms a tick)
const FLOW_TOP = 60;
window.CEWP_LAYERS["flows"] = layer({ id: "flows", title: "Flows between localities (exact counts)", cls: "DERIVED",
  source: "tick → globe buffer 10 (flows) · locality_centroids → buffer 2",
  contract: "During tick() the kernel counts every delivery that leaves its signer's locality, the same events as out[6] + out[7], into a 181 × 181 u32 matrix (row = from, column = to), buffer 10. Positions: locality_centroids(lat, lon, metro, n), buffer 2, f64 × 3 (lat°, lon°, nodes). The page sums the last 25 ticks and draws the 60 largest pairs, ribbon width ∝ log(count). Checked every tick: Σ matrix = out[6] + out[7].",
  available: has("globe_buf_ptr", "locality_centroids"),
  onTick(){
    if (FLOW.W !== W){ wasm.locality_centroids(bufs.lat, bufs.lon, bufs.metro, W.n); FLOW.cent = F64(2).slice(); FLOW.W = W; FLOW.win = []; FLOW.sum = null; }
    const m = U32(10).slice(); FLOW.N = Math.round(Math.sqrt(m.length));
    const out = new Float64Array(mem.buffer, bufs.out, OUT); let tot = 0; for (let i = 0; i < m.length; i++) tot += m[i];
    const want = out[6] + out[7];
    Object.assign(this.last, { matrix_sum: tot, out6_plus_out7: want, check_ok: tot === want ? 1 : 0 });
    FLOW.checked = tot === want ? "Σ = out[6]+out[7] ✓" : "MISMATCH Σ " + tot + " ≠ " + want;
    FLOW.win.push(m); if (FLOW.win.length > FLOW_WINDOW) FLOW.win.shift();
    const S = new Uint32Array(m.length); for (const w of FLOW.win) for (let i = 0; i < w.length; i++) S[i] += w[i];
    FLOW.sum = S; this.rebuild();
  },
  rebuild(){
    const S = FLOW.sum, C = FLOW.cent, N = FLOW.N; if (!S || !C) return;
    const pairs = []; for (let i = 0; i < S.length; i++) if (S[i] > 0) pairs.push(i);
    pairs.sort((a, b) => S[b] - S[a]); const top = pairs.slice(0, FLOW_TOP);
    const mx = top.length ? S[top[0]] : 1; const now = getTime();
    FLOW.top = top.map((k) => ({ from: Math.floor(k / N), to: k % N, n: S[k] }));
    FLOW.arcs = FLOW.top.filter((f) => C[f.from * 3 + 2] > 0 && C[f.to * 3 + 2] > 0).map((f) => {
      const a = xyz(C[f.from * 3], C[f.from * 3 + 1]), b = xyz(C[f.to * 3], C[f.to * 3 + 1]);
      const th = Math.acos(Math.max(-1, Math.min(1, a[0] * b[0] + a[1] * b[1] + a[2] * b[2])));
      return { a, b, birth: 0, life: 0, tier: 4, width: 0.7 + 2.6 * Math.log1p(f.n) / Math.log1p(mx), kind: 1, lift: 0.01 + 0.09 * th / Math.PI, f };
    });
    let total = 0; for (let i = 0; i < S.length; i++) total += S[i];
    Object.assign(this.last, { ticks: FLOW.win.length, crossings: total, pairs: pairs.length, drawn: FLOW.arcs.length }); this.info.top = FLOW.top.slice(0, 5);
  },
  arcs(){ return FLOW.arcs; },
  draw2D(g, F){ // count labels on the five largest, at the arc's midpoint
    if (!FLOW.cent) return; g.font = "600 11px " + MFONT; g.textAlign = "center"; g.textBaseline = "middle";
    for (const r of FLOW.arcs.slice(0, 5)){ const m = [(r.a[0] + r.b[0]), (r.a[1] + r.b[1]), (r.a[2] + r.b[2])]; const L = Math.hypot(...m) || 1;
      const la = Math.asin(m[1] / L) * 180 / Math.PI, lo = Math.atan2(m[0], m[2]) * 180 / Math.PI; const p = proj(F, la, lo); if (p[2] < 0.05) continue;
      const t = fmt(r.f.n); g.lineWidth = 3; g.strokeStyle = "rgba(6,9,14,0.9)"; g.strokeText(t, p[0], p[1]); g.fillStyle = "#f4e7c8"; g.fillText(t, p[0], p[1]); }
    g.textAlign = "left";
  },
  status(){ const w = this.last; return w.ticks ? fmt(w.crossings) + " crossings in " + w.ticks + " ticks · " + FLOW.checked : "waiting for a tick"; } });

// ------------------------------------------------------------------ one room's member relay tree (alm.rs)
const ALM = { room: -1, members: null, edges: null, unrooted: null, mf: null, arcs: [], pick: 0 };
window.CEWP_LAYERS["alm"] = layer({ id: "alm", title: "A room's relay tree (CC 6.1.6)", cls: "DERIVED", inputs: "MODEL",
  source: "alm_room → globe buffers 3 members, 4 edges, 5 unrooted, 6 member facts",
  contract: "alm_room(lat, lon, room_off, room_mem, room, publisher, max_members = 64, seed) runs cewp_sim::alm::compute_alm_topology_verified_with_publisher on the first 64 members of the region's public room. Inputs are MODEL: uplink by device class (phone cellular 2, phone Wi-Fi 8, laptop 12, home node 20 Mbit/s), RTT = 10 ms + fibre at 200 km/ms over 1.6× the great circle, every member trusting every other at depth 1. This is the CC 6.1.6 reference construction cewp-sim checks conformance against, not the closed-loop controller a live edge v45 call runs (Admission / ParentSelector); the media layer will show that one. Buffer 4: u32 × 3 (parent ix, child ix, child depth). Buffer 6: f64 × 4 (class, uplink, depth or -1, children). Recomputed when shown or when the room changes.",
  available: has("alm_room"),
  compute(){
    if (!W || !bufs) return;
    const la = pitch * 180 / Math.PI, lo = ((yaw * 180 / Math.PI + 540) % 360) - 180;
    // the region of the locality nearest the screen centre (centroids from the kernel)
    if (FLOW.W !== W){ wasm.locality_centroids(bufs.lat, bufs.lon, bufs.metro, W.n); FLOW.cent = F64(2).slice(); FLOW.W = W; }
    let best = 0, bd = 1e9; const C = FLOW.cent;
    for (let k = 0; k < C.length / 3; k++){ if (!C[k * 3 + 2]) continue; const d = kmf(la, lo, C[k * 3], C[k * 3 + 1]); if (d < bd){ bd = d; best = W.regionOfMetro[k]; } }
    const room = (ALM.pick % 2 === 0) ? W.regionRoom[best] : W.globalRoom; ALM.room = room; ALM.region = best;
    const pub = W.roomMem[W.roomOff[room]];
    const ne = wasm.alm_room(bufs.lat, bufs.lon, bufs.roomOff, bufs.roomMem, room, pub, 64, 0x2026 + ALM.pick);
    ALM.members = U32(3).slice(); ALM.edges = U32(4).slice(); ALM.unrooted = U32(5).slice(); ALM.mf = F64(6).slice();
    let depth = 0, maxKids = 0; for (let i = 0; i < ALM.members.length; i++){ depth = Math.max(depth, ALM.mf[i * 4 + 2]); maxKids = Math.max(maxKids, ALM.mf[i * 4 + 3]); }
    ALM.arcs = []; for (let e = 0; e < ne; e++){ const p = ALM.members[ALM.edges[e * 3]], c = ALM.members[ALM.edges[e * 3 + 1]], d = ALM.edges[e * 3 + 2];
      const a = xyz(W.lat[p], W.lon[p]), b = xyz(W.lat[c], W.lon[c]); const th = Math.acos(Math.max(-1, Math.min(1, a[0] * b[0] + a[1] * b[1] + a[2] * b[2])));
      ALM.arcs.push({ a, b, birth: 0, life: 0, tier: 2, width: Math.max(1.2, 4.2 - d * 0.8), kind: 1, lift: 0.006 + 0.07 * th / Math.PI }); }
    this.info = { region: REG_NAMES[best] || String(best) }; this.last = { room, region: best, members: ALM.members.length, edges: ne, depth, max_children: maxKids, fanout_bound: wasm.alm_bounds(0), rtt_ceiling_ms: wasm.alm_bounds(1), unrooted: ALM.unrooted.length };
  },
  onShow(){ this.compute(); }, arcs(){ return ALM.arcs; },
  next(){ ALM.pick++; this.compute(); gfxDirty(800); },
  draw2D(g, F){
    if (!ALM.members) return; const n = ALM.members.length;
    for (let i = 0; i < n; i++){ const m = ALM.members[i], p = proj(F, W.lat[m], W.lon[m]); if (p[2] < 0.02) continue; const d = ALM.mf[i * 4 + 2];
      g.lineWidth = 1.5; g.strokeStyle = "rgba(6,9,14,0.9)"; g.fillStyle = d < 0 ? "rgba(0,0,0,0)" : "#7cf0b0";
      if (i === 0){ g.beginPath(); g.moveTo(p[0], p[1] - 7); g.lineTo(p[0] + 6, p[1] + 4); g.lineTo(p[0] - 6, p[1] + 4); g.closePath(); g.fillStyle = "#fff6d8"; g.fill(); g.stroke(); continue; } // publisher: triangle
      if (d < 0){ g.strokeStyle = "#ff8a7a"; g.lineWidth = 2; g.beginPath(); g.moveTo(p[0] - 4, p[1] - 4); g.lineTo(p[0] + 4, p[1] + 4); g.moveTo(p[0] + 4, p[1] - 4); g.lineTo(p[0] - 4, p[1] + 4); g.stroke(); continue; } // unrooted: a cross
      g.beginPath(); g.arc(p[0], p[1], Math.max(2, 4.5 - d * 0.6), 0, 6.2832); g.fill(); g.stroke(); }
  },
  status(){ const l = this.last; return l.members ? `${l.members} members · depth ${l.depth} · busiest parent ${l.max_children} children (bound ${l.fanout_bound}) · ${l.unrooted} unrooted` : "–"; },
  controls: [{ label: "Next room", act(){ this.next(); } }] });
const REG_NAMES = ["N. America", "Latin America", "Europe", "MENA", "Sub-Saharan Africa", "South Asia", "East Asia", "SE Asia & Oceania", "Russia & C. Asia"];

// ------------------------------------------------------------------ a room's chunk custody (chunk_custody.rs)
const CUST = { reach: 1000, holders: null, c: null, pick: 0 };
window.CEWP_LAYERS["custody"] = layer({ id: "custody", title: "Who holds a room's chunk", cls: "DERIVED", inputs: "MODEL (reachability draw)",
  source: "custody_room → globe buffers 7 (summary), 8 (holders)",
  contract: "custody_room(lat, lon, metro, room_off, room_mem, room, present_pm, seed): the room's members are the sealed chunk's holders; present_pm (the slider) is the share reachable, drawn per holder (a MODEL stand-in until the failures package supplies who is down). The kernel runs chunk_custody::classify(present, 20, 6) with the tick's own thresholds and Kish n_eff (effective_source_count) over the holders' locality masses. Buffer 7 f64: members, present, target, min viable, state (0 Full, 1 Partial, 2 EnvelopeOnly), n_eff, localities, mean km to centroid, max km. Buffer 8 u32 × 2: holder node, reachable. The room: the first community room of 24-80 members in the relay tree's region (Next room steps on).",
  available: has("custody_room"),
  compute(){
    if (!W || !bufs) return;
    // a community room of 24-80 members in the relay tree's region, so the slider can cross Full,
    // Partial and EnvelopeOnly (a region's public room has thousands of holders)
    const g = ALM.region !== undefined ? ALM.region : 2; let room = -1;
    for (let r = W.nFamilies + CUST.pick; r < W.regionRoom[0]; r++){ const a = W.roomOff[r], n = W.roomOff[r + 1] - a; if (n < 24 || n > 80) continue; if (W.regionOfMetro[W.metro[W.roomMem[a]]] === g){ room = r; break; } }
    if (room < 0) room = W.regionRoom[g];
    wasm.custody_room(bufs.lat, bufs.lon, bufs.metro, bufs.roomOff, bufs.roomMem, room, CUST.reach, 7);
    CUST.c = F64(7).slice(); CUST.holders = U32(8).slice(); const c = CUST.c;
    this.last = { room, members: c[0], present: c[1], target: c[2], min_viable: c[3], state_code: c[4], n_eff: c[5], localities: c[6], mean_km: Math.round(c[7]), max_km: Math.round(c[8]), reachable_permille: CUST.reach }; this.info = { state: ["Full", "Partial", "EnvelopeOnly"][c[4]], state_codes: "0 Full, 1 Partial, 2 EnvelopeOnly" };
  },
  onShow(){ if (ALM.room < 0 && LAYERS.def("alm").available()) LAYERS.def("alm").compute(); this.compute(); },
  draw2D(g, F){
    const H = CUST.holders; if (!H) return;
    for (let i = 0; i < H.length; i += 2){ const m = H[i], up = H[i + 1], p = proj(F, W.lat[m], W.lon[m]); if (p[2] < 0.02) continue;
      g.lineWidth = 1.4; if (up){ g.fillStyle = "#ffd27a"; g.strokeStyle = "rgba(6,9,14,0.9)"; g.fillRect(p[0] - 3, p[1] - 3, 6, 6); g.strokeRect(p[0] - 3, p[1] - 3, 6, 6); }
      else { g.strokeStyle = "#9aa3ad"; g.strokeRect(p[0] - 3, p[1] - 3, 6, 6); g.beginPath(); g.moveTo(p[0] - 3, p[1] + 3); g.lineTo(p[0] + 3, p[1] - 3); g.stroke(); } } // filled = reachable holder; struck box = unreachable
  },
  status(){ const l = this.last; return l.members ? `${this.info.state}: ${l.present} of ${l.members} reachable (Full at ${l.target}, floor ${l.min_viable}) · n_eff ${l.n_eff} over ${l.localities} localities · ${fmt(l.mean_km)} km mean spread` : "–"; },
  controls: [{ label: "Next room", act(){ CUST.pick = (CUST.pick + 97) % 5000; this.compute(); gfxDirty(600); } }, { range: [0, 1000, 50], label: "Holders reachable", value: () => CUST.reach, set(v){ CUST.reach = v; this.compute(); gfxDirty(600); }, show: (v) => (v / 10).toFixed(0) + " %" }] });

// ------------------------------------------------------------------ reachability classes in the street world (micro.rs)
window.CEWP_LAYERS["reach"] = layer({ id: "reach", title: "How each home is reachable (street scale)", cls: "MODEL",
  source: "micro_export → micro buffer 6 households (f64 × 7, field 4 router class) · buffer 1 node flags",
  contract: "The focal world's households from micro_export, buffer 6, f64 × 7: x, y, building, adopting, router (0 public listener: IPv6 or a forwarded port; 1 NAT a facilitated punch can traverse; 2 hard NAT / CGNAT), in community, persons. Router shares are micro.rs PARAM_SPECS (MODEL). Read every frame the street world is on screen. Glyph by class: filled disc, ring, hatched square (never colour alone).",
  available: () => typeof microOk === "function" && microOk(),
  draw2D(g, F){
    if (!M || !M.hh || !(typeof microVisible === "function" && microVisible())) return;
    const A = patchAffine(F); if (A.front <= 0.02) return; const ppm = A.pxPerM; if (ppm < 0.12) return;
    const n = [0, 0, 0]; const s = Math.max(2.5, Math.min(7, ppm * 2.2)); const al = Math.min(1, (ppm - 0.12) * 4);
    g.globalAlpha = al;
    for (let i = 0; i < M.hh.length; i += 7){ const x = M.hh[i], y = M.hh[i + 1], r = M.hh[i + 4] | 0; n[r]++;
      const px = A.a * x + A.c * y + A.e, py = A.b * x + A.d * y + A.f; if (px < -10 || py < -10 || px > F.w + 10 || py > F.h + 10) continue;
      g.lineWidth = 1.3; g.strokeStyle = "rgba(6,9,14,0.85)";
      if (r === 0){ g.fillStyle = "#8ee6ff"; g.beginPath(); g.arc(px, py, s * 0.55, 0, 6.2832); g.fill(); g.stroke(); }
      else if (r === 1){ g.strokeStyle = "#c9b8ff"; g.lineWidth = 2; g.beginPath(); g.arc(px, py, s * 0.5, 0, 6.2832); g.stroke(); }
      else { g.strokeStyle = "#ff9d7a"; g.lineWidth = 1.4; g.strokeRect(px - s * 0.5, py - s * 0.5, s, s); g.beginPath(); g.moveTo(px - s * 0.5, py + s * 0.5); g.lineTo(px + s * 0.5, py - s * 0.5); g.moveTo(px - s * 0.5, py); g.lineTo(px, py - s * 0.5); g.stroke(); }
    }
    g.globalAlpha = 1; this.last = { public_listener: n[0], punchable_nat: n[1], hard_nat: n[2], households: n[0] + n[1] + n[2] };
  },
  status(){ const l = this.last; return l.households ? `● ${l.public_listener} public · ○ ${l.punchable_nat} punchable NAT · ▨ ${l.hard_nat} hard NAT / CGNAT` : "zoom to a street"; } });

// ------------------------------------------------------------------ seat load against its capacity curve (capacity.rs)
const SEAT = { curve: null, key: "" };
window.CEWP_LAYERS["seat"] = layer({ id: "seat", title: "Serve-seat load across fleet sizes", cls: "MODEL", inputs: "calibrated to seat-1 (MEASURED/DERIVED/MODEL per capacity.rs COST_SPECS)",
  source: "cap_curve → globe buffer 9 (f64 × 4: devices, ρ cpu, ρ permits, round latency s)",
  contract: "cap_curve(d0 = 10, d1 = 100,000, n = 64, shape, read, seats, select, cand) runs capacity::run over 64 fleet sizes with the bundle-bootstrap setting the street chip shows (v44 / v45 / whole-plane). Buffer 9 f64 × 4. The patch marker is seatRounders() from the street world through cap_seat (buffer 12, ρ cpu). No knee is printed: the knee is under correction by the calibration package.",
  available: has("cap_curve"),
  compute(){
    const [shape, read, cand] = (typeof bsCap === "function") ? bsCap() : [1, 1, 1];
    const key = shape + "," + read + "," + cand; if (SEAT.key === key && SEAT.curve) return; SEAT.key = key;
    wasm.cap_curve(10, 100000, 64, shape, read, 1, 0, cand); SEAT.curve = F64(9).slice();
  },
  onShow(){ this.compute(); },
  panel(c){ // a small chart inside the dock: ρ (CPU solid, permits dashed) against devices on a log axis
    this.compute(); const C = SEAT.curve; if (!C) return;
    const dpr = window.devicePixelRatio || 1, W2 = c.clientWidth || 260, H2 = 110; if (c.width !== W2 * dpr){ c.width = W2 * dpr; c.height = H2 * dpr; }
    const g = c.getContext("2d"); g.setTransform(dpr, 0, 0, dpr, 0, 0); g.clearRect(0, 0, W2, H2);
    const L = 34, B = H2 - 18, T = 8, Rr = W2 - 22, lx = (d) => L + (Math.log10(d) - 1) / 4 * (Rr - L), ly = (r) => B - Math.min(1.5, r) / 1.5 * (B - T);
    g.strokeStyle = "rgba(200,210,225,0.25)"; g.lineWidth = 1; g.font = "10px " + MFONT; g.fillStyle = "#aab4c2";
    for (const d of [10, 100, 1000, 10000, 100000]){ g.beginPath(); g.moveTo(lx(d), T); g.lineTo(lx(d), B); g.stroke(); g.fillText(d >= 1000 ? d / 1000 + "k" : String(d), lx(d) - 8, H2 - 5); }
    g.setLineDash([2, 3]); g.beginPath(); g.moveTo(L, ly(1)); g.lineTo(Rr, ly(1)); g.stroke(); g.setLineDash([]); g.fillText("ρ=1", 4, ly(1) + 3); g.fillText("0", 18, B);
    const line = (off, dash, col) => { g.strokeStyle = col; g.lineWidth = 1.8; g.setLineDash(dash); g.beginPath(); for (let i = 0; i < C.length; i += 4){ const x = lx(C[i]), y = ly(C[i + off]); i ? g.lineTo(x, y) : g.moveTo(x, y); } g.stroke(); g.setLineDash([]); };
    line(1, [], "#7fd0ff"); line(2, [5, 3], "#ffcf7a");
    // this street's seat, if the street world is up: devices that round with it now, through cap_seat
    let here = null; if (typeof M !== "undefined" && M && typeof seatRounders === "function" && typeof capSeat === "function"){ try { const sids = []; for (let i = 0; i < M.n; i++) if (nodeKind(i) === 4) sids.push(i); if (sids.length){ const d = seatRounders(sids[0]); if (d > 0){ const [sh, rd, cd] = bsCap(); const o = capSeat(d, sh, rd, cd); if (o) here = { devices: d, rho: o.rhoCpu }; } } } catch (e) {} }
    if (here && here.devices >= 10){ const x = lx(here.devices), y = ly(here.rho); g.fillStyle = "#fff"; g.beginPath(); g.arc(x, y, 3.5, 0, 6.2832); g.fill(); }
    this.last = { points: C.length / 4, first_devices: C[0], first_rho_cpu: C[1], last_devices: C[C.length - 4], last_rho_cpu: C[C.length - 3] }; if (here){ this.last.street_devices = here.devices; this.last.street_rho_cpu = here.rho; } this.info = { setting: SEAT.key };
  },
  status(){ const h = this.last; return "solid ρ CPU · dashed ρ permits" + (h.street_devices ? ` · this street's seat: ${h.street_devices} devices round with it, ρ ${h.street_rho_cpu.toFixed(3)}` : "") + " · no knee is marked: the seat model is being recalibrated"; } });

// ------------------------------------------------------------------ package views (kernel-wasm views.rs): each runs once per page
const VW = (slot) => { const p = wasm.view_ptr(slot) >>> 0, n = wasm.view_len(slot) >>> 0; return (wasm.view_is_u32(slot) ? new Uint32Array(mem.buffer, p, n) : new Float32Array(mem.buffer, p, n)).slice(); };
const DEG = Math.PI / 180;
// a view that takes a second or more runs after the dock has said so, not inside the click
function later(d){ if (d.ran) return; d.ran = true; d.busy = true; setTimeout(() => { try { d.compute(); } catch (e) { console.warn("layer " + d.id, e); } d.busy = false; render(); gfxDirty(800); }, 40); }
// a great circle as lat/lon points, one per ~3° (for drawing on the 2D overlay)
function gcLL(la1, lo1, la2, lo2){
  const a = xyz(la1, lo1), b = xyz(la2, lo2); const th = Math.acos(Math.max(-1, Math.min(1, a[0] * b[0] + a[1] * b[1] + a[2] * b[2])));
  const n = Math.max(1, Math.ceil(th / (3 * DEG))); const out = [];
  for (let i = 0; i <= n; i++){ const t = i / n; let x, y, z;
    if (th < 1e-6){ x = a[0]; y = a[1]; z = a[2]; } else { const s = Math.sin(th), p = Math.sin((1 - t) * th) / s, q = Math.sin(t * th) / s; x = p * a[0] + q * b[0]; y = p * a[1] + q * b[1]; z = p * a[2] + q * b[2]; }
    out.push(Math.asin(Math.max(-1, Math.min(1, y))) / DEG, Math.atan2(x, z) / DEG); }
  return out;
}
function strokeLL(g, F, ll){ let pen = false; for (let i = 0; i < ll.length; i += 2){ const p = proj(F, ll[i], ll[i + 1]); if (p[2] < 0.02){ pen = false; continue; } if (pen) g.lineTo(p[0], p[1]); else { g.moveTo(p[0], p[1]); pen = true; } } }

// ------------------------------------------------------------------ topology (twin-topology: netgraph/view.rs)
const TOPO = { seed: 7, pts: null, links: null, segs: null, path: null, pick: 0, peering: false, fixed: [] };
const TOPO_ROLES = ["fixed access AS", "IXP", "regional transit AS", "content / data-centre AS", "metro", "mobile access AS", "tier-1 AS"];
const TOPO_KINDS = ["provider to customer", "peering", "IXP membership", "terrestrial long-haul", "submarine long-haul"];
window.CEWP_LAYERS["topology"] = layer({ id: "topology", title: "Networks, exchanges and cables (generated)", cls: "MODEL",
  source: "view_topology(7) → view slots 0 points, 1 links, 15 field classes · view_topology_path → slot 2 (cewp_sim::netgraph::view::TopologyView)",
  contract: "The network graph the twin routes over, as twin-topology's TopologyView builds it once per page. Points f32 × 5 (lat, lon, role 0 fixed access AS, 1 IXP, 2 regional transit, 3 content / data centre, 4 metro, 5 mobile access, 6 tier-1, log10 size, region): metros are the page's 181 localities (real places at city grain); every AS, IXP, relationship and cable is invented. AS and IXP positions are a display spiral around their home metro, not locations. Links u32 × 3 (a, b, kind 0 provider-customer, 1 peering, 2 IXP membership, 3 terrestrial, 4 submarine). Drawn by default: kinds 0, 3, 4; peering and IXP membership behind the toggle. The route: TopologyView::path between two fixed access ASes, header [AS hops, long-haul km]. All MODEL (view_classes). No AS, IXP or cable dataset is approved; a real one drops in behind NetGraph.",
  available: has("view_topology", "view_ptr"),
  compute(){
    if (TOPO.pts) return;
    wasm.view_topology(TOPO.seed); TOPO.pts = VW(0); TOPO.links = VW(1); TOPO.classes = VW(15);
    const P = TOPO.pts, L = TOPO.links; TOPO.segs = [];
    for (let i = 0; i < L.length; i += 3){ const a = L[i], b = L[i + 1], k = L[i + 2]; TOPO.segs.push({ k, ll: gcLL(P[a * 5], P[a * 5 + 1], P[b * 5], P[b * 5 + 1]) }); }
    TOPO.fixed = []; TOPO.metros = 0; for (let p = 0; p < P.length / 5; p++){ if (P[p * 5 + 2] === 0) TOPO.fixed.push(p); if (P[p * 5 + 2] === 4) TOPO.metros++; }
    this.route();
  },
  route(){ // a deterministic walk over pairs of fixed access ASes on different continents' halves of the list
    const f = TOPO.fixed; if (f.length < 2) return; const k = TOPO.pick;
    const a = f[(k * 37) % f.length], b = f[f.length - 1 - ((k * 53) % f.length)];
    const n = wasm.view_topology_path(a, b); TOPO.path = n >= 2 ? VW(2) : null; TOPO.pathAB = [a, b];
    if (TOPO.path){ const w = TOPO.path; TOPO.pathLL = []; for (let i = 2; i + 3 < w.length; i += 2) TOPO.pathLL.push(...gcLL(w[i], w[i + 1], w[i + 2], w[i + 3])); }
  },
  onShow(){ this.compute(); },
  draw2D(g, F){
    if (!TOPO.pts) return; const P = TOPO.pts;
    const style = [[ "rgba(150,190,230,0.16)", 0.6, [] ], [ "rgba(200,180,255,0.35)", 0.8, [1, 3] ], [ "rgba(255,210,140,0.30)", 0.7, [1, 2] ], [ "rgba(160,230,200,0.55)", 1.3, [] ], [ "rgba(120,200,255,0.60)", 1.3, [6, 4] ]];
    for (let k = 0; k < 5; k++){ if ((k === 1 || k === 2) && !TOPO.peering) continue; const [c, w, d] = style[k];
      g.strokeStyle = c; g.lineWidth = w; g.setLineDash(d); g.beginPath(); for (const s of TOPO.segs) if (s.k === k) strokeLL(g, F, s.ll); g.stroke(); }
    g.setLineDash([]);
    const counts = [0, 0, 0, 0, 0, 0, 0];
    for (let i = 0; i < P.length; i += 5){ const role = P[i + 2]; const p = proj(F, P[i], P[i + 1]); if (p[2] < 0.03) continue; counts[role]++;
      const r = 1.6 + 0.9 * Math.min(4, P[i + 3]); const x = p[0], y = p[1]; g.lineWidth = 1; g.strokeStyle = "rgba(6,9,14,0.85)";
      if (role === 4){ g.strokeStyle = "rgba(220,230,240,0.75)"; g.beginPath(); g.arc(x, y, 2.5, 0, 6.2832); g.stroke(); } // metro: ring
      else if (role === 1){ g.strokeStyle = "#ffd27a"; g.lineWidth = 1.6; g.beginPath(); g.moveTo(x - 3, y); g.lineTo(x + 3, y); g.moveTo(x, y - 3); g.lineTo(x, y + 3); g.stroke(); } // IXP: plus
      else if (role === 6){ g.fillStyle = "#ff9fd0"; g.beginPath(); g.moveTo(x, y - r - 2); g.lineTo(x + r + 2, y); g.lineTo(x, y + r + 2); g.lineTo(x - r - 2, y); g.closePath(); g.fill(); g.stroke(); } // tier-1: diamond
      else if (role === 2){ g.fillStyle = "#c9b8ff"; g.fillRect(x - r * 0.8, y - r * 0.8, r * 1.6, r * 1.6); g.strokeRect(x - r * 0.8, y - r * 0.8, r * 1.6, r * 1.6); } // transit: square
      else if (role === 3){ g.fillStyle = "#7cf0b0"; g.beginPath(); g.moveTo(x, y - r); g.lineTo(x + r, y); g.lineTo(x, y + r); g.lineTo(x - r, y); g.closePath(); g.fill(); } // content / DC: small diamond
      else if (role === 5){ g.fillStyle = "#8ee6ff"; g.beginPath(); g.moveTo(x, y - 2.6); g.lineTo(x + 2.4, y + 1.8); g.lineTo(x - 2.4, y + 1.8); g.closePath(); g.fill(); } // mobile access: triangle
      else { g.fillStyle = "#d8dee6"; g.beginPath(); g.arc(x, y, 1.4, 0, 6.2832); g.fill(); } } // fixed access: dot
    if (TOPO.pathLL){ g.strokeStyle = "rgba(6,9,14,0.9)"; g.lineWidth = 4.5; g.beginPath(); strokeLL(g, F, TOPO.pathLL); g.stroke(); g.strokeStyle = "#fff2c4"; g.lineWidth = 2.2; g.beginPath(); strokeLL(g, F, TOPO.pathLL); g.stroke(); }
    const lk = [0, 0, 0, 0, 0]; for (let i = 2; i < TOPO.links.length; i += 3) lk[TOPO.links[i]]++;
    this.last = { points: P.length / 5, metros: TOPO.metros, links: TOPO.links.length / 3, provider_customer: lk[0], peering: lk[1], ixp_membership: lk[2], terrestrial: lk[3], submarine: lk[4], points_in_view: counts.reduce((a, b) => a + b, 0) };
    if (TOPO.path){ this.last.route_as_hops = TOPO.path[0]; this.last.route_long_haul_km = Math.round(TOPO.path[1]); this.last.route_from = TOPO.pathAB[0]; this.last.route_to = TOPO.pathAB[1]; }
    this.info = { roles: TOPO_ROLES.map((r, i) => i + " " + r).join(", "), kinds: TOPO_KINDS.map((r, i) => i + " " + r).join(", "), all_fields: "MODEL" };
  },
  status(){ const l = this.last; return l.points ? `${fmt(l.points)} points · ${fmt(l.links)} links · route: ${l.route_as_hops !== undefined ? l.route_as_hops + " AS hops, " + fmt(l.route_long_haul_km) + " km long-haul" : "none"} · invented graph, real cities` : "–"; },
  controls: [{ label: "Another route", act(){ TOPO.pick++; this.route(); gfxDirty(600); } }, { label: "Peering and IXP links", act(){ TOPO.peering = !TOPO.peering; gfxDirty(600); } }] });

// ------------------------------------------------------------------ NAT reachability (twin-nat: reach/hooks.rs), drawn in the dock: a generated street, no place
const NAT = { seed: 1, nodes: null, pos: null, pairs: null, joint: null, key: "" };
const NAT_OUT = ["direct IPv6", "direct listener", "hole-punched", "relayed, no attempt", "punch failed, relayed", "same LAN"];
window.CEWP_LAYERS["nat"] = layer({ id: "nat", title: "How each pair of devices connects (a generated street)", cls: "MODEL",
  source: "view_nat(1) → view slots 3 nodes, 4 positions, 5 pairs, 6 joint (cewp_sim::reach::hooks nat_view_*)",
  contract: "twin-nat's view scenario, run once: nodes u32 × 4 (access 0 fixed, 1 phone on home Wi-Fi, 2 phone on a carrier; NAT behaviour 0 public, 1 EIM, 2 address-dependent, 3 symmetric, 4-6 the same behind CGN; IPv6 0-3; household); positions f32 × 2 in metres in a GENERATED street frame (no real place, so it is drawn here, not on the globe); pairs u32 × 3 (a, b, outcome 0 direct IPv6, 1 direct listener, 2 hole-punched, 3 relayed with no attempt, 4 punch failed then relayed, 5 same LAN), one seeded draw per pair of direct_path's probability; joint f32 [3, 7, then 21 shares of devices], access by behaviour over 16 correlated metros. All MODEL: shares calibrated to micro's router marginals, no NAT survey.",
  available: has("view_nat", "view_ptr"),
  compute(){ if (NAT.nodes) return; wasm.view_nat(NAT.seed); NAT.nodes = VW(3); NAT.pos = VW(4); NAT.pairs = VW(5); NAT.joint = VW(6);
    const oc = [0, 0, 0, 0, 0, 0]; for (let i = 2; i < NAT.pairs.length; i += 3) oc[NAT.pairs[i]]++;
    const ac = [0, 0, 0]; for (let i = 0; i < NAT.nodes.length; i += 4) ac[NAT.nodes[i]]++;
    const J = NAT.joint, cgn = J.length >= 23 ? [0, 1, 2].reduce((s, r) => s + J[2 + r * 7 + 4] + J[2 + r * 7 + 5] + J[2 + r * 7 + 6], 0) : NaN;
    this.last = { devices: NAT.nodes.length / 4, pairs: NAT.pairs.length / 3, fixed: ac[0], phone_wifi: ac[1], phone_carrier: ac[2], direct_ipv6: oc[0], direct_listener: oc[1], hole_punched: oc[2], relayed_no_attempt: oc[3], punch_failed_relayed: oc[4], same_lan: oc[5], joint_share_behind_cgn: +cgn.toFixed(4) };
    this.info = { outcomes: NAT_OUT.map((s, i) => i + " " + s).join(", ") }; },
  onShow(){ this.compute(); },
  panel(c){
    this.compute(); const N = NAT.nodes, X = NAT.pos, Pp = NAT.pairs; if (!N) return;
    const dpr = window.devicePixelRatio || 1, W2 = c.clientWidth || 260, H2 = 200; if (c.width !== Math.round(W2 * dpr)){ c.width = Math.round(W2 * dpr); c.height = H2 * dpr; NAT.key = ""; }
    if (NAT.key === "drawn") return; NAT.key = "drawn";
    const g = c.getContext("2d"); g.setTransform(dpr, 0, 0, dpr, 0, 0); g.clearRect(0, 0, W2, H2);
    let x0 = 1e9, x1 = -1e9, y0 = 1e9, y1 = -1e9; for (let i = 0; i < X.length; i += 2){ x0 = Math.min(x0, X[i]); x1 = Math.max(x1, X[i]); y0 = Math.min(y0, X[i + 1]); y1 = Math.max(y1, X[i + 1]); }
    const pad = 10, H3 = H2 - 44, s = Math.min((W2 - 2 * pad) / Math.max(1, x1 - x0), (H3 - 2 * pad) / Math.max(1, y1 - y0));
    const at = (i) => [pad + (X[i * 2] - x0) * s, pad + (X[i * 2 + 1] - y0) * s];
    const ps = [["#8ee6ff", []], ["#8ee6ff", []], ["#c9b8ff", [1, 2]], ["rgba(170,180,190,0.5)", [4, 3]], ["#ff9d7a", [1, 2, 5, 2]], ["rgba(255,255,255,0.25)", []]];
    for (let o = 0; o < 6; o++){ const [col, dash] = ps[o]; g.strokeStyle = col; g.setLineDash(dash); g.lineWidth = o === 4 ? 1.1 : 0.7; g.globalAlpha = o === 3 ? 0.5 : 0.8; g.beginPath();
      for (let i = 0; i < Pp.length; i += 3){ if (Pp[i + 2] !== o) continue; const a = at(Pp[i]), b = at(Pp[i + 1]); g.moveTo(a[0], a[1]); g.lineTo(b[0], b[1]); } g.stroke(); }
    g.setLineDash([]); g.globalAlpha = 1;
    for (let i = 0; i < N.length / 4; i++){ const [x, y] = at(i), ac = N[i * 4], bh = N[i * 4 + 1]; g.fillStyle = bh === 0 ? "#8ee6ff" : bh >= 4 ? "#ff9d7a" : "#c9b8ff"; g.strokeStyle = "rgba(6,9,14,0.9)"; g.lineWidth = 1;
      if (ac === 0){ g.fillRect(x - 2.5, y - 2.5, 5, 5); g.strokeRect(x - 2.5, y - 2.5, 5, 5); } else if (ac === 1){ g.beginPath(); g.arc(x, y, 2.4, 0, 6.2832); g.fill(); g.stroke(); } else { g.beginPath(); g.moveTo(x, y - 3); g.lineTo(x + 3, y + 2.2); g.lineTo(x - 3, y + 2.2); g.closePath(); g.fill(); g.stroke(); }
      if (bh === 0){ g.fillStyle = "#fff"; g.fillRect(x - 0.8, y - 0.8, 1.6, 1.6); } else if (bh >= 4){ g.strokeStyle = "#ff9d7a"; g.beginPath(); g.arc(x, y, 4.6, 0, 6.2832); g.stroke(); } } // behaviour by mark, not colour alone: white centre public, outer ring behind CGN
    g.font = "10px " + MFONT; g.fillStyle = "#aab4c2";
    g.fillText("■ fixed  ● Wi-Fi phone  ▲ carrier phone", 4, H2 - 30);
    g.fillText("white centre: public · outer ring: behind CGN", 4, H2 - 18);
    g.fillText("— direct  ┄ punched  - - relayed  -·- failed punch", 4, H2 - 6);
  },
  status(){ const l = this.last; return l.pairs ? `${l.devices} devices, ${fmt(l.pairs)} pairs: ${l.direct_ipv6 + l.direct_listener} direct, ${l.hole_punched} punched, ${l.relayed_no_attempt + l.punch_failed_relayed} relayed (${l.punch_failed_relayed} after a failed punch), ${l.same_lan} same LAN · generated street, no real place` : "–"; } });

// ------------------------------------------------------------------ people (twin-people: people_hooks.rs): the week's rhythm, ties, synchronized events
const PPL = { seed: 1, loc: null, week: null, ties: null, flash: null, tiesLL: [] };
const PPL_TIES = 40;
const DOY_WEEKDAY_OFFSET_H = 24; // sim day 314 of 2026 is a Tuesday; the hook's hour 0 is Monday 00:00 UTC
window.CEWP_LAYERS["people"] = layer({ id: "people", title: "The week's rhythm, ties and a flash crowd", cls: "MODEL", inputs: "FRONTIER (event inputs)",
  source: "view_people(population grid, 1) → view slots 7 localities, 8 week, 9 ties, 10 events (cewp_sim::people_hooks people_view_*)",
  contract: "twin-people's one-week bursty run of 6,000 generated persons placed on the page's population grid (the grid's bytes are handed to the kernel). Localities f32 × 3 (lat, lon, population weight; DERIVED from the grid): the ring is the weight, the inner disc the week-intensity multiplier at the sim's hour of week (week f32 [k × 168 + h], 1.0 = an average hour; MODEL). Ties u32 × 3 (a, b, count of person-to-person ties; MODEL): the strongest 40 drawn. Events f32 × 6 (lat, lon or NaN for world-wide, start, peak, end hour of week, peak multiplier; DERIVED from FRONTIER event inputs): the staged Wednesday 20:30 UTC flash crowd first, then region and world events; one shows as a ring while the sim's hour of week is inside it. Hour of week = sim hours since 00:00 UTC of day 314 + 24 (day 314 of 2026 is a Tuesday). \"Jump to the flash crowd\" moves the sim clock to just after its onset. Small localities are noisy.",
  available: has("view_people", "view_ptr", "alloc"),
  compute(){
    if (PPL.loc) return; const bin = atob(POP_GRID_B64); const n = bin.length; const p = wasm.alloc(n) >>> 0; const u8 = new Uint8Array(mem.buffer, p, n); for (let i = 0; i < n; i++) u8[i] = bin.charCodeAt(i);
    const k = wasm.view_people(p, n, PPL.seed); if (!k){ PPL.loc = new Float32Array(0); return; }
    PPL.loc = VW(7); PPL.week = VW(8); PPL.ties = VW(9); PPL.flash = VW(10);
    const L = PPL.loc; PPL.tiesLL = []; for (let i = 0; i < Math.min(PPL_TIES, PPL.ties.length / 3); i++){ const a = PPL.ties[i * 3], b = PPL.ties[i * 3 + 1]; PPL.tiesLL.push({ n: PPL.ties[i * 3 + 2], ll: gcLL(L[a * 3], L[a * 3 + 1], L[b * 3], L[b * 3 + 1]) }); }
  },
  onShow(){ later(this); },
  how(){ const t = (typeof SIM !== "undefined") ? SIM.t / 3600 : (GFX.simH || 0); return (((t + DOY_WEEKDAY_OFFSET_H) % 168) + 168) % 168; }, // SIM.t keeps the day; the sun's clock is mod 24
  draw2D(g, F){
    const L = PPL.loc; if (!L || !L.length) return; const K = L.length / 3, h = Math.floor(this.how()), Wk = PPL.week;
    const mx = PPL.tiesLL.length ? PPL.tiesLL[0].n : 1;
    g.setLineDash([2, 3]); for (const t of PPL.tiesLL){ g.strokeStyle = "rgba(255,200,230,0.55)"; g.lineWidth = 0.6 + 2 * t.n / mx; g.beginPath(); strokeLL(g, F, t.ll); g.stroke(); } g.setLineDash([]);
    let hi = 0, hiK = -1, msum = 0, wsum = 0;
    for (let k = 0; k < K; k++){ const p = proj(F, L[k * 3], L[k * 3 + 1]); const w = L[k * 3 + 2], m = Wk[k * 168 + h]; msum += m * w; wsum += w; if (m > hi && w > 0.002){ hi = m; hiK = k; } if (p[2] < 0.03) continue;
      const R = 2 + 40 * Math.sqrt(w); g.lineWidth = 1; g.strokeStyle = "rgba(255,220,180,0.55)"; g.beginPath(); g.arc(p[0], p[1], R, 0, 6.2832); g.stroke();
      g.fillStyle = "rgba(255,190,120,0.55)"; g.beginPath(); g.arc(p[0], p[1], Math.max(0.8, R * Math.min(1.6, Math.sqrt(Math.max(0, m)) / 1.6)), 0, 6.2832); g.fill(); }
    const Fl = PPL.flash, how = this.how(); const live = [];
    for (let i = 0; i < Fl.length; i += 6){ if (how < Fl[i + 2] || how > Fl[i + 4]) continue; live.push(i / 6); if (!isFinite(Fl[i])) continue; const p = proj(F, Fl[i], Fl[i + 1]); if (p[2] < 0.03) continue;
      const age = (how - Fl[i + 2]) / Math.max(1e-3, Fl[i + 4] - Fl[i + 2]); const R = 10 + 36 * age; g.strokeStyle = "#ffffff"; g.globalAlpha = 1 - 0.8 * age; g.lineWidth = 2; g.beginPath(); g.arc(p[0], p[1], R, 0, 6.2832); g.stroke(); g.globalAlpha = 1;
      g.font = "600 11px " + MFONT; g.fillStyle = "#fff6d8"; g.fillText("×" + Fl[i + 5].toFixed(1) + " at onset", p[0] + R + 4, p[1]); }
    this.last = { localities: K, persons: 6000, hour_of_week: +how.toFixed(2), weighted_mean_multiplier: +(msum / Math.max(1e-9, wsum)).toFixed(3), busiest_locality: hiK, busiest_multiplier: +hi.toFixed(3), ties_pairs: PPL.ties.length / 3, ties_drawn: PPL.tiesLL.length, strongest_tie: mx, events: Fl.length / 6, events_live: live.length, flash_peak_multiplier: Fl.length ? +Fl[5].toFixed(2) : -1, flash_start_hour: Fl.length ? +Fl[2].toFixed(2) : -1 };
    this.info = { live_events: live.slice(0, 5) };
  },
  status(){ const l = this.last; if (this.busy) return "running the generated week once…"; if (!l.localities) return "–"; const d = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"][Math.floor(l.hour_of_week / 24) % 7], hh = Math.floor(l.hour_of_week % 24);
    return `${d} ${String(hh).padStart(2, "0")}:00 UTC · activity ×${l.weighted_mean_multiplier} of an average hour · ${l.events_live ? l.events_live + " event(s) under way" : "no event under way"} · flash crowd Wed 20:30 UTC peaks ×${l.flash_peak_multiplier}`; },
  controls: [{ label: "Jump to the flash crowd", act(){ if (typeof SIM !== "undefined" && PPL.flash && PPL.flash.length){ SIM.t = (PPL.flash[2] - DOY_WEEKDAY_OFFSET_H + 0.05) * 3600; SIM.tTick = SIM.t; } gfxDirty(800); } }] });

// ------------------------------------------------------------------ holon certificates (twin-holon: holon_view.rs)
const HOL = { seed: 43, cells: null, names: null, cu: null, cf: null };
const HOL_STATE = ["certified", "refused", "not evaluated"];
window.CEWP_LAYERS["holon"] = layer({ id: "holon", title: "Where a coarse tier is trusted (certificates)", cls: "DERIVED", inputs: "MODEL",
  source: "view_holon(43) → view slots 11 cells, 12 bound index, 16 bound names, 13/14 certificate table (cewp_sim::holon_view cells_of, certificates_of over holon_demo::demonstration_seeded)",
  contract: "twin-holon's R5 demonstration, run once per page (two 2-day runs, normal and outage): cells f32 × 8 (lat, lon, half-size°, level 0 the tier / 1 a cell, state 0 certified 1 refused 2 not evaluated, bound index, signed margin in online share, worst normalised defect on record 0). The cells are cells of a tier (host kind × access network), laid out as a patch centred 47.3769, 8.5417; the position is layout only, not a place. Glyph by state: certified solid square, refused crossed square, not evaluated dashed outline. Bound names come from the kernel (BOUND_NAMES). Certificate table: u32 × 9 and f32 × 20 per (tier, observable). DERIVED; the micro world under it is MODEL. A refusal is shown as a refusal: that cell goes back to the finer tier.",
  available: has("view_holon", "view_ptr"),
  compute(){
    if (HOL.cells) return; wasm.view_holon(HOL.seed); HOL.cells = VW(11); const B = VW(12), bytes = VW(16); const s = String.fromCharCode(...bytes);
    HOL.names = []; for (let i = 0; i < B.length; i += 3) HOL.names[B[i]] = s.substr(B[i + 1], B[i + 2]); HOL.cu = VW(13); HOL.cf = VW(14);
    const C = HOL.cells, st = [0, 0, 0], binds = {}; for (let i = 8; i < C.length; i += 8){ st[C[i + 4]]++; const b = C[i + 5]; if (b >= 0){ const n = HOL.names[b] || String(b); binds[n] = (binds[n] || 0) + 1; } }
    let dist = 0; for (let r = 0; r < HOL.cu.length / 9; r++) dist += HOL.cu[r * 9 + 2];
    this.last = { cells: C.length / 8 - 1, certified: st[0], refused: st[1], not_evaluated: st[2], tier_state: C[4], tier_worst_defect: +C[7].toFixed(3), certificate_rows: HOL.cu.length / 9, distributional_certified_rows: dist };
    if (isFinite(C[6])) this.last.tier_min_margin = +C[6].toFixed(4); // NaN: a whole-tier bound has no distance
    this.info = { tier_state: HOL_STATE[C[4]], tier_bound: C[5] >= 0 ? HOL.names[C[5]] : "none", binding_bounds: Object.entries(binds).map(([k, v]) => k + " " + v).join(", "), states: "0 certified, 1 refused, 2 not evaluated", label: "cells of a tier, not places" };
  },
  onShow(){ later(this); },
  draw2D(g, F){
    const C = HOL.cells; if (!C) return;
    const c0 = proj(F, C[0], C[1]), c1 = proj(F, C[0] + C[2], C[1]); if (c0[2] < 0.03) return;
    if (Math.hypot(c1[0] - c0[0], c1[1] - c0[1]) < 5){ // the patch is a street block: mark it until it is big enough to draw
      const l = this.last; g.lineWidth = 1.5; g.strokeStyle = "#fff"; g.setLineDash([3, 2]); g.strokeRect(c0[0] - 6, c0[1] - 6, 12, 12); g.setLineDash([]);
      g.font = "600 11px " + MFONT; g.lineWidth = 3; g.strokeStyle = "rgba(6,9,14,0.9)"; const t = `tier certificate: ${l.certified} cells certified, ${l.refused} refused (zoom in)`;
      g.strokeText(t, c0[0] + 10, c0[1] - 10); g.fillStyle = "#e8eef6"; g.fillText(t, c0[0] + 10, c0[1] - 10); return; }
    for (let i = 0; i < C.length; i += 8){ const la = C[i], lo = C[i + 1], h = C[i + 2], lv = C[i + 3], s = C[i + 4]; const hl = h / Math.max(0.2, Math.cos(la * DEG));
      const q = [[la - h, lo - hl], [la - h, lo + hl], [la + h, lo + hl], [la + h, lo - hl]].map(([a, b]) => proj(F, a, b)); if (q.some((p) => p[2] < 0.03)) continue;
      g.beginPath(); g.moveTo(q[0][0], q[0][1]); for (let k = 1; k < 4; k++) g.lineTo(q[k][0], q[k][1]); g.closePath();
      if (lv === 0){ g.setLineDash([6, 4]); g.strokeStyle = "rgba(255,255,255,0.7)"; g.lineWidth = 1.5; g.stroke(); g.setLineDash([]); continue; }
      if (s === 0){ g.fillStyle = "rgba(124,240,176,0.30)"; g.fill(); g.strokeStyle = "#7cf0b0"; g.lineWidth = 1.4; g.stroke(); }
      else if (s === 1){ g.fillStyle = "rgba(255,138,122,0.25)"; g.fill(); g.strokeStyle = "#ff8a7a"; g.lineWidth = 1.6; g.stroke(); g.beginPath(); g.moveTo(q[0][0], q[0][1]); g.lineTo(q[2][0], q[2][1]); g.moveTo(q[1][0], q[1][1]); g.lineTo(q[3][0], q[3][1]); g.stroke(); }
      else { g.setLineDash([2, 3]); g.strokeStyle = "rgba(200,210,225,0.8)"; g.lineWidth = 1.2; g.stroke(); g.setLineDash([]); } }
    g.font = "600 11px " + MFONT; g.lineWidth = 3; g.strokeStyle = "rgba(6,9,14,0.9)"; g.strokeText("cells of a tier, not places", c1[0] - 80, c1[1] - 8); g.fillStyle = "#e8eef6"; g.fillText("cells of a tier, not places", c1[0] - 80, c1[1] - 8);
  },
  status(){ const l = this.last; if (this.busy) return "running the tier demonstration once (two 2-day runs)…"; return l.cells !== undefined ? `■ ${l.certified} certified · ⊠ ${l.refused} refused · ⬚ ${l.not_evaluated} not evaluated of ${l.cells} cells · tier ${this.info.tier_state}${this.info.tier_bound !== "none" ? " (" + this.info.tier_bound + ")" : ""} · cells of a tier, not places` : "–"; } });

// ------------------------------------------------------------------ Phase 2 contracts (not shown until the export exists)
const CONTRACT = (id, title, cls, exp, contract) => layer({ id, title, cls, source: exp.join(", "), contract, available: has(...exp), pending: true });
CONTRACT("failures", "Failure domains going dark, the mesh re-forming", "MODEL", ["failures_view_nodes", "failures_view_domains", "failures_view_step", "failures_view_steps"],
  "nodes f32 × 3 (lat, lon, kind); domains f32 × 6 (lat, lon, radius km, kind, step down, step up | -1); step(s) u32 per node: component id, u32::MAX = down; summary f32 × 4 per step.");
CONTRACT("media", "A room call: one tree per publisher, a parent leaves, the tree repairs", "DERIVED", ["media_view_members", "media_view_tree", "media_view_edges", "media_view_rx", "media_view_steps", "media_view_events"],
  "Built once per page (the scenario runs once; slice it, never per frame). members f32 × 4 (lat, lon, access 0 fibre home 1 cable laptop Wi-Fi 2 DSL home 3 phone home Wi-Fi 4 phone LTE, nominal uplink B/s), MODEL; tree u32 × 3 (stream, parent, child; every edge at any step; stream == parent is a direct leg); edges(step) f32 × 4 aligned to tree (rate B/s, loss ‰, queue ms, layer 0 detached 1 audio+base 2 full), DERIVED; rx(step) f32 × 3 at stream·n + member (layer -1 none 0 audio 1 base 2 both, stall 0/1, glass-to-glass ms), DERIVED; steps = 60 (1 s); events f32 × 5 (t ms, kind 0 parent left 1 audio resumed 2 video resumed 3 not resumed, stream, child, parent). The tree is edge v45's closed-loop controller, not alm.rs.");
CONTRACT("calibration", "Seat knee with its interval", "DERIVED", ["calibration_view_curve", "calibration_view_knee", "calibration_view_residuals"],
  "curve(scenario) f64 × 6, 24 rows (devices, ρ central, lower, upper, class, binding code 0-5); knee(scenario) 7 fields (lower, central, upper, interval kind, binds lower, binds upper, class), -1 = none, crossing ρ 0.70, drawn only as its interval, never the central value alone, never labelled with a confidence level; residuals f64 × 4 (counter id, observed, predicted, out-of-sample). Not shown until twin-calibration says the knee is safe.");

// ------------------------------------------------------------------ the dock
const LAYERS = window.LAYERS = {
  list: REG, def: (id) => REG.find((d) => d.id === id),
  on(id){ const d = this.def(id); return !!d && KEEP.on.has(id) && d.available(); },
  set(id, v){ const d = this.def(id); if (!d) return; if (v) KEEP.on.add(id); else KEEP.on.delete(id); save(); if (v && d.onShow) try { d.onShow(); } catch (e) { console.warn("layer " + id, e); } render(); gfxDirty(800); },
  animating(){ return !GFX.reduced && playing && REG.some((d) => d.arcs && this.on(d.id) && d.arcs().length); },
  onTick(){ for (const d of REG) if (d.onTick && d.available()) try { d.onTick(); } catch (e) { console.warn("layer " + d.id, e); } },
  arcSets(sets){ for (const d of REG) if (d.arcs && this.on(d.id)) sets.push(d.arcs()); },
  drawGL(){},
  draw2D(g, F){ const dpr = mcv.width / Math.max(1, F.w); g.save(); g.setTransform(dpr, 0, 0, dpr, 0, 0);
    for (const d of REG) if (d.draw2D && this.on(d.id)) try { d.draw2D(g, F); } catch (e) { console.warn("layer " + d.id, e); }
    g.restore(); if (frame % 10 === 0) refresh(); },
  // for the console check: what an export returned and what the layer drew
  check(){ return Object.fromEntries(REG.filter((d) => this.on(d.id)).map((d) => [d.id, Object.assign({}, d.last, d.info)])); },
};
const CODE = { MEASURED: "ME", DERIVED: "DE", MODEL: "MO", PROJECTED: "PR", FRONTIER: "FR" };
let dock = null, open = false;
function render(){
  if (!dock){ dock = document.getElementById("layer-dock") || document.body.appendChild(Object.assign(document.createElement("div"), { id: "layer-dock" })); dock.setAttribute("role", "group"); dock.setAttribute("aria-label", "Globe layers");
    dock.addEventListener("click", (e) => { const b = e.target.closest("button"); if (!b || b.dataset.layerClaim) return; e.stopPropagation();
      if (b.dataset.toggle){ open = !open; render(); return; }
      if (b.dataset.layer){ LAYERS.set(b.dataset.layer, !KEEP.on.has(b.dataset.layer)); return; }
      if (b.dataset.ctl){ const [id, k] = b.dataset.ctl.split(":"); const d = LAYERS.def(id); d.controls[+k].act.call(d); render(); } });
    dock.addEventListener("input", (e) => { const r = e.target; if (!r.dataset.ctl) return; const [id, k] = r.dataset.ctl.split(":"); const d = LAYERS.def(id); d.controls[+k].set.call(d, +r.value); const o = r.parentElement.querySelector("output"); if (o) o.textContent = d.controls[+k].show(+r.value); refresh(); });
    for (const ev of ["pointerdown", "wheel", "touchstart", "keydown"]) dock.addEventListener(ev, (e) => e.stopPropagation(), { passive: ev !== "keydown" });
    window.addEventListener("keydown", (e) => { if (e.key === "l" && !/INPUT|TEXTAREA|SELECT/.test((e.target || {}).tagName || "") && !e.metaKey && !e.ctrlKey){ open = !open; render(); } }); }
  const shown = REG.filter((d) => d.available() && !d.pending);
  const head = `<button class="ly-toggle" data-toggle="1" aria-expanded="${open}" aria-controls="ly-body">Layers <span class="ly-n">${shown.filter((d) => KEEP.on.has(d.id)).length}/${shown.length}</span></button>`;
  if (!open){ dock.innerHTML = head; dock.classList.remove("open"); return; }
  dock.classList.add("open");
  dock.innerHTML = head + `<div id="ly-body" class="ly-body">` + shown.map((d) => {
    const on = KEEP.on.has(d.id);
    const ctl = on && d.controls ? d.controls.map((c, k) => c.range ? `<label class="ly-ctl">${c.label} <input type="range" min="${c.range[0]}" max="${c.range[1]}" step="${c.range[2]}" value="${c.value()}" data-ctl="${d.id}:${k}"><output>${c.show(c.value())}</output></label>` : `<button class="ly-ctl-b" data-ctl="${d.id}:${k}">${c.label}</button>`).join("") : "";
    return `<div class="ly-row${on ? " on" : ""}"><button class="ly-sw" data-layer="${d.id}" aria-pressed="${on}"><span class="ly-box" aria-hidden="true">${on ? "■" : "□"}</span><span class="ly-t">${d.title}</span></button>
      <button type="button" class="ly-cls" data-layer-claim="${d.id}" title="${d.cls}: class of what this layer draws (FSD §8.2); open its evidence card" aria-label="${d.cls}, open the evidence card for ${d.title}">${CODE[d.cls]}</button>${d.inputs ? `<span class="ly-in" title="inputs: ${d.inputs}">inputs ${d.inputs.split(" ")[0]}</span>` : ""}
      ${on ? `<div class="ly-st" id="ly-st-${d.id}"></div>${d.panel ? `<canvas class="ly-chart" id="ly-ch-${d.id}" aria-label="${d.title} chart"></canvas>` : ""}${ctl}<details class="ly-src"><summary>Where this comes from</summary><p><code>${d.source}</code></p><p>${d.contract}</p></details>` : ""}</div>`; }).join("") +
    `</div>`;
  refresh();
}
function refresh(){ if (!dock || !open) return; for (const d of REG){ if (!LAYERS.on(d.id)) continue; const el = $id("ly-st-" + d.id); if (el && d.status) el.textContent = d.status(); const c = $id("ly-ch-" + d.id); if (c && d.panel) d.panel(c); } }
// once the kernel is in, show the dock and fill the layers that are on
(function boot(){ if (typeof wasm === "undefined" || !wasm || !W || !bufs){ setTimeout(boot, 300); return; } for (const id of KEEP.on){ const d = LAYERS.def(id); if (d && d.onShow && d.available()) try { d.onShow(); } catch (e) { console.warn(e); } } render(); })();
})();
