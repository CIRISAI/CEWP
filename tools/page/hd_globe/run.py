# Witness: does holon-draw draw the globe's passes that docs/hdglobe.js moves onto it (?hdg=: fade, veil, lines, land) as the
# twin's own programs draw them, and no slower? The question gating each layer into HDG_DEFAULT (CEWP#24).
#
# One page load at 1280 x 800, dpr 1, with ?hdg=1 (so the lines and the land keep their sources), the Story closed and the
# sim paused. For each view (the planet at zoom 1; 4000, 600 and 100 m a pixel), the camera is parked, the sim's clock is set
# so the kernel's terminator runs through the view's centre (the hardest place for the GLOBE light), and the tiles settle.
# Then, inside ONE synchronous evaluation (no animation frame runs between the draws, so nothing but the switch differs), the
# globe's draw() is called with every holon-draw layer OFF (the twin's own programs), again OFF (the control), with the
# layer under test ON, and OFF once more (the control after it), and the drawing buffer is read after each:
#   fade   draw(), then zrFade(0.6)
#   veil   with dmVeil replaced, for the evaluation, by incellVeil(0.82) (the veil a data map draws, inside the land stencil)
#   lines  draw()
#   land   draw()
# and the first OFF and the ON pictures are compared over every pixel (largest channel difference per pixel): mean_abs,
# over_8 (% of pixels over 8), over_32, max; for the lines also after a 3 x 3 box filter of both pictures (the "box"
# numbers), which keeps a line's integrated coverage and drops the profile of its antialiased edge. Before, after and
# |difference| x 4 pictures go to --out. The page's resolution governor is held off for the whole run (GFX.scale 1,
# GFX.budget 1e9), so the drawing buffer stays 1280 x 800 and the frame time is not hidden by a lower resolution.
# Then the frame time: the lines and the land ON against every layer OFF (fade and veil are not timed: they draw only in a
# hand-off frame or under a data map, one screen quad each), the page forced to draw every frame (GFX.dirtyUntil), Chrome
# with vsync and the frame-rate limit off, MEASURE_S seconds each, off, on, off, on, the two runs of each pooled: the median
# and 95th percentile interval between animation frames, and the median of draw()'s own CPU time (GFX.drawMs).
# On the machine's GPU through EGL (channel chrome, --use-angle=gl-egl), as the other page witnesses run.
#
# --assert. THE STAKES, written 2026-11-23 before the first run of this file, and not to be edited after a run fails them:
#   S0 the control: at every view and for every comparison, the three frames drawn with the layers off (before and after
#      the ON frame) are identical (max 0), all four frames are 1280 x 800, every number is finite, and the view was ready
#      (holon-draw loaded, the land's sources kept). If not, the comparison is VOID and the run FAILS (the page is not
#      deterministic enough to judge anything). (The tick's arcs, which fade by the clock, are off for the four frames of a
#      comparison; nothing else is changed.)
#      If not, the comparison is VOID and the run FAILS (the page is not deterministic enough to judge anything).
#   S1 fade: max <= 2 at every view (a screen quad of the same colour and alpha; 8-bit rounding only).
#   S2 veil: max <= 2 at every view.
#   S3 land: mean_abs <= 0.25 and over_8 <= 0.1 % at every view (the same triangles and the same formula per pixel).
#   S4 lines: on the 3 x 3 box-filtered pictures, mean_abs <= 1.0 and over_32 <= 0.5 % at every view (a different drawing
#      of a line: a tube antialiased by the canvas's multisampling against the twin's quads with one pixel of coverage
#      falloff; the stake says no visible change in the picture, not the same pixels; the raw numbers are printed, not judged).
#   S5 in each ON frame the layer drew through holon-draw (HDG.drawn[layer] grew: for the lines, only a draw with records
#      counts), and GL reports no error raised during the four frames (the error state is cleared before them).
#   S6 frame time: at every view, the median frame with the lines and the land ON is at most 1.10 x the median with every
#      layer OFF.
#   S7 no page error (an uncaught exception) at all, and no console error that names holon-draw or hdglobe; other console
#      errors are printed and not judged.
#
# AMENDMENT 2026-11-23 (committed alone, before any run under it). S6 measured the wrong thing. The first run (FAIL, kept as
# recorded in docs/holon/README.md) gave median intervals between animation frames of 0.4 to 8.2 ms with the layers off,
# below the median of draw()'s own CPU time at two views (0.4 against 0.2, 2.9 against 0.5 at most): unthrottled, Chrome
# delivers animation frames in bursts, and the median of a bimodal interval is not a frame's cost. Its ratios ran from 0.21
# to 7.0. S6 now reads: at every view, 40 pairs of frames are drawn synchronously, alternating every layer OFF and the lines
# and the land ON, each frame being draw() followed by a one-pixel gl.readPixels (which waits for the GPU to finish it),
# timed with performance.now(); the median ON time is at most 1.10 x the median OFF time. The interval numbers are still
# printed, and no longer judged. Every other stake is unchanged.
# A layer may become a default (HDG_DEFAULT) only when S1..S5 hold for it at every view and S0, S6 and S7 hold.
# --plant fade|veil|land|lines puts a fault into that layer for the run (fade and veil at 0.8 of their alpha; the land's
# ground and its tiles each at 0.8 of their alpha; the lines twice as wide): that layer's stake must then FAIL. It is the
# witness's calibration.
# Usage: python3 tools/page/hd_globe/run.py [port] [--out DIR] [--plant LAYER] [--assert]
import asyncio, json, os, statistics, sys, base64
from playwright.async_api import async_playwright
ARGS = sys.argv[1:]
def opt(name, d=""): return ARGS[ARGS.index(name) + 1] if name in ARGS else d
PORT = next((a for i, a in enumerate(ARGS) if not a.startswith("--") and not (i and ARGS[i - 1] in ("--out", "--plant"))), "8732")
CHECK = "--assert" in ARGS; OUT = opt("--out"); PLANT = opt("--plant")
W, H, MEASURE_S, SETTLE_S = 1280, 800, 5, 12
VIEWS = [("planet", 20.0, 10.0, None), ("continent", 48.0, 10.0, 4000), ("region", 52.2, 5.0, 600), ("city", 40.72, -74.0, 100)]
LAYERS = ["fade", "veil", "lines", "land"]
STAKE = {"fade": {"max": 2}, "veil": {"max": 2}, "land": {"mean_abs": 0.25, "over_8": 0.1}, "lines": {"mean_abs": 1.0, "over_32": 0.5}}
FRAME_RATIO = 1.10
PLANTS = {
    "fade": "(() => { const f = hdgFade; hdgFade = (a) => f(a * 0.8); })()",
    "veil": "(() => { const f = hdgVeil; hdgVeil = (a) => f(a * 0.8); })()",
    "land": "(() => { const f = hdgLand; hdgLand = (R, S, asp, sun, g, t) => f(R, S, asp, sun, g * 0.8, t * 0.8); })()",
    "lines": "(() => { const f = hdgLines; hdgLines = (R, S, asp, gs, LS, nk, w, sun) => f(R, S, asp, gs, LS, nk, w.map((x) => 2 * x), sun); })()",
}
INIT = r"""(() => { const T = window.__T = { on: false, t: [] };
  const raf = window.requestAnimationFrame.bind(window);
  window.requestAnimationFrame = (f) => raf((t) => { if (T.on) T.t.push(t); f(t); }); })();"""
# The comparison, in the page: the layers off, off again, then `layer` on; readPixels after each; stats and pictures.
COMPARE = r"""async (layer) => {
  const grab = () => { const w = gl.drawingBufferWidth, h = gl.drawingBufferHeight, px = new Uint8Array(w * h * 4); gl.readPixels(0, 0, w, h, gl.RGBA, gl.UNSIGNED_BYTE, px); return { w, h, px }; };
  const frame = () => { draw(); if (layer === 'fade') zrFade(0.6); return grab(); };
  const veil = dmVeil; if (layer === 'veil') dmVeil = () => incellVeil(0.82);
  const n0 = HDG.drawn[layer];
  let A, A2, A3, B, err; const arcs = showArcs; showArcs = false;   // the tick's arcs fade by the clock: off for all four frames
  try { while (gl.getError() !== gl.NO_ERROR) {}
    HDG.on = new Set(); A = frame(); A2 = frame(); HDG.on = new Set([layer]); B = frame(); err = gl.getError(); HDG.on = new Set(); A3 = frame(); }
  finally { dmVeil = veil; HDG.on = new Set(); showArcs = arcs; }
  const drew = HDG.drawn[layer] - n0;
  const stats = (P, Q, diff) => { const a = P.px, b = Q.px, n = P.w * P.h; let s = 0, o8 = 0, o32 = 0, mx = 0;
    for (let i = 0; i < n; i++){ let m = 0; for (let c = 0; c < 3; c++){ const d = Math.abs(a[4 * i + c] - b[4 * i + c]); s += d; if (d > m) m = d; if (diff) diff[4 * i + c] = Math.min(255, 4 * d); } if (diff) diff[4 * i + 3] = 255; if (m > 8) o8++; if (m > 32) o32++; if (m > mx) mx = m; }
    return { mean_abs: +(s / (3 * n)).toFixed(4), over_8: +(100 * o8 / n).toFixed(4), over_32: +(100 * o32 / n).toFixed(4), max: mx }; };
  const png = async (w, h, px) => { const c = new OffscreenCanvas(w, h), x = c.getContext('2d'), im = x.createImageData(w, h);
    for (let y = 0; y < h; y++) im.data.set(px.subarray((h - 1 - y) * w * 4, (h - y) * w * 4), y * w * 4);   // readPixels is bottom-up
    x.putImageData(im, 0, 0); const b = new Uint8Array(await (await c.convertToBlob({ type: 'image/png' })).arrayBuffer());
    let s = ''; for (let i = 0; i < b.length; i += 0x8000) s += String.fromCharCode.apply(null, b.subarray(i, i + 0x8000)); return btoa(s); };
  const box = (P) => { const w = P.w, h = P.h, a = P.px, o = new Uint8Array(a.length);
    for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) for (let c = 0; c < 3; c++){ let s = 0, n = 0;
      for (let dy = -1; dy <= 1; dy++) for (let dx = -1; dx <= 1; dx++){ const yy = y + dy, xx = x + dx; if (yy < 0 || yy >= h || xx < 0 || xx >= w) continue; s += a[4 * (yy * w + xx) + c]; n++; }
      o[4 * (y * w + x) + c] = Math.round(s / n); }
    return { w, h, px: o }; };
  const sizes = [A, A2, A3, B].map((P) => P.w + 'x' + P.h);
  if (new Set(sizes).size !== 1) return { void: 'frame sizes differ: ' + sizes.join(' '), sizes, drew, glError: err };
  const diff = new Uint8Array(A.px.length);
  const c1 = stats(A, A2, null), c2 = stats(A, A3, null);
  const control = { mean_abs: Math.max(c1.mean_abs, c2.mean_abs), over_8: Math.max(c1.over_8, c2.over_8), max: Math.max(c1.max, c2.max) };
  const cmp = stats(A, B, diff), bx = layer === 'lines' ? stats(box(A), box(B), null) : null;
  for (let i = 3; i < A.px.length; i += 4){ A.px[i] = 255; B.px[i] = 255; }
  return { control, cmp, box: bx, drew, glError: err, sizes, before: await png(A.w, A.h, A.px), after: await png(B.w, B.h, B.px), diff: await png(A.w, A.h, diff) };
}"""
PARK = r"""(v) => { const [la, lo, mpp] = v; velYaw = 0; velPitch = 0; yaw = lo * Math.PI / 180; pitch = la * Math.PI / 180;
  if (mpp) { const F = camFrame(); zoom = targetZoom = EARTH_M / (0.86 * mpp * Math.min(F.w, F.h) / 2); } else zoom = targetZoom = 1.0;
  // the hour whose terminator runs through the view's centre (the kernel's sun on the sim's day), to a hundredth of an hour
  const c = xyz(la, lo); let best = 0, bd = 9;
  for (let k = 0; k < 2400; k++){ const s = gfxSun(k / 100); const d = Math.abs(s[0] * c[0] + s[1] * c[1] + s[2] * c[2]); if (d < bd){ bd = d; best = k / 100; } }
  SIM.t = best * 3600; SIM.tTick = SIM.t; gfxDirty(SETTLE_MS); return { hour: best, sun_at_centre: bd }; }"""
async def main():
    async with async_playwright() as p:
        b = await p.chromium.launch(channel="chrome", headless=True, args=["--use-angle=gl-egl", "--ignore-gpu-blocklist", "--disable-gpu-vsync", "--disable-frame-rate-limit"])
        pg = await (await b.new_context(viewport={"width": W, "height": H}, device_scale_factor=1)).new_page()
        errs = []; pg.on("pageerror", lambda e: errs.append("pageerror: " + str(e)[:200])); pg.on("console", lambda m: errs.append("console: " + m.text[:200]) if m.type == "error" else None)
        await pg.add_init_script(INIT)
        await pg.goto(f"http://127.0.0.1:{PORT}/index.html?hdg=1")
        await pg.wait_for_function("() => typeof exitStoryMode === 'function' && typeof wasm !== 'undefined' && wasm && typeof HDG !== 'undefined'", timeout=90000)
        await asyncio.sleep(3)
        await pg.evaluate("() => { exitStoryMode(); playing = false; GFX.scale = 1; GFX.budget = 1e9; }")
        if PLANT: await pg.evaluate(PLANTS[PLANT])
        rows, timing = [], []
        for name, la, lo, mpp in VIEWS:
            sun = await pg.evaluate(PARK.replace("SETTLE_MS", str(SETTLE_S * 1000)), [la, lo, mpp])
            await asyncio.sleep(SETTLE_S)
            ready = await pg.evaluate("() => !!hdgDrawer() && !!LAND_SRC")
            for layer in LAYERS:
                await pg.evaluate("() => { GFX.scale = 1; GFX.budget = 1e9; }")
                r = await pg.evaluate(COMPARE, layer)
                if r.get("void"):
                    rows.append({"view": name, "layer": layer, "mpp": mpp, "hour": sun["hour"], "void": r["void"], "control": {"mean_abs": None, "over_8": None, "max": None},
                                 "mean_abs": None, "over_8": None, "over_32": None, "max": None, "box": None, "drew": r["drew"], "glError": r["glError"], "ready": ready, "sizes": r["sizes"]})
                    continue
                if OUT:
                    os.makedirs(OUT, exist_ok=True)
                    for k in ("before", "after", "diff"): open(os.path.join(OUT, f"{name}_{layer}_{k}.png"), "wb").write(base64.b64decode(r[k]))
                rows.append({"view": name, "layer": layer, "mpp": mpp, "hour": sun["hour"], "control": r["control"], **r["cmp"], "box": r["box"], "drew": r["drew"], "glError": r["glError"], "ready": ready, "sizes": r["sizes"]})
            # frame time, all four on against all off (off first, then on, then off and on again; the medians of the two runs pooled)
            fr = {}
            for on in (False, True, False, True):
                await pg.evaluate("(on) => { HDG.on = new Set(on ? ['lines', 'land'] : []); GFX.dirtyUntil = Infinity; GFX.drawMs.length = 0; GFX.scale = 1; GFX.budget = 1e9; }", on)
                await asyncio.sleep(1)
                await pg.evaluate("() => { __T.t = []; __T.on = true; GFX.drawMs.length = 0; }")
                await asyncio.sleep(MEASURE_S)
                t = await pg.evaluate("() => { __T.on = false; return { t: __T.t, d: GFX.drawMs.slice() }; }")
                iv = [y - x for x, y in zip(t["t"], t["t"][1:])]
                f = fr.setdefault(on, {"iv": [], "d": []}); f["iv"] += iv; f["d"] += t["d"]
            await pg.evaluate("() => { HDG.on = new Set(); GFX.dirtyUntil = 0; }")
            # S6 as amended: synchronous pairs, each frame draw() and a one-pixel readPixels
            sync = await pg.evaluate("""() => { const px = new Uint8Array(4), off = [], on = [], arcs = showArcs; showArcs = false;
              const one = (layers) => { HDG.on = new Set(layers); const t0 = performance.now(); draw(); gl.readPixels(0, 0, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, px); return performance.now() - t0; };
              one([]); one(['lines', 'land']);
              for (let k = 0; k < 40; k++){ off.push(one([])); on.push(one(['lines', 'land'])); }
              HDG.on = new Set(); showArcs = arcs; const med = (a) => { const b = a.slice().sort((x, y) => x - y); return b[b.length >> 1]; };
              return { off: +med(off).toFixed(3), on: +med(on).toFixed(3) }; }""")
            med = lambda a: round(statistics.median(a), 3) if a else None
            p95 = lambda a: round(sorted(a)[int(0.95 * (len(a) - 1))], 3) if a else None
            timing.append({"view": name, "off_med": med(fr[False]["iv"]), "off_p95": p95(fr[False]["iv"]), "on_med": med(fr[True]["iv"]), "on_p95": p95(fr[True]["iv"]),
                           "off_draw_ms": med(fr[False]["d"]), "on_draw_ms": med(fr[True]["d"]), "frames": [len(fr[False]["iv"]), len(fr[True]["iv"])],
                           "sync_off_ms": sync["off"], "sync_on_ms": sync["on"]})
        gpu = await pg.evaluate("() => { const e = gl.getExtension('WEBGL_debug_renderer_info'); return e ? gl.getParameter(e.UNMASKED_RENDERER_WEBGL) : null; }")
        await b.close()
    print(f"renderer: {gpu}; {W}x{H} at dpr 1; plant: {PLANT or 'none'}")
    print("%-10s %-6s | %-24s | %9s %8s %8s %4s | %5s %s" % ("view", "layer", "control (mean/o8/max)", "mean_abs", "over_8%", "over_32%", "max", "drew", "stake"))
    why, held = [], {l: True for l in LAYERS}
    import math
    for r in rows:
        c = r["control"]; st = STAKE[r["layer"]]
        if r.get("void") or not r["ready"]:
            held[r["layer"]] = False; why.append(f"{r['view']} {r['layer']}: VOID, " + (r.get("void") or "the view was not ready (holon-draw or the land's sources missing)"))
            print("%-10s %-6s | VOID: %s" % (r["view"], r["layer"], r.get("void") or "not ready")); continue
        judged = r["box"] if r["layer"] == "lines" else r
        nums = [c["mean_abs"], c["over_8"], c["max"], r["mean_abs"], r["over_8"], r["over_32"], r["max"]] + ([judged["mean_abs"], judged["over_32"]] if r["layer"] == "lines" else [])
        if not all(isinstance(x, (int, float)) and math.isfinite(x) for x in nums):
            held[r["layer"]] = False; why.append(f"{r['view']} {r['layer']}: VOID, a number is not finite: {nums}"); continue
        bad = [f"{'box ' if r['layer'] == 'lines' else ''}{k} {judged[k]} > {v}" for k, v in st.items() if judged[k] > v]
        if c["max"] != 0: held[r["layer"]] = False; why.append(f"{r['view']} {r['layer']}: VOID, the control differs (max {c['max']})")
        if r["drew"] <= 0: bad.append("holon-draw did not draw it")
        if r["glError"]: bad.append(f"GL error {r['glError']}")
        if bad: held[r["layer"]] = False; why.append(f"{r['view']} {r['layer']}: " + "; ".join(bad))
        print("%-10s %-6s | %-24s | %9s %8s %8s %4s | %5s %s%s" % (r["view"], r["layer"], f"{c['mean_abs']}/{c['over_8']}/{c['max']}", r["mean_abs"], r["over_8"], r["over_32"], r["max"], r["drew"], "held" if not bad else "FAILED: " + "; ".join(bad),
              ("   (box: mean_abs %s, over_32 %s, max %s)" % (judged["mean_abs"], judged["over_32"], judged["max"])) if r["layer"] == "lines" else ""))
    print("%-10s | %9s %9s %11s | %9s %9s %11s | %8s %8s %6s" % ("view", "off med", "off p95", "off draw ms", "on med", "on p95", "on draw ms", "sync off", "sync on", "ratio"))
    for t in timing:
        ratio = t["sync_on_ms"] / t["sync_off_ms"] if t["sync_on_ms"] and t["sync_off_ms"] else None
        print("%-10s | %9s %9s %11s | %9s %9s %11s | %8s %8s %6s   frames %s" % (t["view"], t["off_med"], t["off_p95"], t["off_draw_ms"], t["on_med"], t["on_p95"], t["on_draw_ms"], t["sync_off_ms"], t["sync_on_ms"], f"{ratio:.3f}" if ratio else "-", t["frames"]))
        if ratio is None or ratio > FRAME_RATIO: why.append(f"{t['view']}: the median frame with the layers on is {ratio} x off (stake {FRAME_RATIO})")
    judged_errs = [e for e in errs if e.startswith("pageerror") or "holon-draw" in e or "hdglobe" in e]
    if errs: print("page and console errors:", errs[:8])
    if judged_errs: why.append(f"page errors, or console errors naming holon-draw: {judged_errs[:3]}")
    common = not any(("VOID" in w or "median frame" in w or "errors naming" in w) for w in why)
    print("layers whose own stakes held at every view:", [l for l in LAYERS if held[l]], "; S0, S6, S7:", "held" if common else "FAILED")
    print(json.dumps({"rows": rows, "timing": timing}, default=str)[:3000])
    if CHECK:
        print("why:", why); print("OK" if not why else "FAIL"); sys.exit(0 if not why else 1)
asyncio.run(main())
