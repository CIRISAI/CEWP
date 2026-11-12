// The globe's renderer upgrades, called from draw() and loop() in globe.js. WebGL1 with two
// extensions every current iOS Safari and Android Chrome exposes (ANGLE_instanced_arrays,
// OES_standard_derivatives); each path below falls back to the f73b918 drawing when one is missing.
// Why WebGL1 and not WebGL2: every shader on the page (globe.js, relief.js, tiling.js) is GLSL ES
// 1.00, the two extensions give us everything WebGL2 would here (instancing, derivatives), and a
// WebGL2 context would change extension behaviour under shaders we share with other packages.
//
// What is data and what is not, on this layer of the page:
//   sun position and the night side ... the simulation's clock, computed in the kernel (clock_sun)
//   night glow ........................ the population grid the sim places people on (MODEL input),
//                                       lit where the sim's clock says it is night; not satellite lights
//   relief light ...................... NOAA ETOPO 2022 elevation, shaded by the sim's sun, vertical
//                                       exaggeration ×RELIEF_EXAG (display only)
//   ocean colour, glint, atmosphere ... illustration, not data (the legend says so)
//   arcs .............................. the kernel's own deliveries (tick's arcs buffer, a sample) and,
//                                       on the flows layer, its exact counts
const GFX = {
  inst: gl.getExtension("ANGLE_instanced_arrays"),
  deriv: gl.getExtension("OES_standard_derivatives"),
  hiFrag: (() => { const f = gl.getShaderPrecisionFormat(gl.FRAGMENT_SHADER, gl.HIGH_FLOAT); return !!(f && f.precision > 0); })(),
  reduced: !!(window.matchMedia && matchMedia("(prefers-reduced-motion: reduce)").matches),
  // resolution governor: the canvas renders at dpr × scale; scale falls when frames run long
  scale: 1, dprCap: 2, ema: 16.7, slow: 0, fast: 0, budget: 16.7,
  frames: [], drawMs: [], lastFrame: 0, tickFrame: false,
  // idle: nothing moves, nothing loads, the sim is paused → no draw at all
  dirtyUntil: 0, idleFrames: 0, skipped: 0, drawn: 0,
  sun: [0, 0, 1], sunLL: [0, 0], utc: 12,
  progs: {},
};
if (window.matchMedia){ try { matchMedia("(prefers-reduced-motion: reduce)").addEventListener("change", (e) => { GFX.reduced = e.matches; gfxDirty(1000); }); } catch (e) {} }
const FP = GFX.hiFrag ? "precision highp float;" : "precision mediump float;";
function gfxDirty(ms){ GFX.dirtyUntil = Math.max(GFX.dirtyUntil, performance.now() + (ms || 600)); }
// any texture or buffer upload (a tile arriving, a layer filling) asks for frames
{ const t = gl.texImage2D.bind(gl), b = gl.bufferData.bind(gl), s = gl.bufferSubData.bind(gl);
  gl.texImage2D = function(){ gfxDirty(400); return t.apply(null, arguments); };
  gl.bufferData = function(){ gfxDirty(400); return b.apply(null, arguments); };
  gl.bufferSubData = function(){ gfxDirty(200); return s.apply(null, arguments); }; }
for (const ev of ["pointerdown", "pointermove", "wheel", "keydown", "touchstart", "touchmove", "resize", "gesturechange", "click"])
  window.addEventListener(ev, () => gfxDirty(1200), { passive: true });
document.addEventListener("visibilitychange", () => gfxDirty(600));

// ---- the simulation's clock. Sim seconds since the epoch (00:00 UTC on day SIM_DOY). It runs only
// while the sim plays; the story sets its own hour per step. The kernel turns it into the sun.
const SIM_DOY = 314; // MODEL scenario parameter: 10 November (the day the page's numbers were taken)
// the sim's clock opens at the visitor's real UTC time of day, so the first view (their own longitude,
// globe.js HOME_LON) shows their side of the world at their local hour, lit as it is now
const SIM_T0 = (() => { try { return (Date.now() / 1000) % 86400; } catch (e) { return 8.5 * 3600; } })();
// rate: sim seconds per real second. 1 = real time, so the lit side is where it is now and does not
// sweep (it was 0.85 h a second, a whole day in 28 s); the Story's acts set their own hours
const SIM = { t: SIM_T0, rate: 1, tTick: SIM_T0, lastNow: performance.now(), diurnal: 1 }; // diurnal: who signs and who lights follow local time (workload.rs); 0 only for comparison runs
function simClockAdvance(now){
  const dt = Math.min(0.25, Math.max(0, (now - SIM.lastNow) / 1000)); SIM.lastNow = now;
  if (typeof playing !== "undefined" && playing && !(typeof storyActive !== "undefined" && storyActive)) SIM.t += dt * SIM.rate;
}
// before a tick: the kernel's clock spans from the last tick to now, signers follow the diurnal wave
function simClockForTick(){
  if (!wasm || !wasm.clock_set) return;
  const t = (typeof storyActive !== "undefined" && storyActive) ? getSimUtcHours(getTime()) * 3600 : SIM.t;
  const span = Math.max(0, t - SIM.tTick); SIM.tTick = t;
  wasm.clock_set(t, span > 0 && span < 86400 ? span : 0.12 * SIM.rate, SIM_DOY, SIM.diurnal);
}
// the sun for this frame, from the kernel; world frame as xyz() in globe.js
function gfxSun(simHours){
  GFX.simH = simHours; // sim hours since 00:00 UTC of day SIM_DOY (the people layer reads its hour of week)
  const t = simHours * 3600;
  if (wasm && wasm.clock_sun){
    wasm.clock_sun(t);
    const s = new Float64Array(mem.buffer, wasm.globe_buf_ptr(0) >>> 0, 8);
    GFX.sunLL = [s[0], s[1]]; GFX.utc = s[2];
  } else { GFX.sunLL = [18, (12 - simHours) * 15]; GFX.utc = simHours % 24; } // no kernel: the f73b918 sun
  GFX.sun = xyz(GFX.sunLL[0], GFX.sunLL[1]);
  return GFX.sun;
}

// ---- frame bookkeeping and the resolution governor
// Budget: 16.7 ms a frame (60 Hz). The governor watches the frame interval (frames that ran a sim
// tick are left out: their cost is the kernel, not the GPU) and lowers the backing-store scale by
// 15 % after 20 slow frames (EMA > 1.25 × budget), raising it 10 % after 120 fast ones (< 0.7 × budget).
function gfxFrameBegin(w, h){
  const now = performance.now(), iv = now - (GFX.lastFrame || now); GFX.lastFrame = now;
  if (iv > 0 && iv < 250 && !GFX.tickFrame){
    GFX.ema = GFX.ema * 0.9 + iv * 0.1;
    GFX.frames.push(iv); if (GFX.frames.length > 240) GFX.frames.shift();
    if (GFX.ema > GFX.budget * 1.25){ GFX.fast = 0; if (++GFX.slow > 20){ GFX.slow = 0; GFX.scale = Math.max(0.5, GFX.scale * 0.85); } }
    else if (GFX.ema < GFX.budget * 0.7){ GFX.slow = 0; if (++GFX.fast > 120){ GFX.fast = 0; GFX.scale = Math.min(1, GFX.scale * 1.1); } }
    else { GFX.slow = 0; GFX.fast = 0; }
  }
  GFX.tickFrame = false;
  const dpr = Math.min(GFX.dprCap, window.devicePixelRatio || 1), k = Math.max(0.75, dpr * GFX.scale);
  const bw = Math.round(w * k), bh = Math.round(h * k);
  if (cv.width !== bw || cv.height !== bh){ cv.width = bw; cv.height = bh; }
  GFX.k = k; return k;
}
function gfxStats(){
  const f = GFX.frames.slice().sort((a, b) => a - b), d = GFX.drawMs.slice().sort((a, b) => a - b);
  const q = (A, p) => A.length ? +A[Math.min(A.length - 1, Math.floor(A.length * p))].toFixed(2) : null;
  return { frame_p50: q(f, 0.5), frame_p95: q(f, 0.95), draw_cpu_p50: q(d, 0.5), draw_cpu_p95: q(d, 0.95), scale: +GFX.scale.toFixed(3), backing: [cv.width, cv.height], drawn: GFX.drawn, skipped: GFX.skipped, instancing: !!GFX.inst, reduced: GFX.reduced };
}
// Does this frame need drawing? Not when the sim is paused, the camera is still, nothing is fading
// and nothing has loaded: then the loop costs a requestAnimationFrame callback and nothing on the GPU.
function gfxNeedsDraw(now){
  if (document.hidden) return false;
  if (now < GFX.dirtyUntil) return true;
  if (typeof playing !== "undefined" && playing) return true;
  if (typeof storyActive !== "undefined" && storyActive && !storyPaused) return true;
  if (drag || pinch) return true;
  if (Math.abs(velYaw) > 0.00002 || Math.abs(velPitch) > 0.00002) return true;
  if (Math.abs(targetZoom / zoom - 1) > 1e-4) return true;
  if (typeof microGlide !== "undefined" && microGlide > 0) return true;
  if (activeArcs.length > 0 || (activeBlocks && activeBlocks.length > 0)) return true;
  if (typeof microVisible === "function" && microVisible() && typeof M !== "undefined" && M && microRunning()) return true;
  if (typeof LAYERS !== "undefined" && LAYERS.animating()) return true;
  return false;
}
// the street world runs on the sim's clock: it steps while the sim plays (or the story drives it)
function microRunning(){ return (typeof playing !== "undefined" && playing) || (typeof microStory !== "undefined" && microStory); }

// ---- shared shader pieces
function gfxProg(name, v, f){ if (!GFX.progs[name]){ const p = prog(v, f); const L = { p, u: {}, a: {} };
  const nu = gl.getProgramParameter(p, gl.ACTIVE_UNIFORMS); for (let i = 0; i < nu; i++){ const u = gl.getActiveUniform(p, i); L.u[u.name.replace(/\[0\]$/, "")] = gl.getUniformLocation(p, u.name); }
  const na = gl.getProgramParameter(p, gl.ACTIVE_ATTRIBUTES); for (let i = 0; i < na; i++){ const a = gl.getActiveAttrib(p, i); L.a[a.name] = gl.getAttribLocation(p, a.name); }
  GFX.progs[name] = L; } return GFX.progs[name]; }
function gfxCommon(L, R, S, asp){ gl.useProgram(L.p); if (L.u.R) gl.uniformMatrix3fv(L.u.R, false, R); if (L.u.S) gl.uniform1f(L.u.S, S); if (L.u.asp) gl.uniform2fv(L.u.asp, asp); if (L.u.uRes) gl.uniform2f(L.u.uRes, cv.width, cv.height); if (L.u.uSun) gl.uniform3fv(L.u.uSun, GFX.sun); }

// ---- ocean and atmosphere (illustration, not data). Orthographic camera: every view ray is +z.
// The shell's thickness is drawn ×2 its real 100 km so it reads at planet scale; Rayleigh weights
// are 1/λ⁴ at 680/550/440 nm; Mie is Henyey-Greenstein, g = 0.76; twilight reaches 18° past the
// terminator because the shell is lit from beyond it.
const ATM_R = 1.032;
const OV = `attribute vec2 p;uniform float S;uniform vec2 asp;varying vec2 vPos;void main(){vPos=p*${ATM_R.toFixed(3)}*1.06;gl_Position=vec4(vPos.x*S*asp.x,vPos.y*S*asp.y,0.95,1.0);}`;
const OF = `${FP}uniform vec3 uSunCam;uniform float uHaze;uniform float uZoom;uniform float uReduced;varying vec2 vPos;
const vec3 BR=vec3(0.18,0.42,1.0);const float RA=${ATM_R.toFixed(3)};
vec3 scatter(float L,float lit,float mu){float ray=0.75*(1.0+mu*mu);float g=0.76;float mie=(1.0-g*g)/pow(1.0+g*g-2.0*g*mu,1.5)*0.08;vec3 ext=1.0-exp(-BR*L*3.2);return (ext*ray+vec3(mie)*min(1.0,L*4.0))*lit;}
void main(){float r=length(vPos);if(r>RA*1.04)discard;float mu=-uSunCam.z;
if(r<=1.0){vec3 n=vec3(vPos,sqrt(max(0.0,1.0-r*r)));float sd=dot(n,uSunCam);
 float L=sqrt(max(0.0,RA*RA-r*r))-n.z;float lit=smoothstep(-0.16,0.22,sd);
 vec3 haze=scatter(L,lit,mu);
 if(uHaze>0.5){gl_FragColor=vec4(haze*0.6/(1.0+uZoom*0.6),1.0);return;}
 float day=smoothstep(-0.10,0.16,sd);float lam=max(sd,0.0);
 vec3 deep=vec3(0.020,0.066,0.135)*(0.22+1.55*pow(lam,0.8));
 vec3 h=normalize(uSunCam+vec3(0.0,0.0,1.0));float nh=max(dot(n,h),0.0);
 float fres=0.02+0.98*pow(1.0-n.z,5.0);
 float glint=(pow(nh,900.0)*1.1+pow(nh,90.0)*0.22+pow(nh,12.0)*0.04)*smoothstep(0.0,0.25,sd)*sqrt(n.z);
 vec3 sky=vec3(0.36,0.60,0.98)*day;
 vec3 col=deep+sky*fres*0.55+vec3(1.0,0.95,0.86)*glint+vec3(0.006,0.010,0.022)*(1.0-day);
 
 gl_FragColor=vec4(col,1.0);
}else{
 if(uHaze>0.5)discard;
 float L=2.0*sqrt(max(0.0,RA*RA-r*r));vec3 pn=normalize(vec3(vPos/r,0.0));float lit=smoothstep(-0.20,0.22,dot(pn,uSunCam));
 vec3 c=scatter(L,lit,mu)*1.25;
 gl_FragColor=vec4(vec3(0.043,0.051,0.07)+c,1.0);
}}`;
let gfxDiscBuf = null;
function gfxOcean(S, asp, sunCam, haze){
  const L = gfxProg("ocean", OV, OF);
  if (!gfxDiscBuf){ const v = []; const N = 128; for (let i = 0; i < N; i++){ const a0 = i / N * 6.2831853, a1 = (i + 1) / N * 6.2831853; v.push(0, 0, Math.cos(a0), Math.sin(a0), Math.cos(a1), Math.sin(a1)); } gfxDiscBuf = gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER, gfxDiscBuf); gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(v), gl.STATIC_DRAW); }
  gl.useProgram(L.p); gl.uniform1f(L.u.S, S); gl.uniform2fv(L.u.asp, asp); gl.uniform3fv(L.u.uSunCam, sunCam); gl.uniform1f(L.u.uHaze, haze ? 1 : 0); gl.uniform1f(L.u.uZoom, Math.max(0, zoom - 1)); gl.uniform1f(L.u.uReduced, GFX.reduced ? 1 : 0);
  gl.bindBuffer(gl.ARRAY_BUFFER, gfxDiscBuf); gl.enableVertexAttribArray(L.a.p); gl.vertexAttribPointer(L.a.p, 2, gl.FLOAT, false, 0, 0);
  if (haze){ gl.blendFunc(gl.ONE, gl.ONE); } else { gl.disable(gl.BLEND); }
  gl.drawArrays(gl.TRIANGLES, 0, 128 * 3);
  gl.enable(gl.BLEND); gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
}

// ---- night glow from the population grid (MODEL input): a 1024 × 512 equirectangular image of the
// grid's own density (each 1.5° cell's weight over its scatter square), blurred by 3 px so the cell
// squares do not show; lit where the kernel's sun is below the horizon.
let gfxGlow = null;
function gfxGlowInit(){
  const G = initPopGrid(), W2 = 1024, H2 = 512, img = new Float32Array(W2 * H2);
  let dmax = 0; const dens = (c) => { const w = G.cumW[c] - (c ? G.cumW[c - 1] : 0), r = cellHalf(G, c); return w / (r * r); };
  for (let c = 0; c < G.nCells; c++) dmax = Math.max(dmax, dens(c));
  for (let c = 0; c < G.nCells; c++){
    const r = cellHalf(G, c), d = dens(c) / dmax;
    const x0 = Math.floor((G.cellLon[c] - r + 180) / 360 * W2), x1 = Math.ceil((G.cellLon[c] + r + 180) / 360 * W2);
    const y0 = Math.floor((90 - G.cellLat[c] - r) / 180 * H2), y1 = Math.ceil((90 - G.cellLat[c] + r) / 180 * H2);
    for (let y = Math.max(0, y0); y < Math.min(H2, y1); y++) for (let x = x0; x < x1; x++){ const xx = ((x % W2) + W2) % W2; img[y * W2 + xx] = Math.max(img[y * W2 + xx], d); }
  }
  const tmp = new Float32Array(W2 * H2), R = 3;
  // the same two box passes as running sums (each pixel adds one and drops one), not 7 reads a pixel a pass: this
  // init was the page's longest task at load on a phone at 4x CPU throttle
  for (let pass = 0; pass < 2; pass++){
    for (let y = 0; y < H2; y++){ const o = y * W2; let s = 0; for (let k = -R; k <= R; k++) s += img[o + ((k + W2) % W2)];
      for (let x = 0; x < W2; x++){ tmp[o + x] = s / (2 * R + 1); s += img[o + ((x + R + 1) % W2)] - img[o + ((x - R + W2) % W2)]; } }
    for (let x = 0; x < W2; x++){ let s = 0, n = 0; for (let k = 0; k <= R && k < H2; k++){ s += tmp[k * W2 + x]; n++; }
      for (let y = 0; y < H2; y++){ img[y * W2 + x] = s / n; const add = y + R + 1, drop = y - R; if (add < H2){ s += tmp[add * W2 + x]; n++; } if (drop >= 0){ s -= tmp[drop * W2 + x]; n--; } } }
  }
  // scale: the 99th percentile of lit pixels is full brightness (one extreme cell would otherwise dim every other)
  const nzv = []; for (let i = 0; i < img.length; i += 7) if (img[i] > 0) nzv.push(img[i]); nzv.sort((a, b) => a - b);
  const q = nzv.length ? nzv[Math.floor(nzv.length * 0.99)] : 1;
  const u8 = new Uint8Array(W2 * H2); for (let i = 0; i < u8.length; i++) u8[i] = Math.min(255, Math.round(255 * Math.log1p(9 * Math.min(1, img[i] / q)) / Math.log(10)));
  const tex = gl.createTexture(); gl.bindTexture(gl.TEXTURE_2D, tex);
  const rgba = new Uint8Array(W2 * H2 * 4); for (let i = 0; i < u8.length; i++){ rgba[i * 4] = u8[i]; rgba[i * 4 + 3] = 255; } // RGBA: LUMINANCE sampled wrong on one ANGLE/Intel path
  gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, W2, H2, 0, gl.RGBA, gl.UNSIGNED_BYTE, rgba);
  gl.generateMipmap(gl.TEXTURE_2D); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR_MIPMAP_LINEAR); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.REPEAT); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  const st = 2, V = new Float32Array((180 / st) * (360 / st) * 6 * 5); let vn = 0; const put = (la, lo) => { const q = xyz(la, lo); V[vn++] = q[0]; V[vn++] = q[1]; V[vn++] = q[2]; V[vn++] = (lo + 180) / 360; V[vn++] = (90 - la) / 180; };
  for (let la = 90; la > -90; la -= st) for (let lo = -180; lo < 180; lo += st){ put(la, lo); put(la, lo + st); put(la - st, lo + st); put(la, lo); put(la - st, lo + st); put(la - st, lo); }
  const buf = gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER, buf); gl.bufferData(gl.ARRAY_BUFFER, V, gl.STATIC_DRAW);
  gfxGlow = { tex, buf, n: V.length / 5, u8, W2, H2 };
}
const NGV = `attribute vec3 p;attribute vec2 uv;uniform mat3 R;uniform float S;uniform vec2 asp;uniform vec3 uSun;varying vec2 vUv;varying float vz;varying float vSun;void main(){vec3 q=R*p;vz=q.z;vSun=dot(p,uSun);vUv=uv;gl_Position=vec4(q.x*S*asp.x,q.y*S*asp.y,0.9,1.0);}`;
const NGF = `${FP}uniform sampler2D T;uniform float uA;varying vec2 vUv;varying float vz;varying float vSun;void main(){if(vz<0.0)discard;float d=texture2D(T,vUv).r;float night=1.0-smoothstep(-0.14,0.03,vSun);
float g=pow(d,1.6)*night*uA;vec3 c=vec3(1.0,0.58,0.24)*g*0.62+vec3(1.0,0.84,0.60)*pow(d,5.0)*night*uA*0.55;gl_FragColor=vec4(c*smoothstep(0.0,0.08,vz),1.0);}`;
function gfxNightGlow(R, S, asp, mpp, a){
  // the glow image is built in an idle slice, not inside the first frame (it fades in when ready)
  if (a <= 0.004) return; if (!gfxGlow){ if (!GFX.glowSoon){ GFX.glowSoon = true; (window.requestIdleCallback || ((f) => setTimeout(f, 30)))(() => { gfxGlowInit(); gfxDirty(600); }, { timeout: 2000 }); } return; }
  const L = gfxProg("glow", NGV, NGF); gfxCommon(L, R, S, asp);
  // a 0.35° texel says nothing below region scale: the glow is gone by 500 m a pixel
  gl.uniform1f(L.u.uA, a * sstep(500, 4000, mpp)); gl.uniform1i(L.u.T, 0); gl.activeTexture(gl.TEXTURE0); gl.bindTexture(gl.TEXTURE_2D, gfxGlow.tex);
  gl.bindBuffer(gl.ARRAY_BUFFER, gfxGlow.buf); gl.enableVertexAttribArray(L.a.p); gl.enableVertexAttribArray(L.a.uv);
  gl.vertexAttribPointer(L.a.p, 3, gl.FLOAT, false, 20, 0); gl.vertexAttribPointer(L.a.uv, 2, gl.FLOAT, false, 20, 12);
  gl.blendFunc(gl.ONE, gl.ONE); gl.drawArrays(gl.TRIANGLES, 0, gfxGlow.n); gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
  gl.disableVertexAttribArray(L.a.uv);
}

// ---- anti-aliased lines at any zoom: every segment of an existing gl.LINES buffer becomes a
// screen-space quad, w device pixels wide plus one pixel of coverage falloff each side. The
// segment endpoints are read as instance attributes straight from that buffer (stride 24 bytes),
// so no line data is duplicated.
const AV = `attribute vec2 corner;attribute vec3 pa;attribute vec3 pb;uniform mat3 R;uniform float S;uniform vec2 asp;uniform vec2 uRes;uniform float uW;uniform vec3 uSun;varying float vEdge;varying float vz;varying float vSun;varying float vHW;
void main(){vec3 qa=R*pa,qb=R*pb;vec2 ca=vec2(qa.x*S*asp.x,qa.y*S*asp.y),cb=vec2(qb.x*S*asp.x,qb.y*S*asp.y);vec2 h=uRes*0.5;vec2 a=ca*h,b=cb*h;vec2 d=b-a;float l=length(d);d=l>1e-6?d/l:vec2(1.0,0.0);vec2 nr=vec2(-d.y,d.x);
float hw=uW*0.5+1.0;vec2 P=mix(a,b,corner.x)+d*(corner.x*2.0-1.0)*0.5+nr*corner.y*hw;vHW=uW*0.5;vEdge=corner.y*hw;vz=mix(qa.z,qb.z,corner.x);vSun=dot(mix(pa,pb,corner.x),uSun);gl_Position=vec4(P/h,-vz*0.5,1.0);}`;
const AF = `${FP}uniform vec3 uCol;uniform float uA;varying float vEdge;varying float vz;varying float vSun;varying float vHW;void main(){float edge=smoothstep(-0.02,0.12,vz);if(edge<0.01)discard;float cov=clamp(vHW+0.5-abs(vEdge),0.0,1.0);if(cov<0.003)discard;float day=smoothstep(-0.10,0.12,vSun);gl_FragColor=vec4(uCol*mix(0.48,1.0,day),uA*mix(0.6,1.0,day)*edge*cov);}`;
let gfxQuadBuf = null;
function gfxQuad(){ if (!gfxQuadBuf){ gfxQuadBuf = gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER, gfxQuadBuf); gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([0, -1, 1, -1, 1, 1, 0, -1, 1, 1, 0, 1]), gl.STATIC_DRAW); } return gfxQuadBuf; }
// draws the shorelines/borders/rivers kinds of every set (same choice of sets and colours as f73b918)
function gfxLines(R, S, asp, gsets, LS, nk, widthsPx){
  if (!GFX.inst) return false;
  const I = GFX.inst, L = gfxProg("aaline", AV, AF); gfxCommon(L, R, S, asp);
  gl.bindBuffer(gl.ARRAY_BUFFER, gfxQuad()); gl.enableVertexAttribArray(L.a.corner); gl.vertexAttribPointer(L.a.corner, 2, gl.FLOAT, false, 0, 0);
  gl.enableVertexAttribArray(L.a.pa); gl.enableVertexAttribArray(L.a.pb); I.vertexAttribDivisorANGLE(L.a.pa, 1); I.vertexAttribDivisorANGLE(L.a.pb, 1);
  for (let k = 0; k < nk; k++){
    gl.uniform3fv(L.u.uCol, LS[k][0]); gl.uniform1f(L.u.uA, LS[k][1] * GEO.rf); gl.uniform1f(L.u.uW, widthsPx[k] * GFX.k);
    for (const G of gsets){ const Ln = (k === 3) ? G.shoreK : G.lines[k]; if (!Ln || !Ln.n) continue;
      gl.bindBuffer(gl.ARRAY_BUFFER, Ln.b); gl.vertexAttribPointer(L.a.pa, 3, gl.FLOAT, false, 24, 0); gl.vertexAttribPointer(L.a.pb, 3, gl.FLOAT, false, 24, 12);
      I.drawArraysInstancedANGLE(gl.TRIANGLES, 0, 6, Ln.n / 2); }
  }
  I.vertexAttribDivisorANGLE(L.a.pa, 0); I.vertexAttribDivisorANGLE(L.a.pb, 0); gl.disableVertexAttribArray(L.a.pa); gl.disableVertexAttribArray(L.a.pb);
  return true;
}

// ---- arcs as instanced ribbons: one instance per arc (endpoints, birth, life, tier, width), the
// great circle and its lift computed on the GPU. A point of the arc is hidden exactly when the
// sphere stands between it and the eye (orthographic: q.z < 0 and |q.xy| < 1), so arcs pass
// behind the limb correctly instead of fading by depth. Style by tier is a dash pattern as well as a
// colour (class is never colour alone): 0 shaped = solid, 1 seat = long dash, 2 relay tree /
// unshaped = beads, 3 onion = double rule. Flowing heads move from sender to receiver; with
// prefers-reduced-motion the ribbon is drawn whole and still.
const ARC_SEG = 40, ARC_STRIDE = 12; // floats per instance: a.xyz b.xyz birth life tier width kind lift
const RV2 = `attribute vec2 corner;attribute vec3 ia;attribute vec3 ib;attribute vec4 im;attribute vec2 ik;uniform mat3 R;uniform float S;uniform vec2 asp;uniform vec2 uRes;uniform float uTime;uniform float uDpr;
varying float vT;varying float vEdge;varying float vHW;varying float vAge;varying float vTier;varying float vHid;varying float vKind;varying float vLen;
vec3 arcP(float t,float th,float lift){float s=sin(th);float w1=s<1e-4?1.0-t:sin((1.0-t)*th)/s;float w2=s<1e-4?t:sin(t*th)/s;vec3 v=normalize(ia*w1+ib*w2);return v*(1.0+lift*sin(t*3.14159265));}
void main(){float th=acos(clamp(dot(ia,ib),-1.0,1.0));float lift=ik.y;float t=corner.x;vec3 p=arcP(t,th,lift);vec3 p2=arcP(min(1.0,t+0.02),th,lift);vec3 p0=arcP(max(0.0,t-0.02),th,lift);
vec3 q=R*p,qa=R*p0,qb=R*p2;vec2 h=uRes*0.5;vec2 c=vec2(q.x*S*asp.x,q.y*S*asp.y)*h;vec2 d=(vec2(qb.x*S*asp.x,qb.y*S*asp.y)-vec2(qa.x*S*asp.x,qa.y*S*asp.y))*h;float l=length(d);d=l>1e-6?d/l:vec2(1.0,0.0);
float hw=im.w*0.5*uDpr+1.0;vec2 P=c+vec2(-d.y,d.x)*corner.y*hw;vHW=im.w*0.5*uDpr;vEdge=corner.y*hw;vT=t;vAge=im.y>0.0?(uTime-im.x)/im.y:-1.0;vTier=im.z;vKind=ik.x;vLen=th;
vHid=(q.z<0.0)?(1.0-smoothstep(1.0,1.012,length(q.xy))):0.0;gl_Position=vec4(P/h,0.0,1.0);}`;
const RF2 = `${FP}uniform float uTime;uniform float uReduced;varying float vT;varying float vEdge;varying float vHW;varying float vAge;varying float vTier;varying float vHid;varying float vKind;varying float vLen;
void main(){bool still=vAge<0.0;if(!still&&(vAge<=0.0||vAge>=1.0))discard;float cov=clamp(vHW+0.5-abs(vEdge),0.0,1.0);if(cov<0.004)discard;float vis=1.0-vHid;if(vis<0.01)discard;
float env=still?1.0:smoothstep(0.0,0.16,vAge)*(1.0-smoothstep(0.62,1.0,vAge));
vec3 col=vec3(1.0,0.62,0.30);if(vTier>3.5)col=vec3(1.0,0.84,0.52);else if(vTier>2.5)col=vec3(0.78,0.56,1.0);else if(vTier>1.5)col=vec3(0.36,0.90,0.62);else if(vTier>0.5)col=vec3(0.42,0.74,1.0);
float pat=1.0;float k=vT*max(6.0,vLen*60.0);
if(vTier>3.5){pat=1.0;}else if(vTier>2.5){pat=0.55+0.45*step(0.35,abs(vEdge)/max(1.0,vHW+1.0));}
else if(vTier>1.5){pat=0.30+0.70*smoothstep(0.42,0.18,abs(fract(k)-0.5));}
else if(vTier>0.5){pat=0.25+0.75*step(0.28,fract(k*0.5));}
float head=0.0;if(uReduced<0.5){float hp=still?fract(uTime*0.22+vLen*0.37):fract(vAge*1.35);float dd=vT-hp;head=0.55*exp(-dd*dd*500.0)+0.25*smoothstep(-0.25,0.0,dd)*step(dd,0.0);}else{head=0.0;}
float base=(vKind>0.5)?0.26:0.20;float a=min(1.0,env*(base*pat+head*(vKind>0.5?0.9:0.6)))*cov*vis;if(a<0.004)discard;
vec3 c=mix(col,vec3(1.0,0.97,0.9),clamp(head*0.35,0.0,0.3));gl_FragColor=vec4(c,min(a,0.72));}`;
const ARCS = { cap: 24000, data: null, n: 0, buf: null, geo: null, dirty: true };
function gfxArcGeo(){ if (!ARCS.geo){ const v = []; for (let i = 0; i <= ARC_SEG; i++){ const t = i / ARC_SEG; v.push(t, -1, t, 1); } ARCS.geo = gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER, ARCS.geo); gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(v), gl.STATIC_DRAW); } return ARCS.geo; }
// Arc sets: the tick's sample arcs (globe.js activeArcs) and any layer's arcs, packed into one
// instance buffer per frame they change. Each arc: {a:[x,y,z], b:[x,y,z], birth, life, tier, width, kind, lift}
function gfxArcsDraw(R, S, asp, nowSec, sets){
  if (!GFX.inst) return false;
  let total = 0; for (const s of sets) total += s.length;
  if (!total) return true;
  if (!ARCS.data || ARCS.data.length < total * ARC_STRIDE) ARCS.data = new Float32Array(Math.min(ARCS.cap, Math.max(1024, total * 2)) * ARC_STRIDE);
  let w = 0; const D = ARCS.data, cap = D.length / ARC_STRIDE;
  for (const s of sets) for (const r of s){ if (w >= cap) break; const o = w * ARC_STRIDE;
    D[o] = r.a[0]; D[o + 1] = r.a[1]; D[o + 2] = r.a[2]; D[o + 3] = r.b[0]; D[o + 4] = r.b[1]; D[o + 5] = r.b[2];
    D[o + 6] = r.birth; D[o + 7] = r.life; D[o + 8] = r.tier; D[o + 9] = r.width; D[o + 10] = r.kind || 0; D[o + 11] = r.lift; w++; }
  if (!ARCS.buf) ARCS.buf = gl.createBuffer();
  gl.bindBuffer(gl.ARRAY_BUFFER, ARCS.buf); gl.bufferData(gl.ARRAY_BUFFER, D.subarray(0, w * ARC_STRIDE), gl.STREAM_DRAW);
  const I = GFX.inst, L = gfxProg("arc", RV2, RF2); gfxCommon(L, R, S, asp);
  gl.uniform1f(L.u.uTime, nowSec); gl.uniform1f(L.u.uDpr, GFX.k); gl.uniform1f(L.u.uReduced, GFX.reduced ? 1 : 0);
  gl.bindBuffer(gl.ARRAY_BUFFER, gfxArcGeo()); gl.enableVertexAttribArray(L.a.corner); gl.vertexAttribPointer(L.a.corner, 2, gl.FLOAT, false, 0, 0);
  gl.bindBuffer(gl.ARRAY_BUFFER, ARCS.buf); const B = ARC_STRIDE * 4;
  for (const [nm, sz, off] of [["ia", 3, 0], ["ib", 3, 12], ["im", 4, 24], ["ik", 2, 40]]){ gl.enableVertexAttribArray(L.a[nm]); gl.vertexAttribPointer(L.a[nm], sz, gl.FLOAT, false, B, off); I.vertexAttribDivisorANGLE(L.a[nm], 1); }
  // ordinary alpha blending (not additive): where many arcs cross they do not sum to white
  gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
  I.drawArraysInstancedANGLE(gl.TRIANGLE_STRIP, 0, (ARC_SEG + 1) * 2, w);
  gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
  for (const nm of ["ia", "ib", "im", "ik"]){ I.vertexAttribDivisorANGLE(L.a[nm], 0); gl.disableVertexAttribArray(L.a[nm]); }
  GFX.arcsDrawn = w;
  return true;
}
// the tick's sample arcs as arc records (kept beside globe.js activeArcs, built once per arc)
function gfxArcRec(sx, tx, birth, life, tier){ const a = xyz(W.lat[sx], W.lon[sx]), b = xyz(W.lat[tx], W.lon[tx]); const th = Math.acos(Math.max(-1, Math.min(1, a[0] * b[0] + a[1] * b[1] + a[2] * b[2]))); return { a, b, birth, life, tier, width: 0.95, kind: 0, lift: 0.015 + 0.10 * (th / Math.PI) }; }
