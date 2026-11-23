# Witness: is the street tier drawn through holon-draw (?hd=2, its records RESIDENT on the GPU) no slower than through the
# tiling's own program (?hd=1, the wasm-built vertex stream)? The question gating ?hd=2 as the default.
#
# For each path, the page is loaded at the same size, the Story closed, and the camera parked over the same places at street
# zoom (a dense city, a suburb, a village; 3 and 10 m a pixel). Once the street has settled the page runs UNTHROTTLED
# (Chrome with vsync and the frame-rate limit off), and every frame is counted for MEASURE_S seconds:
#   frame_ms     the interval between animation frames: median and 95th percentile (the whole page's frame, so the two paths
#                differ only by the street tier)
#   uploads      bytes handed to bufferData/bufferSubData per frame (holon-draw's resident path should add none once settled)
#   draws        draw calls per frame
# On the machine's GPU through EGL (channel chrome, --use-angle=gl-egl), as the other page witnesses run.
#
# --assert (the stakes, written before the first run): at every stop, ?hd=2's median frame is at most 1.10 x ?hd=1's
# (FRAME_RATIO); ?hd=2's uploads per frame are at most ?hd=1's; both paths drew the street (TL.drawn > 0) and ?hd=2 drew
# through draw.js (TL.hdTiles > 0); no page or console error.
#
# AMENDMENT 2026-11-23 (committed alone, before any run under it). The upload stake above measured the wrong thing. It
# compared ALL the page's buffer uploads on the two paths. The page uploads 20 thousand to 1.9 million bytes a frame
# whatever draws the street, and both runs under that stake (2026-11-22 and 2026-11-23) failed it by rounding: within 7 %
# either way, then by 2 to 29 bytes a frame. What the stake was for is that the resident path adds no upload. It now reads
# exactly that: the bytes draw.js itself uploads (its stats().bytesUploaded, summed over the stop's draw() calls) are ZERO
# at every stop once the street has settled. The page-wide numbers are still printed, and no longer judged. The frame stake
# is unchanged. Both earlier runs stay as recorded in docs/holon/README.md: FAIL under the stake as it was.
# Usage: python3 tools/page/hd_timing/run.py [port] [--size WxH] [--assert]
import asyncio, json, statistics, sys
from playwright.async_api import async_playwright
ARGS = sys.argv[1:]
PORT = next((a for i, a in enumerate(ARGS) if not a.startswith("--") and not (i and ARGS[i - 1] == "--size")), "8732")
CHECK = "--assert" in ARGS
W, H = (int(x) for x in (ARGS[ARGS.index("--size") + 1] if "--size" in ARGS else "1280x800").split("x"))
FRAME_RATIO, MEASURE_S, SETTLE_S = 1.10, 6, 10
PLACES = [("dense", 23.722, 90.408), ("suburban", 40.099, -83.114), ("village", 40.107, -83.267)]
INIT = r"""(() => { const W = window.__T = { on: false, t: [], up: 0, dc: 0, hdUp: 0, hdCalls: 0 };
  for (const P of [WebGLRenderingContext.prototype, typeof WebGL2RenderingContext !== 'undefined' ? WebGL2RenderingContext.prototype : null]) { if (!P) continue;
    for (const n of ['drawArrays', 'drawElements', 'drawArraysInstanced', 'drawElementsInstanced']) { const f = P[n]; if (!f) continue; P[n] = function (...a) { if (W.on) W.dc++; return f.apply(this, a); }; }
    for (const n of ['bufferData', 'bufferSubData']) { const f = P[n]; P[n] = function (...a) { if (W.on) { const d = a[1]; W.up += typeof d === 'number' ? 0 : (a.length >= 5 && n === 'bufferSubData' ? a[4] : (d && d.byteLength) || 0); } return f.apply(this, a); }; } }
  const raf = window.requestAnimationFrame.bind(window);
  window.requestAnimationFrame = (f) => raf((t) => { if (W.on) W.t.push(t); f(t); }); })();"""
async def run(b, hd):
    pg = await (await b.new_context(viewport={"width": W, "height": H}, device_scale_factor=1)).new_page()
    errs = []; pg.on("pageerror", lambda e: errs.append("pageerror: " + str(e)[:200])); pg.on("console", lambda m: errs.append("console: " + m.text[:200]) if m.type == "error" else None)
    await pg.add_init_script(INIT)
    await pg.goto(f"http://127.0.0.1:{PORT}/index.html?hd={hd}")
    for _ in range(90):
        await asyncio.sleep(1)
        try:
            if await pg.evaluate("(hd) => typeof TL_HD !== 'undefined' && ((TL_HD.on && (hd !== 2 || !!TL_HD.drawer)) || !!TL_HD.err) && typeof tlReady === 'function' && !!tlReady()", hd): break
        except Exception: pass
    await pg.evaluate("() => exitStoryMode()")
    rows = []
    for name, la, lo in PLACES:
        for mpp in (3, 10):
            await pg.evaluate("(a) => { const [la, lo, m] = a; velYaw = 0; velPitch = 0; const F = camFrame(); zoom = targetZoom = EARTH_M / (0.86 * m * Math.min(F.w, F.h) / 2); yaw = lo * Math.PI / 180; pitch = la * Math.PI / 180; }", [la, lo, mpp])
            await asyncio.sleep(SETTLE_S)
            await pg.evaluate("() => { const D = typeof TL_HD !== 'undefined' && TL_HD.drawer; if (D && !D.__counted){ const d = D.draw; D.__counted = true; D.draw = function(...a){ const r = d.apply(D, a); __T.hdUp += D.stats().bytesUploaded; __T.hdCalls++; return r; }; } Object.assign(__T, { t: [], up: 0, dc: 0, hdUp: 0, hdCalls: 0, on: true }); }")
            await asyncio.sleep(MEASURE_S)
            r = await pg.evaluate("() => { __T.on = false; return { t: __T.t, up: __T.up, dc: __T.dc, hdUp: __T.hdUp, hdCalls: __T.hdCalls, drawn: TL.drawn | 0, hd_tiles: TL.hdTiles | 0 }; }")
            iv = [b2 - a2 for a2, b2 in zip(r["t"], r["t"][1:])]; n = max(1, len(r["t"]))
            rows.append({"place": name, "mpp": mpp, "frames": len(r["t"]), "frame_ms_median": round(statistics.median(iv), 3) if iv else None,
                         "frame_ms_p95": round(sorted(iv)[int(0.95 * (len(iv) - 1))], 3) if iv else None,
                         "uploads_per_frame": round(r["up"] / n), "draws_per_frame": round(r["dc"] / n, 1), "drawn": r["drawn"], "hd_tiles": r["hd_tiles"],
                         "draw_js_bytes": r["hdUp"], "draw_js_calls": r["hdCalls"]})
    gpu = await pg.evaluate("() => { const c = document.createElement('canvas').getContext('webgl2'); const e = c && c.getExtension('WEBGL_debug_renderer_info'); return e ? c.getParameter(e.UNMASKED_RENDERER_WEBGL) : null; }")
    await pg.context.close()
    return rows, errs, gpu
async def main():
    async with async_playwright() as p:
        b = await p.chromium.launch(channel="chrome", headless=True, args=["--use-angle=gl-egl", "--ignore-gpu-blocklist", "--disable-gpu-vsync", "--disable-frame-rate-limit"])
        own, e1, gpu = await run(b, 1)
        hdr, e2, _ = await run(b, 2)
        await b.close()
    print(f"renderer: {gpu}; {W}x{H} at dpr 1; {MEASURE_S} s a stop, unthrottled")
    print("%-9s %4s | %9s %9s %10s %7s | %9s %9s %10s %7s %6s | %6s | %s" % ("place", "m/px", "own med", "own p95", "own up/f", "own dc", "hd med", "hd p95", "hd up/f", "hd dc", "tiles", "ratio", "draw.js bytes / calls"))
    why = []
    for a, h in zip(own, hdr):
        ratio = (h["frame_ms_median"] / a["frame_ms_median"]) if a["frame_ms_median"] and h["frame_ms_median"] else None
        print("%-9s %4d | %9s %9s %10d %7.1f | %9s %9s %10d %7.1f %6d | %6s | %d / %d" % (a["place"], a["mpp"], a["frame_ms_median"], a["frame_ms_p95"], a["uploads_per_frame"], a["draws_per_frame"],
              h["frame_ms_median"], h["frame_ms_p95"], h["uploads_per_frame"], h["draws_per_frame"], h["hd_tiles"], f"{ratio:.3f}" if ratio else "-", h["draw_js_bytes"], h["draw_js_calls"]))
        tag = f"{a['place']} at {a['mpp']} m/px"
        if not a["drawn"] or not h["drawn"]: why.append(f"{tag}: the street was not drawn on both paths")
        if not h["hd_tiles"]: why.append(f"{tag}: ?hd=2 drew no tile through draw.js")
        if ratio is None or ratio > FRAME_RATIO: why.append(f"{tag}: ?hd=2's median frame is {ratio} x ?hd=1's (stake {FRAME_RATIO})")
        if h["draw_js_calls"] == 0 or h["draw_js_bytes"] != 0: why.append(f"{tag}: draw.js uploaded {h['draw_js_bytes']} B in {h['draw_js_calls']} draw() calls (stake: 0 bytes, and at least one call)")
    if e1 or e2: why.append(f"page or console errors: {(e1 + e2)[:3]}")
    print(json.dumps({"own": own, "hd": hdr}, indent=None)[:4000])
    if CHECK:
        print("why:", why); print("OK" if not why else "FAIL"); sys.exit(0 if not why else 1)
asyncio.run(main())
