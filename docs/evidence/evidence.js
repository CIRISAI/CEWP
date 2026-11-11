// The evidence layer of the twin page. Every figure carries a data-claim attribute naming an entry of the
// claims ledger (docs/evidence/claims.js, generated from crates/cewp-evidence/src/page_ledger.rs).
// A click, a tap, or Enter / Space on a figure opens its entry: class, what supports it, the
// interval, how it can be wrong, how to reproduce it. A panel holds the legend, the tier the visitor
// is looking at now, and the whole ledger. Loaded before globe.js, which calls fig().
// No network: the ledger is a local script.
const EV = (() => {
  const L = window.CEWP_LEDGER || { claims: [], classes: [] };
  return { L, by: new Map(L.claims.map(c => [c.id, c])), cls: new Map(L.classes.map(c => [c.class, c])), opener: null };
})();
function evEsc(s){ return String(s).replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]); }
function evCode(id){ const c = EV.by.get(id); return c ? c.code : "??"; }
function evClasses(id){ const c = EV.by.get(id); return "fig fig-" + evCode(id) + (c && c.status === "under_correction" ? " fig-corr" : ""); }
// a figure inside a template: the value, marked with its ledger entry
function fig(id, html){ return `<span class="${evClasses(id)}" data-claim="${id}" data-code="${evCode(id)}" tabindex="0" role="button" aria-haspopup="dialog">${html}</span>`; }
// static figures and elements that carry data-claim in the markup get the same attributes; a figure
// inside a button or a link stays part of that control and is not made a second target
function evDecorate(root){
  const all = root.querySelectorAll ? [...(root.matches && root.matches("[data-claim]") ? [root] : []), ...root.querySelectorAll("[data-claim]")] : [];
  for (const el of all){
    if (el.dataset.code) continue;
    const id = el.getAttribute("data-claim");
    el.dataset.code = evCode(id);
    evClasses(id).split(" ").forEach(k => el.classList.add(k));
    if (el.parentElement && el.parentElement.closest("button, a")){ el.classList.add("fig-inert"); continue; }
    el.setAttribute("tabindex", "0"); el.setAttribute("role", "button"); el.setAttribute("aria-haspopup", "dialog");
  }
}
function evField(label, value, code){ return value ? `<div class="ev-f"><span>${label}</span>${code ? `<code>${evEsc(value)}</code>` : `<p>${evEsc(value)}</p>`}</div>` : ""; }
function evOpen(id, opener, extra){
  const c = EV.by.get(id), card = document.getElementById("claim-card"); if (!card) return;
  EV.opener = opener || null; EV.layer = null;
  if (!c){ card.innerHTML = `<div class="panel-head"><b id="claim-title">No ledger entry: ${evEsc(id)}</b><button class="panel-close" id="claim-x" type="button" aria-label="Close">✕</button></div>${extra || ""}`; }
  else {
    const k = EV.cls.get(c.class) || { means: "" };
    const iv = c.interval ? `[${c.interval.lo}, ${c.interval.hi}] ${c.interval.what}` : "";
    card.innerHTML = `<div class="panel-head"><b id="claim-title"><span class="ev-badge ev-${c.code}">${c.code}</span> ${evEsc(c.class)}${c.live ? " · live" : ""}</b><button class="panel-close" id="claim-x" type="button" aria-label="Close">✕</button></div>
      <p class="ev-means">${evEsc(k.means)}</p>
      ${c.status === "under_correction" ? `<p class="ev-corr"><b>Under correction.</b> ${evEsc(c.correction)}</p>` : ""}
      ${evField("What the page shows", c.shows)}
      ${evField("Source", c.source)}
      ${evField("Commit", c.commit, true)}
      ${evField("Measurement window", c.window)}
      ${evField("Parameters", c.param)}
      ${evField("Interval", iv)}
      ${evField("What is missing", c.blocker)}
      <div class="ev-f"><span>Known failure conditions</span><ul>${c.failure.map(f => `<li>${evEsc(f)}</li>`).join("")}</ul></div>
      ${evField("Reproduce", c.reproduce, true)}
      ${extra || ""}
      <div class="ev-foot"><code>${evEsc(c.id)}</code> · <button type="button" class="ev-link" data-ev-open>legend, tier and full ledger</button></div>`;
  }
  card.classList.remove("panel-hidden");
  document.getElementById("claim-x").onclick = evClose;
  card.focus({ preventScroll: true });
}
// a globe layer (docs/layers/*.js registers window.CEWP_LAYERS[id] = { id, title, cls, source, contract, last }):
// its ledger entry is layer-<id>; the card adds what the layer itself declares and the values it drew this frame
function evLayerLive(id){
  const r = (window.CEWP_LAYERS || {})[id];
  if (!r) return `<div class="ev-f"><span>Layer</span><p>Not registered on this page.</p></div>`;
  const c = EV.by.get("layer-" + id), last = r.last && typeof r.last === "object" ? Object.entries(r.last) : [];
  const vals = last.length ? `<ul>${last.map(([k, v]) => `<li><code>${evEsc(k)}</code> ${evEsc(typeof v === "number" ? +v.toPrecision(4) : v)}</li>`).join("")}</ul>` : "<p>Nothing drawn yet.</p>";
  return `${c && r.cls !== c.class ? `<p class="ev-corr"><b>Class mismatch.</b> The layer declares ${evEsc(r.cls)}; its ledger entry says ${evEsc(c.class)}.</p>` : ""}
    ${evField("Layer", r.title || id)}${evField("Layer class", r.cls)}${r.inputs ? evField("Its inputs", r.inputs) : ""}${evField("Draws from", r.source, true)}${evField("Data contract", r.contract)}
    <div class="ev-f" data-ev-live><span>Values drawn this frame</span>${vals}</div>`;
}
function evOpenLayer(id, opener){
  evOpen("layer-" + id, opener, evLayerLive(id)); EV.layer = id;
}
function evLayerTick(){
  const card = document.getElementById("claim-card");
  if (!EV.layer || !card || card.classList.contains("panel-hidden")) return;
  const r = (window.CEWP_LAYERS || {})[EV.layer], box = card.querySelector("[data-ev-live]");
  if (!r || !box || box.contains(document.activeElement)) return;
  const n = document.createElement("div"); n.innerHTML = evLayerLive(EV.layer);
  const fresh = n.querySelector("[data-ev-live]"); if (fresh && fresh.innerHTML !== box.innerHTML) box.innerHTML = fresh.innerHTML;
}
function evClose(){
  const card = document.getElementById("claim-card"); if (!card || card.classList.contains("panel-hidden")) return;
  card.classList.add("panel-hidden");
  const o = EV.opener; EV.opener = null;
  if (o && document.contains(o) && o.focus) o.focus({ preventScroll: true });
}
function evPanel(open){
  const p = document.getElementById("ev-panel"); if (!p) return;
  const show = open == null ? p.classList.contains("panel-hidden") : open;
  p.classList.toggle("panel-hidden", !show);
  const b = document.getElementById("tier-now"); if (b) b.setAttribute("aria-expanded", String(show));
  if (show){ evTier(true); p.focus({ preventScroll: true }); }
}
function evLegend(){
  const rows = EV.L.classes.map(k => `<li><span class="ev-badge ev-${k.code}">${k.code}</span> <span class="ev-sample fig fig-${k.code} fig-inert" data-code="${k.code}">${k.class}</span> ${evEsc(k.means)}</li>`).join("");
  const groups = EV.L.classes.map(k => {
    const items = EV.L.claims.filter(c => c.class === k.class);
    if (!items.length) return "";
    return `<h3><span class="ev-badge ev-${k.code}">${k.code}</span> ${k.class}</h3><ul class="ev-index">${items.map(c => `<li><button type="button" class="ev-item" data-ev-claim="${c.id}">${evEsc(c.shows)}${c.status === "under_correction" ? " <b>(under correction)</b>" : ""}</button></li>`).join("")}</ul>`;
  }).join("");
  return `<div class="panel-head"><b id="ev-title">Evidence: what each figure rests on</b><button class="panel-close" id="ev-x" type="button" aria-label="Close">✕</button></div>
  <h3>Tier you are looking at now</h3>
  <p id="ev-tier" data-evidence="figs" aria-live="polite"></p>
  <h3>The classes</h3>
  <p class="ev-means">Every figure is underlined in its class's line style and tagged with its two-letter code; click, tap or press Enter on it to open its entry.</p>
  <ul class="ev-legend">${rows}</ul>
  <h3>The full ledger</h3>
  ${groups}
  <p class="ev-means">Generated from <code>crates/cewp-evidence/src/page_ledger.rs</code>; <code>cargo test -p cewp-evidence page_</code> fails when the page shows a figure with no entry.</p>`;
}
// what the visitor is looking at: the cohort tier above street zoom, the R5 patch at street zoom
// evidence:scan
function evTier(force){
  const t = document.getElementById("tier-now"); if (!t) return;
  const micro = typeof microVisible === "function" && microVisible() && typeof M !== "undefined" && M;
  const label = micro ? "Tier: R5 street, 1:1 · generated, not real" : "Tier: cohorts · simulated";
  if (t.textContent !== label) t.textContent = label;
  t.classList.toggle("tier-micro", !!micro);
  // a phone-width explorer stacks the pill under the nav strip and the live strip, whichever is lower
  if (innerWidth <= 800 && !document.body.classList.contains("story-first")){
    const below = ["live-strip", "nav-strip"].map(k => document.getElementById(k) || document.querySelector("." + k)).filter(e => e && getComputedStyle(e).display !== "none" && e.getBoundingClientRect().height > 0).map(e => e.getBoundingClientRect().bottom);
    const top = Math.round(Math.max(0, ...below) + 6) + "px"; if (t.style.top !== top) t.style.top = top;
  } else if (t.style.top) t.style.top = "";
  const p = document.getElementById("ev-tier"), panel = document.getElementById("ev-panel");
  if (!p || (!force && (!panel || panel.classList.contains("panel-hidden")))) return;
  if (p.contains(document.activeElement)) return;
  const world = typeof W !== "undefined" && W && typeof fmt === "function";
  p.innerHTML = micro
    ? `${fig("tier-micro", "The R5 micro tier")}: one generated neighbourhood in which every device is a node of its own. ${fig("generated-below-region", "Its streets, homes and devices are generated, never real.")} Outside the patch the page shows cohorts or a drawing.`
    : `${fig("tier-cohorts", "The cohort tier")}: ${world ? fig("cohort-world", fmt(W.n) + " cohorts") + " stand for " + fig("world-population", fmt(currentPop) + " people") : "cohorts stand for the population"}, ${fig("all-simulated", "all simulated")}. Zoom to street scale for the 1:1 tier. ${fig("tier-cohorts", "No tier on this page is certified.")}`;
}
// evidence:end
function evInit(){
  document.body.insertAdjacentHTML("beforeend", `<div id="claim-card" class="panel-hidden" role="dialog" aria-labelledby="claim-title" tabindex="-1"></div>
<div id="ev-panel" class="panel-hidden" role="dialog" aria-labelledby="ev-title" tabindex="-1"></div>
<button id="tier-now" type="button" aria-controls="ev-panel" aria-expanded="false" title="Which tier is running, and what each figure rests on"></button>`);
  document.getElementById("ev-panel").innerHTML = evLegend();
  document.getElementById("ev-x").onclick = () => evPanel(false);
  document.getElementById("tier-now").onclick = () => evPanel();
  const nav = document.getElementById("btn-evidence"); if (nav) nav.onclick = () => evPanel(true);
  const menu = document.getElementById("sf-menu");
  if (menu && !menu.querySelector("[data-ev-open]")){
    const b = document.createElement("button"); b.type = "button"; b.setAttribute("data-ev-open", ""); b.textContent = "Evidence: what each figure rests on";
    const src = menu.querySelector('[data-panel="foot"]'); menu.insertBefore(b, src ? src.nextSibling : null);
  }
  evDecorate(document.body);
  if (window.MutationObserver) new MutationObserver(ms => { for (const m of ms) for (const n of m.addedNodes) if (n.nodeType === 1) evDecorate(n); }).observe(document.body, { childList: true, subtree: true });
  // pointer: a press and release on the same figure without moving opens it, so a figure that a
  // panel re-renders between press and release (the device card does, every 300 ms) still opens
  let down = null;
  addEventListener("pointerdown", e => { const f = e.target.closest && e.target.closest("[data-claim]:not(.fig-inert)"); down = f ? [f.getAttribute("data-claim"), e.clientX, e.clientY] : null; }, true);
  addEventListener("pointerup", e => {
    const f = e.target.closest && e.target.closest("[data-claim]:not(.fig-inert)"), d = down; down = null;
    if (!f || !d || d[0] !== f.getAttribute("data-claim") || Math.hypot(e.clientX - d[1], e.clientY - d[2]) > 8) return;
    evOpen(d[0], f);
    EV.swallow = performance.now(); // the click this release makes lands on whatever the card now covers
  }, true);
  addEventListener("click", e => { if (EV.swallow && performance.now() - EV.swallow < 600){ EV.swallow = 0; e.preventDefault(); e.stopPropagation(); } }, true);
  addEventListener("click", e => {
    const o = e.target.closest && e.target.closest("[data-ev-open]"); if (o){ e.preventDefault(); evClose(); evPanel(true); return; }
    const it = e.target.closest && e.target.closest("[data-ev-claim]"); if (it){ evOpen(it.getAttribute("data-ev-claim"), it); return; }
    const ly = e.target.closest && e.target.closest("[data-layer-claim]"); if (ly){ e.preventDefault(); e.stopPropagation(); evOpenLayer(ly.getAttribute("data-layer-claim"), ly); }
  });
  addEventListener("keydown", e => {
    if (e.key === "Escape"){ const c = document.getElementById("claim-card"); if (c && !c.classList.contains("panel-hidden")) evClose(); else evPanel(false); return; }
    if (e.key !== "Enter" && e.key !== " ") return;
    const f = e.target.closest && e.target.closest("[data-claim]:not(.fig-inert)");
    if (f && f === e.target){ e.preventDefault(); evOpen(f.getAttribute("data-claim"), f); }
  });
  evTier(); setInterval(() => { evTier(false); evLayerTick(); }, 500);
}
if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", evInit); else setTimeout(evInit, 0);
