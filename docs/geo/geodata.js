// Real population and land cover at coarse grain, for the globe (orbit view and ground tint).
// Built by tools/geodata/build_geodata.py from two CC BY 4.0 rasters (see FSD/twin/geodata.md):
//   population: GHS-POP R2023A, epoch 2020 (European Commission, JRC; Schiavina et al. 2023,
//               doi:10.2905/2FF68A52-5B5B-4A22-8F40-C41DA8332CFE);
//   land cover: Copernicus Global Land Service Land Cover 100 m, collection 3, epoch 2019, V3.0.1
//               (Buchhorn et al. 2020, doi:10.5281/zenodo.3939050). Contains modified Copernicus
//               Service information 2020.
// Grid: equal-angle, cell-centred, row 0 at the north; cell (r, c) covers latitudes 90-(r+1)s ..
// 90-r*s and longitudes -180+c*s .. -180+(c+1)*s. Level 0: s = 1/2 degree (720 x 360); level 1:
// s = 1/8 degree (2880 x 1440, about 14 km at the equator). Files under geo/gd/, gzip, row-major:
//   pop0.bin.gz  float32 LE people per level-0 cell (the conserved totals);
//   pop1.bin.gz  uint8 code per level-1 cell: 0 = under 0.1 person, k >= 1 stands for
//                10^(-1 + (k - 0.5) * 8 / 255) people; each 4 x 4 block is rescaled here to sum to
//                its level-0 value, so level 1 adds shape and never changes a total;
//   lc0.bin.gz, lc1.bin.gz  uint8 LC100 class code of the cell's majority LAND class (codes 0 and
//                200 left out of the vote); 200 = no land pixel (sea); 0 = no input data;
//                255 = outside the product (it covers about 80 N to 60 S: no Antarctica).
// Nodata rule: populationAt answers 0 people where the source has none; landcoverAt answers null
// for 0 and 255, and the caller keeps its own rule there.
// Real data stops here: nothing below 1/8 degree is read from these files. Every street, building,
// household and device the page draws below this grain is generated, never placed on a real one.
// No request leaves the page's own origin: only geo/gd/ is fetched, and only when geoDataBoot runs.
const GEO_LC_LEGEND = {
  // code: [name, official LC100 colour (PUM Table 4), suggested ground colour (display choice, MODEL)]
  0:   ["No input data", [40, 40, 40], null],
  20:  ["Shrubs", [255, 187, 34], [128, 118, 78]],
  30:  ["Herbaceous vegetation", [255, 255, 76], [146, 148, 86]],
  40:  ["Cultivated and managed vegetation / agriculture (cropland)", [240, 150, 255], [168, 160, 100]],
  50:  ["Urban / built up", [250, 0, 0], [128, 124, 120]],
  60:  ["Bare / sparse vegetation", [180, 180, 180], [196, 176, 138]],
  70:  ["Snow and ice", [240, 240, 240], [236, 240, 244]],
  80:  ["Permanent water bodies", [0, 50, 200], [54, 82, 110]],
  90:  ["Herbaceous wetland", [0, 150, 160], [78, 104, 86]],
  100: ["Moss and lichen", [250, 230, 160], [140, 138, 112]],
  111: ["Closed forest, evergreen needle leaf", [88, 72, 31], [46, 70, 46]],
  112: ["Closed forest, evergreen broad leaf", [0, 153, 0], [40, 80, 38]],
  113: ["Closed forest, deciduous needle leaf", [112, 102, 62], [62, 80, 50]],
  114: ["Closed forest, deciduous broad leaf", [0, 204, 0], [56, 90, 44]],
  115: ["Closed forest, mixed", [78, 117, 31], [50, 80, 44]],
  116: ["Closed forest, unknown", [0, 120, 0], [52, 82, 44]],
  121: ["Open forest, evergreen needle leaf", [102, 96, 0], [76, 90, 60]],
  122: ["Open forest, evergreen broad leaf", [141, 180, 0], [82, 104, 56]],
  123: ["Open forest, deciduous needle leaf", [141, 116, 0], [90, 98, 62]],
  124: ["Open forest, deciduous broad leaf", [160, 220, 0], [96, 110, 60]],
  125: ["Open forest, mixed", [146, 153, 0], [88, 104, 58]],
  126: ["Open forest, unknown", [100, 140, 0], [90, 106, 58]],
  200: ["Open sea", [0, 0, 128], [24, 44, 72]],
};
const GEODATA = {
  base: "geo/gd/", v: 1, index: null, pop: [null, null], lc: [null, null], pending: new Map(), errors: [], pop1code: null,
  NX: [720, 2880], NY: [360, 1440], STEP: [0.5, 0.125],
};
async function geoFetchGz(path){
  const r = await fetch(GEODATA.base + path + "?v=" + GEODATA.v);
  if (!r.ok) throw new Error(path + " " + r.status);
  const buf = new Uint8Array(await r.arrayBuffer());
  // a host that already undid the gzip (Content-Encoding) hands back the raw array
  if (buf[0] !== 0x1f || buf[1] !== 0x8b) return buf.buffer;
  if (typeof DecompressionStream === "undefined") throw new Error("no DecompressionStream: geodata off");
  const s = new Blob([buf]).stream().pipeThrough(new DecompressionStream("gzip"));
  return await new Response(s).arrayBuffer();
}
function geoLoad(key, level){
  const k = key + level; if (GEODATA.pending.has(k)) return GEODATA.pending.get(k);
  const p = geoFetchGz(k + ".bin.gz").then(ab => {
    const n = GEODATA.NX[level] * GEODATA.NY[level];
    if (key === "lc"){ const a = new Uint8Array(ab); if (a.length !== n) throw new Error(k + " size"); GEODATA.lc[level] = a; return; }
    if (level === 0){ const a = new Float32Array(ab); if (a.length !== n) throw new Error(k + " size"); GEODATA.pop[0] = a; }
    else { const a = new Uint8Array(ab); if (a.length !== n) throw new Error(k + " size"); GEODATA.pop1code = a; }
    geoPopFine();
  }).catch(e => { GEODATA.errors.push(e.message); console.warn("geodata " + e.message); });
  GEODATA.pending.set(k, p); return p;
}
// level 1 people: each code's value, each 4 x 4 block rescaled to its level-0 total (conserved)
function geoPopFine(){
  const p0 = GEODATA.pop[0], c = GEODATA.pop1code; if (!p0 || !c || GEODATA.pop[1]) return;
  const NX1 = 2880, NX0 = 720, NY0 = 360, out = new Float32Array(2880 * 1440);
  const val = new Float64Array(256); for (let k = 1; k < 256; k++) val[k] = Math.pow(10, -1 + (k - 0.5) * 8 / 255);
  for (let R = 0; R < NY0; R++) for (let C = 0; C < NX0; C++){
    const tot = p0[R * NX0 + C]; if (!(tot > 0)) continue;
    let s = 0; for (let i = 0; i < 4; i++) for (let j = 0; j < 4; j++) s += val[c[(R * 4 + i) * NX1 + C * 4 + j]];
    for (let i = 0; i < 4; i++) for (let j = 0; j < 4; j++){
      const q = (R * 4 + i) * NX1 + C * 4 + j;
      out[q] = s > 0 ? tot * val[c[q]] / s : tot / 16;   // all 16 under 0.1 person: spread evenly
    }
  }
  GEODATA.pop[1] = out; GEODATA.pop1code = null;
}
// Load level 0 of both rasters (about 0.3 MB); geoDataBoot(1) adds level 1 (about 1 MB more).
// Returns a promise that settles when the asked levels are in (or failed: GEODATA.errors).
function geoDataBoot(level = 0){
  const ps = [];
  for (let l = 0; l <= Math.min(1, level); l++){ ps.push(geoLoad("pop", l)); if (l === 1) ps.push(geoLoad("pop", 0)); ps.push(geoLoad("lc", l)); }
  if (!GEODATA.pending.has("index")){
    const p = fetch(GEODATA.base + "index.json?v=" + GEODATA.v).then(r => r.json()).then(j => { GEODATA.index = j; }).catch(e => { GEODATA.errors.push(e.message); });
    GEODATA.pending.set("index", p); ps.push(p);
  }
  return Promise.all(ps);
}
function geoCell(lat, lon, level){
  const s = GEODATA.STEP[level], nx = GEODATA.NX[level], ny = GEODATA.NY[level];
  let c = Math.floor((((lon + 180) % 360 + 360) % 360) / s); if (c >= nx) c = nx - 1;
  let r = Math.floor((90 - lat) / s); r = Math.max(0, Math.min(ny - 1, r));
  return [r, c];
}
// km² of a cell in row r at a level (sphere of radius 6371.0088 km)
function geoCellKm2(r, level){
  const s = GEODATA.STEP[level], R = 6371.0088, d = Math.PI / 180;
  const n = 90 - r * s, so = 90 - (r + 1) * s;
  return R * R * s * d * (Math.sin(n * d) - Math.sin(so * d));
}
// {people, perKm2, level, cellDeg} of the finest loaded cell holding the point, or null before
// level 0 is in. `maxLevel` 0 asks for the orbit cell even when level 1 is loaded.
function geoPopulationAt(lat, lon, maxLevel = 1){
  for (let l = Math.min(1, maxLevel); l >= 0; l--){
    const a = GEODATA.pop[l]; if (!a) continue;
    const [r, c] = geoCell(lat, lon, l), people = a[r * GEODATA.NX[l] + c];
    return { people, perKm2: people / geoCellKm2(r, l), level: l, cellDeg: GEODATA.STEP[l] };
  }
  return null;
}
// {code, name, colour, ground, level} of the finest loaded cell holding the point, or null where
// the product has no answer (code 0, outside the product, or not loaded): the caller's own rule holds.
function geoLandcoverAt(lat, lon, maxLevel = 1){
  for (let l = Math.min(1, maxLevel); l >= 0; l--){
    const a = GEODATA.lc[l]; if (!a) continue;
    const [r, c] = geoCell(lat, lon, l), code = a[r * GEODATA.NX[l] + c], e = GEO_LC_LEGEND[code];
    if (!e || code === 0) return null;
    return { code, name: e[0], colour: e[1], ground: e[2], level: l };
  }
  return null;
}
// Attribution the page must show wherever these layers are drawn (CC BY 4.0 for both).
const GEO_ATTRIBUTION = "Population: GHS-POP R2023A (epoch 2020), © European Union 1995-2026, JRC, CC BY 4.0 (Schiavina et al. 2023, doi:10.2905/2FF68A52-5B5B-4A22-8F40-C41DA8332CFE), aggregated to 1/2° and 1/8° cells. " +
  "Land cover: Copernicus Global Land Service Land Cover 100 m, epoch 2019 (Buchhorn et al. 2020, doi:10.5281/zenodo.3939050), CC BY 4.0; contains modified Copernicus Service information 2019, reduced to the majority land class per 1/2° and 1/8° cell.";
if (typeof module !== "undefined") module.exports = { GEODATA, GEO_LC_LEGEND, GEO_ATTRIBUTION, geoDataBoot, geoPopulationAt, geoLandcoverAt, geoCell, geoCellKm2, geoPopFine };
