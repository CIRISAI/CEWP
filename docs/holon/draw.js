// holon-draw, the browser backend: draws the core's packed instance batches with instancing.
//
// One ES module, no dependency. The Rust core (`../src/lib.rs`) packs a scene into batches
// of fixed 52-byte records (`Instance::pack`); this file uploads each batch as it is (or
// reads it from a buffer the host made resident, see RESIDENT BUFFERS) and draws it with one
// instanced call. Nothing is built per vertex or per record in script.
// `./draw-webgpu.js` is the WebGPU sibling: this header is the contract of both; that file's
// header says only how it is hosted and what differs.
//
// THE RECORD (little-endian, 52 bytes):
//   f32 pos[3] @0   f32 size[3] @12   f32 aux[3] @24   f32 angle @36
//   u8 rgba[4] @40 (straight alpha)   u32 tag @44 (not read here)
//   u8 kind @48   u8 detail @49   u8 flags @50 (bit 0 pinned, bit 1 coarse)   u8 variant @51
// Only bytes 0..43 reach a shader. Bytes 44..51 are uploaded and never pointed at.
//
// KINDS (world z is up):
//   1 quad, 9 patch   flat rectangle in the plane z = pos.z, size[0] by size[1], turned by angle about z
//   2 line            a thin tube from pos to aux, WIDTH size[0]
//   3 arc             a quadratic Bezier from pos to aux, WIDTH size[0], bent by its CONTROL point:
//                     the chord's midpoint raised size[1] along world z and moved size[2] along
//                     the horizontal across the chord (z x chord, unit). size[1] = size[2] = 0
//                     is the chord, which is how an arc was drawn before it could bend. Drawn
//                     on a 16-segment tube (or ribbon), so it is smooth at any width.
//   4 disc, 10 glyph  flat 24-gon in the plane z = pos.z, radius size[0]
//   5 box, 6 prism    base centre pos, size = (x, y, z extents), turned by angle about z (prism: flat top)
//                     With bit 3 of `variant` set (0x08, VARIANT_ORIENT) the box is ORIENTED: aux is
//                     its local z axis (any length; zero means world z), size.z runs along it from
//                     pos, and angle turns its local x about that axis, starting from world x
//                     projected onto the plane across the axis (world y when the axis is along x).
//                     With aux = (0, 0, 1) it is the same box as without the bit. An oriented box
//                     has no RAMP t (aux is taken), so in RAMP mode it reads t = 0.
//   7 sphere          centre pos, radius size[0]
//   8 rod             capped cylinder from pos to aux, radius size[0]
// Boxes, prisms, spheres and rods are lit. Flat marks and lines are drawn in their exact
// colour unless `opts.flatLit` is set.
//
// MATERIAL of a LIT mark (box, prism, sphere, rod), bits 4..5 of `variant` (VARIANT_MATERIAL
// masks them), a closed list:
//   0x00 KIND    the kind's specular and rim (or the batch's `shade`)
//   0x10 MATTE   no specular, a quarter of the rim
//   0x20 METAL   three times the specular, tighter (exponent 90), a stronger rim, darker diffuse
//   0x30 GLOW    unlit: the colour as given, as a light source looks; it neither casts nor
//                receives a shadow
// Unlit kinds ignore these bits (a glyph's variant is its shape).
//
// SHADOWS (`opts.shadow`, off without it): { center: [x, y, z], radius, size = 1024,
// strength = 0.55, bias = 0.0006 }. The lookup is taken a texel and a half out along the
// surface normal (normal offset), and the depth test has a slope-scaled bias, so a lit
// surface does not shadow itself in bands. Before the world pass, every world batch of a lit kind or
// a line or arc (tube, not flat) is drawn from the light into a size x size shadow map, an
// orthographic view along -light covering the sphere (center, radius). A batch with
// `castShadow === false` casts none; a mark whose alpha (with FADE and RING) is under one
// half casts none. In the world pass every world mark receives: a lit mark's direct light is
// scaled by the lit fraction (four taps on a rotated grid), an unlit mark's colour by
// mix(1 - strength, 1, lit fraction). GLOW marks neither cast nor receive. The map is RGBA8
// with depth packed in it (no extension, WebGL 1 and 2 alike). On a device with no high
// float in the fragment stage the test is coarse. The shadow pass binds its own framebuffer
// and sets its own viewport, then puts back the framebuffer binding and the viewport it found
// (`opts.viewport`, or the GL viewport, read once). The map is bound on texture unit 4.
//
// SCREEN BATCHES (`batch.screen === true`): positions are DEVICE PIXELS from the viewport's
// top-left, y down; `offset` is the tile's origin in pixels; `vp` does not apply. They are
// drawn AFTER every world batch, in the order given, with the depth test off. Kinds there:
//   1 quad, 9 patch   rectangle size[0] by size[1] centred at pos, corner radius aux[0] px
//   4 disc, 10 glyph  filled circle, radius size[0] px
//   2 line, 3 arc     from pos to aux, width size[0] px, flat ends
//   11 run            a text run from the text provider's atlas (`./text.js`): rectangle
//                     size[0] by size[1] centred at pos, its top-left rounded to a whole pixel;
//                     u0 = aux[0], v0 = aux[1], u1 = aux[2], v1 = angle; atlas page = variant
//                     (byte 51, pages 0..3); colour = rgba.rgb, alpha = rgba.a * coverage.
//   12 image          a rectangle of a HOST texture, laid out as a run: size[0] by size[1]
//                     centred at pos, top-left on a whole pixel, u0 = aux[0], v0 = aux[1],
//                     u1 = aux[2], v1 = angle; texture SLOT = variant (byte 51, 0..255), set
//                     with setImage(); colour = texture.rgb * rgba.rgb, alpha = texture.a *
//                     rgba.a. The core puts one slot in a batch, so the first record names it.
//                     A batch whose slot holds no texture is skipped (stats().missingImages).
// Shapes are cut by a signed distance in the fragment stage, with about 1 px of antialiasing.
// Other kinds in a screen batch, a run or an image in a world batch, and a run with no provider
// bound are skipped. The viewport is `opts.viewport` ([x, y, w, h], as GL has it) or, without it,
// gl.getParameter(gl.VIEWPORT), which allocates: a host that draws screen batches every
// frame passes `opts.viewport`.
//
// RESIDENT BUFFERS. A batch's records either arrive with it (`bytes`: uploaded on every
// draw(), into this file's own per-kind buffers) or already live on the GPU:
//   const h = drawer.resident(bytes)   one buffer, uploaded once (GL: STATIC_DRAW); h is a
//                                      positive integer. bytes.byteLength is a multiple of 52.
//   drawer.update(h, bytes)            new contents: written in place when they fit the
//                                      buffer's allocation (a shrink keeps the allocation),
//                                      else the buffer is reallocated; the handle stays h
//   drawer.release(h)                  frees it; h may be handed out again by resident()
//   batch = { kind, handle: h, first, count, offset, params, screen?, coarse?, shade?,
//             castShadow?, slot? }     records first .. first + count - 1 of that buffer,
//                                      drawn with NO upload. `first` defaults to 0.
// Everything else about a resident batch is a `bytes` batch's: one handle may hold many
// batches of any kinds (the batch's `kind` says how its range is drawn), a world one is drawn
// into the shadow map like any other (and not with `castShadow === false`), it may be mixed
// with `bytes` batches in one draw(), in any order. A draw() whose batches are all resident
// uploads no record: stats().bytesUploaded is 0 (tile settings are uniforms, counted in
// paramUploads as before). draw() throws, before it changes any state, on an unknown or
// released handle and on a range past the records the buffer holds now (after update()).
// A RESIDENT IMAGE BATCH (kind 12, screen) must carry `slot` (0..255): the texture slot that
// a `bytes` image batch reads from its first record's byte 51, which for a resident batch is
// on the GPU, not in hand. draw() throws on a resident image batch without an integer `slot`
// in 0..255. On a `bytes` image batch `slot` is not read (byte 51 names it, as before).
// stats() gains residentBuffers and residentBytes: what is resident now (bytes allocated, so
// a shrinking update() does not lower it), not reset by draw().
// WHAT A RESIDENT BATCH COSTS IN GL CALLS, after its first draw:
//   with vertex array objects (WebGL 2, or WebGL 1 with OES_vertex_array_object):
//     bind its vertex array (skipped when it is the one bound), the offset uniform, the draw.
//     Each distinct (handle, slot's mesh, first) gets its own vertex array, made on first use
//     (about 25 calls, once; the shadow pass and the picture share it) and deleted with the
//     handle; at most 256 per handle, beyond which that batch is drawn the bare way below.
//   bare WebGL 1: bind the buffer, 6 attribute pointers, the offset uniform, the draw.
//   a `bytes` batch, for comparison: the same plus its upload.
// As before, add the slot's uniforms when the slot changes from the batch before, and the
// tile settings when they differ from the last uploaded.
//
// TILE SETTINGS (`batch.params`, 32 numbers = 8 groups of 4; absent means neutral). Slots:
//   [0..2]   DIM     rgb multiplier on every colour of the tile, applied last (after lighting)
//   [3]      FADE    alpha multiplier on every mark of the tile, runs too (the core's DIM.w)
//   [4..6]   RAMP0   the colour at t = 0        [8..10]  RAMP1   the colour at t = 1
//   [12..14] TINT    rgb multiplier for TINT marks
//   [16..19] NOISE   x0, y0, sx, sy: field position = (x0, y0) + (sx, sy) * (the mark's local xy)
//   [20]     TEX     the procedural field's strength, 0 off
//   [21]     FLOOR_CASING  a CASING line's or arc's least HALF width, in pixels; 0 off
//   [24]     FLOOR   any other world line's or arc's least HALF width, in pixels; 0 off
//   [25]     A_GROUND  [26] A_CASING  [27] A_ROAD  [28] A_ROOF   alpha multiplier per surface
//                    class (below), read as given: 1 neutral, 0 hides the class.
//   [29..31] RING    centre x, centre y, radius, in device pixels from the viewport's top-left,
//                    y down: alpha *= 1 - smoothstep(0.85 r, r, distance), on every WORLD mark
//                    of the tile; radius 0 off. Screen batches do not read it. The distance
//                    is taken PER PIXEL (the twin takes it per vertex, which is the same on
//                    its short segments and wrong on a long street or a large patch, whose
//                    ends can all lie outside the ring while its middle crosses it).
//   [7] [11] [15] [22] [23] are not read.
// They are uploaded per batch only when their values differ from the last ones uploaded to
// that program (compared number by number; nothing is allocated).
//
// COLOUR MODE, the top two bits of byte 51 (`variant`), for every kind but run:
//   0x00 LITERAL  rgba as given
//   0x40 RAMP     rgb = mix(RAMP0, RAMP1, clamp(t, 0, 1)); alpha from rgba
//   0x80 TINT     rgb = rgba.rgb * TINT
// then rgb *= DIM in every mode (runs too). Where t is (the core's rule): aux[0] for world
// quad, patch, disc, glyph, box, prism and sphere; `angle` for line, arc, rod and every
// SCREEN mark.
//
// SURFACE CLASS of a WORLD mark, from its kind, its colour mode and bit 0 of `variant`.
// It picks the alpha multiplier and the procedural field. A mark in no class has neither.
//   GROUND  a patch (any mode); a quad in RAMP          A_GROUND   ground field
//   ROOF    a quad, box or prism in TINT                 A_ROOF     roof field
//   CASING  a literal quad (a cast shadow);              A_CASING   none
//           a line or arc with bit 0 SET (a casing);     A_CASING   none
//           a disc with bit 0 SET                        A_CASING   none
//   ROAD    a line or arc with bit 0 clear               A_ROAD     none
// (This is the twin's four kinds; its shadows and canopies ride in its casing kind.)
// On a DISC the two things are separate bits of `variant`: bit 0 (0x01, the core's
// twin::DISC_CASING) puts it in the casing class, with no field; bit 2 (0x04,
// twin::DISC_CANOPY_FIELD) gives it the canopy field, whatever its class. The twin's
// mottling patches are 0x01 and its trees 0x05. On a line or arc, bit 0 (0x01,
// VARIANT_CASING) is the casing and bit 1 the round ends (below). A glyph's variant is its
// shape: a glyph is in no class.
//
// LINE ENDS. A world line or arc ends FLUSH (cut square at pos and at aux) unless bit 1 of
// its `variant` is set (0x02, VARIANT_ROUND, proposed here): then each end has a round cap
// of the line's own half width (after FLOOR), so a polyline of such segments has round
// joins. It is one mesh either way: a flush line's cap vertices sit on its end. Where two
// translucent round-ended segments overlap at a join, the overlap is blended twice.
// LINE SHAPE. With the depth test on, a line is a tube (8 sides; hemispheres for round
// ends). With `opts.flatLines` it is a FLAT RIBBON in the plane that holds the line and
// the horizontal across it (half discs for round ends), which is what a map wants;
// `flatLines` defaults to true when `opts.depthTest` is false, else false. A vertical line
// has no horizontal across it that means anything: draw those as tubes.
//
// THE FIELD (the twin's FSD/twin/texture.md, as its tiling.js TLF has it), when TEX > 0:
// band-limited value noise on the grid of the field position, each octave faded out between
// 4 and 2 pixels a cycle; rgb *= 1 + sum * TEX. Luminance only. Octaves (cell, amplitude):
//   ground  48 0.10, 12 0.07, 3 0.05, each times (1 - density/2), density = a RAMP mark's t
//   canopy  6 0.16, 1.5 0.10
//   roof    4 0.05, 1 0.04
// The lattice repeats every 4096 units. THE HASH IS THE TWIN'S, BIT FOR BIT IN ITS
// ARITHMETIC, ONLY WHERE THE FRAGMENT STAGE HAS HIGH PRECISION; without it a different hash
// is used (see PRECISION), so the pattern differs there.
// Pixels per field unit, and the pixels of FLOOR, come from `opts.pixelsPerUnit` (pixels
// per WORLD unit, exact for a map) or, without it, are estimated per vertex from `vp`, the
// clip w and the viewport's height.
//
// A FLAT MAP (the twin's street tier): pass `opts.depthTest: false`. World batches are then
// painted in the order given with no depth test, so a road lies over its casing and a roof
// over a road by ORDER. With the depth test on, a line is a tube and a wider casing's top
// is above its road's. The core emits a tile's kinds in a fixed order (quad, line, arc,
// disc, ..., patch) and orders tiles by layer: a host splits a leaf into tiles on layers
// (ground, streets, buildings).
//
// PRECISION (INTERFACE.md §7): NO UNIFORM NAME APPEARS IN BOTH SHADER STAGES, in either
// program. World programs: the vertex stage owns u_vp, u_offset, u_eye, u_mode, u_pix,
// u_wview, u_params (and, with HD_SHADOW, u_lvp and u_lno); the fragment stage owns u_light,
// u_shade (and, with HD_SHADOW, u_shadow and u_shp). The shadow program is the world vertex
// stage with a fragment stage that has no uniforms. Screen program: the vertex stage owns
// u_view, u_soffset, u_smode, u_sparams; the fragment stage owns u_fmode, u_t0..u_t3. So
// there is no pair whose precision could disagree at link. The tile settings are read in
// the VERTEX stage only; what the fragment stage needs of them arrives as varyings.
// Lighting is `mediump`. Three things want more where the device has it: the field's
// position and hash, pixel distances of screen shapes, and atlas coordinates. Their
// precision is a macro, PF, defined at the top of every shader source. It is `mediump` in
// BOTH stages when gl.getShaderPrecisionFormat reports no high float for the fragment stage
// or `options.forceMediump` is set. The fragment stage is `mediump` besides wherever
// GL_FRAGMENT_PRECISION_HIGH is not defined, and names high precision only inside that
// #ifdef (the twin's gate renames the macro in fragment sources, hides the query's answer,
// and fails a fragment source that still says the word). In the mediump path the field's hash is a
// different one, whose intermediate values stay small; in the highp path it is the twin's
// own (computed on mod(i, m) / m). Varyings are declared with PF or an explicit precision
// in both stages. On a true mediump-only device the field's position, text sampling and
// large rounded shapes lose accuracy (not seen here: no such device in the test).
//
// STENCIL. This file makes no stencil call at all today: not the test, the function, the
// operation, the mask or a clear. A host's stencil test, if enabled, therefore applies to
// everything draw() draws, with the host's function and mask. THE RULE if this file ever
// does use the stencil: only bits 4..128 (mask 0xFC), with stencilMask(0xFC) while it
// writes; bits 1 and 2 are the host's and are left as found.
//
// GL STATE. draw() does not clear, does not set the viewport, does not touch the bound
// framebuffer, scissor, stencil or colour mask. It CHANGES, and leaves changed:
//   - the current program (left as one of this drawer's two programs);
//   - BLEND enabled, blendFuncSeparate(SRC_ALPHA, ONE_MINUS_SRC_ALPHA, ONE, ONE_MINUS_SRC_ALPHA)
//     (the blend equation is not touched: a host that uses another must restore FUNC_ADD first);
//   - DEPTH_TEST enabled, depthFunc(LEQUAL), depthMask(true) (the test is off while screen
//     batches are drawn, and for world batches under `opts.depthTest: false`; it is
//     enabled again on return in every case);
//   - CULL_FACE is off while flat marks are drawn and enabled again on return;
//   - CULL_FACE (left enabled), cullFace(BACK), frontFace(CCW);
//   - ARRAY_BUFFER binding (left null);
//   - the vertex array object binding, where one exists (WebGL 2 or OES_vertex_array_object):
//     left null, i.e. the default object, whose own attribute state is NOT touched
//     (resident batches are drawn through this drawer's own vertex arrays: the same rule);
//   - where no vertex array object exists (bare WebGL 1): attribute slots 0..7 (all eight) (pointers,
//     enabled flags, divisors) and the ELEMENT_ARRAY_BUFFER binding. On return slots 0..7 are
//     disabled, their divisors are 0, and ELEMENT_ARRAY_BUFFER is null;
//   - only with `opts.shadow`: the TEXTURE_2D binding of unit 4 (left on the shadow map), and
//     BLEND is off while the shadow map is drawn (enabled again for the world pass);
//   - only when a run batch is drawn: activeTexture (left TEXTURE0) and the TEXTURE_2D
//     bindings of units 0..3 (left on the atlas pages that exist; a unit whose page does
//     not exist is not touched);
//   - only when an image batch is drawn: activeTexture (left TEXTURE0) and the TEXTURE_2D
//     binding of unit 5 (left on the last image drawn).
//
// IMAGES. setImage(slot, source, { nearest }) puts a texture in a slot: `source` is anything
// texImage2D takes (ImageBitmap, ImageData, a canvas, an image or a video element; a video's
// frame is copied when called, so a host calls it again for each new frame). `nearest: true`
// samples it nearest (a QR grid, whose modules must stay square); otherwise LINEAR. Always
// clamped, never mipmapped, so any size works on WebGL 1. Not premultiplied: the blend is
// SRC_ALPHA. setImage saves and puts back the TEXTURE_2D binding of the active unit,
// UNPACK_FLIP_Y_WEBGL and UNPACK_PREMULTIPLY_ALPHA_WEBGL, and changes nothing else.
// deleteImage(slot) frees one. The core never sees pixels: what a slot holds is the host's.
// A draw() with nothing to draw changes no state.
// resident() and update() bind ARRAY_BUFFER and put back the binding they found (read with
// getParameter); they change nothing else. release() deletes the buffer and this drawer's
// vertex arrays made for it, and binds nothing (a host never has either bound: it never sees
// them). A draw() with resident batches leaves exactly the state listed above.
//
// Translucent records are blended in the order given and write depth; a host that needs
// correct layering of translucent marks orders its batches back to front.
//
// A `vp` that mirrors the picture on screen flips winding; closed meshes would then be
// culled from the wrong side. Not handled.

export const INSTANCE_BYTES = 52;

export const KIND = Object.freeze({
  QUAD: 1, LINE: 2, ARC: 3, DISC: 4, BOX: 5, PRISM: 6, SPHERE: 7, ROD: 8, PATCH: 9, GLYPH: 10, RUN: 11, IMAGE: 12,
});

// Attribute slots, bound before link. Slot 0 is a mesh attribute (never instanced), which
// desktop GL needs.
// Slot 7 is the record's four trailing bytes (kind, detail, flags, variant). Eight slots in
// all, WebGL 1's guaranteed minimum: there is no room for another attribute.
const A_P = 0, A_N = 1, I_POS = 2, I_SIZE = 3, I_AUX = 4, I_ANGLE = 5, I_COLOR = 6, I_META = 7;
const N_ATTR = 8;
const MAX_PAGES = 4;

// The top of every shader source: PF, the precision of what wants more than mediump.
// HD_MEDIUMP is defined above these lines, in BOTH stages, when the device's precision
// query reports no high float in the fragment stage or `options.forceMediump` is set. The
// fragment stage also takes the mediump path by itself where GL_FRAGMENT_PRECISION_HIGH is
// not defined. Its source says the word for high precision once, inside that #ifdef and
// nowhere else (comments included), so a gate that strips the block finds none left.
const PF_HEAD_V = `#ifdef HD_MEDIUMP
#define PF mediump
#else
#define PF highp
#endif
`;
const PF_HEAD_F = `#ifdef GL_FRAGMENT_PRECISION_HIGH
#define HD_HP highp
#endif
#if defined(HD_HP) && !defined(HD_MEDIUMP)
#define PF HD_HP
#define HD_HIGH 1
#else
#define PF mediump
#endif
`;

/** Where each setting is in a tile's 32 numbers (see TILE SETTINGS at the top). */
export const PARAM = Object.freeze({ DIM: 0, FADE: 3, RAMP0: 4, RAMP1: 8, TINT: 12, NOISE: 16, TEX: 20, FLOOR_CASING: 21, FLOOR: 24, A_GROUND: 25, A_CASING: 26, A_ROAD: 27, A_ROOF: 28, RING: 29 });
/** Bit 0 of `variant` on a line or arc: a casing. */
export const VARIANT_CASING = 0x01;
/** On a disc: bit 0 the casing class (no field), bit 2 the canopy field. A tree is both (0x05). */
export const DISC_CASING = 0x01, DISC_CANOPY_FIELD = 0x04;
/** Bit 1 of `variant` on a line or arc: round ends (a cap of the line's half width at each). */
export const VARIANT_ROUND = 0x02;
/** Bit 3 of `variant` on a box or prism: oriented, aux is its local z axis (see KINDS). */
export const VARIANT_ORIENT = 0x08;
/** Bits 4..5 of `variant` on a lit mark: its material (see MATERIAL at the top). */
export const VARIANT_MATERIAL = 0x30;
export const MATERIAL = Object.freeze({ KIND: 0x00, MATTE: 0x10, METAL: 0x20, GLOW: 0x30 });
/** Tile settings that leave every colour as its record gives it (the core's PARAMS_NEUTRAL). */
export const PARAMS_NEUTRAL = Object.freeze([1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 0]);
export const COLOUR = Object.freeze({ LITERAL: 0x00, RAMP: 0x40, TINT: 0x80 });

const VS = `
precision highp float;
attribute vec3 a_p;
attribute vec3 a_n;
attribute vec3 i_pos;
attribute vec3 i_size;
attribute vec3 i_aux;
attribute float i_angle;
attribute vec4 i_color;
attribute vec4 i_meta;     // kind, detail, flags, variant as 0..255
uniform mat4 u_vp;
uniform vec3 u_offset;
uniform vec4 u_eye;        // homogeneous eye: xyz - p*w points from p to the viewer
uniform vec2 u_mode;       // x: 0 sphere, 1 tube, 2 scaled-and-turned, 3 bent tube (arc); y: radius scale
uniform vec3 u_pix;        // half the viewport's height; pixels per world unit if given, else 0; kind code
                           // (0 other, 1 quad, 2 patch, 3 line or arc, 4 disc, 5 box or prism)
uniform vec2 u_wview;      // the viewport's width and height in pixels
uniform vec4 u_params[8];  // the tile's settings
#ifdef HD_SHADOW
uniform mat4 u_lvp;        // the light's view-projection (shadows)
uniform float u_lno;       // the normal offset for the shadow lookup, in world units
varying PF vec4 v_ls;      // the position in the light's clip space
#endif
varying mediump float v_mat; // the material (0 kind, 1 matte, 2 metal, 3 glow)
varying mediump vec3 v_n;
varying mediump vec3 v_v;
varying lowp vec4 v_c;
varying mediump vec4 v_dim;     // rgb: DIM; a: the alpha multiplier (class times FADE)
varying PF vec2 v_np;      // the field's position
varying PF vec3 v_fp;      // pixels per field unit; the field's amplitude (0 off); which field (0 ground, 1 canopy, 2 roof)
varying PF vec2 v_ring;    // (pixel - ring centre) / ring radius; 0 when the ring is off
// Pixels per world unit at clip w.
float ppuAt(float w) {
  return u_pix.y > 0.0 ? u_pix.y
    : u_pix.x * length(vec3(u_vp[0][1], u_vp[1][1], u_vp[2][1])) / max(w, 1e-6);
}
void main() {
  float kc = u_pix.z;
  float var = floor(i_meta.w + 0.5);
  float bit0 = mod(var, 2.0);
  float bit1 = mod(floor(var / 2.0), 2.0);
  float bit2 = mod(floor(var / 4.0), 2.0);
  bool orient = kc > 4.5 && kc < 5.5 && mod(floor(var / 8.0), 2.0) > 0.5;
  bool liney = kc > 2.5 && kc < 3.5;
  vec3 p;
  vec3 n;
  vec3 o = i_pos + u_offset;
  if (u_mode.x < 0.5) {
    n = a_n;
    p = o + a_p * i_size.x;
  } else if (u_mode.x < 1.5 || u_mode.x > 2.5) {
    vec3 d = i_aux - i_pos;
    // Along the axis: t in [0, 1] on the body; a cap's vertices lie beyond, by e radii.
    float t = clamp(a_p.z, 0.0, 1.0);
    vec3 c0 = o + d * t;     // the centre line's point at t
    vec3 dd = d;             // its direction
    if (u_mode.x > 2.5) {
      // An arc: the quadratic Bezier o -> ctrl -> o + d, ctrl from size[1] (up) and size[2] (across).
      vec3 hz = vec3(d.xy, 0.0);
      float hl = length(hz);
      vec3 across = hl > 1e-20 ? vec3(-hz.y, hz.x, 0.0) / hl : vec3(0.0, 1.0, 0.0);
      vec3 ctrl = 0.5 * d + vec3(0.0, 0.0, i_size.y) + across * i_size.z;
      float u = 1.0 - t;
      c0 = o + 2.0 * u * t * ctrl + t * t * d;
      dd = 2.0 * u * ctrl + 2.0 * t * (d - ctrl);
    }
    float len = length(dd);
    vec3 dn = len > 1e-20 ? dd / len : vec3(0.0, 0.0, 1.0);
    vec3 up = abs(dn.z) < 0.999 ? vec3(0.0, 0.0, 1.0) : vec3(1.0, 0.0, 0.0);
    vec3 bx = normalize(cross(up, dn));
    vec3 by = cross(dn, bx);
    float r = i_size.x * u_mode.y;
    float e = liney && bit1 > 0.5 ? a_p.z - t : 0.0;
    float fl = liney ? (bit0 > 0.5 ? u_params[5].y : u_params[6].x) : 0.0;
    if (fl > 0.0) {
      // the street floor: at least this many pixels of half width, measured on the centre line
      r = max(r, fl / ppuAt((u_vp * vec4(c0, 1.0)).w));
    }
    p = c0 + (bx * a_p.x + by * a_p.y) * r + dn * (e * r);
    n = bx * a_n.x + by * a_n.y + dn * a_n.z;
  } else {
    vec3 s = u_mode.y > 0.5 ? i_size.xxx : i_size;
    float c = cos(i_angle);
    float sn = sin(i_angle);
    vec3 q = a_p * s;
    if (orient) {
      // The box's own frame: z along aux; x from world x (world y if aux is along x) laid
      // across it, then turned by angle about z.
      float al = length(i_aux);
      vec3 ez = al > 1e-20 ? i_aux / al : vec3(0.0, 0.0, 1.0);
      vec3 r0 = abs(ez.x) < 0.999 ? vec3(1.0, 0.0, 0.0) : vec3(0.0, 1.0, 0.0);
      vec3 x0 = normalize(r0 - ez * dot(r0, ez));
      vec3 y0 = cross(ez, x0);
      vec3 ex = c * x0 + sn * y0;
      vec3 ey = cross(ez, ex);
      p = o + ex * q.x + ey * q.y + ez * q.z;
      n = ex * a_n.x + ey * a_n.y + ez * a_n.z;
    } else {
      p = o + vec3(c * q.x - sn * q.y, sn * q.x + c * q.y, q.z);
      n = vec3(c * a_n.x - sn * a_n.y, sn * a_n.x + c * a_n.y, a_n.z);
    }
  }
  v_n = n;
  v_v = normalize(u_eye.xyz - p * u_eye.w);
  float cm = floor(var / 64.0);
  bool ramp = cm > 0.5 && cm < 1.5;
  bool tint = cm > 1.5 && cm < 2.5;
  bool tube = u_mode.x > 0.5 && u_mode.x < 1.5 || u_mode.x > 2.5;
  vec3 rgb = i_color.rgb;
  float dens = 0.0;
  if (ramp) {
    dens = clamp(tube ? i_angle : orient ? 0.0 : i_aux.x, 0.0, 1.0);
    rgb = mix(u_params[1].rgb, u_params[2].rgb, dens);
  } else if (tint) {
    rgb *= u_params[3].rgb;
  }
  // The surface class: its alpha multiplier (1 for a mark in no class) and its field (-1 none).
  float am = 1.0;
  float fk = -1.0;
  float fa = 1.0;
  if (kc > 1.5 && kc < 2.5 || kc > 0.5 && kc < 1.5 && ramp) {
    am = u_params[6].y; fk = 0.0; fa = 1.0 - 0.5 * dens;                 // ground
  } else if (tint && (kc > 0.5 && kc < 1.5 || kc > 4.5)) {
    am = u_params[7].x; fk = 2.0;                                        // roof
  } else if (kc > 0.5 && kc < 1.5) {
    am = u_params[6].z;                                                  // a literal quad: a shadow
  } else if (kc > 2.5 && kc < 3.5) {
    am = bit0 > 0.5 ? u_params[6].z : u_params[6].w;                     // casing, road
  } else if (kc > 3.5 && kc < 4.5) {
    if (bit0 > 0.5) am = u_params[6].z;                                  // a disc in the casing class
    if (bit2 > 0.5) fk = 1.0;                                            // a disc with the canopy field
  }
  gl_Position = u_vp * vec4(p, 1.0);
#ifdef HD_SHADOW
  v_ls = u_lvp * vec4(p + n * u_lno, 1.0);
#endif
  v_mat = mod(floor(var / 16.0), 4.0);
  // The ring: the pixel's place relative to the centre, in radii, for the fragment stage.
  float ringR = u_params[7].w;
  if (ringR > 0.0) {
    vec2 px = vec2(0.5 + 0.5 * gl_Position.x / gl_Position.w, 0.5 - 0.5 * gl_Position.y / gl_Position.w) * u_wview;
    v_ring = (px - u_params[7].yz) / ringR;
  } else {
    v_ring = vec2(0.0);
  }
  v_c = vec4(rgb, i_color.a);
  v_dim = vec4(u_params[0].rgb, am * u_params[0].w);
  float amp = fk < -0.5 ? 0.0 : u_params[5].x * fa;
  if (amp > 0.0) {
    vec2 pl = (p.xy - o.xy) + i_pos.xy;
    v_np = u_params[4].xy + u_params[4].zw * pl;
    v_fp = vec3(ppuAt(gl_Position.w) / max(abs(u_params[4].z), 1e-6), amp, fk);
  } else {
    v_np = vec2(0.0);
    v_fp = vec3(0.0);
  }
}
`;

const FS = `
precision mediump float;
uniform vec3 u_light;  // unit, toward the light
uniform vec4 u_shade;  // x lit (0 or 1), y ambient, z specular, w rim
#ifdef HD_SHADOW
uniform sampler2D u_shadow;
uniform vec4 u_shp;    // x strength, y one texel, z bias
varying PF vec4 v_ls;
#endif
varying mediump float v_mat;
varying mediump vec3 v_n;
varying mediump vec3 v_v;
varying lowp vec4 v_c;
varying mediump vec4 v_dim;     // rgb: DIM; a: the alpha multiplier (class times FADE)
varying PF vec2 v_np;
varying PF vec3 v_fp;
varying PF vec2 v_ring;
// Value noise on a lattice that repeats every 4096 units. The hash is taken on
// mod(i, m) / m, which lies in [0, 1): at PF's better precision it is then the twin's own
// hash; at mediump, one whose intermediate values stay under a few hundred, inside range.
PF float h2(PF vec2 i, PF float m) {
  PF vec2 q = mod(i, m) / m;
#ifdef HD_HIGH
  return fract(sin(dot(q, vec2(12.9898, 78.233)) * m) * 43758.5453);
#else
  return fract(sin(dot(q, vec2(127.1, 311.7))) * 43.7585);
#endif
}
PF float vn(PF vec2 x, PF float cell) {
  PF vec2 g = x / cell;
  PF vec2 i = floor(g);
  PF vec2 t = fract(g);
  t = t * t * (3.0 - 2.0 * t);
  PF float m = 4096.0 / cell;
  return mix(mix(h2(i, m), h2(i + vec2(1.0, 0.0), m), t.x),
             mix(h2(i + vec2(0.0, 1.0), m), h2(i + vec2(1.0, 1.0), m), t.x), t.y) * 2.0 - 1.0;
}
// An octave, faded out between 4 and 2 pixels a cycle.
PF float oct(PF vec2 x, PF float cell, PF float amp) {
  PF float w = 1.0 - smoothstep(0.25, 0.5, 1.0 / (v_fp.x * cell));
  return w > 0.0 ? amp * w * vn(x, cell) : 0.0;
}
#ifdef HD_SHADOW
PF float unpackDepth(PF vec4 c) {
  return dot(c, vec4(1.0, 1.0 / 255.0, 1.0 / 65025.0, 1.0 / 16581375.0));
}
// The lit fraction at this pixel: 1 in light, 0 in full shadow; four taps on a rotated grid.
float litFraction(float ndl) {
  PF vec3 q = v_ls.xyz / v_ls.w * 0.5 + 0.5;
  if (q.x < 0.0 || q.x > 1.0 || q.y < 0.0 || q.y > 1.0 || q.z > 1.0) return 1.0;
  PF float bias = u_shp.z * (1.0 + 3.0 * (1.0 - ndl));
  PF float z = q.z - bias;
  PF float t = u_shp.y;
  PF float s = (z > unpackDepth(texture2D(u_shadow, q.xy + vec2(-0.4, -1.2) * t)) ? 0.0 : 1.0)
             + (z > unpackDepth(texture2D(u_shadow, q.xy + vec2(1.2, -0.4) * t)) ? 0.0 : 1.0)
             + (z > unpackDepth(texture2D(u_shadow, q.xy + vec2(0.4, 1.2) * t)) ? 0.0 : 1.0)
             + (z > unpackDepth(texture2D(u_shadow, q.xy + vec2(-1.2, 0.4) * t)) ? 0.0 : 1.0);
  return s * 0.25;
}
#endif
void main() {
  vec3 n = normalize(v_n);
  if (!gl_FrontFacing) n = -n;
  vec3 v = normalize(v_v);
  float ndl = max(dot(n, u_light), 0.0);
  float glow = u_shade.x > 0.5 && v_mat > 2.5 ? 1.0 : 0.0;
#ifdef HD_SHADOW
  float shf = glow > 0.5 ? 1.0 : litFraction(u_shade.x > 0.5 ? ndl : 1.0);
  float strength = u_shp.x;
#else
  float shf = 1.0;
  float strength = 0.0;
#endif
  float sky = 0.75 + 0.25 * n.z;
  float lit = u_shade.y * sky + (1.0 - u_shade.y) * ndl * shf;
  vec3 h = normalize(u_light + v);
  float spec = u_shade.z, shin = 40.0, rimk = u_shade.w;
  if (v_mat > 0.5 && v_mat < 1.5) { spec = 0.0; rimk *= 0.25; }
  else if (v_mat > 1.5 && v_mat < 2.5) { spec = min(1.0, spec * 3.0 + 0.15); shin = 90.0; rimk = rimk * 1.6 + 0.06; lit *= 0.85; }
  float sp = pow(max(dot(n, h), 0.0), shin) * spec * step(0.0001, ndl) * shf;
  float rim = pow(1.0 - max(dot(n, v), 0.0), 3.0) * rimk;
  vec3 base = v_c.rgb;
  if (v_fp.y > 0.0) {
    PF float s;
    if (v_fp.z < 0.5) s = oct(v_np, 48.0, 0.10) + oct(v_np, 12.0, 0.07) + oct(v_np, 3.0, 0.05);
    else if (v_fp.z < 1.5) s = oct(v_np, 6.0, 0.16) + oct(v_np, 1.5, 0.10);
    else s = oct(v_np, 4.0, 0.05) + oct(v_np, 1.0, 0.04);
    base *= 1.0 + s * v_fp.y;
  }
  vec3 shaded = glow > 0.5 ? base : base * (lit + rim) + vec3(sp);
  vec3 flatc = base * mix(1.0 - strength, 1.0, shf);
  // min() keeps the length inside mediump's range far outside the ring
  float ring = 1.0 - smoothstep(0.85, 1.0, length(min(abs(v_ring), vec2(8.0))));
  gl_FragColor = vec4(mix(flatc, shaded, u_shade.x) * v_dim.rgb, v_c.a * v_dim.a * ring);
}
`;

// The shadow map's fragment stage: depth packed in RGBA8. No uniforms (so none can be shared
// with the vertex stage). A mark under half alpha, or a GLOW mark, casts nothing.
const ZFS = `
precision mediump float;
varying mediump float v_mat;
varying lowp vec4 v_c;
varying mediump vec4 v_dim;
varying PF vec2 v_ring;
void main() {
  float ring = 1.0 - smoothstep(0.85, 1.0, length(min(abs(v_ring), vec2(8.0))));
  if (v_c.a * v_dim.a * ring < 0.5 || v_mat > 2.5) discard;
  PF vec4 e = fract(gl_FragCoord.z * vec4(1.0, 255.0, 65025.0, 16581375.0));
  e -= e.yzww * vec4(1.0 / 255.0, 1.0 / 255.0, 1.0 / 255.0, 0.0);
  gl_FragColor = e;
}
`;

// The screen program.
const SVS = `
precision highp float;
attribute vec3 a_p;
attribute vec3 i_pos;
attribute vec3 i_size;
attribute vec3 i_aux;
attribute float i_angle;
attribute vec4 i_color;
attribute vec4 i_meta;   // kind, detail, flags, variant as 0..255
uniform vec2 u_view;     // viewport width and height in pixels
uniform vec3 u_soffset;
uniform float u_smode;   // 0 rectangle, 1 circle, 2 line, 3 run or image
uniform vec4 u_sparams[8];
varying PF vec2 v_a;     // max(v_a, v_b) = |local| - (half - radius): small near every edge
varying PF vec2 v_b;
varying PF vec2 v_uv;
varying PF float v_r;
varying mediump float v_page;
varying lowp vec4 v_c;
varying mediump float v_fade;
void main() {
  vec2 o = i_pos.xy + u_soffset.xy;
  vec2 px;
  if (u_smode > 2.5) {
    vec2 tl = floor(o - 0.5 * i_size.xy + 0.5);
    vec2 t = a_p.xy + 0.5;
    px = tl + t * i_size.xy;
    v_uv = mix(i_aux.xy, vec2(i_aux.z, i_angle), t);
    v_a = vec2(0.0);
    v_b = vec2(0.0);
    v_r = 0.0;
    v_page = i_meta.w;
  } else {
    vec2 hs;
    float r = 0.0;
    vec2 ux = vec2(1.0, 0.0);
    vec2 c = o;
    if (u_smode < 0.5) {
      hs = 0.5 * abs(i_size.xy);
      r = i_aux.x;
    } else if (u_smode < 1.5) {
      hs = vec2(abs(i_size.x));
      r = hs.x;
    } else {
      vec2 d = i_aux.xy - i_pos.xy;
      float len = length(d);
      if (len > 1e-6) ux = d / len;
      c = o + 0.5 * d;
      hs = vec2(0.5 * len, 0.5 * abs(i_size.x));
    }
    r = clamp(r, 0.0, min(hs.x, hs.y));
    vec2 l = a_p.xy * 2.0 * (hs + 1.0);   // one pixel of margin for the antialiased edge
    px = c + ux * l.x + vec2(-ux.y, ux.x) * l.y;
    vec2 inner = hs - r;
    v_a = l - inner;
    v_b = -l - inner;
    v_r = r;
    v_uv = vec2(0.0);
    v_page = 0.0;
  }
  vec3 rgb = i_color.rgb;
  if (u_smode < 2.5) {
    float cm = floor(i_meta.w / 64.0 + 0.01);
    if (cm > 0.5 && cm < 1.5) rgb = mix(u_sparams[1].rgb, u_sparams[2].rgb, clamp(i_angle, 0.0, 1.0));
    else if (cm > 1.5 && cm < 2.5) rgb *= u_sparams[3].rgb;
  }
  v_c = vec4(rgb * u_sparams[0].rgb, i_color.a);
  v_fade = u_sparams[0].w;
  gl_Position = vec4(px.x / u_view.x * 2.0 - 1.0, 1.0 - px.y / u_view.y * 2.0, 0.0, 1.0);
}
`;

const SFS = `
precision PF float;
uniform sampler2D u_t0;
uniform sampler2D u_t1;
uniform sampler2D u_t2;
uniform sampler2D u_t3;
uniform sampler2D u_img;
uniform mediump float u_fmode;   // 0 shape, 1 run, 2 image
varying PF vec2 v_a;
varying PF vec2 v_b;
varying PF vec2 v_uv;
varying PF float v_r;
varying mediump float v_page;
varying lowp vec4 v_c;
varying mediump float v_fade;
void main() {
  float cov;
  if (u_fmode > 1.5) {
    vec4 im = texture2D(u_img, v_uv);
    gl_FragColor = vec4(im.rgb * v_c.rgb, im.a * v_c.a * v_fade);
    return;
  }
  if (u_fmode > 0.5) {
    if (v_page < 0.5) cov = texture2D(u_t0, v_uv).a;
    else if (v_page < 1.5) cov = texture2D(u_t1, v_uv).a;
    else if (v_page < 2.5) cov = texture2D(u_t2, v_uv).a;
    else cov = texture2D(u_t3, v_uv).a;
  } else {
    vec2 q = max(v_a, v_b);
    float d = length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - v_r;
    cov = clamp(0.5 - d, 0.0, 1.0);
  }
  gl_FragColor = vec4(v_c.rgb, v_c.a * cov * v_fade);
}
`;

// ---- unit meshes: interleaved position (3) and normal (3), 16-bit indices ----

function sphereMesh(segs, stacks) {
  const v = [], ix = [];
  for (let i = 0; i <= stacks; i++) {
    const t = (Math.PI * i) / stacks, st = Math.sin(t), ct = Math.cos(t);
    for (let j = 0; j <= segs; j++) {
      const f = (2 * Math.PI * j) / segs;
      const x = st * Math.cos(f), y = st * Math.sin(f);
      v.push(x, y, ct, x, y, ct);
    }
  }
  const w = segs + 1;
  for (let i = 0; i < stacks; i++) {
    for (let j = 0; j < segs; j++) {
      const a = i * w + j, b = a + w, c = b + 1, d = a + 1;
      if (i > 0) ix.push(a, c, d);
      if (i < stacks - 1) ix.push(a, b, c);
    }
  }
  return { v, ix };
}

// A tube along +z from 0 to 1, radius 1.
function tubeMesh(sides, caps) {
  const v = [], ix = [];
  for (let j = 0; j <= sides; j++) {
    const f = (2 * Math.PI * j) / sides, x = Math.cos(f), y = Math.sin(f);
    v.push(x, y, 0, x, y, 0, x, y, 1, x, y, 0);
  }
  for (let j = 0; j < sides; j++) {
    const a = 2 * j, b = a + 2, c = b + 1, d = a + 1;
    ix.push(a, b, c, a, c, d);
  }
  if (caps) {
    for (let z = 0; z <= 1; z++) {
      const base = v.length / 6, nz = z ? 1 : -1;
      v.push(0, 0, z, 0, 0, nz);
      for (let j = 0; j < sides; j++) {
        const f = (2 * Math.PI * j) / sides;
        v.push(Math.cos(f), Math.sin(f), z, 0, 0, nz);
      }
      for (let j = 0; j < sides; j++) {
        const p = base + 1 + j, q = base + 1 + ((j + 1) % sides);
        if (z) ix.push(base, p, q); else ix.push(base, q, p);
      }
    }
  }
  return { v, ix };
}

// x and y in [-1/2, 1/2], z in [0, 1].
// A line as a tube: a surface of revolution about +z, radius 1, body from z = 0 to 1, with a
// hemisphere beyond each end (z in [-1, 0] and [1, 2]). The vertex stage takes
// clamp(z, 0, 1) as the place on the body and the rest as the cap's reach in radii, so a
// flush line folds the caps onto its ends. Normals are the unit profile's.
function capsuleMesh(sides, rings) {
  const prof = [];   // [radius, z, normal z]
  for (let k = rings; k >= 0; k--) { const f = (Math.PI / 2) * (k / rings); prof.push([Math.cos(f), -Math.sin(f), -Math.sin(f)]); }
  for (let k = 0; k <= rings; k++) { const f = (Math.PI / 2) * (k / rings); prof.push([Math.cos(f), 1 + Math.sin(f), Math.sin(f)]); }
  const v = [], ix = [], w = sides + 1;
  for (const [r, z, nz] of prof) {
    for (let j = 0; j <= sides; j++) {
      const f = (2 * Math.PI * j) / sides, x = Math.cos(f), y = Math.sin(f);
      v.push(x * r, y * r, z, x * r, y * r, nz);
    }
  }
  for (let i = 0; i + 1 < prof.length; i++) {
    for (let j = 0; j < sides; j++) {
      const a = i * w + j, b = a + 1, c = b + w, d = a + w;
      ix.push(a, b, c, a, c, d);
    }
  }
  return { v, ix };
}

// An arc's tube: the capsule with `segs` body segments, so the vertex stage can bend it.
function arcCapsuleMesh(sides, rings, segs) {
  const prof = [];
  for (let k = rings; k >= 0; k--) { const f = (Math.PI / 2) * (k / rings); prof.push([Math.cos(f), -Math.sin(f), -Math.sin(f)]); }
  for (let i = 1; i < segs; i++) prof.push([1, i / segs, 0]);
  for (let k = 0; k <= rings; k++) { const f = (Math.PI / 2) * (k / rings); prof.push([Math.cos(f), 1 + Math.sin(f), Math.sin(f)]); }
  const v = [], ix = [], w = sides + 1;
  for (const [r, z, nz] of prof) {
    for (let j = 0; j <= sides; j++) {
      const f = (2 * Math.PI * j) / sides, x = Math.cos(f), y = Math.sin(f);
      v.push(x * r, y * r, z, x * r, y * r, nz);
    }
  }
  for (let i = 0; i + 1 < prof.length; i++) {
    for (let j = 0; j < sides; j++) {
      const a = i * w + j, b = a + 1, c = b + w, d = a + w;
      ix.push(a, b, c, a, c, d);
    }
  }
  return { v, ix };
}

// An arc's flat ribbon: the stadium with `segs` body segments (a strip), and a fan per end.
function arcRibbonMesh(arc, segs) {
  const v = [], ix = [];
  for (let i = 0; i <= segs; i++) {
    v.push(-1, 0, i / segs, 0, 1, 0, 1, 0, i / segs, 0, 1, 0);
    if (i > 0) { const a = 2 * (i - 1); ix.push(a, a + 1, a + 3, a, a + 3, a + 2); }
  }
  for (const end of [0, 1]) {
    const c = v.length / 6;
    v.push(0, 0, end, 0, 1, 0);
    for (let k = 0; k <= arc; k++) {
      const a = (Math.PI * k) / arc;
      v.push(Math.cos(a), 0, end ? 1 + Math.sin(a) : -Math.sin(a), 0, 1, 0);
      if (k > 0) ix.push(c, c + k, c + k + 1);
    }
  }
  return { v, ix };
}

// A line as a flat ribbon, in the tube's x-z plane (y = 0, normal +y): a stadium, drawn as a
// fan. The same z convention as the capsule.
function ribbonMesh(arc) {
  const v = [], ix = [];
  for (let k = 0; k <= arc; k++) { const a = (Math.PI * k) / arc; v.push(Math.cos(a), 0, -Math.sin(a), 0, 1, 0); }
  for (let k = 0; k <= arc; k++) { const a = (Math.PI * k) / arc; v.push(-Math.cos(a), 0, 1 + Math.sin(a), 0, 1, 0); }
  for (let k = 1; k + 1 < v.length / 6; k++) ix.push(0, k, k + 1);
  return { v, ix };
}

function boxMesh() {
  const faces = [
    [[1, 0, 0], [0, 1, 0], [0, 0, 1]], [[-1, 0, 0], [0, 0, 1], [0, 1, 0]],
    [[0, 1, 0], [0, 0, 1], [1, 0, 0]], [[0, -1, 0], [1, 0, 0], [0, 0, 1]],
    [[0, 0, 1], [1, 0, 0], [0, 1, 0]], [[0, 0, -1], [0, 1, 0], [1, 0, 0]],
  ];
  const corners = [[-1, -1], [1, -1], [1, 1], [-1, 1]];
  const v = [], ix = [];
  for (let f = 0; f < 6; f++) {
    const [n, u, w] = faces[f];
    for (const [a, b] of corners) {
      v.push(
        0.5 * (n[0] + a * u[0] + b * w[0]),
        0.5 * (n[1] + a * u[1] + b * w[1]),
        0.5 * (n[2] + a * u[2] + b * w[2]) + 0.5,
        n[0], n[1], n[2],
      );
    }
    const k = 4 * f;
    ix.push(k, k + 1, k + 2, k, k + 2, k + 3);
  }
  return { v, ix };
}

function quadMesh() {
  return {
    v: [-0.5, -0.5, 0, 0, 0, 1, 0.5, -0.5, 0, 0, 0, 1, 0.5, 0.5, 0, 0, 0, 1, -0.5, 0.5, 0, 0, 0, 1],
    ix: [0, 1, 2, 0, 2, 3],
  };
}

function discMesh(sides) {
  const v = [0, 0, 0, 0, 0, 1], ix = [];
  for (let j = 0; j < sides; j++) {
    const f = (2 * Math.PI * j) / sides;
    v.push(Math.cos(f), Math.sin(f), 0, 0, 0, 1);
    ix.push(0, 1 + j, 1 + ((j + 1) % sides));
  }
  return { v, ix };
}

// ---- slots: how a kind is drawn. Index = kind; 11 and 12 are the coarse sphere and rod. ----
// mode: [vertex mode (0 sphere, 1 tube, 2 scaled-and-turned, 3 bent tube), radius scale or uniform-scale switch]; shade: [lit, specular, rim]; cull.
// The vertex stage's kind code: 1 quad, 2 patch, 3 line or arc, 4 disc, 5 box or prism, 0 the rest.
const KIND_CODE = [0, 1, 3, 3, 4, 5, 5, 0, 0, 2, 0, 0, 0, 3, 3];
const SLOT_COARSE_SPHERE = 11, SLOT_COARSE_ROD = 12, SLOT_FLAT_LINE = 13, SLOT_FLAT_ARC = 14, N_SLOT = 15;
const SCREEN_ID = 15;   // the id of every screen slot, for a resident batch's vertex arrays (after the world slots' 0..14)

function compile(gl, type, src) {
  const s = gl.createShader(type);
  gl.shaderSource(s, src);
  gl.compileShader(s);
  if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) {
    const log = gl.getShaderInfoLog(s);
    gl.deleteShader(s);
    throw new Error('holon-draw: shader did not compile: ' + log);
  }
  return s;
}

/**
 * @param {WebGLRenderingContext|WebGL2RenderingContext} gl a context the HOST owns
 * @param {{sphereSegments?: number, rodSides?: number, text?: {texture: (page: number) => WebGLTexture|null}, forceMediump?: boolean}} [options]
 *   text: a provider from `./text.js`; without one, run batches are skipped.
 *   forceMediump: write `mediump` for the screen program's fragment precision whatever the device has.
 */
export function createDrawer(gl, options) {
  options = options || {};
  const isGL2 = typeof WebGL2RenderingContext !== 'undefined' && gl instanceof WebGL2RenderingContext;

  // Instancing: native on WebGL 2, ANGLE_instanced_arrays on WebGL 1.
  let divisor, drawInstanced;
  if (isGL2) {
    divisor = (i, d) => gl.vertexAttribDivisor(i, d);
    drawInstanced = (n, count) => gl.drawElementsInstanced(gl.TRIANGLES, n, gl.UNSIGNED_SHORT, 0, count);
  } else {
    const ext = gl.getExtension('ANGLE_instanced_arrays');
    if (!ext) {
      throw new Error('holon-draw: this context has no instancing (neither WebGL 2 nor ANGLE_instanced_arrays on WebGL 1); nothing can be drawn');
    }
    divisor = (i, d) => ext.vertexAttribDivisorANGLE(i, d);
    drawInstanced = (n, count) => ext.drawElementsInstancedANGLE(gl.TRIANGLES, n, gl.UNSIGNED_SHORT, 0, count);
  }

  // Vertex array objects keep this drawer's attribute state off the host's.
  let createVAO = null, bindVAO = null, deleteVAO = null, boundVAO = null;
  if (isGL2) {
    boundVAO = () => gl.getParameter(gl.VERTEX_ARRAY_BINDING);
    createVAO = () => gl.createVertexArray();
    bindVAO = (v) => gl.bindVertexArray(v);
    deleteVAO = (v) => gl.deleteVertexArray(v);
  } else {
    const ext = gl.getExtension('OES_vertex_array_object');
    if (ext) {
      boundVAO = () => gl.getParameter(ext.VERTEX_ARRAY_BINDING_OES);
      createVAO = () => ext.createVertexArrayOES();
      bindVAO = (v) => ext.bindVertexArrayOES(v);
      deleteVAO = (v) => ext.deleteVertexArrayOES(v);
    }
  }
  const hasVAO = !!createVAO;

  // Program.
  // HD_MEDIUMP in every source: the no-high-precision path, forced or found by the query.
  const hp = gl.getShaderPrecisionFormat(gl.FRAGMENT_SHADER, gl.HIGH_FLOAT);
  const PF = !options.forceMediump && hp && hp.precision > 0 ? 'highp' : 'mediump';
  const med = PF === 'mediump' ? '#define HD_MEDIUMP 1\n' : '';
  const headV = med + PF_HEAD_V, headF = med + PF_HEAD_F;
  // Two world programs: without shadows (what every draw() without opts.shadow uses, at no
  // cost from them) and with them (HD_SHADOW: the light-space varying and the four-tap lookup).
  function worldProgram(defs, fsrc) {
    const vs = compile(gl, gl.VERTEX_SHADER, defs + headV + VS);
    const fs = compile(gl, gl.FRAGMENT_SHADER, defs + headF + fsrc);
    const program = gl.createProgram();
    gl.attachShader(program, vs);
    gl.attachShader(program, fs);
    gl.bindAttribLocation(program, A_P, 'a_p');
    gl.bindAttribLocation(program, A_N, 'a_n');
    gl.bindAttribLocation(program, I_POS, 'i_pos');
    gl.bindAttribLocation(program, I_SIZE, 'i_size');
    gl.bindAttribLocation(program, I_AUX, 'i_aux');
    gl.bindAttribLocation(program, I_ANGLE, 'i_angle');
    gl.bindAttribLocation(program, I_COLOR, 'i_color');
    gl.bindAttribLocation(program, I_META, 'i_meta');
    gl.linkProgram(program);
    gl.deleteShader(vs);
    gl.deleteShader(fs);
    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
      const log = gl.getProgramInfoLog(program);
      gl.deleteProgram(program);
      throw new Error('holon-draw: program did not link: ' + log);
    }
    const u = (n) => gl.getUniformLocation(program, n);
    return {
      program, uVP: u('u_vp'), uOffset: u('u_offset'), uEye: u('u_eye'), uMode: u('u_mode'),
      uLight: u('u_light'), uShade: u('u_shade'), uPix: u('u_pix'), uWView: u('u_wview'),
      uParams: u('u_params[0]'), uLvp: u('u_lvp'), uLno: u('u_lno'), uShadow: u('u_shadow'),
      uShp: u('u_shp'), samplerSet: false, last: new Float32Array(32).fill(NaN),
    };
  }
  const WP = worldProgram('', FS);

  // The shadow programs (the world one with HD_SHADOW, and the depth one: the world vertex
  // stage with ZFS), compiled on the first draw() that asks for shadows, before any GL state
  // of that draw() is changed. A host that never asks pays nothing for them.
  let WS = null, Z = null;
  function shadowPrograms() {
    if (!WS) WS = worldProgram('#define HD_SHADOW 1\n', FS);
    if (!Z) Z = worldProgram('', ZFS);
  }
  // The shadow map, made on first use: an RGBA8 texture and a 16-bit depth buffer.
  let zTex = null, zDepth = null, zFbo = null, zSize = 0;

  // The screen program.
  const svs = compile(gl, gl.VERTEX_SHADER, headV + SVS);
  const sfs = compile(gl, gl.FRAGMENT_SHADER, headF + SFS);
  const sprogram = gl.createProgram();
  gl.attachShader(sprogram, svs);
  gl.attachShader(sprogram, sfs);
  gl.bindAttribLocation(sprogram, A_P, 'a_p');
  gl.bindAttribLocation(sprogram, I_POS, 'i_pos');
  gl.bindAttribLocation(sprogram, I_SIZE, 'i_size');
  gl.bindAttribLocation(sprogram, I_AUX, 'i_aux');
  gl.bindAttribLocation(sprogram, I_ANGLE, 'i_angle');
  gl.bindAttribLocation(sprogram, I_COLOR, 'i_color');
  gl.bindAttribLocation(sprogram, I_META, 'i_meta');
  gl.linkProgram(sprogram);
  gl.deleteShader(svs);
  gl.deleteShader(sfs);
  if (!gl.getProgramParameter(sprogram, gl.LINK_STATUS)) {
    const log = gl.getProgramInfoLog(sprogram);
    gl.deleteProgram(sprogram);
    gl.deleteProgram(WP.program);
    throw new Error('holon-draw: screen program did not link: ' + log);
  }
  const uView = gl.getUniformLocation(sprogram, 'u_view');
  const uSOffset = gl.getUniformLocation(sprogram, 'u_soffset');
  const uSMode = gl.getUniformLocation(sprogram, 'u_smode');
  const uSParams = gl.getUniformLocation(sprogram, 'u_sparams[0]');
  const uFMode = gl.getUniformLocation(sprogram, 'u_fmode');
  const uTex = [0, 1, 2, 3].map((i) => gl.getUniformLocation(sprogram, 'u_t' + i));
  const uImg = gl.getUniformLocation(sprogram, 'u_img');
  let samplersSet = false;
  const text = options.text || null;

  // Meshes, built once. createDrawer() puts back the buffer and vertex array bindings it
  // found, and changes no other state.
  const prevArray = gl.getParameter(gl.ARRAY_BUFFER_BINDING);
  const prevElement = hasVAO ? null : gl.getParameter(gl.ELEMENT_ARRAY_BUFFER_BINDING);
  const meshes = [];
  function mesh(m) {
    const vbo = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, vbo);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(m.v), gl.STATIC_DRAW);
    const out = { vbo, ibo: gl.createBuffer(), n: m.ix.length, ix: new Uint16Array(m.ix) };
    meshes.push(out);
    return out;
  }
  const segs = Math.max(8, options.sphereSegments | 0 || 28);
  const sides = Math.max(6, options.rodSides | 0 || 20);
  const mSphere = mesh(sphereMesh(segs, Math.max(4, segs >> 1)));
  const mSphereCoarse = mesh(sphereMesh(10, 6));
  const mRod = mesh(tubeMesh(sides, true));
  const mRodCoarse = mesh(tubeMesh(8, true));
  const mLine = mesh(capsuleMesh(8, 3));
  const mRibbon = mesh(ribbonMesh(10));
  const mArc = mesh(arcCapsuleMesh(8, 3, 16));
  const mArcRibbon = mesh(arcRibbonMesh(10, 16));
  const mBox = mesh(boxMesh());
  const mQuad = mesh(quadMesh());
  const mDisc = mesh(discMesh(24));

  // slot -> { mesh, kind (whose instance buffer), mode x, mode y, lit, spec, rim, cull, vao }
  const slots = new Array(N_SLOT).fill(null);
  const slot = (i, kind, m, mx, my, lit, spec, rim, cull) => {
    slots[i] = { mesh: m, kind, mx, my, lit, spec, rim, cull, vao: null, kc: KIND_CODE[i], id: i };
  };
  slot(KIND.QUAD, KIND.QUAD, mQuad, 2, 0, 0, 0, 0, false);
  slot(KIND.PATCH, KIND.PATCH, mQuad, 2, 0, 0, 0, 0, false);
  slot(KIND.LINE, KIND.LINE, mLine, 1, 0.5, 0, 0, 0, true);
  slot(KIND.ARC, KIND.ARC, mArc, 3, 0.5, 0, 0, 0, true);
  slot(KIND.DISC, KIND.DISC, mDisc, 2, 1, 0, 0, 0, false);
  slot(KIND.GLYPH, KIND.GLYPH, mDisc, 2, 1, 0, 0, 0, false);
  slot(KIND.BOX, KIND.BOX, mBox, 2, 0, 1, 0.05, 0, true);
  slot(KIND.PRISM, KIND.PRISM, mBox, 2, 0, 1, 0.05, 0, true);
  slot(KIND.SPHERE, KIND.SPHERE, mSphere, 0, 0, 1, 0.3, 0.18, true);
  slot(KIND.ROD, KIND.ROD, mRod, 1, 1, 1, 0.18, 0.08, true);
  slot(SLOT_COARSE_SPHERE, KIND.SPHERE, mSphereCoarse, 0, 0, 1, 0.3, 0.18, true);
  slot(SLOT_COARSE_ROD, KIND.ROD, mRodCoarse, 1, 1, 1, 0.18, 0.08, true);
  slot(SLOT_FLAT_LINE, KIND.LINE, mRibbon, 1, 0.5, 0, 0, 0, false);
  slot(SLOT_FLAT_ARC, KIND.ARC, mArcRibbon, 3, 0.5, 0, 0, 0, false);

  // Screen slots, by kind: vertex mode (0 rectangle, 1 circle, 2 line, 3 run).
  const sslots = new Array(13).fill(null);
  sslots[KIND.QUAD] = { kind: KIND.QUAD, smode: 0 };
  sslots[KIND.PATCH] = { kind: KIND.PATCH, smode: 0 };
  sslots[KIND.DISC] = { kind: KIND.DISC, smode: 1 };
  sslots[KIND.GLYPH] = { kind: KIND.GLYPH, smode: 1 };
  sslots[KIND.LINE] = { kind: KIND.LINE, smode: 2 };
  sslots[KIND.ARC] = { kind: KIND.ARC, smode: 2 };
  if (text) sslots[KIND.RUN] = { kind: KIND.RUN, smode: 3 };
  sslots[KIND.IMAGE] = { kind: KIND.IMAGE, smode: 3, image: true };
  for (const s of sslots) if (s) { s.id = SCREEN_ID; s.mesh = mQuad; }   // every screen slot draws the quad mesh
  // the host's textures, by slot (setImage)
  const images = new Map();
  const IMAGE_UNIT = 5;
  let screenVAO = null;

  function meshAttribs(m) {
    gl.bindBuffer(gl.ARRAY_BUFFER, m.vbo);
    gl.vertexAttribPointer(A_P, 3, gl.FLOAT, false, 24, 0);
    gl.vertexAttribPointer(A_N, 3, gl.FLOAT, false, 24, 12);
    gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, m.ibo);
    for (let i = 0; i < N_ATTR; i++) {
      gl.enableVertexAttribArray(i);
      divisor(i, i >= I_POS ? 1 : 0);
    }
  }

  if (hasVAO) {
    const prevVAO = boundVAO();
    for (const s of slots) {
      if (!s) continue;
      s.vao = createVAO();
      bindVAO(s.vao);
      meshAttribs(s.mesh);
      gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, s.mesh.ix, gl.STATIC_DRAW);
    }
    screenVAO = createVAO();
    bindVAO(screenVAO);
    meshAttribs(mQuad);
    bindVAO(prevVAO);
  } else {
    for (const m of meshes) {
      gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, m.ibo);
      gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, m.ix, gl.STATIC_DRAW);
    }
    gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, prevElement);
  }
  gl.bindBuffer(gl.ARRAY_BUFFER, prevArray);

  // One instance buffer per kind, grown as needed and reused. Within one draw() every batch
  // of a kind is written at its own offset in that buffer, so a later batch never overwrites
  // bytes an earlier draw call of the same frame still reads.
  const instBuf = new Array(13).fill(null);
  const instCap = new Int32Array(13);
  const need = new Int32Array(13);
  const cursor = new Int32Array(13);

  // Scratch, allocated once.
  const light = new Float32Array(3);
  const lvp = new Float32Array(16);
  let wAt = new Int32Array(64);   // per batch: where its records were uploaded this draw(), or -1
  const inv = new Float64Array(16);
  const st = { drawCalls: 0, instances: 0, bytesUploaded: 0, paramUploads: 0, missingImages: 0, residentBuffers: 0, residentBytes: 0 };
  let dead = false;
  let flat = false;   // lines as flat ribbons, for the draw() in progress

  // The eye, homogeneous, from vp alone: the preimage of clip (0, 0, -1, 0). For a
  // perspective vp that is the camera position (w != 0); for an orthographic one it is the
  // direction toward the viewer (w = 0).
  function eyeFrom(m) {
    const a00 = m[0], a01 = m[1], a02 = m[2], a03 = m[3], a10 = m[4], a11 = m[5], a12 = m[6], a13 = m[7];
    const a20 = m[8], a21 = m[9], a22 = m[10], a23 = m[11], a30 = m[12], a31 = m[13], a32 = m[14], a33 = m[15];
    const b00 = a00 * a11 - a01 * a10, b01 = a00 * a12 - a02 * a10, b02 = a00 * a13 - a03 * a10;
    const b03 = a01 * a12 - a02 * a11, b04 = a01 * a13 - a03 * a11, b05 = a02 * a13 - a03 * a12;
    const b06 = a20 * a31 - a21 * a30, b07 = a20 * a32 - a22 * a30, b08 = a20 * a33 - a23 * a30;
    const b09 = a21 * a32 - a22 * a31, b10 = a21 * a33 - a23 * a31, b11 = a22 * a33 - a23 * a32;
    const det = b00 * b11 - b01 * b10 + b02 * b09 + b03 * b08 - b04 * b07 + b05 * b06;
    if (!(Math.abs(det) > 0) || !isFinite(det)) {
      inv[0] = 0; inv[1] = 0; inv[2] = 1; inv[3] = 0;
      return;
    }
    // Third column of the inverse (elements 8..11 of a column-major inverse), negated.
    let x = -(a31 * b05 - a32 * b04 + a33 * b03) / det;
    let y = -(a32 * b02 - a30 * b05 - a33 * b01) / det;
    let z = -(a30 * b04 - a31 * b02 + a33 * b00) / det;
    let w = -(a31 * b01 - a30 * b03 - a32 * b00) / det;
    // Scale so the numbers are tame in f32, and so that w >= 0.
    let s = Math.max(Math.abs(x), Math.abs(y), Math.abs(z), Math.abs(w));
    if (!(s > 0) || !isFinite(s)) { x = 0; y = 0; z = 1; w = 0; s = 1; }
    if (Math.abs(w) > 1e-12 * s) s = w; else { w = 0; s = Math.hypot(x, y, z) || 1; }
    inv[0] = x / s; inv[1] = y / s; inv[2] = z / s; inv[3] = w / s;
  }

  // The slot a batch is drawn with, or null if it is not drawn.
  function slotOf(b) {
    const k = b.kind | 0;
    if (k < 1 || k > 12) return null;
    if (b.screen === true) return sslots[k];
    if (k > 10) return null;
    if (flat) {
      if (k === KIND.LINE) return slots[SLOT_FLAT_LINE];
      if (k === KIND.ARC) return slots[SLOT_FLAT_ARC];
    }
    if (b.coarse === true) {
      if (k === KIND.SPHERE) return slots[SLOT_COARSE_SPHERE];
      if (k === KIND.ROD) return slots[SLOT_COARSE_ROD];
    }
    return slots[k];
  }

  // The settings last uploaded to each program (a program keeps its uniforms between
  // frames, so these do too). NaN at the start: the first batch always uploads.
  const lastS = new Float32Array(32).fill(NaN);
  function settings(loc, last, src) {
    if (!src) src = PARAMS_NEUTRAL;
    let same = true;
    for (let i = 0; i < 32; i++) {
      const v = Math.fround(src[i]);
      if (last[i] !== v) { last[i] = v; same = false; }   // a NaN never equals: it uploads every time
    }
    if (same) return;
    gl.uniform4fv(loc, last);
    st.paramUploads++;
  }

  // Upload one `bytes` batch at its kind's cursor; its offset in that kind's buffer.
  function upload(b, k, len) {
    const at = cursor[k];
    cursor[k] = at + len;
    gl.bindBuffer(gl.ARRAY_BUFFER, instBuf[k]);
    const bytes = b.bytes;
    if (bytes.byteLength === len) gl.bufferSubData(gl.ARRAY_BUFFER, at, bytes);
    else if (isGL2) gl.bufferSubData(gl.ARRAY_BUFFER, at, bytes, 0, len);
    else gl.bufferSubData(gl.ARRAY_BUFFER, at, bytes.subarray(0, len)); // the one view made; pass exact-length bytes to avoid it
    st.bytesUploaded += len;
    return at;
  }
  // Point the instanced attributes at records `at` bytes into `buf`.
  function pointers(buf, at) {
    gl.bindBuffer(gl.ARRAY_BUFFER, buf);
    gl.vertexAttribPointer(I_POS, 3, gl.FLOAT, false, INSTANCE_BYTES, at);
    gl.vertexAttribPointer(I_SIZE, 3, gl.FLOAT, false, INSTANCE_BYTES, at + 12);
    gl.vertexAttribPointer(I_AUX, 3, gl.FLOAT, false, INSTANCE_BYTES, at + 24);
    gl.vertexAttribPointer(I_ANGLE, 1, gl.FLOAT, false, INSTANCE_BYTES, at + 36);
    gl.vertexAttribPointer(I_COLOR, 4, gl.UNSIGNED_BYTE, true, INSTANCE_BYTES, at + 40);
    gl.vertexAttribPointer(I_META, 4, gl.UNSIGNED_BYTE, false, INSTANCE_BYTES, at + 48);
  }

  // ---- resident buffers (RESIDENT BUFFERS at the top) ----
  // handles[h]: { buf, cap (bytes allocated), size (bytes of records now), vaos: Map(first * 32 + slot id -> vertex array) }
  const handles = [null];
  const freeHandles = [];
  const VAO_CAP = 256;
  function handleOf(h, what) {
    const r = typeof h === 'number' ? handles[h] : undefined;
    if (!r) throw new Error('holon-draw: ' + what + ': no resident buffer with handle ' + h);
    return r;
  }
  function recordBytes(bytes, what) {
    if (!bytes || typeof bytes.byteLength !== 'number' || bytes.byteLength % INSTANCE_BYTES !== 0) {
      throw new Error('holon-draw: ' + what + ' takes whole 52-byte records (an ArrayBufferView whose byteLength is a multiple of 52)');
    }
  }
  /** Upload records once; returns their handle (see RESIDENT BUFFERS at the top). */
  function resident(bytes) {
    if (dead) throw new Error('holon-draw: resident() after destroy()');
    recordBytes(bytes, 'resident()');
    const prev = gl.getParameter(gl.ARRAY_BUFFER_BINDING);
    const r = { buf: gl.createBuffer(), cap: bytes.byteLength, size: bytes.byteLength, vaos: hasVAO ? new Map() : null };
    gl.bindBuffer(gl.ARRAY_BUFFER, r.buf);
    gl.bufferData(gl.ARRAY_BUFFER, bytes, gl.STATIC_DRAW);
    gl.bindBuffer(gl.ARRAY_BUFFER, prev);
    const h = freeHandles.length ? freeHandles.pop() : handles.length;
    handles[h] = r;
    st.residentBuffers++;
    st.residentBytes += r.cap;
    return h;
  }
  /** New records for a handle: in place when they fit, else reallocated; the handle stays. */
  function update(h, bytes) {
    if (dead) throw new Error('holon-draw: update() after destroy()');
    const r = handleOf(h, 'update()');
    recordBytes(bytes, 'update()');
    const prev = gl.getParameter(gl.ARRAY_BUFFER_BINDING);
    gl.bindBuffer(gl.ARRAY_BUFFER, r.buf);
    if (bytes.byteLength <= r.cap) {
      if (bytes.byteLength > 0) gl.bufferSubData(gl.ARRAY_BUFFER, 0, bytes);
    } else {
      // The same buffer object, so the vertex arrays that point into it stay good.
      gl.bufferData(gl.ARRAY_BUFFER, bytes, gl.STATIC_DRAW);
      st.residentBytes += bytes.byteLength - r.cap;
      r.cap = bytes.byteLength;
    }
    r.size = bytes.byteLength;
    gl.bindBuffer(gl.ARRAY_BUFFER, prev);
  }
  /** Free a handle's buffer (and its vertex arrays); the number may be handed out again. */
  function release(h) {
    const r = handleOf(h, 'release()');
    if (r.vaos) { for (const v of r.vaos.values()) deleteVAO(v); r.vaos.clear(); }
    gl.deleteBuffer(r.buf);
    handles[h] = null;
    freeHandles.push(h);
    st.residentBuffers--;
    st.residentBytes -= r.cap;
  }

  // Make batch b's records the instanced attributes' source for a draw with slot s: `at` is
  // where a `bytes` batch was uploaded this draw() (pointed at through `vao`, the slot's own
  // vertex array), or -2 for a resident batch (its own vertex array, or the bare pointers).
  // `bound` is the vertex array this draw() last bound (null at its start).
  let bound = null;
  function attach(b, s, at, vao, count) {
    st.drawCalls++;
    st.instances += count;
    if (at >= 0) {
      if (hasVAO && bound !== vao) { bindVAO(vao); bound = vao; }
      pointers(instBuf[s.kind], at);
      return;
    }
    const r = handles[b.handle], first = b.first | 0;
    if (!hasVAO) { pointers(r.buf, first * INSTANCE_BYTES); return; }
    const key = first * 32 + s.id;
    let v = r.vaos.get(key);
    if (v !== undefined) {
      if (bound !== v) { bindVAO(v); bound = v; }
      return;
    }
    if (r.vaos.size < VAO_CAP) {
      v = createVAO();
      r.vaos.set(key, v);
      bindVAO(v);
      bound = v;
      meshAttribs(s.mesh);
    } else if (bound !== vao) {
      bindVAO(vao);
      bound = vao;
    }
    pointers(r.buf, first * INSTANCE_BYTES);
  }

  // The light's orthographic view-projection over the sphere (c, r), into lvp.
  function lightMatrix(c, r) {
    const lx = light[0], ly = light[1], lz = light[2];
    const ex = c[0] + lx * 2 * r, ey = c[1] + ly * 2 * r, ez = c[2] + lz * 2 * r;
    // view axes: f toward the light (z of the view), s and u across it
    let ux = 0, uy = 0, uz = 1;
    if (Math.abs(lz) > 0.99) { ux = 1; uz = 0; }
    let sx = uy * lz - uz * ly, sy = uz * lx - ux * lz, sz = ux * ly - uy * lx;
    let sl = Math.hypot(sx, sy, sz) || 1; sx /= sl; sy /= sl; sz /= sl;
    const vx = ly * sz - lz * sy, vy = lz * sx - lx * sz, vz = lx * sy - ly * sx;
    const n = 0.01 * r, f = 4 * r, k = 1 / r, zz = -2 / (f - n), zw = -(f + n) / (f - n);
    // ortho * view, column-major
    lvp[0] = k * sx; lvp[1] = k * vx; lvp[2] = zz * lx; lvp[3] = 0;
    lvp[4] = k * sy; lvp[5] = k * vy; lvp[6] = zz * ly; lvp[7] = 0;
    lvp[8] = k * sz; lvp[9] = k * vz; lvp[10] = zz * lz; lvp[11] = 0;
    lvp[12] = -k * (sx * ex + sy * ey + sz * ez);
    lvp[13] = -k * (vx * ex + vy * ey + vz * ez);
    lvp[14] = -zz * (lx * ex + ly * ey + lz * ez) + zw;
    lvp[15] = 1;
  }

  function shadowTarget(size) {
    if (zFbo && zSize === size) return;
    if (!zTex) { zTex = gl.createTexture(); zDepth = gl.createRenderbuffer(); zFbo = gl.createFramebuffer(); }
    zSize = size;
    gl.activeTexture(gl.TEXTURE4);
    gl.bindTexture(gl.TEXTURE_2D, zTex);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, size, size, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    gl.activeTexture(gl.TEXTURE0);
    const prevRB = gl.getParameter(gl.RENDERBUFFER_BINDING);
    gl.bindRenderbuffer(gl.RENDERBUFFER, zDepth);
    gl.renderbufferStorage(gl.RENDERBUFFER, gl.DEPTH_COMPONENT16, size, size);
    gl.bindRenderbuffer(gl.RENDERBUFFER, prevRB);
    const prevFB = gl.getParameter(gl.FRAMEBUFFER_BINDING);
    gl.bindFramebuffer(gl.FRAMEBUFFER, zFbo);
    gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, zTex, 0);
    gl.framebufferRenderbuffer(gl.FRAMEBUFFER, gl.DEPTH_ATTACHMENT, gl.RENDERBUFFER, zDepth);
    gl.bindFramebuffer(gl.FRAMEBUFFER, prevFB);
  }

  /**
   * @param {Array<{kind:number, offset?:ArrayLike<number>, count:number, bytes?:Uint8Array, handle?:number, first?:number, slot?:number, screen?:boolean, coarse?:boolean, params?:ArrayLike<number>, shade?:ArrayLike<number>}>} batches
   *   each batch has `bytes` (uploaded now) or `handle` (a resident buffer, see RESIDENT
   *   BUFFERS; `first` its first record, and `slot` on a resident image batch).
   *   `shade`, optional on a world batch of a lit kind: [specular, rim] for that batch in place
   *   of the kind's own (box and prism 0.05, 0; sphere 0.3, 0.18; rod 0.18, 0.08).
   * @param {Float32Array} vp column-major, 16 (world batches only)
   * @param {{light?:ArrayLike<number>, ambient?:number, flatLit?:boolean, viewport?:ArrayLike<number>, pixelsPerUnit?:number, depthTest?:boolean, flatLines?:boolean, shadow?:{center:ArrayLike<number>, radius:number, size?:number, strength?:number, bias?:number}}} [opts]
   *   A world batch may also carry `castShadow: false`.
   */
  function draw(batches, vp, opts) {
    if (dead) throw new Error('holon-draw: draw() after destroy()');
    st.drawCalls = 0; st.instances = 0; st.bytesUploaded = 0; st.paramUploads = 0; st.missingImages = 0;
    flat = !!opts && (opts.flatLines === true || (opts.flatLines !== false && opts.depthTest === false));
    if (opts && opts.shadow && opts.shadow.center && opts.shadow.radius > 0) shadowPrograms();
    const nb = batches.length;

    // Pass 1: bytes needed per kind; how many world and screen batches there are.
    need.fill(0);
    let nWorld = 0, nScreen = 0;
    for (let i = 0; i < nb; i++) {
      const b = batches[i], c = b.count | 0;
      if (c <= 0) continue;
      const s = slotOf(b);
      if (!s) continue;
      if (b.handle !== undefined) {
        const r = handleOf(b.handle, 'batch ' + i), first = b.first === undefined ? 0 : b.first;
        if (first !== (first | 0) || first < 0 || (first + c) * INSTANCE_BYTES > r.size) {
          throw new Error('holon-draw: batch ' + i + ' (kind ' + b.kind + ') asks for records ' + first + '..' + (first + c - 1) + ' of a resident buffer that holds ' + r.size / INSTANCE_BYTES);
        }
        if (s.image && !(b.slot === (b.slot | 0) && b.slot >= 0 && b.slot <= 255)) {
          throw new Error('holon-draw: batch ' + i + ' is a resident image batch: it needs `slot` (0..255), since its first record is not in hand');
        }
      } else {
        if (b.bytes.byteLength < c * INSTANCE_BYTES) {
          throw new Error('holon-draw: batch ' + i + ' (kind ' + b.kind + ') has ' + b.bytes.byteLength + ' bytes for ' + c + ' records of ' + INSTANCE_BYTES);
        }
        need[s.kind] += c * INSTANCE_BYTES;
      }
      if (b.screen === true) nScreen++; else nWorld++;
    }
    if (nWorld + nScreen === 0) return;

    for (let k = 1; k <= 12; k++) {
      cursor[k] = 0;
      if (need[k] <= instCap[k]) continue;
      let cap = instCap[k] || 64 * INSTANCE_BYTES;
      while (cap < need[k]) cap *= 2;
      if (!instBuf[k]) instBuf[k] = gl.createBuffer();
      gl.bindBuffer(gl.ARRAY_BUFFER, instBuf[k]);
      gl.bufferData(gl.ARRAY_BUFFER, cap, gl.DYNAMIC_DRAW);
      instCap[k] = cap;
    }

    // Frame state.
    bound = null;
    gl.enable(gl.BLEND);
    gl.blendFuncSeparate(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA, gl.ONE, gl.ONE_MINUS_SRC_ALPHA);
    gl.enable(gl.DEPTH_TEST);
    gl.depthFunc(gl.LEQUAL);
    gl.depthMask(true);
    gl.cullFace(gl.BACK);
    gl.frontFace(gl.CCW);
    gl.enable(gl.CULL_FACE);
    let culling = true;

    // Pass 2: the world batches, in the order given (uploaded once, read by the shadow pass
    // too). wAt[i]: where batch i was uploaded; -2 resident (nothing to upload); -1 not drawn.
    if (nWorld > 0) {
      if (wAt.length < nb) wAt = new Int32Array(Math.max(nb, 2 * wAt.length));
      for (let i = 0; i < nb; i++) {
        const b = batches[i], c = b.count | 0;
        const s = c > 0 && b.screen !== true ? slotOf(b) : null;
        wAt[i] = !s ? -1 : b.handle !== undefined ? -2 : upload(b, s.kind, c * INSTANCE_BYTES);
      }
      let lx = 0.35, ly = -0.5, lz = 0.8;
      const L = opts && opts.light;
      if (L) { lx = +L[0]; ly = +L[1]; lz = +L[2]; }
      let ll = Math.sqrt(lx * lx + ly * ly + lz * lz);
      if (!(ll > 0) || !isFinite(ll)) { lx = 0; ly = 0; lz = 1; ll = 1; }
      light[0] = lx / ll; light[1] = ly / ll; light[2] = lz / ll;
      let ambient = opts && typeof opts.ambient === 'number' ? opts.ambient : 0.25;
      ambient = ambient < 0 ? 0 : ambient > 1 ? 1 : ambient;
      const flatLit = !!(opts && opts.flatLit);
      const V = opts && opts.viewport;
      const vw = V ? V[2] : gl.drawingBufferWidth, vh = V ? V[3] : gl.drawingBufferHeight;
      const halfH = 0.5 * vh;
      const ppu = opts && opts.pixelsPerUnit > 0 ? +opts.pixelsPerUnit : 0;
      eyeFrom(vp);

      // The shadow map, drawn from the light, before the picture.
      const so = opts && opts.shadow;
      const shadowsOn = !!(so && so.center && so.radius > 0);
      if (shadowsOn) {
        const size = Math.max(64, Math.min(so.size | 0 || 1024, gl.getParameter(gl.MAX_TEXTURE_SIZE)));
        shadowTarget(size);
        lightMatrix(so.center, +so.radius);
        const prevFB = gl.getParameter(gl.FRAMEBUFFER_BINDING);
        const prevClear = gl.getParameter(gl.COLOR_CLEAR_VALUE), prevDepthClear = gl.getParameter(gl.DEPTH_CLEAR_VALUE);
        const HV = V || gl.getParameter(gl.VIEWPORT);   // the host's, put back after
        const scissor = gl.isEnabled(gl.SCISSOR_TEST);
        if (scissor) gl.disable(gl.SCISSOR_TEST);
        gl.bindFramebuffer(gl.FRAMEBUFFER, zFbo);
        gl.viewport(0, 0, size, size);
        gl.clearColor(1, 1, 1, 1);
        gl.clearDepth(1);
        gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
        gl.disable(gl.BLEND);
        gl.useProgram(Z.program);
        gl.uniformMatrix4fv(Z.uVP, false, lvp);
        gl.uniform4f(Z.uEye, light[0], light[1], light[2], 0);
        gl.uniform2f(Z.uWView, size, size);
        gl.cullFace(gl.FRONT);
        let zcur = null;
        for (let i = 0; i < nb; i++) {
          if (wAt[i] === -1) continue;
          const b = batches[i];
          if (b.castShadow === false) continue;
          const s = slotOf(b);
          if (!(s.lit || ((s.kind === KIND.LINE || s.kind === KIND.ARC) && !flat))) continue;
          if (s !== zcur) {
            zcur = s;
            if (!hasVAO) meshAttribs(s.mesh);
            gl.uniform2f(Z.uMode, s.mx, s.my);
            gl.uniform3f(Z.uPix, 0.5 * size, ppu, s.kc);
            if (s.cull !== culling) {
              culling = s.cull;
              if (culling) gl.enable(gl.CULL_FACE); else gl.disable(gl.CULL_FACE);
            }
          }
          attach(b, s, wAt[i], s.vao, b.count | 0);
          settings(Z.uParams, Z.last, b.params);
          const o = b.offset;
          if (o) gl.uniform3f(Z.uOffset, o[0], o[1], o[2]); else gl.uniform3f(Z.uOffset, 0, 0, 0);
          drawInstanced(s.mesh.n, b.count | 0);
        }
        gl.cullFace(gl.BACK);
        gl.enable(gl.BLEND);
        gl.bindFramebuffer(gl.FRAMEBUFFER, prevFB);
        gl.viewport(HV[0], HV[1], HV[2], HV[3]);
        gl.clearColor(prevClear[0], prevClear[1], prevClear[2], prevClear[3]);
        gl.clearDepth(prevDepthClear);
        if (scissor) gl.enable(gl.SCISSOR_TEST);
        gl.activeTexture(gl.TEXTURE4);
        gl.bindTexture(gl.TEXTURE_2D, zTex);
        gl.activeTexture(gl.TEXTURE0);
      }

      const W = shadowsOn ? WS : WP;
      gl.useProgram(W.program);
      gl.uniformMatrix4fv(W.uVP, false, vp);
      gl.uniform4f(W.uEye, inv[0], inv[1], inv[2], inv[3]);
      gl.uniform3fv(W.uLight, light);
      gl.uniform2f(W.uWView, vw, vh);
      if (shadowsOn) {
        gl.uniformMatrix4fv(W.uLvp, false, lvp);
        // look the shadow up a texel and a half out along the normal: no self-shadow banding
        gl.uniform1f(W.uLno, 3 * +so.radius / zSize);
        if (!W.samplerSet) { W.samplerSet = true; gl.uniform1i(W.uShadow, 4); }
        gl.uniform4f(W.uShp,
          typeof so.strength === 'number' ? Math.max(0, Math.min(1, so.strength)) : 0.55,
          1 / zSize,
          typeof so.bias === 'number' ? so.bias : 0.0006, 0);
      }
      if (opts && opts.depthTest === false) gl.disable(gl.DEPTH_TEST);

      let cur = null, curSpec = -1, curRim = -1;
      for (let i = 0; i < nb; i++) {
        if (wAt[i] === -1) continue;
        const b = batches[i], c = b.count | 0;
        const s = slotOf(b);
        const sh = b.shade;
        const spec = sh ? +sh[0] : s.spec, rim = sh ? +sh[1] : s.rim;
        if (s !== cur || spec !== curSpec || rim !== curRim) {
          curSpec = spec; curRim = rim;
          gl.uniform4f(W.uShade, s.lit || flatLit ? 1 : 0, ambient, spec, rim);
        }
        if (s !== cur) {
          cur = s;
          if (!hasVAO) meshAttribs(s.mesh);
          gl.uniform2f(W.uMode, s.mx, s.my);
          gl.uniform3f(W.uPix, halfH, ppu, s.kc);
          if (s.cull !== culling) {
            culling = s.cull;
            if (culling) gl.enable(gl.CULL_FACE); else gl.disable(gl.CULL_FACE);
          }
        }
        attach(b, s, wAt[i], s.vao, c);
        settings(W.uParams, W.last, b.params);
        const o = b.offset;
        if (o) gl.uniform3f(W.uOffset, o[0], o[1], o[2]); else gl.uniform3f(W.uOffset, 0, 0, 0);
        drawInstanced(s.mesh.n, c);
      }
    }

    // Pass 3: the screen batches, over everything, in the order given.
    if (nScreen > 0) {
      gl.useProgram(sprogram);
      gl.disable(gl.DEPTH_TEST);
      if (culling) { gl.disable(gl.CULL_FACE); culling = false; }
      const V = (opts && opts.viewport) || gl.getParameter(gl.VIEWPORT);
      gl.uniform2f(uView, V[2], V[3]);
      if (!hasVAO) meshAttribs(mQuad);
      let smode = -1, fmode = -1, textured = false, imageUnitSet = false;
      for (let i = 0; i < nb; i++) {
        const b = batches[i], c = b.count | 0;
        if (c <= 0 || b.screen !== true) continue;
        const s = slotOf(b);
        if (!s) continue;
        if (s.image) {
          const img = images.get(b.handle !== undefined ? b.slot : b.bytes[51]);
          if (!img) { st.missingImages++; continue; }
          if (!imageUnitSet) { imageUnitSet = true; gl.uniform1i(uImg, IMAGE_UNIT); }
          gl.activeTexture(gl.TEXTURE0 + IMAGE_UNIT);
          gl.bindTexture(gl.TEXTURE_2D, img.tex);
          gl.activeTexture(gl.TEXTURE0);
        }
        if (s.smode !== smode) {
          smode = s.smode;
          gl.uniform1f(uSMode, smode);
        }
        const fm = s.image ? 2 : smode === 3 ? 1 : 0;
        if (fm !== fmode) {
          fmode = fm;
          gl.uniform1f(uFMode, fm);
        }
        if (smode === 3 && !s.image && !textured) {
          // Once per draw(): the atlas pages on units 0..3. Pages are bound here, at draw
          // time, so a page the provider made or refilled this frame is the one sampled.
          textured = true;
          if (!samplersSet) {
            samplersSet = true;
            for (let u = 0; u < MAX_PAGES; u++) gl.uniform1i(uTex[u], u);
          }
          for (let u = MAX_PAGES - 1; u >= 0; u--) {
            const t = text.texture(u);
            if (!t) continue;
            gl.activeTexture(gl.TEXTURE0 + u);
            gl.bindTexture(gl.TEXTURE_2D, t);
          }
          gl.activeTexture(gl.TEXTURE0);
        }
        attach(b, s, b.handle !== undefined ? -2 : upload(b, s.kind, c * INSTANCE_BYTES), screenVAO, c);
        settings(uSParams, lastS, b.params);
        const o = b.offset;
        if (o) gl.uniform3f(uSOffset, o[0], o[1], 0); else gl.uniform3f(uSOffset, 0, 0, 0);
        drawInstanced(mQuad.n, c);
      }
    }
    gl.enable(gl.DEPTH_TEST);

    // Leave as documented at the top of this file.
    if (hasVAO) {
      bindVAO(null);
    } else {
      for (let i = 0; i < N_ATTR; i++) {
        divisor(i, 0);
        gl.disableVertexAttribArray(i);
      }
      gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, null);
    }
    gl.bindBuffer(gl.ARRAY_BUFFER, null);
    if (!culling) gl.enable(gl.CULL_FACE);
  }

  /**
   * drawCalls, instances, bytesUploaded, paramUploads and missingImages are the last draw()'s
   * (bytesUploaded counts `bytes` batches only: 0 for a draw() of resident batches);
   * residentBuffers and residentBytes are what is resident now. The SAME object every call
   * (no allocation); copy it to keep it.
   */
  function stats() {
    return st;
  }

  /** Put a texture in an image slot (0..255); see IMAGES in the header. */
  function setImage(slot, source, o) {
    if (dead) throw new Error('holon-draw: setImage() after destroy()');
    slot = slot | 0;
    if (slot < 0 || slot > 255) throw new Error('holon-draw: an image slot is 0..255');
    let img = images.get(slot);
    if (!img) {
      img = { tex: gl.createTexture() };
      images.set(slot, img);
    }
    const nearest = !!(o && o.nearest);
    const was = gl.getParameter(gl.TEXTURE_BINDING_2D);
    const flip = gl.getParameter(gl.UNPACK_FLIP_Y_WEBGL), pre = gl.getParameter(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL);
    gl.bindTexture(gl.TEXTURE_2D, img.tex);
    gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, false);
    gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, false);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, source);
    const f = nearest ? gl.NEAREST : gl.LINEAR;
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, f);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, f);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, flip);
    gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, pre);
    gl.bindTexture(gl.TEXTURE_2D, was);
  }

  /** Free an image slot's texture. */
  function deleteImage(slot) {
    const img = images.get(slot | 0);
    if (!img) return;
    gl.deleteTexture(img.tex);
    images.delete(slot | 0);
  }

  function destroy() {
    if (dead) return;
    for (let h = 1; h < handles.length; h++) if (handles[h]) release(h);
    dead = true;
    for (const img of images.values()) gl.deleteTexture(img.tex);
    images.clear();
    for (const m of meshes) {
      gl.deleteBuffer(m.vbo);
      gl.deleteBuffer(m.ibo);
    }
    for (let k = 0; k < instBuf.length; k++) {
      if (instBuf[k]) gl.deleteBuffer(instBuf[k]);
      instBuf[k] = null;
      instCap[k] = 0;
    }
    if (hasVAO) {
      for (const s of slots) if (s && s.vao) deleteVAO(s.vao);
      deleteVAO(screenVAO);
    }
    gl.deleteProgram(WP.program);
    gl.deleteProgram(sprogram);
    if (WS) gl.deleteProgram(WS.program);
    if (Z) gl.deleteProgram(Z.program);
    if (zTex) { gl.deleteTexture(zTex); gl.deleteRenderbuffer(zDepth); gl.deleteFramebuffer(zFbo); }
  }

  return { draw, stats, destroy, setImage, deleteImage, resident, update, release, instancing: isGL2 ? 'webgl2' : 'ANGLE_instanced_arrays', vertexArrays: hasVAO, screenPrecision: PF };
}
