// Conditional consistency of the generated street tier on GHS-POP 2020 (popcond.js), run against the
// page's own assets and kernel: the first instance of CIRISHolon#3's conditional-consistency check.
// Claim: every holon's households equal the GHS-POP people of its rectangle (1/8-degree cells under
// the floor counting zero) divided by persons per household, within the largest-remainder rounding:
// at most one household per apportionment step from the 1.5-degree root down to it (|b - E| <= level + 1).
// Where GHS-POP reads under the floor, the street tile holds no household.
"use strict";
const test = require("node:test"), assert = require("node:assert");
const fs = require("node:fs"), path = require("node:path"), zlib = require("node:zlib");
const HOLON = require("../holon.js");
const { POPCOND, pcPeople, pcHouseholds, pcWeight, pcCellAt } = require("./popcond.js");
const G = require("./geodata.js");

const here = (p) => path.join(__dirname, p);
G.GEODATA.pop[0] = new Float32Array(zlib.gunzipSync(fs.readFileSync(here("gd/pop0.bin.gz"))).buffer.slice(0));
G.GEODATA.pop1code = new Uint8Array(zlib.gunzipSync(fs.readFileSync(here("gd/pop1.bin.gz"))));
G.geoPopFine();
const P = G.GEODATA.pop[1];

let wasm = null;
async function kernel(){
  if (wasm) return wasm;
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(here("../kernel/kernel_scale.wasm")), {});
  wasm = instance.exports; return wasm;
}
function tree(w){
  const root = (c) => { const b = HOLON.boundsDeg(c); return pcHouseholds(pcPeople(P, b[0], b[1], b[2], b[3])); };
  const ext = (c) => { const b = HOLON.boundsDeg(c); return pcWeight(pcPeople(P, b[0], b[1], b[2], b[3])); };
  const split = (p, pb, weights) => {
    const ep = w.holon_ext_ptr(); new Uint32Array(w.memory.buffer, ep, 25).set(weights);
    const n = w.holon_split_exact(HOLON.pack(p), pb);
    const op = w.holon_buf_ptr(0); return Array.from(new Float64Array(w.memory.buffer, op, n));
  };
  return HOLON.tree(root, ext, split, 60);
}
const expected = (c) => { const b = HOLON.boundsDeg(c); return pcPeople(P, b[0], b[1], b[2], b[3]) / POPCOND.PPH; };

// places to probe: populated contrast cases and land GHS-POP leaves empty
const PEOPLED = [["Zurich", 47.3769, 8.5417], ["Cairo", 30.0444, 31.2357], ["Lagos", 6.5244, 3.3792], ["US suburb (Maryland)", 39.2, -77.25], ["rural French village (Cher)", 46.3, 2.3]];
const EMPTY = [["mid-Urals", 60.0, 59.5], ["central Sahara", 23.5, 12.0], ["Greenland interior", 72.0, -40.0], ["Amazon interior", -3.5, -67.5], ["open Atlantic", 0.0, -30.0]];

test("level-1 people are the 1/2-degree totals (geodata conserves them)", () => {
  let s0 = 0, s1 = 0; for (const v of G.GEODATA.pop[0]) s0 += v; for (const v of P) s1 += v;
  assert.ok(Math.abs(s1 - s0) / s0 < 1e-6, s0 + " vs " + s1);
});

test("every holon from the root to the street tile holds its rectangle's people / 2.5, within the rounding", async () => {
  const w = await kernel(); assert.equal(typeof w.holon_split_exact, "function", "kernel without holon_split_exact");
  const T = tree(w); const rows = [];
  for (const [name, la, lo] of PEOPLED.concat(EMPTY)){
    const leaf = T.leafAt(la, lo);
    for (let l = 0; l <= leaf.level; l++){
      const a = HOLON.ancestor(leaf, l), b = T.budget(a), e = expected(a);
      assert.ok(Math.abs(b - e) <= l + 1, `${name} L${l}: ${b} households, people/2.5 = ${e.toFixed(2)}`);
    }
    const cell = pcCellAt(P, la, lo);
    rows.push(`${name}: leaf L${leaf.level} ${T.budget(leaf)} households (E ${expected(leaf).toFixed(2)}); 1/8° cell ${cell.people.toFixed(1)} people, ${cell.perKm2.toFixed(2)}/km²`);
  }
  console.log(rows.join("\n"));
});

test("land GHS-POP leaves empty gets no households; peopled places get some", async () => {
  const T = tree(await kernel());
  for (const [name, la, lo] of EMPTY){
    const cell = pcCellAt(P, la, lo);
    assert.ok(cell.perKm2 < POPCOND.FLOOR_KM2, `${name}: ${cell.perKm2}/km²`);
    assert.equal(T.budget(T.leafAt(la, lo)), 0, name);
  }
  for (const [name, la, lo] of PEOPLED) assert.ok(T.budget(T.leafAt(la, lo)) > 0, name);
});
