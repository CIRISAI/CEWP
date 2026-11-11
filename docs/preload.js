// The Zurich path, ready before the zoom (Eric: "pre-cache all the Zurich tiles we need so the zoom
// has no loading"). The Story and the default zoom go from orbit to a Zurich street. At idle after
// first paint this file asks, once, for exactly what that path draws, through the page's own
// loaders (so the results are decoded and on the GPU when the camera arrives), then builds the
// Zurich street tiling in the wasm in slices under ZP_SLICE_MS of idle time. Nothing else on the
// globe is prefetched. A service worker (sw.js) keeps the versioned files for a return visit.
"use strict";
const ZP = { lat: 47.3769, lon: 8.5417, started: false, net: false, built: 0, leaves: 0, done: false, t0: 0, log: [], bytes: 0 };
const ZP_SLICE_MS = 8;       // longest idle slice of wasm work
const ZP_RADIUS_DEG = 0.012; // street tiling prebuilt around the centre (about 1.3 km across)
const zpIdle = (fn, timeout) => (typeof requestIdleCallback === "function") ? requestIdleCallback(fn, { timeout: timeout || 2000 }) : setTimeout(() => fn({ timeRemaining: () => ZP_SLICE_MS, didTimeout: true }), 50);
function zpNote(what){ ZP.log.push([Math.round(performance.now()), what]); }

// ne10 tiles the zoom shows at and below GEO_MPP_10 * 2 around Zurich, for this viewport
function zpNe10Keys(){
  const w = cv.clientWidth || 1440, h = cv.clientHeight || 900, mpp = GEO_MPP_10 * 2;
  const ext = Math.min(60, 1.25 * Math.max(w, h) / 2 * mpp / 111000), lx = Math.min(180, ext / Math.max(0.15, Math.cos(ZP.lat * Math.PI / 180)));
  const out = new Set();
  for (let a = Math.floor((ZP.lat - ext) / 10) * 10; a < ZP.lat + ext; a += 10){ if (a < -90 || a >= 90) continue;
    for (let o = Math.floor((ZP.lon - lx) / 10) * 10; o < ZP.lon + lx; o += 10) out.add(a + "_" + (((o + 180) % 360 + 360) % 360 - 180)); }
  return [...out];
}
// relief tiles of level z the zoom shows around Zurich (the same window reliefVisible draws, at the
// coarsest metres a pixel the level is drawn at)
function zpReliefKeys(z){
  const w = cv.clientWidth || 1440, h = cv.clientHeight || 900, mpp = z === 1 ? 6000 : 1500, deg = z === 1 ? 45 : 15;
  const ext = Math.min(90, 1.25 * Math.max(w, h) / 2 * mpp / 111000), lx = Math.min(180, ext / Math.max(0.15, Math.cos(ZP.lat * Math.PI / 180)));
  const out = new Set();
  for (let a = ZP.lat - ext; a < ZP.lat + ext + deg; a += deg){ const r = Math.floor((90 - Math.max(-89.999, Math.min(89.999, a))) / deg);
    for (let o = ZP.lon - lx; o < ZP.lon + lx + deg; o += deg) out.add(r + "_" + Math.floor((((o + 180) % 360 + 360) % 360) / deg)); }
  return [...out].map(k => k.split("_").map(Number));
}
// network: once, low priority, through the page's loaders
function zpNetwork(){
  if (ZP.net) return; ZP.net = true; zpNote("net start");
  geoLoad50(); geoLoadPlaces();                                   // coast, lakes, places (the tiling stands on them)
  if (typeof geoLoad === "function"){ geoLoad("pop", 0); geoLoad("pop", 1); geoLoad("lc", 1); } // people and land cover at 1/8°
  if (typeof reliefBoot === "function") reliefBoot();
  if (!GEO.pIdx){ GEO.pIdx = true; fetch("geo/ne10/index.json?v=1").then(r => r.json()).then(j => { GEO.idx10 = new Set(j.map(t => t[0] + "_" + t[1])); }).catch(e => console.warn(e.message)); }
  const later = () => {
    if (!(GEO.idx10 && GEO.lakes && RELIEF.idx)){ setTimeout(later, 400); return; }
    for (const k of zpNe10Keys()) if (GEO.idx10.has(k) && typeof geoWant10 === "function") geoWant10(k);
    for (const z of [1, 2]) for (const [r, c] of zpReliefKeys(z)) if (RELIEF.idx[z].has(r + "_" + c)) reliefLoad(z, r, c);
    zpNote("net asked");
  };
  later();
}
// CPU: the Zurich street tiling (tree budgets, leaf layouts, arterials) in idle slices
function zpBuild(dl){
  if (ZP.done) return;
  if (!(typeof tlReady === "function" && tlReady())){ zpIdle(zpBuild, 1500); return; }
  if (!ZP.queue){ // the leaves under a lattice around the centre, nearest first
    const T = tlTree(), q = [], seen = new Set(), n = 15, st = 2 * ZP_RADIUS_DEG / n, cl = Math.cos(ZP.lat * Math.PI / 180);
    for (let a = 0; a <= n; a++) for (let b = 0; b <= n; b++){ const la = ZP.lat - ZP_RADIUS_DEG + a * st, lo = ZP.lon - ZP_RADIUS_DEG / cl + b * st / cl; q.push([Math.hypot(la - ZP.lat, (lo - ZP.lon) * cl), la, lo]); }
    q.sort((x, y) => x[0] - y[0]); ZP.queue = q; ZP.seen = seen; zpNote("build start"); }
  const t0 = performance.now(), T = tlTree();
  while (ZP.queue.length && performance.now() - t0 < ZP_SLICE_MS && (!dl || dl.didTimeout || dl.timeRemaining() > 1)){
    const [, la, lo] = ZP.queue.shift(), leaf = T.leafAt(la, lo), k = HOLON.pack(leaf);
    if (ZP.seen.has(k)) continue; ZP.seen.add(k); ZP.leaves++;
    if (!TL.lay.has(k) && T.budget(leaf) > 0){ const e = tlBuildLeaf(leaf); TL.lay.set(k, e); TL.bytes += e.bytes; ZP.built++; }
    const t2 = HOLON.ancestor(leaf, 2), k2 = HOLON.pack(t2); if (!TL.mains.has(k2)) tlArterials(t2);
  }
  if (ZP.queue.length){ zpIdle(zpBuild, 1000); return; }
  ZP.done = true; zpNote("build done " + ZP.built + " leaves");
}
function zpStart(){
  if (ZP.started) return; ZP.started = true; ZP.t0 = performance.now();
  zpIdle(() => { zpNetwork(); zpIdle(zpBuild, 3000); }, 1500);
}
// after the first painted frames
if (document.readyState === "complete") setTimeout(zpStart, 800); else window.addEventListener("load", () => setTimeout(zpStart, 800));
// return visits: the service worker keeps the versioned assets (sw.js)
if ("serviceWorker" in navigator && location.protocol !== "file:") window.addEventListener("load", () => { navigator.serviceWorker.register("sw.js").catch((e) => console.info("no service worker: " + e.message)); });
