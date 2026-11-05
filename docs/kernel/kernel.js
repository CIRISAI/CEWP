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

// ---------------------------------------------------------------- tables

export function loadTables(transitions, policies) {
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
  return { families, policy, defaults };
}

export function transitionOf(tables, family, verb, state) {
  const ts = tables.families.get(family);
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

  const room = (id) => { if (!rooms.has(id)) rooms.set(id, { id, founders: new Set(), members: new Set(), removed: new Set(), moderators: new Set() }); return rooms.get(id); };

  for (const h of rows) {
    const { row } = h;
    let o = objects.get(row.object);
    if (!o) {
      o = { id: row.object, family: row.family, state: "none", rows: [], scope: row.scope || { tier: "self" }, author: row.signer, subject: row.subject || null, pending: null, outcome: null };
      objects.set(o.id, o);
    }
    o.rows.push(h);
    const t = transitionOf(tables, row.family, row.verb, o.state);
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
    const voices = { for: new Set([o.pending.opener]), against: new Set(), ballots: [] };
    for (const { row } of o.rows) {
      if (row.stance === "for") voices.for.add(row.signer);
      if (row.stance === "against") { voices.against.add(row.signer); voices.for.delete(row.signer); }
      if (row.ranking) voices.ballots.push({ signer: row.signer, ranking: row.ranking });
    }
    const roomId = o.scope.room;
    const base = roomId && rooms.has(roomId) ? Math.max(2, rooms.get(roomId).members.size + rooms.get(roomId).founders.size) : 2;
    const windowEnd = o.pending.since + (p.window_secs || 0) * 1000;
    const met = (frac, n) => frac && n * frac.den >= frac.num * base;
    let outcome = null, missing = null;
    if (p.kind === "two_path") {
      if (met(p.enact, voices.for.size)) outcome = "enacted";
      else if (met(p.cancel, voices.against.size)) outcome = "cancelled";
      else if (now >= windowEnd) outcome = p.at_window || "cancelled";
      else missing = { voices: Math.ceil((p.enact.num * base) / p.enact.den) - voices.for.size, until: windowEnd };
    } else if (p.kind === "mirror") {
      // one acts at once; the window decides whether it stands
      if (met(p.cancel, voices.against.size)) outcome = "reversed";
      else if (now >= windowEnd) outcome = "stands";
      else { outcome = "acting"; missing = { until: windowEnd }; }
    } else if (p.kind === "cosigned") {
      if (met(p.enact || { num: 1, den: 2 }, voices.for.size)) outcome = "enacted";
      else missing = { voices: Math.ceil(((p.enact || { num: 1, den: 2 }).num * base) / (p.enact || { num: 1, den: 2 }).den) - voices.for.size };
    } else if (p.kind === "ranked") {
      if (now >= windowEnd) outcome = { seated: irv(voices.ballots) };
      else missing = { until: windowEnd, ballots: voices.ballots.length };
    }
    o.outcome = outcome; o.missing = missing; o.voices = { for: [...voices.for], against: [...voices.against] };
  }

  // standing folds
  for (const o of objects.values()) {
    const { family, state } = o;
    const live = state === "admitted";
    if (family === "key:record" && live) registered.add(o.author);
    if (family === "transport:{kind}" && live) { if (!nodesOf.has(o.author)) nodesOf.set(o.author, new Set()); nodesOf.get(o.author).add(o.node || o.id); }
    if (family === "trust:{job}:{version}" && live) { holders.add(o.author); for (const { row } of o.rows) if (row.stance === "for") holders.add(row.signer); }
    if (family === "group:roster" && live) { const r = room(o.scope.room || o.id); r.founders.add(o.author); }
    if (family === "membership:{stage}" && o.subject) {
      const r = room(o.scope.room);
      if (live && o.outcome === "enacted") r.members.add(o.subject);
      if (state === "withdrawn") { r.members.delete(o.subject); r.removed.add(o.subject); }
    }
    if (family === "delegates_to" && live && o.dimension === "moderate") duties.push({ from: o.author, to: o.subject, scope: "moderate", room: o.scope.room });
    if (family === "accord:invoke:constitutional:{halt_id}" && live) { halt.latched = true; halt.by = o.author; halt.since = o.last.firstHeld; halt.confirmed = o.voices && o.voices.for.length * 2 > Math.max(holders.size, 1); }
    if (family === "ballot:moderator" && o.outcome && o.outcome.seated) { room(o.scope.room).moderators.add(o.outcome.seated); }
  }
  for (const r of rooms.values()) {
    for (const f of r.founders) r.moderators.add(f);           // a founder keeps the duty
    for (const d of duties) if (d.room === r.id) r.moderators.add(d.to);
    r.locked = r.moderators.size === 0;
  }
  if (halt.latched && !halt.confirmed && now - halt.since > 86400 * 1000) halt.latched = false; // a lone latch lapses after a day

  return { objects, registered, rooms, holders, nodesOf, halt, now };
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
  const o = object ? f.objects.get(object.id || object) : null;
  const roomId = o ? o.scope.room : (object && object.room);
  if (roomId && f.rooms.has(roomId)) {
    const r = f.rooms.get(roomId);
    if (r.founders.has(person)) roles.add("founder");
    if (r.founders.has(person) || r.members.has(person)) roles.add("member");
    if (r.moderators.has(person)) roles.add("moderate");
  }
  if (o) {
    if (o.author === person) { roles.add("author"); roles.add("self"); }
    if (o.subject === person) roles.add("subject");
  } else {
    roles.add("self");
  }
  return roles;
}

export function enabledFor(tables, family, state, roles) {
  const ts = tables.families.get(family) || [];
  const rs = roles instanceof Set ? roles : new Set(roles);
  return ts.filter((t) => t.from === state && t.roles.length > 0 && (t.roles.some((r) => rs.has(r)) || (t.roles.includes("registered") && rs.size > 0 && rs.has("registered"))));
}

// ---------------------------------------------------------------- the one function

// step: the door before it commits. Returns {ok:true, plan} or {ok:false, refusal}.
export function step(held, tables, row, now) {
  if (held.has(row.id)) return { ok: false, refusal: "already_held" };
  const f = fold(held.values(), tables, now);
  const o = f.objects.get(row.object);
  const state = o ? o.state : "none";
  const family = o ? o.family : row.family;
  if (o && o.family !== row.family) return { ok: false, refusal: "family_mismatch" };
  const roles = standing(f, row.signer, o || { room: row.scope && row.scope.room });
  // founding: a roster's first version seats a listed member iff that member signed it, no owner gate
  if (!o && row.family === "group:roster" && row.verb === "attest" && f.registered.has(row.signer)) { roles.add("founder"); roles.add("member"); }
  let t = transitionOf(tables, family, row.verb, state);
  if (!t && o && o.pending && (row.stance || row.ranking)) {
    // a voice on an object under a policy: a row the fold counts, moving nothing by itself
    t = { verb: "voice", from: state, to: state, roles: ["member", "subject", "holder"], policy: o.pending.policy.kind, policyRows: [], widens: false };
  }
  if (!t) return { ok: false, refusal: "no_transition" };
  if (t.roles.length === 0) return { ok: false, refusal: "closed" };
  if (!t.roles.some((r) => roles.has(r))) return { ok: false, refusal: "no_standing" };
  const selfKey = row.family === "key:record" && row.verb === "attest";
  if (!selfKey && !f.registered.has(row.signer)) return { ok: false, refusal: "unregistered" };
  if (row.scope && row.scope.tier === "room") {
    const r = f.rooms.get(row.scope.room);
    const inRoom = r && (r.members.has(row.signer) || r.founders.has(row.signer));
    const invitee = row.family === "membership:{stage}" && row.subject === row.signer;
    if (!r && row.family !== "group:roster") return { ok: false, refusal: "no_such_room" };
    if (r && !inRoom && !invitee) return { ok: false, refusal: "subject_outside_audience" };
    if (row.subject && r && !(r.members.has(row.subject) || r.founders.has(row.subject)) && row.family !== "membership:{stage}") return { ok: false, refusal: "subject_outside_audience" };
  }
  if (f.halt.latched && row.signerType === "agent") return { ok: false, refusal: "halted" };
  return { ok: true, plan: { row, transition: t, firstHeld: now, policy: policyRow(tables, t) } };
}

export function commit(held, plan) { held.set(plan.row.id, { row: plan.row, firstHeld: plan.firstHeld }); return plan; }

// door = step then commit
export function door(held, tables, row, now) { const s = step(held, tables, row, now); if (s.ok) commit(held, s.plan); return s; }

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
  addNode(id, owner, kind = "phone") { this.nodes.set(id, { id, owner, kind, held: new Map(), offered: new Set() }); return this.nodes.get(id); }
  link(a, b) { this.links.add([a, b].sort().join("|")); }
  neighbours(id) { const out = []; for (const l of this.links) { const [a, b] = l.split("|"); if (a === id) out.push(b); if (b === id) out.push(a); } return out; }
  audience(node, row) {
    const f = fold(node.held.values(), this.tables, this.now);
    const tier = (row.scope && row.scope.tier) || "self";
    if (tier === "federation" || CONSENT_FREE.has(row.family)) return null; // everyone may carry it
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
        for (const { row } of node.held.values()) {
          const key = row.id + ">" + nb;
          if (node.offered.has(key) || other.held.has(row.id)) continue;
          const aud = this.audience(node, row);
          if (aud && !aud.has(other.owner)) continue; // a stranger is offered nothing it must consent to carry
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
