# Witness: the page on a device whose fragment stage has no highp (many phones). Such a device reports no HIGH_FLOAT for
# the fragment stage and leaves GL_FRAGMENT_PRECISION_HIGH undefined, so every fragment shader runs at mediump; a uniform
# declared in both stages must then have one precision, or the program does not link (GLSL ES 1.00, 4.5.3). Emulated here
# on a desktop browser, whose fragment stage has highp:
#   - the precision query reports nothing for FRAGMENT_SHADER / HIGH_FLOAT (gfx.js GFX.hiFrag, and the FP prefix it picks);
#   - every fragment source has GL_FRAGMENT_PRECISION_HIGH renamed to a name never defined, so its #ifdef takes mediump;
#   - a fragment source that still asks for highp outside that #ifdef is recorded (it would not compile on such a device).
# For every program the page links: the link status, and each uniform declared in both stages with the precision each
# stage gives it. Then, the Story closed, the camera on a street near Zurich at 3 m a pixel: the street tier must draw
# (tiles drawn > 0), and the ground texture switch must read "not on this device".
# --assert: no program fails to link, no shared uniform differs in precision, no fragment asks for highp, the street tier
# draws, the switch reads as above, and no page or console error.
# --query ?hd=2: the page with holon-draw's backend drawing the leaves; its programs (docs/holon/draw.js, built with its own
# createProgram, so not found by name) are audited as they link, and at least one tile must be drawn through it.
# Usage: python3 tools/page/no_highp/run.py [port] [--query ?hd=2] [--assert]
import asyncio, glob, json, os, re, sys
ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "..", "docs")
# every program the page can build: each prog(V, F) / gfxProg("name", V, F) with named shader sources, from the page's source
PAIRS = sorted({(v, f) for path in glob.glob(os.path.join(ROOT, "**", "*.js"), recursive=True)
    for v, f in re.findall(r'(?:\bprog|gfxProg\("\w+",)\s*\(?\s*([A-Z][A-Z0-9_]*)\s*,\s*([A-Z][A-Z0-9_]*)\s*\)', open(path).read())})
from playwright.async_api import async_playwright
ARGS = sys.argv[1:]; QUERY = ARGS[ARGS.index("--query") + 1] if "--query" in ARGS else ""
PORT = next((a for i, a in enumerate(ARGS) if not a.startswith("--") and not (i and ARGS[i - 1] == "--query")), "8732"); CHECK = "--assert" in ARGS
INIT = r"""(() => { const W = window.__NH = { links: [], highp: [] }; const P = WebGLRenderingContext.prototype;
  const q = P.getShaderPrecisionFormat; P.getShaderPrecisionFormat = function(st, pt){ const r = q.call(this, st, pt);
    return (st === this.FRAGMENT_SHADER && (pt === this.HIGH_FLOAT || pt === this.HIGH_INT)) ? { rangeMin: 0, rangeMax: 0, precision: 0 } : r; };
  const kind = new WeakMap(), src = new WeakMap(), att = new WeakMap();
  const cs = P.createShader; P.createShader = function(t){ const s = cs.call(this, t); kind.set(s, t === this.FRAGMENT_SHADER ? 'f' : 'v'); return s; };
  // the branch a device without fragment highp takes: GL_FRAGMENT_PRECISION_HIGH undefined
  const pre = (s) => { const out = []; const st = []; for (const ln of s.split('\n')){ const m = ln.match(/^\s*#\s*(ifdef|ifndef|else|endif)\b\s*(\w*)/);
      if (m && (m[1] === 'ifdef' || m[1] === 'ifndef') && m[2] === 'CEWP_NO_HIGHP'){ st.push(m[1] === 'ifndef'); continue; }
      if (m && m[1] === 'else' && st.length){ st[st.length - 1] = !st[st.length - 1]; continue; }
      if (m && m[1] === 'endif' && st.length){ st.pop(); continue; }
      if (st.every(Boolean)) out.push(ln); } return out.join('\n'); };
  const ss = P.shaderSource; P.shaderSource = function(s, text){ let t = text;
    if (kind.get(s) === 'f'){ t = t.replace(/GL_FRAGMENT_PRECISION_HIGH/g, 'CEWP_NO_HIGHP'); if (/\bhighp\b/.test(pre(t))) W.highp.push(pre(t).slice(0, 160)); }
    src.set(s, kind.get(s) === 'f' ? pre(t) : t); return ss.call(this, s, t); };
  const at = P.attachShader; P.attachShader = function(p, s){ if (!att.has(p)) att.set(p, []); att.get(p).push(s); return at.call(this, p, s); };
  const lp = P.linkProgram; P.linkProgram = function(p){ lp.call(this, p);
    const sh = att.get(p) || [], v = sh.find(s => kind.get(s) === 'v'), f = sh.find(s => kind.get(s) === 'f');
    const uni = (text, stage) => { const d = (text.match(/precision\s+(lowp|mediump|highp)\s+float\s*;/) || [])[1] || (stage === 'v' ? 'highp' : 'none');
      const o = {}; for (const m of text.matchAll(/uniform\s+(?:(lowp|mediump|highp)\s+)?(\w+)\s+([^;]+);/g)){
        const ty = m[2], pr = m[1] || (/^sampler/.test(ty) ? 'lowp' : /^(int|ivec|bool|bvec)/.test(ty) ? (stage === 'v' ? 'highp' : 'mediump') : d);
        for (const n of m[3].split(',')) o[n.trim().replace(/\[.*$/, '')] = /^bool|^bvec/.test(ty) ? 'any' : pr; } return o; };
    const U = v ? uni(src.get(v) || '', 'v') : {}, F = f ? uni(src.get(f) || '', 'f') : {};
    const shared = Object.keys(U).filter(n => n in F).map(n => ({ name: n, vertex: U[n], fragment: F[n], same: U[n] === F[n] || U[n] === 'any' }));
    W.links.push({ ok: !!this.getProgramParameter(p, this.LINK_STATUS), log: String(this.getProgramInfoLog(p) || '').replace(/\u0000/g, '').trim().slice(0, 200),
      fragment_head: (src.get(f) || '').replace(/\s+/g, ' ').slice(0, 70), shared }); }; })();"""
async def main():
    async with async_playwright() as p:
        b = await p.chromium.launch(channel="chrome", headless=True, args=["--use-angle=gl-egl", "--ignore-gpu-blocklist"])
        pg = await (await b.new_context(viewport={"width": 390, "height": 844}, device_scale_factor=2, is_mobile=True, has_touch=True)).new_page()
        errs = []; pg.on("pageerror", lambda e: errs.append("pageerror: " + str(e)[:200])); pg.on("console", lambda m: errs.append("console: " + m.text[:200]) if m.type == "error" else None)
        await pg.add_init_script(INIT); await pg.goto(f"http://127.0.0.1:{PORT}/index.html{QUERY}")
        for _ in range(60):
            await asyncio.sleep(1)
            try:
                if await pg.evaluate("(hd) => typeof tlReady === 'function' && !!tlReady() && typeof exitStoryMode === 'function' && (!hd || (typeof TL_HD !== 'undefined' && (!!TL_HD.drawer || !!TL_HD.err)))", "hd=2" in QUERY): break
            except Exception: pass
        await pg.evaluate("() => exitStoryMode()")
        await pg.evaluate("() => { velYaw = 0; velPitch = 0; const F = camFrame(); zoom = targetZoom = EARTH_M / (0.86 * 3 * Math.min(F.w, F.h) / 2); yaw = 8.5417 * Math.PI / 180; pitch = 47.3769 * Math.PI / 180; }")
        drawn = 0
        for _ in range(45):
            await asyncio.sleep(1)
            drawn = await pg.evaluate("(hd) => typeof TL !== 'undefined' && (!hd || (TL.hdTiles | 0) > 0) ? (TL.drawn | 0) : 0", "hd=2" in QUERY)
            if drawn > 0: break
        await asyncio.sleep(2)
        # each of the page's programs linked once more by name, so one the run did not reach is audited too
        named = await pg.evaluate("""(pairs) => pairs.map(([v, f]) => { const n0 = window.__NH.links.length; let err = null;
            try { const V = eval(v), F = eval(f); prog(V, F); } catch (e) { err = String(e).slice(0, 160); }
            const l = window.__NH.links[n0]; return { pair: v + '/' + f, found: !(err && /is not defined/.test(err)), ok: !!(l && l.ok), shared: l ? l.shared : [], err }; })""", [list(x) for x in PAIRS])
        r = await pg.evaluate("""() => { const o = { hiFrag: typeof GFX !== 'undefined' ? GFX.hiFrag : null, tl_on: typeof TL !== 'undefined' && !!TL.on, tl_drawn: typeof TL !== 'undefined' ? TL.drawn | 0 : -1, frames: typeof frame !== 'undefined' ? frame : -1 };
          try { if (typeof DM_ADV_OPEN !== 'undefined'){ DM_ADV_OPEN.open = true; dmRender(); } } catch (e) { o.adv = String(e); }
          o.hd_tiles = typeof TL !== 'undefined' ? TL.hdTiles | 0 : 0; o.hd_drawer = typeof TL_HD !== 'undefined' && !!TL_HD.drawer; o.hd_err = typeof TL_HD !== 'undefined' ? TL_HD.err : null;
          const b = document.querySelector('[data-tl-tex]'); o.switch = b ? b.textContent : null; o.switch_disabled = b ? b.disabled : null; return o; }""")
        r.update(await pg.evaluate("() => window.__NH"))
        await b.close()
    bad_links = [l for l in r["links"] if not l["ok"]]; mism = [dict(s, program=l["fragment_head"]) for l in r["links"] for s in l["shared"] if not s["same"]]
    out = {"hiFrag": r["hiFrag"], "programs_linked": len(r["links"]), "link_failures": bad_links, "shared_uniforms_differing": mism,
           "fragment_asks_highp": r["highp"], "street_tier": {"on": r["tl_on"], "tiles_drawn": r["tl_drawn"], "through_draw_js": r["hd_tiles"], "draw_js": r["hd_drawer"], "holon_error": r["hd_err"]}, "frames": r["frames"],
           "texture_switch": [r["switch"], r["switch_disabled"]], "errors": errs,
           "named_programs": [{"pair": n["pair"], "ok": n["ok"], "found": n["found"], "shared": [f"{s['name']} {s['vertex']}/{s['fragment']}" for s in n["shared"]]} for n in named],
           "audit": [{"program": l["fragment_head"], "ok": l["ok"], "shared": [f"{s['name']} {s['vertex']}/{s['fragment']}" for s in l["shared"]]} for l in r["links"]]}
    print(json.dumps(out, indent=1))
    if CHECK:
        why = []
        if r["hiFrag"] is not False: why.append("the emulation did not take (hiFrag not false)")
        if bad_links: why.append(f"{len(bad_links)} programs failed to link")
        if mism: why.append(f"{len(mism)} uniforms differ in precision between stages")
        if r["highp"]: why.append("a fragment shader asks for highp")
        if not (r["tl_drawn"] > 0): why.append("the street tier drew no tiles at 3 m a pixel")
        if r["switch"] != "ground texture: not on this device" or not r["switch_disabled"]: why.append("the texture switch is offered")
        nb = [n["pair"] for n in named if not n["ok"]]
        if nb: why.append(f"programs that do not link (or cannot be found by name): {nb}")
        if len(named) < 15: why.append(f"only {len(named)} named programs found in the source")
        if "hd=2" in QUERY and not (r["hd_drawer"] and r["hd_tiles"] > 0): why.append(f"holon-draw's backend drew no tile ({r['hd_err']})")
        if "hd=2" in QUERY and len(r["links"]) < len(named) + 2: why.append("holon-draw's programs were not linked")
        if errs: why.append(f"{len(errs)} page or console errors")
        print("why:", why); print("OK" if not why else "FAIL"); sys.exit(0 if not why else 1)
asyncio.run(main())
