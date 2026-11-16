# Witness: the zoom range switch (globe.js ZRANGE; More and Controls, "zoom: street to globe"). At 390 x 844, 6x CPU, cold.
#   1. On by default (no stored choice): zooming far out stops at the globe's edge and far in at the street's (the clamps of
#      before the switch); the page says what it always said.
#   2. A street view (Zurich, 3 m a pixel) and a globe view (7,000 m a pixel) are visited and warmed, the simulation's clock
#      held, and a frame of each taken with the page's interface hidden.
#   3. Switched off through the More menu: from the street view the camera goes a decade past the street's edge and back, and
#      from the globe view a decade past the globe's edge and back. During both trips: no task over LONG_MAX ms; past each
#      edge the twin draws nothing (w 0) and the pill reads "Illustration".
#   4. Back in the band, each view is drawn as before: the share of pixels moved by more than DELTA is at most SAME_MAX, or
#      the share two frames of the view moved by before any trip (the clock held and the arcs off, little else moves), plus SAME_MAX.
#   5. Reloaded: the switch is still off.
# --assert: all of the above and no page or console error.
# Usage: python3 tools/page/zoom_range/run.py [port] [--out DIR] [--assert]   (--out: the four frames, before and after)
import asyncio, io, json, sys
import numpy as np
from PIL import Image
from playwright.async_api import async_playwright
ARGS = sys.argv[1:]; CHECK = "--assert" in ARGS; OUTDIR = ARGS[ARGS.index("--out") + 1] if "--out" in ARGS else ""
PORT = next((a for i, a in enumerate(ARGS) if not a.startswith("--") and not (i and ARGS[i - 1] == "--out")), "8732")
LONG_MAX, DELTA, SAME_MAX = 200, 30, 0.01
HIDE = "*{visibility:hidden !important} canvas#c, canvas#micro{visibility:visible !important}"
INIT = r"""(() => { const L = window.__ZR = { lt: [], on: false };
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
async def trip(pg, factor, target_z, edge_key):
    # step the zoom toward target_z and back, through zoomBy (the clamp every gesture goes through), sampling state
    seen = []
    for direction in (1, -1):
        for _ in range(400):
            st = await pg.evaluate(STATE); seen.append(st)
            if (direction == 1 and ((factor < 1 and st["z"] >= target_z) or (factor > 1 and st["z"] <= target_z))): break
            if direction == -1 and st["side"] is None and abs(st["z"] - st[edge_key]) > 0.05: break
            await pg.evaluate("(f) => zoomBy(f)", factor if direction == 1 else 1 / factor); await asyncio.sleep(0.08)
    return seen
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
        await pg.evaluate("() => { if (typeof SIM !== 'undefined'){ SIM.rate = 0; SIM.t = SIM.tTick = 43200; } microTimeScale = 0; showArcs = false; }")
        for _ in range(120): # the lit hour slews toward the held clock at a capped rate: wait until it has arrived
            if await pg.evaluate("() => drawnHour !== null && Math.abs(drawnHour - getSimUtcHours(getTime())) < 1e-6"): break
            await asyncio.sleep(1)
        await asyncio.sleep(3); ref_street = await shot(pg); await asyncio.sleep(2); ref_street2 = await shot(pg)
        await pg.evaluate(CAM, list(GLOBE)); await asyncio.sleep(6); ref_globe = await shot(pg); await asyncio.sleep(2); ref_globe2 = await shot(pg)
        # 3. off, through the More menu; the trips
        await pg.evaluate("() => { document.getElementById('sf-more').click(); document.getElementById('sf-zrange').click(); }")
        await asyncio.sleep(1); out["off"] = await pg.evaluate(STATE)
        await pg.evaluate(CAM, list(STREET)); await asyncio.sleep(6)
        await pg.evaluate("() => { window.__ZR.on = true; }")
        s0 = await pg.evaluate(STATE)
        inward = await trip(pg, 1.6, s0["street"] - 1.0, "street")
        await pg.evaluate(CAM, list(STREET)); await asyncio.sleep(3)
        await pg.evaluate("() => { window.__ZR.on = false; }"); back_street = await shot(pg)
        await pg.evaluate(CAM, list(GLOBE)); await asyncio.sleep(6)
        await pg.evaluate("() => { window.__ZR.on = true; }")
        g0 = await pg.evaluate(STATE)
        outward = await trip(pg, 1 / 1.6, g0["globe"] + 1.0, "globe")
        await pg.evaluate(CAM, list(GLOBE)); await asyncio.sleep(3)
        await pg.evaluate("() => { window.__ZR.on = false; }"); back_globe = await shot(pg)
        lt = await pg.evaluate("() => window.__ZR.lt")
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
            if out[k]["pill_past"] and not out[k]["pill_past"].startswith("Illustration"): why.append(f"{k}: the pill does not say Illustration past the edge")
        if out["tasks_over"]: why.append(f"{len(out['tasks_over'])} tasks over {LONG_MAX} ms during the trips")
        for v in ("street", "globe"):
            if out[v + "_moved"] > max(SAME_MAX, out[v + "_noise"] + SAME_MAX): why.append(f"the {v} view moved {out[v + '_moved']} after the trip (noise {out[v + '_noise']})")
        if out["after_reload"]["band"]: why.append("the switch did not stay off across a reload")
        if errs: why.append(f"{len(errs)} page or console errors")
        print("why:", why); print("OK" if not why else "FAIL"); sys.exit(0 if not why else 1)
asyncio.run(main())
