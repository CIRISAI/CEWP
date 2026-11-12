// The service worker's cache is named by the same commit stamp as the page's AGPL source line
// (tools/page/stamp_source.sh writes both), so a deploy always gets a new cache.
"use strict";
const test = require("node:test"), assert = require("node:assert");
const fs = require("node:fs"), path = require("node:path");
const read = (p) => fs.readFileSync(path.join(__dirname, "..", p), "utf8");
test("sw.js's stamp is the page's stamp, and the cache is named by it", () => {
  const page = read("source.js").match(/CEWP_SOURCE_COMMIT = "([0-9a-f]{40})"/), sw = read("sw.js").match(/^const SW_STAMP = "([0-9a-f]{40})";$/m);
  assert.ok(page && sw, "both stamps present");
  assert.strictEqual(sw[1], page[1], "run tools/page/stamp_source.sh");
  assert.match(read("sw.js"), /const SW_CACHE = "cewp-twin-" \+ SW_STAMP;/);
  for (const html of ["globe.html", "index.html"]) assert.match(read(html), /<script src="source\.js"><\/script>/, `${html} loads the stamp unversioned (network-first)`);
});
