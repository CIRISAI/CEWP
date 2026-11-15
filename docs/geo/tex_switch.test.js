// The ground texture's switch (docs/tiling.js, FSD/twin/texture.md known limits): the texture needs highp in the fragment
// stage, so without it the switch is disabled, reads "not on this device", ?tex=1 has no effect and the shader's texture
// path stays off; with highp the switch and the shader's input are exactly what they were before the gate.
"use strict";
const test = require("node:test"), assert = require("node:assert");
const fs = require("node:fs"), path = require("node:path"), vm = require("node:vm");
const read = (p) => fs.readFileSync(path.join(__dirname, "..", p), "utf8");
const tiling = read("tiling.js"), globe = read("globe.js");
const block = tiling.slice(tiling.indexOf("// ---- the texture switch"), tiling.indexOf("// ---- end of the texture switch"));
const ctx = vm.createContext({});
vm.runInContext(block + "\n;globalThis.T = { tlTexOk, tlTexMake, tlTexUniform, tlTexToggle, tlTexButton };", ctx);
const { tlTexMake, tlTexUniform, tlTexToggle, tlTexButton } = ctx.T;
// the switch and the uniform as they were before the gate
const oldButton = (on) => `<button type="button" class="dm-b" data-tl-tex aria-pressed="${on}">ground texture: ${on ? "procedural" : "flat"}</button>`;
const oldUniform = (on, mapActive) => on ? (mapActive ? 0.5 : 1) : 0;

test("the switch block is found in tiling.js and used by the page", () => {
  assert.ok(block.length > 200, "markers present");
  assert.match(tiling, /const TL_TEX = tlTexMake\(typeof GFX !== "undefined" \? GFX : null/);
  assert.match(tiling, /gl\.uniform1f\(tlLoc\.tex, tlTexUniform\(TL_TEX,/);
  assert.match(globe, /typeof TL_TEX !== "undefined" \? tlTexButton\(TL_TEX\)/);
  assert.match(globe, /closest\("\[data-tl-tex\]"\); if \(tx && typeof TL_TEX !== "undefined" && TL_TEX\.ok\)/);
  assert.match(tiling, /if\(uTex>0\.5\)\{float s=0\.0;/, "the shader's noise runs only when uTex is on");
});

test("with highp: unchanged (off by default, ?tex=1 starts it on, the same button and uniform as before)", () => {
  const hi = { hiFrag: true };
  for (const [search, on] of [["", false], ["?tex=1", true], ["?a=1&tex=1", true], ["?tex=10", false]]){
    const t = tlTexMake(hi, search);
    assert.strictEqual(t.ok, true); assert.strictEqual(t.on, on, search);
    assert.strictEqual(tlTexButton(t), oldButton(on));
    for (const m of [0, 3, 7, undefined]) assert.strictEqual(tlTexUniform(t, m), oldUniform(on, m));
  }
  const t = tlTexMake(hi, ""); assert.strictEqual(tlTexToggle(t), true); assert.strictEqual(tlTexButton(t), oldButton(true));
  assert.strictEqual(tlTexToggle(t), false);
});

test("without highp: disabled, reads 'not on this device', ?tex=1 has no effect, the texture path is off", () => {
  for (const g of [{ hiFrag: false }, null, {}]){
    const t = tlTexMake(g, "?tex=1");
    assert.strictEqual(t.ok, false); assert.strictEqual(t.on, false);
    const b = tlTexButton(t);
    assert.match(b, / disabled /); assert.match(b, /aria-disabled="true"/); assert.match(b, />ground texture: not on this device</);
    assert.strictEqual(tlTexToggle(t), false, "a click does not turn it on");
    t.on = true; // even if something sets it, the shader gets no texture
    for (const m of [0, 3]) assert.strictEqual(tlTexUniform(t, m), 0);
  }
});
