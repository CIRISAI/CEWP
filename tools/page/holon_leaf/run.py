# Witness: holon-draw's wasm leaf builder (docs/holon/holon_draw.wasm) gives the same vertex stream as the twin's JS builder
# on REAL leaves, the kernel's own layouts. The page is loaded with ?hdcheck=1, so every leaf the tiling, the preload and the
# Story's street search build is built both ways and compared float for float (tiling.js tlHdCompare, TL_HD.cmp). The
# Story is closed once the wasm is in; the camera then visits a village, a suburb and a dense city at 10 and 3 m a pixel,
# and Zurich, where the preload lays out its leaves.
# --assert: the wasm builder loaded and drew (TL_HD.on), at least MIN_LEAVES leaves compared, no float differs, no stream of
# another length, no page or console error.
# Usage: python3 tools/page/holon_leaf/run.py [port] [--assert]
import asyncio, json, sys
from playwright.async_api import async_playwright
PORT = next((a for a in sys.argv[1:] if not a.startswith("--")), "8732"); CHECK = "--assert" in sys.argv
MIN_LEAVES = 500
PLACES = [("zurich", 47.3769, 8.5417), ("village", 40.107, -83.267), ("suburban", 40.099, -83.114), ("dense", 23.722, 90.408)]
async def main():
    async with async_playwright() as p:
        b = await p.chromium.launch(channel="chrome", headless=True, args=["--use-angle=gl-egl", "--ignore-gpu-blocklist"])
        pg = await (await b.new_context(viewport={"width": 390, "height": 844}, device_scale_factor=2, is_mobile=True, has_touch=True)).new_page()
        errs = []; pg.on("pageerror", lambda e: errs.append("pageerror: " + str(e)[:200])); pg.on("console", lambda m: errs.append("console: " + m.text[:200]) if m.type == "error" else None)
        await pg.goto(f"http://127.0.0.1:{PORT}/index.html?hdcheck=1")
        for _ in range(90):
            await asyncio.sleep(1)
            try:
                if await pg.evaluate("() => typeof TL_HD !== 'undefined' && (TL_HD.on || !!TL_HD.err) && typeof tlReady === 'function' && !!tlReady()"): break
            except Exception: pass
        await pg.evaluate("() => exitStoryMode()")
        steps = []
        for name, la, lo in PLACES:
            for mpp in (10, 3):
                await pg.evaluate("(a) => { const [la, lo, m] = a; velYaw = 0; velPitch = 0; const F = camFrame(); zoom = targetZoom = EARTH_M / (0.86 * m * Math.min(F.w, F.h) / 2); yaw = lo * Math.PI / 180; pitch = la * Math.PI / 180; }", [la, lo, mpp])
                await asyncio.sleep(8)
                steps.append([name, mpp, await pg.evaluate("() => ({ compared: TL_HD.cmp.leaves, drawn: TL.drawn | 0 })")])
        r = await pg.evaluate("() => ({ on: TL_HD.on, err: TL_HD.err, cmp: TL_HD.cmp })")
        await b.close()
    out = {"wasm_builder": {"on": r["on"], "error": r["err"]}, "compared": r["cmp"], "steps": steps, "errors": errs}
    print(json.dumps(out, indent=1))
    if CHECK:
        c = r["cmp"]; why = []
        if not r["on"]: why.append(f"the wasm builder did not load ({r['err']})")
        if c["leaves"] < MIN_LEAVES: why.append(f"only {c['leaves']} leaves compared")
        if c["differ"]: why.append(f"{c['differ']} floats differ")
        if c["length"]: why.append(f"{c['length']} streams of another length")
        if errs: why.append(f"{len(errs)} page or console errors")
        print("why:", why); print("OK" if not why else "FAIL"); sys.exit(0 if not why else 1)
asyncio.run(main())
