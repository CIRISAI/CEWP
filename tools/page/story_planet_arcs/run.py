# Witness: after the Story's street steps, the globe chapters draw no line from the street. The street steps leave the
# Story's street world (M) in the kernel; microDraw's far view (the patch under 160 px across) draws an arc from the street
# to each off-patch node it links to (its serve seat or server, placed at a generated site 1,500 to 8,000 km away), so in
# "A local community" and every chapter after it one straight green line ran from Switzerland to a seat in the ocean.
# Plays the Story at 390 x 844 from navigation through "Cities connect" and counts, per frame of every globe chapter after
# the street steps (STORY_ACTS index 3 on), the arcs microDraw draws (its drawArc calls), and names each one the first time
# (from the street's node to the far node, lat/lon, kind of link). --assert: none.
# Usage: python3 tools/page/story_planet_arcs/run.py [port] [--assert]
import asyncio, json, sys
from playwright.async_api import async_playwright
PORT = next((a for a in sys.argv[1:] if not a.startswith("--")), "8732"); CHECK = "--assert" in sys.argv
INIT = r"""(() => { const P = window.__P = { arcs: 0, frames: 0, first: null, inMd: false };
  document.addEventListener('DOMContentLoaded', () => {
    const da = window.drawArc; window.drawArc = function(g, F, la1, lo1, la2, lo2, rgb, a){
      if (P.inMd && storyActive && storyCurrentAct >= 3){ P.arcs++;
        if (!P.first){ const links = []; if (M) for (let l = 0; l < M.links.length; l += 4){ const x = M.links[l], y = M.links[l + 1], ox = nodeFlags(x) & NF.offPatch, oy = nodeFlags(y) & NF.offPatch;
            if (!!ox !== !!oy){ const far = ox ? x : y, near = ox ? y : x; links.push({ near, near_kind: MK[nodeKind(near)], far, far_kind: MK[nodeKind(far)], far_at: [+M.nf[far * 4 + 2].toFixed(2), +M.nf[far * 4 + 3].toFixed(2)], shaped: ((M.links[l + 2] >>> 8) & 0xff) > 0 }); } }
          P.first = { act: storyCurrentAct, title: STORY_ACTS[storyCurrentAct].title, from: [+la1.toFixed(2), +lo1.toFixed(2)], to: [+la2.toFixed(2), +lo2.toFixed(2)], colour: rgb, alpha: +a.toFixed(2), street: M ? String(M.key) : null, off_patch_links: links }; } }
      return da.apply(this, arguments); };
    const md = window.microDraw; window.microDraw = function(){ if (typeof storyCurrentAct !== 'undefined' && storyActive && storyCurrentAct >= 3) P.frames++; P.inMd = true; try { return md.apply(this, arguments); } finally { P.inMd = false; } };
  }); })();"""
async def main():
    async with async_playwright() as p:
        b = await p.chromium.launch(channel="chrome", headless=True, args=["--use-angle=gl-egl", "--ignore-gpu-blocklist"])
        pg = await (await b.new_context(viewport={"width": 390, "height": 844}, device_scale_factor=2, is_mobile=True, has_touch=True)).new_page()
        errs = []; pg.on("pageerror", lambda e: errs.append(str(e)))
        await pg.add_init_script(INIT); await pg.goto(f"http://127.0.0.1:{PORT}/index.html")
        for _ in range(400):
            await asyncio.sleep(1)
            try: act = await pg.evaluate("() => storyCurrentAct")
            except Exception: continue
            if act >= 5: break
        P = await pg.evaluate("() => window.__P"); await b.close()
    out = {"globe_chapter_frames": P["frames"], "street_arcs_drawn": P["arcs"], "first": P["first"], "errors": errs}
    print(json.dumps(out, indent=1))
    if CHECK:
        ok = P["frames"] > 30 and P["arcs"] == 0 and not errs
        print("OK" if ok else "FAIL"); sys.exit(0 if ok else 1)
asyncio.run(main())
