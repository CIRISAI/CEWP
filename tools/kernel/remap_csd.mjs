// Re-resolves CEWP's CSD-to-namespace synthesis (docs/kernel/tables/csd_namespace_map.json) against the vendored
// constitution tables after a re-vendor (tools/kernel/vendor.sh): each card field's canonical family is what the kernel's
// matcher resolves it to now; the families list is the vendored transitions' families (rule ids from the registry, the
// transitions' count and roles; csd_ui_surface where a card field resolves to it, else substrate_autonomous); the
// topologies' family lists follow their cards; the summary's counts are recounted. The cards themselves (ids, titles,
// fields, topologies) are the synthesis and are not touched. Prints every card field whose family moved.
// Usage: node tools/kernel/remap_csd.mjs <label, e.g. rc19>
import { readFileSync, writeFileSync } from "node:fs";
import { loadTables, matchFamily } from "../../docs/kernel/kernel.js";
const dir = new URL("../../docs/kernel/tables/", import.meta.url);
const J = (f) => JSON.parse(readFileSync(new URL(f, dir)));
const label = process.argv[2]; if (!label) { console.error("usage: remap_csd.mjs <label>"); process.exit(2); }
const m = J("csd_namespace_map.json"), tr = J("transitions.json"), pol = J("governance_policy.json"), reg = J("namespace_registry.json");
const t = loadTables(tr, pol, reg, m.aliases);
const moved = [];
for (const c of m.cards) for (const s of c.shows) {
  if (s.field.startsWith("x_private:") || s.field.startsWith("holds_bytes:")) continue;
  const r = matchFamily(s.field, t);
  if (r.family !== s.canonical_family) { moved.push(`${c.id} ${s.field}: ${s.canonical_family} -> ${r.family}${r.refusal ? " (refused: " + r.refusal + ")" : ""}`); s.canonical_family = r.family; }
}
const ruleIds = new Map(), oldFam = new Map(m.families.map((f) => [f.pattern, f]));
for (const f of reg.families) for (const leaf of [f.prefix, ...(f.leaves || [])]) if (!ruleIds.has(leaf)) ruleIds.set(leaf, f.cc_section ? ["CC " + f.cc_section] : []);
const refs = new Map(), tops = new Map();
for (const c of m.cards) for (const s of c.shows) { const k = s.canonical_family; if (!k) continue;
  if (!refs.has(k)) { refs.set(k, new Set()); tops.set(k, new Set()); } refs.get(k).add(c.id); for (const tp of c.topologies) tops.get(k).add(tp); }
// a family the synthesis already had keeps its reading (category, cards, topologies); a family new at this commit takes
// the union of the old families whose names now resolve to it (rc19's capacity:{factor} from rc17's five capacity rows),
// else what the cards' fields say
const heirs = new Map();
for (const o of m.families) { if (tr.rows.some((r) => r.family === o.pattern)) continue; const r = matchFamily(o.pattern, t);
  if (r.family) { if (!heirs.has(r.family)) heirs.set(r.family, []); heirs.get(r.family).push(o); } else moved.push(`family ${o.pattern}: gone at ${label}, no heir`); }
m.families = tr.rows.map((r) => {
  const o = oldFam.get(r.family), ref = refs.get(r.family), hs = heirs.get(r.family) || [];
  const roles = [...new Set(r.transitions.flatMap((x) => x.roles.value || []))].sort();
  if (o) return { ...o, transitions_count: r.transitions.length, roles_required: roles };
  const cards = new Set([...(ref || []), ...hs.flatMap((h) => h.referenced_by_csd_cards)]), tp = new Set([...(ref ? tops.get(r.family) : []), ...hs.flatMap((h) => h.topologies)]);
  if (hs.length) moved.push(`family ${r.family}: new at ${label}, heir of ${hs.map((h) => h.pattern).join(", ")}`);
  return { pattern: r.family, authority: hs.length ? hs[0].authority : "registry", rule_ids: hs.length ? [...new Set(hs.flatMap((h) => h.rule_ids))].sort() : (ruleIds.get(r.family) || []),
    transitions_count: r.transitions.length, roles_required: roles, category: cards.size || hs.some((h) => h.category === "csd_ui_surface") ? "csd_ui_surface" : "substrate_autonomous",
    referenced_by_csd_cards: [...cards].sort(), topologies: [...tp].sort() };
});
const known = new Set(m.families.map((f) => f.pattern));
for (const [tid, tp] of Object.entries(m.topologies)) {
  const fams = new Set(); for (const cid of tp.cards || []) { const c = m.cards.find((x) => x.id === cid); if (c) for (const s of c.shows) if (known.has(s.canonical_family)) fams.add(s.canonical_family); }
  for (const f of tp.families || []){ if (known.has(f)) fams.add(f); else { const r = matchFamily(f, t); if (r.family && known.has(r.family)) fams.add(r.family); } }
  tp.families = [...fams].sort();
}
const mapped = m.families.filter((f) => f.category === "csd_ui_surface").length;
Object.assign(m.summary, { total_families: tr.rows.length, total_transitions: tr.rows.reduce((a, r) => a + r.transitions.length, 0), total_policies: pol.families.length,
  csd_mapped_families: mapped, substrate_autonomous_families: tr.rows.length - mapped });
m.version = `cewp-synthesis-${label}-v6`;
m._meta.description = m._meta.description.replace(/CC rc\d+'s namespace_registry\.json \(\d+ families\)/, `CC ${label}'s namespace_registry.json (${tr.rows.length} families)`);
writeFileSync(new URL("csd_namespace_map.json", dir), JSON.stringify(m, null, 2) + "\n");
console.log(moved.join("\n") || "no card field moved"); console.log(JSON.stringify(m.summary));
