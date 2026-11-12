# Film strip and witness for the Story's zoom into Zurich: is every tile the zoom shows loaded before the camera needs it?
# Plays the Story from navigation on index.html (the way the page starts, still loading) through the street steps, and records:
#   * a film strip: every painted frame (CDP screencast), kept at 100 ms, drawn as a contact sheet with each frame's time,
#     act and metres a pixel; 100 ms frames from the street step's start to its end, 500 ms before it;
#   * per frame: what is under the focal point (the screen centre): the relief level wanted and held, the coastline set
#     in use against the one the zoom wants, the street tiling's leaf under the centre, the generated street;
#   * per tile: when it was asked for, by what (the preload, preload.js, or a draw that needed it), when its bytes
#     arrived (Resource Timing), when it was first drawn, and where the bytes came from (network, HTTP cache, service worker);
#     street-tiling leaves (built on the CPU) the same way, by the idle prebuild or by a frame.
# Cold: a fresh profile. Warm: the same profile after one earlier visit that played the Story through its street steps
# (service worker and HTTP cache filled), then a new page.
# With --assert (the witness; it holds on cold runs too): inside the street steps, zero tiles fetched on demand and no frame
# with unloaded ground under the focal point (relief level wanted but not held, coast tiles missing, the street tiling off
# at street zoom, the leaf under the centre not laid out, a street step at street zoom with no street); and, while the
# camera is on the street (from settling until the last step's pull-back), zero street-tiling leaves built on demand, and
# the buildings the tiling says it drew show on screen (every 2 s, the lit roof halves of its drawn leaves outside the
# generated street's square differ in colour from the ground around them in at least PAINTED_MIN of the points); WebGL
# context loss, the GL error state and console errors are recorded. --break-detail is the check's own calibration: the
# leaves stay counted as drawn and paint nothing, and the witness must fail.
# Leaves the dive and the pull-back pass over are still laid out as they come into view (counted, not asserted): the
# detail is drawn as a disc of laid-out leaves (tiling.js), so they appear at its faded edge, not as holes.
# Usage: python3 tools/page/story_zoom/run.py [--port 8732] [--w 390 --h 844 --mob 1] [--throttle 1] [--cache warm]
#        [--out DIR] [--assert]
import argparse, asyncio, base64, io, json, os, sys, time
from playwright.async_api import async_playwright

ap = argparse.ArgumentParser()
ap.add_argument("--port", default="8732"); ap.add_argument("--w", type=int, default=390); ap.add_argument("--h", type=int, default=844)
ap.add_argument("--mob", type=int, default=1); ap.add_argument("--throttle", type=float, default=1); ap.add_argument("--cache", default="warm")
ap.add_argument("--out", default=""); ap.add_argument("--break-detail", action="store_true"); ap.add_argument("--assert", dest="check", action="store_true"); ap.add_argument("--max-s", type=float, default=240)
A = ap.parse_args()
URL = f"http://127.0.0.1:{A.port}/index.html"

INIT = r"""(() => {
  Error.stackTraceLimit = 30;
  const Z = window.__Z = { req: [], builds: [], frames: [], open: {}, origin0: performance.timeOrigin };
  // a request stays open until its tile is ready (on the GPU) and then until it is first drawn; a tile asked again
  // (evicted, then wanted) opens a new request
  const ask = (r, k) => { r.ready = null; r.first_drawn = null; Z.req.push(r); Z.open[k] = r; };
  const ready = (k, now) => { const r = Z.open[k]; if (r && r.ready === null) r.ready = Math.round(now); };
  const drawn = (k, now) => { const r = Z.open[k]; if (r && r.ready !== null && r.first_drawn === null) r.first_drawn = Math.round(now); };
  try { performance.setResourceTimingBufferSize(20000); } catch (e) {}
  const who = () => { const s = new Error().stack || ""; return s.includes("preload.js") ? "preload" : "demand"; };
  const by = () => (new Error().stack || "").split("\n").slice(4, 8).map((l) => l.trim().replace(/^at /, "").replace(/\(?https?:\/\/[^/]+\//, "").replace(/\?v=\d+/g, "").replace(/\)$/, "")).join(" < ");
  const ctx = () => ({ t: Math.round(performance.now()), act: typeof storyCurrentAct !== "undefined" ? storyCurrentAct : -1, mpp: (typeof GEO !== "undefined" && GEO.mpp) ? Math.round(GEO.mpp) : null });
  const wrap = (name, pre) => { const f = window[name]; if (typeof f !== "function") return; window[name] = function(){ try { pre.apply(this, arguments); } catch (e) {} return f.apply(this, arguments); }; };
  document.addEventListener("DOMContentLoaded", () => {
    Z.gl = { lost: 0, restored: 0 };
    try { cv.addEventListener("webglcontextlost", () => { Z.gl.lost++; }); cv.addEventListener("webglcontextrestored", () => { Z.gl.restored++; }); } catch (e) {}
    wrap("reliefLoad", (z, r, c) => { const k = z + "/" + r + "_" + c; if (!RELIEF.tiles.has(k)) ask({ kind: "relief", key: k, url: "geo/relief/" + k + ".jpg", origin: who(), by: by(), ...ctx() }, k); });
    wrap("geoWant10", (k) => { if (!GEO.t10.has(k)) ask({ kind: "ne10", key: k, url: "geo/ne10/" + k + ".bin", origin: who(), by: by(), ...ctx() }, "ne10:" + k); });
    wrap("geoLoad", (key, level) => { if (!GEODATA.pending.has(key + level)) Z.req.push({ ready: null, first_drawn: null, kind: "grid", key: key + level, url: "geo/gd/" + key + level + ".bin.gz", origin: who(), by: by(), ...ctx() }); });
    wrap("geoLoad50", () => { if (!GEO.p50) Z.req.push({ ready: null, first_drawn: null, kind: "land50", key: "ne50+lakes", url: "geo/ne50.bin", origin: who(), by: by(), ...ctx() }); });
    wrap("geoLoadPlaces", () => { if (!GEO.pPlaces) Z.req.push({ ready: null, first_drawn: null, kind: "places", key: "places", url: "geo/places.bin", origin: who(), by: by(), ...ctx() }); });
    const tb = window.tlBuildLeaf;
    if (typeof tb === "function") window.tlBuildLeaf = function(leaf){ const o = who(), t0 = performance.now(), c = ctx(); const e = tb.apply(this, arguments);
      let k = null, d = null; try { k = HOLON.pack(leaf); const b = HOLON.boundsDeg(leaf), la = pitch * 180 / Math.PI, lo = ((yaw * 180 / Math.PI + 540) % 360) - 180;
        d = Math.round(Math.hypot(((b[0] + b[2]) / 2 - la) * 111320, ((b[1] + b[3]) / 2 - lo) * 111320 * Math.cos(la * Math.PI / 180))); } catch (x) {}
      Z.builds.push({ key: k, origin: o, dur: Math.round((performance.now() - t0) * 10) / 10, m_from_centre: d, ...c }); return e; };
    const md = window.microDraw;
    if (typeof md === "function") window.microDraw = function(){ const r = md.apply(this, arguments); try { sample(); } catch (e) { Z.err = String(e); } return r; };
  });
  function sample(){
    const now = performance.now(), w = cv.clientWidth, h = cv.clientHeight, mpp = GEO.mpp, la = pitch * 180 / Math.PI, lo = ((yaw * 180 / Math.PI + 540) % 360) - 180;
    const f = { t: Math.round(now), act: storyCurrentAct, el: Math.round(storyActElapsed * 100) / 100, zoom: Math.round(zoom * 1000) / 1000, tz: Math.round(targetZoom * 1000) / 1000, mpp: Math.round(mpp * 10) / 10, la: Math.round(la * 1e4) / 1e4, lo: Math.round(lo * 1e4) / 1e4, rf: Math.round(GEO.rf * 100) / 100, geo: GEO.used, rel: RELIEF.used };
    // relief: the level each threshold asks for, and the tile under the centre
    f.relief = {};
    if (RELIEF.idx && GEO.rf > 0.01) for (const z of [2, 1]){ if ((z === 2 && mpp > 1500) || (z === 1 && mpp > 6000)) continue;
      const deg = z === 1 ? 45 : 15, r = Math.floor((90 - la) / deg), c = Math.floor((lo + 180) / deg), key = z + "/" + r + "_" + c;
      if (!RELIEF.idx[z].has(r + "_" + c)) continue; const e = RELIEF.tiles.get(key);
      const vis = reliefVisible(z, w, h, mpp); let miss = 0; for (const [rr, cc] of vis){ const k2 = z + "/" + rr + "_" + cc, t = RELIEF.tiles.get(k2); if (t && t.tex) drawn(k2, now); else miss++; }
      f.relief[z] = { focal: !e ? "none" : e.tex ? "ok" : "pending", vis: vis.length, miss }; }
    for (const [k, e] of RELIEF.tiles) if (e.tex) ready(k, now);
    // coastline sets: below GEO_MPP_10 the zoom wants the 10 m tiles; the 50 m set stands in until every visible one is in
    if (GEO.idx10 && mpp < GEO_MPP_10 * 2){ const vis = geoVisibleTiles(w, h, mpp).filter((k) => GEO.idx10.has(k)); let miss = 0;
      for (const k of vis){ const t = GEO.t10.get(k); if (!t || t.pending) miss++; else if (GEO.used === "10m") drawn("ne10:" + k, now); }
      f.ne10 = { want: mpp < GEO_MPP_10 && GEO.rf > 0.004, vis: vis.length, miss }; }
    for (const [k, t] of GEO.t10) if (!t.pending) ready("ne10:" + k, now);
    // the street tiling: on, leaves waiting for a layout, and the leaf under the centre at detail zoom
    f.tl = { on: !!TL.on, pending: TL.pending | 0, drawn: TL.drawn | 0, ring: Math.round(TL.ringEff || TL.ring.ring), gov: Math.round(TL.ring.ring), ema: Math.round(TL.ring.ema || 0), cut: Math.round(TL.cut || 0) };
    if (TL.on && mpp < 26){ try { const T = tlTree(), leaf = T.leafAt(la, lo), k = HOLON.pack(leaf); f.tl.focal = T.budget(leaf) > 0 ? (TL.lay.has(k) ? "ok" : "none") : "empty"; } catch (e) { f.tl.focal = "err"; } }
    f.micro = { story: !!microStory, M: !!M, actors: !!(MSTORY && MSTORY.actors), wait: !!(MSTORY && (MSTORY.waitSearch || MSTORY.waitTiled)), beat: MSTORY && MSTORY.beat !== undefined ? MSTORY.beat : null };
    // the generated street's square on screen (CSS px: centre and half-width), so the pixel check samples around it
    if (M && M.half){ const q = projLL(camFrame(), M.lat0, M.lon0); f.micro.box = [Math.round(q[0]), Math.round(q[1]), Math.round(M.half / mpp)]; }
    // calibration only (--break-detail): the tiling's leaves stay counted as drawn but paint nothing, as in the one flat run
    if (window.__breakDetail && TL.batch) for (const bt of TL.batch.values()) bt.n = 0;
    // painted buildings: on the street, every 15th sampled frame, the screen points (CSS px) of up to 80 lit roof halves of
    // the leaves the tiling drew, outside the generated street's square; the analysis checks those pixels are not ground
    if (TL.on && mpp < 2 && TL.live && f.micro.box && (Z.frames.length % 15) === 0){
      const F = camFrame(), [bx, by, bh] = f.micro.box, pts = [];
      for (const e of TL.live){ if (pts.length >= 80) break; const A = tlAffine(F, tlFrameOf(...tlCentre(e.t))), d = e.data; let k = 0;
        for (let v = 0; v + 30 <= d.length && k < 6; v += 30){ if (d[v + 9] !== 3 || d[v + 5] < 0.35) continue;
          const xm = (d[v] + d[v + 10] + d[v + 20]) / 3, ym = (d[v + 1] + d[v + 11] + d[v + 21]) / 3, x = A.a * xm + A.c * ym + A.e, y = A.b * xm + A.d * ym + A.f;
          if (x < 4 || y < 4 || x > F.w - 4 || y > F.h - 4 || (Math.abs(x - bx) < bh + 6 && Math.abs(y - by) < bh + 6)) continue;
          pts.push([Math.round(x), Math.round(y)]); k++; } }
      f.bpts = pts; }
    Z.frames.push(f);
  }
})()"""

def ts():
    return time.time()

async def play(pg, cdp, rec, stop_act, max_s):
    """Navigate and run until the act index stop_act begins (the street steps are over); record screencast frames if rec."""
    frames = []
    if rec:
        async def on_frame(ev):
            frames.append((ev["metadata"]["timestamp"], ev["data"]))
            try: await cdp.send("Page.screencastFrameAck", {"sessionId": ev["sessionId"]})
            except Exception: pass
        cdp.on("Page.screencastFrame", lambda ev: asyncio.ensure_future(on_frame(ev)))
        await cdp.send("Page.startScreencast", {"format": "jpeg", "quality": 70, "maxWidth": min(A.w, 720), "maxHeight": 900, "everyNthFrame": 1})
    t0 = ts()
    await pg.goto(URL, wait_until="commit")
    end = None
    while ts() - t0 < max_s:
        await asyncio.sleep(0.25)
        try: st = await pg.evaluate("() => (typeof storyCurrentAct === 'undefined') ? null : { act: storyCurrentAct, n: STORY_ACTS.length }")
        except Exception: continue
        if st and st["act"] >= stop_act:
            if end is None: end = ts()
            if ts() - end > 1.0: break
    if rec:
        await cdp.send("Page.stopScreencast")
    return frames, t0

# below this share of drawn lit roofs that show on screen, the tiling's detail was counted as drawn but not painted:
# with --break-detail (leaves counted, nothing painted) 0.0 to 0.105; normal runs at least 0.59 (1440) and 0.77 (390)
PAINTED_MIN = 0.35

def classify(f):
    """Defects under the focal point in one frame (inside the street steps)."""
    out = []
    for z, r in (f.get("relief") or {}).items():
        if r["focal"] != "ok": out.append(f"relief z{z} {r['focal']} under the centre")
    n = f.get("ne10")
    if n and n["want"] and n["miss"]: out.append(f"coast 10m: {n['miss']} of {n['vis']} tiles missing, {f['geo']} drawn")
    t = f.get("tl") or {}
    if f["mpp"] < 400 and f["zoom"] >= 8 and not t.get("on"): out.append("street tiling off at street zoom (bare land)")
    if t.get("focal") == "none": out.append("leaf under the centre not laid out (mosaic only)")
    m = f.get("micro") or {}
    if m.get("story") and f["mpp"] < 26 and not m.get("M"): out.append("street step at street zoom with no street")
    return out

def contact_sheet(frames, z_from, z_to, path, title, marks):
    from PIL import Image, ImageDraw
    from PIL import ImageStat
    keep, last = [], -1e9
    for t, im in frames:
        step = 0.1 if z_from - 1.0 <= t <= z_to else 0.5
        if t - last >= step - 0.02: keep.append((t, im)); last = t
    if not keep: return 0, []
    # a flash: the mean brightness of consecutive frames (every painted frame, not only the kept ones) jumps by more than 24 of 255
    lum, flashes = [], []
    for t, im in frames:
        if z_from - 1.0 <= t <= z_to: lum.append((t, ImageStat.Stat(Image.open(io.BytesIO(base64.b64decode(im))).convert("L")).mean[0]))
    for (t0, a), (t1, b) in zip(lum, lum[1:]):
        if abs(b - a) > 24: flashes.append((round(t1, 2), round(a), round(b)))
    first = Image.open(io.BytesIO(base64.b64decode(keep[0][1])))
    tw = 200 if first.width >= first.height else 120; th = round(first.height * tw / first.width); lab = 14
    cols = max(1, 1800 // (tw + 4)); rows = (len(keep) + cols - 1) // cols
    sheet = Image.new("RGB", (cols * (tw + 4) + 4, 30 + rows * (th + lab + 4) + 4), (16, 18, 24)); d = ImageDraw.Draw(sheet)
    d.text((6, 8), title, fill=(230, 230, 230))
    for i, (t, im) in enumerate(keep):
        x, y = 4 + (i % cols) * (tw + 4), 30 + (i // cols) * (th + lab + 4)
        img = Image.open(io.BytesIO(base64.b64decode(im))).convert("RGB").resize((tw, th))
        sheet.paste(img, (x, y + lab)); m = marks(t); fl = any(abs(t - f[0]) <= 0.06 for f in flashes)
        d.text((x + 1, y), m[0] + (" L" if fl else ""), fill=(255, 120, 110) if (m[1] or fl) else (200, 200, 200))
        if m[1]: d.rectangle([x - 2, y + lab - 2, x + tw + 1, y + lab + th + 1], outline=(255, 60, 50), width=2)
    sheet.save(path, quality=82); return len(keep), flashes

async def main():
    tag = f"zurich-{A.w}-x{A.throttle:g}-{A.cache}"
    out = os.path.join(A.out, tag) if A.out else ""
    if out: os.makedirs(out, exist_ok=True)
    async with async_playwright() as p:
        b = await p.chromium.launch(channel="chrome", headless=True, args=["--use-angle=gl-egl", "--ignore-gpu-blocklist"])
        ctx = await b.new_context(viewport={"width": A.w, "height": A.h}, device_scale_factor=2 if A.mob else 1, is_mobile=bool(A.mob), has_touch=bool(A.mob), service_workers="allow")
        errs = []
        if A.cache == "warm":
            pg0 = await ctx.new_page(); c0 = await ctx.new_cdp_session(pg0)
            await play(pg0, c0, False, 3, A.max_s)
            await pg0.wait_for_timeout(1500); await pg0.close()
        pg = await ctx.new_page(); cdp = await ctx.new_cdp_session(pg)
        pg.on("pageerror", lambda e: errs.append(str(e)))
        cons = []; pg.on("console", lambda m: cons.append(f"[{m.type}] {m.text}"[:300]) if m.type in ("error", "warning") else None)
        await pg.add_init_script(INIT)
        if A.break_detail: await pg.add_init_script("window.__breakDetail = true;")
        src = {}
        await cdp.send("Network.enable")
        cdp.on("Network.responseReceived", lambda ev: src.__setitem__(ev["response"]["url"].split("?")[0].split(f":{A.port}/")[-1],
            "service worker" if ev["response"].get("fromServiceWorker") else "http cache" if ev["response"].get("fromDiskCache") else "network"))
        if A.throttle != 1: await cdp.send("Emulation.setCPUThrottlingRate", {"rate": A.throttle})
        frames, t_nav = await play(pg, cdp, True, 3, A.max_s)
        Z = await pg.evaluate("""() => { const Z = window.__Z; const rt = {}; for (const e of performance.getEntriesByType('resource')) { const u = e.name.split('?')[0].replace(location.origin + '/', ''); if (!(u in rt)) rt[u] = { start: Math.round(e.startTime), end: Math.round(e.responseEnd), size: e.transferSize }; }
          return { req: Z.req, builds: Z.builds, frames: Z.frames, origin0: Z.origin0, rt, err: Z.err || null, gl: Z.gl || null, glErr: (typeof gl !== 'undefined' && gl) ? gl.getError() : null, ctxLost: (typeof gl !== 'undefined' && gl) ? gl.isContextLost() : null,
            zp: typeof ZP !== 'undefined' ? { log: ZP.log, built: ZP.built, leaves: ZP.leaves, done: ZP.done } : null, acts: STORY_ACTS.map(a => a.title) }; }""")
        await b.close()
    if out:  # the raw record first, so an analysis error never costs the run
        json.dump({"Z": Z, "src": src, "errs": errs}, open(os.path.join(out, "raw.json"), "w"))
        import pickle; pickle.dump(frames, open(os.path.join(out, "frames.pkl"), "wb"))
    o0 = Z["origin0"] / 1000.0  # epoch seconds of performance.now() == 0
    fr = Z["frames"]
    # the street steps: from the first frame of act 1 to the first frame of act 3
    a1 = next((f["t"] for f in fr if f["act"] >= 1), None); a3 = next((f["t"] for f in fr if f["act"] >= 3), fr[-1]["t"] if fr else None)
    if a1 is None: print(json.dumps({"tag": tag, "error": "the Story never reached its street step", "errors": errs})); sys.exit(2)
    inwin = lambda t: a1 <= t < a3
    arrive = next((f["t"] for f in fr if f["t"] >= a1 and f["mpp"] < 1 and abs(f["zoom"] / max(1e-9, f["tz"]) - 1) < 0.02), a3)
    # the camera on the street: from settling until the last street step's last beat (the pull-back to the globe) begins
    last_beat = max([f["micro"].get("beat") or 0 for f in fr if f["act"] == 2] or [0])
    pull = next((f["t"] for f in fr if f["act"] == 2 and f["micro"].get("beat") == last_beat and last_beat > 0), a3)
    onstreet = lambda t: arrive <= t < pull
    # per-frame defects, and backwards steps (detail held, then lost, at the same or finer scale)
    bad, back, prev = [], [], None
    for f in fr:
        if not inwin(f["t"]): prev = f; continue
        d = classify(f)
        if prev is not None and inwin(prev["t"]):
            for z in ("2", "1"):
                if (prev.get("relief") or {}).get(z, {}).get("focal") == "ok" and (f.get("relief") or {}).get(z, {}).get("focal") not in (None, "ok"): back.append((f["t"], f"relief z{z} lost"))
            if prev["geo"] == "10m" and f["geo"] != "10m" and f["mpp"] <= prev["mpp"] * 1.05: back.append((f["t"], "coast 10m -> " + f["geo"]))
            if (prev.get("tl") or {}).get("focal") == "ok" and (f.get("tl") or {}).get("focal") == "none": back.append((f["t"], "leaf under the centre lost"))
            if (prev.get("micro") or {}).get("M") and not (f.get("micro") or {}).get("M") and f["act"] == prev["act"]: back.append((f["t"], "street lost"))
        if d: bad.append((f["t"], f["act"], f["mpp"], d))
        prev = f
    # the painted-buildings check: while the camera is on the street, every 2 s, the lit roof halves the tiling says it drew
    # outside the generated street's square must show on screen: the pixel at each differs in colour from the ground around
    # it (the median of eight pixels 12 px away). "Counted as drawn but nothing painted" leaves them all ground.
    ring = []
    try:
        from PIL import Image
        withpts = [f for f in fr if f.get("bpts")]
        cur = arrive + 1000
        while cur < pull - 500 and withpts:
            f = min(withpts, key=lambda x: abs(x["t"] - cur))
            near = min(frames, key=lambda x: abs((x[0] - Z["origin0"] / 1000.0) * 1000.0 - f["t"])) if frames else None
            if abs(f["t"] - cur) < 1000 and near:
                img = Image.open(io.BytesIO(base64.b64decode(near[1]))).convert("RGB"); sc = img.width / A.w; px = img.load(); W, H = img.size
                top, bot, right = int(0.14 * H), int(0.22 * H), int(70 * sc)
                n = hit = 0
                for (x, y) in f["bpts"]:
                    X, Y = int(x * sc), int(y * sc)
                    if Y < top or Y > H - bot or X > W - right or X < 13 or Y < 13 or X > W - 14 or Y > H - 14: continue
                    ring_px = sorted([px[X + dx, Y + dy] for dx, dy in [(-12, 0), (12, 0), (0, -12), (0, 12), (-9, -9), (9, 9), (-9, 9), (9, -9)]], key=lambda c: sum(c))
                    g = ring_px[4]; c = px[X, Y]; n += 1; hit += sum((a - b) ** 2 for a, b in zip(c, g)) ** 0.5 > 18
                if n >= 12: ring.append((f["t"], round(hit / n, 3), n))
            cur += 2000
    except ImportError:
        pass
    flat = [r for r in ring if r[1] < PAINTED_MIN]
    if out and flat:  # the frames themselves, for a closer look
        for tms, *_ in flat:
            near = min(frames, key=lambda x: abs((x[0] - Z["origin0"] / 1000.0) * 1000.0 - tms))
            open(os.path.join(out, f"painted_low_{tms / 1000:07.2f}.jpg"), "wb").write(base64.b64decode(near[1]))
    # tiles: asked, arrived, first drawn, origin, source
    tiles = []
    for r in Z["req"]:
        rt = Z["rt"].get(r["url"]) or {}
        tiles.append({**r, "bytes_in": rt.get("end"), "source": src.get(r["url"], "?"), "in_steps": inwin(r["t"])})
    demand_in = [t for t in tiles if t["origin"] == "demand" and t["in_steps"]]
    late = [t for t in tiles if t["first_drawn"] is not None and t["ready"] is not None and t["first_drawn"] >= a1 and t["ready"] >= a1]
    builds = Z["builds"]; b_in = [x for x in builds if inwin(x["t"])]
    summ = {
        "tag": tag, "acts": Z["acts"][:4], "street_steps_ms": [a1, a3], "camera_settled_on_street_ms": arrive, "pull_back_ms": pull, "frames": len(fr), "screencast_frames": len(frames),
        "tiles_asked": len(tiles), "tiles_preload": sum(t["origin"] == "preload" for t in tiles), "tiles_demand": sum(t["origin"] == "demand" for t in tiles),
        "demand_in_steps": [{k: t[k] for k in ("kind", "key", "t", "act", "mpp", "by", "source", "ready", "first_drawn")} for t in demand_in],
        "ready_after_steps_began_and_drawn": [{k: t[k] for k in ("kind", "key", "origin", "t", "ready", "first_drawn")} for t in late],
        "leaf_builds": {"preload": sum(x["origin"] == "preload" for x in builds), "demand": sum(x["origin"] == "demand" for x in builds),
                        "demand_in_steps": sum(x["origin"] == "demand" for x in b_in), "demand_in_steps_ms": round(sum(x["dur"] for x in b_in if x["origin"] == "demand"), 1),
                        "longest_in_steps_ms": max([x["dur"] for x in b_in] or [0]),
                        "demand_in_steps_at_detail": sum(x["origin"] == "demand" and (x["mpp"] or 99) < 12 for x in b_in),
                        "demand_on_street": sum(x["origin"] == "demand" and onstreet(x["t"]) for x in builds),
                        "demand_in_steps_nearest_m": min([x["m_from_centre"] for x in b_in if x["origin"] == "demand" and x["m_from_centre"] is not None] or [None], key=lambda v: 1e18 if v is None else v)},
        "painted_checks": len(ring), "painted_low": flat, "painted_min": min([r[1] for r in ring] or [None], key=lambda v: 9 if v is None else v), "painted_points": sum(r[2] for r in ring), "bad_frames": len(bad), "bad_kinds": sorted({k for _, _, _, d in bad for k in [x.split(":")[0].split(" under")[0] for x in d]}),
        "bad_first": [(t, a, m, d) for t, a, m, d in bad[:12]], "backwards": back[:20], "n_backwards": len(back),
        "zp": Z["zp"], "errors": errs + ([Z["err"]] if Z["err"] else []), "gl": {"events": Z.get("gl"), "error_at_end": Z.get("glErr"), "context_lost": Z.get("ctxLost")}, "console": cons[:20],
    }
    if out:
        json.dump({"summary": summ, "tiles": tiles, "builds": builds, "frames": fr}, open(os.path.join(out, "log.json"), "w"), indent=0)
        # sheet: screencast timestamps are epoch seconds; page times are ms after timeOrigin
        fl = [((t - o0) * 1000.0, im) for t, im in frames]
        badset = {t for t, *_ in bad}
        def mark(tms):
            near = min(fr, key=lambda f: abs(f["t"] - tms)) if fr else None
            act = near["act"] if near else -1; mpp = near["mpp"] if near else 0
            isbad = any(abs(t - tms) <= 60 for t in badset)
            return (f"{tms / 1000:5.1f}s A{act + 1} {mpp:g}m/px" + (" !" if isbad else ""), isbad)
        n, flashes = contact_sheet([(t / 1000.0, im) for t, im in fl], a1 / 1000.0, arrive / 1000.0 + 3.0, os.path.join(out, "sheet.jpg"),
                          f"{tag}  100 ms frames from the street step's start ({a1 / 1000:.1f}s) until 3 s after the camera settles ({arrive / 1000:.1f}s), 500 ms elsewhere, to the step's end ({a3 / 1000:.1f}s); red = ground not loaded under the centre; L = brightness jump", lambda s: mark(s * 1000.0))
        summ["sheet_frames"] = n; summ["sheet"] = os.path.join(out, "sheet.jpg"); summ["brightness_jumps"] = flashes
        # the dive's frames at full size (every painted frame from the step's start until 3 s after the camera settles), for a closer look
        os.makedirs(os.path.join(out, "dive"), exist_ok=True)
        for tms, im in fl:
            if a1 - 500 <= tms <= arrive + 3000: open(os.path.join(out, "dive", f"{tms / 1000:07.3f}.jpg"), "wb").write(base64.b64decode(im))
        os.remove(os.path.join(out, "frames.pkl"))  # the sheet and the dive frames hold what the strip shows; the raw frames are large
    print(json.dumps(summ, indent=1))
    if A.check:
        ok = not demand_in and not bad and not summ["errors"] and summ["leaf_builds"]["demand_on_street"] == 0 and ring and not flat
        print("OK" if ok else "FAIL"); sys.exit(0 if ok else 1)

asyncio.run(main())
