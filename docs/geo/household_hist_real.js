// Household sizes over the street leaves the page actually builds, at the probe places:
// docs/geo/household_hist_real.json. The real-leaf companion of crates/cewp-sim/examples/household_hist.rs
// (synthetic leaves, docs/geo/household_hist.json).
//
// The page's own path, end to end: the shipped GHS-POP assets (docs/geo/gd/pop0, pop1) through
// geodata.js geoPopFine, loaded into the page's kernel (docs/kernel/kernel_scale.wasm, built from
// cewp_sim) by pop_load; the page's tree (HOLON.tree with tiling.js's tlRoot / tlSplit callbacks:
// pop_root, pop_split, pop_of); every leaf under the place's L3 street tile (the tree's leaf rule:
// L3 or below and at most 60 households, or L12); the leaf's context with its people in slot 32
// (tiling.js tlContext); holon_layout for its buildings and micro_build_leaf + micro_population for
// its households' persons, as globe.js does. Households are pushed in building order, each building
// giving `units` dwellings (micro.rs populate), so household k sits in the building whose running
// unit count first passes k.
//
//   node docs/geo/household_hist_real.js            print the JSON
//   node docs/geo/household_hist_real.js --write    write docs/geo/household_hist_real.json
//   node docs/geo/household_hist_real.js --check    regenerate and compare (exit 1 when stale)
//   node docs/geo/household_hist_real.js --sample F write (id, context, persons, units) of every
//        leaf with households to F, one line each, for the native cross-check
//        (cargo run --release -p cewp-sim --example household_hist -- --crosscheck F)
"use strict";
const fs = require("node:fs"), path = require("node:path"), zlib = require("node:zlib"), crypto = require("node:crypto");
const HOLON = require("../holon.js");
const G = require("./geodata.js");

const here = (p) => path.join(__dirname, p);
const OUT = here("household_hist_real.json");
const COMMAND = "node docs/geo/household_hist_real.js --write";
const POP_GRID_PEOPLE = 7830639988; // docs/globe.js: the cohort tier's GHS total, the street tier's too
const TARGET = 60; // tiling.js TL_TARGET (P::TargetHouseholds)
// the probe places of FSD/twin/globe.md and docs/geo/popcond.test.js, same coordinates
const PLACES = [["Zurich", 47.3769, 8.5417], ["Cairo", 30.0444, 31.2357], ["Lagos", 6.5244, 3.3792],
  ["US suburb (Maryland)", 39.2, -77.25], ["rural French village (Cher)", 46.3, 2.3],
  ["Dhaka", 23.7104, 90.4074], ["Mumbai", 19.043, 72.856], ["Karachi", 24.8607, 67.0011],
  ["Kowloon", 22.3193, 114.1694], ["Manhattan", 40.7831, -73.9712], ["Tokyo", 35.6895, 139.6917], ["Paris", 48.8566, 2.3522]];
const CLASSES = JSON.parse(fs.readFileSync(here("classes.json"), "utf8")).classes; // ascending lower bounds
const ENUM_CLASS = ["UrbanCore", "Urban", "Suburban", "Village"]; // micro.rs DensityClass discriminants
const BANDS = [["1", 1, 1], ["2-4", 2, 4], ["5+", 5, Infinity]];
const sha = (f) => crypto.createHash("sha256").update(fs.readFileSync(f)).digest("hex");

function kernel(){
  G.GEODATA.pop[0] = new Float32Array(zlib.gunzipSync(fs.readFileSync(here("gd/pop0.bin.gz"))).buffer.slice(0));
  G.GEODATA.pop1code = new Uint8Array(zlib.gunzipSync(fs.readFileSync(here("gd/pop1.bin.gz"))));
  G.geoPopFine();
  const P = G.GEODATA.pop[1];
  const m = new WebAssembly.Module(fs.readFileSync(here("../kernel/kernel_scale.wasm")));
  const w = new WebAssembly.Instance(m, {}).exports, ptr = w.alloc(P.length * 4) >>> 0;
  new Float32Array(w.memory.buffer, ptr, P.length).set(P);
  w.pop_load(ptr, P.length, POP_GRID_PEOPLE);
  return w;
}

const gcd = (a, b) => (b === 0 ? a : gcd(b, a % b));
const rat = (p, q) => { const g = gcd(p, q) || 1; return `${p / g}/${q / g}`; };
function hist(){ return { counts: [0], households: 0, persons: 0 }; }
function add(h, s){ while (h.counts.length <= s) h.counts.push(0); h.counts[s]++; h.households++; h.persons += s; }
function histOut(h){
  let max = h.counts.length - 1; while (max > 0 && h.counts[max] === 0) max--;
  return { counts_by_size: h.counts.slice(0, max + 1), households: h.households, persons: h.persons,
    mean: h.households ? { persons: h.persons, households: h.households, reduced: rat(h.persons, h.households), decimal: (h.persons / h.households).toFixed(6) } : null,
    max_size: max, size_0_households: h.counts[0], over_6_households: h.counts.slice(7).reduce((a, b) => a + b, 0) };
}
// people / households over leaves: min, lower median, max, exact (cross-multiplied, products < 2^53)
function ratios(rs){
  if (!rs.length) return null;
  const s = rs.slice().sort((a, b) => a[0] * b[1] - b[0] * a[1] || a[1] - b[1]);
  const f = (r) => ({ reduced: rat(r[0], r[1]), decimal: (r[0] / r[1]).toFixed(6) });
  return { min: f(s[0]), median_lower: f(s[(s.length - 1) >> 1]), max: f(s[s.length - 1]) };
}
function group(){ return { leaves: 0, final: hist(), ratios: [] }; }
function groupOut(g){ return { leaves: g.leaves, ...histOut(g.final), people_per_household: ratios(g.ratios) }; }

function className(density){ let n = CLASSES[0].name; for (const c of CLASSES) if (density >= c.lower_people_per_km2) n = c.name; return n; }

function render(sampleOut){
  const w = kernel();
  const out = (n, which = 0) => Array.from(new Float64Array(w.memory.buffer, w.holon_buf_ptr(which) >>> 0, n));
  const PP = new Map();
  const T = HOLON.tree(
    (c) => { const v = w.pop_root(HOLON.pack(c)); PP.set(HOLON.pack(c), out(2)[0]); return v; },
    () => 0,
    (p, pb) => { const k = HOLON.pack(p); let pp = PP.get(k); if (pp === undefined){ w.pop_of(k); pp = out(2)[0]; PP.set(k, pp); }
      const n = w.pop_split(k, pp, pb), o = out(2 * n), kids = HOLON.children(p), hh = [];
      for (let q = 0; q < n; q++){ PP.set(HOLON.pack(kids[q]), o[q]); hh.push(o[n + q]); }
      return hh; },
    TARGET);
  const people = (c) => { const k = HOLON.pack(c); let pp = PP.get(k); if (pp === undefined){ w.pop_of(k); pp = out(2)[0]; } return pp; };
  const places = [], all = { leaves: 0, final: hist(), ratios: [] }, allClass = {}, allBand = {};
  const samples = [];
  for (const [name, la, lo] of PLACES){
    const tile = HOLON.at(HOLON.L3, la, lo), leaves = [];
    (function down(c){ if (T.isLeaf(c)) leaves.push(c); else for (const k of HOLON.children(c)) down(k); })(tile);
    const pl = group(), byClass = {}, byBand = {}; let empty = 0, unitsMismatch = 0;
    for (const leaf of leaves){
      const hh = T.budget(leaf), pp = people(leaf);
      if (hh === 0){ empty++; continue; }
      const id = HOLON.pack(leaf), ctx = T.context(leaf); ctx[32] = pp;
      const cp = w.holon_ctx_ptr() >>> 0;
      new Float64Array(w.memory.buffer, cp, 33).set(ctx);
      w.holon_layout(id);
      const meta = out(w.holon_buf_len(4), 4), bl = out(w.holon_buf_len(2), 2);
      const units = []; for (let b = 0; b + 7 < bl.length; b += 8) if (bl[b + 6] > 0) units.push(bl[b + 6]);
      new Float64Array(w.memory.buffer, cp, 33).set(ctx);
      w.micro_build_leaf(id);
      const n = w.micro_population(), o = out(4 + n), per = o.slice(4, 4 + n);
      if (n !== hh || o[1] !== pp) throw new Error(`${name} ${id}: ${n} households / ${o[1]} persons, budget ${hh} / ${pp}`);
      const hu = []; for (const u of units) for (let k = 0; k < u; k++) hu.push(u);
      if (hu.length !== n){ unitsMismatch++; continue; }
      const cls = className(meta[4]);
      const want = meta[3] === 0 ? (meta[4] >= 12000 ? "dense fabric" : "apartment blocks") : ["terraced streets", "suburban streets", "village"][meta[3] - 1];
      if (cls !== want) throw new Error(`${name} ${id}: density ${meta[4]} classes as ${cls}, layout class ${ENUM_CLASS[meta[3]]}`);
      if (sampleOut) samples.push([id, ...ctx, n, ...per, units.length, ...units].join(" "));
      const gs = [pl, all, byClass[cls] = byClass[cls] || group(), allClass[cls] = allClass[cls] || group()];
      for (const g of gs){ g.leaves++; g.ratios.push([pp, hh]); }
      per.forEach((s, k) => {
        const band = BANDS.find((b) => hu[k] >= b[1] && hu[k] <= b[2])[0];
        for (const g of [...gs, byBand[band] = byBand[band] || group(), allBand[band] = allBand[band] || group()]) add(g.final, s);
      });
      for (const band of new Set(hu.map((u) => BANDS.find((b) => u >= b[1] && u <= b[2])[0]))){ byBand[band].leaves++; allBand[band].leaves++; }
    }
    if (unitsMismatch) throw new Error(`${name}: ${unitsMismatch} leaves whose building units do not sum to their households`);
    const order = (o) => Object.fromEntries(Object.keys(o).sort((a, b) => a.localeCompare(b)).map((k) => [k, groupOut(o[k])]));
    places.push({ place: name, lat: la, lon: lo, l3_tile: HOLON.pack(tile), leaves_in_tile: leaves.length, leaves_without_households: empty,
      ...groupOut(pl), by_density_class: order(byClass), by_units_per_building: order(byBand) });
  }
  if (sampleOut) fs.writeFileSync(sampleOut, samples.join("\n") + "\n");
  const order = (o) => Object.fromEntries(Object.keys(o).sort((a, b) => a.localeCompare(b)).map((k) => [k, groupOut(o[k])]));
  const doc = {
    what: "household sizes (persons per household) over every street leaf the page builds inside each probe place's L3 street tile, from the page's own path (GHS-POP assets -> geodata.js geoPopFine -> kernel pop_load / pop_root / pop_split / pop_of -> HOLON.tree -> holon_layout -> micro_build_leaf -> micro_population), MODEL",
    command: COMMAND,
    inputs: { "docs/kernel/kernel_scale.wasm": sha(here("../kernel/kernel_scale.wasm")), "docs/geo/gd/pop0.bin.gz": sha(here("gd/pop0.bin.gz")),
      "docs/geo/gd/pop1.bin.gz": sha(here("gd/pop1.bin.gz")), "docs/geo/geodata.js": sha(here("geodata.js")), "docs/holon.js": sha(here("../holon.js")), "docs/geo/classes.json": sha(here("classes.json")) },
    parameters: "micro parameters at their defaults (micro_params_reset; the page's sliders set adoption and reach shares, which no household size or layout reads); leaf rule L3+ and <= 60 households or L12; 2.5 persons per household",
    density_class: "the leaf layout's people per km2 (holon_layout meta slot 4) against docs/geo/classes.json lower bounds; checked against the layout's own DensityClass (meta slot 3, dense fabric = UrbanCore at >= 12,000)",
    units_per_building: "the dwellings of the building a household sits in (holon_layout buildings slot 6), bands 1, 2-4, 5+; leaves counts the leaves with at least one household in the band",
    people_per_household: "the leaf's people over its households, over the leaves of the group: min, lower median (the floor((n-1)/2)-th of n sorted), max",
    places,
    all_places: { ...groupOut(all), by_density_class: order(allClass), by_units_per_building: order(allBand) },
  };
  return JSON.stringify(doc, null, 1) + "\n";
}

if (require.main === module){
  const a = process.argv.slice(2), si = a.indexOf("--sample");
  const json = render(si >= 0 ? a[si + 1] : null);
  if (a.includes("--write")){ fs.writeFileSync(OUT, json); console.log("wrote docs/geo/household_hist_real.json"); }
  else if (a.includes("--check")){
    if (fs.readFileSync(OUT, "utf8") !== json){ console.error(`docs/geo/household_hist_real.json is stale: run ${COMMAND}`); process.exit(1); }
    console.log("docs/geo/household_hist_real.json is current");
  } else if (si < 0) process.stdout.write(json);
}
module.exports = { render, OUT, COMMAND };
