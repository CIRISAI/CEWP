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
//   against the overlap computed independently in JavaScript, through the in-cell seam's pixels where a holon
//   covers a cell in part);
// * at the leaf: its households' persons sum to its people exactly, and its households' devices sum to
//   the leaf's device population exactly (the node cap is a display sample of it); its households
//   summed by access network (their second parent, netgraph::attach_access) are its households;
// * crossings: on each side of every split from L2 down, the arterials crossing the children along
//   the side sum to those crossing the parent's side, and the holon across the side, answering from
//   its own split, counts the same (tiling::arterial_crossings; written with "side" and "peer");
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
const { POPCOND, pcPeople, pcCellAt, pcCellKm2 } = require("./popcond.js");
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
  const w = instance.exports, n = P.length;
  // the in-cell seam's anchors, as the page passes them (tiling.js tlPopKernel): every Natural Earth place, largest first
  { const buf = fs.readFileSync(here("places.bin")), dv = new DataView(buf.buffer, buf.byteOffset, buf.byteLength), np = dv.getUint32(4, true), ll = []; let i = 8;
    for (let k = 0; k < np; k++){ ll.push(dv.getInt32(i, true), dv.getInt32(i + 4, true)); i += 14 + buf[i + 13]; }
    const ap = w.alloc(ll.length * 4) >>> 0; new Int32Array(w.memory.buffer, ap, ll.length).set(ll); w.pop_anchors(ap, np); }
  const ptr = w.alloc(n * 4) >>> 0;
  new Float32Array(w.memory.buffer, ptr, n).set(P);
  const cells = w.pop_load(ptr, n, POP_GRID_PEOPLE);
  const tot = Array.from(new Float64Array(w.memory.buffer, w.holon_buf_ptr(0) >>> 0, 4));
  K = { w, cells, tot }; return K;
}
const out = (w, n) => Array.from(new Float64Array(w.memory.buffer, w.holon_buf_ptr(0) >>> 0, n));
function root(w, c){ w.pop_root(HOLON.pack(c)); const [p, h] = out(w, 2); return { p, h }; }
function split(w, c, pop){ const n = w.pop_split(HOLON.pack(c), pop.p, pop.h), o = out(w, 2 * n); return HOLON.children(c).map((k, i) => ({ c: k, p: o[i], h: o[n + i] })); }
const sum = (a) => a.reduce((x, y) => x + y, 0);
// the overlap the chain should give, computed here in JavaScript: GHS-POP over the holon's rectangle, a cell the holon
// covers whole at its people, a cell it covers in part through the in-cell seam's pixels (kernel pop_cell_pixels,
// CIRISHolon's UNEVEN-0), each pixel by the share of it covered; popcond.js pcPeople is the even spread (seam off)
function seamPeople(w, la0, lo0, la1, lo1){
  const s = POPCOND.STEP, NX = POPCOND.NX, NY = POPCOND.NY, pd = s / 15;
  const r0 = Math.max(0, Math.floor((90 - la1) / s)), r1 = Math.min(NY - 1, Math.ceil((90 - la0) / s) - 1);
  const c0 = Math.floor((lo0 + 180) / s), c1 = Math.ceil((lo1 + 180) / s) - 1; let total = 0;
  for (let r = r0; r <= r1; r++){
    const top = 90 - r * s, fy = (Math.min(top, la1) - Math.max(top - s, la0)) / s; if (fy <= 0) continue;
    for (let c = c0; c <= c1; c++){
      const cc = ((c % NX) + NX) % NX, p = P[r * NX + cc]; if (!(p > 0) || p / pcCellKm2(r) < POPCOND.FLOOR_KM2) continue;
      const left = -180 + c * s, fx = (Math.min(left + s, lo1) - Math.max(left, lo0)) / s; if (fx <= 0) continue;
      if (fx >= 1 && fy >= 1){ total += p; continue; }
      w.pop_cell_pixels(r, cc); const px = out(w, 225);
      for (let i = 0; i < 225; i++){ if (!px[i]) continue;
        const pt = top - Math.floor(i / 15) * pd, pl = left + (i % 15) * pd;
        const gy = (Math.min(pt, la1) - Math.max(pt - pd, la0)) / pd, gx = (Math.min(pl + pd, lo1) - Math.max(pl, lo0)) / pd;
        if (gx > 0 && gy > 0) total += px[i] * gx * gy; }
    }
  }
  return total;
}
// the four sides of a split's arterial crossings (kernel pop_crossings): [{ parent, peer, children }]
function crossings(w, c){
  const len = w.pop_crossings(HOLON.pack(c)); if (!len) return [];
  const o = out(w, len), r = []; let k = 0;
  for (let d = 0; d < 4; d++){ const n = o[k + 2]; r.push({ parent: o[k], peer: o[k + 1], children: o.slice(k + 3, k + 3 + n) }); k += 3 + n; }
  return r;
}

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
  const { w, tot } = await kernel(); const lines = [], rows = [], seen = new Set(); let nets = 0, ncross = 0, streets = 0; const streetless = [], seamTiles = [];
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
      // arterial crossings of this split, per side, with the holon across it answering from its side
      if (s.c.level >= 2){
        for (const [c, back] of [[s.c, null], ...[0, 1, 2, 3].map((d) => [HOLON.neighbour(s.c, d), (d + 2) % 4])]){
          if (!c) continue;
          const sides = crossings(w, c);
          sides.forEach((x, d) => {
            if (back !== null && d !== back) return;
            const key = HOLON.pack(c) + ":" + d; if (seen.has(key)) return; seen.add(key);
            assert.strictEqual(sum(x.children), x.parent, `${name} L${c.level} side ${d} crossings`);
            if (x.peer >= 0) line({ id: HOLON.pack(c), level: c.level, seed: 0, kind: "crossings", side: d, peer: x.peer, parent: x.parent, children: x.children });
            ncross += x.parent;
          });
        }
      }
      // cross-check against JavaScript's own overlap (popcond.js), scaled to the cohort total
      const b = HOLON.boundsDeg(s.c), e = seamPeople(w, b[0], b[1], b[2], b[3]) * scale;
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
      // the household seam: CIRISHolon's generator for the leaf's L4 tile, households split whole over the tile's leaves
      const sn = w.pop_household_seam(HOLON.pack(leaf)), so = out(w, sn);
      if (so[0] === 1 && so[1] === 1){
        const nl = so[11], lp = [], lh = []; for (let k = 0; k < nl; k++){ lp.push(so[12 + 2 * k]); lh.push(so[13 + 2 * k]); }
        assert.strictEqual(sum(lp), so[3], `${name}: the tile's households hold its people exactly`);
        assert.strictEqual(sum(lh), so[10], `${name}: the tile's households over its leaves`);
        assert.strictEqual(n, so[9], `${name}: the leaf holds the households the generator gave it`);
        const tile = so[2], tc = HOLON.unpack(tile);
        line({ id: tile, level: tc.level, seed: 0, kind: "people", parent: so[3], children: lp });
        line({ id: tile, level: tc.level, seed: 0, kind: "households", parent: so[10], children: lh });
        seamTiles.push(`${name} ${["certified", "refused", "unresolved"][so[7]]}/${["certified", "refused", "unresolved"][so[8]]}`);
        assert.ok(lp.some((p, k) => p === sum(per) && lh[k] === n), `${name}: the leaf's persons are its households' sizes, one of the tile's leaves`);
      } else {
        assert.strictEqual(n, pop.h, `${name}: households`);
        assert.strictEqual(sum(per), pop.p, `${name}: persons = people`);
      }
      assert.strictEqual(sum(devs), dev, `${name}: devices`);
      // the household's second parent: summed by access network, the leaf's households again
      const na = w.micro_access(), acc = out(w, 2 * na), byNet = acc.filter((_, k) => k % 2 === 1);
      assert.ok(na >= 1 && acc.every((v, k) => k % 2 === 1 || v > 0), `${name}: every household has an access network`);
      assert.strictEqual(sum(byNet), n, `${name}: households by access network`);
      nets += na;
      // and summed by street (the household's building's frontage street, tiling::StreetGraph)
      const ns = w.micro_streets(), st = out(w, 2 * ns);
      assert.strictEqual(sum(st.filter((_, k) => k % 2 === 1)), n, `${name}: households by street`);
      // every household is on a street where the leaf has one (a small leaf whose lines all pass outside it has none)
      const roads = w.micro_buf_len(4) / 5;
      assert.ok(roads === 0 || st.every((v, k) => k % 2 === 1 || v >= 0), `${name}: every household is on a street`);
      if (!roads) streetless.push(name);
      streets += ns;
      const id = HOLON.pack(leaf);
      line({ id, level: leaf.level, seed: 0, kind: "people", parent: sum(per), children: per }); // with the seam, the leaf's people follow its households
      line({ id, level: leaf.level, seed: 0, kind: "households", parent: n, children: per.map(() => 1) });
      line({ id, level: leaf.level, seed: 0, kind: "devices", parent: dev, children: devs });
    }
    const cell = pcCellAt(P, la, lo);
    if (EMPTY.has(name)){ assert.ok(cell.perKm2 < POPCOND.FLOOR_KM2, name); assert.strictEqual(pop.h, 0, name); }
    rows.push(`${name}: leaf L${leaf.level}, ${pop.p} people, ${pop.h} households, ${persons} persons, ${dev} devices (${nodes} nodes drawn)`);
  }
  fs.writeFileSync(OUT, lines.join("\n") + "\n");
  console.log(rows.join("\n")); console.log(`${lines.length} lines -> ${OUT}; ${nets} (leaf, access network) pairs, households summed both ways; ${streets} (leaf, street) pairs (leaves with no street: ${streetless.join(", ") || "none"}); household seam tiles: ${seamTiles.length} (${[...new Set(seamTiles.map((x) => x.split(" ").pop()))].join(", ")}); ${seen.size} crossing sides carrying ${ncross} arterial crossings`);
});
