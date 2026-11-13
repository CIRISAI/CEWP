# Witness: every data map in the layer strip changes the picture, in its own colours, at every zoom from the whole globe
# down to one street. Headless, the Story closed, over a populated place (Columbus, Ohio by default: the United States, so
# Awake and Data read their reference rather than the hatch). For each zoom (metres a pixel) the page is drawn with no map
# (Off), then with each map, then Off again; the page's own interface is hidden for the shots, so only the canvases count.
# A map passes at a zoom when
#   changed:  the share of the picture whose colour moved by more than DELTA (of 441, RGB distance) from Off is at least
#             CHANGED_MIN, and at least NOISE_X times what moved between the two Off shots (the street's own animation);
#   shown:    the share of the picture moved to a colour brighter than LIT (the veil's near-black is not a map) AND in the
#             map's colours (below) is at least SHOWN_MIN: a map that only dims the land, or only removes the night glow, fails;
#   its ramp: of the moved, lit pixels, at least RAMP_MIN have a colour the map can produce: one of its ramp's colours (or its
#             hatch grey, or either darkened by the hatch) laid over the Off picture, or over it under the map's veil, at an
#             opacity from 0.35 to 1, within NEAR (RGB distance).
# Writes one shot per map and zoom and a contact grid (maps across, zooms down) with --out.
# Usage: python3 tools/page/layer_zoom/run.py [--port 8732] [--w 1440 --h 900 --mob 0] [--mpp 7000,1500,600,100,10]
#        [--query ?tex=1] [--out DIR] [--assert]
import argparse, asyncio, io, json, math, os, sys
import numpy as np
from PIL import Image, ImageDraw, ImageFont
from playwright.async_api import async_playwright

ap = argparse.ArgumentParser()
ap.add_argument("--port", default="8732"); ap.add_argument("--w", type=int, default=1440); ap.add_argument("--h", type=int, default=900)
ap.add_argument("--mob", type=int, default=0); ap.add_argument("--mpp", default="7000,1500,600,100,10")
ap.add_argument("--lat", type=float, default=39.985); ap.add_argument("--lon", type=float, default=-82.985)
ap.add_argument("--maps", default="1,2,6,4,5,3,7"); ap.add_argument("--query", default="", help="appended to the page's address, e.g. ?tex=1 (the procedural ground texture on)"); ap.add_argument("--out", default=""); ap.add_argument("--assert", dest="check", action="store_true")
A = ap.parse_args()
DELTA, CHANGED_MIN, NOISE_X, RAMP_MIN, NEAR, LIT, SHOWN_MIN = 30.0, 0.05, 3.0, 0.60, 42.0, 45.0, 0.04
NAMES = {1: "People", 2: "Awake", 3: "Checked?", 4: "Households", 5: "Power", 6: "Data", 7: "Climate"}

HIDE = "*{visibility:hidden !important} canvas#c, canvas#micro{visibility:visible !important}"

async def shot(pg):
    return np.asarray(Image.open(io.BytesIO(await pg.screenshot())).convert("RGB")).astype(np.float32)

def palette(pal):
    # the map's colours over the Off picture: c * s + off * (1 - s)
    return np.array(pal, dtype=np.float32), np.array([1.0, 0.8, 0.6, 0.45, 0.35], dtype=np.float32)

def judge(off, on, off2, pal):
    d = np.sqrt(((on - off) ** 2).sum(-1)); n = np.sqrt(((off2 - off) ** 2).sum(-1))
    moved = d > DELTA; changed = float(moved.mean()); noise = float((n > DELTA).mean())
    lit = moved & (on.max(-1) > LIT)                                   # moved to a colour brighter than the veil's dark
    ys, xs = np.nonzero(lit)
    if len(ys) == 0: return changed, noise, 0.0, 0.0
    k = np.linspace(0, len(ys) - 1, min(len(ys), 6000)).astype(int); ys, xs = ys[k], xs[k]
    o, c = off[ys, xs], on[ys, xs]
    P, S = palette(pal)
    best = np.full(len(ys), 1e9, dtype=np.float32)
    # under the map: the Off picture, or the Off picture under the veil (dmVeil: 0.82 of a near-black)
    for bg in (o, o * 0.18 + np.array([13, 15, 18], dtype=np.float32) * 0.82):
        for k in (1.0, 0.86):                                        # the hatch darkens a ramp colour by up to 14 %
            for s in S:
                cand = P[None, :, :] * k * s + bg[:, None, :] * (1 - s)   # pixels x palette x 3
                best = np.minimum(best, np.sqrt(((cand - c[:, None, :]) ** 2).sum(-1)).min(1))
    ramp = float((best < NEAR).mean())
    return changed, noise, ramp, float(lit.mean()) * ramp

async def main():
    mpps = [float(x) for x in A.mpp.split(",")]; maps = [int(x) for x in A.maps.split(",")]
    async with async_playwright() as p:
        b = await p.chromium.launch(channel="chrome", headless=True, args=["--use-angle=gl-egl", "--ignore-gpu-blocklist"])
        ctx = await b.new_context(viewport={"width": A.w, "height": A.h}, device_scale_factor=2 if A.mob else 1, is_mobile=bool(A.mob), has_touch=bool(A.mob))
        pg = await ctx.new_page(); errs = []
        pg.on("pageerror", lambda e: errs.append(str(e)))
        await pg.goto(f"http://127.0.0.1:{A.port}/index.html{A.query}")
        await pg.wait_for_function("() => typeof wasm !== 'undefined' && wasm && typeof exitStoryMode === 'function' && typeof tlReady === 'function' && tlReady() && typeof HOLT !== 'undefined' && HOLT.countries && HOLT.t.grid0 && HOLT.t.presence0_us", timeout=180000, polling=250)
        await pg.evaluate(f"() => {{ exitStoryMode(); rotStopped = true; velYaw = velPitch = 0; yaw = {A.lon} * Math.PI / 180; pitch = {A.lat} * Math.PI / 180; }}")
        # every map's palette, read from the page (the ramps the shaders sample, and the climate groups' colours)
        pals = await pg.evaluate("""() => { const r = (k) => Array.from({length: 33}, (_, i) => dmRampAt(DM_RAMPS[k], i / 32)), grey = [92, 97, 112];
            const teal = [41, 199, 204], tm = teal.map((x) => Math.round(x * 0.62));
            const clim = typeof CLIMATE_GROUPS !== 'undefined' ? Object.values(CLIMATE_GROUPS).map((g) => g.rgb) : [];
            return { 1: r('people'), 2: [...r('awake'), grey], 3: [teal, tm, grey], 4: [...r('households'), grey], 5: [...r('power'), grey], 6: [...r('data'), grey], 7: [...clim, ...clim.map((c) => c.map((x) => x * 0.5)), grey] }; }""")
        await pg.add_style_tag(content=HIDE)
        rows, shots = [], {}
        for mpp in mpps:
            await pg.evaluate(f"() => {{ const F = camFrame(); zoom = targetZoom = EARTH_M / (0.86 * {mpp} * Math.min(F.w, F.h) / 2); yaw = {A.lon} * Math.PI / 180; pitch = {A.lat} * Math.PI / 180; dmSet(0); gfxDirty(5000); }}")
            await pg.wait_for_timeout(9000)
            off = await shot(pg); shots[(mpp, 0)] = off
            res = {}
            for m in maps:
                await pg.evaluate(f"() => {{ dmSet({m}); gfxDirty(5000); }}")
                await pg.wait_for_timeout(6000)
                res[m] = await shot(pg); shots[(mpp, m)] = res[m]
                await pg.evaluate("() => { dmSet(0); gfxDirty(3000); }"); await pg.wait_for_timeout(1500)
            off2 = await shot(pg)
            got = await pg.evaluate("() => { const F = camFrame(); return Math.round(EARTH_M / (0.86 * zoom * Math.min(F.w, F.h) / 2)); }")
            for m in maps:
                ch, nz, rp, sh = judge(off, res[m], off2, pals[str(m)])
                ok = ch >= CHANGED_MIN and ch >= NOISE_X * nz and rp >= RAMP_MIN and sh >= SHOWN_MIN
                rows.append({"mpp": mpp, "drawn_mpp": got, "map": NAMES[m], "changed": round(ch, 3), "noise": round(nz, 3), "in_ramp": round(rp, 3), "shown": round(sh, 3), "ok": ok})
                print(json.dumps(rows[-1]), flush=True)
        await b.close()
    if A.out:
        os.makedirs(A.out, exist_ok=True)
        cols = [0] + maps; tw = 300 if A.w > A.h else 150; th = round(A.h * tw / A.w); lab = 22
        try: font = ImageFont.load_default(size=15)
        except Exception: font = ImageFont.load_default()
        g = Image.new("RGB", (len(cols) * (tw + 6) + 6, 30 + len(mpps) * (th + lab + 6)), (16, 18, 24)); d = ImageDraw.Draw(g)
        d.text((8, 8), f"layers x zoom, {A.w} px wide, at {A.lat}, {A.lon}; label: map, m/px, share of the picture changed / shown in the map's colours", fill=(235, 235, 235), font=font)
        for i, mpp in enumerate(mpps):
            for j, m in enumerate(cols):
                im = Image.fromarray(shots[(mpp, m)].astype(np.uint8)); im.save(os.path.join(A.out, f"{int(mpp)}_{NAMES.get(m, 'Off')}.png"))
                x, y = 6 + j * (tw + 6), 30 + i * (th + lab + 6)
                g.paste(im.resize((tw, th)), (x, y + lab))
                r = next((r for r in rows if r["mpp"] == mpp and r["map"] == NAMES.get(m)), None)
                t = f"{NAMES.get(m, 'Off')} {int(mpp)}" + (f" {r['changed']:.2f}/{r['shown']:.3f}" if r else "")
                d.text((x + 2, y + 2), t, fill=(255, 120, 110) if r and not r["ok"] else (220, 220, 220), font=font)
        g.save(os.path.join(A.out, "grid.jpg"), quality=88)
        json.dump({"rows": rows, "errors": errs, "bounds": {"DELTA": DELTA, "CHANGED_MIN": CHANGED_MIN, "NOISE_X": NOISE_X, "RAMP_MIN": RAMP_MIN, "NEAR": NEAR, "LIT": LIT, "SHOWN_MIN": SHOWN_MIN}}, open(os.path.join(A.out, "log.json"), "w"), indent=1)
    bad = [r for r in rows if not r["ok"]]
    print(json.dumps({"failed": [(r["map"], r["mpp"]) for r in bad], "errors": errs}))
    if A.check:
        ok = not bad and not errs
        print("OK" if ok else "FAIL"); sys.exit(0 if ok else 1)
asyncio.run(main())
