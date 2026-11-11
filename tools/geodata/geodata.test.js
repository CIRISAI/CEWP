// node --test tools/geodata/geodata.test.js
// The committed docs/geo/gd/ assets read through docs/geo/geodata.js, as the page reads them.
// The expected totals are the build's own (docs/geo/gd/index.json) and the JRC's published 2020
// world total (GHSL Data Package 2023, Table 20: 7,840,952,947 on the 1 km Mollweide grid).
"use strict";
const test = require("node:test");
const assert = require("node:assert");
const fs = require("node:fs");
const path = require("node:path");
const DOCS = path.join(__dirname, "..", "..", "docs");

global.fetch = async (url) => {
  const p = path.join(DOCS, url.split("?")[0]);
  if (!fs.existsSync(p)) return { ok: false, status: 404 };
  const b = fs.readFileSync(p);
  return { ok: true, status: 200, arrayBuffer: async () => b.buffer.slice(b.byteOffset, b.byteOffset + b.length), json: async () => JSON.parse(b.toString("utf8")) };
};
const G = require(path.join(DOCS, "geo", "geodata.js"));

test("level 0 conserves the GHS-POP 2020 world total", async () => {
  await G.geoDataBoot(0);
  assert.deepStrictEqual(G.GEODATA.errors, []);
  let s = 0; for (const v of G.GEODATA.pop[0]) s += v;
  const idx = G.GEODATA.index.population.totals;
  assert.ok(Math.abs(idx.source_total_float64 - 7840952947) < 1, "build read the JRC total");
  assert.ok(Math.abs(s - idx.source_total_float64) / idx.source_total_float64 < 1e-8, `level 0 sums to ${s}`);
});

test("level 1 blocks sum to their level-0 cell", async () => {
  await G.geoDataBoot(1);
  assert.deepStrictEqual(G.GEODATA.errors, []);
  const p0 = G.GEODATA.pop[0], p1 = G.GEODATA.pop[1];
  let s1 = 0, worst = 0;
  for (let R = 0; R < 360; R++) for (let C = 0; C < 720; C++){
    let b = 0; for (let i = 0; i < 4; i++) for (let j = 0; j < 4; j++) b += p1[(R * 4 + i) * 2880 + C * 4 + j];
    s1 += b; const t = p0[R * 720 + C]; if (t > 1000) worst = Math.max(worst, Math.abs(b - t) / t);
  }
  assert.ok(worst < 1e-5, `worst block error ${worst}`);
  assert.ok(Math.abs(s1 - 7840952947) / 7840952947 < 1e-6, `level 1 sums to ${s1}`);
});

test("population at a few places", async () => {
  await G.geoDataBoot(1);
  const dhaka = G.geoPopulationAt(23.81, 90.41), sea = G.geoPopulationAt(-30, -140), sahara = G.geoPopulationAt(23.0, 12.0);
  assert.strictEqual(dhaka.level, 1);
  assert.ok(dhaka.perKm2 > 2000, `Dhaka ${dhaka.perKm2}/km2`);
  assert.strictEqual(sea.people, 0);
  assert.ok(sahara.perKm2 < 1, `Sahara ${sahara.perKm2}/km2`);
  const o = G.geoPopulationAt(23.81, 90.41, 0); assert.strictEqual(o.level, 0);
  // a whole sphere of cells adds up to 4 pi R^2
  let a = 0; for (let r = 0; r < 360; r++) a += 720 * G.geoCellKm2(r, 0);
  assert.ok(Math.abs(a - 4 * Math.PI * 6371.0088 ** 2) < 1);
});

test("land cover at a few places", async () => {
  await G.geoDataBoot(1);
  assert.deepStrictEqual(G.GEODATA.errors, []);
  const at = (la, lo) => { const r = G.geoLandcoverAt(la, lo); return r && r.code; };
  assert.strictEqual(at(-30, -140), 200, "South Pacific is sea");
  assert.strictEqual(at(23.0, 12.0), 60, "central Sahara is bare");
  assert.ok([112, 116].includes(at(-3.5, -62.0)), "central Amazon is closed evergreen broadleaf forest: " + at(-3.5, -62.0));
  assert.strictEqual(at(72.5, -40.0), 70, "Greenland ice sheet");
  assert.strictEqual(at(-80, 0), null, "Antarctica is outside the product");
  for (const code of Object.keys(G.GEO_LC_LEGEND)) assert.strictEqual(G.GEO_LC_LEGEND[code].length, 3);
  // every code in the rasters has a legend entry, or is 255 (outside)
  for (const a of G.GEODATA.lc) for (const v of new Set(a)) assert.ok(v === 255 || G.GEO_LC_LEGEND[v], "code " + v);
});

test("the POP_GRID drop-in decodes in initPopGrid's layout", () => {
  const src = fs.readFileSync(path.join(DOCS, "geo", "popgrid_ghs.js"), "utf8");
  const b = Buffer.from(/const POP_GRID_GHS_B64="([^"]+)"/.exec(src)[1], "base64");
  const nC = b.readUInt16LE(0), nL = b.readUInt16LE(2);
  let off = 4 + nL; if (off % 2) off++;
  assert.strictEqual(b.length, off + 8 * nC);
  assert.strictEqual(nL, 181);
  let w = 0;
  for (let c = 0; c < nC; c++, off += 8){
    const la = b.readInt16LE(off) / 100, lo = b.readInt16LE(off + 2) / 100, loc = b[off + 6], meta = b[off + 7];
    assert.ok(Math.abs(la) < 90 && Math.abs(lo) < 180 && loc < nL && (meta & 15) === 12 && (meta >> 4) === b[4 + loc]);
    w += b.readUInt16LE(off + 4);
  }
  // weights are thousands of people: within rounding of the people in the kept cells
  assert.ok(w * 1000 > 7.8e9 && w * 1000 < 7.85e9, "weights x 1000 = " + w * 1000);
});
