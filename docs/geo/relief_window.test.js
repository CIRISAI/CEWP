// The relief tiles a view asks for (relief.js reliefWindow, shared by the draw and the Zurich preload): every tile the
// view's window overlaps and no tile wholly outside it. The window used to step from its edge in whole tiles and could
// add a tile beyond the view; a film strip of the Story's dive caught it fetching such tiles on demand.
"use strict";
const test = require("node:test"), assert = require("node:assert");
const fs = require("node:fs"), path = require("node:path"), vm = require("node:vm");
const src = fs.readFileSync(path.join(__dirname, "..", "relief.js"), "utf8");
const fn = src.match(/^function reliefWindow[\s\S]*?^}/m);
const ctx = {}; vm.runInNewContext(fn[0] + "\nthis.reliefWindow = reliefWindow;", ctx);
const W = ctx.reliefWindow;
// splitmix32: a fixed sequence, so the cases are the same on every run
function rng(seed){ let s = seed >>> 0; return () => { s = (s + 0x9e3779b9) >>> 0; let z = s; z = Math.imul(z ^ (z >>> 16), 0x85ebca6b) >>> 0; z = Math.imul(z ^ (z >>> 13), 0xc2b2ae35) >>> 0; return ((z ^ (z >>> 16)) >>> 0) / 4294967296; }; }
const view = (z, w, h, mpp, la) => { const ext = Math.min(90, 1.25 * Math.max(w, h) / 2 * mpp / 111000); return { ext, lx: Math.min(180, ext / Math.max(0.15, Math.cos(la * Math.PI / 180))), deg: z === 1 ? 45 : 15 }; };
test("no tile wholly outside the window, and the window's centre, edges and corners are covered", () => {
  const r = rng(7);
  for (let t = 0; t < 5000; t++){
    const z = 1 + (t % 2), la = -85 + r() * 170, lo = -180 + r() * 360, w = 300 + r() * 1500, h = 300 + r() * 1200, mpp = (z === 1 ? 6000 : 1500) * (0.01 + r());
    const { ext, lx, deg } = view(z, w, h, mpp, la), tiles = W(z, w, h, mpp, la, lo), n = 360 / deg;
    assert.strictEqual(new Set(tiles.map(([a, b]) => a + "_" + b)).size, tiles.length, "no tile twice");
    for (const [row, col] of tiles){
      const top = 90 - row * deg, bot = top - deg;
      assert.ok(bot <= Math.min(90, la + ext) + 1e-9 && top >= Math.max(-90, la - ext) - 1e-9, `row ${row} outside ${la - ext}..${la + ext}`);
      assert.ok(col >= 0 && col < n);
      if (lx < 180){ // the column overlaps [lo - lx, lo + lx] on the circle
        const c0 = -180 + col * deg, d = (x) => ((x % 360) + 540) % 360 - 180, mid = d(c0 + deg / 2 - lo);
        assert.ok(Math.abs(mid) <= lx + deg / 2 + 1e-9, `column ${col} outside ${lo - lx}..${lo + lx}`);
      }
    }
    for (const [a, o] of [[la, lo], [la + ext, lo], [la - ext, lo], [la, lo + lx], [la, lo - lx], [la + ext, lo + lx], [la - ext, lo - lx]]){
      const aa = Math.max(-89.999, Math.min(89.999, a)), oo = ((o + 180) % 360 + 360) % 360, row = Math.floor((90 - aa) / deg), col = Math.min(n - 1, Math.floor(oo / deg));
      assert.ok(tiles.some(([x, y]) => x === row && y === col), `point ${a},${o} not covered`);
    }
  }
});
test("the case the strip caught: a 390 px phone over Zurich at 565 m a pixel asks for the row it is in, not the one beyond", () => {
  // the window runs from about 45.4 to 49.2 N: row 2 (45 to 60 N) only; the old window also asked for row 1 (60 to 75 N)
  const tiles = W(2, 390, 600, 565, 47.29, 7.93);
  assert.deepStrictEqual([...new Set(tiles.map(([row]) => row))], [2]);
});
