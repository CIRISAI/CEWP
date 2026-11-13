// REGION-0 through the twin (docs/climate.js stateAt / stateRule, kernel door_leaf_key): the shipped grids are the sidecar's,
// CIRISHolon's 50 labelled points read their state through the twin's lookup, the kernel applies the same rules as the page,
// and each hazard REGION-0 names is handled: a coded cell under 13 of 25 points falls back to the country's row, lake water
// with a code is not drawn as a state's land, DC is one cell, US land with no code falls back, and the whole-cell share is
// recoverable from the two grids.
"use strict";
const test = require("node:test"), assert = require("node:assert"), fs = require("node:fs"), path = require("node:path"), crypto = require("node:crypto"), zlib = require("node:zlib");
const ROOT = path.join(__dirname, "..", "..", "..");
const { stateRule, STATE_MIN_POINTS, climateWater } = require(path.join(ROOT, "docs/climate.js"));
const { geoCell } = require(path.join(ROOT, "docs/geo/geodata.js"));
const { countriesDecode, countryNear } = require(path.join(ROOT, "docs/geo/countries.js"));
const H = require(path.join(ROOT, "docs/holon.js"));
const idx = JSON.parse(fs.readFileSync(path.join(__dirname, "index.json"), "utf8")).tables.region0;
const meta = JSON.parse(fs.readFileSync(path.join(__dirname, "region0.json"), "utf8"));
const g = (f) => zlib.gunzipSync(fs.readFileSync(path.join(ROOT, "docs/geo/holon/region0", f)));
const CODE = g("us_state_cell.bin.gz"), SHARE = g("us_state_cell_share.bin.gz"), COUNT = g("us_state_cell_classed.bin.gz");
const cb = fs.readFileSync(path.join(ROOT, "docs/geo/countries.bin")), MASK = countriesDecode(cb.buffer.slice(cb.byteOffset, cb.byteOffset + cb.byteLength));
const LC1 = zlib.gunzipSync(fs.readFileSync(path.join(ROOT, "docs/geo/gd/lc1.bin.gz")));
// the page's lookup (climate.js stateAt) on the arrays: the cell's values, the twin's own US mask and its land cover
function page(lat, lon){ const [r, c] = geoCell(lat, lon, 1), k = r * 2880 + c, lc = LC1[k], land = lc === 0 || lc === 255 ? null : !climateWater(lc);
  return { k, raw: CODE[k], points: COUNT[k], ...stateRule(CODE[k], COUNT[k], countryNear(MASK, lat, lon) === 840, land) }; }
// CIRISHolon REGION0_PREREG.md R1 (frozen before the parse): cell centre, code, cell; copied as published
const R1 = [[43.0625,-76.1875,36,1080830],[34.0625,-118.1875,6,1287854],[41.9375,-87.6875,17,1106658],[29.8125,-95.3125,48,1385957],[33.4375,-112.0625,4,1302303],[39.6875,-104.9375,8,1158360],
  [33.8125,-84.4375,13,1293884],[47.5625,-122.3125,53,976781],[28.5625,-81.4375,12,1414868],[44.9375,-93.3125,27,1037493],[64.8125,-147.6875,2,579138],[19.5625,-155.4375,15,1621636],[43.5625,-116.1875,16,1068990],
  [36.1875,-86.8125,47,1239145],[32.3125,-86.3125,1,1328429],[34.6875,-92.3125,5,1273661],[41.8125,-72.6875,9,1109658],[39.1875,-75.5625,10,1170115],[39.8125,-86.1875,18,1155630],[41.5625,-93.5625,19,1115251],
  [37.6875,-97.3125,20,1204501],[38.0625,-84.4375,21,1195964],[31.3125,-92.4375,22,1351420],[44.3125,-69.8125,23,1052081],[39.3125,-76.5625,24,1167227],[42.3125,-71.8125,25,1098145],[42.6875,-84.5625,26,1089403],
  [32.3125,-90.1875,28,1328398],[38.9375,-92.3125,29,1175741],[45.8125,-108.5625,30,1017211],[40.8125,-96.6875,31,1132506],[40.8125,-115.8125,32,1132353],[43.1875,-71.5625,33,1077987],[40.4375,-74.4375,34,1141324],
  [35.0625,-106.6875,35,1264906],[35.8125,-78.6875,37,1247850],[46.8125,-100.8125,38,994233],[39.9375,-82.9375,39,1152776],[35.4375,-97.5625,40,1256339],[44.0625,-121.3125,41,1057429],[40.8125,-77.8125,42,1132657],
  [41.6875,-71.5625,44,1112547],[34.0625,-81.0625,45,1288151],[44.4375,-100.3125,46,1048957],[40.8125,-111.9375,49,1132384],[44.3125,-72.5625,50,1052059],[37.5625,-77.4375,51,1207540],[38.3125,-81.6875,54,1190226],
  [43.0625,-89.4375,55,1080724],[42.8125,-106.3125,56,1086349]];

test("the shipped grids are the sidecar's, and every code is in its table", () => {
  for (const [p, e] of Object.entries(idx.binary_assets)){ const raw = zlib.gunzipSync(fs.readFileSync(path.join(ROOT, p)));
    assert.strictEqual(crypto.createHash("sha256").update(raw).digest("hex"), e.sha256_unpacked, p); assert.strictEqual(raw.length, 2880 * 1440); }
  const codes = new Set(Object.keys(meta.data.states).map(Number)); assert.strictEqual(codes.size, 51);
  for (const b of new Set(CODE)) assert.ok(b === 0 || codes.has(b), "code " + b);
  assert.ok(COUNT.every((n) => n <= 25));
  assert.ok(Object.values(meta.data.states).every((s) => !s.name.includes("\u0000")), "the published names' NUL padding is dropped");
});

test("CIRISHolon's 50 labelled points read their state through the twin's lookup", () => {
  assert.strictEqual(R1.length, 50);
  for (const [la, lo, code, cell] of R1){ const p = page(la, lo);
    assert.strictEqual(p.k, cell, `${la},${lo}: the twin's cell is Holon's`); assert.strictEqual(p.used, code, `${la},${lo}: ${meta.data.states[code].name} (why ${p.why})`); }
});

test("the hazards: few points fall back, lakes are not land, DC is one cell, uncoded US land falls back, the whole share is recoverable", () => {
  // (1) coded cells under 13 of 25 points: none takes its state
  let few = 0; for (let k = 0; k < CODE.length; k++) if (CODE[k] && COUNT[k] < STATE_MIN_POINTS){ few++; assert.deepStrictEqual(stateRule(CODE[k], COUNT[k], true, true), { used: 0, why: 3 }); }
  assert.strictEqual(few, 2064, "REGION-0's count of coded cells under 13 points");
  // (2) lake water inside a state's outline carries its code (Lake Michigan's middle): the twin's land cover reads water, no state is drawn there
  const lake = page(43.9, -87.0); assert.ok(lake.raw && lake.points >= 13, "a coded lake cell"); assert.ok(climateWater(LC1[lake.k]), "the twin reads water"); assert.deepStrictEqual([lake.used, lake.why], [0, 5]);
  // (3) DC is one cell, and it reads DC
  const dc = []; for (let k = 0; k < CODE.length; k++) if (CODE[k] === 11) dc.push(k); assert.strictEqual(dc.length, 1);
  // (4) US land with no code: Attu, the westernmost Aleutians, inside the twin's US mask
  const attu = page(52.9, 173.1); assert.strictEqual(attu.raw, 0); assert.deepStrictEqual([attu.used, attu.why], [0, 2]);
  // (5) the winner's share of the whole cell, points over 25, recovered from the share and the count within rounding
  for (let k = 0; k < CODE.length; k++) if (CODE[k]){ const pts = SHARE[k] * COUNT[k] / 255; assert.ok(Math.abs(pts - Math.round(pts)) <= 0.5 * COUNT[k] / 255 + 1e-9 && Math.round(pts) <= COUNT[k], `cell ${k}`); }
});

test("the kernel's state per leaf is the page's", async () => {
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(path.join(ROOT, "docs/kernel/kernel_scale.wasm")), {}), w = instance.exports;
  const put = (bytes) => { const p = w.door_stage(Math.max(65536, bytes.length)) >>> 0; new Uint8Array(w.memory.buffer, p, bytes.length).set(bytes); return bytes.length; };
  const load = (name, bytes, file) => { assert.strictEqual(w.door_begin(put(new TextEncoder().encode(`${name}\nREGION-0\n${meta.version}\n${idx.binary_assets["docs/geo/holon/region0/" + file].sha256_unpacked}`))), 0);
    for (let at = 0; at < bytes.length; at += 65536) assert.ok(w.door_feed(put(bytes.subarray(at, Math.min(bytes.length, at + 65536)))) > 0); return w.door_finish(); };
  // before the tables: every leaf reads "no state table"
  w.door_leaf_key(H.pack(H.at(5, 39.95, -82.95))); assert.strictEqual(new Float64Array(w.memory.buffer, w.holon_buf_ptr(0) >>> 0, 8)[7], 1);
  assert.strictEqual(load("state", CODE, "us_state_cell.bin.gz"), 2); assert.strictEqual(load("state_count", COUNT, "us_state_cell_classed.bin.gz"), 3);
  let n = 0, own = 0, few = 0;
  const check = (la, lo, lv) => { const c = H.at(lv, la, lo), t = H.ancestor(c, 3), b = H.boundsDeg(t), cla = (b[0] + b[2]) / 2, clo = (b[1] + b[3]) / 2;
    w.door_leaf_key(H.pack(c)); const o = new Float64Array(w.memory.buffer, w.holon_buf_ptr(0) >>> 0, 8), [r, col] = geoCell(cla, clo, 1), k = r * 2880 + col;
    // a leaf is on land by construction (the kernel does not read land cover), so the page's rule is taken with land true
    const p = stateRule(CODE[k], COUNT[k], countryNear(MASK, cla, clo) === 840, true);
    assert.deepStrictEqual([o[4], o[5], o[6], o[7]], [p.used, CODE[k], COUNT[k], p.why], `leaf at ${cla},${clo}`); n++; if (p.why === 0) own++; if (p.why === 3) few++; };
  for (let i = 0; i < 3000; i++) check(24 + ((i * 7919) % 2600) / 100, -125 + ((i * 104729) % 5900) / 100, 3 + (i % 4));
  // the border cells under the count: a leaf in one falls back in the kernel as on the page
  let border = 0; for (let k = 0; k < CODE.length && border < 40; k += 1) if (CODE[k] && COUNT[k] < STATE_MIN_POINTS && (k % 7 === 0)){ const r = Math.floor(k / 2880), c = k % 2880; check(90 - (r + 0.5) * 0.125, -180 + (c + 0.5) * 0.125, 3); border++; }
  assert.ok(own > 1000 && few >= 20, `${n} leaves, ${own} with their own state, ${few} under the count`);
});
