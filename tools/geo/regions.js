#!/usr/bin/env node
// Writes docs/geo/regions.json: the region partition of the page's 1.5-degree population grid as
// data (every populated grid cell's row, column, packed id, locality and region), decoded with the
// page's own initPopGrid from docs/globe.js POP_GRID_B64 (the GHS-POP grid of docs/geo/popgrid_ghs.js).
// Usage: node tools/geo/regions.js [--check]   (--check exits 1 if the committed file differs)
"use strict";
const fs = require("node:fs"), path = require("node:path");
const ROOT = path.join(__dirname, "..", "..");
const STEP = 1.5, ROWS = 120, COLS = 240;

// the page's decoder, taken from docs/globe.js itself so the table cannot drift from what the page reads
function pageGrid(){
  const src = fs.readFileSync(path.join(ROOT, "docs/globe.js"), "utf8");
  const b64 = src.match(/const POP_GRID_B64="([^"]+)"/)[1];
  const reg = JSON.parse(src.match(/const REG = (\[[^\]]+\]);/)[1]);
  const i = src.indexOf("function initPopGrid(){"), j = src.indexOf("\n}\n", i) + 2;
  const atob = (s) => Buffer.from(s, "base64").toString("binary");
  const f = new Function("POP_GRID_B64", "atob", "let POP_GRID = null;\n" + src.slice(i, j) + "\nreturn initPopGrid();");
  return { G: f(b64, atob), REG: reg };
}

function table(){
  const { G, REG } = pageGrid();
  const cells = [];
  for (let c = 0; c < G.nCells; c++){
    const latC = Math.round(G.cellLat[c] * 100), lonC = Math.round(G.cellLon[c] * 100); // centidegrees as stored
    const row = Math.round((9000 - latC) / 150 - 0.5), col = Math.round((lonC + 18000) / 150 - 0.5);
    if (9000 - 150 * row - 75 !== latC || -18000 + 150 * col + 75 !== lonC) throw new Error(`cell ${c} centre ${latC},${lonC} is not a 1.5-degree cell centre`);
    const loc = G.cellLoc[c];
    cells.push([row, col, row * COLS + col, loc, G.regionOfMetro[loc]]);
  }
  return {
    schema: "cewp-twin-regions/v1",
    class: "MODEL",
    note: "The partition is MODEL, not data: localities and regions are inherited from the old population grid by nearest old cell (docs/geo/popgrid_ghs.js header), and may be restated when the root population is unified. Cells are the 1.5-degree GHS-POP R2023A cells of at least 20,000 people (the cohort tier).",
    source: "docs/globe.js POP_GRID_B64 decoded by the page's initPopGrid; written by tools/geo/regions.js",
    grid: { cell_deg_x10: 15, rows: ROWS, cols: COLS, origin: "row 0, col 0 is the north-west cell: its north edge at 90 N, its west edge at 180 W; rows run south, columns run east", centre_centideg: "lat = 9000 - 150 * row - 75, lon = -18000 + 150 * col + 75", packed_id: "row * 240 + col" },
    regions: REG,
    localities: G.nLocs,
    columns: ["row", "col", "id", "locality", "region"],
    cells,
  };
}

function render(t){
  const head = JSON.stringify({ ...t, cells: undefined }, null, 1).replace(/\n}$/, "");
  return head + ',\n "cells": [\n' + t.cells.map((c) => "  [" + c.join(",") + "]").join(",\n") + "\n ]\n}\n";
}

if (require.main === module){
  const out = path.join(ROOT, "docs/geo/regions.json"), text = render(table());
  if (process.argv.includes("--check")){ const same = fs.existsSync(out) && fs.readFileSync(out, "utf8") === text; console.log(same ? "regions.json current" : "regions.json stale"); process.exit(same ? 0 : 1); }
  fs.writeFileSync(out, text); console.log(`${table().cells.length} cells -> ${out}`);
}
module.exports = { table, render, pageGrid };
