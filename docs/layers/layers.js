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
  contract: "The population grid the sim places people on (GHS-POP 2020, a MEASURED input, 1.5° cells, scattered ±cellRad), blurred 3 px into a 1024×512 image; lit where the sun from clock_sun(t) is below the horizon. Not satellite night lights. Read once at load; the sun every frame.",
  available: () => true, status(){ this.last = { sun_lat_deg: +GFX.sunLL[0].toFixed(2), sun_lon_deg: +GFX.sunLL[1].toFixed(2), utc_hour: +GFX.utc.toFixed(3) }; return "sun over " + GFX.sunLL.map((v) => v.toFixed(1) + "°").join(", ") + " · " + String(Math.floor(GFX.utc)).padStart(2, "0") + ":" + String(Math.floor((GFX.utc % 1) * 60)).padStart(2, "0") + " UTC"; } });

// ------------------------------------------------------------------ the tick's sample arcs
window.CEWP_LAYERS["arcs"] = layer({ id: "arcs", title: "Deliveries crossing localities (a sample)", cls: "MODEL", inputs: "MODEL (generated world and traffic mix)",
  source: "tick → arcs buffer (u32 × 2: signer, target | tier << 28)",
  contract: "tick() writes up to 420 (signer, target, tier) pairs, itself a 1-in-3 to 1-in-48 sample of the deliveries it ran; the page adds at most 2 a tick at orbit zoom (globe.js ARC_ADD_ORBIT, about 17 a second at 120 ms ticks; 10 a tick closer in), each fading in over at least 0.6 s and out over at least 1.4 s, drawn with ordinary alpha blending and a capped head so crossings never sum to white; with prefers-reduced-motion they are drawn still, with no travelling head. Tier: 0 shaped hop, 1 seat replication, 2 relay tree / unshaped, 3 onion. Read every tick (120 ms). The exact counts are on the flows layer.",
  available: () => true, status(){ this.last = { arcs_on_screen: GFX.arcsDrawn || 0, new_per_tick_at_orbit: 2 }; return this.last.arcs_on_screen + " arcs on screen · a sample: at most 2 new a tick at orbit zoom"; } });

// ------------------------------------------------------------------ flows between localities (exact counts)
const FLOW = { cent: null, W: null, win: [], sum: null, arcs: [], top: [], checked: "", N: 0 };
const FLOW_WINDOW = 25; // ticks summed (3 s of sim at 120 ms a tick)
const FLOW_TOP = 60;
window.CEWP_LAYERS["flows"] = layer({ id: "flows", title: "Flows between localities (exact counts)", cls: "MODEL", inputs: "MODEL (generated world, 90/9/1 room mix)",
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
window.CEWP_LAYERS["alm"] = layer({ id: "alm", title: "A room's relay tree (CC 6.1.6)", cls: "MODEL", inputs: "MODEL (uplinks, class shares, RTT formula, trust)",
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
window.CEWP_LAYERS["custody"] = layer({ id: "custody", title: "Who holds a room's chunk", cls: "MODEL", inputs: "MODEL (reachability draw)",
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

// ------------------------------------------------------------------ seat load against fleet size on the calibrated model (seat_hooks.rs)
const SEAT = { curve: new Map(), key: "" };
window.CEWP_LAYERS["seat"] = layer({ id: "seat", title: "Serve-seat load across fleet sizes (both publish-set cases)", cls: "MODEL", inputs: "MODEL, fit to seat-1 counters (MEASURED); g FRONTIER",
  source: "view_calibration_curve(0) and (4) → view slot 17 (cewp_sim::seat_hooks::calibration_view_curve) · view_seat_fleet(devices, g) → slot 32 for this street's seat (cewp_sim::seat_view::seat_view_fleet)",
  contract: "One v45.0.0 seat (edge release/45.0.0 at 8e46165) on the calibrated seat model: CostParams::calibrated() with key_publish_set growth g = 0 (the publish set stays at today's size) and g = 1 (it grows with the fleet); which holds is FRONTIER. Each band is ρ, the binding utilisation (the largest of CPU, sweep permits and the busiest key-plane lock), as outer bounds over the input intervals (seat_calibration::input_intervals, vertex method, each term alone), at 24 fleet sizes from 500 to 100,000 devices: f32 × 6 (devices, ρ central, lower, upper, class 2 MODEL, binding code). Bounds, not a probability. The street's seat is seatRounders() through view_seat_fleet: its ρ interval at both g. No knee here: the knee is an interval on the seat card and in the calibration layer.",
  available: has("view_calibration_curve", "view_seat_fleet", "view_ptr"),
  compute(){ for (const sc of [0, 4]) if (!SEAT.curve.has(sc)){ wasm.view_calibration_curve(sc); SEAT.curve.set(sc, VW(17)); } },
  onShow(){ later(this); },
  panel(c){ // ρ against devices on a log axis: two outer bands (g 0 amber, g 1 blue), ρ 0.70 and ρ 1 marked
    if (this.busy || SEAT.curve.size < 2) return;
    const dpr = window.devicePixelRatio || 1, W2 = c.clientWidth || 260, H2 = 132; if (c.style.height !== H2 + "px") c.style.height = H2 + "px"; if (c.width !== Math.round(W2 * dpr)){ c.width = Math.round(W2 * dpr); c.height = H2 * dpr; }
    const g = c.getContext("2d"); g.setTransform(dpr, 0, 0, dpr, 0, 0); g.clearRect(0, 0, W2, H2);
    const L = 34, B = H2 - 34, T = 8, Rr = W2 - 10, d0 = Math.log10(500), lx = (d) => L + (Math.log10(d) - d0) / (5 - d0) * (Rr - L), RM = 2, ly = (r) => B - Math.min(RM, Math.max(0, r)) / RM * (B - T);
    g.strokeStyle = "rgba(200,210,225,0.22)"; g.lineWidth = 1; g.font = "10px " + MFONT; g.fillStyle = "#aab4c2";
    for (const d of [1000, 10000, 100000]){ g.beginPath(); g.moveTo(lx(d), T); g.lineTo(lx(d), B); g.stroke(); const t = d / 1000 + "k"; g.fillText(t, Math.min(lx(d) - 8, Rr - g.measureText(t).width), B + 11); }
    g.fillText("500", L - 4, B + 11);
    for (const [r, t] of [[0.7, "ρ 0.70"], [1, "ρ 1"]]){ g.setLineDash(r === 1 ? [2, 3] : [5, 3]); g.strokeStyle = "rgba(255,255,255,0.5)"; g.beginPath(); g.moveTo(L, ly(r)); g.lineTo(Rr, ly(r)); g.stroke(); g.setLineDash([]); g.fillText(t, 2, ly(r) + 3); }
    const band = (C, fill, edge) => { g.fillStyle = fill; g.beginPath(); for (let i = 0; i < C.length; i += 6){ const x = lx(C[i]), y = ly(C[i + 3]); i ? g.lineTo(x, y) : g.moveTo(x, y); } for (let i = C.length - 6; i >= 0; i -= 6) g.lineTo(lx(C[i]), ly(C[i + 2])); g.closePath(); g.fill();
      g.strokeStyle = edge; g.lineWidth = 1.2; for (const off of [2, 3]){ g.beginPath(); for (let i = 0; i < C.length; i += 6){ const x = lx(C[i]), y = ly(C[i + off]); i ? g.lineTo(x, y) : g.moveTo(x, y); } g.stroke(); } };
    band(SEAT.curve.get(0), "rgba(255,207,122,0.22)", "#ffcf7a"); band(SEAT.curve.get(4), "rgba(127,208,255,0.25)", "#7fd0ff");
    // this street's seat, if the street world is up: its ρ interval at both g, as a vertical bar
    let here = null; if (typeof M !== "undefined" && M && typeof seatRounders === "function" && typeof seatFleet === "function"){ try { for (let i = 0; i < M.n; i++) if (nodeKind(i) === 4){ const d = seatRounders(i); if (d > 0){ const a = seatFleet(d, 0), b = seatFleet(d, 4); if (a && b) here = { devices: d, lo: Math.min(a.rho[0], b.rho[0]), hi: Math.max(a.rho[1], b.rho[1]) }; } break; } } catch (e) {} }
    if (here){ const x = lx(Math.max(500, here.devices)); g.strokeStyle = "#fff"; g.lineWidth = 2; g.beginPath(); g.moveTo(x, ly(here.lo) + 2); g.lineTo(x, ly(here.hi) - 2); g.stroke(); g.fillStyle = "#fff"; g.fillText((here.devices < 500 ? "◂ " : "") + "this street's seat", Math.min(x + 4, Rr - 100), ly(here.hi) - 5); }
    let xx = 4, yy = H2 - 6; for (const [col, t] of [["#ffcf7a", "publish set stays (g 0)"], ["#7fd0ff", "grows with the fleet (g 1)"]]){ const tw = 14 + g.measureText(t).width; if (xx + tw > W2){ xx = 4; yy -= 12; } g.fillStyle = col; g.fillRect(xx, yy - 7, 8, 8); g.fillStyle = "#aab4c2"; g.fillText(t, xx + 11, yy); xx += tw; }
    const c0 = SEAT.curve.get(0), c4 = SEAT.curve.get(4), last = c0.length - 6;
    this.last = { points: c0.length / 6, first_devices: c0[0], g0_rho_lower_at_100k: +c0[last + 2].toFixed(3), g0_rho_upper_at_100k: +c0[last + 3].toFixed(3), g1_rho_lower_at_100k: +c4[last + 2].toFixed(3), g1_rho_upper_at_100k: +c4[last + 3].toFixed(3) };
    if (here){ this.last.street_devices = here.devices; this.last.street_rho_lower = +here.lo.toFixed(4); this.last.street_rho_upper = +here.hi.toFixed(4); }
    this.info = { band: "outer bounds over the input intervals, not a probability", growth: "g 0 and g 1 (FRONTIER: which holds is not known)" };
  },
  status(){ const h = this.last; if (this.busy) return "running the seat model's input intervals once…"; return h.points ? `at 100,000 devices ρ ${h.g0_rho_lower_at_100k} to ${h.g0_rho_upper_at_100k} if the publish set stays, ${h.g1_rho_lower_at_100k} to ${h.g1_rho_upper_at_100k} if it grows with the fleet` + (h.street_devices ? ` · this street's seat: ${h.street_devices} devices, ρ ${h.street_rho_lower} to ${h.street_rho_upper}` : "") + " · MODEL, bounds over the inputs · the knee is on the seat card" : "–"; } });

// ------------------------------------------------------------------ world model: CIRISHolon's tables (FSD/twin/holon-intake.md)
// Each consumed table is a row here with its verdict and envelope; the charts are drawn from the table itself.
// Hatched = not certified (unresolved or refused); a solid edge = certified.
const WM = { cls: 1, hcls: 0 };
const WM_CLASSES = ["village", "suburban", "terraced", "apartment blocks", "dense fabric"];
function wmCanvas(c, H2){ const dpr = window.devicePixelRatio || 1, W2 = c.clientWidth || 260; if (c.style.height !== H2 + "px") c.style.height = H2 + "px"; if (c.width !== Math.round(W2 * dpr)){ c.width = Math.round(W2 * dpr); c.height = H2 * dpr; } const g = c.getContext("2d"); g.setTransform(dpr, 0, 0, dpr, 0, 0); g.clearRect(0, 0, W2, H2); g.font = "10px " + MFONT; return [g, W2]; }
function wmHatch(g, x, y, w, h, col){ g.save(); g.beginPath(); g.rect(x, y, w, h); g.clip(); g.strokeStyle = col; g.lineWidth = 1; for (let k = -h; k < w; k += 4){ g.beginPath(); g.moveTo(x + k, y + h); g.lineTo(x + k + h, y); g.stroke(); } g.restore(); g.strokeStyle = col; g.strokeRect(x + 0.5, y + 0.5, w - 1, h - 1); }
window.CEWP_LAYERS["wm-uneven"] = layer({ id: "wm-uneven", title: "World model · people inside a cell, by region (UNEVEN-0)", cls: "MODEL", inputs: "MODEL via CIRISHolon, unresolved",
  source: "docs/geo/holon/uneven0.json (tools/holon/import.py from CIRISHolon conformance/scene/tables/uneven0: transition_table.json and transition_table_by_region.json)",
  contract: "CIRISHolon's UNEVEN-0 reads GHS-POP at 30 arc-seconds (about 0.9 km tiles) inside every peopled 1/8-degree cell: for each cell class, the share of tiles and of people at each tile class, world-wide and by the twin's nine regions (docs/geo/regions.json). The chart: for the chosen cell class, the share of the cell's people who live in tiles denser than the cell's class, one bar per region and the world. A region with fewer than 30 cells of the class publishes no shares (hatched grey, the world row stands in). Every row's verdict is unresolved: a reading of the reference, no generator scored. Limits: nothing below about 1 km is seen; the fine grid is itself a model; coastal cells read as uneven. The twin's generator reads it (popchain::incell, the kernel's in-cell seam): each cell's people are divided over its ~0.9 km pixels by the regional row where published, else the world row (CIRISHolon's divide() rule, matched exactly on the world table; which pixel is the town is the twin's display rule, MODEL: the 1/8-degree people field interpolated at the pixel plus a pull toward the cell's Natural Earth place), and the globe draws the result at region and city zoom as one texture per 1.5-degree block (incell.js), hatched because unresolved. The switch below sets the legacy even spread for comparison.",
  available: () => typeof holLoad === "function",
  onShow(){ holLoad(); },
  panel(c){
    const u = typeof HOLT !== "undefined" && HOLT.t.uneven0; if (!u) return;
    const [g, W2] = wmCanvas(c, 150), k = WM.cls, D = u.data, above = (v) => v.slice(k + 2).reduce((a, b) => a + b, 0) / 1e6;
    const bars = D.regions.map((name, r) => { const e = D.regional[r][k]; return { name, v: e.people ? above(e.people) : null, cells: e.cells }; });
    bars.push({ name: "world", v: above(D.world.people[k]), cells: D.world.cells[k], world: true });
    const L = 8, B = 118, T = 14, bw = (W2 - L - 6) / bars.length;
    g.fillStyle = "#aab4c2"; g.fillText(`${WM_CLASSES[k]}: people in denser 1 km tiles`, L, 10);
    bars.forEach((b, i) => { const x = L + i * bw + 2, w = bw - 4;
      if (b.v === null){ wmHatch(g, x, B - 18, w, 18, "rgba(170,170,190,0.55)"); }
      else { const h = Math.max(1, b.v * (B - T)); wmHatch(g, x, B - h, w, h, b.world ? "rgba(255,255,255,0.75)" : "rgba(255,207,122,0.8)"); g.fillStyle = "#e6ebf2"; g.fillText(Math.round(b.v * 100) + "%", x, B - h - 2); }
      g.save(); g.translate(x + w / 2 + 3, B + 4); g.rotate(Math.PI / 4); g.fillStyle = "#aab4c2"; g.fillText(b.name.replace("Sub-Saharan Africa", "SS Africa").replace("SE Asia & Oceania", "SE Asia/Oc.").replace("Russia & C. Asia", "Russia/C.As."), 0, 0); g.restore(); });
    this.last = Object.fromEntries(bars.map((b) => [b.name.toLowerCase().replace(/[^a-z]+/g, "_"), b.v === null ? -1 : +b.v.toFixed(3)]));
    this.info = { class: WM_CLASSES[k], verdict: "unresolved (every row)", holon_commit: u.holon_commit, renormalised_vectors: String(u.renormalised.length), below_floor: "hatched grey: fewer than 30 cells, the world row stands in" };
  },
  controls: [{ label: "Next cell class", act(){ WM.cls = (WM.cls + 1) % 5; gfxDirty(300); } },
    { label: "In-cell rule: CIRISHolon / legacy", act(){ if (typeof tlIncell === "function") tlIncell(TL.incell === false); } }],
  status(){ const u = typeof HOLT !== "undefined" && HOLT.t.uneven0; return u ? `in-cell rule: ${typeof TL !== "undefined" && TL.incell === false ? "legacy even spread (no certificate)" : "CIRISHolon UNEVEN-0 (unresolved)"} · ${WM_CLASSES[WM.cls]} cells · verdict: unresolved (a reading, not a certificate) · envelope: world, nine regions, five classes, ~1 km grain · CIRISHolon ${u.holon_commit.slice(0, 7)}` : "loading CIRISHolon's table…"; } });
window.CEWP_LAYERS["wm-household"] = layer({ id: "wm-household", title: "World model · household size against census (HOUSEHOLD-0)", cls: "MODEL", inputs: "MODEL via CIRISHolon: a verdict",
  source: "docs/geo/holon/household0_us.json and household0_de.json (tools/holon/import.py from CIRISHolon conformance/scene/tables/household0_us and household0_de)",
  contract: "CIRISHolon scored the twin's own household-size rule (docs/geo/household_hist.json) against census tables with budgets fixed in advance. United States (ACS 2020-2024, keyed by tract density): mean persons per household certified (2.50 vs 2.53), one-person share refused (0.18 vs 0.29). Germany (Zensus 2022, 1 km tiles): refused on both (2.50 vs 2.04; 0.18 vs 0.43). The chart: share of households by size, the twin's rule against each reference for the chosen class (bands as each census publishes them). For the twin's own leaves every verdict is outside the envelope (a census unit's density key, two countries). The rule is kept as the labelled legacy fallback and not retuned: CIRISHolon's household generator replaces it.",
  available: () => typeof holLoad === "function",
  onShow(){ holLoad(); },
  panel(c){
    const us = typeof HOLT !== "undefined" && HOLT.t.household0_us, de = us && HOLT.t.household0_de; if (!us || !de) return;
    const cls = ["nation", ...WM_CLASSES][WM.hcls], [g, W2] = wmCanvas(c, 150), gen = us.data.generator.shares_by_size_0_to_7;
    const ur = us.data.rows[cls], dr = de.data.rows[cls], L = 8, B = 118, T = 14, n = 7, bw = (W2 - L - 6) / n;
    const twin = [1, 2, 3, 4, 5, 6, 7].map((s) => gen[s] || 0);
    const usv = ur ? ur.shares_ppm.map((v) => v / 1e6) : null; // bands 1..7+
    const dev = dr ? dr.shares_ppm.map((v) => v / 1e6) : null, dband = dr ? dr.bands : [];
    g.fillStyle = "#aab4c2"; g.fillText(`${cls}: dashed twin · US · DE`, L, 10);
    const ly = (v) => Math.max(1, v * 2 * (B - T));
    for (let s = 0; s < n; s++){ const x = L + s * bw + 2, w = (bw - 4) / 3;
      g.strokeStyle = "rgba(200,200,210,0.7)"; g.setLineDash([3, 2]); g.strokeRect(x + 0.5, B - ly(twin[s]) + 0.5, w - 1, ly(twin[s]) - 1); g.setLineDash([]);
      const uv = usv && s < usv.length ? usv[s] : null; if (uv !== null){ const vd = ur.p1 ? ur.p1.verdict : "unresolved"; if (vd === "certified"){ g.fillStyle = "rgba(124,240,176,0.8)"; g.fillRect(x + w, B - ly(uv), w, ly(uv)); } else wmHatch(g, x + w, B - ly(uv), w, ly(uv), vd === "refused" ? "rgba(255,110,90,0.85)" : "rgba(170,170,190,0.7)"); }
      const di = dband.findIndex((b) => b === String(s + 1) || (b.endsWith("+") && s + 1 >= parseInt(b))); const dv = dev && di >= 0 && (dband[di] === String(s + 1) || s + 1 === parseInt(dband[di])) ? dev[di] : null;
      if (dv !== null){ const vd = dr.p1 ? dr.p1.verdict : "unresolved"; wmHatch(g, x + 2 * w, B - ly(dv), w, ly(dv), vd === "refused" ? "rgba(255,150,110,0.85)" : "rgba(170,170,190,0.7)"); }
      g.fillStyle = "#aab4c2"; g.fillText(s === 6 ? "7+" : String(s + 1), x + w, B + 11); }
    this.last = { class_shown: WM.hcls, twin_one_person: +twin[0].toFixed(3), us_one_person: usv ? +usv[0].toFixed(3) : -1, de_one_person: dev ? +dev[0].toFixed(3) : -1 };
    this.info = { us_mean: ur && ur.mean ? `${ur.mean.verdict}: ${ur.mean.reason}` : "-", us_composition: ur && ur.p1 ? `${ur.p1.verdict}: ${ur.p1.reason}` : "-", de_mean: dr && dr.mean ? `${dr.mean.verdict}: ${dr.mean.reason}` : "-", de_composition: dr && dr.p1 ? `${dr.p1.verdict}: ${dr.p1.reason}` : "-", envelope: "United States (tract density) and Germany (1 km tiles); a twin leaf is outside both" };
  },
  controls: [{ label: "Next class", act(){ WM.hcls = (WM.hcls + 1) % 6; gfxDirty(300); } },
    { label: "Household rule: CIRISHolon / legacy", act(){ if (typeof tlHouseholds === "function") tlHouseholds(TL.housegen === false); } }],
  status(){ const us = typeof HOLT !== "undefined" && HOLT.t.household0_us; if (!us) return "loading CIRISHolon's tables…"; const cls = ["nation", ...WM_CLASSES][WM.hcls], u = us.data.rows[cls], d = HOLT.t.household0_de && HOLT.t.household0_de.data.rows[cls];
    const v = (r, k) => r && r[k] ? r[k].verdict : "–"; return `household rule on the street: ${typeof TL !== "undefined" && TL.housegen === false ? "legacy (no certificate)" : "CIRISHolon HOUSEGEN-1 per ~1 km tile"} · these verdicts score the legacy rule: ${cls} · US: mean ${v(u, "mean")}, composition ${v(u, "p1")} · Germany: mean ${v(d, "mean")}, composition ${v(d, "p1")}`; } });

// presence (PRESENCE-0, US time diaries): the first rhythm, drawn as it is: a 24-hour band per day type, six states stacked
const PRES_COL = ["#3b4a7a", "#7fa0e6", "#ffcf7a", "#7ce0b0", "#5a5f78", "#ff9a7a"];
window.CEWP_LAYERS["wm-presence"] = layer({ id: "wm-presence", title: "World model · who is where, hour by hour (PRESENCE-0, US)", cls: "MODEL", inputs: "MODEL via CIRISHolon: a verdict",
  source: "docs/geo/holon/presence0_us.json (tools/holon/import.py from CIRISHolon conformance/scene/tables/presence0_us)",
  contract: "<span class=\"fig\" data-claim=\"holon-presence0-us\">CIRISHolon's PRESENCE-0</span>: the American Time Use Survey diaries of 2022-2024, ages 15 and over, as the share of persons in six states (home asleep, home awake, workplace or school, elsewhere awake, elsewhere asleep, travelling) by day type (Monday to Thursday, Friday, Saturday, Sunday) and clock hour, with standard errors. The registry row presence.people. Drawn as it is: a stacked 24-hour band per day type. The twin's stand-in (awake 0.85 of the time, model.diurnal.v1) is refused: awake 0.623 of the time (15.0 h a day; 0.04 at 03:00, 0.98 early evening). At most 0.51 of the employed are at a workplace at once; 0.61 of awake time is at home. Limits: one diary per household, so whether anyone is home is a bracket; nobody under 15; a person awake is not a device awake; nothing about devices or networks; it has a year (at home 9 to 17 on working days rose from 0.414 in 2015-2019 to 0.501 in 2022-2024). United States only: outside it the twin keeps its stand-in, hatched ('no reference outside the United States'). The kernel-side use (tinting activity by the awake share at local hour) is twin-theory's presence seam.",
  available: () => typeof holLoad === "function",
  onShow(){ holLoad(); },
  panel(c){
    const t = typeof HOLT !== "undefined" && HOLT.t.presence0_us; if (!t) return;
    const D = t.data, [g, W2] = wmCanvas(c, 172), L = 66, R = W2 - 6, bh = 26, gap = 9, T0 = 14;
    g.fillStyle = "#aab4c2"; g.fillText("share of persons, by clock hour (0 to 23)", L, 10);
    D.day_types.forEach((dt, k) => { const y0 = T0 + k * (bh + gap); g.fillStyle = "#aab4c2"; g.fillText(dt.replace("Monday to Thursday", "Mon-Thu"), 4, y0 + bh / 2 + 3);
      for (let h = 0; h < 24; h++){ const x0 = L + (R - L) * h / 24, w = (R - L) / 24 + 0.5; let y = y0 + bh;
        D.table[k][h].forEach((sh, st) => { const hh = sh * bh; g.fillStyle = PRES_COL[st]; g.fillRect(x0, y - hh, w, hh); y -= hh; }); } });
    let xx = 4, yy = 172 - 4; for (let st = 0; st < 6; st++){ const tx = D.states[st], tw = 12 + g.measureText(tx).width; if (xx + tw > W2){ xx = 4; yy -= 11; } g.fillStyle = PRES_COL[st]; g.fillRect(xx, yy - 7, 8, 8); g.fillStyle = "#aab4c2"; g.fillText(tx, xx + 10, yy); xx += tw + 4; }
    const tn = D.three_numbers.weekly_mean_awake_share;
    this.last = { awake_share: +tn.reference.toFixed(3), awake_hours: +tn.awake_hours_a_day.toFixed(1), twin_awake_share: tn.twin, lowest_hour: +tn.lowest_and_highest_hour_of_week[0].toFixed(3), highest_hour: +tn.lowest_and_highest_hour_of_week[1].toFixed(3), diaries: D.diaries };
    this.info = { verdict: "the twin's awake 0.85: refused", envelope: "United States, 2022-2024, ages 15 and over", years: D.years };
  },
  status(){ const t = typeof HOLT !== "undefined" && HOLT.t.presence0_us; if (!t) return "loading CIRISHolon's table…"; const tn = t.data.three_numbers.weekly_mean_awake_share, w = t.data.three_numbers.employed_at_workplace_Mon_Thu;
    return `awake ${tn.reference.toFixed(3)} of the time (${tn.awake_hours_a_day.toFixed(1)} h a day; ${tn.lowest_and_highest_hour_of_week[0].toFixed(2)} at the quietest hour, ${tn.lowest_and_highest_hour_of_week[1].toFixed(2)} early evening) against the twin's 0.85: refused · at most ${w.peak_share.toFixed(2)} of the employed at a workplace at once · envelope: United States 2022-2024; outside it the twin's stand-in, no reference`; } });
// power (POWER-0, priors): device, network and data-centre power by class and state; the energy pass (twin/theory) consumes it
window.CEWP_LAYERS["wm-power"] = layer({ id: "wm-power", title: "World model · device and network power, priors (POWER-0)", cls: "MODEL", inputs: "MODEL via CIRISHolon, unresolved",
  source: "docs/geo/holon/power0.json (tools/holon/import.py from CIRISHolon conformance/scene/tables/power0, version 3)",
  contract: "<span class=\"fig\" data-claim=\"holon-power0\">CIRISHolon's POWER-0</span>: device, network and data-centre power by class and state, each row with a would-be-on-anyway flag, a year, a source and a basis (four rows MEASURED sales-weighted averages for new US network equipment; the rest read from documents). Verdict unresolved on every row: priors, nothing scored. The chart lists the watts rows by class on a log scale, hatched (unresolved), solid edge where the row is would-be-on-anyway. Network energy is mostly baseload per line: pricing traffic kept local at an average kWh per GB would overstate any saving by a large factor. Consumed by the energy pass on twin/theory, not by the globe.",
  available: () => typeof holLoad === "function",
  onShow(){ holLoad(); },
  panel(c){
    const t = typeof HOLT !== "undefined" && HOLT.t.power0; if (!t) return;
    const rows = t.data.rows.filter((r) => r.unit === "W" && r.value > 0).slice(0, 14), H2 = 16 + rows.length * 11, [g, W2] = wmCanvas(c, H2), L = 150, R = W2 - 40, lx = (v) => L + (Math.log10(v) + 1) / 4 * (R - L);
    g.fillStyle = "#aab4c2"; g.fillText("watts, log scale (0.1 to 1,000)", L, 10);
    rows.forEach((r, i) => { const y = 14 + i * 11; g.fillStyle = "#aab4c2"; g.fillText((r.device_class + ", " + r.state).slice(0, 26), 2, y + 8);
      const w = Math.max(1, lx(Math.max(0.1, r.value)) - L); wmHatch(g, L, y + 1, w, 8, /yes/.test(String(r.would_be_on_anyway)) ? "rgba(255,207,122,0.85)" : "rgba(170,170,190,0.7)"); g.fillStyle = "#e6ebf2"; g.fillText(r.value + " W", L + w + 3, y + 8); });
    this.last = { rows: t.data.rows.length, watts_rows_shown: rows.length };
    this.info = { verdict: "unresolved on every row (priors)", consumer: "twin/theory energy pass" };
  },
  status(){ const t = typeof HOLT !== "undefined" && HOLT.t.power0; return t ? `${t.data.rows.length} rows · verdict: unresolved (priors, nothing scored) · consumer: the energy pass (twin/theory) · ${String(t.data.finding).slice(0, 160)}` : "loading CIRISHolon's table…"; } });

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
    const dpr = window.devicePixelRatio || 1, W2 = c.clientWidth || 260, H2 = 200; if (c.style.height !== H2 + "px") c.style.height = H2 + "px"; if (c.width !== Math.round(W2 * dpr)){ c.width = Math.round(W2 * dpr); c.height = H2 * dpr; NAT.key = ""; }
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
window.CEWP_LAYERS["holon"] = layer({ id: "holon", title: "Where a coarse tier is trusted (certificates)", cls: "MODEL", inputs: "MODEL (the micro world under the tier)",
  source: "view_holon(43) → view slots 11 cells, 12 bound index, 16 bound names, 13/14 certificate table (cewp_sim::holon_view cells_of, certificates_of over holon_demo::demonstration_seeded)",
  contract: "twin-holon's R5 demonstration, run once per page (two 2-day runs, normal and outage): cells f32 × 8 (lat, lon, half-size°, level 0 the tier / 1 a cell, state 0 certified 1 refused 2 not evaluated, bound index, signed margin in online share, worst normalised defect on record 0). The cells are cells of a tier (host kind × access network), laid out as a patch centred 47.3769, 8.5417; the position is layout only, not a place. Glyph by state: certified solid square, refused crossed square, not evaluated dashed outline. Bound names come from the kernel (BOUND_NAMES). Certificate table: u32 × 9 and f32 × 20 per (tier, observable). MODEL: the certificates are computed exactly, but over the twin's MODEL micro world, and simulation output over MODEL inputs reads MODEL. A refusal is shown as a refusal: that cell goes back to the finer tier. Certified means the coarse tier held within budget over the twin's MODEL world, not the real mesh.",
  available: has("view_holon", "view_ptr"),
  compute(){
    if (HOL.cells) return; wasm.view_holon(HOL.seed); HOL.cells = VW(11); const B = VW(12), bytes = VW(16); const s = String.fromCharCode(...bytes);
    HOL.names = []; for (let i = 0; i < B.length; i += 3) HOL.names[B[i]] = s.substr(B[i + 1], B[i + 2]); HOL.cu = VW(13); HOL.cf = VW(14);
    const C = HOL.cells, st = [0, 0, 0], binds = {}; for (let i = 8; i < C.length; i += 8){ st[C[i + 4]]++; const b = C[i + 5]; if (b >= 0){ const n = HOL.names[b] || String(b); binds[n] = (binds[n] || 0) + 1; } }
    let dist = 0; for (let r = 0; r < HOL.cu.length / 9; r++) dist += HOL.cu[r * 9 + 2];
    this.last = { cells: C.length / 8 - 1, certified: st[0], refused: st[1], not_evaluated: st[2], tier_state: C[4], tier_worst_defect: +C[7].toFixed(3), certificate_rows: HOL.cu.length / 9, distributional_certified_rows: dist };
    if (isFinite(C[6])) this.last.tier_min_margin = +C[6].toFixed(4); // NaN: a whole-tier bound has no distance
    this.info = { tier_state: HOL_STATE[C[4]], tier_bound: C[5] >= 0 ? HOL.names[C[5]] : "none", binding_bounds: Object.entries(binds).map(([k, v]) => k + " " + v).join(", "), states: "0 certified, 1 refused, 2 not evaluated", label: "cells of a tier, not places", certified_means: "certified means the coarse tier held within budget over the twin's MODEL world, not the real mesh" };
  },
  onShow(){ later(this); },
  draw2D(g, F){
    const C = HOL.cells; if (!C) return;
    const c0 = proj(F, C[0], C[1]); if (c0[2] < 0.03) return;
    // the tier's cells are a street-block patch; below a legible size they are drawn enlarged around the
    // patch's spot (a callout, said so on the map), so certified, refused and unresolved read at any zoom
    const small = F.w < 600, want = small ? 150 : 230;
    const ex = proj(F, C[0], C[1] + C[2] / Math.max(0.2, Math.cos(C[0] * DEG))), ey = proj(F, C[0] + C[2], C[1]);
    const realPx = 2 * Math.max(Math.hypot(ex[0] - c0[0], ex[1] - c0[1]), Math.hypot(ey[0] - c0[0], ey[1] - c0[1]));
    const k = realPx < want ? want / Math.max(1e-6, realPx) : 1, enlarged = k > 1.01;
    // the callout: right of the spot (desktop); on a phone above or below it, on the side with more room, never over it
    const below = c0[1] < F.h / 2, boxH = want + 104;
    const ox = enlarged ? (small ? F.w / 2 - c0[0] : want / 2 + 70) : 0;
    const oy = enlarged ? (small ? (below ? 40 + want / 2 + 30 : -(40 + want / 2 + 74)) : 0) : 0;
    const at = (la, lo) => { const p = proj(F, la, lo); return [c0[0] + ox + (p[0] - c0[0]) * k, c0[1] + oy + (p[1] - c0[1]) * k]; };
    const cx = c0[0] + ox, cy = c0[1] + oy;
    if (enlarged){ // the spot and a leader to the callout
      g.strokeStyle = "rgba(255,255,255,0.75)"; g.lineWidth = 1.2; g.beginPath(); g.arc(c0[0], c0[1], 5, 0, 6.2832); g.stroke();
      g.setLineDash([3, 3]); g.beginPath(); g.moveTo(c0[0], c0[1]); g.lineTo(small ? cx : cx - want / 2 - 14, small ? (below ? cy - want / 2 - 30 : cy + want / 2 + 74) : cy); g.stroke(); g.setLineDash([]);
      g.fillStyle = "rgba(8,12,18,0.82)"; g.strokeStyle = "rgba(200,215,235,0.35)"; g.lineWidth = 1; g.beginPath(); g.roundRect ? g.roundRect(small ? cx - 150 : cx - want / 2 - 14, cy - want / 2 - 30, small ? 300 : Math.max(want + 28, 400), want + (small ? 104 : 78), 10) : g.rect(small ? cx - 150 : cx - want / 2 - 14, cy - want / 2 - 30, small ? 300 : Math.max(want + 28, 400), want + (small ? 104 : 78)); g.fill(); g.stroke(); }
    const STC = [["124,240,176", "#7cf0b0"], ["255,122,106", "#ff7a6a"], ["200,210,225", "#c8d2e1"]];
    for (let pass = 0; pass < 2; pass++) for (let i = 0; i < C.length; i += 8){ const la = C[i], lo = C[i + 1], h = C[i + 2], lv = C[i + 3], st = C[i + 4]; const hl = h / Math.max(0.2, Math.cos(la * DEG));
      const q = [[la - h, lo - hl], [la - h, lo + hl], [la + h, lo + hl], [la + h, lo - hl]].map(([a, b]) => at(a, b));
      const path = () => { g.beginPath(); g.moveTo(q[0][0], q[0][1]); for (let j = 1; j < 4; j++) g.lineTo(q[j][0], q[j][1]); g.closePath(); };
      if (lv === 0){ if (pass === 1){ path(); g.setLineDash([6, 4]); g.strokeStyle = "rgba(255,255,255,0.75)"; g.lineWidth = 1.5; g.stroke(); g.setLineDash([]); } continue; }
      const [rgb, col] = STC[Math.min(2, st)];
      if (pass === 0){ path(); g.fillStyle = `rgba(${rgb},${st === 2 ? 0.10 : 0.34})`; g.fill(); continue; }
      path(); g.save(); g.shadowColor = col; g.shadowBlur = st === 2 ? 0 : 10; g.strokeStyle = col; g.lineWidth = st === 2 ? 1.2 : 2; if (st === 2) g.setLineDash([3, 3]); g.stroke(); g.restore(); g.setLineDash([]);
      if (st === 1){ g.strokeStyle = col; g.lineWidth = 1.6; g.beginPath(); g.moveTo(q[0][0], q[0][1]); g.lineTo(q[2][0], q[2][1]); g.moveTo(q[1][0], q[1][1]); g.lineTo(q[3][0], q[3][1]); g.stroke(); } }
    // title and legend: shape and dash carry the state, colour repeats it
    const l = this.last, x0 = enlarged ? (small ? cx - 140 : cx - want / 2 - 4) : c0[0] - 80, yT = enlarged ? cy - want / 2 - 12 : c0[1] - 60, yL = enlarged ? cy + want / 2 + 20 : c0[1] + 60;
    g.font = "600 11px " + MFONT; g.lineWidth = 3; g.strokeStyle = "rgba(6,9,14,0.9)"; const t = small ? "tier certificates" + (enlarged ? " (enlarged)" : "") : "tier certificates · cells of a tier, not places" + (enlarged ? " (enlarged)" : "");
    g.strokeText(t, x0, yT); g.fillStyle = "#e8eef6"; g.fillText(t, x0, yT);
    g.font = "10.5px " + MFONT; let lx = x0;
    for (const [s2, txt] of [[0, `${l.certified} certified`], [1, `${l.refused} refused`], [2, `${l.not_evaluated} unresolved`]]){ const [rgb, col] = STC[s2];
      g.fillStyle = `rgba(${rgb},0.34)`; g.fillRect(lx, yL - 9, 11, 11); g.strokeStyle = col; g.lineWidth = 1.5; if (s2 === 2) g.setLineDash([2, 2]); g.strokeRect(lx, yL - 9, 11, 11); g.setLineDash([]);
      if (s2 === 1){ g.beginPath(); g.moveTo(lx, yL - 9); g.lineTo(lx + 11, yL + 2); g.moveTo(lx + 11, yL - 9); g.lineTo(lx, yL + 2); g.stroke(); }
      g.lineWidth = 3; g.strokeStyle = "rgba(6,9,14,0.9)"; g.strokeText(txt, lx + 15, yL); g.fillStyle = col; g.fillText(txt, lx + 15, yL); lx += 22 + g.measureText(txt).width; }
    const t2 = small ? ["cells of a tier, not places;", "certified means the twin's MODEL", "world, not the real mesh"] : ["certified means the twin's MODEL world, not the real mesh"];
    t2.forEach((tt, j) => { g.lineWidth = 3; g.strokeStyle = "rgba(6,9,14,0.9)"; g.strokeText(tt, x0, yL + 15 + 13 * j); g.fillStyle = "#aab4c2"; g.fillText(tt, x0, yL + 15 + 13 * j); });
    this.last.enlarged = enlarged ? 1 : 0;
  },
  status(){ const l = this.last; if (this.busy) return "running the tier demonstration once (two 2-day runs)…"; return l.cells !== undefined ? `■ ${l.certified} certified · ⊠ ${l.refused} refused · ⬚ ${l.not_evaluated} unresolved (not evaluated) of ${l.cells} cells · tier ${this.info.tier_state}${this.info.tier_bound !== "none" ? " (" + this.info.tier_bound + ")" : ""} · cells of a tier, not places${l.enlarged ? ", drawn enlarged around their spot" : ""} · certified means the coarse tier held within budget over the twin's MODEL world, not the real mesh` : "–"; } });

// ------------------------------------------------------------------ a correlated outage stepping through time (twin-failures: failure.rs via failure_view.rs)
const FAIL = { sc: 0, cache: new Map(), d: null, step: -1, play: null, key: "" };
const FAIL_SC = ["national shutdown", "nationwide ISP outage", "regional power failure"];
const FAIL_TGT = ["generated country", "fixed-line access network", "power-grid region"];
const FAIL_KIND = ["phone", "home node", "laptop", "hosted server", "serve seat"];
// cut-off islands by component label: hue cycles, never alone (they are also diamonds)
const FAIL_ISLE = ["#ffb347", "#ff7ad9", "#c6f36b", "#ffd84d", "#ff8f6b", "#b18cff", "#6bf3d8", "#ffa0a0"];
window.CEWP_LAYERS["failures"] = layer({ id: "failures", title: "An outage going dark, the mesh splitting and healing", cls: "MODEL", inputs: "MODEL (generated world, scenario durations); FRONTIER (tower battery, home reach)",
  source: "view_failures(s) → view slots 26 meta, 27 nodes, 28 metros, 29 steps, 30 state, 31 components (cewp_sim::failure_view over failure::evaluate, view_event_steps, view_event_components)",
  contract: "twin-failures' generated world (2,000 nodes, seed 2026: twelve generated countries, Zipf metros, four ISPs each, international cables, cloud regions) and one correlated event, evaluated at 43 hours: two before onset and 41 from onset to 1.25 × the event's duration. Scenario 0: a full national shutdown (international links and domestic cellular) of the second most populous generated country for shutdown.median_h (48 h, MODEL); 1: the busiest fixed-line ISP of the most populous country down for isp.median_h (4 h, MODEL); 2: failure::view_demo, the most populous power region dark for 12 h with a rolling return over its last 60 % (MODEL). Per node per hour, state u32: 0 down because of the event, 1 up in the largest component (the mainland), 2 up but cut off from it, 3 withholding, 4 not up even with nothing failed (asleep); component u32: failure::view_event_components' dense label (u32::MAX down). Steps f32 × 7: hour, up, mainland, cut off, components, down by the event, withholding. Placement is display only: generated country c is drawn in page region (c + 6) mod 9, its anchor at that region's heaviest free locality and its other metros spread across the region (farthest-point), nodes on a spiral around their metro, so a generated country spans several real ones and is none of them. Glyph by state: disc mainland, diamond cut off (one hue per island), cross down, faint dot asleep. Lines: each generated country's metros to its anchor, and the world's two-country international links (slot 33, cables u32 × 2) between anchors, drawn red and dashed while either end has under a fifth of its nodes on the mainland (a page-side reading of the states, not the link's own state). All MODEL: computed once per scenario, cached; the page only counts and draws.",
  available: has("view_failures", "view_ptr"),
  compute(){
    if (!FAIL.cache.has(FAIL.sc)){ wasm.view_failures(FAIL.sc); const m = VW(26);
      const mt = VW(28), anc = new Map(), links = []; for (let k = 0; k < mt.length; k += 4) if (!anc.has(mt[k + 2])) anc.set(mt[k + 2], [mt[k], mt[k + 1]]);
      for (let k = 0; k < mt.length; k += 4){ const a = anc.get(mt[k + 2]); if (a[0] !== mt[k] || a[1] !== mt[k + 1]) links.push({ dom: 1, a: mt[k + 2], b: mt[k + 2], ll: gcLL(a[0], a[1], mt[k], mt[k + 1]) }); }
      const cb = (typeof wasm.view_len === "function" && wasm.view_len(33)) ? VW(33) : new Uint32Array(0), seen = new Set();
      for (let k = 0; k < cb.length; k += 2){ const a = cb[k], b = cb[k + 1]; if (b === 0xffffffff || a === b) continue; const key = Math.min(a, b) + ":" + Math.max(a, b); if (seen.has(key)) continue; seen.add(key);
        const A = anc.get(a), B = anc.get(b); if (A && B) links.push({ dom: 0, a, b, ll: gcLL(A[0], A[1], B[0], B[1]) }); }
      FAIL.cache.set(FAIL.sc, { meta: m, nodes: VW(27), metros: mt, steps: VW(29), state: VW(30), comp: VW(31), n: m[0], S: m[1], pre: m[3], links }); }
    FAIL.d = FAIL.cache.get(FAIL.sc); if (FAIL.step < 0 || FAIL.step >= FAIL.d.S) FAIL.step = FAIL.d.pre + 1;
    const D = FAIL.d; let cx = 0, cy = 0, cz = 0, k = 0; // centroid of the nodes the event takes down or cuts off (for "zoom to")
    for (let s = D.pre + 1; s < D.S && !k; s++) for (let i = 0; i < D.n; i++){ const c = D.state[s * D.n + i]; if (c === 0 || c === 2){ const v = xyz(D.nodes[i * 4], D.nodes[i * 4 + 1]); cx += v[0]; cy += v[1]; cz += v[2]; k++; } }
    const L = Math.hypot(cx, cy, cz) || 1; D.focus = k ? [Math.asin(cy / L) / DEG, Math.atan2(cx, cz) / DEG] : [D.nodes[0], D.nodes[1]];
    this.tally();
  },
  tally(){
    const D = FAIL.d; if (!D) return; const s = Math.min(FAIL.step, D.S - 1), q = D.steps.subarray(s * 7, s * 7 + 7);
    const isl = new Set(); for (let i = 0; i < D.n; i++) if (D.state[s * D.n + i] === 2) isl.add(D.comp[s * D.n + i]);
    this.last = { scenario: FAIL.sc, step: s, steps: D.S, hour_from_onset: +q[0].toFixed(2), nodes: D.n, up: q[1], mainland: q[2], cut_off: q[3], islands: isl.size, components: q[4], down_by_event: q[5], withholding: q[6], duration_h: +D.meta[4].toFixed(2), target: D.meta[6], countries: D.meta[7], metros: D.meta[8] };
    this.info = { scenario: FAIL_SC[FAIL.sc], target: FAIL_TGT[FAIL.sc] + " " + D.meta[6] + " (generated index, no real place)", states: "0 down by the event, 1 mainland, 2 cut off, 3 withholding, 4 asleep (not up with nothing failed)", placement: "display only: generated countries spread over page regions, none a real country" };
  },
  onShow(){ this.compute(); },
  setStep(v){ if (!FAIL.d) return; FAIL.step = Math.max(0, Math.min(FAIL.d.S - 1, v)); this.tally(); FAIL.key = ""; const r = document.querySelector('input[data-ctl="failures:0"]'); if (r && +r.value !== FAIL.step){ r.value = FAIL.step; const o = r.parentElement.querySelector("output"); if (o) o.textContent = this.controls[0].show(FAIL.step); } refresh(); gfxDirty(700); },
  draw2D(g, F){
    const D = FAIL.d; if (!D) return; const s = Math.min(FAIL.step, D.S - 1), N = D.nodes, St = D.state, Cp = D.comp, o = s * D.n;
    // a soft glow under each island and under what went dark: drawn first, additive, so the eye finds the event
    const zoomPx = (() => { const a = proj(F, D.focus[0], D.focus[1]), b = proj(F, D.focus[0] + 1, D.focus[1]); return Math.hypot(b[0] - a[0], b[1] - a[1]); })();
    const r0 = Math.max(2.2, Math.min(5, zoomPx * 0.09));
    // links: a generated country's metros to its anchor (thin) and international links between anchors
    // (wide); lit when both ends still have nodes on the mainland, a red dashed line when either is dark or cut off
    const mainOf = new Map(); for (let i = 0; i < D.n; i++){ const c = N[i * 4 + 3]; const e = mainOf.get(c) || [0, 0]; e[1]++; if (St[o + i] === 1) e[0]++; mainOf.set(c, e); }
    const live = (c) => { const e = mainOf.get(c); return !!e && e[0] > 0.2 * e[1]; };
    for (const L of D.links){ const ok = live(L.a) && live(L.b);
      g.strokeStyle = ok ? (L.dom ? "rgba(105,210,255,0.35)" : "rgba(140,220,255,0.55)") : "rgba(255,90,78,0.8)"; g.lineWidth = L.dom ? 1 : 1.6; g.setLineDash(ok ? [] : [5, 4]);
      g.beginPath(); strokeLL(g, F, L.ll); g.stroke(); }
    g.setLineDash([]);
    // a dark well under every generated metro, so its nodes stand off the cohort mosaic beneath
    { const M2 = D.metros, rr = Math.max(10, zoomPx * 0.09 * Math.sqrt(80) * 1.25 + 8);
      for (let k = 0; k < M2.length; k += 4){ if (!M2[k + 3]) continue; const p = proj(F, M2[k], M2[k + 1]); if (p[2] < 0.03) continue;
        const gr = g.createRadialGradient(p[0], p[1], rr * 0.35, p[0], p[1], rr); gr.addColorStop(0, "rgba(4,7,12,0.72)"); gr.addColorStop(1, "rgba(4,7,12,0)"); g.fillStyle = gr; g.beginPath(); g.arc(p[0], p[1], rr, 0, 6.2832); g.fill(); } }
    g.save(); g.globalCompositeOperation = "lighter";
    for (let i = 0; i < D.n; i++){ const c = St[o + i]; if (c !== 0 && c !== 2 && c !== 1) continue; const p = proj(F, N[i * 4], N[i * 4 + 1]); if (p[2] < 0.03) continue;
      const R = r0 * (c === 1 ? 2.6 : 4.2), gr = g.createRadialGradient(p[0], p[1], 0, p[0], p[1], R); const col = c === 0 ? "255,80,70" : c === 2 ? "255,180,80" : "80,190,255";
      gr.addColorStop(0, `rgba(${col},${c === 1 ? 0.12 : 0.16})`); gr.addColorStop(1, `rgba(${col},0)`); g.fillStyle = gr; g.fillRect(p[0] - R, p[1] - R, 2 * R, 2 * R); }
    g.restore();
    const n = [0, 0, 0, 0, 0];
    for (let i = 0; i < D.n; i++){ const c = St[o + i]; n[c]++; const p = proj(F, N[i * 4], N[i * 4 + 1]); if (p[2] < 0.03) continue; const x = p[0], y = p[1];
      if (c === 4){ g.fillStyle = "rgba(170,185,205,0.28)"; g.fillRect(x - 0.7, y - 0.7, 1.4, 1.4); continue; } // asleep
      if (c === 0){ g.strokeStyle = "#ff5a4e"; g.lineWidth = 1.5; const h = r0 * 0.75; g.beginPath(); g.moveTo(x - h, y - h); g.lineTo(x + h, y + h); g.moveTo(x + h, y - h); g.lineTo(x - h, y + h); g.stroke(); continue; } // down: a cross
      g.lineWidth = 1; g.strokeStyle = "rgba(6,9,14,0.85)";
      if (c === 1){ g.fillStyle = N[i * 4 + 2] >= 3 ? "#e8f6ff" : "#69d2ff"; g.beginPath(); g.arc(x, y, r0 * 0.62, 0, 6.2832); g.fill(); } // mainland: disc (servers lighter)
      else if (c === 2){ g.fillStyle = FAIL_ISLE[Cp[o + i] % FAIL_ISLE.length]; const h = r0 * 0.9; g.beginPath(); g.moveTo(x, y - h); g.lineTo(x + h, y); g.lineTo(x, y + h); g.lineTo(x - h, y); g.closePath(); g.fill(); g.stroke(); } // cut off: diamond
      else { g.fillStyle = "#b18cff"; g.fillRect(x - r0 * 0.6, y - r0 * 0.6, r0 * 1.2, r0 * 1.2); } } // withholding: square
    // one label at the event's focus while anything is affected
    const q = D.steps.subarray(s * 7, s * 7 + 7), p = proj(F, D.focus[0], D.focus[1]);
    if (p[2] > 0.05){ const t1 = `${FAIL_SC[FAIL.sc]} · ${q[0] < 0 ? "before onset" : q[0].toFixed(1) + " h after onset"}`, t2 = q[5] + q[3] > 0 ? `${q[5]} down · ${q[3]} cut off from the mainland` : q[0] < 0 ? "nothing failed yet" : "recovered";
      g.font = "600 12px " + MFONT; g.lineWidth = 3.5; g.strokeStyle = "rgba(6,9,14,0.92)"; const x = Math.max(6, Math.min(p[0] + 18, F.w - 310)), y = p[1] - 26;
      g.strokeText(t1, x, y); g.fillStyle = "#fff2d8"; g.fillText(t1, x, y); g.font = "11px " + MFONT; g.strokeText(t2, x, y + 15); g.fillStyle = q[5] + q[3] > 0 ? "#ffb4a8" : "#bfe9ff"; g.fillText(t2, x, y + 15);
      const t3 = ["generated country, not a real one", "generated access network, not a real one", "generated power region, not a real one"][FAIL.sc] + " · MODEL";
      g.font = "italic 11px " + MFONT; g.lineWidth = 3.5; g.strokeStyle = "rgba(6,9,14,0.92)"; g.strokeText(t3, x, y + 30); g.fillStyle = "#d6dde8"; g.fillText(t3, x, y + 30);
      g.strokeStyle = "rgba(255,242,216,0.6)"; g.lineWidth = 1; g.beginPath(); g.moveTo(p[0] + 4, p[1] - 4); g.lineTo(x - 4, y + 4); g.stroke(); }
    this.last.drawn_down = n[0]; this.last.drawn_mainland = n[1]; this.last.drawn_cut_off = n[2];
  },
  panel(c){ // the timeline: stacked mainland / cut off / down by the event against hours, onset and cursor marked
    const D = FAIL.d; if (!D) return; const dpr = window.devicePixelRatio || 1, W2 = c.clientWidth || 260, H2 = 128;
    if (c.style.height !== H2 + "px") c.style.height = H2 + "px"; if (c.width !== Math.round(W2 * dpr)){ c.width = Math.round(W2 * dpr); c.height = H2 * dpr; FAIL.key = ""; }
    const key = FAIL.sc + ":" + FAIL.step; if (FAIL.key === key) return; FAIL.key = key;
    const g = c.getContext("2d"); g.setTransform(dpr, 0, 0, dpr, 0, 0); g.clearRect(0, 0, W2, H2);
    const L = 30, T = 8, B = 84, Rr = W2 - 8, t0 = D.steps[0], t1 = D.steps[(D.S - 1) * 7], x = (t) => L + (t - t0) / (t1 - t0) * (Rr - L);
    let top = 1; for (let s = 0; s < D.S; s++){ const q = D.steps.subarray(s * 7, s * 7 + 7); top = Math.max(top, q[1] + q[5]); }
    const y = (v) => B - v / top * (B - T);
    const band = (lo, hi, col) => { g.fillStyle = col; g.beginPath(); for (let s = 0; s < D.S; s++){ const q = D.steps.subarray(s * 7, s * 7 + 7); const X = x(q[0]), Y = y(hi(q)); s ? g.lineTo(X, Y) : g.moveTo(X, Y); } for (let s = D.S - 1; s >= 0; s--){ const q = D.steps.subarray(s * 7, s * 7 + 7); g.lineTo(x(q[0]), y(lo(q))); } g.closePath(); g.fill(); };
    band(() => 0, (q) => q[2], "rgba(105,210,255,0.55)");
    band((q) => q[2], (q) => q[2] + q[3], "rgba(255,179,71,0.75)");
    band((q) => q[2] + q[3], (q) => q[2] + q[3] + q[5], "rgba(255,90,78,0.75)");
    g.strokeStyle = "rgba(200,210,225,0.3)"; g.lineWidth = 1; g.beginPath(); g.moveTo(L, B); g.lineTo(Rr, B); g.stroke();
    g.setLineDash([3, 3]); g.strokeStyle = "rgba(255,255,255,0.6)"; g.beginPath(); g.moveTo(x(0), T - 2); g.lineTo(x(0), B + 3); g.stroke(); g.setLineDash([]);
    const cs = D.steps[Math.min(FAIL.step, D.S - 1) * 7]; g.strokeStyle = "#fff"; g.lineWidth = 1.5; g.beginPath(); g.moveTo(x(cs), T - 4); g.lineTo(x(cs), B + 5); g.stroke();
    g.font = "10px " + MFONT; g.fillStyle = "#aab4c2"; g.fillText(top.toLocaleString(), 2, T + 8); g.fillText("0", 18, B); g.fillText("onset", x(0) + 3, T + 8);
    const lbl = t1 >= 10 ? Math.round(t1) + " h" : t1.toFixed(1) + " h"; g.fillText(lbl, Rr - g.measureText(lbl).width, B + 13); g.fillText((t0 >= -10 ? t0.toFixed(1) : Math.round(t0)) + " h", L - 4, B + 13);
    let xx = 4, yy = H2 - 16; for (const [col, t] of [["rgba(105,210,255,0.8)", "● mainland"], ["rgba(255,179,71,0.9)", "◆ cut off"], ["rgba(255,90,78,0.9)", "✕ down by the event"]]){ const tw = 14 + g.measureText(t).width; if (xx + tw > W2){ xx = 4; yy += 12; } g.fillStyle = col; g.fillRect(xx, yy - 7, 8, 8); g.fillStyle = "#aab4c2"; g.fillText(t, xx + 11, yy); xx += tw; }
  },
  status(){ const l = this.last; if (!l.steps) return "–"; const h = l.hour_from_onset < 0 ? "before onset" : l.hour_from_onset.toFixed(1) + " h after onset (lasts " + l.duration_h + " h)";
    return `${FAIL_SC[FAIL.sc]}, ${h}: ${fmt(l.up)} of ${fmt(l.nodes)} nodes up, ${fmt(l.mainland)} on the mainland, ${fmt(l.cut_off)} cut off in ${l.islands} island${l.islands === 1 ? "" : "s"}, ${fmt(l.down_by_event)} down because of it · a generated world of ${l.countries} countries, none a real one; MODEL, one seeded run`; },
  controls: [{ range: [0, 42, 1], label: "Hour", value: () => Math.max(0, FAIL.step), set(v){ this.setStep(v); }, show: (v) => { const D = FAIL.d; if (!D) return "–"; const t = D.steps[Math.min(v, D.S - 1) * 7]; return (t < 0 ? "" : "+") + t.toFixed(1) + " h"; } },
    { label: "Play", act(){ if (FAIL.play){ clearInterval(FAIL.play); FAIL.play = null; return; } if (FAIL.step >= FAIL.d.S - 1) this.setStep(0); FAIL.play = setInterval(() => { if (!LAYERS.on("failures") || FAIL.step >= FAIL.d.S - 1){ clearInterval(FAIL.play); FAIL.play = null; return; } this.setStep(FAIL.step + 1); }, 320); } },
    { label: "Next scenario", act(){ FAIL.sc = (FAIL.sc + 1) % 3; FAIL.step = -1; FAIL.key = ""; this.compute(); gfxDirty(800); } },
    { label: "Zoom to the outage", act(){ const D = FAIL.d; if (!D) return; const w = cv.clientWidth, h = cv.clientHeight, z = EARTH_M / (0.86 * 6000 * Math.min(w, h) / 2);
      velYaw = 0; velPitch = 0; yaw = D.focus[1] * DEG; pitch = D.focus[0] * DEG; targetZoom = z; zoomAnchor = null; lastTouchTime = performance.now(); gfxDirty(1500); } }] });
// ------------------------------------------------------------------ a room call, second by second (twin-media: media_view.rs)
const MED = { seed: 1, n: 0, steps: 0, ec: 0, mem: null, tree: null, edges: null, rx: null, ev: null, step: 0, play: null, key: "" };
const MED_ACCESS = ["fibre home, wired", "cable home, laptop on Wi-Fi", "DSL home", "phone on home Wi-Fi", "phone on LTE"];
const MED_EV = ["parent left", "audio resumed", "video resumed", "not resumed by the session's end"];
window.CEWP_LAYERS["media"] = layer({ id: "media", title: "A room call: one tree per publisher, a parent leaves, the tree repairs", cls: "MODEL", inputs: "MODEL (access profiles, loss, media parameters; generated people)",
  source: "view_media(1) → slots 20 members, 21 tree, 22 edges per step, 23 rx per step, 24 events, 25 dims (cewp_sim::media_view::media_view, run once per page)",
  contract: "twin-media's churn room with household traffic off: four homes and six phones join and leave over 60 one-second steps; two phones behind hard NAT take every stream through a home relay, and that relay leaves with a signed end at 30 s. Members f32 × 4 (lat, lon, access 0 fibre home, 1 cable laptop Wi-Fi, 2 DSL home, 3 phone home Wi-Fi, 4 phone LTE, nominal uplink B/s), MODEL: places are a seeded scatter of about a degree around one of two metro anchors (this scenario puts every member at the first, 45.5, -73.6), never an address. Tree u32 × 3 (stream, parent, child), one tree per publishing member. Edges per step f32 × 4 aligned to the tree (rate B/s, loss ‰, queue ms, layer 0 detached, 1 audio + base, 2 full). Rx per step f32 × 3 at stream·n + member (layer -1 none, 0 audio only, 1 base, 2 both; stall; glass-to-glass ms). Events f32 × 5 (t ms, kind, stream, child, departed parent). The view's own field codes call these DERIVED (computed by the engine); the engine is stochastic (Gilbert-Elliott loss, a Markov cellular chain, seeded churn), so on the page every result is MODEL output, and the sums, counts and medians the page takes over it stay MODEL. It is one seeded run: a median is over that run's sessions, not a rate. The tree is edge v45's closed-loop controller as twin-media models it, not alm.rs and not a measured call. The page sums streams per (parent, child) pair to draw one line per pair; every other figure is read as given.",
  available: has("view_media", "view_ptr"),
  compute(){
    if (!MED.mem){ MED.n = wasm.view_media(MED.seed); const D = VW(25); MED.steps = D[1]; MED.ec = D[2]; MED.mem = VW(20); MED.tree = VW(21); MED.edges = VW(22); MED.rx = VW(23); MED.ev = VW(24); }
    this.tally();
  },
  // per-step figures, all read from the slices (sums and counts only)
  tally(){
    const n = MED.n, st = Math.min(MED.step, MED.steps - 1), E = MED.edges, R = MED.rx, o = st * MED.ec * 4, ro = st * n * n * 3;
    let attached = 0, full = 0, bytes = 0, lossy = 0; for (let e = 0; e < MED.ec; e++){ const k = o + e * 4; if (E[k + 3] > 0){ attached++; if (E[k + 3] === 2) full++; bytes += E[k]; if (E[k + 1] > 0) lossy++; } }
    let sess = 0, stalls = 0, audioOnly = 0, none = 0, dmax = 0; for (let i = 0; i < n * n; i++){ const k = ro + i * 3; if (R[k + 2] < 0 && R[k] < 0) continue; sess++; if (R[k + 1] > 0) stalls++; if (R[k] === 0) audioOnly++; if (R[k] < 0) none++; dmax = Math.max(dmax, R[k + 2]); }
    // per departure three rows: the leave, then audio and video each resumed or not by the session's end (the
    // row's time is that end). A not-resumed row before the run's last second is a receiver that left the call.
    const ev = MED.ev, left = [], gaps = { 1: [], 2: [] }, endMs = MED.steps * 1000; let notRes = 0, notResLeft = 0;
    for (let i = 0; i < ev.length; i += 15){ left.push(ev[i]); for (const j of [5, 10]){ const k = ev[i + j + 1]; if (k === 3){ notRes++; if (ev[i + j] < endMs - 1000) notResLeft++; } else gaps[k].push(ev[i + j] - ev[i]); } }
    const med = (a) => { if (!a.length) return -1; const b = a.slice().sort((x, y) => x - y); return Math.round(b[(b.length - 1) >> 1]); };
    const slow = gaps[1].slice().sort((x, y) => y - x);
    this.last = { members: n, steps: MED.steps, tree_edges: MED.ec, step: st, edges_attached: attached, edges_full: full, uplink_bytes_per_s: Math.round(bytes), edges_with_loss: lossy, sessions: sess, stalled: stalls, audio_only: audioOnly, nothing_played: none, worst_glass_to_glass_ms: Math.round(dmax), departures: left.length, audio_resumes: gaps[1].length, audio_resume_median_ms: med(gaps[1]), audio_resume_slowest_ms: slow.length ? Math.round(slow[0]) : -1, audio_resume_second_slowest_ms: slow.length > 1 ? Math.round(slow[1]) : -1, video_resume_median_ms: med(gaps[2]), not_resumed: notRes, not_resumed_receiver_left: notResLeft, not_resumed_in_call: notRes - notResLeft };
    const acc = [0, 0, 0, 0, 0]; for (let i = 0; i < n; i++) acc[MED.mem[i * 4 + 2]]++;
    this.info = { access: MED_ACCESS.map((a, i) => a + " " + acc[i]).filter((t) => !/ 0$/.test(t)).join(", "), places: "a seeded scatter of about a degree around one metro anchor, not addresses", tree: "edge v45 closed-loop controller as twin-media models it, not a measured call", run: "one seeded run of the churn scenario (seed 1); medians are over this run's sessions, not rates", events: MED_EV.join(", ") };
  },
  onShow(){ later(this); },
  setStep(v){ MED.step = Math.max(0, Math.min(MED.steps ? MED.steps - 1 : 59, v)); if (MED.mem) this.tally(); MED.key = ""; const r = document.querySelector('input[data-ctl="media:0"]'); if (r && +r.value !== MED.step){ r.value = MED.step; const o = r.parentElement.querySelector("output"); if (o) o.textContent = this.controls[0].show(MED.step); } refresh(); gfxDirty(700); },
  draw2D(g, F){
    if (!MED.mem) return; const n = MED.n, M = MED.mem, T = MED.tree, E = MED.edges, o = Math.min(MED.step, MED.steps - 1) * MED.ec * 4;
    // one line per (parent, child) pair: streams summed, the best layer drawn, any loss marked
    const pair = new Map(); for (let e = 0; e < MED.ec; e++){ const k = o + e * 4; if (E[k + 3] <= 0) continue; const key = T[e * 3 + 1] * 64 + T[e * 3 + 2]; const q = pair.get(key) || { p: T[e * 3 + 1], c: T[e * 3 + 2], rate: 0, layer: 0, loss: 0 }; q.rate += E[k]; q.layer = Math.max(q.layer, E[k + 3]); q.loss = Math.max(q.loss, E[k + 1]); pair.set(key, q); }
    for (const q of pair.values()){ const a = [M[q.p * 4], M[q.p * 4 + 1]], b = [M[q.c * 4], M[q.c * 4 + 1]];
      const ll = gcLL(a[0], a[1], b[0], b[1]); g.lineWidth = Math.max(1, Math.min(5, Math.sqrt(q.rate / 4000))); g.strokeStyle = q.layer === 2 ? "rgba(127,208,255,0.85)" : "rgba(255,207,122,0.85)";
      g.setLineDash(q.layer === 2 ? [] : [5, 3]); g.beginPath(); strokeLL(g, F, ll); g.stroke(); g.setLineDash([]);
      if (q.loss > 0){ const h = ll.length > 4 ? (ll.length >> 2) * 2 : -1, pa = proj(F, a[0], a[1]), pb = proj(F, b[0], b[1]), m = h >= 0 ? proj(F, ll[h], ll[h + 1]) : [(pa[0] + pb[0]) / 2, (pa[1] + pb[1]) / 2, Math.min(pa[2], pb[2])]; if (m[2] > 0.02){ g.strokeStyle = "#ff8a7a"; g.lineWidth = 1.6; g.beginPath(); g.moveTo(m[0] - 3, m[1] - 3); g.lineTo(m[0] + 3, m[1] + 3); g.moveTo(m[0] + 3, m[1] - 3); g.lineTo(m[0] - 3, m[1] + 3); g.stroke(); } } }
    // members: square = home, circle = phone; filled while in the call, hollow when out; a ring where a parent left in this second
    { let la0 = 90, la1 = -90; for (let i = 0; i < n; i++){ la0 = Math.min(la0, M[i * 4]); la1 = Math.max(la1, M[i * 4]); } const lo = M[1], c0 = proj(F, la0, lo), c1 = proj(F, la1, lo);
      if (c0[2] > 0.02 && Math.hypot(c1[0] - c0[0], c1[1] - c0[1]) < 24){ const l = this.last; g.lineWidth = 1.5; g.strokeStyle = "#fff"; g.setLineDash([3, 2]); g.strokeRect(c0[0] - 8, c1[1] - 8, 16, c0[1] - c1[1] + 16); g.setLineDash([]);
        const t = `room call: ${n} generated members, second ${l.step} (zoom in or press Zoom to the call)`; g.font = "600 11px " + MFONT; g.lineWidth = 3; g.strokeStyle = "rgba(6,9,14,0.9)"; g.strokeText(t, c0[0] + 12, c1[1] - 10); g.fillStyle = "#e8eef6"; g.fillText(t, c0[0] + 12, c1[1] - 10); return; } }
    const st = Math.min(MED.step, MED.steps - 1), R = MED.rx, ro = st * n * n * 3, inCall = new Array(n).fill(false);
    for (let s = 0; s < n; s++) for (let m = 0; m < n; m++){ const k = ro + (s * n + m) * 3; if (!(R[k + 2] < 0 && R[k] < 0)){ inCall[s] = true; inCall[m] = true; } }
    const leftNow = new Set(); for (let i = 0; i < MED.ev.length; i += 5) if (MED.ev[i + 1] === 0 && Math.floor(MED.ev[i] / 1000) === st) leftNow.add(MED.ev[i + 4]);
    for (let i = 0; i < n; i++){ const p = proj(F, M[i * 4], M[i * 4 + 1]); if (p[2] < 0.02) continue; const phone = M[i * 4 + 2] >= 3;
      g.lineWidth = 1.6; g.strokeStyle = inCall[i] ? "rgba(6,9,14,0.9)" : "#c8d2e1"; g.fillStyle = inCall[i] ? (phone ? "#e8eef6" : "#7cf0b0") : "rgba(0,0,0,0)";
      g.beginPath(); if (phone) g.arc(p[0], p[1], 4.5, 0, 6.2832); else g.rect(p[0] - 4.5, p[1] - 4.5, 9, 9); g.fill(); g.stroke();
      if (leftNow.has(i)){ g.strokeStyle = "#ff8a7a"; g.lineWidth = 1.6; g.beginPath(); g.arc(p[0], p[1], 10, 0, 6.2832); g.stroke(); } }
  },
  panel(c){
    if (this.busy || !MED.mem) return; const dpr = window.devicePixelRatio || 1, W2 = c.clientWidth || 260, H2 = 146;
    if (c.style.height !== H2 + "px") c.style.height = H2 + "px"; if (c.width !== Math.round(W2 * dpr)){ c.width = Math.round(W2 * dpr); c.height = H2 * dpr; MED.key = ""; }
    if (MED.key === String(MED.step)) return; MED.key = String(MED.step);
    const g = c.getContext("2d"); g.setTransform(dpr, 0, 0, dpr, 0, 0); g.clearRect(0, 0, W2, H2);
    const n = MED.n, S = MED.steps, R = MED.rx, L = 30, Rr = W2 - 8, x = (s) => L + s / Math.max(1, S - 1) * (Rr - L);
    // rows: one per member as receiver; each second coloured by the lowest layer it was shown across the streams it receives, a dark bar where any of them showed no video frame
    const T = 6, rowH = Math.min(8, (H2 - 40) / n);
    for (let m = 0; m < n; m++){ const y = T + m * rowH; for (let s = 0; s < S; s++){ let worst = 9, any = false, stall = false; for (let p = 0; p < n; p++){ const k = (s * n * n + p * n + m) * 3; if (R[k + 2] < 0 && R[k] < 0) continue; any = true; if (R[k + 1] > 0) stall = true; worst = Math.min(worst, R[k]); }
        if (!any) continue; g.fillStyle = worst >= 2 ? "rgba(127,208,255,0.75)" : worst === 1 ? "rgba(127,208,255,0.4)" : worst === 0 ? "rgba(255,207,122,0.7)" : "rgba(255,138,122,0.85)";
        const w = (Rr - L) / S; g.fillRect(x(s) - w / 2, y, w + 0.3, rowH - 1.5); if (stall){ g.fillStyle = "rgba(6,9,14,0.9)"; g.fillRect(x(s) - w / 2, y + rowH / 2 - 1, w + 0.3, 1.5); } } }
    g.font = "10px " + MFONT; g.fillStyle = "#aab4c2"; g.fillText("each", 2, T + 8); g.fillText("rcvr", 2, T + 18);
    const yb = T + n * rowH + 2;
    // departures as ticks under the rows; the cursor at this second
    for (let i = 0; i < MED.ev.length; i += 5) if (MED.ev[i + 1] === 0){ const xx = x(MED.ev[i] / 1000); g.strokeStyle = "#ff8a7a"; g.lineWidth = 1.4; g.beginPath(); g.moveTo(xx, yb); g.lineTo(xx, yb + 6); g.stroke(); }
    g.strokeStyle = "#fff"; g.lineWidth = 1.2; g.beginPath(); g.moveTo(x(MED.step), T - 3); g.lineTo(x(MED.step), yb + 8); g.stroke();
    g.fillStyle = "#aab4c2"; g.fillText("0 s", L - 6, yb + 18); g.fillText(S + " s", Rr - 22, yb + 18); g.fillStyle = "#ff8a7a"; g.fillText("| a parent left", L + 40, yb + 18);
    { let xx = 4, yy = H2 - 16; for (const [col, t] of [["rgba(127,208,255,0.75)", "both layers"], ["rgba(127,208,255,0.4)", "base"], ["rgba(255,207,122,0.7)", "audio"], ["rgba(255,138,122,0.85)", "nothing"], ["rgba(6,9,14,0.9)", "▬ no video frame"]]){ const tw = 15 + g.measureText(t).width; if (xx + tw > W2){ xx = 4; yy += 12; } g.fillStyle = col; g.fillRect(xx, yy - 7, 8, 8); g.fillStyle = "#aab4c2"; g.fillText(t, xx + 11, yy); xx += tw; } }
  },
  status(){ const l = this.last; if (this.busy) return "running the room call once (60 one-second steps)…"; return l.steps ? `second ${l.step} of ${l.steps}: ${l.edges_attached} tree legs attached (${l.edges_full} full), ${(l.uplink_bytes_per_s * 8 / 1e6).toFixed(1)} Mbit/s on the wire, ${l.stalled} of ${l.sessions} sessions showed no video frame this second (${l.audio_only} kept audio) · whole call (MODEL): ${l.departures} departures; audio back in ${l.audio_resume_median_ms < 0 ? "–" : fmt(l.audio_resume_median_ms) + " ms"} (median of ${l.audio_resumes}), the slowest ${l.audio_resume_slowest_ms < 0 ? "–" : fmt(l.audio_resume_slowest_ms) + " ms"}${l.audio_resume_second_slowest_ms >= 0 ? " and " + fmt(l.audio_resume_second_slowest_ms) + " ms" : ""}; video back in ${l.video_resume_median_ms < 0 ? "–" : fmt(l.video_resume_median_ms) + " ms"} (median); video never resumed in ${l.not_resumed} session${l.not_resumed === 1 ? "" : "s"}: ${l.not_resumed_receiver_left} because the receiver left the call first, ${l.not_resumed_in_call} a real case (no video until the run ended) · MODEL: one seeded run, figures over its sessions, not rates; generated people around one metro anchor, never an address` : "–"; },
  controls: [{ range: [0, 59, 1], label: "Second of the call", value: () => MED.step, set(v){ this.setStep(v); }, show: (v) => v + " s" },
    { label: "Play the call", act(){ if (MED.play){ clearInterval(MED.play); MED.play = null; return; } if (MED.step >= MED.steps - 1) this.setStep(0); MED.play = setInterval(() => { if (!LAYERS.on("media") || MED.step >= MED.steps - 1){ clearInterval(MED.play); MED.play = null; return; } this.setStep(MED.step + 1); }, 400); } },
    { label: "Zoom to the call", act(){ if (!MED.mem) return; let la = 0, lo = 0; for (let i = 0; i < MED.n; i++){ la += MED.mem[i * 4]; lo += MED.mem[i * 4 + 1]; } la /= MED.n; lo /= MED.n;
      const w = cv.clientWidth, h = cv.clientHeight, z = EARTH_M / (0.86 * 700 * Math.min(w, h) / 2); velYaw = 0; velPitch = 0; yaw = lo * DEG; pitch = la * DEG; targetZoom = z; zoomAnchor = null; lastTouchTime = performance.now(); gfxDirty(1500); } },
    { label: "Jump to the relay leaving", act(){ let t = -1; for (let i = 0; i < MED.ev.length; i += 5) if (MED.ev[i + 1] === 0){ t = MED.ev[i]; break; } if (t >= 0) this.setStep(Math.floor(t / 1000)); } }] });

// ------------------------------------------------------------------ seat knee with its interval (twin-calibration: seat_hooks.rs)
const CAL = { sc: 4, curve: new Map(), knee: new Map(), res: null, key: "" };
const CAL_GROWTH = [0, 0.25, 0.5, 0.75, 1];
const CAL_COUNTER = ["window B CPU", "window A composer", "window A attestation queries", "window B attestation queries", "window B composer", "window B permits", "window A' permits", "window C permits", "window C attestation queries", "busy-hour CPU"];
const CAL_BIND = (c) => c === 0 ? "CPU" : c === 1 ? "sweep permits" : c >= 2 ? (["Key", "occurrence", "Revocation", "occurrence-withdrawal"][c - 2] || "a key-plane") + " plane lock" : "nothing";
const CAL_CLS = ["MEASURED", "DERIVED", "MODEL", "PROJECTED", "FRONTIER"];
const CAL_CEIL = 0.70;
window.CEWP_LAYERS["calibration"] = layer({ id: "calibration", title: "Seat load and its knee, as an interval", cls: "MODEL", inputs: "FRONTIER (g, key_publish_set growth); fit to seat-1 counters, MEASURED",
  source: "view_calibration_curve(g) → slot 17, view_calibration_knee(g) → slot 18, view_calibration_residuals() → slot 19 (cewp_sim::seat_hooks calibration_view_*)",
  contract: "twin-calibration's seat views for one v45.0.0 seat. Scenario 0..4 picks key_publish_set growth g = 0, 0.25, 0.5, 0.75, 1 (FRONTIER: how fast the published key set grows with announced nodes is not measured). curve(g) f32 × 6, 24 rows (devices 500 to 100,000, ρ central, lower, upper, class, binding code: 0 CPU, 1 sweep permits, 2+ a key-plane lock); the band is outer bounds per term over the fit's input intervals, not a probability. knee(g) 7 fields (lower, central, upper, interval kind 0 outer bounds, binds at lower, binds at upper, class), -1 = no claim; the knee is where the seat crosses ρ 0.70 on any term. Drawn only as its interval with g named, never the central value alone, never with a confidence level. residuals f32 × 4 (counter id, observed, predicted, out of sample 0/1): predicted over observed per counter, in-sample counters marked apart from held-out ones. Class 2, MODEL with its interval (past the measured fleet it is an extrapolation of one seat's model, not PROJECTED in FSD §8.2's sense).",
  available: has("view_calibration_curve", "view_calibration_knee", "view_calibration_residuals", "view_ptr"),
  compute(){
    const sc = CAL.sc;
    if (!CAL.curve.has(sc)){ wasm.view_calibration_curve(sc); CAL.curve.set(sc, VW(17)); }
    if (!CAL.knee.has(sc)){ wasm.view_calibration_knee(sc); CAL.knee.set(sc, VW(18)); }
    if (!CAL.res){ wasm.view_calibration_residuals(); CAL.res = VW(19); }
    const C = CAL.curve.get(sc), K = CAL.knee.get(sc), R = CAL.res;
    let inN = 0, outN = 0, worst = 0, worstId = -1, worstR = NaN;
    for (let i = 0; i < R.length; i += 4){ if (R[i + 3]) outN++; else inN++; const r = R[i + 2] / R[i + 1]; const e = Math.abs(Math.log(r)); if (R[i + 3] && isFinite(e) && e > worst){ worst = e; worstId = R[i]; worstR = r; } }
    this.last = { growth_g: CAL_GROWTH[sc], curve_rows: C.length / 6, knee_lower: K[0] < 0 ? -1 : Math.round(K[0]), knee_upper: K[2] < 0 ? -1 : Math.round(K[2]), binds_lower: K[4], binds_upper: K[5], class: K[6], ceiling: CAL_CEIL, counters_in_sample: inN, counters_held_out: outN, worst_held_out_pred_over_obs: worstId >= 0 ? +worstR.toFixed(3) : -1, worst_held_out_counter: worstId };
    this.info = { knee: "an interval only (outer bounds over the fit's input intervals, not a confidence level); the central value is not shown", growth: "g " + CAL_GROWTH[sc] + " (FRONTIER)", binds: CAL_BIND(K[4]) + (K[4] !== K[5] ? " at the low end, " + CAL_BIND(K[5]) + " at the high end" : ""), class: CAL_CLS[K[6]] || "?", counters: CAL_COUNTER.map((c, i) => i + " " + c).join(", ") };
  },
  onShow(){ later(this); },
  panel(c){
    if (this.busy || !CAL.curve.has(CAL.sc)) return; const C = CAL.curve.get(CAL.sc), K = CAL.knee.get(CAL.sc), R = CAL.res;
    const dpr = window.devicePixelRatio || 1, W2 = c.clientWidth || 260, H2 = 196; if (c.style.height !== H2 + "px") c.style.height = H2 + "px"; if (c.width !== Math.round(W2 * dpr)){ c.width = Math.round(W2 * dpr); c.height = H2 * dpr; CAL.key = ""; }
    if (CAL.key === String(CAL.sc)) return; CAL.key = String(CAL.sc);
    const g = c.getContext("2d"); g.setTransform(dpr, 0, 0, dpr, 0, 0); g.clearRect(0, 0, W2, H2);
    // the band: ρ against devices, log axis 500 .. 100,000
    const L = 34, T = 8, B = 108, Rr = W2 - 10, d0 = Math.log10(500), d1 = 5;
    const lx = (d) => L + (Math.log10(d) - d0) / (d1 - d0) * (Rr - L), RMAX = 2, ly = (r) => B - Math.min(RMAX, Math.max(0, r)) / RMAX * (B - T);
    g.font = "10px " + MFONT; g.fillStyle = "#aab4c2"; g.strokeStyle = "rgba(200,210,225,0.22)"; g.lineWidth = 1;
    for (const d of [1000, 10000, 100000]){ g.beginPath(); g.moveTo(lx(d), T); g.lineTo(lx(d), B); g.stroke(); const t = d / 1000 + "k"; g.fillText(t, Math.min(lx(d) - 8, Rr - g.measureText(t).width), B + 11); }
    g.fillText("500", L - 4, B + 11); g.fillText("ρ 0", 4, B); g.fillText("ρ 2", 4, T + 8);
    // the knee interval as a bracket on the ceiling line (no central tick)
    const klo = K[0] >= 0 ? lx(Math.max(500, K[0])) : L, khi = K[2] >= 0 ? lx(Math.min(100000, K[2])) : Rr;
    g.fillStyle = "rgba(255,207,122,0.10)"; g.fillRect(klo, T, khi - klo, B - T);
    g.strokeStyle = "#ffcf7a"; g.lineWidth = 1.4; g.beginPath(); g.moveTo(klo, B + 2); g.lineTo(klo, T); g.moveTo(khi, B + 2); g.lineTo(khi, T); g.stroke();
    g.setLineDash([2, 3]); g.strokeStyle = "rgba(255,255,255,0.55)"; g.lineWidth = 1; g.beginPath(); g.moveTo(L, ly(CAL_CEIL)); g.lineTo(Rr, ly(CAL_CEIL)); g.stroke(); g.setLineDash([]);
    g.fillStyle = "#aab4c2"; g.fillText("ρ 0.70", 4, ly(CAL_CEIL) + 3);
    // outer band, hatched by its edges; central dotted (a model line, not a point estimate of the knee)
    g.fillStyle = "rgba(127,208,255,0.22)"; g.beginPath();
    for (let i = 0; i < C.length; i += 6){ const x = lx(C[i]), y = ly(C[i + 3]); i ? g.lineTo(x, y) : g.moveTo(x, y); }
    for (let i = C.length - 6; i >= 0; i -= 6) g.lineTo(lx(C[i]), ly(C[i + 2]));
    g.closePath(); g.fill();
    g.strokeStyle = "#7fd0ff"; g.lineWidth = 1; for (const off of [2, 3]){ g.beginPath(); for (let i = 0; i < C.length; i += 6){ const x = lx(C[i]), y = ly(C[i + off]); i ? g.lineTo(x, y) : g.moveTo(x, y); } g.stroke(); }
    g.setLineDash([1, 2]); g.lineWidth = 1.6; g.beginPath(); for (let i = 0; i < C.length; i += 6){ const x = lx(C[i]), y = ly(C[i + 1]); i ? g.lineTo(x, y) : g.moveTo(x, y); } g.stroke(); g.setLineDash([]);
    const kt = (K[0] < 0 ? "no claim" : Math.round(K[0]).toLocaleString()) + " to " + (K[2] < 0 ? "no claim" : Math.round(K[2]).toLocaleString()) + " devices";
    g.font = "600 10px " + MFONT; g.lineWidth = 3; g.strokeStyle = "rgba(6,9,14,0.9)"; const tx = Math.max(L + 2, Math.min(klo + 3, Rr - g.measureText(kt).width - 2));
    g.strokeText(kt, tx, T + 10); g.fillStyle = "#ffcf7a"; g.fillText(kt, tx, T + 10);
    // residuals: predicted / observed per counter on a log strip 0.5 .. 2; in-sample filled square, held out open ring
    const RY = 146, rx = (r) => L + (Math.log2(Math.min(4, Math.max(0.25, r))) + 2) / 4 * (Rr - L);
    g.strokeStyle = "rgba(200,210,225,0.3)"; g.lineWidth = 1; g.beginPath(); g.moveTo(L, RY); g.lineTo(Rr, RY); for (const t of [0.5, 1, 2]){ g.moveTo(rx(t), RY - (t === 1 ? 9 : 4)); g.lineTo(rx(t), RY + (t === 1 ? 9 : 4)); } g.stroke();
    g.font = "10px " + MFONT; g.fillStyle = "#aab4c2"; g.fillText("×¼", L - 4, RY + 19); g.fillText("×1", rx(1) - 6, RY + 19); g.fillText("×4", Rr - 14, RY + 19); g.fillText("predicted / observed, per seat-1 counter", L - 4, RY - 13);
    for (let i = 0, j = 0; i < R.length; i += 4, j++){ const r = R[i + 2] / R[i + 1], x = rx(r), y = RY + ((j % 3) - 1) * 4;
      if (r > 4 || r < 0.25){ const e = r > 4 ? 1 : -1; g.fillStyle = R[i + 3] ? "#ff9d7a" : "#e8eef6"; g.beginPath(); g.moveTo(x + 6 * e, RY); g.lineTo(x, RY - 4); g.lineTo(x, RY + 4); g.closePath(); g.fill(); // off the strip: an arrow at the edge with its value, never clamped silently
        const t = "×" + (r >= 10 ? r.toFixed(0) : r.toFixed(2)); g.fillText(t, r > 4 ? x - g.measureText(t).width - 2 : x + 4, RY - 7 - ((j % 2) * 9)); continue; }
      if (R[i + 3]){ g.strokeStyle = "#ff9d7a"; g.lineWidth = 1.4; g.beginPath(); g.arc(x, y, 3.2, 0, 6.2832); g.stroke(); } else { g.fillStyle = "#e8eef6"; g.fillRect(x - 2.5, y - 2.5, 5, 5); } }
    g.fillStyle = "#aab4c2"; g.fillText("■ the fit read it   ○ held out", 4, H2 - 18);
    g.fillText("g " + CAL_GROWTH[CAL.sc] + " (FRONTIER) · band = outer bounds", 4, H2 - 5);
  },
  status(){ const l = this.last; if (this.busy) return "running the seat model's input intervals once…"; return l.curve_rows ? `knee at g ${l.growth_g} (FRONTIER): ${l.knee_lower < 0 ? "no claim" : fmt(l.knee_lower)} to ${l.knee_upper < 0 ? "no claim" : fmt(l.knee_upper)} devices, ${this.info.binds} · MODEL, interval · ${l.counters_held_out} held-out counters, furthest: ${CAL_COUNTER[l.worst_held_out_counter] || "?"} predicted at ×${l.worst_held_out_pred_over_obs} of observed` : "–"; },
  controls: [{ range: [0, 4, 1], label: "Key set growth g", value: () => CAL.sc, set(v){ if (v === CAL.sc) return; CAL.sc = v; CAL.key = ""; this.ran = false; later(this); }, show: (v) => CAL_GROWTH[v] + " (FRONTIER)" }] });

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
    window.addEventListener("keydown", (e) => { if (e.key === "l" && !/INPUT|TEXTAREA|SELECT/.test((e.target || {}).tagName || "") && !e.metaKey && !e.ctrlKey){ open = !open; render(); } });
    // phone: any gesture on the map collapses the sheet to its button, so the globe is never held under it
    const fold = () => { if (open && window.innerWidth <= 800){ open = false; render(); } };
    for (const ev of ["pointerdown", "touchstart", "wheel"]) (typeof cv !== "undefined" && cv ? cv : window).addEventListener(ev, fold, { passive: true }); }
  document.body.classList.toggle("ly-open", open);
  const shown = REG.filter((d) => d.available() && !d.pending);
  const head = `<button class="ly-toggle" data-toggle="1" aria-expanded="${open}" aria-controls="ly-body">Layers <span class="ly-n">${shown.filter((d) => KEEP.on.has(d.id)).length}/${shown.length}</span></button>`;
  if (!open){ dock.innerHTML = head; dock.classList.remove("open"); return; }
  dock.classList.add("open");
  dock.innerHTML = head + `<div id="ly-body" class="ly-body"><button class="ly-handle" data-toggle="1" aria-label="Close the layers">Layers · tap or touch the map to close</button>` + shown.map((d) => {
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
