// APPEARANCE-0 through the twin (docs/climate.js, kernel door_*): the shipped grid is the one CIRISHolon's sidecar names
// (sha256 packed and unpacked), the twelve places CIRISHolon's results list read their class through the page's lookup,
// and the kernel, given the grid through its table door, answers as the page does, cell by cell and leaf by leaf.
"use strict";
const test = require("node:test"), assert = require("node:assert"), fs = require("node:fs"), path = require("node:path"), crypto = require("node:crypto"), zlib = require("node:zlib");
const ROOT = path.join(__dirname, "..", "..", "..");
const { climateCode, CLIMATE_SYMBOLS, climateColour } = require(path.join(ROOT, "docs/climate.js"));
const { geoCell } = require(path.join(ROOT, "docs/geo/geodata.js"));
const H = require(path.join(ROOT, "docs/holon.js"));
const idx = JSON.parse(fs.readFileSync(path.join(__dirname, "index.json"), "utf8")).tables.appearance0;
const meta = JSON.parse(fs.readFileSync(path.join(__dirname, "appearance0.json"), "utf8"));
const FILE = "docs/geo/holon/appearance0/koppen_1991_2020_cell.bin.gz";
const sha = (b) => crypto.createHash("sha256").update(b).digest("hex");
const grid = zlib.gunzipSync(fs.readFileSync(path.join(ROOT, FILE)));
// CIRISHolon APPEARANCE0_PREREG.md (frozen at ae50633) R1: the places and coordinates; appearance0/results.json at bfab1d8:
// the cell and class each read. Copied here as published.
const R1 = [
  ["Singapore", 1.35, 103.82, 2044190, "Af"], ["Manaus", -3.10, -60.02, 2143679, "Af"], ["Cairo", 30.04, 31.24, 1381209, "BWh"],
  ["Riyadh", 24.71, 46.68, 1505173, "BWh"], ["Phoenix", 33.45, -112.07, 1302303, "BWh"], ["Madrid", 40.42, -3.70, 1141890, "BSk"],
  ["Rome", 41.90, 12.50, 1107460, "Csa"], ["London", 51.51, -0.13, 885598, "Cfb"], ["Sydney", -33.87, 151.21, 2853849, "Cfa"],
  ["Moscow", 55.75, 37.62, 790860, "Dfb"], ["Yakutsk", 62.03, 129.73, 644717, "Dfc"], ["Nuuk", 64.18, -51.72, 594306, "ET"],
];

test("the shipped grids are the sidecar's, packed and unpacked", () => {
  for (const [p, e] of Object.entries(idx.binary_assets)){
    const packed = fs.readFileSync(path.join(ROOT, p)), raw = zlib.gunzipSync(packed);
    assert.strictEqual(packed.length, e.bytes_packed); assert.strictEqual(sha(packed), e.sha256_packed, p + " packed");
    assert.strictEqual(raw.length, 2880 * 1440); assert.strictEqual(sha(raw), e.sha256_unpacked, p + " unpacked");
    const f = meta.data.files[path.basename(p)]; assert.strictEqual(f.sha256_unpacked, e.sha256_unpacked);
  }
  assert.ok(grid.every((b) => b <= 30));
  // the sidecar's counts by class are the grid's
  const n = new Array(31).fill(0); for (const b of grid) n[b]++;
  for (const [sym, c] of Object.entries(meta.data.cells_by_class)) assert.strictEqual(n[CLIMATE_SYMBOLS.indexOf(sym) + 1], c, sym);
  assert.deepStrictEqual(Object.values(meta.data.legend).map((e) => e.symbol), CLIMATE_SYMBOLS);
  for (let c = 1; c <= 30; c++) assert.ok(climateColour(c).every((v) => v >= 0 && v <= 255));
});

test("CIRISHolon's twelve places read their class through the page's lookup", () => {
  for (const [name, la, lo, cell, cls] of R1){
    const [r, c] = geoCell(la, lo, 1);
    assert.strictEqual(r * 2880 + c, cell, name + ": the twin's cell is Holon's");
    assert.strictEqual(CLIMATE_SYMBOLS[climateCode(grid, la, lo) - 1], cls, name);
  }
});

async function kernel(){
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(path.join(ROOT, "docs/kernel/kernel_scale.wasm")), {});
  const w = instance.exports;
  const put = (bytes) => { const p = w.door_stage(Math.max(65536, bytes.length)) >>> 0; new Uint8Array(w.memory.buffer, p, bytes.length).set(bytes); return bytes.length; };
  const load = (bytes, shaHex) => {
    const rc = w.door_begin(put(new TextEncoder().encode(`climate\nAPPEARANCE-0\n${meta.version}\n${shaHex}`))); if (rc < 0) return rc;
    for (let at = 0; at < bytes.length; at += 65536){ const n = w.door_feed(put(bytes.subarray(at, Math.min(bytes.length, at + 65536)))); if (n < 0) return n; }
    return w.door_finish();
  };
  return { w, load };
}

test("the kernel refuses a grid whose sha256 differs and keeps the slot empty", async () => {
  const { w, load } = await kernel();
  const bad = Uint8Array.from(grid); bad[123456] ^= 1;
  assert.strictEqual(load(bad, idx.binary_assets[FILE].sha256_unpacked), -5);
  w.door_versions(); assert.strictEqual(new Float64Array(w.memory.buffer, w.holon_buf_ptr(0) >>> 0, 5)[0], 0);
  assert.strictEqual(w.door_climate_at(51.51, -0.13), 0);
});

test("the kernel's lookup is the page's: twelve places, a sample of cells, and every leaf key", async () => {
  const { w, load } = await kernel();
  assert.strictEqual(load(grid, idx.binary_assets[FILE].sha256_unpacked), 0);
  w.door_versions(); assert.strictEqual(new Float64Array(w.memory.buffer, w.holon_buf_ptr(0) >>> 0, 5)[0], meta.version);
  for (const [name, la, lo, , cls] of R1) assert.strictEqual(CLIMATE_SYMBOLS[w.door_climate_at(la, lo) - 1], cls, name);
  let n = 0, bad = 0;
  for (let k = 0; k < 2880 * 1440; k += 211){ const r = Math.floor(k / 2880), c = k % 2880;
    for (const [fy, fx] of [[0.5, 0.5], [0.01, 0.99], [0.97, 0.03]]){ const la = 90 - (r + fy) * 0.125, lo = -180 + (c + fx) * 0.125;
      n++; if (w.door_climate_at(la, lo) !== climateCode(grid, la, lo)) bad++; } }
  assert.ok(n > 50000); assert.strictEqual(bad, 0, `${bad} of ${n} points differ`);
  // a leaf keys on its L3 tile's centre cell: the kernel's key against the page's tree geometry
  let leaves = 0;
  for (let i = 0; i < 4000; i++){ const la = -55 + ((i * 7919) % 13500) / 100, lo = -180 + ((i * 104729) % 36000) / 100, lv = 3 + (i % 5);
    const c = H.at(lv, la, lo), t = H.ancestor(c, 3), b = H.boundsDeg(t), cla = (b[0] + b[2]) / 2, clo = (b[1] + b[3]) / 2;
    assert.strictEqual(w.door_leaf_key(H.pack(c)), 8);
    const o = new Float64Array(w.memory.buffer, w.holon_buf_ptr(0) >>> 0, 4), [r, col] = geoCell(cla, clo, 1);
    assert.deepStrictEqual([o[0], o[2], o[3]], [climateCode(grid, cla, clo), r, col], `leaf ${H.pack(c)}`); leaves++; }
  assert.strictEqual(leaves, 4000);
});
