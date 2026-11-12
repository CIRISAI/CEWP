// Buildings and dwellings per building by street class, from the twin's own layout code
// (tiling.rs LeafLayout::build through the page's kernel, holon_layout): docs/geo/building_hist.json.
// Rule and file:line: FSD/twin/hhsize.md, "Dwellings per building".
//
// Two sets of leaves, under separate keys:
// * synthetic: for each class of docs/geo/classes.json a ladder of six people-per-km2 rungs across
//   its range; each rung is a uniform world (every level-0 holon holds round(rung / 2.5 x its area)
//   households, every split apportions a parent's households over its children by cos(latitude of
//   the child's centre) x 1e6, rounded: area weights, through the kernel's holon_split_exact), walked
//   by the page's tree (HOLON.tree, leaf rule L3+ and <= 60 households, or L12); the first 400 leaves
//   in tree order of the L3 tiles HOLON.make(3, i0, j0 + k), k = 0, 1, 2, ..., where (i0, j0) is the
//   L3 tile at 51.0 N 10.0 E. A leaf's id is its seed, as on the page.
// * real: every leaf of the L3 street tile at sixteen places (the twelve probe places of
//   household_hist_real.js and four in Germany), through the page's path as there (GHS-POP assets,
//   geoPopFine, pop_load / pop_root / pop_split / pop_of, HOLON.tree, the leaf's context).
// Per leaf: holon_layout's meta (w, h, households, class, people per km2) and buildings
// (cx cy w d angle floors units street). Leaf area is w x h (tiling.rs LeafLayout::build: the
// holon's size in metres at its centre latitude), the area the class density is defined on.
//
//   node docs/geo/building_hist.js            print the JSON
//   node docs/geo/building_hist.js --write    write docs/geo/building_hist.json
//   node docs/geo/building_hist.js --check    regenerate and compare (exit 1 when stale)
//   node docs/geo/building_hist.js --sample F every leaf's (set, id, context, buildings) to F for
//        cargo run --release -p cewp-sim --example household_hist -- --crosscheck-layout F
"use strict";
const fs = require("node:fs"), path = require("node:path"), zlib = require("node:zlib"), crypto = require("node:crypto");
const HOLON = require("../holon.js");
const G = require("./geodata.js");

const here = (p) => path.join(__dirname, p);
const OUT = here("building_hist.json");
const COMMAND = "node docs/geo/building_hist.js --write";
const POP_GRID_PEOPLE = 7830639988; // docs/globe.js
const TARGET = 60; // tiling.js TL_TARGET
const PPH = 2.5;
const LEAVES_PER_RUNG = 400;
const SYN_AT = [51.0, 10.0];
const LADDER = { // people per km2 (classes.json lower bounds 0, 250, 1500, 6000, 12000)
  "village": [5, 15, 40, 80, 150, 230],
  "suburban streets": [270, 400, 600, 850, 1150, 1450],
  "terraced streets": [1600, 2200, 3000, 4000, 5000, 5900],
  "apartment blocks": [6100, 7000, 8000, 9500, 10500, 11800],
  "dense fabric": [12500, 16000, 22000, 30000, 45000, 70000],
};
const PLACES = [["Zurich", 47.3769, 8.5417], ["Cairo", 30.0444, 31.2357], ["Lagos", 6.5244, 3.3792],
  ["US suburb (Maryland)", 39.2, -77.25], ["rural French village (Cher)", 46.3, 2.3],
  ["Dhaka", 23.7104, 90.4074], ["Mumbai", 19.043, 72.856], ["Karachi", 24.8607, 67.0011],
  ["Kowloon", 22.3193, 114.1694], ["Manhattan", 40.7831, -73.9712], ["Tokyo", 35.6895, 139.6917], ["Paris", 48.8566, 2.3522],
  ["Berlin centre", 52.52, 13.405], ["Munich suburb (Unterhaching)", 48.066, 11.615],
  ["Ruhr terraced district (Gelsenkirchen)", 51.5177, 7.0857], ["Mecklenburg village (Krakow am See)", 53.65, 12.27]];
const CLASSES = JSON.parse(fs.readFileSync(here("classes.json"), "utf8")).classes;
const BANDS = [["1", 1, 1], ["2", 2, 2], ["3-6", 3, 6], ["7-12", 7, 12], ["13+", 13, Infinity]];
const FOOT_BIN = 10; // m2
const sha = (f) => crypto.createHash("sha256").update(fs.readFileSync(f)).digest("hex");

function kernel(){
  G.GEODATA.pop[0] = new Float32Array(zlib.gunzipSync(fs.readFileSync(here("gd/pop0.bin.gz"))).buffer.slice(0));
  G.GEODATA.pop1code = new Uint8Array(zlib.gunzipSync(fs.readFileSync(here("gd/pop1.bin.gz"))));
  G.geoPopFine();
  const P = G.GEODATA.pop[1];
  const w = new WebAssembly.Instance(new WebAssembly.Module(fs.readFileSync(here("../kernel/kernel_scale.wasm"))), {}).exports;
  const ptr = w.alloc(P.length * 4) >>> 0;
  new Float32Array(w.memory.buffer, ptr, P.length).set(P);
  w.pop_load(ptr, P.length, POP_GRID_PEOPLE);
  if (w.pop_households) w.pop_households(0); // the legacy household rule, the one CIRISHolon's HOUSEHOLD-0 scored (the page runs HOUSEGEN-1 by default)
  return w;
}

function className(density, enumClass){
  let n = CLASSES[0].name; for (const c of CLASSES) if (density >= c.lower_people_per_km2) n = c.name;
  const want = enumClass === 0 ? (density >= 12000 ? "dense fabric" : "apartment blocks") : ["terraced streets", "suburban streets", "village"][enumClass - 1];
  if (n !== want) throw new Error(`density ${density} classes as ${n}, layout class ${enumClass}`);
  return n;
}

const inc = (o, k, by = 1) => { o[k] = (o[k] || 0) + by; };
const sortedObj = (o) => Object.fromEntries(Object.keys(o).map(Number).sort((a, b) => a - b).map((k) => [String(k), o[k]]));
function q3(xs){ if (!xs.length) return null; const s = xs.slice().sort((a, b) => a - b); return { min: s[0], median_lower: s[(s.length - 1) >> 1], max: s[s.length - 1] }; }
const r6 = (x) => +x.toFixed(6);

function group(){ return { leaves: 0, area_m2: 0, households: 0, homes: 0, empty: 0, byUnits: {}, floors: {}, foot: {}, leafBpk: [], leafDpk: [], leafDens: [], noBuilding: 0 }; }
function addLeaf(g, meta, bl){
  const area = meta[0] * meta[1];
  g.leaves++; g.area_m2 += area; g.households += meta[2]; g.leafDens.push(r6(meta[4]));
  let homes = 0;
  for (let b = 0; b + 7 < bl.length; b += 8){
    const units = bl[b + 6];
    if (units === 0){ g.empty++; continue; }
    homes++; inc(g.byUnits, units); inc(g.floors, bl[b + 5]); inc(g.foot, Math.floor(bl[b + 2] * bl[b + 3] / FOOT_BIN) * FOOT_BIN);
  }
  g.homes += homes;
  if (!homes) g.noBuilding++;
  g.leafBpk.push(r6(homes / area * 1e6)); g.leafDpk.push(r6(meta[2] / area * 1e6));
}
function groupOut(g){
  const bandB = {}, bandD = {};
  for (const [name] of BANDS){ bandB[name] = 0; bandD[name] = 0; }
  let dw = 0;
  for (const [u, c] of Object.entries(g.byUnits)){
    const n = +u, band = BANDS.find((b) => n >= b[1] && n <= b[2])[0];
    bandB[band] += c; bandD[band] += c * n; dw += c * n;
  }
  if (dw !== g.households) throw new Error(`dwellings ${dw} != households ${g.households}`);
  const km2 = g.area_m2 / 1e6;
  return {
    leaves: g.leaves, leaf_area_km2: r6(km2), households: g.households,
    dwelling_buildings: g.homes, non_dwelling_buildings: g.empty, leaves_without_a_dwelling_building: g.noBuilding,
    dwellings_per_building: g.homes ? { dwellings: g.households, buildings: g.homes, decimal: (g.households / g.homes).toFixed(6) } : null,
    buildings_by_dwellings_exact: sortedObj(g.byUnits),
    buildings_by_dwellings_band: bandB, dwellings_by_dwellings_band: bandD,
    dwelling_buildings_per_km2: r6(g.homes / km2), dwellings_per_km2: r6(g.households / km2),
    per_leaf: { people_per_km2: q3(g.leafDens), dwelling_buildings_per_km2: q3(g.leafBpk), dwellings_per_km2: q3(g.leafDpk) },
    floors_per_dwelling_building: sortedObj(g.floors),
    footprint_m2_per_dwelling_building: { bin_m2: FOOT_BIN, counts_by_bin_lower_bound: sortedObj(g.foot) },
  };
}
const outGroups = (o) => Object.fromEntries(CLASSES.map((c) => c.name).filter((n) => o[n]).map((n) => [n, groupOut(o[n])]));

function render(sampleOut){
  const w = kernel();
  const buf = (which, n) => Array.from(new Float64Array(w.memory.buffer, w.holon_buf_ptr(which) >>> 0, n));
  const samples = [];
  function layout(set, leaf, ctx){
    const id = HOLON.pack(leaf);
    new Float64Array(w.memory.buffer, w.holon_ctx_ptr() >>> 0, 33).set(ctx);
    if (w.holon_layout(id) < 0) throw new Error(`bad leaf ${id}`);
    const meta = buf(4, w.holon_buf_len(4)), bl = buf(2, w.holon_buf_len(2));
    let units = 0; for (let b = 6; b < bl.length; b += 8) units += bl[b];
    if (units !== meta[2] || meta[2] !== ctx[0]) throw new Error(`leaf ${id}: ${units} dwellings, ${meta[2]} households, budget ${ctx[0]}`);
    if (sampleOut) samples.push([set, id, ...ctx, bl.length / 8, ...bl].join(" "));
    return { meta, bl };
  }
  function leavesUnder(T, tile){ const out = []; (function down(c){ if (T.isLeaf(c)) out.push(c); else for (const k of HOLON.children(c)) down(k); })(tile); return out; }

  // (a) synthetic ladders
  const synthetic = { rungs: [], by_class: {} }, synAll = {};
  const t0 = HOLON.at(HOLON.L3, SYN_AT[0], SYN_AT[1]);
  for (const [cls, rungs] of Object.entries(LADDER)){
    for (const d of rungs){
      const hk = d / PPH;
      const areaKm2 = (c) => { const b = HOLON.boundsDeg(c), la = (b[0] + b[2]) / 2; return (b[2] - b[0]) * 111.32 * (b[3] - b[1]) * 111.32 * Math.cos(la * Math.PI / 180); };
      const T = HOLON.tree((c) => Math.round(hk * areaKm2(c)), () => 0, (p, pb) => {
        const kids = HOLON.children(p), ext = new Uint32Array(w.memory.buffer, w.holon_ext_ptr() >>> 0, kids.length);
        kids.forEach((k, q) => { const b = HOLON.boundsDeg(k); ext[q] = Math.round(Math.cos((b[0] + b[2]) / 2 * Math.PI / 180) * 1e6); });
        const n = w.holon_split_exact(HOLON.pack(p), pb); return buf(0, n);
      }, TARGET);
      const rung = group(), classes = {}; let tiles = 0;
      for (let k = 0; rung.leaves < LEAVES_PER_RUNG; k++){
        const tile = HOLON.make(HOLON.L3, t0.i, t0.j + k); tiles++;
        for (const leaf of leavesUnder(T, tile)){
          if (rung.leaves >= LEAVES_PER_RUNG) break;
          if (T.budget(leaf) === 0) continue;
          const { meta, bl } = layout(0, leaf, T.context(leaf));
          const c = className(meta[4], meta[3]);
          inc(classes, c);
          addLeaf(rung, meta, bl); addLeaf(synAll[c] = synAll[c] || group(), meta, bl);
        }
      }
      synthetic.rungs.push({ ladder_class: cls, people_per_km2: d, l3_tiles_walked: tiles, leaves_by_layout_class: classes, ...groupOut(rung) });
    }
  }
  synthetic.by_class = outGroups(synAll);

  // (b) real leaves
  const PP = new Map();
  const T = HOLON.tree(
    (c) => { const v = w.pop_root(HOLON.pack(c)); PP.set(HOLON.pack(c), buf(0, 2)[0]); return v; },
    () => 0,
    (p, pb) => { const k = HOLON.pack(p); let pp = PP.get(k); if (pp === undefined){ w.pop_of(k); pp = buf(0, 2)[0]; PP.set(k, pp); }
      const n = w.pop_split(k, pp, pb), o = buf(0, 2 * n), kids = HOLON.children(p), hh = [];
      for (let q = 0; q < n; q++){ PP.set(HOLON.pack(kids[q]), o[q]); hh.push(o[n + q]); }
      return hh; },
    TARGET);
  const people = (c) => { const k = HOLON.pack(c); let pp = PP.get(k); if (pp === undefined){ w.pop_of(k); pp = buf(0, 2)[0]; } return pp; };
  const places = [], realAll = {};
  for (const [name, la, lo] of PLACES){
    const tile = HOLON.at(HOLON.L3, la, lo), leaves = leavesUnder(T, tile), byClass = {};
    let empty = 0;
    for (const leaf of leaves){
      if (T.budget(leaf) === 0){ empty++; continue; }
      const ctx = T.context(leaf); ctx[32] = people(leaf);
      const { meta, bl } = layout(1, leaf, ctx);
      const c = className(meta[4], meta[3]);
      addLeaf(byClass[c] = byClass[c] || group(), meta, bl); addLeaf(realAll[c] = realAll[c] || group(), meta, bl);
    }
    places.push({ place: name, lat: la, lon: lo, l3_tile: HOLON.pack(tile), leaves_in_tile: leaves.length, leaves_without_households: empty, by_class: outGroups(byClass) });
  }
  if (sampleOut) fs.writeFileSync(sampleOut, samples.join("\n") + "\n");
  const doc = {
    what: "buildings and dwellings per building by street class, from the twin's own layout code (tiling.rs LeafLayout::build, run by the page's kernel holon_layout), MODEL",
    command: COMMAND,
    inputs: { "docs/kernel/kernel_scale.wasm": sha(here("../kernel/kernel_scale.wasm")), "docs/geo/gd/pop0.bin.gz": sha(here("gd/pop0.bin.gz")),
      "docs/geo/gd/pop1.bin.gz": sha(here("gd/pop1.bin.gz")), "docs/geo/geodata.js": sha(here("geodata.js")), "docs/holon.js": sha(here("../holon.js")), "docs/geo/classes.json": sha(here("classes.json")) },
    definitions: {
      class: "docs/geo/classes.json, from the layout's own people per km2 (holon_layout meta slot 4: households x 2.5 / leaf area) and checked against its DensityClass (meta slot 3; dense fabric = UrbanCore at >= 12,000)",
      leaf_area: "w x h of holon_layout meta (HolonId::size_m: the holon's side lengths in metres at its centre latitude)",
      dwelling_building: "a building with units >= 1; non_dwelling_buildings are village barns / sheds (units 0)",
      bands: "dwellings per building 1, 2, 3-6, 7-12, 13+; buildings_by_dwellings_exact gives the raw counts",
      per_leaf: "over the group's leaves: min, lower median (the floor((n-1)/2)-th of n sorted), max; per km2 of that leaf's area",
      footprint: "floor(w x d / 10) x 10 m2 of each dwelling building (frontage x depth)",
      parameters: "micro parameters at their defaults (the class thresholds and 2.5 persons per household are PARAM_SPECS MODEL values)",
    },
    synthetic: { scheme: `per rung a uniform world at the rung's people per km2 (households per km2 = rung / ${PPH}); level-0 budgets round(households per km2 x area), splits by cos(child centre latitude) x 1e6 through holon_split_exact; the page's tree and leaf rule; the first ${LEAVES_PER_RUNG} leaves holding households, in tree order, of the L3 tiles HOLON.make(3, i0, j0 + k), k = 0, 1, ..., (i0, j0) = HOLON.at(3, ${SYN_AT[0]}, ${SYN_AT[1]}); a leaf's seed is its id`, ...synthetic },
    real: { scheme: "every leaf of each place's L3 street tile, through the page's path (GHS-POP assets -> geoPopFine -> pop_load / pop_root / pop_split / pop_of -> HOLON.tree -> the leaf's context -> holon_layout)", places, by_class: outGroups(realAll) },
  };
  return JSON.stringify(doc, null, 1) + "\n";
}

if (require.main === module){
  const a = process.argv.slice(2), si = a.indexOf("--sample");
  const json = render(si >= 0 ? a[si + 1] : null);
  if (a.includes("--write")){ fs.writeFileSync(OUT, json); console.log("wrote docs/geo/building_hist.json"); }
  else if (a.includes("--check")){
    if (fs.readFileSync(OUT, "utf8") !== json){ console.error(`docs/geo/building_hist.json is stale: run ${COMMAND}`); process.exit(1); }
    console.log("docs/geo/building_hist.json is current");
  } else if (si < 0) process.stdout.write(json);
}
module.exports = { render, OUT, COMMAND };
