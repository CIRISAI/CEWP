// docs/geo/regions.json, the region partition as data (tools/geo/regions.js): current, every page
// cell exactly once, and the table's cell -> locality -> region lookup is the page's own.
"use strict";
const test = require("node:test"), assert = require("node:assert");
const fs = require("node:fs"), path = require("node:path");
const R = require("../../tools/geo/regions.js");
const here = (p) => path.join(__dirname, p);

test("regions.json is current and reproduces the page's lookup", () => {
  const committed = fs.readFileSync(here("regions.json"), "utf8");
  assert.strictEqual(committed, R.render(R.table()), "stale: node tools/geo/regions.js");
  const j = JSON.parse(committed), { G, REG } = R.pageGrid();
  // the page's grid is the GHS grid shipped in popgrid_ghs.js
  const ghs = fs.readFileSync(here("popgrid_ghs.js"), "utf8").match(/POP_GRID_GHS_B64="([^"]+)"/)[1];
  const page = fs.readFileSync(here("../globe.js"), "utf8").match(/const POP_GRID_B64="([^"]+)"/)[1];
  assert.strictEqual(page, ghs, "globe.js POP_GRID_B64 is the GHS grid");
  assert.deepStrictEqual(j.regions, REG); assert.strictEqual(j.regions.length, 9); assert.strictEqual(j.localities, 181);
  assert.strictEqual(j.cells.length, G.nCells);
  const ids = new Set();
  j.cells.forEach(([row, col, id, loc, reg], c) => {
    for (const v of [row, col, id, loc, reg]) assert.ok(Number.isInteger(v) && v >= 0);
    assert.ok(row < 120 && col < 240 && loc < 181 && reg < 9);
    assert.strictEqual(id, row * 240 + col); assert.ok(!ids.has(id), `cell ${id} twice`); ids.add(id);
    // the page's lookup: cell centre -> its locality -> that locality's region
    assert.strictEqual(Math.round(G.cellLat[c] * 100), 9000 - 150 * row - 75);
    assert.strictEqual(Math.round(G.cellLon[c] * 100), -18000 + 150 * col + 75);
    assert.strictEqual(loc, G.cellLoc[c]); assert.strictEqual(reg, G.regionOfMetro[G.cellLoc[c]]);
  });
  // every locality is used and lies in one region
  const regOf = new Map(); for (const [, , , loc, reg] of j.cells){ assert.ok(!regOf.has(loc) || regOf.get(loc) === reg); regOf.set(loc, reg); }
  assert.strictEqual(regOf.size, 181);
});
