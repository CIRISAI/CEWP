// Service worker for return visits: the twin's assets carry versioned URLs (?v=N), so a cached copy
// of a versioned URL is always the right file. Versioned and geo/kernel assets: cache first; pages
// and anything else: network first, cache as fallback. No cross-origin request is touched.
const SW_CACHE = "cewp-twin-v1";
self.addEventListener("install", (e) => { self.skipWaiting(); });
self.addEventListener("activate", (e) => { e.waitUntil(caches.keys().then((ks) => Promise.all(ks.filter((k) => k !== SW_CACHE).map((k) => caches.delete(k)))).then(() => self.clients.claim())); });
self.addEventListener("fetch", (e) => {
  const r = e.request, u = new URL(r.url);
  if (r.method !== "GET" || u.origin !== self.location.origin) return;
  const stable = u.search.includes("v=") || /\/(geo|kernel)\//.test(u.pathname);
  if (stable){
    e.respondWith(caches.open(SW_CACHE).then((c) => c.match(r).then((hit) => hit || fetch(r).then((res) => { if (res.ok) c.put(r, res.clone()); return res; }))));
  } else {
    e.respondWith(fetch(r).then((res) => { if (res.ok){ const cp = res.clone(); caches.open(SW_CACHE).then((c) => c.put(r, cp)); } return res; }).catch(() => caches.match(r)));
  }
});
