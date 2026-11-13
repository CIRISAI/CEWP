// node --test docs/kernel/kernel.test.js
// Parity with the constitution's executable mesh story (manifests/mesh_story_trace.json): the same
// tables, the same standing, the same queue and the same refusals, step by step. Then the mesh.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { loadTables, enabledFor, enabled, Mesh, fold, standing, door, admit, queue, preview, matchFamily, extractDimensions, checkPolarity, ruleVotes } from "./kernel.js";

const here = new URL("./tables/", import.meta.url);
const J = (f) => JSON.parse(readFileSync(new URL(f, here)));
const transitions = J("transitions.json");
const policies = J("governance_policy.json");
const registry = J("namespace_registry.json");
const csdMap = J("csd_namespace_map.json");
const matchVectors = J("namespace_match_vectors.json");
const policyVectors = J("governance_policy_vectors.json");

const tables = loadTables(transitions, policies, registry, csdMap.aliases);
const trace = J("mesh_story_trace.json");

test("the queue is the door run forwards: parity with the constitution's story", () => {
  const states = new Map();
  for (const s of trace.steps) {
    const before = states.get(s.view.object) ?? "none";
    // a view that knows its stage is served from the stage (transitions.json _meta.stages)
    const verbs = enabledFor(tables, s.view.family, before, s.standing, s.view.atoms && s.view.atoms.stage).map((t) => t.verb);
    assert.deepEqual(verbs, s.queue, `step ${s.n}: ${s.note}`);
    assert.equal(!verbs.includes(s.chosen ?? "\u0000"), s.refused, `step ${s.n} refusal`);
    states.set(s.view.object, s.view.state_after);
  }
});

test("a life of a mesh: rows travel one hop per tick, strangers carry only the consent-free set, folds agree", () => {
  const m = new Mesh(tables);
  m.addNode("alice-phone", "alice"); m.addNode("bob-phone", "bob"); m.addNode("carol-phone", "carol"); m.addNode("seat", "nobody", "server");
  m.link("alice-phone", "seat"); m.link("bob-phone", "seat"); m.link("carol-phone", "seat"); // alice reaches bob only through the seat
  let id = 0; const row = (o) => ({ id: "r" + (++id), scope: { tier: "self" }, ...o });
  for (const p of ["alice", "bob", "carol"]) assert.ok(m.sign(p + "-phone", row({ family: "key:record", verb: "attest", signer: p, object: "key:" + p })).ok);
  // a stranger's row is refused at the door until the key is held: no standing
  const early = m.sign("alice-phone", row({ family: "chat:*", verb: "attest", signer: "bob", object: "post:x", scope: { tier: "room", room: "room:1" } }));
  assert.equal(early.ok, false);
  m.tick(); m.tick();
  assert.ok(m.nodes.get("seat").held.size >= 3, "keys reach the seat");
  assert.ok(m.nodes.get("carol-phone").held.has("r1"), "alice's key reaches carol through the seat");
  // bob founds a room; the roster row reaches only members: carol's phone never holds it
  assert.ok(m.sign("bob-phone", row({ family: "group:roster", verb: "attest", signer: "bob", object: "room:1", scope: { tier: "room", room: "room:1" } })).ok);
  m.tick(); m.tick(); m.tick();
  assert.equal(m.nodes.get("carol-phone").held.has("r5"), false, "a stranger is not offered a room row");
  assert.equal(m.nodes.get("carol-phone").sealed.has("r5"), false, "a phone is not a carrier");
  assert.ok(m.nodes.get("seat").sealed.has("r5") && !m.nodes.get("seat").held.has("r5"), "a seat carries the room row sealed and never folds it");
  // bob invites alice; the invitation is two-path; alice's yes is the second voice
  assert.ok(m.sign("bob-phone", row({ family: "membership:{stage}", verb: "attest", signer: "bob", object: "invite:alice", subject: "alice", scope: { tier: "room", room: "room:1" } })).ok);
  m.tick(); m.tick();
  const fA = m.foldOf("alice-phone");
  assert.equal(fA.objects.get("invite:alice").outcome, null, "no single vote admits");
  // the invitee's yes is a voice on the same object: a row the fold counts, admitted through the same door
  assert.ok(m.sign("alice-phone", row({ family: "membership:{stage}", verb: "attest", signer: "alice", object: "invite:alice", subject: "alice", stance: "for", scope: { tier: "room", room: "room:1" } })).ok, "the invitee's acceptance is admitted");
  m.tick(); m.tick();
  for (const n of ["alice-phone", "bob-phone"]) assert.ok(m.foldOf(n).rooms.get("room:1").members.has("alice"), n + " seats alice");
  // the same rows, the same count on both nodes; windows are each node's own
  const a = m.foldOf("alice-phone").objects.get("invite:alice"), b = m.foldOf("bob-phone").objects.get("invite:alice");
  assert.deepEqual(a.voices, b.voices);
  // alice may now post; carol, a stranger, sees nothing on the room
  const qa = m.queueOf("alice-phone");
  assert.ok(qa.starts.some((s) => s.family === "chat:*" && s.room === "room:1"), "alice may start a post in the room");
  assert.equal(standing(m.foldOf("carol-phone"), "carol", { room: "room:1" }).has("member"), false);
  // preview equals execution
  const post = row({ family: "chat:*", verb: "attest", signer: "alice", object: "post:1", scope: { tier: "room", room: "room:1" } });
  const pv = m.previewAt("alice-phone", [post]);
  assert.ok(pv.verdicts[0].ok && pv.changed.some((c) => c.object === "post:1" && c.to === "admitted"));
  assert.ok(m.sign("alice-phone", post).ok);
  assert.equal(m.foldOf("alice-phone").objects.get("post:1").state, "admitted");
});

test("partition & re-merge convergence fuzzing: fold & queue permutation invariance across 100 random merges", () => {
  // Deterministic PRNG (Mulberry32) for reproducible 100-permutation fuzzing
  const makeRng = (seed) => () => {
    let t = (seed += 0x6d2b79f5);
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
  const rng = makeRng(0x5eedcafe);

  // Build pre-partition shared base state via valid door() admissions so all actors and room:1 exist
  const baseHeld = new Map();
  let clock = 100;
  const admit = (held, r) => {
    const rowWithClock = { scope: { tier: "self" }, asserted_at: (clock += 10), ...r };
    const res = door(held, tables, rowWithClock, rowWithClock.asserted_at);
    assert.equal(res.ok, true, `expected ${rowWithClock.id} (${rowWithClock.family}/${rowWithClock.verb}) to be admitted, got ${res.refusal}`);
    return rowWithClock;
  };

  // The story's executable rows from the constitution's mesh_story_trace.json (rc17 had 13, rc19 has 68; this test uses
  // row1 to row13, which rc19 keeps, but for row3: Bob's device binding is now identity:occurrence)
  const storyById = new Map(trace.steps.filter((s) => s.row).map((s) => [s.row.id, s.row]));
  for (let k = 1; k <= 13; k++) assert.ok(storyById.has("row" + k), "the story keeps row" + k);

  // Register keys for all story actors + concurrent actors (row2 is bob's key from the story)
  for (const p of ["alice", "carol", "dave", "erin", "frank", "agent:bob"]) {
    admit(baseHeld, { id: `key:${p}:row`, family: "key:record", verb: "attest", signer: p, object: `key:${p}` });
  }
  admit(baseHeld, { ...storyById.get("row2") }); // bob's key:record

  // Bob founds room:1 (story row5) and Frank co-founds room:1 so Frank holds moderator standing in room:1
  admit(baseHeld, { ...storyById.get("row5"), scope: { tier: "self", room: "room:1" } });
  admit(baseHeld, { id: "row5:frank-founder", family: "group:roster", verb: "attest", signer: "frank", object: "room:1:frank", scope: { tier: "self", room: "room:1" } });

  // Bob invites Dave (story row6) & Carol to room:1, and both accept before the submarine cable cut
  admit(baseHeld, { ...storyById.get("row6"), subject: "dave", scope: { tier: "room", room: "room:1" } });
  admit(baseHeld, { id: "row6:dave-ack", family: "membership:{stage}", verb: "attest", signer: "dave", object: "invite:dave", subject: "dave", stance: "for", scope: { tier: "room", room: "room:1" } });
  admit(baseHeld, { id: "invite:carol:row", family: "membership:{stage}", verb: "attest", signer: "bob", object: "invite:carol", subject: "carol", scope: { tier: "room", room: "room:1" } });
  admit(baseHeld, { id: "invite:carol:ack", family: "membership:{stage}", verb: "attest", signer: "carol", object: "invite:carol", subject: "carol", stance: "for", scope: { tier: "room", room: "room:1" } });

  // Submarine cable cut: 3 disconnected islands diverge from baseHeld and admit concurrent acts via door()
  const islandA = new Map(baseHeld);
  const islandB = new Map(baseHeld);
  const islandC = new Map(baseHeld);

  // Island A (alice, frank): trust root authorization, ops room founding, invite, posts, transport binding
  admit(islandA, { ...storyById.get("row1") }); // alice authorizes root (trust:{job}:{version})
  admit(islandA, { id: "row1:frank-root", family: "trust:{job}:{version}", verb: "authorize", signer: "frank", object: "root:frank" });
  admit(islandA, { id: "ops:roster", family: "group:roster", verb: "attest", signer: "alice", object: "room:ops", scope: { tier: "room", room: "room:ops" } });
  admit(islandA, { id: "ops:invite:frank", family: "membership:{stage}", verb: "attest", signer: "alice", object: "invite:frank:ops", subject: "frank", scope: { tier: "room", room: "room:ops" } });
  admit(islandA, { id: "ops:invite:frank:ack", family: "membership:{stage}", verb: "attest", signer: "frank", object: "invite:frank:ops", subject: "frank", stance: "for", scope: { tier: "room", room: "room:ops" } });
  admit(islandA, { id: "ops:post:1", family: "chat:*", verb: "attest", signer: "alice", object: "post:ops:1", scope: { tier: "room", room: "room:ops" } });
  admit(islandA, { id: "ops:post:1:sup", family: "chat:*", verb: "supersede", signer: "alice", object: "post:ops:1", scope: { tier: "room", room: "room:ops" } });
  admit(islandA, { id: "bind:frank:row", family: "identity:occurrence", verb: "attest", signer: "frank", object: "bind:frank", scope: { tier: "federation" } });

  // Island B (bob, dave, frank, agent:bob): story rows 3, 7, 8, 9, 11, 12 + concurrent room post/withdrawal
  admit(islandB, { ...storyById.get("row3"), scope: { tier: "federation" } }); // bind:bob
  admit(islandB, { ...storyById.get("row7"), scope: { tier: "room", room: "room:1" } }); // dave post:1
  admit(islandB, { ...storyById.get("row8"), scope: { tier: "room", room: "room:1" } }); // dave post:2
  admit(islandB, { ...storyById.get("row9"), scope: { tier: "room", room: "room:1" } }); // frank moderation flag:post2
  admit(islandB, { ...storyById.get("row11"), scope: { tier: "room", room: "room:1" } }); // dave recants post:1
  admit(islandB, { ...storyById.get("row12") }); // agent:bob trace:1
  admit(islandB, { id: "room1:post:3", family: "chat:*", verb: "attest", signer: "bob", object: "post:3", scope: { tier: "room", room: "room:1" } });
  admit(islandB, { id: "room1:post:3:withdraw", family: "chat:*", verb: "withdraw", signer: "bob", object: "post:3", scope: { tier: "room", room: "room:1" } });

  // Island C (carol, erin): story rows 4, 10, 13 + research room founding, invite, posts, objection vote
  admit(islandC, { ...storyById.get("row4") }); // carol consent contact:carol-bob
  admit(islandC, { ...storyById.get("row10"), scope: { tier: "room", room: "room:1" } }); // carol objection obj:flag
  admit(islandC, { ...storyById.get("row13") }); // carol withdraws contact:carol-bob
  admit(islandC, { id: "res:roster", family: "group:roster", verb: "attest", signer: "carol", object: "room:res", scope: { tier: "room", room: "room:res" } });
  admit(islandC, { id: "res:invite:erin", family: "membership:{stage}", verb: "attest", signer: "carol", object: "invite:erin:res", subject: "erin", scope: { tier: "room", room: "room:res" } });
  admit(islandC, { id: "res:invite:erin:ack", family: "membership:{stage}", verb: "attest", signer: "erin", object: "invite:erin:res", subject: "erin", stance: "for", scope: { tier: "room", room: "room:res" } });
  admit(islandC, { id: "res:post:1", family: "chat:*", verb: "attest", signer: "erin", object: "post:res:1", scope: { tier: "room", room: "room:res" } });

  // Union of all rows held across the 3 islands
  const unionMap = new Map();
  for (const isl of [islandA, islandB, islandC]) {
    for (const [id, entry] of isl) unionMap.set(id, entry.row);
  }
  const allRows = [...unionMap.values()];
  for (let k = 1; k <= 13; k++) assert.ok(unionMap.has("row" + k), `story row row${k} present in union`);

  const people = ["alice", "bob", "carol", "dave", "erin", "frank", "agent:bob"];
  const snapshotFoldAndQueue = (heldMap, now) => {
    const f = fold(heldMap.values(), tables, now);
    const objects = [...f.objects.entries()].map(([id, o]) => ({
      id,
      family: o.family,
      state: o.state,
      author: o.author,
      subject: o.subject,
      scope: o.scope,
      outcome: o.outcome,
      voices: o.voices ? { for: [...o.voices.for].sort(), against: [...o.voices.against].sort() } : null,
      missingVoices: o.missing && o.missing.voices !== undefined ? o.missing.voices : null,
    }));
    const rooms = [...f.rooms.entries()].map(([id, r]) => ({
      id,
      founders: [...r.founders].sort(),
      members: [...r.members].sort(),
      removed: [...r.removed].sort(),
      moderators: [...r.moderators].sort(),
      locked: r.locked,
    }));
    const queues = Object.fromEntries(
      people.map((p) => {
        const q = queue(heldMap, tables, p, now);
        return [p, { items: q.items, starts: q.starts }];
      })
    );
    return {
      objects,
      rooms,
      registered: [...f.registered].sort(),
      holders: [...f.holders].sort(),
      nodesOf: [...f.nodesOf.entries()].map(([k, v]) => [k, [...v].sort()]),
      halt: f.halt,
      inert: [...f.inert.keys()].sort(),
      queues,
    };
  };

  let baseline = null;
  for (let i = 0; i < 100; i++) {
    // Fisher-Yates shuffle with deterministic PRNG
    const perm = [...allRows];
    for (let j = perm.length - 1; j > 0; j--) {
      const k = Math.floor(rng() * (j + 1));
      [perm[j], perm[k]] = [perm[k], perm[j]];
    }
    const evalNow = 100_000 + Math.floor(rng() * 50_000);

    // 1) Permuted delivery with jittered firstHeld arrival timestamps
    const jitteredHeld = new Map();
    for (const r of perm) {
      const firstHeld = 10_000 + Math.floor(rng() * 40_000);
      jitteredHeld.set(r.id, { row: r, firstHeld });
    }
    const snap = snapshotFoldAndQueue(jitteredHeld, evalNow);
    if (!baseline) baseline = snap;
    else assert.deepEqual(snap, baseline, `permutation ${i} diverged on jittered held fold/queue`);

    // 2) The same rows through door(), each delivered ONCE in this permutation's order (rc19: the door keeps a row that
    // waits for another, inert, and settles it when its cure arrives; no retry pass is needed)
    const doorHeld = new Map();
    for (const r of perm) {
      const s = door(doorHeld, tables, r, 10_000 + Math.floor(rng() * 900));
      assert.ok(s.ok || s.inert, `permutation ${i}: ${r.id} kept (${s.refusal})`);
    }
    assert.equal(doorHeld.size, allRows.length, `permutation ${i} keeps all ${allRows.length} rows`);
    assert.equal(fold(doorHeld.values(), tables, evalNow).inert.size, 0, `permutation ${i}: every waiting row cured by the end`);
    const doorSnap = snapshotFoldAndQueue(doorHeld, evalNow);
    assert.deepEqual(doorSnap, baseline, `permutation ${i} diverged after door() admission passes`);
  }
});

test("churn, mobile sleep/wake & stranger refusal under partition healing", () => {
  const m = new Mesh(tables);
  // Multi-device rosters for alice and bob, single device for dave, stranger device for mallory, and a server seat
  m.addNode("alice-phone", "alice");
  m.addNode("alice-laptop", "alice");
  m.addNode("bob-phone", "bob");
  m.addNode("bob-tablet", "bob");
  m.addNode("dave-phone", "dave");
  m.addNode("mallory-phone", "mallory");
  m.addNode("seat", "nobody", "server");

  const allClients = ["alice-phone", "alice-laptop", "bob-phone", "bob-tablet", "dave-phone", "mallory-phone"];
  for (const c of allClients) m.link(c, "seat");

  const sleep = (nodeId) => {
    for (const l of [...m.links]) {
      if (l.split("|").includes(nodeId)) m.links.delete(l);
    }
  };
  const wake = (nodeId) => m.link(nodeId, "seat");

  let seq = 0;
  const makeRow = (o) => ({ id: "churn-r" + (++seq), scope: { tier: "self" }, ...o });

  // Helper that verifies preview-equals-execution at every intermediate churn step before committing via sign()
  const signChecked = (nodeId, r) => {
    r.asserted_at = r.asserted_at ?? m.now;
    const node = m.nodes.get(nodeId);
    const heldSizeBefore = node.held.size;
    const pv = preview(node.held, tables, [r], m.now);
    assert.equal(node.held.size, heldSizeBefore, "preview must never mutate held");
    const res = m.sign(nodeId, r);
    assert.equal(pv.verdicts[0].ok, res.ok, `preview ok parity on ${r.id} at ${nodeId}`);
    assert.equal(pv.verdicts[0].refusal, res.refusal, `preview refusal parity on ${r.id} at ${nodeId}`);
    if (res.ok) {
      const fAfter = m.foldOf(nodeId);
      for (const ch of pv.changed) {
        assert.equal(fAfter.objects.get(ch.object).state, ch.to, `preview state parity for ${ch.object}`);
      }
      for (const rm of pv.rooms) {
        const roomAfter = fAfter.rooms.get(rm.room);
        for (const joined of rm.joined) assert.ok(roomAfter.members.has(joined), `preview room join parity for ${joined}`);
      }
    }
    return res;
  };

  // Phase 1: Bootstrap keys & transport bindings while all nodes are awake
  for (const [nodeId, owner] of [
    ["alice-phone", "alice"],
    ["bob-phone", "bob"],
    ["dave-phone", "dave"],
    ["mallory-phone", "mallory"],
  ]) {
    assert.ok(signChecked(nodeId, makeRow({ family: "key:record", verb: "attest", signer: owner, object: `key:${owner}` })).ok);
    assert.ok(signChecked(nodeId, makeRow({ family: "identity:occurrence", verb: "attest", signer: owner, object: `bind:${nodeId}`, scope: { tier: "federation" } })).ok);
  }
  m.tick();
  m.tick();
  for (const c of allClients) {
    assert.equal(m.foldOf(c).registered.size, 4, `${c} has all 4 registered keys`);
  }

  // Phase 2: alice-laptop and dave-phone go offline/asleep
  sleep("alice-laptop");
  sleep("dave-phone");

  // While alice-laptop and dave-phone sleep, alice signs a self-scoped trace and consent row on alice-phone
  const aliceSelfTrace = makeRow({ family: "trace:{form}:{version}", verb: "attest", signer: "alice", object: "trace:alice:private", scope: { tier: "self" } });
  assert.ok(signChecked("alice-phone", aliceSelfTrace).ok);
  assert.ok(signChecked("alice-phone", makeRow({ family: "consent:{kind}", verb: "attest", signer: "alice", object: "consent:alice-bob" })).ok);

  // Bob founds room:alpha on bob-phone and invites alice and dave
  assert.ok(signChecked("bob-phone", makeRow({ family: "group:roster", verb: "attest", signer: "bob", object: "room:alpha", scope: { tier: "room", room: "room:alpha" } })).ok);
  assert.ok(signChecked("bob-phone", makeRow({ family: "membership:{stage}", verb: "attest", signer: "bob", object: "invite:alice:alpha", subject: "alice", scope: { tier: "room", room: "room:alpha" } })).ok);
  assert.ok(signChecked("bob-phone", makeRow({ family: "membership:{stage}", verb: "attest", signer: "bob", object: "invite:dave:alpha", subject: "dave", scope: { tier: "room", room: "room:alpha" } })).ok);
  m.tick();
  m.tick();

  // Alice accepts on alice-phone and posts; bob-tablet (awake second device of bob) also participates
  assert.ok(signChecked("alice-phone", makeRow({ family: "membership:{stage}", verb: "attest", signer: "alice", object: "invite:alice:alpha", subject: "alice", stance: "for", scope: { tier: "room", room: "room:alpha" } })).ok);
  m.tick();
  m.tick();
  assert.ok(signChecked("alice-phone", makeRow({ family: "chat:*", verb: "attest", signer: "alice", object: "post:alpha:1", scope: { tier: "room", room: "room:alpha" } })).ok);
  assert.ok(signChecked("bob-tablet", makeRow({ family: "chat:*", verb: "attest", signer: "bob", object: "post:alpha:2", scope: { tier: "room", room: "room:alpha" } })).ok);
  assert.ok(signChecked("alice-phone", makeRow({ family: "chat:*", verb: "recant", signer: "alice", object: "post:alpha:1", scope: { tier: "room", room: "room:alpha" } })).ok);

  // Stranger (mallory-phone) tries to post in room:alpha and withdraw bob's post: refused by both preview and door
  // rc19: a row refused only for want of another row is kept inert on its signer's own node, with the row it waits for
  const mp = signChecked("mallory-phone", makeRow({ family: "chat:*", verb: "attest", signer: "mallory", object: "post:mallory:1", scope: { tier: "room", room: "room:alpha" } }));
  assert.equal(mp.ok, false); assert.equal(mp.inert, true); assert.equal(mp.missing.family, "group:roster", "it waits for room:alpha's roster");
  const mw = signChecked("mallory-phone", makeRow({ family: "chat:*", verb: "withdraw", signer: "mallory", object: "post:alpha:2", scope: { tier: "room", room: "room:alpha" } }));
  assert.equal(mw.ok, false); assert.equal(mw.inert, true);
  const malloryRows = [mp.plan.row.id, mw.plan.row.id];

  m.tick();
  m.tick();

  // Verify sleeping nodes missed the partition activity while offline
  assert.equal(m.foldOf("alice-laptop").objects.has("trace:alice:private"), false, "sleeping alice-laptop has not yet received alice's self trace");
  assert.equal(m.foldOf("alice-laptop").rooms.has("room:alpha"), false, "sleeping alice-laptop has not yet received room:alpha");
  assert.equal(m.foldOf("dave-phone").rooms.has("room:alpha"), false, "sleeping dave-phone has not yet received room:alpha");

  // Phase 3: Rolling churn — alice-phone and bob-phone go to sleep; alice-laptop and dave-phone wake up!
  sleep("alice-phone");
  sleep("bob-phone");
  wake("alice-laptop");
  wake("dave-phone");

  m.tick();
  m.tick();
  m.tick();

  // alice-laptop catches up on both alice's self-scoped row and room:alpha rows from the carrier seat
  assert.equal(m.foldOf("alice-laptop").objects.get("trace:alice:private")?.state, "admitted", "woken alice-laptop syncs alice's self-scoped trace");
  assert.equal(m.foldOf("alice-laptop").objects.get("post:alpha:1")?.state, "recanted", "woken alice-laptop folds recanted post:alpha:1");

  // dave-phone catches up on room:alpha and invite:dave:alpha, then accepts and posts while bob-phone & alice-phone sleep
  assert.ok(signChecked("dave-phone", makeRow({ family: "membership:{stage}", verb: "attest", signer: "dave", object: "invite:dave:alpha", subject: "dave", stance: "for", scope: { tier: "room", room: "room:alpha" } })).ok);
  m.tick();
  m.tick();
  assert.ok(signChecked("dave-phone", makeRow({ family: "chat:*", verb: "attest", signer: "dave", object: "post:alpha:3", scope: { tier: "room", room: "room:alpha" } })).ok);
  assert.ok(signChecked("alice-laptop", makeRow({ family: "chat:*", verb: "attest", signer: "alice", object: "post:alpha:4", scope: { tier: "room", room: "room:alpha" } })).ok);

  // Phase 4: Heal partition completely — wake alice-phone and bob-phone and sync across ticks
  wake("alice-phone");
  wake("bob-phone");
  for (let hop = 0; hop < 4; hop++) m.tick();

  // Verify 100% fold & queue parity between multi-device siblings (alice-phone == alice-laptop, bob-phone == bob-tablet)
  const canonFold = (nodeId, roomOnly = false) => {
    const f = m.foldOf(nodeId);
    const objs = [...f.objects.entries()]
      .filter(([, o]) => !roomOnly || (o.scope && o.scope.tier === "room"))
      .map(([id, o]) => [id, o.family, o.state, o.outcome, o.voices]);
    const rms = [...f.rooms.entries()].map(([id, r]) => [id, [...r.founders].sort(), [...r.members].sort(), [...r.moderators].sort(), r.locked]);
    return { objs, rms, reg: [...f.registered].sort() };
  };

  assert.deepEqual(canonFold("alice-phone"), canonFold("alice-laptop"), "alice-phone and alice-laptop reach 100% fold parity");
  assert.deepEqual(m.queueOf("alice-phone").items, m.queueOf("alice-laptop").items, "alice devices reach 100% queue items parity");
  assert.deepEqual(m.queueOf("alice-phone").starts, m.queueOf("alice-laptop").starts, "alice devices reach 100% queue starts parity");

  assert.deepEqual(canonFold("bob-phone"), canonFold("bob-tablet"), "bob-phone and bob-tablet reach 100% fold parity");
  assert.deepEqual(m.queueOf("bob-phone").items, m.queueOf("bob-tablet").items, "bob devices reach 100% queue items parity");
  assert.deepEqual(m.queueOf("bob-phone").starts, m.queueOf("bob-tablet").starts, "bob devices reach 100% queue starts parity");

  // All 5 member devices agree 100% on room:alpha state, roster, and posts
  for (const dev of ["alice-laptop", "bob-phone", "bob-tablet"]) {
    assert.deepEqual(canonFold(dev, true), canonFold("alice-phone", true), `${dev} matches alice-phone on all room folds`);
  }
  // a room row's audience is the room's members at the row's own head (CC 5.2; route.py at 2fb305c): dave joined after
  // alice's and bob's first posts, so he is not offered them (nothing he can see names them); everything else agrees
  const before = new Set(["post:alpha:1", "post:alpha:2"]), dv = canonFold("dave-phone", true), al = canonFold("alice-phone", true);
  assert.deepEqual({ ...dv, objs: dv.objs }, { ...al, objs: al.objs.filter(([id]) => !before.has(id)) }, "dave-phone matches alice-phone on every room row of his time");
  for (const id of before) assert.equal(m.foldOf("dave-phone").objects.has(id), false, `dave-phone was never offered ${id}, signed before he joined`);

  // Verify stranger refusal invariant on mallory-phone across all partition-healing bursts
  const malloryNode = m.nodes.get("mallory-phone");
  assert.equal(malloryNode.sealed.size, 0, "stranger phone never carries sealed rows");
  const consentFreeFamilies = new Set(["key:record", "transport:{kind}", "identity:occurrence", "consent:{kind}", "accord:invoke:constitutional:{halt_id}", "trust:{job}:{version}"]);
  for (const { row } of malloryNode.held.values()) {
    const isConsentFree = consentFreeFamilies.has(row.family) || (row.scope && row.scope.tier === "federation") || row.signer === "mallory";
    assert.equal(isConsentFree, true, `stranger mallory-phone must not hold private/room row ${row.id} (${row.family})`);
    // the only room rows a stranger holds are her own, inert there: concluding nothing and carried to nobody
    if (row.scope?.tier === "room"){ assert.equal(row.signer, "mallory"); assert.ok(m.foldOf("mallory-phone").inert.has(row.id), `${row.id} is inert on mallory-phone`); }
  }
  for (const [id, n] of m.nodes) if (id !== "mallory-phone") for (const r of malloryRows) assert.equal(n.held.has(r) || n.sealed.has(r), false, `${id} was never offered mallory's inert row ${r}`);
  assert.equal(m.foldOf("mallory-phone").rooms.size, 0, "stranger folds zero rooms");
});

test("CC namespace_match_vectors.json conformance: every dimension, row-type and polarity vector of the vendored commit", () => {
  let passedVectors = 0;
  for (const v of matchVectors.vectors) {
    const res = matchFamily(v.dimension, tables);
    assert.equal(res.family, v.family, `Vector ${v.dimension} family mismatch`);
    assert.equal(res.refusal, v.refusal, `Vector ${v.dimension} refusal mismatch`);
    if (!v.refusal && v.family) {
      const binds = extractDimensions(v.dimension, tables);
      assert.ok(binds, `Dimensions extracted for ${v.dimension}`);
    }
    passedVectors++;
  }
  assert.equal(passedVectors, matchVectors.vectors.length, "every dimension vector replayed"); assert.ok(passedVectors >= 1256, "rc19 has 1,299; rc17 had 1,256");

  const STRUCTURAL = new Set(["scores", "delegates_to", "supersedes", "withdraws", "recants"]);
  let passedRowTypes = 0;
  for (const v of matchVectors.row_type_vectors) {
    const isStruct = STRUCTURAL.has(v.attestation_type);
    const ref = isStruct ? null : "attestation_type_unregistered";
    assert.equal(ref, v.refusal, `Row type ${v.attestation_type} refusal mismatch`);
    passedRowTypes++;
  }
  assert.equal(passedRowTypes, matchVectors.row_type_vectors.length, "every row-type vector replayed");

  let passedPolarity = 0;
  for (const v of matchVectors.polarity_vectors || []) {
    const res = checkPolarity(tables, v.family, v.score);
    assert.equal(res.admissible, v.admissible, `Polarity vector ${v.family} admissible mismatch`);
    assert.equal(res.refusal, v.refusal, `Polarity vector ${v.family} refusal mismatch`);
    passedPolarity++;
  }
  assert.equal(passedPolarity, (matchVectors.polarity_vectors || []).length, "every polarity vector replayed"); assert.ok(passedPolarity > 0);
});

test("every family's every transition, every role token & csd_namespace_map.json verification", () => {
  // 1. Verify csd_namespace_map.json integrity & summary (re-resolved at each re-vendor by tools/kernel/remap_csd.mjs)
  const nTransitions = transitions.rows.reduce((a, r) => a + r.transitions.length, 0);
  assert.equal(csdMap.version, "cewp-synthesis-rc19-v6");
  assert.equal(csdMap.summary.total_families, transitions.rows.length);
  assert.equal(csdMap.families.length, transitions.rows.length);
  assert.equal(csdMap.summary.total_transitions, nTransitions);
  assert.equal(csdMap.summary.total_policies, policies.families.length);
  assert.equal(csdMap.summary.total_csd_cards, 105);
  assert.equal(csdMap.summary.csd_mapped_families + csdMap.summary.substrate_autonomous_families, transitions.rows.length);
  assert.equal(csdMap.summary.total_csd_shows_fields, 660);
  assert.equal(csdMap.summary.resolved_csd_shows_fields, 152);

  // Verify all 105 cards and their 660 fields
  let resolvedShowsCount = 0;
  for (const card of csdMap.cards) {
    assert.ok(card.id.startsWith("CSD-"));
    assert.ok(card.title);
    assert.ok(Array.isArray(card.topologies) && card.topologies.length > 0);
    for (const show of card.shows) {
      if (show.field.startsWith("x_private:")) {
        assert.equal(show.canonical_family, "x_private:*");
        const m = matchFamily(show.field, tables);
        assert.equal(m.refusal, "namespace_private_use_not_federatable");
        resolvedShowsCount++;
      } else if (show.field.startsWith("holds_bytes:")) {
        assert.equal(show.canonical_family, "holds_bytes:sha256:{prefix}");
        resolvedShowsCount++;
      } else {
        const m = matchFamily(show.field, tables);
        assert.equal(m.family, show.canonical_family, `Field ${show.field} in ${card.id}`);
        resolvedShowsCount++;
      }
    }
  }
  assert.equal(resolvedShowsCount, 660, "all 660 csd:shows fields resolved");

  // 2. Verify all 17 role tokens across all 474 transitions & 3 equivalences
  let testedTransitions = 0;
  const cannot = []; // transitions whose signer the twin kernel's standing cannot confer (listed below, never skipped silently)
  let gapCellsTested = 0;
  let regularTested = 0;

  for (const row of transitions.rows) {
    const family = row.family;
    for (const t of row.transitions) {
      testedTransitions++;
      const roles = t.roles.value || [];
      const held = new Map();
      let now = 1000;
      const testObj = 'obj:' + family.replace(/[:{}*]/g, '_') + ':' + testedTransitions;

      const fromNone = row.transitions.find(x => x.from === 'none');

      // (A) Equivalence 1: refused_previews_nothing on unauthorized stranger
      if (family !== 'key:record') {
        const strangerRow = {
          id: 'row:stranger:' + testedTransitions,
          family,
          verb: t.verb,
          signer: 'stranger-nobody',
          object: testObj,
          scope: { tier: 'self' },
          asserted_at: now++
        };
        const unauthPrev = preview(held, tables, [strangerRow], now);
        assert.equal(unauthPrev.verdicts[0].ok, false);
        assert.equal(unauthPrev.changed.length, 0);
        const unauthDoor = door(held, tables, strangerRow, now);
        assert.equal(unauthDoor.ok, false);
      }

      if (roles.length === 0) {
        gapCellsTested++;
        continue;
      }

      if (t.from === 'admitted' && (!fromNone || fromNone.roles.value?.length === 0)) {
        continue;
      }

      // (B) Authorized actor holding the required role
      regularTested++;
      const role = roles[0];
      const isSystemDetector = role === 'identity:lenscore_detector' || fromNone?.roles.value?.[0] === 'identity:lenscore_detector';
      const isSystemPersist = role === 'identity:substrate_persist' || fromNone?.roles.value?.[0] === 'identity:substrate_persist';
      
      let signer = isSystemDetector ? 'identity:lenscore_detector' :
                   isSystemPersist ? 'identity:substrate_persist' :
                   'actor-omni';

      const isRoomRole = roles.some(r => ['founder', 'member', 'moderate'].includes(r));
      const needsRoom = isRoomRole || (fromNone && fromNone.roles.value?.some(r => ['founder', 'member', 'moderate'].includes(r)));
      const roomId = 'room:omni';
      const scope = needsRoom ? { tier: 'room', room: roomId } : { tier: 'self' };

      door(held, tables, { id: 'key:' + signer, family: 'key:record', verb: 'attest', signer, object: 'key:' + signer, scope: { tier: 'self' }, asserted_at: now++ }, now);
      door(held, tables, { id: 'trust:init:' + signer, family: 'trust:{job}:{version}', verb: 'authorize', signer, object: 'trust:root:' + signer, scope: { tier: 'self' }, asserted_at: now++ }, now);
      const extraRoles = isSystemDetector ? ['identity:lenscore_detector'] :
                         isSystemPersist ? ['identity:substrate_persist'] :
                         ['steward', 'wa', 'takedown', 'review', 'slash', 'attest', 'custodian', 'operator', 'serve', 'tally', 'winner'];
      for (const spec of extraRoles) {
        door(held, tables, { id: 'trust:' + spec + ':obj', family: 'trust:{job}:{version}', verb: 'authorize', signer, object: 'trust:' + spec + ':1', scope: { tier: 'self' }, asserted_at: now++ }, now);
      }

      if (needsRoom) {
        door(held, tables, { id: 'room:roster:' + signer, family: 'group:roster', verb: 'attest', signer, object: roomId, scope: { tier: 'room', room: roomId }, asserted_at: now++ }, now);
      }

      if (t.from === 'admitted' && fromNone && fromNone.roles.value?.length > 0) {
        const initVerb = fromNone.verb;
        door(held, tables, { id: testObj + ':seed', family, verb: initVerb, signer, object: testObj, subject: signer, scope, asserted_at: now++ }, now);
      }

      const validRow = {
        id: 'row:valid:' + testedTransitions,
        family,
        verb: t.verb,
        signer,
        object: testObj,
        subject: signer,
        scope,
        asserted_at: now++
      };

      // Equivalence 2: door_is_queue
      const fBefore = fold(held.values(), tables, now);
      const curRoles = standing(fBefore, signer, { object: testObj, id: testObj, subject: signer, author: signer, room: needsRoom ? roomId : undefined });
      const qVerbs = enabledFor(tables, family, t.from, curRoles).map(x => x.verb);
      if (!qVerbs.includes(t.verb)) { cannot.push(`${family} ${t.verb} from ${t.from}: needs ${roles.join("|")}`); continue; }

      // Equivalence 3: preview_is_execution
      const validPrev = preview(held, tables, [validRow], now);
      if (!validPrev.verdicts[0].ok){ cannot.push(`${family} ${t.verb} from ${t.from}: queue offers it, the door refuses ${validPrev.verdicts[0].refusal}`); continue; }
      assert.ok(validPrev.changed.some(c => c.object === testObj && c.to === t.to), `Preview changed missing ${t.to} for ${family}`);

      const validAdm = admit(held, tables, validRow, now);
      assert.equal(validAdm.ok, true, `Admit failed for ${family} / ${t.verb}`);
      const fAfter = fold(held.values(), tables, now);
      assert.equal(fAfter.objects.get(testObj)?.state, t.to, `Final state mismatch for ${family}`);
    }
  }

  assert.equal(testedTransitions, nTransitions, "every transition tested");
  // rc17: 474 transitions, 13 gap, 458 executable; rc19 (db3cacf): see the printed counts
  console.log(`transitions ${testedTransitions}, closed (no signer) ${gapCellsTested}, executable ${regularTested}`);
  // Loud, never skipped: at rc19 four families take their first row from `infra:attest` (an infrastructure node's standing:
  // CC's transport, delivery, key-boundary and reachability rows), which the twin kernel has no row to confer; their rows
  // are refused at the door, so the later verbs cannot be reached either. A change here is a change in the kernel's reach.
  assert.deepEqual(cannot, [
    "delivery:{class} attest from none: needs infra:attest",
    "delivery:{class} withdraw from admitted: queue offers it, the door refuses no_transition",
    "key_boundary:{scope} attest from none: needs infra:attest",
    "key_boundary:{scope} withdraw from admitted: queue offers it, the door refuses no_transition",
    "peer_reachability:{network} attest from none: needs infra:attest",
    "peer_reachability:{network} withdraw from admitted: queue offers it, the door refuses no_transition",
    "peer_reachability:{network} supersede from admitted: queue offers it, the door refuses no_transition",
    "transport:{kind} attest from none: needs infra:attest",
    "transport:{kind} withdraw from admitted: queue offers it, the door refuses no_transition",
    "transport:{kind} supersede from admitted: queue offers it, the door refuses no_transition",
  ]);
  assert.ok(regularTested > 400 && gapCellsTested > 0);
});

test("every rule kind in the policy table and its vectors is one the kernel reads", () => {
  const kinds = new Set();
  for (const p of policies.families) for (const c of ["enact_rule", "cancel_rule", "floor_rule"]) if (p[c] && typeof p[c] === "object") kinds.add(p[c].kind);
  for (const v of policyVectors.rule_vectors) if (v.rule && typeof v.rule === "object") kinds.add(v.rule.kind);
  assert.deepEqual([...kinds].filter((k) => !RULE_KINDS.includes(k)), [], "a rule kind the kernel does not read");
});
test("CC §4.5.13 governance_policy_vectors.json conformance: replay every rule vector", () => {
  assert.ok(policyVectors.rule_vectors.length >= 270, "rc19 at 2fb305c has 270 rule vectors (1da70f0 had 240, rc17 180)");
  for (const v of policyVectors.rule_vectors) {
    const strict = v.cell === "enact";
    const got = ruleVotes(v.rule, v.base, strict);
    assert.equal(got, v.votes, `Rule vector mismatch for ${v.family} ${v.cell} base ${v.base}`);
    const pol = tables.policy.get(v.family);
    assert.ok(pol, `Missing policy for ${v.family}`);
    const polRule = pol[v.cell + "_rule"];
    const gotPol = ruleVotes(polRule, v.base, strict);
    assert.equal(gotPol, v.votes, `Policy rule mismatch for ${v.family} ${v.cell} base ${v.base}`);
  }
});

// ---------------------------------------------------------------- ranked choice (CC 4.5.13, formal/ranked_choice)
import { rankedSummary, rankedMerge, rankedFinish, electionFinish, irv as irvFn, optionId, cellId, RULE_KINDS } from "./kernel.js";
const RV = J("../ranked_choice_vectors.json");
const OPTS = RV.options_declared;
const rowsOf = (ballots) => ballots.map((b, i) => ({ person: "p" + i, ranking: [...b] }));
test("ranked choice gives the constitution's model's winner and rounds on every profile of up to four ballots", () => {
  let n = 0;
  for (const v of RV.profiles){
    const rounds = v[v.length - 1], winner = v[v.length - 2] || null, elected = v[v.length - 3] || null, ballots = v.slice(0, -3);
    const s = rankedSummary("poll:x", OPTS, rowsOf(ballots));
    const f = rankedFinish("poll:x", OPTS, s.cells, 3, RV.floor);
    assert.equal(f.winner, winner, JSON.stringify(ballots)); if (winner) assert.equal(f.rounds, rounds, JSON.stringify(ballots));
    // an election: a tie for weakest in any round seats nobody (the count runs again over the ballots then held)
    assert.equal(electionFinish("poll:x", OPTS, s.cells, 3, RV.floor).winner, elected, "election " + JSON.stringify(ballots)); n++;
  }
  assert.equal(n, RV.profiles.length); assert.ok(n > 3000);
  for (const c of RV.persons){ const s = rankedSummary("poll:x", OPTS, c.rows.map(([person, ranking]) => ({ person, ranking })));
    assert.equal(rankedFinish("poll:x", OPTS, s.cells, 3, RV.floor).winner, c.winner, JSON.stringify(c.rows)); }
});
test("ranked choice: the same ballots in every order give the same winner, and summaries of parts merge to the whole", () => {
  let seed = 7; const rnd = () => ((seed = (seed * 1103515245 + 12345) & 0x7fffffff) / 0x7fffffff);
  const R = []; for (const a of OPTS){ R.push([a]); for (const b of OPTS) if (b !== a){ R.push([a, b]); for (const c of OPTS) if (c !== a && c !== b) R.push([a, b, c]); } }
  for (let t = 0; t < 200; t++){
    const rows = Array.from({ length: 2 + Math.floor(rnd() * 9) }, (_, i) => ({ person: "p" + Math.floor(rnd() * 8), ranking: R[Math.floor(rnd() * R.length)] }));
    const whole = rankedSummary("poll:y", OPTS, rows), w = rankedFinish("poll:y", OPTS, whole.cells).winner;
    for (let k = 0; k < 6; k++){ const sh = rows.slice().sort(() => rnd() - 0.5); assert.equal(rankedFinish("poll:y", OPTS, rankedSummary("poll:y", OPTS, sh).cells).winner, w); }
    // split by person (a person's rows stay together: agreement is resolved before adding), summarised apart, merged
    const pa = rows.filter((r) => r.person < "p4"), pb = rows.filter((r) => r.person >= "p4");
    const m = rankedMerge(rankedSummary("poll:y", OPTS, pa).cells, rankedSummary("poll:y", OPTS, pb).cells);
    assert.deepEqual([...m].sort(), [...whole.cells].sort()); assert.equal(rankedFinish("poll:y", OPTS, m).winner, w);
  }
});
test("ranked choice: a person whose ballots disagree has none, and a tie for weakest goes against the later declared option", () => {
  // p1's two ballots disagree: neither counts, so b's two ballots win against a's one
  const rows = [{ person: "p1", ranking: ["a"] }, { person: "p1", ranking: ["c"] }, { person: "p2", ranking: ["b"] }, { person: "p3", ranking: ["b"] }, { person: "p4", ranking: ["a"] }];
  assert.equal(rankedFinish("poll:z", OPTS, rankedSummary("poll:z", OPTS, rows).cells).winner, "b");
  // the same rows in the other order: the last-held ballot would have changed the count, the rule does not
  assert.equal(rankedFinish("poll:z", OPTS, rankedSummary("poll:z", OPTS, rows.slice().reverse()).cells).winner, "b");
  // all three hold two first choices of six: each round's weakest is the later declared option. Declared [a, b, c]: c
  // goes (its ballots pass to a and b, 3 and 3), then b goes, and a wins; declared [c, b, a]: a goes, then b, and c wins
  const tie = [{ person: "q1", ranking: ["a", "b"] }, { person: "q2", ranking: ["c", "a"] }, { person: "q3", ranking: ["b"] }, { person: "q4", ranking: ["b"] }, { person: "q5", ranking: ["a", "c"] }, { person: "q6", ranking: ["c", "b"] }];
  const s = rankedSummary("poll:t", OPTS, tie).cells;
  const fwd = rankedFinish("poll:t", ["a", "b", "c"], rankedSummary("poll:t", ["a", "b", "c"], tie).cells), rev = rankedFinish("poll:t", ["c", "b", "a"], rankedSummary("poll:t", ["c", "b", "a"], tie).cells);
  assert.equal(fwd.winner, "a"); assert.equal(rev.winner, "c"); assert.ok(s.size > 0);
  // a ranking that repeats an option or names one not offered is refused, never counted
  const bad = rankedSummary("poll:z", OPTS, [{ person: "x", ranking: ["a", "a"] }, { person: "y", ranking: ["d"] }]);
  assert.deepEqual(bad.refused.sort(), ["x", "y"]); assert.equal(bad.cells.size, 0);
  // cells are named by digests, never positions: the same ranking in two polls is two cells
  assert.notEqual(cellId("poll:1", [optionId("poll:1", "a")]), cellId("poll:2", [optionId("poll:2", "a")]));
  // pinned in both kernels (crates/cewp-kernel/src/ranked.rs the_same_ids_as_the_page_kernel)
  assert.equal(optionId("poll:1", "a"), "8d089364168555d378185e44e1fce08bb398ce57683b8751204e49cc24cb0d45");
  assert.equal(cellId("poll:1", [optionId("poll:1", "a"), optionId("poll:1", "b")]), "b2cbf297e33bb1b79b409476fe9e470077c83e770ff253c9a159a8832293557f");
  assert.equal(cellId("poll:1", []), "547f7d3f4590e4cc20594c09c08dc853fa69186689a15c8909025d602289eaa6");
  assert.equal(rankedFinish("poll:z", OPTS, new Map([["00", 3]])).refused, "summary_cell_unknown");
  // where the last-held ballot decided (the earlier irv): p1 [a] then [b] gave b, the other order a; the rule gives one answer
  const flip = [{ person: "p1", ranking: ["a"] }, { person: "p1", ranking: ["b"] }, { person: "p2", ranking: ["b"] }, { person: "p3", ranking: ["a"] }];
  const w1 = rankedFinish("poll:o", OPTS, rankedSummary("poll:o", OPTS, flip).cells).winner, w2 = rankedFinish("poll:o", OPTS, rankedSummary("poll:o", OPTS, flip.slice().reverse()).cells).winner;
  assert.equal(w1, w2); assert.equal(w1, "a");
  // the fold's call is an election: ballots by person in any order; a tie seats nobody
  assert.equal(irvFn([{ signer: "p1", ranking: ["a"] }, { signer: "p2", ranking: ["b"] }, { signer: "p3", ranking: ["b"] }, { signer: "p1", ranking: ["c"] }], "poll:f", OPTS), "b");
  assert.equal(irvFn(flip.map((r) => ({ signer: r.person, ranking: r.ranking })), "poll:o", OPTS), null, "p2 b and p3 a tie: an election seats nobody");
});

// ---------------------------------------------------------------- the door keeps (formal/KERNEL.md; CC 5.2 at 820a7a0)
test("an inert row is offered to its signers only; the row it waits for reaches it only under that row's own audience, in every delivery order", () => {
  const orders = [["seat", "alice-phone", "alice-laptop", "bob-phone", "carol-phone", "dave-phone"], ["dave-phone", "carol-phone", "bob-phone", "alice-laptop", "alice-phone", "seat"], ["bob-phone", "seat", "dave-phone", "alice-phone", "carol-phone", "alice-laptop"]];
  const results = [];
  for (const order of orders){
    const m = new Mesh(tables), owner = { "seat": "nobody", "alice-phone": "alice", "alice-laptop": "alice", "bob-phone": "bob", "carol-phone": "carol", "dave-phone": "dave" };
    for (const id of order) m.addNode(id, owner[id], id === "seat" ? "server" : "phone");
    for (const id of order) if (id !== "seat") m.link(id, "seat");
    m.link("alice-phone", "alice-laptop"); // a person's two devices see each other; an inert row cannot ride the seat
    let n = 0; const row = (o) => ({ id: "w" + (++n), scope: { tier: "self" }, ...o }), room = { tier: "room", room: "room:1" };
    for (const [node, p] of [["alice-phone", "alice"], ["bob-phone", "bob"], ["carol-phone", "carol"], ["dave-phone", "dave"]]) assert.ok(m.sign(node, row({ family: "key:record", verb: "attest", signer: p, object: "key:" + p })).ok);
    for (let k = 0; k < 3; k++) m.tick();
    assert.ok(m.sign("bob-phone", row({ family: "group:roster", verb: "attest", signer: "bob", object: "room:1", scope: room })).ok);
    // carol and dave each post in room:1 before anyone admits them: kept inert on their own node, waiting for the roster
    const cp = m.sign("carol-phone", row({ family: "chat:*", verb: "attest", signer: "carol", object: "post:carol", scope: room }));
    const dp = m.sign("dave-phone", row({ family: "chat:*", verb: "attest", signer: "dave", object: "post:dave", scope: room }));
    assert.ok(cp.inert && dp.inert); assert.equal(cp.missing.family, "group:roster");
    // alice, not a member, posts on her phone while her laptop is linked: inert there, carried to her laptop only
    const ap = m.sign("alice-phone", row({ family: "chat:*", verb: "attest", signer: "alice", object: "post:alice", scope: room }));
    assert.ok(ap.inert);
    // bob invites dave (dave is then a subject of a live proposal: in the roster's audience); carol is never invited
    assert.ok(m.sign("bob-phone", row({ family: "membership:{stage}", verb: "attest", signer: "bob", object: "invite:dave", subject: "dave", scope: room })).ok);
    for (let k = 0; k < 4; k++) m.tick();
    { const y = m.sign("dave-phone", row({ family: "membership:{stage}", verb: "attest", signer: "dave", object: "invite:dave", subject: "dave", stance: "for", scope: room })); assert.ok(y.ok || y.inert, y.refusal); }
    for (let k = 0; k < 6; k++) m.tick();
    const fd = m.foldOf("dave-phone"), fc = m.foldOf("carol-phone");
    // the waiter in the cure's audience is cured: dave's post is live on dave's node
    assert.equal(fd.objects.get("post:dave")?.state, "admitted", "dave's waiting post is cured once the roster reaches him");
    // the waiter outside it waits: carol never receives the roster, her post stays inert, naming it
    assert.equal(m.nodes.get("carol-phone").held.has("w5"), false, "the roster never reaches carol");
    assert.ok(fc.inert.has(cp.plan.row.id) && fc.inert.get(cp.plan.row.id).missing.family === "group:roster");
    // inert rows went to their signers' nodes only: no member, no stranger, no seat (not even sealed)
    for (const [id, node] of m.nodes){
      const mine = owner[id];
      for (const r of [cp.plan.row, ap.plan.row]) if (r.signer !== mine) assert.equal(node.held.has(r.id) || node.sealed.has(r.id), false, `${id} holds ${r.signer}'s inert ${r.id}`);
    }
    assert.ok(m.nodes.get("alice-laptop").held.has(ap.plan.row.id), "the signer's own other device holds it");
    assert.ok(m.foldOf("alice-laptop").inert.has(ap.plan.row.id), "and it is inert there too");
    results.push([fd.objects.get("post:dave")?.state, [...fc.inert.keys()].sort(), [...m.foldOf("bob-phone").objects.keys()].sort()]);
  }
  for (const r of results) assert.deepEqual(r, results[0], "the same outcome in every delivery order");
});
test("an inert row confers nothing, and a row bad on its own form is refused in every order", () => {
  const held = new Map(); let t = 100;
  const d = (r) => door(held, tables, { scope: { tier: "self" }, asserted_at: (t += 10), ...r }, t);
  // zed has no key: zed's grant to himself, his room and his post are kept inert and confer nothing
  assert.ok(d({ id: "z1", family: "trust:{job}:{version}", verb: "authorize", signer: "zed", object: "trust:steward:1" }).inert);
  assert.ok(d({ id: "z2", family: "group:roster", verb: "attest", signer: "zed", object: "room:z", scope: { tier: "room", room: "room:z" } }).inert);
  let f = fold(held.values(), tables, t);
  assert.equal(f.rolesOf.get("zed"), undefined, "no standing from an inert row");
  assert.equal(f.rooms.has("room:z"), false, "no room from an inert row");
  assert.deepEqual([...f.inert.values()].map((v) => v.missing.family), ["key:record", "key:record"]);
  // the cure arrives: both conclude
  assert.ok(d({ id: "z0", family: "key:record", verb: "attest", signer: "zed", object: "key:zed" }).ok);
  f = fold(held.values(), tables, t);
  assert.equal(f.inert.size, 0); assert.ok(f.rolesOf.get("zed")?.has("steward")); assert.ok(f.rooms.has("room:z"));
  // own form: malformed, a private family, a verb the family never takes, a transition nobody may sign
  const bad = [{ id: "b1", family: "chat:*", verb: "attest", object: "post:b1" }, { id: "b2", family: "x_private:mine", verb: "attest", signer: "zed", object: "x:1" },
    { id: "b3", family: "chat:*", verb: "authorize", signer: "zed", object: "post:b3" }, { id: "b4", family: "duty:{kind}", verb: "attest", signer: "zed", object: "duty:1" }];
  for (let k = 0; k < 4; k++){ const h2 = new Map(); const order = bad.slice(k).concat(bad.slice(0, k)); const t0 = { t: 500 };
    for (const r of [{ id: "k", family: "key:record", verb: "attest", signer: "zed", object: "key:zed" }, ...order]){ const s = door(h2, tables, { scope: { tier: "self" }, asserted_at: (t0.t += 10), ...r }, t0.t); if (r.id !== "k"){ assert.equal(s.ok, false); assert.ok(!s.inert, `${r.id} refused on its own form (${s.refusal})`); } }
    assert.deepEqual([...h2.keys()], ["k"], "no row bad on its own form is held"); }
  assert.equal(door(new Map(), tables, bad[0], 1).refusal, "malformed");
});

// ---------------------------------------------------------------- a room row's audience at its own head (CC 5.2; route.py at 2fb305c)
test("a later joiner is not offered earlier posts, a removed member still receives the rows of its time, in every node order", () => {
  const out = [];
  for (const order of [["seat", "bob-phone", "carol-phone", "dave-phone", "erin-phone"], ["erin-phone", "dave-phone", "carol-phone", "bob-phone", "seat"], ["carol-phone", "seat", "erin-phone", "bob-phone", "dave-phone"]]){
    const m = new Mesh(tables), owner = { "seat": "nobody", "bob-phone": "bob", "carol-phone": "carol", "dave-phone": "dave", "erin-phone": "erin" };
    for (const id of order) m.addNode(id, owner[id], id === "seat" ? "server" : "phone");
    const links = () => { for (const id of order) if (id !== "seat") m.link(id, "seat"); };
    links();
    let n = 0; const row = (o) => ({ id: "h" + (++n), scope: { tier: "self" }, ...o }), room = { tier: "room", room: "room:h" };
    const sign = (node, o) => { const s = m.sign(node, row(o)); assert.ok(s.ok, `${o.signer} ${o.family} ${o.verb}: ${s.refusal}`); return s.plan.row.id; };
    for (const p of ["bob", "carol", "dave", "erin"]) sign(p + "-phone", { family: "key:record", verb: "attest", signer: p, object: "key:" + p });
    for (let k = 0; k < 3; k++) m.tick();
    sign("bob-phone", { family: "group:roster", verb: "attest", signer: "bob", object: "room:h", scope: room });
    for (const p of ["carol", "dave"]){ sign("bob-phone", { family: "membership:{stage}", verb: "attest", signer: "bob", object: "invite:" + p, subject: p, scope: room }); }
    for (let k = 0; k < 4; k++) m.tick();
    for (const p of ["carol", "dave"]) sign(p + "-phone", { family: "membership:{stage}", verb: "attest", signer: p, object: "invite:" + p, subject: p, stance: "for", scope: room });
    for (let k = 0; k < 4; k++) m.tick();
    // carol goes offline; bob posts once while she is a member
    m.links.delete(["carol-phone", "seat"].sort().join("|"));
    const p1 = sign("bob-phone", { family: "chat:*", verb: "attest", signer: "bob", object: "post:h1", scope: room });
    for (let k = 0; k < 3; k++) m.tick();
    // erin is invited and joins after that post: she is not offered it
    sign("bob-phone", { family: "membership:{stage}", verb: "attest", signer: "bob", object: "invite:erin", subject: "erin", scope: room });
    for (let k = 0; k < 4; k++) m.tick();
    sign("erin-phone", { family: "membership:{stage}", verb: "attest", signer: "erin", object: "invite:erin", subject: "erin", stance: "for", scope: room });
    for (let k = 0; k < 4; k++) m.tick();
    // carol is removed (her membership withdrawn by the room), then bob posts again
    sign("bob-phone", { family: "membership:{stage}", verb: "withdraw", signer: "bob", object: "invite:carol", subject: "carol", scope: room });
    for (let k = 0; k < 3; k++) m.tick();
    const p2 = sign("bob-phone", { family: "chat:*", verb: "attest", signer: "bob", object: "post:h2", scope: room });
    for (let k = 0; k < 3; k++) m.tick();
    // carol comes back
    links(); for (let k = 0; k < 6; k++) m.tick();
    const has = (node, id) => m.nodes.get(node).held.has(id);
    assert.ok(m.foldOf("bob-phone").rooms.get("room:h").removed.has("carol"), "carol is removed");
    assert.ok(has("carol-phone", p1), "a removed member still receives a row of her time (signed at a head where she was seated)");
    assert.equal(has("carol-phone", p2), false, "and not a row signed after her removal");
    assert.equal(has("erin-phone", p1), false, "a later joiner is not offered a post signed before she joined");
    assert.ok(has("erin-phone", p2), "and is offered the posts of her time");
    assert.ok(has("dave-phone", p1) && has("dave-phone", p2), "a member throughout receives both");
    out.push(Object.fromEntries([...m.nodes].map(([id, nd]) => [owner[id], [...nd.held.keys()].sort()])));
  }
  for (const o of out) assert.deepEqual(o, out[0], "the same rows held in every node order");
});
