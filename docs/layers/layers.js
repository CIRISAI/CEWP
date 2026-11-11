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
const KEEP = { on: new Set(["glow", "arcs"]) };
try { const s = localStorage.getItem("cewp.layers"); if (s) KEEP.on = new Set(JSON.parse(s)); } catch (e) {}
const save = () => { try { localStorage.setItem("cewp.layers", JSON.stringify([...KEEP.on])); } catch (e) {} };

const REG = [];
function layer(def){ def.last = {}; REG.push(def); return def; }

// ------------------------------------------------------------------ illustration (not data)
layer({ id: "glow", title: "Night side lit by where people are", cls: "MODEL", decorative: false,
  source: "population grid (globe.js POP_GRID) · clock_sun",
  contract: "The population grid the sim places people on (MODEL input, 1.5° cells, scattered ±cellRad), blurred 3 px into a 1024×512 image; lit where the sun from clock_sun(t) is below the horizon. Not satellite night lights. Read once at load; the sun every frame.",
  available: () => true, status(){ return "sun over " + GFX.sunLL.map((v) => v.toFixed(1) + "°").join(", ") + " · " + String(Math.floor(GFX.utc)).padStart(2, "0") + ":" + String(Math.floor((GFX.utc % 1) * 60)).padStart(2, "0") + " UTC"; } });

// ------------------------------------------------------------------ the tick's sample arcs
layer({ id: "arcs", title: "Deliveries crossing localities (a sample)", cls: "DERIVED",
  source: "tick → arcs buffer (u32 × 2: signer, target | tier << 28)",
  contract: "tick() writes up to 420 (signer, target, tier) pairs, itself a 1-in-3 to 1-in-48 sample of the deliveries it ran; the page draws at most 10 a tick. Tier: 0 shaped hop, 1 seat replication, 2 relay tree / unshaped, 3 onion. Read every tick (120 ms). The exact counts are on the flows layer.",
  available: () => true, status(){ return (GFX.arcsDrawn || 0) + " arcs on screen"; } });

// ------------------------------------------------------------------ flows between localities (exact counts)
const FLOW = { cent: null, W: null, win: [], sum: null, arcs: [], top: [], checked: "", N: 0 };
const FLOW_WINDOW = 25; // ticks summed (3 s of sim at 120 ms a tick)
const FLOW_TOP = 60;
layer({ id: "flows", title: "Flows between localities (exact counts)", cls: "DERIVED",
  source: "tick → globe buffer 10 (flows) · locality_centroids → buffer 2",
  contract: "During tick() the kernel counts every delivery that leaves its signer's locality, the same events as out[6] + out[7], into a 181 × 181 u32 matrix (row = from, column = to), buffer 10. Positions: locality_centroids(lat, lon, metro, n), buffer 2, f64 × 3 (lat°, lon°, nodes). The page sums the last 25 ticks and draws the 60 largest pairs, ribbon width ∝ log(count). Checked every tick: Σ matrix = out[6] + out[7].",
  available: has("globe_buf_ptr", "locality_centroids"),
  onTick(){
    if (FLOW.W !== W){ wasm.locality_centroids(bufs.lat, bufs.lon, bufs.metro, W.n); FLOW.cent = F64(2).slice(); FLOW.W = W; FLOW.win = []; FLOW.sum = null; }
    const m = U32(10).slice(); FLOW.N = Math.round(Math.sqrt(m.length));
    const out = new Float64Array(mem.buffer, bufs.out, OUT); let tot = 0; for (let i = 0; i < m.length; i++) tot += m[i];
    const want = out[6] + out[7];
    this.last.tick = { matrix_sum: tot, out6_plus_out7: want, ok: tot === want };
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
    this.last.window = { ticks: FLOW.win.length, crossings: total, pairs: pairs.length, drawn: FLOW.arcs.length, top: FLOW.top.slice(0, 5) };
  },
  arcs(){ return FLOW.arcs; },
  draw2D(g, F){ // count labels on the five largest, at the arc's midpoint
    if (!FLOW.cent) return; g.font = "600 11px " + MFONT; g.textAlign = "center"; g.textBaseline = "middle";
    for (const r of FLOW.arcs.slice(0, 5)){ const m = [(r.a[0] + r.b[0]), (r.a[1] + r.b[1]), (r.a[2] + r.b[2])]; const L = Math.hypot(...m) || 1;
      const la = Math.asin(m[1] / L) * 180 / Math.PI, lo = Math.atan2(m[0], m[2]) * 180 / Math.PI; const p = proj(F, la, lo); if (p[2] < 0.05) continue;
      const t = fmt(r.f.n); g.lineWidth = 3; g.strokeStyle = "rgba(6,9,14,0.9)"; g.strokeText(t, p[0], p[1]); g.fillStyle = "#f4e7c8"; g.fillText(t, p[0], p[1]); }
    g.textAlign = "left";
  },
  status(){ const w = this.last.window; return w ? fmt(w.crossings) + " crossings in " + w.ticks + " ticks · " + FLOW.checked : "waiting for a tick"; } });

// ------------------------------------------------------------------ one room's member relay tree (alm.rs)
const ALM = { room: -1, members: null, edges: null, unrooted: null, mf: null, arcs: [], pick: 0 };
layer({ id: "alm", title: "A room's relay tree (CC 6.1.6)", cls: "DERIVED", inputs: "MODEL",
  source: "alm_room → globe buffers 3 members, 4 edges, 5 unrooted, 6 member facts",
  contract: "alm_room(lat, lon, room_off, room_mem, room, publisher, max_members = 64, seed) runs cewp_sim::alm::compute_alm_topology_verified_with_publisher on the first 64 members of the region's public room. Inputs are MODEL: uplink by device class (phone cellular 2, phone Wi-Fi 8, laptop 12, home node 20 Mbit/s), RTT = 10 ms + fibre at 200 km/ms over 1.6× the great circle, every member trusting every other at depth 1. Buffer 4: u32 × 3 (parent ix, child ix, child depth). Buffer 6: f64 × 4 (class, uplink, depth or -1, children). Recomputed when shown or when the room changes.",
  available: has("alm_room"),
  compute(){
    if (!W || !bufs) return;
    const la = pitch * 180 / Math.PI, lo = ((yaw * 180 / Math.PI + 540) % 360) - 180;
    // the region of the locality nearest the screen centre (centroids from the kernel)
    if (FLOW.W !== W){ wasm.locality_centroids(bufs.lat, bufs.lon, bufs.metro, W.n); FLOW.cent = F64(2).slice(); FLOW.W = W; }
    let best = 0, bd = 1e9; const C = FLOW.cent;
    for (let k = 0; k < C.length / 3; k++){ if (!C[k * 3 + 2]) continue; const d = kmf(la, lo, C[k * 3], C[k * 3 + 1]); if (d < bd){ bd = d; best = W.regionOfMetro[k]; } }
    const room = (ALM.pick % 2 === 0) ? W.regionRoom[best] : W.globalRoom; ALM.room = room;
    const pub = W.roomMem[W.roomOff[room]];
    const ne = wasm.alm_room(bufs.lat, bufs.lon, bufs.roomOff, bufs.roomMem, room, pub, 64, 0x2026 + ALM.pick);
    ALM.members = U32(3).slice(); ALM.edges = U32(4).slice(); ALM.unrooted = U32(5).slice(); ALM.mf = F64(6).slice();
    let depth = 0, maxKids = 0; for (let i = 0; i < ALM.members.length; i++){ depth = Math.max(depth, ALM.mf[i * 4 + 2]); maxKids = Math.max(maxKids, ALM.mf[i * 4 + 3]); }
    ALM.arcs = []; for (let e = 0; e < ne; e++){ const p = ALM.members[ALM.edges[e * 3]], c = ALM.members[ALM.edges[e * 3 + 1]], d = ALM.edges[e * 3 + 2];
      const a = xyz(W.lat[p], W.lon[p]), b = xyz(W.lat[c], W.lon[c]); const th = Math.acos(Math.max(-1, Math.min(1, a[0] * b[0] + a[1] * b[1] + a[2] * b[2])));
      ALM.arcs.push({ a, b, birth: 0, life: 0, tier: 2, width: Math.max(1.2, 4.2 - d * 0.8), kind: 1, lift: 0.006 + 0.07 * th / Math.PI }); }
    this.last = { room, region: REG_NAMES[best] || best, members: ALM.members.length, edges: ne, depth, max_children: maxKids, fanout_bound: wasm.alm_bounds(0), rtt_ceiling_ms: wasm.alm_bounds(1), unrooted: ALM.unrooted.length };
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
const CUST = { reach: 1000, holders: null, c: null };
layer({ id: "custody", title: "Who holds a room's chunk", cls: "DERIVED", inputs: "MODEL (reachability draw)",
  source: "custody_room → globe buffers 7 (summary), 8 (holders)",
  contract: "custody_room(lat, lon, metro, room_off, room_mem, room, present_pm, seed): the room's members are the sealed chunk's holders; present_pm (the slider) is the share reachable, drawn per holder (a MODEL stand-in until the failures package supplies who is down). The kernel runs chunk_custody::classify(present, 20, 6) with the tick's own thresholds and Kish n_eff (effective_source_count) over the holders' locality masses. Buffer 7 f64: members, present, target, min viable, state (0 Full, 1 Partial, 2 EnvelopeOnly), n_eff, localities, mean km to centroid, max km. Buffer 8 u32 × 2: holder node, reachable. Same room as the relay tree.",
  available: has("custody_room"),
  compute(){
    if (!W || !bufs) return; const room = ALM.room >= 0 ? ALM.room : W.regionRoom[2];
    wasm.custody_room(bufs.lat, bufs.lon, bufs.metro, bufs.roomOff, bufs.roomMem, room, CUST.reach, 7);
    CUST.c = F64(7).slice(); CUST.holders = U32(8).slice(); const c = CUST.c;
    this.last = { room, members: c[0], present: c[1], target: c[2], min_viable: c[3], state: ["Full", "Partial", "EnvelopeOnly"][c[4]], n_eff: c[5], localities: c[6], mean_km: Math.round(c[7]), max_km: Math.round(c[8]), reachable_permille: CUST.reach };
  },
  onShow(){ if (ALM.room < 0 && LAYERS.def("alm").available()) LAYERS.def("alm").compute(); this.compute(); },
  draw2D(g, F){
    const H = CUST.holders; if (!H) return;
    for (let i = 0; i < H.length; i += 2){ const m = H[i], up = H[i + 1], p = proj(F, W.lat[m], W.lon[m]); if (p[2] < 0.02) continue;
      g.lineWidth = 1.4; if (up){ g.fillStyle = "#ffd27a"; g.strokeStyle = "rgba(6,9,14,0.9)"; g.fillRect(p[0] - 3, p[1] - 3, 6, 6); g.strokeRect(p[0] - 3, p[1] - 3, 6, 6); }
      else { g.strokeStyle = "#9aa3ad"; g.strokeRect(p[0] - 3, p[1] - 3, 6, 6); g.beginPath(); g.moveTo(p[0] - 3, p[1] + 3); g.lineTo(p[0] + 3, p[1] - 3); g.stroke(); } } // filled = reachable holder; struck box = unreachable
  },
  status(){ const l = this.last; return l.members ? `${l.state}: ${l.present} of ${l.members} reachable (Full at ${l.target}, floor ${l.min_viable}) · n_eff ${l.n_eff} over ${l.localities} localities · ${fmt(l.mean_km)} km mean spread` : "–"; },
  controls: [{ range: [0, 1000, 50], label: "Holders reachable", value: () => CUST.reach, set(v){ CUST.reach = v; this.compute(); gfxDirty(600); }, show: (v) => (v / 10).toFixed(0) + " %" }] });

// ------------------------------------------------------------------ reachability classes in the street world (micro.rs)
layer({ id: "reach", title: "How each home is reachable (street scale)", cls: "MODEL",
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
layer({ id: "seat", title: "Serve-seat load across fleet sizes", cls: "MODEL", inputs: "calibrated to seat-1 (MEASURED/DERIVED/MODEL per capacity.rs COST_SPECS)",
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
    const L = 34, B = H2 - 18, T = 8, Rr = W2 - 8, lx = (d) => L + (Math.log10(d) - 1) / 4 * (Rr - L), ly = (r) => B - Math.min(1.5, r) / 1.5 * (B - T);
    g.strokeStyle = "rgba(200,210,225,0.25)"; g.lineWidth = 1; g.font = "10px " + MFONT; g.fillStyle = "#aab4c2";
    for (const d of [10, 100, 1000, 10000, 100000]){ g.beginPath(); g.moveTo(lx(d), T); g.lineTo(lx(d), B); g.stroke(); g.fillText(d >= 1000 ? d / 1000 + "k" : String(d), lx(d) - 8, H2 - 5); }
    g.setLineDash([2, 3]); g.beginPath(); g.moveTo(L, ly(1)); g.lineTo(Rr, ly(1)); g.stroke(); g.setLineDash([]); g.fillText("ρ=1", 4, ly(1) + 3); g.fillText("0", 18, B);
    const line = (off, dash, col) => { g.strokeStyle = col; g.lineWidth = 1.8; g.setLineDash(dash); g.beginPath(); for (let i = 0; i < C.length; i += 4){ const x = lx(C[i]), y = ly(C[i + off]); i ? g.lineTo(x, y) : g.moveTo(x, y); } g.stroke(); g.setLineDash([]); };
    line(1, [], "#7fd0ff"); line(2, [5, 3], "#ffcf7a");
    // this street's seat, if the street world is up: devices that round with it now, through cap_seat
    let here = null; if (typeof M !== "undefined" && M && typeof seatRounders === "function" && typeof capSeat === "function"){ try { const sids = []; for (let i = 0; i < M.n; i++) if (nodeKind(i) === 4) sids.push(i); if (sids.length){ const d = seatRounders(sids[0]); if (d > 0){ const [sh, rd, cd] = bsCap(); const o = capSeat(d, sh, rd, cd); if (o) here = { devices: d, rho: o.rhoCpu }; } } } catch (e) {} }
    if (here && here.devices >= 10){ const x = lx(here.devices), y = ly(here.rho); g.fillStyle = "#fff"; g.beginPath(); g.arc(x, y, 3.5, 0, 6.2832); g.fill(); }
    this.last = { setting: SEAT.key, points: C.length / 4, first: { devices: C[0], rho_cpu: C[1] }, last: { devices: C[C.length - 4], rho_cpu: C[C.length - 3] }, this_street: here };
  },
  status(){ const h = this.last.this_street; return "solid ρ CPU · dashed ρ permits" + (h ? ` · this street: ${h.devices} devices, ρ ${h.rho.toFixed(3)}` : ""); } });

// ------------------------------------------------------------------ Phase 2 contracts (not shown until the export exists)
const CONTRACT = (id, title, cls, exp, contract) => layer({ id, title, cls, source: exp.join(", "), contract, available: has(...exp), pending: true });
CONTRACT("topology", "Networks, exchanges and the path a row takes", "MODEL", ["topology_view_points", "topology_view_links", "topology_view_path"],
  "points f32 × 5 (lat, lon, role 0 AS 1 IXP 2 POP 3 seat site, log size, region); links u32 × 3 (a, b, kind 0 customer-provider 1 peering 2 IXP 3 long-haul); path f32 × 2 per hop + [AS hops, km].");
CONTRACT("failures", "Failure domains going dark, the mesh re-forming", "MODEL", ["failures_view_nodes", "failures_view_domains", "failures_view_step", "failures_view_steps"],
  "nodes f32 × 3 (lat, lon, kind); domains f32 × 6 (lat, lon, radius km, kind, step down, step up | -1); step(s) u32 per node: component id, u32::MAX = down; summary f32 × 4 per step.");
CONTRACT("nat", "How each pair connects", "MODEL", ["nat_view_nodes", "nat_view_pairs", "nat_view_joint"],
  "nodes u32 × 4 (access class, NAT behaviour, IPv6, locality); pairs u32 × 3 (a, b, mechanism 0 v6 1 v4 2 punched 3 relayed 4 failed); joint f32 dense with dims first.");
CONTRACT("media", "A room call: tree and per-receiver quality", "MODEL", ["media_view_members", "media_view_tree", "media_view_edges", "media_view_rx", "media_view_steps"],
  "members f32 × 4 (lat, lon, access class, uplink B/s); tree u32 × 3 (parent, child, layer); edges(step) f32 × 3 (rate B/s, loss ‰, queue ms); rx(step) f32 × 3 (layer, stall, delay ms).");
CONTRACT("people", "The week's rhythm and a flash crowd", "MODEL", ["people_view_localities", "people_view_week", "people_view_ties", "people_view_flash"],
  "localities f32 × 3 (lat, lon, weight); week f32 localities × 168; ties u32 × 3 (a, b, weight); flash f32 × 6 (lat, lon, start, peak, end hour-of-week, peak multiplier).");
CONTRACT("holon", "Certificate envelopes per cell (refusals shown as refusals)", "DERIVED", ["holon_view_cells", "holon_view_bounds"],
  "cells f32 × 8 (lat, lon, half-size°, level, state 0 certified 1 refused 2 not evaluated, bound index, margin, defect); bounds named by index.");
CONTRACT("calibration", "Seat knee with its interval", "DERIVED", ["calibration_view_curve", "calibration_view_knee", "calibration_view_residuals"],
  "curve f32 × 5 (devices, central, lower, upper, class); knee [lower, central, upper, level] or -1; residuals f32 × 4 (x, observed, predicted, out-of-sample).");

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
  check(){ return Object.fromEntries(REG.filter((d) => this.on(d.id)).map((d) => [d.id, d.last])); },
};
window.CEWP_LAYERS = Object.fromEntries(REG.map((d) => [d.id, d]));
let dock = null, open = false;
function render(){
  if (!dock){ dock = document.createElement("section"); dock.id = "layer-dock"; dock.setAttribute("aria-label", "Layers"); document.body.appendChild(dock);
    dock.addEventListener("click", (e) => { const b = e.target.closest("button"); if (!b) return; e.stopPropagation();
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
      <span class="ly-cls" title="class of what this layer draws (FSD §8.2)${d.inputs ? "; inputs: " + d.inputs : ""}">${d.cls}${d.inputs ? " · inputs " + d.inputs.split(" ")[0] : ""}</span>
      ${on ? `<div class="ly-st" id="ly-st-${d.id}"></div>${d.panel ? `<canvas class="ly-chart" id="ly-ch-${d.id}" aria-label="${d.title} chart"></canvas>` : ""}${ctl}<details class="ly-src"><summary>Where this comes from</summary><p><code>${d.source}</code></p><p>${d.contract}</p></details>` : ""}</div>`; }).join("") +
    `<p class="ly-note">Not data: the ocean's colour and glint and the atmosphere are illustration.</p></div>`;
  refresh();
}
function refresh(){ if (!dock || !open) return; for (const d of REG){ if (!LAYERS.on(d.id)) continue; const el = $id("ly-st-" + d.id); if (el && d.status) el.textContent = d.status(); const c = $id("ly-ch-" + d.id); if (c && d.panel) d.panel(c); } }
// once the kernel is in, show the dock and fill the layers that are on
(function boot(){ if (typeof wasm === "undefined" || !wasm || !W || !bufs){ setTimeout(boot, 300); return; } for (const id of KEEP.on){ const d = LAYERS.def(id); if (d && d.onShow && d.available()) try { d.onShow(); } catch (e) { console.warn(e); } } render(); })();
})();
