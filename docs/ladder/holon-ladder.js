// holon-draw's ladder on a host's canvas: the adapter the CEWP twin calls.
//
// The twin draws street to globe. Past either edge of that band it calls
//
//   window.HOLON_LADDER.draw(gl, view, slot)
//
// and this file draws the rungs of the ladder (`../src/lib.rs`) there, on the twin's own GL
// context, at one minus the twin's own weight. One ES module beside `draw.js`, `text.js` and
// `holon_draw_ladder.wasm`; no other dependency.
//
//   import { install } from './holon-ladder.js';
//   await install({ wasm: './holon_draw_ladder.wasm' });   // sets window.HOLON_LADDER
//   window.HOLON_LADDER.prepare(gl, view)   // optional, at idle: the first draw's setup ahead of time
//
// The FIRST draw on a context makes the drawer and the text provider (compiling their programs)
// and builds the rungs' first tiles and text: tens of milliseconds on a desktop, several times
// that on a slow phone, inside whatever frame first goes past an edge. prepare(gl, view) does
// that at a moment the host chooses (its idle, once the switch is off), with no drawing.
//
// INSTALL OPTIONS (all optional):
//   wasm          URL of holon_draw_ladder.wasm; default: beside this file
//   global        false: do not set window.HOLON_LADDER (the adapter is still returned)
//   caption       false: do not draw the engine's caption panel (the rung's name, where what it
//                 shows comes from, a scale bar). On by default: a rung that is an illustration
//                 says so on screen, and this is where it says it.
//   zPixel        'device' (default): view.z is log10 metres per DEVICE pixel, a CSS pixel being
//                 view.dpr of them. 'buffer': per pixel of the GL buffer (a CSS pixel being
//                 view.renderScale of them). Either way a view.k (the Earth's radius on screen
//                 in CSS pixels), if the host sends one, settles the scale on the outward side.
//   restoreState  false: leave GL state as `draw.js` documents it leaves it, instead of putting
//                 every piece this file changes back as it was found (the default)
//   inward        (view, t) => [yaw, pitch] for the rungs finer than the street, t the decades
//                 past the street edge. Default: straight down with north up at the edge (a map,
//                 as the street tier is), tilting to 0.55 rad above the ground a decade in.
//   onError       (error) => void; default console.warn, once per message
//
// THE VIEW RECORD, as the twin sends it (fields this file reads):
//   z            log10 metres per device pixel at the view's centre
//   edges        { globe, street }: the z of the band's two edges
//   w            the twin's own weight: 1 in its band, falling to 0 past either edge
//   band         true: the switch is ON, the zoom stays in the twin's band; nothing is drawn
//   yaw, pitch   radians: the LONGITUDE (east positive) and LATITUDE of the view's centre on
//                the globe; north is up on screen
//   viewport     [w, h] in CSS pixels;  dpr;  renderScale (GL buffer pixels per CSS pixel)
//   sun          [lat, lon] in degrees of the point with the twin's sun overhead (optional)
//   utcHours, doy  the twin's simulated hour and day of year (optional; the wall clock if absent)
//   k            the Earth's on-screen radius in CSS pixels (optional; see zPixel)
// `slot` is "out" past the globe and "in" past the street.
//
// WHAT IS DRAWN
//   out   The Earth rung onward. The Earth's centre is the view's centre; the point under it is
//         (longitude yaw, latitude pitch) with north up, which is the twin's globe's frame
//         relabelled (the twin: Y north, Z Greenwich, X 90 E; holon-draw: z north, x Greenwich,
//         y 90 E). At the edge the view is orthographic to within a hundredth of a percent
//         (`Host::far_eye`), so the Earth's limb is 6,371,000 / 10^z device pixels out, the
//         twin's k; the eye comes in to a perspective over the decade beyond. Day and night are
//         lit from view.sun when it is sent.
//   in    The ground underfoot and inward. These rungs are a generic patch of ground, not the
//         twin's place: view.anchor is not used.
// The ladder's share is 1 - view.w and nothing is drawn (and no GL state touched) at share 0,
// with the switch on, or before the module has loaded.
//
// GL STATE. The twin's framebuffer, scissor, colour mask and STENCIL are not touched: no
// stencil call is made (so no stencil bit is written, in or out of bits 4..128), and the
// colour buffer is not cleared. The DEPTH buffer IS cleared, once per rung drawn (each rung is
// its own scale and its own depth range), with depthMask(true) set for it; a scissor test the
// host left on limits that clear, as it limits everything here.
// With `restoreState` (the default) these are read before drawing and put back after:
//   the viewport; the current program; BLEND and the four blend factors; DEPTH_TEST, depthFunc,
//   depthMask; CULL_FACE, cullFace, frontFace; the ARRAY_BUFFER binding; the vertex array
//   object binding (WebGL 2 or OES_vertex_array_object); activeTexture and the TEXTURE_2D
//   bindings of units 0..3.
// Not put back (nothing reads them cheaply): on a bare WebGL 1 context with no vertex array
// objects, attribute slots 0..7 are left disabled with divisor 0 and ELEMENT_ARRAY_BUFFER null,
// as `draw.js` says. The blend EQUATION is not touched (a host using another must restore
// FUNC_ADD before the call, as `draw.js` says). Pixel-store flags are changed and put back by
// `text.js` itself around each upload.
// With `restoreState: false` the state is left as `draw.js` documents, plus the viewport set to
// the whole buffer.

import { createDrawer } from './draw.js';
import { createTextProvider } from './text.js';

const EARTH_RADIUS_M = 6371000;

/** Install the adapter. Resolves once the module is compiled; `draw` is a no-op until then. */
export async function install(opts) {
  opts = opts || {};
  const here = new URL('.', import.meta.url);
  const wasmUrl = opts.wasm ? new URL(opts.wasm, typeof location !== 'undefined' ? location.href : here).href
    : new URL('holon_draw_ladder.wasm', here).href;
  const warned = new Set();
  const onError = opts.onError || ((e) => { const m = String(e && e.message || e); if (!warned.has(m)) { warned.add(m); console.warn('holon-ladder: ' + m); } });

  let w = null;          // the wasm exports
  let gfx = null;        // { gl, text, drawer, vaoExt } for the context last drawn on
  const dec = new TextDecoder();
  const env = {
    host_text(ptr, len, style, out) {
      const o = new Float32Array(w.memory.buffer, out, 8);
      if (!gfx || !gfx.text) { o.fill(0); o[7] = -1; return; }
      const p = gfx.text.place(dec.decode(new Uint8Array(w.memory.buffer, ptr, len)), style);
      o[0] = p.w; o[1] = p.h; o[2] = p.u0; o[3] = p.v0; o[4] = p.u1; o[5] = p.v1; o[6] = p.page;
      o[7] = p.drawn === undefined ? -1 : p.drawn;   // 0 whole, 1 cut, 2 changed, -1 not said
    },
  };

  const band = { lo: NaN, hi: NaN };
  const batches = [], pool = [];
  const saved = { viewport: new Int32Array(4), tex: [null, null, null, null] };
  let lastT = 0;
  const last = { drawn: false, share: 0, passes: 0, z: NaN, yaw: 0, pitch: 0, width: 0, height: 0, ms: 0, slot: null };

  // What is known of a context before anything is made on it (so that its state can be read
  // first), and then the text provider and the drawer, made on the first draw call and again
  // only if the context or the buffer's scale changes. Making them compiles programs and
  // binds buffers: it is done between save() and restore().
  const context = (gl) => {
    if (gfx && gfx.gl === gl) return gfx;
    gfx = { gl, text: null, textScale: 0, drawer: null, vaoExt: null, gl2: typeof WebGL2RenderingContext !== 'undefined' && gl instanceof WebGL2RenderingContext };
    if (!gfx.gl2) gfx.vaoExt = gl.getExtension('OES_vertex_array_object');
    return gfx;
  };
  const graphics = (g, scale) => {
    if (g.drawer && g.textScale === scale) return;
    if (g.drawer) g.drawer.destroy();
    g.text = createTextProvider(g.gl, { dpr: scale });
    g.textScale = scale;
    g.drawer = createDrawer(g.gl, { text: g.text });
    w.hl_text_stale();
  };

  const save = (g) => {
    const gl = g.gl, s = saved;
    s.viewport.set(gl.getParameter(gl.VIEWPORT));
    s.program = gl.getParameter(gl.CURRENT_PROGRAM);
    s.blend = gl.isEnabled(gl.BLEND);
    s.srcRGB = gl.getParameter(gl.BLEND_SRC_RGB); s.dstRGB = gl.getParameter(gl.BLEND_DST_RGB);
    s.srcA = gl.getParameter(gl.BLEND_SRC_ALPHA); s.dstA = gl.getParameter(gl.BLEND_DST_ALPHA);
    s.depth = gl.isEnabled(gl.DEPTH_TEST); s.depthFunc = gl.getParameter(gl.DEPTH_FUNC); s.depthMask = gl.getParameter(gl.DEPTH_WRITEMASK);
    s.cull = gl.isEnabled(gl.CULL_FACE); s.cullMode = gl.getParameter(gl.CULL_FACE_MODE); s.frontFace = gl.getParameter(gl.FRONT_FACE);
    s.arrayBuffer = gl.getParameter(gl.ARRAY_BUFFER_BINDING);
    s.vao = g.gl2 ? gl.getParameter(gl.VERTEX_ARRAY_BINDING) : g.vaoExt ? gl.getParameter(g.vaoExt.VERTEX_ARRAY_BINDING_OES) : null;
    s.activeTexture = gl.getParameter(gl.ACTIVE_TEXTURE);
    for (let i = 0; i < 4; i++) { gl.activeTexture(gl.TEXTURE0 + i); s.tex[i] = gl.getParameter(gl.TEXTURE_BINDING_2D); }
    gl.activeTexture(s.activeTexture);
  };
  const restore = (g) => {
    const gl = g.gl, s = saved;
    for (let i = 0; i < 4; i++) { gl.activeTexture(gl.TEXTURE0 + i); gl.bindTexture(gl.TEXTURE_2D, s.tex[i]); }
    gl.activeTexture(s.activeTexture);
    if (g.gl2) gl.bindVertexArray(s.vao); else if (g.vaoExt) g.vaoExt.bindVertexArrayOES(s.vao);
    gl.bindBuffer(gl.ARRAY_BUFFER, s.arrayBuffer);
    gl.frontFace(s.frontFace); gl.cullFace(s.cullMode);
    if (s.cull) gl.enable(gl.CULL_FACE); else gl.disable(gl.CULL_FACE);
    gl.depthMask(s.depthMask); gl.depthFunc(s.depthFunc);
    if (s.depth) gl.enable(gl.DEPTH_TEST); else gl.disable(gl.DEPTH_TEST);
    gl.blendFuncSeparate(s.srcRGB, s.dstRGB, s.srcA, s.dstA);
    if (s.blend) gl.enable(gl.BLEND); else gl.disable(gl.BLEND);
    gl.useProgram(s.program);
    gl.viewport(s.viewport[0], s.viewport[1], s.viewport[2], s.viewport[3]);
  };

  // The host's clock, in Unix seconds: its simulated day of year and hour if it sends them
  // (in this year), its hour today if only that, else the wall clock.
  const clockOf = (view) => {
    const now = Date.now();
    if (!Number.isFinite(view.utcHours)) return now / 1000;
    const d = new Date(now);
    const day = Number.isFinite(view.doy) ? Date.UTC(d.getUTCFullYear(), 0, 1) + (view.doy - 1) * 86400000
      : Date.UTC(d.getUTCFullYear(), d.getUTCMonth(), d.getUTCDate());
    return day / 1000 + view.utcHours * 3600;
  };

  const inward = opts.inward || ((view, t) => {
    const s = Math.min(1, Math.max(0, t));
    const e = s * s * (3 - 2 * s);
    return [-Math.PI / 2, Math.PI / 2 + (0.55 - Math.PI / 2) * e];
  });

  /** Draw the ladder's share of this frame. Returns true if anything was drawn. */
  function draw(gl, view, slot) {
    last.drawn = false; last.slot = slot || null;
    if (!w || !gl || !view) return false;
    const share = 1 - Math.min(1, Math.max(0, Number.isFinite(view.w) ? view.w : 1));
    last.share = view.band ? 0 : share;
    if (view.band || !(share > 0)) return false;
    const edges = view.edges || {};
    if (!Number.isFinite(view.z) || !Number.isFinite(edges.globe) || !Number.isFinite(edges.street)) return false;
    const side = slot || view.side || (view.z > edges.globe ? 'out' : 'in');
    try {
      const t0 = performance.now();
      const g = context(gl);
      const vp = view.viewport || [gl.drawingBufferWidth, gl.drawingBufferHeight];
      const css = Number.isFinite(view.dpr) && view.dpr > 0 ? view.dpr : 1;
      const rs = Number.isFinite(view.renderScale) && view.renderScale > 0 ? view.renderScale : css;
      // the buffer's size: what the host's scale makes of its CSS size, or the drawing buffer
      // itself when that is within two pixels of it (the host's own rounding)
      let W = Math.round(vp[0] * rs), H = Math.round(vp[1] * rs);
      if (Math.abs(gl.drawingBufferWidth - W) <= 2 && Math.abs(gl.drawingBufferHeight - H) <= 2) { W = gl.drawingBufferWidth; H = gl.drawingBufferHeight; }
      if (!(W > 0 && H > 0)) return false;
      // the ladder's z is metres per pixel of the buffer it draws into
      let shift = opts.zPixel === 'buffer' ? 0 : Math.log10(css / rs);
      if (side === 'out' && Number.isFinite(view.k) && view.k > 0) shift = Math.log10(EARTH_RADIUS_M / (view.k * rs)) - view.z;
      const z = view.z + shift;
      const lo = edges.street + shift, hi = edges.globe + shift;
      if (lo !== band.lo || hi !== band.hi) { w.hl_band(lo, hi, 0); band.lo = lo; band.hi = hi; }
      w.hl_host_weight_override(1 - share, 1);
      w.hl_overlay(opts.caption === false ? 0 : 1, 0);
      const sun = view.sun;
      if (sun && Number.isFinite(sun[0]) && Number.isFinite(sun[1])) w.hl_sun(sun[0], sun[1]); else w.hl_sun(NaN, NaN);
      let yaw = view.yaw || 0, pitch = view.pitch || 0;
      if (side === 'in') { const a = inward(view, edges.street - view.z); yaw = a[0]; pitch = a[1]; }
      pitch = Math.min(Math.PI / 2, Math.max(-Math.PI / 2, pitch));
      const clock = clockOf(view);
      const dt = lastT ? Math.min(0.1, Math.max(0, (t0 - lastT) / 1000)) : 0;
      lastT = t0;

      const keep = opts.restoreState !== false;
      if (keep) save(g);
      try {
        graphics(g, rs);
        gl.viewport(0, 0, W, H);
        g.text.beginFrame();
        w.hl_advance(dt);
        const epoch = g.text.epoch;
        let n = w.hl_view(z, yaw, pitch, W, H, rs, clock);
        if (g.text.epoch !== epoch) { w.hl_text_stale(); n = w.hl_view(z, yaw, pitch, W, H, rs, clock); }
        const viewport = [0, 0, W, H];
        for (let p = 0; p < n; p++) {
          const m = w.hl_pass_batches(p);
          batches.length = 0;
          for (let i = 0; i < m; i++) {
            const b = pool[i] || (pool[i] = { offset: [0, 0, 0] });
            b.kind = w.hl_batch_kind(p, i); b.count = w.hl_batch_count(p, i);
            b.screen = !!w.hl_batch_screen(p, i); b.coarse = !!w.hl_batch_coarse(p, i);
            b.offset[0] = w.hl_batch_offset(p, i, 0); b.offset[1] = w.hl_batch_offset(p, i, 1); b.offset[2] = w.hl_batch_offset(p, i, 2);
            b.bytes = new Uint8Array(w.memory.buffer, w.hl_batch_ptr(p, i), b.count * 52);
            b.params = new Float32Array(w.memory.buffer, w.hl_batch_params(p, i), 32);
            batches.push(b);
          }
          if (m === 0) continue;
          // each rung is its own scale: its depth starts clean
          gl.depthMask(true);
          gl.clear(gl.DEPTH_BUFFER_BIT);
          g.drawer.draw(batches, new Float32Array(w.memory.buffer, w.hl_pass_vp(p), 16), { viewport });
        }
        last.passes = n;
      } finally {
        if (keep) restore(g);
      }
      last.drawn = true; last.z = z; last.yaw = yaw; last.pitch = pitch; last.width = W; last.height = H; last.ms = performance.now() - t0;
      return true;
    } catch (e) {
      onError(e);
      return false;
    }
  }

  /**
   * Do ahead of time, at the host's idle, what the first draw call would otherwise do inside a
   * frame: make the text provider and the drawer on this context (which compiles their
   * programs), and build the first view on each side of the band once without drawing it (which
   * lays out the rungs' first tiles and rasterises their text). The same GL state is saved and
   * put back as for a draw. Nothing is drawn and the depth buffer is not cleared. Returns true
   * when done; false before the module has loaded or without a usable view.
   */
  function prepare(gl, view) {
    if (!w || !gl || !view) return false;
    const edges = view.edges || {};
    if (!Number.isFinite(edges.globe) || !Number.isFinite(edges.street)) return false;
    try {
      const g = context(gl);
      const vp = view.viewport || [gl.drawingBufferWidth, gl.drawingBufferHeight];
      const css = Number.isFinite(view.dpr) && view.dpr > 0 ? view.dpr : 1;
      const rs = Number.isFinite(view.renderScale) && view.renderScale > 0 ? view.renderScale : css;
      const W = Math.max(1, Math.round(vp[0] * rs)), H = Math.max(1, Math.round(vp[1] * rs));
      const shift = opts.zPixel === 'buffer' ? 0 : Math.log10(css / rs);
      const lo = edges.street + shift, hi = edges.globe + shift;
      if (lo !== band.lo || hi !== band.hi) { w.hl_band(lo, hi, 0); band.lo = lo; band.hi = hi; }
      const keep = opts.restoreState !== false;
      if (keep) save(g);
      try {
        graphics(g, rs);
        g.text.beginFrame();
        const clock = clockOf(view);
        const inn = inward(view, 0.25);
        w.hl_host_weight_override(0.5, 1);
        w.hl_view(lo - 0.25, inn[0], inn[1], W, H, rs, clock);
        w.hl_view(hi + 0.25, view.yaw || 0, Math.min(Math.PI / 2, Math.max(-Math.PI / 2, view.pitch || 0)), W, H, rs, clock);
      } finally {
        if (keep) restore(g);
      }
      return true;
    } catch (e) {
      onError(e);
      return false;
    }
  }

  const adapter = {
    draw,
    prepare,
    /** What the last draw call did: { drawn, share, passes, z, yaw, pitch, width, height, ms, slot }. */
    last,
    /** The wasm exports, once loaded (for a host's own legend: hl_rungs, hl_rung_text). */
    get wasm() { return w; },
    ready: false,
  };
  if (opts.global !== false && typeof window !== 'undefined') window.HOLON_LADDER = adapter;
  const bytes = await (await fetch(wasmUrl)).arrayBuffer();
  w = (await WebAssembly.instantiate(bytes, { env })).instance.exports;
  adapter.ready = true;
  return adapter;
}
