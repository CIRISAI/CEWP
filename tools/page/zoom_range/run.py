# Witness: the zoom range switch (globe.js ZRANGE; More and Controls, "zoom: street to globe"). At 390 x 844, 6x CPU, cold.
#   1. On by default (no stored choice): zooming far out stops at the globe's edge and far in at the street's (the clamps of
#      before the switch); the page says what it always said.
#   2. A street view (Zurich, 3 m a pixel) and a globe view (7,000 m a pixel) are visited and warmed, the simulation's clock
#      held, and a frame of each taken with the page's interface hidden.
#   3. Switched off through the More menu: from the street view the camera goes a decade past the street's edge and back, and
#      from the globe view a decade past the globe's edge and back. During both trips: no task over LONG_MAX ms; past each
#      edge the twin draws nothing (w 0) and the pill says the view is past the street and the globe.
#   4. Back in the band, each view is drawn as before: the share of pixels moved by more than DELTA is at most SAME_MAX, or
#      the share two frames of the view moved by before any trip (the clock held and the arcs off, little else moves), plus SAME_MAX.
#      The street's detail ring is held with the clock (HOLON.governor made a no-op), and both street frames wait until the
#      street has finished laying out its leaves (TL.cut 0): the disc of detail ends at the
#      nearest leaf with no layout yet and grows as generation catches up, which on a busy machine is still going a minute in.
#   5. Reloaded: the switch is still off.
#   6. Past each edge holon-draw's ladder draws (window.HOLON_LADDER, loaded once the switch is off): at the deepest and the
#      farthest point of the trips, at least LIT_MIN of the picture (the page's interface hidden) is not the background.
#   7. Across every call into the ladder, the GL state the twin relies on is as it was: program, blend (on, its factors),
#      depth test, cull, stencil test, viewport, scissor, colour mask, ARRAY_BUFFER and the vertex array object.
# --assert: all of the above and no page or console error.
# Usage: python3 tools/page/zoom_range/run.py [port] [--out DIR] [--assert]   (--out: the four frames, before and after)
import asyncio, io, json, sys
import numpy as np
from PIL import Image
from playwright.async_api import async_playwright
ARGS = sys.argv[1:]; CHECK = "--assert" in ARGS; OUTDIR = ARGS[ARGS.index("--out") + 1] if "--out" in ARGS else ""
PORT = next((a for i, a in enumerate(ARGS) if not a.startswith("--") and not (i and ARGS[i - 1] == "--out")), "8732")
LONG_MAX, DELTA, SAME_MAX, LIT_MIN = 200, 30, 0.01, 0.01
HIDE = "*{visibility:hidden !important} canvas#c, canvas#micro{visibility:visible !important}"
INIT = r"""(() => { const L = window.__ZR = { lt: [], on: false, calls: 0, drew: 0, changed: [] };
  const G = (gl) => { const v = gl.getExtension('OES_vertex_array_object'), x = (e) => Array.from(gl.getParameter(e) || []).join(','), bf = gl.getParameter.bind(gl);
    return { program: bf(gl.CURRENT_PROGRAM), blend: bf(gl.BLEND), src: bf(gl.BLEND_SRC_RGB), dst: bf(gl.BLEND_DST_RGB), srcA: bf(gl.BLEND_SRC_ALPHA), dstA: bf(gl.BLEND_DST_ALPHA),
      depth: bf(gl.DEPTH_TEST), cull: bf(gl.CULL_FACE), stencil: bf(gl.STENCIL_TEST), scissor: bf(gl.SCISSOR_TEST), viewport: x(gl.VIEWPORT), mask: x(gl.COLOR_WRITEMASK),
      buffer: bf(gl.ARRAY_BUFFER_BINDING), vao: v ? bf(v.VERTEX_ARRAY_BINDING_OES) : null }; };
  const wrap = () => { const H = window.HOLON_LADDER; if (!H || H.__wrapped) return; const d = H.draw; H.__wrapped = true;
    H.draw = function(gl, view, slot){ const a = G(gl), r = d.apply(this, arguments), b = G(gl); L.calls++; if (r) L.drew++;
      for (const k in a) if (a[k] !== b[k]) L.changed.push([k, String(a[k]), String(b[k])].join(' ')); return r; }; };
  setInterval(wrap, 200);
  new PerformanceObserver((l) => { if (L.on) for (const e of l.getEntries()) L.lt.push([Math.round(e.startTime), Math.round(e.duration)]); }).observe({ entryTypes: ['longtask'] }); })();"""
CAM = "(a) => { const [la, lo, m] = a; velYaw = 0; velPitch = 0; const F = camFrame(); zoom = targetZoom = EARTH_M / (0.86 * m * Math.min(F.w, F.h) / 2); yaw = lo * Math.PI / 180; pitch = la * Math.PI / 180; zoomAnchor = null; if (typeof gfxDirty === 'function') gfxDirty(4000); }"
STATE = "() => ({ band: ZRANGE.band, z: ZRANGE.view.z, w: ZRANGE.view.w, side: ZRANGE.view.side, globe: ZRANGE.view.edges.globe, street: ZRANGE.view.edges.street, pill: (document.getElementById('tier-now') || {}).textContent || '', stored: (() => { try { return localStorage.getItem('cewp.zoomBand'); } catch (e) { return 'n/a'; } })() })"
STREET, GLOBE = (47.3769, 8.5417, 3), (47.3769, 8.5417, 7000)
async def shot(pg):
    await pg.add_style_tag(content=HIDE); await asyncio.sleep(0.3)
    a = np.asarray(Image.open(io.BytesIO(await pg.screenshot())).convert("RGB"), float)
    await pg.evaluate("() => { for (const s of [...document.querySelectorAll('style')]) if (s.textContent.startsWith('*{visibility:hidden')) s.remove(); }")
    return a
def moved(a, b): return float((np.sqrt(((a - b) ** 2).sum(2)) > DELTA).mean())
def lit(a): return float((np.sqrt(((a - np.array([11, 13, 18])) ** 2).sum(2)) > DELTA).mean()) # not the page's background
async def trip(pg, factor, target_z, edge_key):
    # step the zoom toward target_z and back, through zoomBy (the clamp every gesture goes through), sampling state; one frame
    # of the picture at the far end
    seen, far = [], None
    for direction in (1, -1):
        for _ in range(400):
            st = await pg.evaluate(STATE); seen.append(st)
            if (direction == 1 and ((factor < 1 and st["z"] >= target_z) or (factor > 1 and st["z"] <= target_z))):
                await asyncio.sleep(1); far = await shot(pg); break
            if direction == -1 and st["side"] is None and abs(st["z"] - st[edge_key]) > 0.05: break
            await pg.evaluate("(f) => zoomBy(f)", factor if direction == 1 else 1 / factor); await asyncio.sleep(0.08)
    return seen, far
async def laid_out(pg, limit=180):
    for _ in range(limit):
        if await pg.evaluate("() => typeof TL !== 'undefined' && (TL.cut || 0) === 0"): return True
        await asyncio.sleep(1)
    return False
async def main():
    async with async_playwright() as p:
        b = await p.chromium.launch(channel="chrome", headless=True, args=["--use-angle=gl-egl", "--ignore-gpu-blocklist"])
        ctx = await b.new_context(viewport={"width": 390, "height": 844}, device_scale_factor=2, is_mobile=True, has_touch=True)
        pg = await ctx.new_page(); cdp = await ctx.new_cdp_session(pg)
        errs = []; pg.on("pageerror", lambda e: errs.append("pageerror: " + str(e)[:200])); pg.on("console", lambda m: errs.append("console: " + m.text[:200]) if m.type == "error" else None)
        await pg.add_init_script(INIT); await cdp.send("Emulation.setCPUThrottlingRate", {"rate": 6})
        await pg.goto(f"http://127.0.0.1:{PORT}/index.html")
        for _ in range(120):
            await asyncio.sleep(1)
            try:
                if await pg.evaluate("() => typeof tlReady === 'function' && !!tlReady() && typeof ZRANGE !== 'undefined' && !!ZRANGE.view"): break
            except Exception: pass
        await pg.evaluate("() => exitStoryMode()")
        out = {"default": await pg.evaluate(STATE)}
        # 1. the clamps with the switch on
        for _ in range(60): await pg.evaluate("() => zoomBy(0.5)")
        await asyncio.sleep(3); far_out = await pg.evaluate(STATE)
        for _ in range(80): await pg.evaluate("() => zoomBy(2)")
        await asyncio.sleep(4); far_in = await pg.evaluate(STATE)
        out["on_far_out"] = far_out; out["on_far_in"] = far_in
        # 2. warm both views, hold the clock, reference frames
        await pg.evaluate(CAM, list(GLOBE)); await asyncio.sleep(15)
        await pg.evaluate(CAM, list(STREET)); await asyncio.sleep(30)
        # the clock held, and the street's detail ring too: HOLON.governor shrinks it while frames are slow (the trip's frames
        # past the edge are), and at a frame between its thresholds it holds, so without this the street after the trip is
        # drawn with a smaller disc of detail whatever the zoom switch did (the governor's recovery is reported apart)
        await pg.evaluate("() => { if (typeof SIM !== 'undefined'){ SIM.rate = 0; SIM.t = SIM.tTick = 43200; } microTimeScale = 0; showArcs = false; HOLON.governor = (s) => s; }")
        for _ in range(120): # the lit hour slews toward the held clock at a capped rate: wait until it has arrived
            if await pg.evaluate("() => drawnHour !== null && Math.abs(drawnHour - getSimUtcHours(getTime())) < 1e-6"): break
            await asyncio.sleep(1)
        # the camera put back where the trip will return it to: over the warm-up it drifts by a few microradians, which at 3 m a
        # pixel is a pixel or two, and the comparison would read that drift as the switch's doing
        await pg.evaluate(CAM, list(STREET)); await laid_out(pg)
        await asyncio.sleep(3); ref_street = await shot(pg); await asyncio.sleep(2); ref_street2 = await shot(pg)
        await pg.evaluate(CAM, list(GLOBE)); await asyncio.sleep(6); ref_globe = await shot(pg); await asyncio.sleep(2); ref_globe2 = await shot(pg)
        # 3. off, through the More menu; the trips
        await pg.evaluate("() => { document.getElementById('sf-more').click(); document.getElementById('sf-zrange').click(); }")
        await asyncio.sleep(1); out["off"] = await pg.evaluate(STATE)
        for _ in range(120):
            if await pg.evaluate("() => ZRANGE.ladder === 'ready' || ZRANGE.ladder === 'failed'"): break
            await asyncio.sleep(1)
        out["ladder"] = await pg.evaluate("() => ZRANGE.ladder")
        await pg.evaluate(CAM, list(STREET)); await asyncio.sleep(6)
        await pg.evaluate("() => { window.__ZR.on = true; }")
        s0 = await pg.evaluate(STATE)
        inward, far_in = await trip(pg, 1.6, s0["street"] - 1.0, "street")
        await pg.evaluate(CAM, list(STREET)); await asyncio.sleep(3)
        await pg.evaluate("() => { window.__ZR.on = false; }"); await laid_out(pg); back_street = await shot(pg)
        await pg.evaluate(CAM, list(GLOBE)); await asyncio.sleep(6)
        await pg.evaluate("() => { window.__ZR.on = true; }")
        g0 = await pg.evaluate(STATE)
        outward, far_out = await trip(pg, 1 / 1.6, g0["globe"] + 1.0, "globe")
        await pg.evaluate(CAM, list(GLOBE)); await asyncio.sleep(3)
        await pg.evaluate("() => { window.__ZR.on = false; }"); back_globe = await shot(pg)
        lt = await pg.evaluate("() => window.__ZR.lt"); hook = await pg.evaluate("() => ({ calls: window.__ZR.calls, drew: window.__ZR.drew, changed: [...new Set(window.__ZR.changed)] })")
        # 5. reload
        await pg.reload(); await asyncio.sleep(12); out["after_reload"] = await pg.evaluate(STATE)
        await b.close()
    if OUTDIR:
        for n, a in (("street_before", ref_street), ("street_after", back_street), ("globe_before", ref_globe), ("globe_after", back_globe)):
            Image.fromarray(a.astype("uint8")).save(f"{OUTDIR}/{n}.png")
    past_in = [s for s in inward if s["side"] == "in" and s["w"] <= 0]; past_out = [s for s in outward if s["side"] == "out" and s["w"] <= 0]
    out.update({"inward": {"deepest_z": min(s["z"] for s in inward), "street_edge": s0["street"], "frames_past": len(past_in), "pill_past": past_in[-1]["pill"] if past_in else None},
                "outward": {"farthest_z": max(s["z"] for s in outward), "globe_edge": g0["globe"], "frames_past": len(past_out), "pill_past": past_out[-1]["pill"] if past_out else None},
                "longest_task_ms": max([d for _, d in lt] or [0]), "tasks_over": [x for x in lt if x[1] > LONG_MAX],
                "hook": hook, "lit_past_street": round(lit(far_in), 4) if far_in is not None else None, "lit_past_globe": round(lit(far_out), 4) if far_out is not None else None,
                "street_moved": round(moved(ref_street, back_street), 4), "globe_moved": round(moved(ref_globe, back_globe), 4),
                "street_noise": round(moved(ref_street, ref_street2), 4), "globe_noise": round(moved(ref_globe, ref_globe2), 4), "errors": errs})
    print(json.dumps(out, indent=1))
    if CHECK:
        why = []
        d = out["default"]
        if not d["band"] or d["stored"] not in (None, "1"): why.append("the switch is not on by default")
        if abs(out["on_far_out"]["z"] - out["on_far_out"]["globe"]) > 0.02 or out["on_far_out"]["side"]: why.append("with the switch on, zooming out went past the globe's edge")
        if abs(out["on_far_in"]["z"] - out["on_far_in"]["street"]) > 0.02 or out["on_far_in"]["side"]: why.append("with the switch on, zooming in went past the street's edge")
        if out["off"]["band"] or out["off"]["stored"] != "0": why.append("the More menu did not switch it off")
        if not past_in or out["inward"]["deepest_z"] > s0["street"] - 0.9: why.append("the inward trip did not go a decade past the street")
        if not past_out or out["outward"]["farthest_z"] < g0["globe"] + 0.9: why.append("the outward trip did not go a decade past the globe")
        for k in ("inward", "outward"):
            if not (out[k]["pill_past"] or "").startswith("Past the street and the globe"): why.append(f"{k}: the pill does not say the view is past the street and the globe")
        if out["tasks_over"]: why.append(f"{len(out['tasks_over'])} tasks over {LONG_MAX} ms during the trips")
        for v in ("street", "globe"):
            if out[v + "_moved"] > max(SAME_MAX, out[v + "_noise"] + SAME_MAX): why.append(f"the {v} view moved {out[v + '_moved']} after the trip (noise {out[v + '_noise']})")
        if out["after_reload"]["band"]: why.append("the switch did not stay off across a reload")
        if out.get("ladder") != "ready": why.append(f"holon-draw's ladder did not load ({out.get('ladder')})")
        for k in ("lit_past_street", "lit_past_globe"):
            if not (out[k] or 0) >= LIT_MIN: why.append(f"{k}: nothing drawn past the edge ({out[k]})")
        if not hook["drew"]: why.append("the ladder never drew")
        if hook["changed"]: why.append(f"the GL state changed across the ladder's call: {hook['changed']}")
        if errs: why.append(f"{len(errs)} page or console errors")
        print("why:", why); print("OK" if not why else "FAIL"); sys.exit(0 if not why else 1)
asyncio.run(main())
