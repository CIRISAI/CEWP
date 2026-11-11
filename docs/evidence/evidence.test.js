// node --test docs/evidence/evidence.test.js
// The browser-side half of the ledger check (the full scan is `cargo test -p cewp-evidence page_`):
// the ledger script the page loads parses, every claim the page names resolves, every PROJECTED
// entry carries an interval, every FRONTIER entry a blocker.
const test = require("node:test");
const assert = require("node:assert");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");

const dir = __dirname;
const sandbox = { window: {} };
vm.runInNewContext(fs.readFileSync(path.join(dir, "claims.js"), "utf8"), sandbox);
const L = sandbox.window.CEWP_LEDGER;
const json = JSON.parse(fs.readFileSync(path.join(dir, "claims.json"), "utf8"));

test("claims.js and claims.json carry the same ledger", () => {
  assert.deepStrictEqual(JSON.parse(JSON.stringify(L)), json);
});

test("every entry is well formed for its class", () => {
  const classes = new Set(L.classes.map(c => c.class));
  const ids = new Set();
  for (const c of L.claims){
    assert.ok(!ids.has(c.id), "duplicate " + c.id); ids.add(c.id);
    assert.ok(classes.has(c.class), c.id + ": unknown class " + c.class);
    assert.ok(c.source && c.shows && c.failure.length, c.id + ": source, shows and failure are required");
    if (c.class === "PROJECTED") assert.ok(c.interval && c.interval.lo <= c.interval.hi, c.id + ": PROJECTED needs an interval");
    if (c.class === "FRONTIER") assert.ok(c.blocker, c.id + ": FRONTIER needs a blocker");
    if (c.class === "MEASURED") assert.ok(c.window, c.id + ": MEASURED needs a window");
    if (c.status === "under_correction") assert.ok(c.correction, c.id + ": say what is being corrected");
  }
});

test("every claim the page names resolves", () => {
  const ids = new Set(L.claims.map(c => c.id));
  for (const f of ["../globe.js", "evidence.js"]){
    const src = fs.readFileSync(path.join(dir, f), "utf8");
    for (const m of src.matchAll(/(?:data-claim="|[^\w.$]fig\(")([^"$]+)"/g)) assert.ok(ids.has(m[1]), f + ": no ledger entry for " + m[1]);
  }
});
