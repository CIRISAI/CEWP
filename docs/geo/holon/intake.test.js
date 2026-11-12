// The intake path (FSD/twin/holon-intake.md section 1): every table in docs/geo/holon/index.json has
// its asset with the recorded sha256 and a ledger entry carrying its notice, and every holon-* ledger
// entry names a table in the index.
"use strict";
const test = require("node:test"), assert = require("node:assert"), fs = require("node:fs"), path = require("node:path"), crypto = require("node:crypto");
const ROOT = path.join(__dirname, "..", "..", "..");
test("index, assets and ledger agree", () => {
  const idx = JSON.parse(fs.readFileSync(path.join(__dirname, "index.json"), "utf8")).tables;
  const claims = JSON.parse(fs.readFileSync(path.join(ROOT, "docs/evidence/claims.json"), "utf8"));
  const list = Array.isArray(claims) ? claims : claims.claims || Object.values(claims);
  const byId = new Map(list.map((c) => [c.id, c]));
  for (const [ref, e] of Object.entries(idx)){
    const bytes = fs.readFileSync(path.join(ROOT, e.asset));
    assert.strictEqual(crypto.createHash("sha256").update(bytes).digest("hex"), e.asset_sha256, `${ref}: asset sha256`);
    const a = JSON.parse(bytes);
    assert.strictEqual(a.holon_commit, e.holon_commit); assert.ok(a.notice && a.notice === e.notice, `${ref}: notice`);
    const c = byId.get(e.ledger); assert.ok(c, `${ref}: no ledger entry ${e.ledger}`);
    assert.ok(String(c.dataset).includes(e.notice), `${ref}: the ledger entry carries the notice`);
    assert.strictEqual(c.class, "MODEL");
  }
  // every ledger entry sourced from an imported table names a table in the index
  for (const c of list) if (/^docs\/geo\/holon\/[a-z0-9_]+\.json, imported/.test(String(c.source))) assert.ok(Object.values(idx).some((e) => e.ledger === c.id), `${c.id} names no table in the index`);
});
test("UNEVEN-0's share vectors are a million each, and the world row stands in below the floor", () => {
  const u = JSON.parse(fs.readFileSync(path.join(__dirname, "uneven0.json"), "utf8")).data;
  const sum = (v) => v.reduce((a, b) => a + b, 0);
  for (const v of [...u.world.tiles, ...u.world.people]) assert.strictEqual(sum(v), 1000000);
  let published = 0;
  for (const r of u.regional) for (const e of r){ if (e.tiles){ published++; assert.strictEqual(sum(e.tiles), 1000000); assert.strictEqual(sum(e.people), 1000000); } else assert.ok(e.cells < 30 && /fewer than 30/.test(e.reason)); }
  assert.strictEqual(published, 35);
});
