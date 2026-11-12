// Service worker for return visits. Staleness is bounded by construction:
// * the cache is named by the commit the page runs (SW_STAMP, written with docs/source.js by
//   tools/page/stamp_source.sh), so every deploy changes this file's bytes, the browser installs the
//   new worker, and its activate handler deletes every other cache (including the old
//   "cewp-twin-v1");
// * cache-first only inside that per-commit cache, and only for versioned URLs (?v=) and the
//   geo/ and kernel/ assets;
// * pages (navigations) and source.js (the stamp itself) are network-first, the cache a fallback
//   offline; anything else is network-first too. No cross-origin request is touched.
// A visitor sees at most one load of the previous commit's assets after a deploy: the load during
// which the new worker installs.
const SW_STAMP = "4396b3bda87e508c7ed767f4b19dd1ad9ca037cd";
const SW_CACHE = "cewp-twin-" + SW_STAMP;
self.addEventListener("install", () => { self.skipWaiting(); });
self.addEventListener("activate", (e) => {
  e.waitUntil(caches.keys().then((ks) => Promise.all(ks.filter((k) => k !== SW_CACHE).map((k) => caches.delete(k)))).then(() => self.clients.claim()));
});
// fill from the network past the browser's HTTP cache (GitHub Pages sends max-age=600, which could
// otherwise hand a new commit's cache the previous commit's bytes): revalidate, a 304 when unchanged
const net = (r) => (r.mode === "navigate" ? fetch(r) : fetch(r, { cache: "no-cache" }));
self.addEventListener("fetch", (e) => {
  const r = e.request, u = new URL(r.url);
  if (r.method !== "GET" || u.origin !== self.location.origin) return;
  const page = r.mode === "navigate" || /\.html?$/.test(u.pathname) || u.pathname.endsWith("/") || /\/source\.js$/.test(u.pathname);
  const stable = !page && (u.search.includes("v=") || /\/(geo|kernel)\//.test(u.pathname));
  if (stable){
    e.respondWith(caches.open(SW_CACHE).then((c) => c.match(r).then((hit) => hit || net(r).then((res) => { if (res.ok) c.put(r, res.clone()); return res; }))));
  } else {
    e.respondWith(net(r).then((res) => { if (res.ok){ const cp = res.clone(); caches.open(SW_CACHE).then((c) => c.put(r, cp)); } return res; }).catch(() => caches.open(SW_CACHE).then((c) => c.match(r))));
  }
});
