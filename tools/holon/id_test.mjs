// Checks docs/holon.js against the vectors crates/cewp-sim/src/tiling.rs also checks
// (id_vectors_match_the_page). Run: node tools/holon/id_test.mjs [--write]
import { readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
const here = new URL(".", import.meta.url).pathname;
const H = createRequire(import.meta.url)(here + "../../docs/holon.js");
const COORDS = [[0, 0], [-0.001, -0.001], [0.02, 0.02], [-0.02, -0.02], [47.3769, 8.5417], [-33.8688, 151.2093],
  [10, 180], [10, -180], [10, 179.99999], [-10, -179.99999], [89.9999, 0.5], [-90, 12], [64.1466, -21.9426], [1.3521, 103.8198],
  [-0.0000001, 179.9999999], [35.6762, 139.6503], [-54.8, -68.3], [0.3, 0.06]];
const vec = [];
for (const [lat, lon] of COORDS) for (const l of [0, 1, 2, 3, 4, 7, 12]) vec.push([l, lat, lon, H.pack(H.at(l, lat, lon))]);
const file = here + "id_vectors.tsv";
if (process.argv.includes("--write")){
  writeFileSync(file, "# level\tlat\tlon\tpacked id (docs/holon.js = cewp_sim::tiling::HolonId)\n" + vec.map(v => v.join("\t")).join("\n") + "\n");
  console.log("wrote", vec.length);
}
let bad = 0, n = 0;
for (const line of readFileSync(file, "utf8").split("\n")){
  if (!line || line.startsWith("#")) continue;
  const [l, lat, lon, want] = line.split("\t"); n++;
  const c = H.at(+l, +lat, +lon), got = H.pack(c), back = H.unpack(got);
  if (got !== +want || !back || back.i !== c.i || back.j !== c.j){ bad++; console.log("MISMATCH", line, got); }
  for (const k of H.children(c)) if (H.pack(H.parent(k)) !== got){ bad++; console.log("PARENT", line); }
}
// antimeridian: 180E and 180W name one column; east of the last column is the first
const a = H.at(5, 10, 180), b = H.at(5, 10, -180);
if (H.pack(a) !== H.pack(b)){ bad++; console.log("WRAP"); }
const e = H.neighbour(H.at(3, 10, 179.995), 1);
if (H.pack(e) !== H.pack(H.at(3, 10, -179.995))){ bad++; console.log("EAST"); }
console.log(bad ? `FAIL ${bad} of ${n}` : `ok ${n} vectors`);
process.exit(bad ? 1 : 0);
