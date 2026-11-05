// node --test docs/kernel/kernel.test.js
// Parity with the constitution's executable mesh story (manifests/mesh_story_trace.json): the same
// tables, the same standing, the same queue and the same refusals, step by step. Then the mesh.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { loadTables, enabledFor, Mesh, fold, standing } from "./kernel.js";

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
  m.link("alice-phone", "seat"); m.link("bob-phone", "seat"); m.link("carol-phone", "seat"); m.link("alice-phone", "bob-phone");
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
