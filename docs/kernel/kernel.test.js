// node --test docs/kernel/kernel.test.js
// Parity with the constitution's executable mesh story (manifests/mesh_story_trace.json): the same
// tables, the same standing, the same queue and the same refusals, step by step. Then the mesh.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { loadTables, enabledFor, Mesh, fold, standing, door, queue, preview } from "./kernel.js";

const here = new URL("./tables/", import.meta.url);
const J = (f) => JSON.parse(readFileSync(new URL(f, here)));
const tables = loadTables(J("transitions.json"), J("governance_policy.json"));
const trace = J("mesh_story_trace.json");

test("the queue is the door run forwards: parity with the constitution's story", () => {
  const states = new Map();
  for (const s of trace.steps) {
    const before = states.get(s.view.object) ?? "none";
    const verbs = enabledFor(tables, s.view.family, before, s.standing).map((t) => t.verb);
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

  // Pull the 13 non-null rows from the constitution's mesh_story_trace.json
  const storyById = new Map(trace.steps.filter((s) => s.row).map((s) => [s.row.id, s.row]));
  assert.equal(storyById.size, 13, "constitution story has 13 executable rows");

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
  admit(islandA, { id: "bind:frank:row", family: "transport:{kind}", verb: "attest", signer: "frank", object: "bind:frank", scope: { tier: "federation" } });

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
  for (const sId of storyById.keys()) assert.ok(unionMap.has(sId), `story row ${sId} present in union`);

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

    // 2) Causal re-merge through door() in multi-hop retry passes (as in Mesh.tick)
    const doorHeld = new Map();
    let pending = [...perm];
    let passes = 0;
    while (pending.length > 0 && passes < 6) {
      passes++;
      const retry = [];
      for (const r of pending) {
        const arrival = 10_000 + passes * 1000 + Math.floor(rng() * 900);
        const s = door(doorHeld, tables, r, arrival);
        if (!s.ok) retry.push(r);
      }
      pending = retry;
    }
    assert.equal(pending.length, 0, `permutation ${i} admitted all ${allRows.length} rows via door() within ${passes} passes`);
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
    assert.ok(signChecked(nodeId, makeRow({ family: "transport:{kind}", verb: "attest", signer: owner, object: `bind:${nodeId}`, scope: { tier: "federation" } })).ok);
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
  assert.equal(signChecked("mallory-phone", makeRow({ family: "chat:*", verb: "attest", signer: "mallory", object: "post:mallory:1", scope: { tier: "room", room: "room:alpha" } })).ok, false);
  assert.equal(signChecked("mallory-phone", makeRow({ family: "chat:*", verb: "withdraw", signer: "mallory", object: "post:alpha:2", scope: { tier: "room", room: "room:alpha" } })).ok, false);

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
  for (const dev of ["alice-laptop", "bob-phone", "bob-tablet", "dave-phone"]) {
    assert.deepEqual(canonFold(dev, true), canonFold("alice-phone", true), `${dev} matches alice-phone on all room folds`);
  }

  // Verify stranger refusal invariant on mallory-phone across all partition-healing bursts
  const malloryNode = m.nodes.get("mallory-phone");
  assert.equal(malloryNode.sealed.size, 0, "stranger phone never carries sealed rows");
  const consentFreeFamilies = new Set(["key:record", "transport:{kind}", "consent:{kind}", "accord:invoke:constitutional:{halt_id}", "trust:{job}:{version}"]);
  for (const { row } of malloryNode.held.values()) {
    const isConsentFree = consentFreeFamilies.has(row.family) || (row.scope && row.scope.tier === "federation") || row.signer === "mallory";
    assert.equal(isConsentFree, true, `stranger mallory-phone must not hold private/room row ${row.id} (${row.family})`);
    assert.notEqual(row.scope?.tier, "room", `stranger mallory-phone must never hold room-scoped row ${row.id}`);
  }
  assert.equal(m.foldOf("mallory-phone").rooms.size, 0, "stranger folds zero rooms");
});
