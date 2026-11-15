# Witness: nothing the street draws lands away from the street once it is small on the screen, on every path that leaves a
# street world (M) in the page. A node off the street's patch (its serve seat, a server) sits at a generated site 1,500 to
# 8,000 km away in a random direction, so drawn at that site it puts a generated place on real geography: the far view drew
# an arc from the street to it ("A local community" and every globe step after it, and free roam after the Story or after
# any street visit: one straight green line from Switzerland into the Atlantic), and the links, glyphs and pulses to it were
# drawn there whenever the patch was under 300 px across. At street scale such a node is an edge marker, a pointer at the
# screen's edge with its distance, and that is not checked here.
#
# During microDraw every point the street's canvas is given (moveTo, lineTo, curve ends, arc and rect corners), taken through
# the canvas's own transform to screen pixels, must lie within NEAR (the patch's half-diagonal, 0.71 of its width on the
# screen, plus MARGIN for glyphs and tags) of the patch's centre whenever the patch is under PMAX px across. The panel of
# the street's away places and its legend (rhyDraw) and the scale bar are drawn there on purpose and are left out.
# Also counts the far-view arc's calls (drawArc, where the page still has it).
# Paths, at 390 x 844:
#   story: the Story from navigation through "Cities connect", checked in every globe step after the street steps;
#   after: the Story to "A local community", then closed (free roam), the camera taken out from 1 m to 8,000 m a pixel;
#   roam:  the Story closed at once, the camera set onto a street near Zurich, the street built, then the same zoom out.
# Usage: python3 tools/page/story_planet_arcs/run.py [port] [--path story,after,roam] [--assert]
import asyncio, json, sys
from playwright.async_api import async_playwright
args = sys.argv[1:]; CHECK = "--assert" in args
PORT = next((a for a in args if not a.startswith("--") and not (args.index(a) > 0 and args[args.index(a) - 1] == "--path")), "8732")
PATHS = (args[args.index("--path") + 1] if "--path" in args else "story,after,roam").split(",")
PMAX, MARGIN = 300, 60
ZOOMS = [1, 2, 4, 30, 100, 450, 2000, 8000] # the patch is about 150 m across: 150 px at 1 m a pixel, 5 px at 30
INIT = r"""(() => { const W = window.__W = { phase: '', frames: {}, checked: {}, bad: {}, first: {}, arcs: {}, inMd: false, skip: 0, ctx: null };
  const C = CanvasRenderingContext2D.prototype;
  const pt = (g, x, y, what) => { if (!W.inMd || W.skip || g !== W.g || !W.ctx) return;
    const m = g.getTransform(), d = window.devicePixelRatio || 1, px = (m.a * x + m.c * y + m.e) / d, py = (m.b * x + m.d * y + m.f) / d;
    const r = Math.hypot(px - W.ctx.cx, py - W.ctx.cy);
    if (r > W.ctx.near){ W.bad[W.phase] = (W.bad[W.phase] || 0) + 1;
      if (!W.first[W.phase]) W.first[W.phase] = { what, at_px: [Math.round(px), Math.round(py)], from_centre_px: Math.round(r), allowed_px: Math.round(W.ctx.near), patch_px: Math.round(W.ctx.P), mpp: W.ctx.mpp, act: typeof storyCurrentAct !== 'undefined' && storyActive ? storyCurrentAct : null, story: !!storyActive, street: [+M.lat0.toFixed(2), +M.lon0.toFixed(2)], stroke: String(g.strokeStyle), fill: String(g.fillStyle) }; } };
  for (const n of ['moveTo', 'lineTo']){ const f = C[n]; C[n] = function(x, y){ pt(this, x, y, n); return f.apply(this, arguments); }; }
  { const f = C.quadraticCurveTo; C.quadraticCurveTo = function(a, b, x, y){ pt(this, x, y, 'quadraticCurveTo'); return f.apply(this, arguments); }; }
  { const f = C.bezierCurveTo; C.bezierCurveTo = function(a, b, c, d, x, y){ pt(this, x, y, 'bezierCurveTo'); return f.apply(this, arguments); }; }
  { const f = C.arc; C.arc = function(x, y, r){ pt(this, x, y, 'arc'); return f.apply(this, arguments); }; }
  for (const n of ['rect', 'fillRect', 'strokeRect']){ const f = C[n]; C[n] = function(x, y, w, h){ pt(this, x, y, n); pt(this, x + w, y + h, n); return f.apply(this, arguments); }; }
  document.addEventListener('DOMContentLoaded', () => {
    const skip = (n) => { const f = window[n]; if (typeof f !== 'function') return; window[n] = function(){ W.skip++; try { return f.apply(this, arguments); } finally { W.skip--; } }; };
    skip('rhyDraw'); skip('drawScaleBar'); skip('drawEdgeMarkers');
    if (typeof window.drawArc === 'function'){ const da = window.drawArc; window.drawArc = function(){ if (W.inMd && W.ctx) W.arcs[W.phase] = (W.arcs[W.phase] || 0) + 1; return da.apply(this, arguments); }; }
    const md = window.microDraw; window.microDraw = function(){
      W.ctx = null; W.g = typeof mctx !== 'undefined' ? mctx : null;
      if (W.phase && typeof M !== 'undefined' && M && microOk()){ const F = camFrame(), A = patchAffine(F); if (A.front > 0.02){ const P = 2 * M.half * A.pxPerM;
        W.frames[W.phase] = (W.frames[W.phase] || 0) + 1;
        if (P < __PMAX__){ W.checked[W.phase] = (W.checked[W.phase] || 0) + 1;
          W.ctx = { cx: A.e, cy: A.f, P, near: 0.71 * P + __MARGIN__, mpp: Math.round(1 / A.pxPerM) }; } } }
      W.inMd = true; try { return md.apply(this, arguments); } finally { W.inMd = false; W.ctx = null; } };
  }); })();""".replace("__PMAX__", str(PMAX)).replace("__MARGIN__", str(MARGIN))
SETCAM = "(a) => { const [la, lo, mpp, ph] = a; if (ph !== undefined) window.__W.phase = ph; if (typeof storyHold === 'function') storyHold(); velYaw = 0; velPitch = 0; const F = camFrame(); zoom = targetZoom = EARTH_M / (0.86 * mpp * Math.min(F.w, F.h) / 2); yaw = lo * Math.PI / 180; pitch = la * Math.PI / 180; if (typeof gfxDirty === 'function') gfxDirty(4000); }"
async def act(pg):
    try: return await pg.evaluate("() => storyCurrentAct")
    except Exception: return -1
async def zoom_out(pg, tag, la, lo):
    for mpp in ZOOMS:
        await pg.evaluate(SETCAM, [la, lo, mpp, f"{tag}@{mpp}"]); await asyncio.sleep(3)
    await pg.evaluate("() => { window.__W.phase = ''; }")
async def run(p, path):
    b = await p.chromium.launch(channel="chrome", headless=True, args=["--use-angle=gl-egl", "--ignore-gpu-blocklist"])
    pg = await (await b.new_context(viewport={"width": 390, "height": 844}, device_scale_factor=2, is_mobile=True, has_touch=True)).new_page()
    errs = []; pg.on("pageerror", lambda e: errs.append(str(e)[:200]))
    await pg.add_init_script(INIT); await pg.goto(f"http://127.0.0.1:{PORT}/index.html")
    note = {}
    if path in ("story", "after"):
        stop = 5 if path == "story" else 3
        for _ in range(400):
            await asyncio.sleep(1); a = await act(pg)
            if path == "story": await pg.evaluate("(a) => { window.__W.phase = a >= 3 ? 'story' : ''; }", a)
            if a >= stop: break
        await pg.evaluate("() => { window.__W.phase = ''; }")
        if path == "after":
            await asyncio.sleep(3)
            note["street"] = await pg.evaluate("() => M ? [M.lat0, M.lon0] : null")
            await pg.evaluate("() => exitStoryMode()"); await asyncio.sleep(1)
            note["story_after_close"] = await pg.evaluate("() => storyActive")
            if note["street"]: await zoom_out(pg, "after", note["street"][0], note["street"][1])
    else:
        for _ in range(60):
            await asyncio.sleep(1)
            if await pg.evaluate("() => typeof wasm !== 'undefined' && !!wasm && typeof exitStoryMode === 'function' && typeof tlReady === 'function' && !!tlReady()"): break
        await pg.evaluate("() => exitStoryMode()")
        la, lo = 47.3769, 8.5417
        await pg.evaluate(SETCAM.replace("if (typeof storyHold === 'function') storyHold(); ", ""), [la, lo, 3])
        for _ in range(120):
            await asyncio.sleep(1)
            if await pg.evaluate("() => !!(M && M.n > 0)"): break
        note["street"] = await pg.evaluate("() => M ? [M.lat0, M.lon0] : null")
        if note["street"]: await zoom_out(pg, "roam", note["street"][0], note["street"][1])
    W = await pg.evaluate("() => window.__W"); await b.close()
    return {"note": note, "frames": W["frames"], "checked": W["checked"], "off_street_points": W["bad"], "first": W["first"], "far_view_arcs": W["arcs"], "errors": errs}
async def main():
    out = {}
    async with async_playwright() as p:
        for path in PATHS: out[path] = await run(p, path)
    print(json.dumps(out, indent=1))
    if CHECK:
        why = []
        for path, r in out.items():
            n = sum(r["checked"].values())
            if n < 30: why.append(f"{path}: only {n} frames with the street under {PMAX} px")
            if path != "story" and not r["note"].get("street"): why.append(f"{path}: no street world was left to check")
            if sum(r["off_street_points"].values()): why.append(f"{path}: {sum(r['off_street_points'].values())} points drawn away from the street")
            if sum(r["far_view_arcs"].values()): why.append(f"{path}: {sum(r['far_view_arcs'].values())} far-view arcs")
            if r["errors"]: why.append(f"{path}: page errors")
        print("why:", why); print("OK" if not why else "FAIL"); sys.exit(0 if not why else 1)
asyncio.run(main())
