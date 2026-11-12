// The population chain of the street tier, end to end on the page's own assets and kernel
// (cewp_sim::popchain through kernel-wasm pop_load / pop_root / pop_split, and the focal world's
// micro_population): the first instance of CIRISHolon#3's conditional consistency.
//
// Claims, at every holon from the 1.5-degree root down to the street tile, at every probed place:
// * people: the children's people sum to the parent's, exactly (an apportionment);
// * households: the children's households sum to the parent's, exactly (an apportionment by the
//   children's people);
// * a holon's households are its people / 2.5 within the rounding of the steps above it, and its
//   people track the overlap of GHS-POP within the same bound scaled to the cohort total (cross-check
//   against popcond.js pcPeople, computed independently in JavaScript);
// * at the leaf: its households' persons sum to its people exactly, and its households' devices sum to
//   the leaf's device population exactly (the node cap is a display sample of it); its households
//   summed by access network (their second parent, netgraph::attach_access) are its households;
// * where GHS-POP reads under the floor, the street tile holds nobody.
//
// Every check is also written in CIRISHolon's sums format, one JSON object per line per (holon, seed,
// kind), to $CEWP_SUMS_OUT (default: <os tmp>/cewp-popchain-sums.jsonl); check it with
//   (cd /home/emoore/CIRISHolon/engine && cargo run -p holon-scene --example check_sums -- <file>)
// The twin's tree takes no seed (a holon's id is its seed), so every line carries seed 0.
"use strict";
const test = require("node:test"), assert = require("node:assert");
const fs = require("node:fs"), path = require("node:path"), zlib = require("node:zlib"), os = require("node:os");
const HOLON = require("../holon.js");
const { POPCOND, pcPeople, pcCellAt } = require("./popcond.js");
const G = require("./geodata.js");

const here = (p) => path.join(__dirname, p);
G.GEODATA.pop[0] = new Float32Array(zlib.gunzipSync(fs.readFileSync(here("gd/pop0.bin.gz"))).buffer.slice(0));
G.GEODATA.pop1code = new Uint8Array(zlib.gunzipSync(fs.readFileSync(here("gd/pop1.bin.gz"))));
G.geoPopFine();
const P = G.GEODATA.pop[1];
const POP_GRID_PEOPLE = 7830639988; // docs/globe.js: the cohort tier's GHS total, the street tier's too
const OUT = process.env.CEWP_SUMS_OUT || path.join(os.tmpdir(), "cewp-popchain-sums.jsonl");

let K = null;
async function kernel(){
  if (K) return K;
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(here("../kernel/kernel_scale.wasm")), {});
  const w = instance.exports, n = P.length, ptr = w.alloc(n * 4) >>> 0;
  new Float32Array(w.memory.buffer, ptr, n).set(P);
  const cells = w.pop_load(ptr, n, POP_GRID_PEOPLE);
  const tot = Array.from(new Float64Array(w.memory.buffer, w.holon_buf_ptr(0) >>> 0, 4));
  K = { w, cells, tot }; return K;
}
const out = (w, n) => Array.from(new Float64Array(w.memory.buffer, w.holon_buf_ptr(0) >>> 0, n));
function root(w, c){ w.pop_root(HOLON.pack(c)); const [p, h] = out(w, 2); return { p, h }; }
function split(w, c, pop){ const n = w.pop_split(HOLON.pack(c), pop.p, pop.h), o = out(w, 2 * n); return HOLON.children(c).map((k, i) => ({ c: k, p: o[i], h: o[n + i] })); }
const sum = (a) => a.reduce((x, y) => x + y, 0);

const PLACES = [["mid-Urals", 60.0, 59.5], ["central Sahara", 23.5, 12.0], ["Greenland interior", 72.0, -40.0], ["Amazon interior", -3.5, -67.5], ["open Atlantic", 0.0, -30.0],
  ["Zurich", 47.3769, 8.5417], ["Cairo", 30.0444, 31.2357], ["Lagos", 6.5244, 3.3792], ["US suburb (Maryland)", 39.2, -77.25], ["rural French village (Cher)", 46.3, 2.3],
  ["Dhaka", 23.7104, 90.4074], ["Manila", 14.62, 120.97], ["Mumbai", 19.043, 72.856], ["Kolkata", 22.5726, 88.3639], ["Karachi", 24.8607, 67.0011], ["Delhi", 28.6517, 77.2219],
  ["Kowloon", 22.3193, 114.1694], ["Macau", 22.1987, 113.5439], ["Lagos Island", 6.4541, 3.3947], ["Tokyo", 35.6895, 139.6917], ["Paris", 48.8566, 2.3522], ["Manhattan", 40.7831, -73.9712],
  ["densest Karachi", 24.9375, 67.0625], ["Giza", 30.0625, 31.1875], ["densest Dhaka", 23.6875, 90.4375], ["densest Kowloon", 22.3125, 114.1875], ["densest Delhi", 28.6875, 77.3125]];
const EMPTY = new Set(["mid-Urals", "central Sahara", "Greenland interior", "Amazon interior", "open Atlantic"]);

// walk from the root to the street tile under (la, lo): leaf = first holon at L3+ holding <= 60 households
function walk(w, la, lo){
  let c = HOLON.at(0, la, lo), pop = root(w, c); const steps = [];
  for (;;){
    if (c.level >= HOLON.L3 && (pop.h <= 60 || c.level >= HOLON.MAX_LEVEL)) return { leaf: c, pop, steps };
    const kids = split(w, c, pop); steps.push({ c, pop, kids });
    const next = HOLON.at(c.level + 1, la, lo), k = kids.find((x) => x.c.i === next.i && x.c.j === next.j);
    c = k.c; pop = { p: k.p, h: k.h };
  }
}

test("the street tier's world is the cohort tier's GHS total, apportioned", async () => {
  const { tot } = await kernel();
  assert.strictEqual(tot[0], POP_GRID_PEOPLE);
  assert.strictEqual(tot[1], Math.round(POP_GRID_PEOPLE / 2.5));
  console.log(`world: ${tot[0]} people, ${tot[1]} households; floored 1/8-degree cells: ${tot[3]} cells, ${tot[2]} people before the apportionment`);
});

test("every split from the root to the street tile sums exactly; the leaf's persons and devices sum to it", async () => {
  const { w, tot } = await kernel(); const lines = [], rows = []; let nets = 0;
  const scale = tot[0] / tot[2]; // cohort total over the floored cell total
  const line = (o) => lines.push(JSON.stringify(o));
  for (const [name, la, lo] of PLACES){
    const { leaf, pop, steps } = walk(w, la, lo);
    for (const s of steps){
      const id = HOLON.pack(s.c), pp = s.kids.map((k) => k.p), hh = s.kids.map((k) => k.h);
      assert.strictEqual(sum(pp), s.pop.p, `${name} L${s.c.level} people`);
      assert.strictEqual(sum(hh), s.pop.h, `${name} L${s.c.level} households`);
      line({ id, level: s.c.level, seed: 0, kind: "people", parent: s.pop.p, children: pp });
      line({ id, level: s.c.level, seed: 0, kind: "households", parent: s.pop.h, children: hh });
      // cross-check against JavaScript's own overlap (popcond.js), scaled to the cohort total
      const b = HOLON.boundsDeg(s.c), e = pcPeople(P, b[0], b[1], b[2], b[3]) * scale;
      // bound: each overlapped cell was rounded to whole people (0.5 each), one unit per apportionment step, and the scale's float error
      const ncell = Math.ceil((b[2] - b[0]) / POPCOND.STEP + 1) * Math.ceil((b[3] - b[1]) / POPCOND.STEP + 1);
      assert.ok(Math.abs(s.pop.p - e) <= 0.5 * ncell * scale + s.c.level + 1 + 1e-6 * e, `${name} L${s.c.level}: ${s.pop.p} people, overlap x scale ${e.toFixed(1)}`);
      assert.ok(Math.abs(s.pop.h - s.pop.p / 2.5) <= s.c.level + 2, `${name} L${s.c.level}: ${s.pop.h} households for ${s.pop.p} people`);
    }
    let dev = 0, nodes = 0, persons = 0;
    if (pop.h > 0){ // the focal world on the leaf: persons and devices
      // the page's own tree (tiling.js): households memoised by HOLON.tree, people beside them
      const pp = new Map(), T = HOLON.tree((c) => { const r = root(w, c); pp.set(HOLON.pack(c), r.p); return r.h; }, () => 0,
        (p, pb) => { const k = HOLON.pack(p); if (!pp.has(k)){ w.pop_of(k); pp.set(k, out(w, 2)[0]); } return split(w, p, { p: pp.get(k), h: pb }).map((x) => { pp.set(HOLON.pack(x.c), x.p); return x.h; }); }, 60);
      assert.strictEqual(T.budget(leaf), pop.h, `${name}: the page tree agrees with the walk`);
      const ctx = T.context(leaf); ctx[0] = pop.h; ctx[32] = pop.p;
      const cp = w.holon_ctx_ptr(); new Float64Array(w.memory.buffer, cp, 33).set(ctx);
      w.micro_build_leaf(HOLON.pack(leaf)); const n = w.micro_population(), o = out(w, 4 + 2 * n);
      const per = o.slice(4, 4 + n), devs = o.slice(4 + n, 4 + 2 * n);
      persons = o[1]; dev = o[2]; nodes = o[3];
      assert.strictEqual(n, pop.h, `${name}: households`);
      assert.strictEqual(sum(per), pop.p, `${name}: persons = people`);
      assert.strictEqual(sum(devs), dev, `${name}: devices`);
      // the household's second parent: summed by access network, the leaf's households again
      const na = w.micro_access(), acc = out(w, 2 * na), byNet = acc.filter((_, k) => k % 2 === 1);
      assert.ok(na >= 1 && acc.every((v, k) => k % 2 === 1 || v > 0), `${name}: every household has an access network`);
      assert.strictEqual(sum(byNet), pop.h, `${name}: households by access network`);
      nets += na;
      const id = HOLON.pack(leaf);
      line({ id, level: leaf.level, seed: 0, kind: "people", parent: pop.p, children: per });
      line({ id, level: leaf.level, seed: 0, kind: "households", parent: pop.h, children: per.map(() => 1) });
      line({ id, level: leaf.level, seed: 0, kind: "devices", parent: dev, children: devs });
    }
    const cell = pcCellAt(P, la, lo);
    if (EMPTY.has(name)){ assert.ok(cell.perKm2 < POPCOND.FLOOR_KM2, name); assert.strictEqual(pop.h, 0, name); }
    rows.push(`${name}: leaf L${leaf.level}, ${pop.p} people, ${pop.h} households, ${persons} persons, ${dev} devices (${nodes} nodes drawn)`);
  }
  fs.writeFileSync(OUT, lines.join("\n") + "\n");
  console.log(rows.join("\n")); console.log(`${lines.length} lines -> ${OUT}; ${nets} (leaf, access network) pairs, households summed both ways`);
});
