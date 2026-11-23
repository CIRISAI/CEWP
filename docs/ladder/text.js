// holon-draw, the browser TEXT PROVIDER (INTERFACE.md §1: text is drawn, never shaped, by
// the core). One ES module, no dependency.
//
// A whole string is rasterised once with Canvas 2D, so the PLATFORM does the shaping, the
// bidirectional order and the font fallback for every script. The result is white coverage
// in an atlas page; the drawer (`./draw.js`, kind 11 `run`) tints it. Nothing here knows a
// glyph, a font file or a language.
//
//   const text = createTextProvider(gl, { dpr });         // setDpr(d) when the ratio changes
//   const drawer = createDrawer(gl, { text });
//   each frame:  text.beginFrame();  const r = text.place('PASS', 2);  ... pack a run ...
//
// place(text, style) -> { w, h, u0, v0, u1, v1, page, advance, epoch, drawn }, in DEVICE pixels.
//   - The SAME object is returned for the same (text, style) while it is cached: a cache
//     hit allocates nothing. Do not write to it.
//   - w by h is the run's rectangle: draw it at exactly that size, centred where the text
//     should be centred. h is the same for every string of a style, and the baseline is at
//     the same height in it, so runs of one style placed at one centre line share a
//     baseline. The ink starts 1 px in from the left edge and ends 1 px before the right;
//     `advance` (= w - 2) is the platform's advance for the string.
//   - An empty string gives w = 0 (draw nothing).
//   - A string wider than a page is cut at the page's width.
//   - `drawn` says whether the run shows the string as given (the screen standard's L12 asks):
//     0 whole, 1 CUT (wider than a page), 2 CHANGED (its style changed the case). A host passes
//     it to the core as the 8th number of `host_text` (holon-draw's `Drawn`).
//
// STYLES, by number (CSS px times dpr gives the device size):
//   0 body     13px monospace
//   1 heading  11px sans-serif bold, UPPERCASE, letter-spacing 0.08em
//   2 status   16px monospace bold
//   3 button   13px sans-serif
//   4 small    11px monospace
// A host may add its own styles with `options.styles` (an array of { px, weight ('' or
// 'bold ' or a CSS weight and a space), family (a CSS font-family list, so a webfont the page
// has loaded works), upper, spacing (em) }); they are numbered from 5 on, in the order given,
// so the five above keep their numbers. A webfont not yet loaded rasterises in a fallback: a
// host that waits for `document.fonts.ready` and then calls setDpr(provider.dpr) gets it.
// Colour is not part of a style. Letter-spacing uses the 2D context's `letterSpacing`
// where it exists; where it does not (older engines) the heading is drawn unspaced.
//
// ATLAS. Pages are `pageSize` square (2048, or the device's limit if smaller), one byte a
// texel (ALPHA), LINEAR, clamped. Each string is packed on a shelf with a 1 px clear border
// that is uploaded with it, and only its own region is uploaded (texSubImage2D).
//
// EVICTION. When no page has room, a new page is made, up to `maxPages` (4, which is all
// the drawer samples). When all are full, ONE WHOLE PAGE is emptied: the one least recently
// used (the page whose latest place() is the oldest frame), its cache entries are dropped,
// and `epoch` goes up by one. A string from that page asked for again is rasterised again.
// What this cannot protect: a rectangle the host is still holding from an emptied page now
// names texels that belong to other strings. A host that keeps runs across frames (or
// places more in ONE frame than four pages hold, in which case the page emptied was used
// this frame and `stats().overflows` counts it) compares `provider.epoch` before and after
// its layout and lays out again when it moved: ONCE. If one layout holds more than the
// atlas does, every pass empties a page, and a host that loops until the epoch is still
// never returns; that case shows as `overflows` going up and some runs drawn wrong.
//
// GL STATE. place() on a cache miss uploads; it saves and puts back the TEXTURE_2D binding
// of the active unit, UNPACK_FLIP_Y_WEBGL and UNPACK_PREMULTIPLY_ALPHA_WEBGL, and changes
// nothing else. A cache hit touches no GL state. texture(page) and stats() touch none.
//
// WEBGPU. `createTextProviderWebGPU(device, options)` is the same provider (the same
// rasterisation, shelves, place(), epoch, eviction, styles and stats; the code is shared)
// for `./draw-webgpu.js`: texture(page) is then a GPUTexture, r8unorm, coverage in red,
// made with TEXTURE_BINDING | COPY_DST | RENDER_ATTACHMENT, and a cache miss uploads its
// region with device.queue.copyExternalImageToTexture (premultipliedAlpha: true, so red is
// the coverage, as the GL page's alpha is). pageSize is capped at
// device.limits.maxTextureDimension2D. It touches no host state.

const MONO = '"SFMono-Regular", Consolas, "Liberation Mono", Menlo, monospace';
const SANS = '-apple-system, "Segoe UI", sans-serif';

export const STYLE = Object.freeze({ BODY: 0, HEADING: 1, STATUS: 2, BUTTON: 3, SMALL: 4 });

// px: CSS size; weight; family; upper: uppercase the string; spacing: letter-spacing in em.
const STYLES = [
  { px: 13, weight: '', family: MONO, upper: false, spacing: 0 },
  { px: 11, weight: 'bold ', family: SANS, upper: true, spacing: 0.08 },
  { px: 16, weight: 'bold ', family: MONO, upper: false, spacing: 0 },
  { px: 13, weight: '', family: SANS, upper: false, spacing: 0 },
  { px: 11, weight: '', family: MONO, upper: false, spacing: 0 },
];

// The first strong character decides the base direction, as the platform's own rule does.
const RTL_FIRST = /^[^A-Za-zÀ-ɏͰ-ϿЀ-ӿ֐-ࣿऀ-෿฀-๿぀-ヿ一-鿿יִ-﷿ﹰ-ﻼ]*[֐-ࣿיִ-﷿ﹰ-ﻼ]/;

const EMPTY = Object.freeze({ w: 0, h: 0, u0: 0, v0: 0, u1: 0, v1: 0, page: 0, advance: 0, epoch: 0 });

/**
 * @param {WebGLRenderingContext|WebGL2RenderingContext} gl the host's context
 * @param {{dpr?: number, pageSize?: number, maxPages?: number, styles?: Array<{px:number, weight?:string, family?:string, upper?:boolean, spacing?:number}>}} [options]
 */
export function createTextProvider(gl, options) {
  return makeProvider({
    maxTexture: () => gl.getParameter(gl.MAX_TEXTURE_SIZE) | 0,
    create(pageSize) {
      const tex = gl.createTexture();
      const prev = gl.getParameter(gl.TEXTURE_BINDING_2D);
      gl.bindTexture(gl.TEXTURE_2D, tex);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.ALPHA, pageSize, pageSize, 0, gl.ALPHA, gl.UNSIGNED_BYTE, null);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
      gl.bindTexture(gl.TEXTURE_2D, prev);
      return tex;
    },
    upload(tex, x, y, canvas) {
      const prev = gl.getParameter(gl.TEXTURE_BINDING_2D);
      const flip = gl.getParameter(gl.UNPACK_FLIP_Y_WEBGL);
      const premul = gl.getParameter(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL);
      gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, false);
      gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, false);
      gl.bindTexture(gl.TEXTURE_2D, tex);
      gl.texSubImage2D(gl.TEXTURE_2D, 0, x, y, gl.ALPHA, gl.UNSIGNED_BYTE, canvas);
      gl.bindTexture(gl.TEXTURE_2D, prev);
      gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, flip);
      gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, premul);
    },
    remove(tex) { gl.deleteTexture(tex); },
  }, options);
}

/**
 * The same provider for a WebGPU host (`./draw-webgpu.js`); texture(page) is a GPUTexture.
 * @param {GPUDevice} device the host's device
 * @param {{dpr?: number, pageSize?: number, maxPages?: number, styles?: Array<{px:number, weight?:string, family?:string, upper?:boolean, spacing?:number}>}} [options]
 */
export function createTextProviderWebGPU(device, options) {
  const T = GPUTextureUsage;
  return makeProvider({
    maxTexture: () => device.limits.maxTextureDimension2D | 0,
    create(pageSize) {
      return device.createTexture({ label: 'holon-draw atlas page', size: [pageSize, pageSize], format: 'r8unorm', usage: T.TEXTURE_BINDING | T.COPY_DST | T.RENDER_ATTACHMENT });
    },
    upload(tex, x, y, canvas) {
      device.queue.copyExternalImageToTexture({ source: canvas, flipY: false }, { texture: tex, origin: [x, y], premultipliedAlpha: true }, [canvas.width, canvas.height]);
    },
    remove(tex) { tex.destroy(); },
  }, options);
}

// The provider, whatever holds its pages: `gpu` makes a page, uploads a region from the
// scratch canvas, and deletes a page; everything else is here, once.
function makeProvider(gpu, options) {
  options = options || {};
  let dpr = options.dpr > 0 ? +options.dpr : (typeof devicePixelRatio === 'number' ? devicePixelRatio : 1);
  const maxTex = gpu.maxTexture();
  const pageSize = Math.max(64, Math.min(options.pageSize | 0 || 2048, maxTex));
  const maxPages = Math.max(1, Math.min(options.maxPages | 0 || 4, 4));

  // The scratch surface the platform draws on. Resized to each string, which also clears it.
  const canvas = typeof OffscreenCanvas !== 'undefined' ? new OffscreenCanvas(4, 4) : document.createElement('canvas');
  const ctx = canvas.getContext('2d');
  if (!ctx) throw new Error('holon-draw text: no 2D canvas context to rasterise with');
  const hasSpacing = 'letterSpacing' in ctx;

  const defs = STYLES.concat((options.styles || []).map((s) => ({
    px: +s.px > 0 ? +s.px : 13, weight: s.weight || '', family: s.family || SANS, upper: !!s.upper, spacing: +s.spacing || 0,
  })));
  // Per style: the font string, the line box and the baseline in it, in device pixels.
  function metrics(s, out) {
    const size = s.px * dpr;
    const h = Math.ceil(size * 1.7);
    out.font = s.weight + size + 'px ' + s.family;
    out.upper = s.upper;
    out.spacing = s.spacing > 0 ? s.spacing * size + 'px' : '0px';
    out.h = h;
    // Capitals are about 0.72 em tall: put their middle on the box's middle.
    out.baseline = Math.round(h * 0.5 + size * 0.36);
    return out;
  }
  const styles = defs.map((s) => metrics(s, { cache: new Map() /* text -> entry */ }));

  // pages[i]: { tex, shelves: [{ y, h, x }], nextY, used (frame of the latest place), entries: [[styleIndex, text]] }
  const pages = [];
  const st = { pages: 0, entries: 0, rasterised: 0, evictions: 0, overflows: 0, epoch: 0 };
  let frame = 0;
  let dead = false;

  function newPage() {
    const tex = gpu.create(pageSize);
    const p = { tex, shelves: [], nextY: 0, used: frame, entries: [] };
    pages.push(p);
    st.pages = pages.length;
    return p;
  }

  // A cw by ch cell on a shelf of page p: its x and y, or -1 in `cell.x` if it does not fit.
  const cell = { x: 0, y: 0 };
  function shelve(p, cw, ch) {
    for (let i = 0; i < p.shelves.length; i++) {
      const s = p.shelves[i];
      if (s.h >= ch && s.h <= ch + (ch >> 2) && s.x + cw <= pageSize) {
        cell.x = s.x; cell.y = s.y; s.x += cw;
        return true;
      }
    }
    if (p.nextY + ch <= pageSize && cw <= pageSize) {
      p.shelves.push({ y: p.nextY, h: ch, x: cw });
      cell.x = 0; cell.y = p.nextY; p.nextY += ch;
      return true;
    }
    return false;
  }

  function evict() {
    let k = 0;
    for (let i = 1; i < pages.length; i++) if (pages[i].used < pages[k].used) k = i;
    const p = pages[k];
    if (p.used === frame) st.overflows++;
    for (let i = 0; i < p.entries.length; i++) {
      const e = p.entries[i];
      styles[e[0]].cache.delete(e[1]);
    }
    st.entries -= p.entries.length;
    p.entries.length = 0;
    p.shelves.length = 0;
    p.nextY = 0;
    st.evictions++;
    st.epoch++;
    provider.epoch = st.epoch;
    return k;
  }

  function rasterise(text, si) {
    const s = styles[si];
    const shown = s.upper ? text.toUpperCase() : text;
    const rtl = RTL_FIRST.test(shown);
    const setup = () => {
      ctx.font = s.font;
      if (hasSpacing) ctx.letterSpacing = s.spacing;
      ctx.direction = rtl ? 'rtl' : 'ltr';
      ctx.textAlign = 'left';
      ctx.textBaseline = 'alphabetic';
    };
    setup();
    const full = Math.ceil(ctx.measureText(shown).width);
    const advance = Math.min(full, pageSize - 4);
    const drawn = full > pageSize - 4 ? 1 : shown !== text ? 2 : 0;
    const w = advance + 2, h = s.h;       // the run's rectangle
    const cw = w + 2, ch = h + 2;         // with the 1 px clear border around it

    let pi = -1;
    for (let i = 0; i < pages.length && pi < 0; i++) if (shelve(pages[i], cw, ch)) pi = i;
    if (pi < 0 && pages.length < maxPages) {
      newPage();
      pi = pages.length - 1;
      shelve(pages[pi], cw, ch);
    }
    if (pi < 0) {
      pi = evict();
      shelve(pages[pi], cw, ch);
    }
    const p = pages[pi];

    canvas.width = cw;   // resizing clears, and resets the context's state
    canvas.height = ch;
    setup();
    ctx.fillStyle = '#fff';
    ctx.fillText(shown, 2, 1 + s.baseline);

    gpu.upload(p.tex, cell.x, cell.y, canvas);

    const x = cell.x + 1, y = cell.y + 1;
    const e = { w, h, u0: x / pageSize, v0: y / pageSize, u1: (x + w) / pageSize, v1: (y + h) / pageSize, page: pi, advance, epoch: st.epoch, drawn };
    s.cache.set(text, e);
    p.entries.push([si, text]);
    p.used = frame;
    st.entries++;
    st.rasterised++;
    return e;
  }

  const provider = {
    dpr,
    pageSize,
    /** Goes up by one each time a page is emptied; rectangles from before may be stale. */
    epoch: 0,
    /** The line box of a style in device pixels: every run of the style has this h. */
    lineHeight(style) {
      const s = styles[style | 0];
      return s ? s.h : 0;
    },
    place(text, style) {
      if (dead) throw new Error('holon-draw text: place() after destroy()');
      const si = style | 0, s = styles[si];
      if (!s) throw new Error('holon-draw text: no style ' + style + ' (styles are 0..' + (styles.length - 1) + ')');
      if (typeof text !== 'string') text = String(text);
      if (text.length === 0) return EMPTY;
      const hit = s.cache.get(text);
      if (hit) {
        pages[hit.page].used = frame;
        return hit;
      }
      return rasterise(text, si);
    },
    /** The texture of an atlas page, or null if that page does not exist. */
    texture(page) {
      const p = pages[page];
      return p ? p.tex : null;
    },
    /**
     * A new device pixel ratio: every style is measured again and every page emptied, so every
     * string is rasterised again at the new size; `epoch` goes up by one. Also the way to pick
     * up a webfont that finished loading (call it with the same ratio).
     */
    setDpr(d) {
      if (dead) throw new Error('holon-draw text: setDpr() after destroy()');
      dpr = d > 0 ? +d : 1;
      provider.dpr = dpr;
      for (let i = 0; i < styles.length; i++) { styles[i].cache.clear(); metrics(defs[i], styles[i]); }
      for (const p of pages) { p.entries.length = 0; p.shelves.length = 0; p.nextY = 0; }
      st.entries = 0;
      st.epoch++;
      provider.epoch = st.epoch;
    },
    /** Starts a frame: the page-use clock moves on and `rasterised` returns to 0. */
    beginFrame() {
      frame++;
      st.rasterised = 0;
    },
    /** The SAME object every call: { pages, entries, rasterised (since beginFrame), evictions, overflows, epoch }. */
    stats() {
      return st;
    },
    destroy() {
      if (dead) return;
      dead = true;
      for (const p of pages) gpu.remove(p.tex);
      pages.length = 0;
      for (const s of styles) s.cache.clear();
      st.pages = 0; st.entries = 0;
    },
  };
  return provider;
}
