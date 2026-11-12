# Witness for the Story's act changes (the "freezes when playing the story" report): headless, the Story played from
# page load on index.html at 390 x 844, no CPU throttle. Asserts what the fix guarantees, not how fast this machine is:
#   1. the street act ("Two neighbours connect") ADOPTS the world the idle search built (STORY_ADOPTED === 1), so its
#      act change builds no street world (microBuildLeaf, the kernel build's one caller: 0 calls inside storySetupAct or storyMicroSetup);
#   2. after the opening frames no synchronous land upload runs (geoUpload is called for the 110 m base only; the
#      50 m set and the 10 m tiles go through geoUploadSliced);
#   3. the Story reaches its closing act with nothing paused it but its own end.
# It also prints the longest task and worst frame gap per act, for the record (not asserted: they depend on the box).
# Usage: python3 tools/page/story_freeze/run.py [port]   (with docs/ served on that port, default 8732)
import asyncio, json, sys
from playwright.async_api import async_playwright
PORT = sys.argv[1] if len(sys.argv) > 1 else "8732"
INIT = """() => {
  window.__W = { setupBuilds: 0, syncUploads: 0, inSetup: false, acts: {} };
  // the wasm exports object is frozen, so the build is counted at its one JS caller, microBuildLeaf
  const w = microBuildLeaf; microBuildLeaf = function(){ if (window.__W.inSetup) window.__W.setupBuilds++; return w.apply(this, arguments); };
  // the street act's setup (storySetupAct for the act, and storyMicroSetup, which the act's first tick may call)
  for (const name of ["storySetupAct", "storyMicroSetup"]){ const f = window[name] || eval(name); const g = function(){ window.__W.inSetup = true; try { return f.apply(this, arguments); } finally { window.__W.inSetup = false; } }; eval(name + " = g"); }
  const g = geoUpload; geoUpload = function(){ window.__W.syncUploads++; return g.apply(this, arguments); };
  new PerformanceObserver(l => { for (const e of l.getEntries()){ const a = window.__W.acts[storyCurrentAct] || (window.__W.acts[storyCurrentAct] = { longest: 0, gap: 0 }); a.longest = Math.max(a.longest, Math.round(e.duration)); } }).observe({ entryTypes: ["longtask"] });
  let last = performance.now(); (function f(now){ const a = window.__W.acts[storyCurrentAct] || (window.__W.acts[storyCurrentAct] = { longest: 0, gap: 0 }); a.gap = Math.max(a.gap, Math.round(now - last)); last = now; requestAnimationFrame(f); })(last);
}"""
async def main():
    async with async_playwright() as p:
        b = await p.chromium.launch(channel="chrome", headless=True, args=["--use-angle=gl-egl", "--ignore-gpu-blocklist"])
        ctx = await b.new_context(viewport={"width": 390, "height": 844}, device_scale_factor=2, is_mobile=True, has_touch=True)
        pg = await ctx.new_page(); errs = []
        pg.on("pageerror", lambda e: errs.append(str(e)))
        await pg.goto(f"http://127.0.0.1:{PORT}/index.html")
        await pg.wait_for_function("() => typeof wasm !== 'undefined' && wasm && wasm.micro_build_leaf && typeof storySetupAct === 'function'", timeout=60000)
        await pg.evaluate(INIT)
        for _ in range(60):
            await pg.wait_for_timeout(5000)
            st = await pg.evaluate("() => ({ act: storyCurrentAct, n: STORY_ACTS.length, paused: storyPaused, el: storyActElapsed, dur: STORY_ACTS[storyCurrentAct].duration, user: storyUserCam })")
            if st["act"] == st["n"] - 1 and st["paused"] and st["el"] >= st["dur"] - 0.01: break
        w = await pg.evaluate("() => ({ adopted: typeof STORY_ADOPTED === 'undefined' ? null : STORY_ADOPTED, setupBuilds: window.__W.setupBuilds, syncUploads: window.__W.syncUploads, acts: window.__W.acts })")
        names = await pg.evaluate("() => STORY_ACTS.map(a => a.title)")
        await b.close()
    print(json.dumps({ "end": st, "witness": w, "names": names, "errors": errs }, indent=1))
    ok = (w["adopted"] == 1 and w["setupBuilds"] == 0 and w["syncUploads"] == 0 and not errs and st["act"] == st["n"] - 1 and not st["user"])
    print("OK" if ok else "FAIL"); sys.exit(0 if ok else 1)
asyncio.run(main())
