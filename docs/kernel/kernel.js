// The state processor of CC 4.5.13, executable.
//
// A node holds rows, each with the instant it first held it, and nothing else.
// Every verdict is a fold over that set, never stored.
// One step appends and never rewrites.
// The queue a person sees, the preview of an act and the door that admits it are one function.
//
// No dependencies. Runs in a browser as an ES module and under `node --test`.
// Tables: manifests/transitions.json and manifests/governance_policy.json of the constitution, vendored.

export const STATES = ["none", "admitted", "withdrawn", "recanted", "superseded"];

// ---------------------------------------------------------------- default tables

let defaultRegistry = null;
let defaultAliases = null;
if (typeof process !== "undefined" && process?.versions?.node) {
  try {
    const { readFileSync } = await import("node:fs");
    defaultRegistry = JSON.parse(readFileSync(new URL("./tables/namespace_registry.json", import.meta.url)));
    try {
      defaultAliases = JSON.parse(readFileSync(new URL("./tables/csd_namespace_map.json", import.meta.url))).aliases;
    } catch {}
  } catch {}
}

// ---------------------------------------------------------------- CC RC17 dimension matcher

const HEX_PATTERN = /^[0-9a-f]+$/;
const VERSION_LIKE = /^[vV][0-9]+(\.[0-9]+)*$/;
const VERSION_ATTEMPT = /^[vV][0-9][0-9A-Za-z_.\-]*$/;

export class Rules {
  constructor(manifest) {
    const meta = manifest._meta || {};
    const cr = meta.case_rule || {};
    this.vocab = new RegExp("^" + (cr.vocab_pattern || "[a-z0-9][a-z0-9_.-]*") + "$");
    this.literal = new RegExp("^" + (cr.literal_pattern || "[a-z0-9][a-z0-9_]*") + "$");
    this.private = meta.private_use_prefix || "x_private:";
    const vs = cr.version_segment || {};
    this.version = new RegExp("^" + (vs.pattern || "v[0-9]+(\\.[0-9]+)*") + "$");
    this.version_required = vs.required !== false;
    this.exempt = new Set(vs.exempt || []);
    this.tokens = cr.refusal_tokens || {
      case_malformed: "namespace_dimension_case_malformed",
      family_unregistered: "namespace_family_unregistered",
      missing_version_segment: "missing_version_segment",
      private_use_not_federatable: "namespace_private_use_not_federatable",
      vocab_value_unregistered: "namespace_vocab_value_unregistered"
    };
    this.external = {};
    for (const [name, spec] of Object.entries(cr.external_standards || {})) {
      if (spec.pattern) this.external[name] = new RegExp("^" + spec.pattern + "$");
    }
    this.reserved_stems = (cr.reserved_stems || []).map((s) => s.stem);
    this.families = (manifest.families || []).map((f, i) => ({ ...f, index: i }));
    this.familyMap = new Map();
    for (const f of this.families) this.familyMap.set(f.prefix, f);
    const cs = new Set();
    for (const f of this.families) {
      if (f.leaves_closed) {
        const last = f.segments[f.segments.length - 1];
        if (last && last.class === "wildcard") cs.add(f.prefix.slice(0, -1));
        else cs.add(f.prefix.split(":")[0] + ":");
      }
    }
    this.closed_stems = [...cs].sort();
  }
}

function score(fam) {
  const segs = fam.segments;
  const literals = segs.filter((s) => s.class === "literal").length;
  const wildcard = segs[segs.length - 1].class === "wildcard";
  return [literals, wildcard ? 0 : 1, segs.length];
}

function checkSeg(seg, got, rules) {
  if (!got) return rules.tokens.case_malformed;
  if (seg.segment === "{version}") {
    return rules.version.test(got) ? null : rules.tokens.case_malformed;
  }
  const cls = seg.class;
  const name = seg.segment.replace(/^\{|\}$/g, "");
  if (cls === "vocab") {
    if (!rules.vocab.test(got)) return rules.tokens.case_malformed;
    if (seg.values && seg.values.length > 0 && !seg.open && !seg.values.includes(got)) {
      return rules.tokens.vocab_value_unregistered;
    }
    return null;
  }
  if (cls === "hex") {
    const pat = seg.pattern ? new RegExp("^" + seg.pattern + "$") : HEX_PATTERN;
    return pat.test(got) ? null : rules.tokens.case_malformed;
  }
  if (cls === "external") {
    const pat = seg.pattern ? new RegExp("^" + seg.pattern + "$") : rules.external[name];
    return (!pat || pat.test(got)) ? null : rules.tokens.case_malformed;
  }
  if (seg.pattern && !new RegExp("^" + seg.pattern + "$").test(got)) {
    return rules.tokens.case_malformed;
  }
  return null;
}

function matchSegments(fam, parts, rules) {
  const segs = fam.segments;
  const wildcard = segs[segs.length - 1].class === "wildcard";
  const fixed = wildcard ? segs.slice(0, -1) : segs;
  const multiIdx = fixed.findIndex((s) => s.multi);
  let extra = 0;
  if (wildcard) {
    if (parts.length < fixed.length + 1) return [false, {}, null];
  } else if (multiIdx !== -1) {
    extra = parts.length - fixed.length;
    if (extra < 0) return [false, {}, null];
  } else if (parts.length !== fixed.length) {
    return [false, {}, null];
  }
  const binds = {};
  let refusal = null;
  let pi = 0;
  for (let i = 0; i < fixed.length; i++) {
    const seg = fixed[i];
    if (multiIdx === i) {
      const span = parts.slice(pi, pi + 1 + extra);
      pi += 1 + extra;
      for (const got of span) {
        refusal = refusal || checkSeg(seg, got, rules);
      }
      binds[seg.segment.replace(/^\{|\}$/g, "")] = span.join(":");
      continue;
    }
    const got = parts[pi++];
    if (seg.class === "literal") {
      if (seg.segment !== got) return [false, {}, null];
      continue;
    }
    refusal = refusal || checkSeg(seg, got, rules);
    binds[seg.segment.replace(/^\{|\}$/g, "")] = got;
  }
  const tail = parts.slice(pi);
  for (let k = 0; k < tail.length; k++) {
    const got = tail[k];
    const last = pi + k === parts.length - 1;
    if (!got || !rules.vocab.test(got) || VERSION_LIKE.test(got) || (last && VERSION_ATTEMPT.test(got))) {
      refusal = refusal || rules.tokens.case_malformed;
    }
  }
  if (tail.length > 0) binds["*"] = tail.join(":");
  return [true, binds, refusal];
}

function mutatedTail(rules, seg) {
  return Boolean((VERSION_LIKE.test(seg) && !rules.version.test(seg)) || VERSION_ATTEMPT.test(seg));
}

function resolve(rules, parts) {
  const versioned = parts.length > 0 && rules.version.test(parts[parts.length - 1]);
  const candidates = [];
  for (const fam of rules.families) {
    const ends_version = fam.segments[fam.segments.length - 1].segment === "{version}";
    if (ends_version && !versioned) {
      const headless = { ...fam, segments: fam.segments.slice(0, -1) };
      let [ok, binds, ref] = matchSegments(headless, parts, rules);
      if (ok) candidates.push({ sc: score(fam), prefix: fam.prefix, binds, ref, has_version: false, index: fam.index });
      let [ok2, binds2, ref2] = matchSegments(fam, parts, rules);
      if (ok2) candidates.push({ sc: score(fam), prefix: fam.prefix, binds: binds2, ref: ref2, has_version: true, index: fam.index });
    } else if (ends_version || !versioned) {
      let [ok, binds, ref] = matchSegments(fam, parts, rules);
      if (ok) {
        if (!versioned && mutatedTail(rules, parts[parts.length - 1])) {
          ref = ref || rules.tokens.case_malformed;
        }
        candidates.push({ sc: score(fam), prefix: fam.prefix, binds, ref, has_version: ends_version, index: fam.index });
      }
    } else if (parts.length > 1) {
      let [ok, binds, ref] = matchSegments(fam, parts.slice(0, -1), rules);
      if (ok) candidates.push({ sc: score(fam), prefix: fam.prefix, binds, ref, has_version: true, index: fam.index });
    }
  }
  if (candidates.length === 0 && versioned) {
    for (const fam of rules.families) {
      if (fam.segments[fam.segments.length - 1].segment === "{version}") continue;
      let [ok, binds, ref] = matchSegments(fam, parts, rules);
      if (ok) candidates.push({ sc: score(fam), prefix: fam.prefix, binds, ref, has_version: false, index: fam.index });
    }
  }
  if (candidates.length === 0) return null;
  candidates.sort((a, b) => {
    if (a.sc[0] !== b.sc[0]) return b.sc[0] - a.sc[0];
    if (a.sc[1] !== b.sc[1]) return b.sc[1] - a.sc[1];
    if (a.sc[2] !== b.sc[2]) return b.sc[2] - a.sc[2];
    return a.index - b.index;
  });
  return candidates[0];
}

function detectMalformed(rules, parts, cleanOnly = false) {
  const variants = [];
  const low = parts.map((p) => p.toLowerCase());
  if (low.some((p, i) => p !== parts[i])) variants.push(low);
  const n = parts.length;
  if (n > 1 && (VERSION_LIKE.test(parts[n - 1]) || VERSION_ATTEMPT.test(parts[n - 1]))) {
    variants.push(parts.slice(0, -1));
    variants.push(low.slice(0, -1));
    if (n > 2 && VERSION_LIKE.test(parts[n - 2])) {
      variants.push(parts.slice(0, -2));
      variants.push(low.slice(0, -2));
    }
  }
  for (const v of variants) {
    const hit = resolve(rules, v);
    if (hit && (!hit.ref || !cleanOnly)) return hit.prefix;
  }
  return null;
}

export function matchFamily(wireNamespace, tables = null) {
  if (!wireNamespace) return { family: null, binds: {}, refusal: "namespace_dimension_case_malformed", ok: false };
  
  // Fast path 1: explicit aliases from csd_namespace_map.json
  const aliases = tables?.aliases || defaultAliases || {};
  let target = aliases[wireNamespace] || wireNamespace;
  
  // Fast path 2: direct exact lookup in transitions table for template families
  if ((target.includes("{") || target.endsWith("*")) && tables?.families?.has(target)) {
    return { family: target, binds: {}, refusal: null, ok: true };
  }

  // Full segment-by-segment grammar matching
  const rules = tables?.rules || (tables?.registry ? new Rules(tables.registry) : (defaultRegistry ? new Rules(defaultRegistry) : null));
  if (!rules) {
    return { family: target, binds: {}, refusal: null, ok: true };
  }

  if (target.trim() !== target) {
    return { family: null, binds: {}, refusal: rules.tokens.case_malformed, ok: false };
  }
  if (target.startsWith(rules.private)) {
    return { family: null, binds: {}, refusal: rules.tokens.private_use_not_federatable, ok: false };
  }
  const parts = target.split(":");
  if (parts.some((p) => p === "")) {
    return { family: null, binds: {}, refusal: rules.tokens.case_malformed, ok: false };
  }
  const hit = resolve(rules, parts);
  if (!hit) {
    const mutated = detectMalformed(rules, parts, false);
    if (mutated) return { family: mutated, binds: {}, refusal: rules.tokens.case_malformed, ok: false };
    const fenced = [...rules.reserved_stems, ...rules.closed_stems];
    if (fenced.some((stem) => target.startsWith(stem))) {
      return { family: null, binds: {}, refusal: rules.tokens.family_unregistered, ok: false };
    }
    const low = target.toLowerCase();
    if (low !== target && fenced.some((stem) => low.startsWith(stem))) {
      return { family: null, binds: {}, refusal: rules.tokens.case_malformed, ok: false };
    }
    if (rules.version_required && !rules.version.test(parts[parts.length - 1])) {
      if (mutatedTail(rules, parts[parts.length - 1])) {
        return { family: null, binds: {}, refusal: rules.tokens.case_malformed, ok: false };
      }
      return { family: null, binds: {}, refusal: rules.tokens.missing_version_segment, ok: false };
    }
    if (parts.length >= 2 && VERSION_LIKE.test(parts[parts.length - 2])) {
      return { family: null, binds: {}, refusal: rules.tokens.case_malformed, ok: false };
    }
    if (parts.length < 2) {
      return { family: null, binds: {}, refusal: rules.tokens.case_malformed, ok: false };
    }
    return { family: null, binds: {}, refusal: null, ok: true };
  }
  const { prefix, binds, ref, has_version } = hit;
  let refusal = ref;
  const fam = rules.familyMap.get(prefix);
  if (refusal) {
    const mutated = detectMalformed(rules, parts, true);
    if (mutated && mutated !== prefix) {
      return { family: mutated, binds: {}, refusal: rules.tokens.case_malformed, ok: false };
    }
    return { family: prefix, binds, refusal, ok: false };
  }
  const versioned = rules.version.test(parts[parts.length - 1]);
  if (fam.leaves_closed && (fam.segments[fam.segments.length - 1].class === "wildcard" || fam.segments[fam.segments.length - 1].class === "vocab")) {
    const leaf_parts = (versioned && has_version) ? parts.slice(0, -1) : parts;
    const ok_leaf = (fam.leaves || []).some((l) => rules.familyMap.has(l) && matchSegments(rules.familyMap.get(l), leaf_parts, rules)[0]);
    if (!ok_leaf) {
      return { family: prefix, binds, refusal: rules.tokens.family_unregistered, ok: false };
    }
  }
  if (rules.version_required && !rules.exempt.has(prefix) && !has_version) {
    return { family: prefix, binds, refusal: rules.tokens.missing_version_segment, ok: false };
  }
  if (has_version) {
    binds.version = parts[parts.length - 1];
  }
  return { family: prefix, binds, refusal: null, ok: true };
}

export function checkPolarity(tables, familyPrefix, score) {
  const meta = tables?.registry?._meta;
  if (!meta) return { admissible: false, class: null, refusal: "namespace_family_unregistered" };
  const pc = meta.polarity_classes;
  if (!pc) return { admissible: true, class: null, refusal: null };
  const fam = (tables?.registry?.families || []).find((f) => f.prefix === familyPrefix);
  if (!fam) {
    const refusal = meta.case_rule?.refusal_tokens?.family_unregistered || "namespace_family_unregistered";
    return { admissible: false, class: null, refusal };
  }
  const rawPolarity = fam.polarity || "";
  const aliases = pc._aliases || {};
  const name = aliases[rawPolarity] || rawPolarity;
  const cls = pc[name];
  const defaultRefusal = pc.refusal || "score_outside_declared_polarity";
  if (!cls) return { admissible: true, class: name, refusal: null };
  if (cls.kind === "per_leaf") return { admissible: true, class: name, refusal: null };
  let ok = false;
  if (cls.kind === "points") {
    ok = (cls.points || []).some((pv) => Math.abs(score - pv) < 1e-9);
  } else {
    const min = cls.min ?? -1.0;
    const max = cls.max ?? 1.0;
    const minInclusive = cls.min_inclusive ?? false;
    const loOk = score > min || (minInclusive && Math.abs(score - min) < 1e-9);
    ok = loOk && score <= max;
  }
  return {
    admissible: ok,
    class: name,
    refusal: ok ? null : defaultRefusal,
  };
}

export function extractDimensions(wireNamespace, tables = null) {
  const m = matchFamily(wireNamespace, tables);
  return m ? m.binds : {};
}

// ---------------------------------------------------------------- tables

export function ruleVotes(rule, base, strict = false) {
  if (!rule || typeof rule !== "object") return null;
  const k = rule.kind;
  if (k === "fraction") {
    const { num, den } = rule;
    if (!den) return null;
    return strict ? Math.floor((num * base) / den) + 1 : Math.ceil((num * base) / den);
  }
  if (k === "one_of_n") return 1;
  if (k === "strict_majority") return Math.floor(base / 2) + 1;
  if (k === "strict_majority_of_others") return Math.floor((base - 1) / 2) + 1;
  if (k === "role_at" || k === "voices") return rule.count;
  return null;
}

export function loadTables(transitions, policies, registry = null, aliases = null) {
  const families = new Map();
  for (const r of transitions.rows) {
    families.set(r.family, r.transitions.map((t) => ({
      verb: t.verb, from: t.from, to: t.to,
      roles: t.roles.value || [],
      policy: t.policy.value.policy || "selfAct",
      policyRows: t.policy.value.rows || [],
      widens: !!t.widens.value,
    })));
  }
  const policy = new Map();
  for (const p of policies.families) policy.set(p.family, p);
  const defaults = { two_path: "default_standing", mirror: "moderation_action", cosigned: "custodian_replace", ranked: "moderator_election" };
  
  const reg = registry || defaultRegistry;
  const aliasMap = aliases || defaultAliases || {};
  const rules = reg ? new Rules(reg) : null;
  
  const tbl = {
    families,
    policy,
    defaults,
    registry: reg,
    aliases: aliasMap,
    rules,
    min_count: policies.min_count || { community: 2, self: 1 },
    minCount: policies.min_count || { community: 2, self: 1 },
  };
  tbl.matchFamily = (wire) => matchFamily(wire, tbl);
  tbl.extractDimensions = (wire) => extractDimensions(wire, tbl);
  tbl.checkPolarity = (familyPrefix, score) => checkPolarity(tbl, familyPrefix, score);
  return tbl;
}

export function transitionOf(tables, family, verb, state) {
  let famKey = family;
  if (!tables.families.has(famKey)) {
    const m = tables.matchFamily ? tables.matchFamily(family) : matchFamily(family, tables);
    if (m && m.family && tables.families.has(m.family)) {
      famKey = m.family;
    }
  }
  const ts = tables.families.get(famKey);
  if (!ts) return null;
  return ts.find((t) => t.verb === verb && t.from === state) || null;
}

export function policyRow(tables, t) {
  if (t.policy === "selfAct" || t.policy === "none") return null;
  const name = t.policyRows[0] || tables.defaults[t.policy];
  return tables.policy.get(name) || null;
}

// ---------------------------------------------------------------- the fold
//
// held: iterable of {row, firstHeld}
// row:  {id, family, verb, signer, object, scope:{tier:'self'|'room'|'federation', room?}, subject?,
//        stance?: 'for'|'against', ranking?: [person...], asserted_at, note?}
//
// The fold orders rows by their signed instants (deterministic on every node); every comparison
// with `now` uses a first-held instant (this node's).

const byInstant = (a, b) => (a.row.asserted_at - b.row.asserted_at) || (a.row.id < b.row.id ? -1 : a.row.id > b.row.id ? 1 : 0);

export function fold(held, tables, now) {
  const rows = [...held].sort(byInstant);
  const objects = new Map();
  const registered = new Set();
  const rooms = new Map();        // room id -> {founders, members, moderators, votes}
  const holders = new Set();      // the trust root's holders
  const nodesOf = new Map();      // person -> Set(node id)
  const duties = [];              // {from, to, scope, room}
  const halt = { latched: false, by: null, since: null, confirmed: false };
  const rolesOf = new Map();      // person -> Set(role name)
  const systemIdentities = new Set();
  const ownersOf = new Map();     // target -> Set(person)
  const attesters = new Set();    // persons with attest capability

  const addRole = (person, role) => {
    if (!person || !role) return;
    if (!rolesOf.has(person)) rolesOf.set(person, new Set());
    rolesOf.get(person).add(role);
  };

  const room = (id) => { if (!rooms.has(id)) rooms.set(id, { id, founders: new Set(), members: new Set(), removed: new Set(), moderators: new Set() }); return rooms.get(id); };

  const deviceToFedId = new Map();
  for (const h of rows) {
    if (h.row.family.startsWith("self:delegates_to") || h.row.family.startsWith("self:roster")) {
      if (h.row.subject) deviceToFedId.set(h.row.signer, h.row.subject);
    }
  }
  const getFed = (signer) => deviceToFedId.get(signer) || signer;

  for (const h of rows) {
    const { row } = h;
    let famKey = row.family;
    let binds = {};
    if (!tables.families.has(famKey)) {
      const m = tables.matchFamily ? tables.matchFamily(row.family) : matchFamily(row.family, tables);
      if (m?.family) { famKey = m.family; binds = m.binds || {}; }
    }

    let o = objects.get(row.object);
    if (!o) {
      o = { id: row.object, family: famKey, rawFamily: row.family, binds, state: "none", rows: [], scope: row.scope || { tier: "self" }, author: row.signer, subject: row.subject || null, pending: null, outcome: null };
      objects.set(o.id, o);
    }
    o.rows.push(h);
    const isVote = Boolean(row.stance || row.ranking || row.verb === "voice" || row.verb === "assent" || row.verb === "raised" || row.verb === "object");
    let t = transitionOf(tables, famKey, row.verb, o.state);
    if (o.state === "none" && isVote) {
      t = null;
    }
    if (!t) {
      // a vote-shaped row on an object already admitted: counted by the policy fold below
      continue;
    }
    const p = policyRow(tables, t);
    if (!p) { o.state = t.to; o.last = h; }
    else { o.state = t.to; o.last = h; o.pending = { policy: p, transition: t, since: h.firstHeld, opener: row.signer }; }
  }

  // policy folds: the voices on an object decide its move
  for (const o of objects.values()) {
    if (!o.pending) continue;
    const p = o.pending.policy;
    const opener = o.pending.opener;
    const openerFed = getFed(opener);

    const personVotes = new Map();
    personVotes.set(openerFed, "for");
    const ballots = [];

    for (const { row } of o.rows) {
      const fed = getFed(row.signer);
      const stateDim = row.family.startsWith("objection:") ? row.family.slice("objection:".length) : "";
      const isApproval = row.stance === "for"
        || (row.family.startsWith("objection") && (row.verb === "assent" || stateDim === "assent" || stateDim === "proposal"));
      const isObjection = row.stance === "against"
        || row.family === "objection"
        || (row.family.startsWith("objection") && (row.verb === "raised" || row.verb === "object" || stateDim === "raised"));

      if (row.verb === "withdraw" || row.verb === "recant") {
        personVotes.delete(fed);
      } else if (isApproval) {
        personVotes.set(fed, "for");
      } else if (isObjection) {
        personVotes.set(fed, "against");
      }
      if (row.ranking) ballots.push({ signer: row.signer, ranking: row.ranking });
    }

    const fedFor = new Set();
    const fedAgainst = new Set();
    for (const [fed, v] of personVotes) {
      if (v === "for") fedFor.add(fed);
      else if (v === "against") fedAgainst.add(fed);
    }

    const forCount = fedFor.size;
    const againstCount = fedAgainst.size;

    // Base law: compute base from policy domain
    let base = 2;
    if (p.corridor === "self" || p.family?.startsWith("self_roster")) {
      let count = 0;
      for (const [d, fId] of deviceToFedId) {
        if (fId === openerFed) count++;
      }
      base = Math.max(1, count);
    } else if (o.scope?.room && rooms.has(o.scope.room)) {
      const rm = rooms.get(o.scope.room);
      const rawB = Math.max(2, rm.members.size + rm.founders.size);
      base = p.base === "active" ? Math.max(2, Math.floor((rawB * 2) / 3)) : rawB;
    } else if (p.family === "accord_halt") {
      base = Math.max(1, holders.size);
    }

    const minCount = (tables.min_count && p.corridor && tables.min_count[p.corridor])
      || (tables.minCount && p.corridor && tables.minCount[p.corridor])
      || (p.corridor === "self" || p.family?.startsWith("self_roster") ? 1 : 2);
    const cancelBase = p.cancel_over === "others" ? Math.max(0, base - 1) : base;
    const windowSecs = p.window_secs || 0;
    const windowEnd = o.pending.since + windowSecs * 1000;

    const parseFrac = (f) => {
      if (!f) return null;
      if (typeof f === "object" && f.num !== undefined && f.den !== undefined) return f;
      if (typeof f === "string") {
        if (f === "1/N fires") return { num: 1, den: 1 };
        if (f === "1/2 confirms") return { num: 1, den: 2 };
        const parts = f.split("/");
        if (parts.length === 2) {
          const num = parseInt(parts[0], 10);
          const den = parseInt(parts[1], 10);
          if (!isNaN(num) && !isNaN(den)) return { num, den };
        }
      }
      return null;
    };

    const enactFrac = parseFrac(p.enact);
    const cancelFrac = parseFrac(p.cancel);
    const floorFrac = parseFrac(p.floor);

    // Thresholds from typed rules (falling back to fractions)
    const ev = (p.enact_rule ? ruleVotes(p.enact_rule, base, true) : null)
      ?? (enactFrac ? Math.floor((enactFrac.num * base) / enactFrac.den) + 1 : 1);
    const cvRaw = p.cancel_rule ? ruleVotes(p.cancel_rule, cancelBase, false) : (cancelFrac ? Math.ceil((cancelFrac.num * cancelBase) / cancelFrac.den) : null);
    const cv = cvRaw !== null ? Math.max(minCount, cvRaw) : null;
    const fvRaw = p.floor_rule ? ruleVotes(p.floor_rule, base, false) : (floorFrac ? Math.ceil((floorFrac.num * base) / floorFrac.den) : null);
    const fv = fvRaw !== null ? Math.max(minCount, fvRaw) : null;

    let outcome = null, missing = null;
    if (p.kind === "two_path") {
      if (forCount >= ev && !p.irreversible) {
        outcome = "enacted";
      } else if (now < windowEnd) {
        outcome = null;
        missing = { voices: ev - forCount, until: windowEnd };
      } else if (cv !== null && againstCount >= cv) {
        outcome = "cancelled";
      } else if (forCount >= ev) {
        outcome = p.irreversible ? "enacted_irreversible" : "enacted";
      } else if (p.reverse_path && fv !== null && forCount >= fv && againstCount === 0) {
        outcome = "enacted";
      } else {
        if (p.at_window === "stay_open") {
          outcome = null;
          missing = { voices: ev - forCount };
        } else {
          outcome = "cancelled";
        }
      }
    } else if (p.kind === "mirror") {
      if (cv !== null && againstCount >= cv) {
        outcome = "reversed";
      } else if (now >= windowEnd) {
        if (p.at_window === "lapsed") {
          outcome = (forCount * 2 > base) ? "stands" : "lapsed";
        } else {
          outcome = "stands";
        }
      } else {
        outcome = "acting";
        missing = { until: windowEnd };
      }
    } else if (p.kind === "cosigned") {
      const required = (p.enact_rule ? ruleVotes(p.enact_rule, base, true) : null)
        ?? (enactFrac ? Math.floor((enactFrac.num * base) / enactFrac.den) + 1 : 2);
      if (forCount >= required) outcome = "enacted";
      else missing = { voices: required - forCount };
    } else if (p.kind === "ranked") {
      if (now >= windowEnd) {
        const minBallots = fv ?? 2;
        if (ballots.length >= minBallots) {
          const seated = irv(ballots);
          if (seated) outcome = { seated };
          else outcome = p.at_window === "reopens" ? "reopens" : null;
        } else {
          outcome = p.at_window === "reopens" ? "reopens" : null;
        }
      } else {
        missing = { until: windowEnd, ballots: ballots.length };
      }
    }
    o.outcome = outcome; o.missing = missing; o.voices = { for: [...fedFor].sort(), against: [...fedAgainst].sort() };
  }

  // standing folds across families
  for (const o of objects.values()) {
    const { family, state } = o;
    const live = state === "admitted";
    if (family === "key:record" && live) registered.add(o.author);
    if (family === "transport:{kind}" && live) { if (!nodesOf.has(o.author)) nodesOf.set(o.author, new Set()); nodesOf.get(o.author).add(o.node || o.id); }
    if (family === "trust:{job}:{version}" && live) {
      holders.add(o.author);
      for (const { row } of o.rows) if (row.stance === "for") holders.add(row.signer);
      let job = o.binds?.job;
      if (!job || job.includes("{")) {
        const parts = (o.id || "").split(":");
        if (parts.length >= 3) {
          job = parts.slice(1, parts.length - 1).join(":");
        } else if (parts.length === 2) {
          job = parts[1];
        }
      }
      if (!job || job.includes("{")) {
        const p1 = (o.rawFamily || "").split(":")[1];
        if (p1 && !p1.includes("{")) job = p1;
      }
      if (!job || job.includes("{")) {
        const p2 = (o.id || "").split(":")[1];
        if (p2 && !p2.includes("{")) job = p2;
      }
      if (!job || job.includes("{")) {
        job = o.rows?.[0]?.row?.role || o.rows?.[0]?.row?.job;
      }
      if (job) {
        addRole(o.author, job);
        if (job === "attest") attesters.add(o.author);
      }
    }
    if ((family.startsWith("role:") || family === "role:{job}") && live) {
      const job = o.binds?.job || o.rawFamily.split(":")[1] || o.id.split(":")[1];
      if (job) addRole(o.author, job);
    }
    if (family === "ownership:{relation}:{target_kind}:{version}" && live) {
      const target = o.subject || o.id;
      if (!ownersOf.has(target)) ownersOf.set(target, new Set());
      ownersOf.get(target).add(o.author);
    }
    if (family === "group:roster" && live) { const r = room(o.scope.room || o.id); r.founders.add(o.author); }
    if (family === "membership:{stage}" && o.subject) {
      const r = room(o.scope.room);
      if (live && o.outcome === "enacted") r.members.add(o.subject);
      if (state === "withdrawn") { r.members.delete(o.subject); r.removed.add(o.subject); }
    }
    if ((family === "delegates_to" || family === "self:delegates_to") && live) {
      if (o.dimension === "moderate" || o.scope?.tier === "room") duties.push({ from: o.author, to: o.subject, scope: "moderate", room: o.scope.room });
    }
    if (family === "accord:invoke:constitutional:{halt_id}" && live) { halt.latched = true; halt.by = o.author; halt.since = o.last.firstHeld; halt.confirmed = o.voices && o.voices.for.length * 2 > Math.max(holders.size, 1); }
    if (family === "ballot:moderator" && o.outcome && o.outcome.seated) { room(o.scope.room).moderators.add(o.outcome.seated); }
    if (family === "system:*" && live) {
      systemIdentities.add(o.author);
    }
  }
  for (const r of rooms.values()) {
    for (const f of r.founders) r.moderators.add(f);           // a founder keeps the duty
    for (const d of duties) if (d.room === r.id) r.moderators.add(d.to);
    r.locked = r.moderators.size === 0;
  }
  const haltPolicy = tables?.policy?.get("accord_halt");
  const haltDuration = ((haltPolicy && haltPolicy.window_secs) ? haltPolicy.window_secs : 86400) * 1000;
  if (halt.latched && !halt.confirmed && now - halt.since > haltDuration) halt.latched = false;

  return { objects, registered, rooms, holders, nodesOf, halt, rolesOf, systemIdentities, ownersOf, attesters, now };
}

export function irv(ballots) {
  if (!ballots.length) return null;
  const seen = new Map(); for (const b of ballots) seen.set(b.signer, b.ranking); // one ballot per person, the last held
  let alive = new Set([].concat(...[...seen.values()]));
  while (alive.size > 1) {
    const tally = new Map([...alive].map((c) => [c, 0]));
    for (const r of seen.values()) { const first = r.find((c) => alive.has(c)); if (first) tally.set(first, tally.get(first) + 1); }
    const total = [...tally.values()].reduce((a, b) => a + b, 0);
    for (const [c, n] of tally) if (n * 2 > total) return c;
    const min = Math.min(...tally.values());
    const losers = [...tally].filter(([, n]) => n === min).map(([c]) => c).sort();
    alive.delete(losers[0]);
  }
  return [...alive][0] || null;
}

// ---------------------------------------------------------------- standing

export function standing(f, person, object) {
  const roles = new Set();
  if (f.registered.has(person)) roles.add("registered");
  if (f.holders.has(person)) roles.add("holder");
  const o = object ? (f.objects.get(object.id || object) || (typeof object === "object" ? object : null)) : null;
  const roomId = o ? (o.scope?.room || o.room) : (object && object.room);
  if (roomId && f.rooms.has(roomId)) {
    const r = f.rooms.get(roomId);
    if (r.founders.has(person)) roles.add("founder");
    if (r.founders.has(person) || r.members.has(person)) roles.add("member");
    if (r.moderators.has(person)) roles.add("moderate");
  }
  if (o) {
    if (o.author === person || o.signer === person) { roles.add("author"); roles.add("self"); }
    if (o.subject === person) roles.add("subject");
  } else {
    roles.add("self");
  }

  // --- Specialized Roles (Deriving all 17 constitutional role tokens) ---
  const subj = (o && o.subject) || (object && object.subject) || null;
  const targetId = (o && o.id) || (object && (object.id || object.object)) || null;
  const isTargetOwner = (targetId && f.ownersOf?.has(targetId) && f.ownersOf.get(targetId).has(person)) ||
                        (subj && f.ownersOf?.has(subj) && f.ownersOf.get(subj).has(person));
  const isRoomFounder = roomId && f.rooms.has(roomId) && f.rooms.get(roomId).founders.has(person);
  if (person === subj || isTargetOwner || isRoomFounder || (o && (o.author === person || o.signer === person))) {
    roles.add("owner");
  }

  // attest role
  if (f.attesters?.has(person) || f.rolesOf?.get(person)?.has("attest") || f.holders.has(person)) {
    roles.add("attest");
  }

  // group:roster founding bootstrap:
  // When target/object is group:roster or starts with room: and state is none:
  // registered person has founder + member standing.
  const isGroupRoster = (o && o.family === "group:roster") ||
                        (targetId && targetId.startsWith("room:")) ||
                        (object && (object.family === "group:roster" || (object.id && object.id.startsWith("room:"))));
  if (isGroupRoster && f.registered.has(person)) {
    roles.add("founder");
    roles.add("member");
  }

  // Dynamic roles authorized to this person via trust or role declarations
  const personRoles = f.rolesOf?.get(person);
  if (personRoles) {
    for (const r of personRoles) roles.add(r);
  }

  return roles;
}

export function enabled(f, tables, person, target, familyFilter = null) {
  // 1. Halt Gate:
  const isAgent = (target?.signerType === "agent" || target?.signer_type === "agent")
    || f.rolesOf?.get(person)?.has("agent")
    || [...f.objects.values()].some(obj => obj.family.startsWith("agent:charter") && obj.subject === person && obj.state === "admitted");

  if (f.halt.latched && isAgent) {
    return [];
  }

  // 2. Registration Gate:
  const isReg = f.registered.has(person);
  const targetId = target?.id || target?.object || null;
  const o = targetId ? f.objects.get(targetId) : null;
  const out = [];

  if (o) {
    if (familyFilter) {
      const m = tables.matchFamily ? tables.matchFamily(familyFilter) : matchFamily(familyFilter, tables);
      const famKey = m?.family || familyFilter;
      if (o.family !== famKey) return [];
    }

    const isRoomAudience = o.scope?.tier === "room" || target?.tier === "room";
    if (isRoomAudience) {
      const roomId = o.scope?.room || target?.room;
      if (roomId) {
        const r = f.rooms.get(roomId);
        if (!r && o.family !== "group:roster") return [];
        if (r) {
          const inRoom = r.members.has(person) || r.founders.has(person);
          const invitee = o.family === "membership:{stage}" && o.subject === person;
          if (!inRoom && !invitee) return [];
          if (target?.subject) {
            const subjInRoom = r.members.has(target.subject) || r.founders.has(target.subject);
            if (!subjInRoom && o.family !== "membership:{stage}") return [];
          }
        }
      }
    }

    const roles = standing(f, person, target || o);
    if (!isReg && o.family !== "key:record") return [];

    const ts = tables.families.get(o.family) || [];
    for (const t of ts) {
      if (t.from === o.state && t.roles.length > 0 && t.roles.some(r => roles.has(r))) {
        out.push(t);
      }
    }

    if (o.pending && (roles.has("member") || roles.has("subject") || roles.has("holder"))) {
      const pol = o.pending.policy?.kind || "two_path";
      for (const vVerb of ["voice", "assent", "raised", "object"]) {
        out.push({
          verb: vVerb,
          from: o.state,
          to: o.state,
          roles: ["member", "subject", "holder"],
          policy: pol,
          policyRows: [],
          widens: false,
        });
      }
    }
  } else {
    // Starts (from "none")
    const isRoomAudience = target?.tier === "room";
    if (isRoomAudience) {
      const roomId = target?.room;
      if (roomId) {
        const r = f.rooms.get(roomId);
        const m = familyFilter ? (tables.matchFamily ? tables.matchFamily(familyFilter) : matchFamily(familyFilter, tables)) : null;
        const isRoster = m?.family === "group:roster";
        if (!r && !isRoster) return [];
        if (r) {
          const inRoom = r.members.has(person) || r.founders.has(person);
          const invitee = m?.family === "membership:{stage}" && target?.subject === person;
          if (!inRoom && !invitee && !isRoster) return [];
        }
      }
    }

    const roles = standing(f, person, target);
    let familiesToCheck = [];
    if (familyFilter) {
      const m = tables.matchFamily ? tables.matchFamily(familyFilter) : matchFamily(familyFilter, tables);
      familiesToCheck.push(m?.family || familyFilter);
    } else {
      familiesToCheck = [...tables.families.keys()];
    }

    for (const fam of familiesToCheck) {
      if (!isReg && fam !== "key:record") continue;
      const ts = tables.families.get(fam) || [];
      for (const t of ts) {
        if (t.from === "none" && t.roles.length > 0 && t.roles.some(r => roles.has(r))) {
          out.push(t);
        }
      }
    }

    // Out-of-order votes on uncreated objects (CC CIRISPersist#1198)
    if (isReg && targetId) {
      for (const vVerb of ["voice", "assent", "raised", "object"]) {
        out.push({
          verb: vVerb,
          from: "none",
          to: "none",
          roles: ["member", "subject", "holder"],
          policy: "two_path",
          policyRows: [],
          widens: false,
        });
      }
    }
  }

  return out;
}

export function enabledFor(tables, family, state, roles) {
  let famKey = family;
  if (!tables.families.has(famKey)) {
    const m = tables.matchFamily ? tables.matchFamily(family) : matchFamily(family, tables);
    if (m && m.family && tables.families.has(m.family)) famKey = m.family;
  }
  const ts = tables.families.get(famKey) || [];
  const rs = roles instanceof Set ? roles : new Set(roles);
  return ts.filter((t) => t.from === state && t.roles.length > 0 && (t.roles.some((r) => rs.has(r)) || (t.roles.includes("registered") && rs.size > 0 && rs.has("registered"))));
}

// ---------------------------------------------------------------- the one function

// step: the door before it commits. Returns {ok:true, plan} or {ok:false, refusal}.
export function step(held, tables, row, now) {
  if (held.has(row.id)) return { ok: false, refusal: "already_held" };
  const f = fold(held.values(), tables, now);
  
  // Resolve wire family
  let famKey = row.family;
  if (!tables.families.has(famKey)) {
    const m = tables.matchFamily ? tables.matchFamily(row.family) : matchFamily(row.family, tables);
    if (m?.refusal) return { ok: false, refusal: m.refusal };
    if (m?.family) famKey = m.family;
  }

  const o = f.objects.get(row.object);
  if (o && o.family !== famKey) return { ok: false, refusal: "family_mismatch" };

  const target = {
    id: row.object,
    object: row.object,
    subject: row.subject || null,
    author: row.signer,
    room: row.scope?.room || null,
    signerType: row.signerType || row.signer_type || null,
    tier: row.scope?.tier || "self",
  };

  const isVote = Boolean(row.stance || row.ranking || row.verb === "voice" || row.verb === "assent" || row.verb === "raised" || row.verb === "object");

  const enabledTransitions = enabled(f, tables, row.signer, target, row.family);
  const t = enabledTransitions.find((x) => x.verb === row.verb || (x.verb === "voice" && isVote));

  if (t) {
    return { ok: true, plan: { row, transition: t, firstHeld: now, policy: policyRow(tables, t) } };
  }

  // Refusal determination:
  const isAgent = (row.signerType === "agent" || row.signer_type === "agent")
    || f.rolesOf?.get(row.signer)?.has("agent")
    || [...f.objects.values()].some(obj => obj.family.startsWith("agent:charter") && obj.subject === row.signer && obj.state === "admitted");
  if (f.halt.latched && isAgent) return { ok: false, refusal: "halted" };

  if (o && o.family !== famKey) return { ok: false, refusal: "family_mismatch" };

  const selfKey = (famKey === "key:record" || row.family === "key:record") && row.verb === "attest";
  if (!selfKey && !f.registered.has(row.signer)) return { ok: false, refusal: "unregistered" };

  if (row.scope && row.scope.tier === "room") {
    const r = f.rooms.get(row.scope.room);
    const inRoom = r && (r.members.has(row.signer) || r.founders.has(row.signer));
    const invitee = (famKey === "membership:{stage}" || row.family === "membership:{stage}") && row.subject === row.signer;
    if (!r && famKey !== "group:roster" && row.family !== "group:roster") return { ok: false, refusal: "no_such_room" };
    if (r && !inRoom && !invitee) return { ok: false, refusal: "subject_outside_audience" };
    if (row.subject && r && !(r.members.has(row.subject) || r.founders.has(row.subject)) && famKey !== "membership:{stage}" && row.family !== "membership:{stage}") return { ok: false, refusal: "subject_outside_audience" };
  }

  const rawT = transitionOf(tables, famKey, row.verb, o ? o.state : "none");
  if (!rawT) return { ok: false, refusal: "no_transition" };
  if (rawT.roles.length === 0) return { ok: false, refusal: "closed" };

  return { ok: false, refusal: "no_standing" };
}

export function commit(held, plan) { held.set(plan.row.id, { row: plan.row, firstHeld: plan.firstHeld }); return plan; }

// door = step then commit
export function door(held, tables, row, now) { const s = step(held, tables, row, now); if (s.ok) commit(held, s.plan); return s; }

// admit = alias of door
export function admit(held, tables, row, now) { return door(held, tables, row, now); }

// queue = the door run forwards: every transition enabled for this person, one item each
export function queue(held, tables, person, now) {
  const f = fold(held.values(), tables, now);
  const items = [];
  for (const o of f.objects.values()) {
    const roles = standing(f, person, o);
    for (const t of enabledFor(tables, o.family, o.state, roles)) {
      items.push({ kind: "choice", object: o.id, family: o.family, verb: t.verb, to: t.to, policy: t.policy, confirm: t.widens });
    }
  }
  // what the person may start: a family's first transition, from none
  const starts = [];
  const globalRoles = standing(f, person, null);
  for (const [family, ts] of tables.families) {
    for (const t of ts) if (t.from === "none" && t.roles.length && t.roles.some((r) => globalRoles.has(r))) starts.push({ kind: "choice", family, verb: t.verb, to: t.to, policy: t.policy, confirm: t.widens });
  }
  if (globalRoles.has("registered")) { const t = transitionOf(tables, "group:roster", "attest", "none"); if (t) starts.push({ kind: "choice", family: "group:roster", verb: "attest", to: t.to, policy: t.policy, confirm: t.widens, found: true }); }
  for (const r of f.rooms.values()) {
    const roles = standing(f, person, { room: r.id });
    for (const [family, ts] of tables.families) {
      if (!roles.has("member")) continue; // a room offers its starts to its members
      for (const t of ts) if (t.from === "none" && t.roles.length && t.roles.some((x) => roles.has(x))) starts.push({ kind: "choice", family, verb: t.verb, to: t.to, policy: t.policy, confirm: t.widens, room: r.id });
    }
  }
  return { items, starts, fold: f };
}

// preview = the same step over a copy of the held set; impact is the difference of the folds
export function preview(held, tables, rows, now) {
  const before = fold(held.values(), tables, now);
  const copy = new Map(held);
  const verdicts = rows.map((row) => { const s = step(copy, tables, row, now); if (s.ok) commit(copy, s.plan); return { row: row.id, ...s }; });
  const after = fold(copy.values(), tables, now);
  const changed = [];
  for (const [id, o] of after.objects) {
    const b = before.objects.get(id);
    if (!b) changed.push({ object: id, family: o.family, from: "none", to: o.state, pending: o.missing || null });
    else if (b.state !== o.state || JSON.stringify(b.outcome) !== JSON.stringify(o.outcome)) changed.push({ object: id, family: o.family, from: b.state, to: o.state, outcome: o.outcome, pending: o.missing || null });
  }
  const rooms = [];
  for (const [id, r] of after.rooms) {
    const b = before.rooms.get(id);
    const diff = { room: id, joined: [...r.members].filter((m) => !b || !b.members.has(m)), left: b ? [...b.members].filter((m) => !r.members.has(m)) : [], locked: r.locked && !(b && b.locked) };
    if (diff.joined.length || diff.left.length || diff.locked) rooms.push(diff);
  }
  const widens = verdicts.some((v) => v.ok && v.plan.transition.widens);
  const audience = rows.map((r) => (r.scope && r.scope.tier) || "self");
  return { verdicts, changed, rooms, impact: { direction: widens ? "widens" : "same", audience: [...new Set(audience)], undo: verdicts.filter((v) => v.ok).map((v) => !!transitionOf(tables, v.plan.row.family, "withdraw", v.plan.transition.to)) } };
}

// ---------------------------------------------------------------- the mesh: nodes as cells, rows as signals
//
// Each node applies the same fold to what it holds. A row travels one hop per tick along a link,
// and only to a node whose owner is in the row's audience as the SENDER folds it; a stranger carries
// only the consent-free set (keys, bindings, requests, the halt).

const CONSENT_FREE = new Set(["key:record", "transport:{kind}", "consent:{kind}", "accord:invoke:constitutional:{halt_id}", "trust:{job}:{version}"]);

export class Mesh {
  constructor(tables) { this.tables = tables; this.nodes = new Map(); this.links = new Set(); this.now = 0; this.log = []; }
  // a serve seat (kind "server") is a carrier that has said yes: it carries scoped rows sealed, never folding them
  addNode(id, owner, kind = "phone") { this.nodes.set(id, { id, owner, kind, held: new Map(), sealed: new Map(), offered: new Set(), carrier: kind === "server" }); return this.nodes.get(id); }
  link(a, b) { this.links.add([a, b].sort().join("|")); }
  neighbours(id) { const out = []; for (const l of this.links) { const [a, b] = l.split("|"); if (a === id) out.push(b); if (b === id) out.push(a); } return out; }
  audience(node, row) {
    const f = fold(node.held.values(), this.tables, this.now);
    const tier = (row.scope && row.scope.tier) || "self";
    const m = this.tables.matchFamily ? this.tables.matchFamily(row.family) : matchFamily(row.family, this.tables);
    const famKey = m?.family || row.family;
    if (tier === "federation" || CONSENT_FREE.has(famKey) || CONSENT_FREE.has(row.family)) return null; // everyone may carry it
    if (tier === "self") return new Set([row.signer]);
    const r = f.rooms.get(row.scope.room);
    const who = new Set(r ? [...r.members, ...r.founders] : []);
    if (row.subject) who.add(row.subject);
    // the subject of a live proposal sees the room's rows: a first join sees a fresh roster
    for (const o of f.objects.values()) if (o.family === "membership:{stage}" && o.scope.room === row.scope.room && o.state === "admitted" && o.subject) who.add(o.subject);
    return who;
  }
  // a person signs a row at one of their nodes: the door there
  sign(nodeId, row) {
    const node = this.nodes.get(nodeId);
    row.asserted_at = row.asserted_at ?? this.now;
    const s = door(node.held, this.tables, row, this.now);
    this.log.push({ t: this.now, node: nodeId, row: row.id, ...(s.ok ? { admitted: true } : { refused: s.refusal }) });
    return s;
  }
  // one tick: every node offers what it holds to each neighbour in the row's audience; the neighbour's door decides
  tick(dt = 1000) {
    this.now += dt;
    const moves = [];
    for (const node of this.nodes.values()) {
      for (const nb of this.neighbours(node.id)) {
        const other = this.nodes.get(nb);
        const carried = [...[...node.held.values()].map((h) => ({ row: h.row, origin: node })), ...[...node.sealed.values()].map((x) => ({ row: x.row, origin: x.origin }))];
        for (const { row, origin } of carried) {
          const key = row.id + ">" + nb;
          if (node.offered.has(key) || other.held.has(row.id) || other.sealed.has(row.id)) continue;
          const aud = this.audience(origin, row); // the audience is the sender's fold, as it stands now; a carrier folds nothing
          if (aud && !aud.has(other.owner)) {
            if (!other.carrier || other === origin) continue; // a stranger is offered nothing it must consent to carry
            other.sealed.set(row.id, { row, origin }); node.offered.add(key); // a seat carries it sealed
            moves.push({ from: node.id, to: nb, row: row.id, ok: true, sealed: true });
            continue;
          }
          const s = door(other.held, this.tables, row, this.now);
          if (s.ok) node.offered.add(key); // a refusal is retried next tick: what it lacked may arrive
          moves.push({ from: node.id, to: nb, row: row.id, ok: s.ok, refusal: s.refusal });
        }
      }
    }
    return moves;
  }
  foldOf(nodeId) { return fold(this.nodes.get(nodeId).held.values(), this.tables, this.now); }
  queueOf(nodeId) { const n = this.nodes.get(nodeId); return queue(n.held, this.tables, n.owner, this.now); }
  previewAt(nodeId, rows) { return preview(this.nodes.get(nodeId).held, this.tables, rows, this.now); }
}
