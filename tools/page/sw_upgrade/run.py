# Headless upgrade test of docs/sw.js: an old "cewp-twin-v1" worker, then two deploys that change
# assets under the SAME URLs (a versioned script and an unversioned geo file) with a new stamp each.
import asyncio, os, sys, re
from playwright.async_api import async_playwright
# Usage: python3 run.py <new sw.js> <old sw.js> <site dir>, with serve.py serving <site dir> on 8761
# (Cache-Control max-age=600, as GitHub Pages). Old worker: git show 4396b3b:docs/sw.js.
NEW, OLD, S = sys.argv[1], sys.argv[2], sys.argv[3]; PORT = 8761
PAGE = '<!doctype html><meta charset=utf-8><script src="source.js"></script><script src="a.js?v=1"></script><script>fetch("geo/t.txt").then(r=>r.text()).then(t=>{window.GEO=t;});navigator.serviceWorker.register("sw.js",{updateViaCache:"none"});</script>'
def deploy(sw_src, stamp, content):
    open(os.path.join(S, "index.html"), "w").write(PAGE)
    open(os.path.join(S, "source.js"), "w").write(f'const CEWP_SOURCE_COMMIT="{stamp}";')
    open(os.path.join(S, "a.js"), "w").write(f'window.A="{content}";')
    open(os.path.join(S, "geo", "t.txt"), "w").write(content)
    sw = open(sw_src).read()
    if "SW_STAMP" in sw: sw = re.sub(r'const SW_STAMP = "[0-9a-f]*";', f'const SW_STAMP = "{stamp}";', sw)
    open(os.path.join(S, "sw.js"), "w").write(sw)
async def state(pg):
    await pg.wait_for_function("() => window.GEO !== undefined", timeout=10000)
    return await pg.evaluate("async () => ({A: window.A, GEO: window.GEO, stamp: CEWP_SOURCE_COMMIT, ctl: !!navigator.serviceWorker.controller, caches: await caches.keys()})")
async def main():
    ok = True
    async with async_playwright() as p:
        b = await p.chromium.launch(); ctx = await b.new_context(); pg = await ctx.new_page()
        url = f"http://127.0.0.1:{PORT}/index.html"
        deploy(OLD, "a" * 40, "OLD")
        await pg.goto(url); await pg.wait_for_timeout(800); await pg.reload(); s = await state(pg); print("old worker      ", s)
        for n, (stamp, content) in enumerate([("b" * 40, "NEW1"), ("c" * 40, "NEW2")]):
            deploy(NEW, stamp, content)
            for k in (1, 2):
                await pg.reload(); await pg.wait_for_timeout(4000); s = await state(pg); print(f"deploy {n+1} reload {k}", s)
                print("   reg", await pg.evaluate("async () => { const r = await navigator.serviceWorker.getRegistration(); return r && {a: r.active && r.active.state, i: !!r.installing, w: !!r.waiting}; }"))
            want = {"A": content, "GEO": content, "stamp": stamp, "caches": ["cewp-twin-" + stamp]}
            good = s["A"] == content and s["GEO"] == content and s["stamp"] == stamp and s["caches"] == want["caches"] and s["ctl"]
            print("  ->", "PASS" if good else "FAIL", want); ok = ok and good
        await b.close()
    print("RESULT", "PASS" if ok else "FAIL"); sys.exit(0 if ok else 1)
asyncio.run(main())
