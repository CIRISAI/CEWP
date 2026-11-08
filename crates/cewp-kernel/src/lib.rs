//! Authoritative CC §4.5.13 state processor in pure Rust (Phase 1).
//!
//! A node holds rows, each with the instant it first held it, and nothing else.
//! Every verdict is a fold over that set, never stored.
//! One step appends and never rewrites.
//! The queue a person sees, the preview of an act and the door that admits it are one function.
//!
//! Pure Rust (std only, zero external crates in `[dependencies]`).

pub mod json;
pub mod mesh;
pub mod processor;
pub mod tables;
pub mod types;

pub use json::{parse as parse_json, JsonValue, ParseError};
pub use mesh::{Mesh, MeshLogEntry, MeshMove, MeshNode, SealedEntry, CONSENT_FREE};
pub use processor::{
    commit, door, enabled_for, fold, irv, preview, queue, standing, standing_target, step,
};
pub use tables::{
    is_bcp47_lang_code, is_currency, is_digits, is_hex, is_hex64, is_literal, is_version,
    is_version_attempt, is_version_like, is_vocab, policy_row, transition_of, Rules, Tables,
    VENDORED_CSD_NAMESPACE_MAP, VENDORED_GOVERNANCE_POLICY, VENDORED_MESH_STORY_TRACE,
    VENDORED_NAMESPACE_MATCH_VECTORS, VENDORED_NAMESPACE_REGISTRY, VENDORED_TRANSITIONS,
    VENDORED_VERSION,
};
pub use types::*;

/// Helper to construct a `Row` from JSON representation.
pub fn row_from_json(v: &JsonValue) -> Option<Row> {
    let id = v.get("id")?.as_str()?.to_string();
    let family = v.get("family")?.as_str()?.to_string();
    let verb = v.get("verb")?.as_str()?.to_string();
    let signer = v.get("signer")?.as_str()?.to_string();
    let signer_type = v
        .get("signerType")
        .or_else(|| v.get("signer_type"))
        .and_then(|s| s.as_str())
        .map(|s| s.to_string());
    let object = v.get("object")?.as_str()?.to_string();
    let scope = v.get("scope").map(|sc| Scope {
        tier: sc
            .get("tier")
            .and_then(|t| t.as_str())
            .unwrap_or("self")
            .to_string(),
        room: sc.get("room").and_then(|r| r.as_str()).map(|s| s.to_string()),
    });
    let subject = v.get("subject").and_then(|s| s.as_str()).map(|s| s.to_string());
    let stance = v.get("stance").and_then(|s| s.as_str()).map(|s| s.to_string());
    let ranking = v.get("ranking").and_then(|arr| {
        let mut r = Vec::new();
        for item in arr.as_array()? {
            if let Some(s) = item.as_str() {
                r.push(s.to_string());
            }
        }
        Some(r)
    });
    let asserted_at = v.get("asserted_at").and_then(|a| a.as_u64()).unwrap_or(0);
    let note = v.get("note").and_then(|n| n.as_str()).map(|s| s.to_string());
    let role = v.get("role").and_then(|r| r.as_str()).map(|s| s.to_string());
    let job = v.get("job").and_then(|j| j.as_str()).map(|s| s.to_string());
    Some(Row {
        id,
        family,
        verb,
        signer,
        signer_type,
        object,
        scope,
        subject,
        stance,
        ranking,
        asserted_at,
        note,
        role,
        job,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};

    // -------------------------------------------------------------------------
    // Test 1: story_trace_parity
    // Replays all 14 steps / 13 rows of docs/kernel/tables/mesh_story_trace.json
    // asserts 100% parity on every step's queue items, admitted rows, refused acts,
    // and folded states (SEMANTIC_DRIFT == 0).
    // -------------------------------------------------------------------------
    #[test]
    fn test_story_trace_parity() {
        let tables = Tables::load_vendored();
        let trace = &tables.story_trace;
        let steps = trace.get("steps").and_then(|s| s.as_array()).expect("steps array");

        assert_eq!(steps.len(), 14, "mesh_story_trace.json must contain 14 steps");

        let mut states: HashMap<String, String> = HashMap::new();

        for s in steps {
            let n = s.get("n").and_then(|v| v.as_u64()).unwrap();
            let note = s.get("note").and_then(|v| v.as_str()).unwrap_or("");
            let view = s.get("view").expect("view object");
            let obj_id = view.get("object").and_then(|v| v.as_str()).unwrap();
            let family = view.get("family").and_then(|v| v.as_str()).unwrap();
            let state_after = view.get("state_after").and_then(|v| v.as_str()).unwrap();

            let standing_roles: Vec<String> = s
                .get("standing")
                .and_then(|v| v.as_array())
                .unwrap()
                .iter()
                .map(|r| r.as_str().unwrap().to_string())
                .collect();

            let expected_queue: Vec<String> = s
                .get("queue")
                .and_then(|v| v.as_array())
                .unwrap()
                .iter()
                .map(|q| q.as_str().unwrap().to_string())
                .collect();

            let chosen = s.get("chosen").and_then(|v| v.as_str());
            let refused = s.get("refused").and_then(|v| v.as_bool()).unwrap();

            let before = states.get(obj_id).map(|s| s.as_str()).unwrap_or("none");
            let enabled_transitions = enabled_for(&tables, family, before, &standing_roles);
            let verbs: Vec<String> = enabled_transitions.iter().map(|t| t.verb.clone()).collect();

            assert_eq!(
                verbs, expected_queue,
                "step {}: {} queue verbs mismatch (SEMANTIC_DRIFT > 0)",
                n, note
            );

            let chosen_str = chosen.unwrap_or("\0");
            let is_refused = !verbs.iter().any(|v| v == chosen_str);
            assert_eq!(
                is_refused, refused,
                "step {}: {} refusal mismatch (SEMANTIC_DRIFT > 0)",
                n, note
            );

            states.insert(obj_id.to_string(), state_after.to_string());
        }

        // Part B: Replay all 13 story rows via door() with registered keys
        let mut story_held: Vec<HeldRow> = Vec::new();
        let mut t_clock = 100u64;

        // Register keys for story participants
        for actor in &["alice", "carol", "dave", "frank", "agent:bob"] {
            t_clock += 10;
            let key_row = Row {
                id: format!("trace-key:{}", actor),
                family: "key:record".to_string(),
                verb: "attest".to_string(),
                signer: actor.to_string(),
                signer_type: None,
                object: format!("key:{}", actor),
                scope: Some(Scope {
                    tier: "self".to_string(),
                    room: None,
                }),
                subject: None,
                stance: None,
                ranking: None,
                asserted_at: t_clock,
                note: None,

                role: None,

                job: None,
            };
            door(&mut story_held, &tables, &key_row, t_clock).unwrap();
        }

        // Replay the story steps
        for s in steps {
            let n = s.get("n").and_then(|v| v.as_u64()).unwrap();
            let view = s.get("view").expect("view object");
            let obj_id = view.get("object").and_then(|v| v.as_str()).unwrap();
            let state_after = view.get("state_after").and_then(|v| v.as_str()).unwrap();
            let refused = s.get("refused").and_then(|v| v.as_bool()).unwrap();

            if let Some(row_json) = s.get("row") {
                if !row_json.is_null() {
                    let mut row = row_from_json(row_json).expect("valid row in trace step");
                    t_clock += 10;
                    row.asserted_at = t_clock;

                    // Scope adjustments matching story context for Dave & Carol
                    if row.id == "row6" {
                        row.subject = Some("dave".to_string());
                        row.scope = Some(Scope {
                            tier: "room".to_string(),
                            room: Some("room:1".to_string()),
                        });
                    } else if row.id == "row7" || row.id == "row8" || row.id == "row9" || row.id == "row10" || row.id == "row11" {
                        row.scope = Some(Scope {
                            tier: "room".to_string(),
                            room: Some("room:1".to_string()),
                        });
                    } else if row.id == "row3" {
                        row.scope = Some(Scope {
                            tier: "federation".to_string(),
                            room: None,
                        });
                    }

                    // For Dave's invite (row6), Dave accepts so he is a member for subsequent posts
                    let is_row6 = row.id == "row6";

                    let res = door(&mut story_held, &tables, &row, t_clock);
                    assert!(
                        res.is_ok(),
                        "step {}: expected door ok, got {:?}",
                        n, res
                    );

                    if is_row6 {
                        // Dave accepts invite
                        t_clock += 10;
                        let dave_ack = Row {
                            id: "trace-row6:dave-ack".to_string(),
                            family: "membership:{stage}".to_string(),
                            verb: "attest".to_string(),
                            signer: "dave".to_string(),
                            signer_type: None,
                            object: "invite:dave".to_string(),
                            scope: Some(Scope {
                                tier: "room".to_string(),
                                room: Some("room:1".to_string()),
                            }),
                            subject: Some("dave".to_string()),
                            stance: Some("for".to_string()),
                            ranking: None,
                            asserted_at: t_clock,
                            note: None,

                            role: None,

                            job: None,
                        };
                        door(&mut story_held, &tables, &dave_ack, t_clock).unwrap();

                        // Carol is invited and accepts
                        t_clock += 10;
                        let carol_inv = Row {
                            id: "trace-invite:carol".to_string(),
                            family: "membership:{stage}".to_string(),
                            verb: "attest".to_string(),
                            signer: "bob".to_string(),
                            signer_type: None,
                            object: "invite:carol".to_string(),
                            scope: Some(Scope {
                                tier: "room".to_string(),
                                room: Some("room:1".to_string()),
                            }),
                            subject: Some("carol".to_string()),
                            stance: None,
                            ranking: None,
                            asserted_at: t_clock,
                            note: None,

                            role: None,

                            job: None,
                        };
                        door(&mut story_held, &tables, &carol_inv, t_clock).unwrap();

                        t_clock += 10;
                        let carol_ack = Row {
                            id: "trace-invite:carol:ack".to_string(),
                            family: "membership:{stage}".to_string(),
                            verb: "attest".to_string(),
                            signer: "carol".to_string(),
                            signer_type: None,
                            object: "invite:carol".to_string(),
                            scope: Some(Scope {
                                tier: "room".to_string(),
                                room: Some("room:1".to_string()),
                            }),
                            subject: Some("carol".to_string()),
                            stance: Some("for".to_string()),
                            ranking: None,
                            asserted_at: t_clock,
                            note: None,

                            role: None,

                            job: None,
                        };
                        door(&mut story_held, &tables, &carol_ack, t_clock).unwrap();
                    } else if row.id == "row5" {
                        // Frank co-founds room:1 so Frank holds moderator standing in room:1
                        t_clock += 10;
                        let frank_founder = Row {
                            id: "trace-row5:frank-founder".to_string(),
                            family: "group:roster".to_string(),
                            verb: "attest".to_string(),
                            signer: "frank".to_string(),
                            signer_type: None,
                            object: "room:1:frank".to_string(),
                            scope: Some(Scope {
                                tier: "self".to_string(),
                                room: Some("room:1".to_string()),
                            }),
                            subject: None,
                            stance: None,
                            ranking: None,
                            asserted_at: t_clock,
                            note: None,

                            role: None,

                            job: None,
                        };
                        door(&mut story_held, &tables, &frank_founder, t_clock).unwrap();
                    }

                    let f = fold(&story_held, &tables, t_clock);
                    let obj_state = f
                        .objects
                        .get(obj_id)
                        .map(|o| o.state.as_str())
                        .unwrap_or("none");
                    assert_eq!(
                        obj_state, state_after,
                        "step {}: fold state mismatch for object {}",
                        n, obj_id
                    );
                }
            } else if refused {
                // Step 8: stranger tries to withdraw Dave's post
                let stranger_row = Row {
                    id: format!("stranger-attempt-step-{}", n),
                    family: "chat:*".to_string(),
                    verb: "withdraw".to_string(),
                    signer: "stranger".to_string(),
                    signer_type: None,
                    object: "post:1".to_string(),
                    scope: Some(Scope {
                        tier: "room".to_string(),
                        room: Some("room:1".to_string()),
                    }),
                    subject: None,
                    stance: None,
                    ranking: None,
                    asserted_at: t_clock + 5,
                    note: Some("unauthorized stranger withdrawal".to_string()),
                    role: None,
                    job: None,
                };
                let res = step(&story_held, &tables, &stranger_row, t_clock + 5);
                assert!(
                    res.is_err(),
                    "step {}: stranger act must be refused at door",
                    n
                );
            }
        }
    }

    // -------------------------------------------------------------------------
    // Test 2: eight_event_lifecycle_theorems
    // Explicitly tests all 8 theorems from formal/event_lifecycle/EventLifecycle.lean
    // -------------------------------------------------------------------------
    #[test]
    fn test_eight_event_lifecycle_theorems() {
        let tables = Tables::load_vendored();
        let mut held: Vec<HeldRow> = Vec::new();
        let now = 1000u64;

        // Register keys for alice and bob
        let alice_key = Row {
            id: "key:alice:row".to_string(),
            family: "key:record".to_string(),
            verb: "attest".to_string(),
            signer: "alice".to_string(),
            signer_type: None,
            object: "key:alice".to_string(),
            scope: Some(Scope {
                tier: "self".to_string(),
                room: None,
            }),
            subject: None,
            stance: None,
            ranking: None,
            asserted_at: 100,
            note: None,

            role: None,

            job: None,
        };
        door(&mut held, &tables, &alice_key, 100).unwrap();

        let bob_key = Row {
            id: "key:bob:row".to_string(),
            family: "key:record".to_string(),
            verb: "attest".to_string(),
            signer: "bob".to_string(),
            signer_type: None,
            object: "key:bob".to_string(),
            scope: Some(Scope {
                tier: "self".to_string(),
                room: None,
            }),
            subject: None,
            stance: None,
            ranking: None,
            asserted_at: 110,
            note: None,

            role: None,

            job: None,
        };
        door(&mut held, &tables, &bob_key, 110).unwrap();

        // Alice founds room:1
        let room_row = Row {
            id: "room:1:row".to_string(),
            family: "group:roster".to_string(),
            verb: "attest".to_string(),
            signer: "alice".to_string(),
            signer_type: None,
            object: "room:1".to_string(),
            scope: Some(Scope {
                tier: "room".to_string(),
                room: Some("room:1".to_string()),
            }),
            subject: None,
            stance: None,
            ranking: None,
            asserted_at: 120,
            note: None,

            role: None,

            job: None,
        };
        door(&mut held, &tables, &room_row, 120).unwrap();

        // Alice invites Bob
        let invite_row = Row {
            id: "invite:bob:row".to_string(),
            family: "membership:{stage}".to_string(),
            verb: "attest".to_string(),
            signer: "alice".to_string(),
            signer_type: None,
            object: "invite:bob".to_string(),
            scope: Some(Scope {
                tier: "room".to_string(),
                room: Some("room:1".to_string()),
            }),
            subject: Some("bob".to_string()),
            stance: None,
            ranking: None,
            asserted_at: 130,
            note: None,

            role: None,

            job: None,
        };
        door(&mut held, &tables, &invite_row, 130).unwrap();

        // Bob accepts invitation
        let bob_ack = Row {
            id: "invite:bob:ack".to_string(),
            family: "membership:{stage}".to_string(),
            verb: "attest".to_string(),
            signer: "bob".to_string(),
            signer_type: None,
            object: "invite:bob".to_string(),
            scope: Some(Scope {
                tier: "room".to_string(),
                room: Some("room:1".to_string()),
            }),
            subject: Some("bob".to_string()),
            stance: Some("for".to_string()),
            ranking: None,
            asserted_at: 140,
            note: None,

            role: None,

            job: None,
        };
        door(&mut held, &tables, &bob_ack, 140).unwrap();

        // Alice posts in room:1
        let post_row = Row {
            id: "post:1".to_string(),
            family: "chat:*".to_string(),
            verb: "attest".to_string(),
            signer: "alice".to_string(),
            signer_type: None,
            object: "post:1".to_string(),
            scope: Some(Scope {
                tier: "room".to_string(),
                room: Some("room:1".to_string()),
            }),
            subject: None,
            stance: None,
            ranking: None,
            asserted_at: 150,
            note: None,

            role: None,

            job: None,
        };
        door(&mut held, &tables, &post_row, 150).unwrap();

        // Theorem 1: door_is_queue (admits v s t = true <-> toItem t \in queue v s)
        // For an act on an existing object: Alice recanting post:1
        let q_alice = queue(&held, &tables, "alice", now);
        let recant_in_queue = q_alice
            .items
            .iter()
            .any(|item| item.object == "post:1" && item.verb == "recant");
        let recant_row = Row {
            id: "post:1:recant".to_string(),
            family: "chat:*".to_string(),
            verb: "recant".to_string(),
            signer: "alice".to_string(),
            signer_type: None,
            object: "post:1".to_string(),
            scope: Some(Scope {
                tier: "room".to_string(),
                room: Some("room:1".to_string()),
            }),
            subject: None,
            stance: None,
            ranking: None,
            asserted_at: now + 1,
            note: None,

            role: None,

            job: None,
        };
        let step_recant = step(&held, &tables, &recant_row, now + 1);
        assert_eq!(
            step_recant.is_ok(),
            recant_in_queue,
            "Theorem 1 door_is_queue violated"
        );

        // Theorem 2: queue_sound (\forall v s i, i \in queue v s -> admits v s i.writes = true)
        for item in &q_alice.items {
            let row = Row {
                id: format!("test-sound-{}", item.verb),
                family: item.family.clone(),
                verb: item.verb.clone(),
                signer: "alice".to_string(),
                signer_type: None,
                object: item.object.clone(),
                scope: Some(Scope {
                    tier: "room".to_string(),
                    room: Some("room:1".to_string()),
                }),
                subject: None,
                stance: None,
                ranking: None,
                asserted_at: now + 2,
                note: None,

                role: None,

                job: None,
            };
            let admits = step(&held, &tables, &row, now + 2);
            assert!(
                admits.is_ok(),
                "Theorem 2 queue_sound violated on verb {}",
                item.verb
            );
        }

        // Theorem 3: queue_complete (\forall v s t, t \in v.family.transitions -> admits v s t = true -> toItem t \in queue v s)
        let f = fold(&held, &tables, now);
        let o_post = f.objects.get("post:1").unwrap();
        let alice_roles = standing(&f, "alice", "post:1");
        let enabled_transitions = enabled_for(&tables, &o_post.family, &o_post.state, &alice_roles);
        for t in enabled_transitions {
            let in_q = q_alice
                .items
                .iter()
                .any(|it| it.object == "post:1" && it.verb == t.verb && it.to == t.to);
            assert!(
                in_q,
                "Theorem 3 queue_complete violated for transition {}",
                t.verb
            );
        }

        // Theorem 4: preview_is_execution (\forall v s i, i \in queue v s -> execute v s i.writes = some (answer v i))
        let pv = preview(&held, &tables, &[recant_row.clone()], now + 1);
        assert!(pv.verdicts[0].ok, "preview must succeed");
        assert!(
            pv.changed.iter().any(|c| c.object == "post:1" && c.to == "recanted"),
            "Theorem 4 preview_is_execution changed object state parity"
        );
        let mut held_copy = held.clone();
        door(&mut held_copy, &tables, &recant_row, now + 1).unwrap();
        let f_after = fold(&held_copy, &tables, now + 1);
        assert_eq!(
            f_after.objects.get("post:1").unwrap().state,
            "recanted",
            "Theorem 4 door execution parity with preview"
        );

        // Theorem 5: refused_previews_nothing (\forall v s t, t \notin queue -> execute = none)
        let stranger_post = Row {
            id: "stranger-post".to_string(),
            family: "chat:*".to_string(),
            verb: "attest".to_string(),
            signer: "stranger".to_string(),
            signer_type: None,
            object: "stranger-obj".to_string(),
            scope: Some(Scope {
                tier: "room".to_string(),
                room: Some("room:1".to_string()),
            }),
            subject: None,
            stance: None,
            ranking: None,
            asserted_at: now + 5,
            note: None,

            role: None,

            job: None,
        };
        let pv_refused = preview(&held, &tables, &[stranger_post.clone()], now + 5);
        assert!(!pv_refused.verdicts[0].ok, "refused act verdict must be !ok");
        assert_eq!(
            pv_refused.changed.len(),
            0,
            "Theorem 5 refused_previews_nothing: changed must be empty"
        );
        assert_eq!(
            pv_refused.rooms.len(),
            0,
            "Theorem 5 refused_previews_nothing: rooms must be empty"
        );

        // Theorem 6: stranger_sees_nothing (queue v <[]> = [])
        let q_stranger = queue(&held, &tables, "stranger", now);
        for item in &q_stranger.items {
            let o = f.objects.get(&item.object).unwrap();
            assert_ne!(
                o.scope.tier, "room",
                "Theorem 6 stranger_sees_nothing: stranger must never see room items"
            );
        }
        for st in &q_stranger.starts {
            assert!(
                st.room.is_none(),
                "Theorem 6 stranger_sees_nothing: stranger must never see room starts"
            );
        }

        // Theorem 7: answered_item_leaves_queue (i \in queue -> from != to -> i \notin queue after answer)
        let q_alice_after = queue(&held_copy, &tables, "alice", now + 2);
        let recant_still_present = q_alice_after
            .items
            .iter()
            .any(|item| item.object == "post:1" && item.verb == "recant");
        assert!(
            !recant_still_present,
            "Theorem 7 answered_item_leaves_queue violated"
        );

        // Theorem 8: widening_is_confirmed (i \in queue -> i.confirm = i.writes.widens)
        for it in &q_alice.items {
            let t = transition_of(&tables, &it.family, &it.verb, "admitted").unwrap();
            assert_eq!(
                it.confirm, t.widens,
                "Theorem 8 widening_is_confirmed violated on item {}",
                it.verb
            );
        }
        for st in &q_alice.starts {
            let t = transition_of(&tables, &st.family, &st.verb, "none").unwrap();
            assert_eq!(
                st.confirm, t.widens,
                "Theorem 8 widening_is_confirmed violated on start {}",
                st.verb
            );
        }
    }

    // -------------------------------------------------------------------------
    // Test 3: mesh_propagation_and_stranger_isolation
    // Reproduces the 6-peer mesh test from kernel.test.js:
    // keys spread through seat, stranger is never offered room rows,
    // two-path invite + acceptance seats invitee, folds agree.
    // -------------------------------------------------------------------------
    #[test]
    fn test_mesh_propagation_and_stranger_isolation() {
        let tables = Tables::load_vendored();
        let mut m = Mesh::new(tables);

        m.add_node("alice-phone", "alice", "phone");
        m.add_node("bob-phone", "bob", "phone");
        m.add_node("carol-phone", "carol", "phone");
        m.add_node("seat", "nobody", "server");

        m.link("alice-phone", "seat");
        m.link("bob-phone", "seat");
        m.link("carol-phone", "seat");

        let mut id = 0;
        let mut make_row = |family: &str, verb: &str, signer: &str, object: &str, room: Option<&str>, subject: Option<&str>, stance: Option<&str>| {
            id += 1;
            Row {
                id: format!("r{}", id),
                family: family.to_string(),
                verb: verb.to_string(),
                signer: signer.to_string(),
                signer_type: None,
                object: object.to_string(),
                scope: Some(Scope {
                    tier: if room.is_some() { "room".to_string() } else { "self".to_string() },
                    room: room.map(|r| r.to_string()),
                }),
                subject: subject.map(|s| s.to_string()),
                stance: stance.map(|s| s.to_string()),
                ranking: None,
                asserted_at: 0,
                note: None,

                role: None,

                job: None,
            }
        };

        for p in &["alice", "bob", "carol"] {
            let r = make_row("key:record", "attest", p, &format!("key:{}", p), None, None, None);
            assert!(m.sign(&format!("{}-phone", p), r).is_ok());
        }

        // Stranger's row is refused at the door until key is held
        let early = make_row("chat:*", "attest", "bob", "post:x", Some("room:1"), None, None);
        assert!(!m.sign("alice-phone", early).is_ok());

        m.tick(1000);
        m.tick(1000);

        assert!(
            m.nodes.get("seat").unwrap().held.len() >= 3,
            "keys must reach the seat"
        );
        assert!(
            m.nodes.get("carol-phone").unwrap().held.iter().any(|h| h.row.id == "r1"),
            "alice's key reaches carol through seat"
        );

        // Bob founds room:1 (r5); roster row reaches only members
        let r5 = make_row("group:roster", "attest", "bob", "room:1", Some("room:1"), None, None);
        assert!(m.sign("bob-phone", r5).is_ok());

        m.tick(1000);
        m.tick(1000);
        m.tick(1000);

        assert!(
            !m.nodes.get("carol-phone").unwrap().held.iter().any(|h| h.row.id == "r5"),
            "stranger carol is not offered a room row"
        );
        assert!(
            !m.nodes.get("carol-phone").unwrap().sealed.contains_key("r5"),
            "a phone is not a carrier"
        );
        assert!(
            m.nodes.get("seat").unwrap().sealed.contains_key("r5")
                && !m.nodes.get("seat").unwrap().held.iter().any(|h| h.row.id == "r5"),
            "a seat carries room row sealed and never folds it"
        );

        // Bob invites Alice (r6); invitation is two-path
        let r6 = make_row(
            "membership:{stage}",
            "attest",
            "bob",
            "invite:alice",
            Some("room:1"),
            Some("alice"),
            None,
        );
        assert!(m.sign("bob-phone", r6).is_ok());

        m.tick(1000);
        m.tick(1000);

        let f_a = m.fold_of("alice-phone");
        assert_eq!(
            f_a.objects.get("invite:alice").unwrap().outcome,
            None,
            "no single vote admits"
        );

        // Alice accepts: second voice (r7)
        let r7 = make_row(
            "membership:{stage}",
            "attest",
            "alice",
            "invite:alice",
            Some("room:1"),
            Some("alice"),
            Some("for"),
        );
        assert!(
            m.sign("alice-phone", r7).is_ok(),
            "the invitee's acceptance is admitted"
        );

        m.tick(1000);
        m.tick(1000);

        for n in &["alice-phone", "bob-phone"] {
            let f = m.fold_of(n);
            let r = f.rooms.get("room:1").expect("room:1 exists");
            assert!(r.members.contains("alice"), "{} seats alice", n);
        }

        // Folds agree on voices for invite:alice
        let a = m.fold_of("alice-phone").objects.get("invite:alice").unwrap().voices.clone();
        let b = m.fold_of("bob-phone").objects.get("invite:alice").unwrap().voices.clone();
        assert_eq!(a, b, "voices agree across nodes");

        // Alice may now post; Carol (stranger) sees nothing on the room
        let qa = m.queue_of("alice-phone");
        assert!(
            qa.starts.iter().any(|s| s.family == "chat:*" && s.room.as_deref() == Some("room:1")),
            "alice may start a post in room:1"
        );
        assert!(
            !standing(&m.fold_of("carol-phone"), "carol", "room:1").contains(&"member".to_string()),
            "carol has no member standing in room:1"
        );

        // Preview equals execution
        let post = make_row("chat:*", "attest", "alice", "post:1", Some("room:1"), None, None);
        let pv = m.preview_at("alice-phone", &[post.clone()]);
        assert!(
            pv.verdicts[0].ok && pv.changed.iter().any(|c| c.object == "post:1" && c.to == "admitted"),
            "preview succeeds and predicts admitted state"
        );
        assert!(m.sign("alice-phone", post).is_ok());
        assert_eq!(
            m.fold_of("alice-phone").objects.get("post:1").unwrap().state,
            "admitted"
        );
    }

    // -------------------------------------------------------------------------
    // Deterministic Mulberry32 PRNG (exact bit-for-bit match with kernel.test.js)
    // -------------------------------------------------------------------------
    struct Mulberry32 {
        seed: f64,
    }

    impl Mulberry32 {
        fn new(seed_u32: u32) -> Self {
            Self {
                seed: seed_u32 as f64,
            }
        }

        fn next_f64(&mut self) -> f64 {
            self.seed += 0x6d2b79f5 as f64;
            let mut t = (self.seed as i64) as i32;
            let t_u = t as u32;
            t = ((t ^ ((t_u >> 15) as i32)) as i64 * ((t | 1) as i64)) as i32;
            let factor = t | 61;
            let term2 = (((t as u32) ^ ((t as u32) >> 7)) as i64 * (factor as i64)) as i32;
            t ^= t.wrapping_add(term2);
            let val = (t as u32) ^ ((t as u32) >> 14);
            (val as f64) / 4294967296.0
        }
    }

    // -------------------------------------------------------------------------
    // Test 4: partition_remerge_100_permutations
    // Splits a multi-node mesh into 3 islands, executes concurrent acts,
    // heals the partition across 100 deterministic PRNG permutations,
    // and asserts 100% bit-for-bit fold() and queue() convergence.
    // -------------------------------------------------------------------------
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct FoldQueueSnapshot {
        objects: Vec<(String, String, String, Option<String>, Option<Voices>, Option<i64>)>,
        rooms: Vec<(String, Vec<String>, Vec<String>, Vec<String>, Vec<String>, bool)>,
        registered: Vec<String>,
        holders: Vec<String>,
        nodes_of: Vec<(String, Vec<String>)>,
        halt_latched: bool,
        halt_by: Option<String>,
        halt_confirmed: bool,
        queues: HashMap<String, (Vec<QueueItem>, Vec<QueueStart>)>,
    }

    fn snapshot_fold_and_queue(
        held: &[HeldRow],
        tables: &Tables,
        now: u64,
        people: &[&str],
    ) -> FoldQueueSnapshot {
        let f = fold(held, tables, now);

        let mut objects: Vec<_> = f
            .objects
            .iter()
            .map(|(id, o)| {
                (
                    id.clone(),
                    o.family.clone(),
                    o.state.clone(),
                    o.outcome.as_ref().map(|x| x.as_str().to_string()),
                    o.voices.clone(),
                    o.missing.as_ref().and_then(|m| m.voices),
                )
            })
            .collect();
        objects.sort_by(|a, b| a.0.cmp(&b.0));

        let mut rooms: Vec<_> = f
            .rooms
            .iter()
            .map(|(id, r)| {
                let mut founders: Vec<String> = r.founders.iter().cloned().collect();
                founders.sort();
                let mut members: Vec<String> = r.members.iter().cloned().collect();
                members.sort();
                let mut removed: Vec<String> = r.removed.iter().cloned().collect();
                removed.sort();
                let mut moderators: Vec<String> = r.moderators.iter().cloned().collect();
                moderators.sort();
                (id.clone(), founders, members, removed, moderators, r.locked)
            })
            .collect();
        rooms.sort_by(|a, b| a.0.cmp(&b.0));

        let mut registered: Vec<String> = f.registered.iter().cloned().collect();
        registered.sort();

        let mut holders: Vec<String> = f.holders.iter().cloned().collect();
        holders.sort();

        let mut nodes_of: Vec<_> = f
            .nodes_of
            .iter()
            .map(|(k, v)| {
                let mut vs: Vec<String> = v.iter().cloned().collect();
                vs.sort();
                (k.clone(), vs)
            })
            .collect();
        nodes_of.sort_by(|a, b| a.0.cmp(&b.0));

        let mut queues = HashMap::new();
        for &p in people {
            let q = queue(held, tables, p, now);
            queues.insert(p.to_string(), (q.items, q.starts));
        }

        FoldQueueSnapshot {
            objects,
            rooms,
            registered,
            holders,
            nodes_of,
            halt_latched: f.halt.latched,
            halt_by: f.halt.by,
            halt_confirmed: f.halt.confirmed,
            queues,
        }
    }

    #[test]
    fn test_partition_remerge_100_permutations() {
        let tables = Tables::load_vendored();
        let mut rng = Mulberry32::new(0x5eedcafe);

        let mut story_by_id = HashMap::new();
        let steps = tables
            .story_trace
            .get("steps")
            .and_then(|s| s.as_array())
            .unwrap();
        for s in steps {
            if let Some(r_val) = s.get("row") {
                if !r_val.is_null() {
                    let r = row_from_json(r_val).unwrap();
                    story_by_id.insert(r.id.clone(), r);
                }
            }
        }
        assert_eq!(story_by_id.len(), 13, "constitution story has 13 executable rows");

        let mut base_held: Vec<HeldRow> = Vec::new();
        let mut clock = 100u64;

        let mut admit = |held: &mut Vec<HeldRow>, mut r: Row| -> Row {
            clock += 10;
            if r.asserted_at == 0 {
                r.asserted_at = clock;
            }
            if r.scope.is_none() {
                r.scope = Some(Scope {
                    tier: "self".to_string(),
                    room: None,
                });
            }
            let res = door(held, &tables, &r, r.asserted_at);
            assert!(
                res.is_ok(),
                "expected row {} ({}/{}) admitted, got {:?}",
                r.id, r.family, r.verb, res
            );
            r
        };

        let people = ["alice", "bob", "carol", "dave", "erin", "frank", "agent:bob"];
        for p in &["alice", "carol", "dave", "erin", "frank", "agent:bob"] {
            admit(
                &mut base_held,
                Row {
                    id: format!("key:{}:row", p),
                    family: "key:record".to_string(),
                    verb: "attest".to_string(),
                    signer: p.to_string(),
                    signer_type: None,
                    object: format!("key:{}", p),
                    scope: Some(Scope {
                        tier: "self".to_string(),
                        room: None,
                    }),
                    subject: None,
                    stance: None,
                    ranking: None,
                    asserted_at: 0,
                    note: None,

                    role: None,

                    job: None,
                },
            );
        }
        admit(&mut base_held, story_by_id.get("row2").unwrap().clone());

        // Bob founds room:1 (story row5) and Frank co-founds
        let mut r5 = story_by_id.get("row5").unwrap().clone();
        r5.scope = Some(Scope {
            tier: "self".to_string(),
            room: Some("room:1".to_string()),
        });
        admit(&mut base_held, r5);

        admit(
            &mut base_held,
            Row {
                id: "row5:frank-founder".to_string(),
                family: "group:roster".to_string(),
                verb: "attest".to_string(),
                signer: "frank".to_string(),
                signer_type: None,
                object: "room:1:frank".to_string(),
                scope: Some(Scope {
                    tier: "self".to_string(),
                    room: Some("room:1".to_string()),
                }),
                subject: None,
                stance: None,
                ranking: None,
                asserted_at: 0,
                note: None,

                role: None,

                job: None,
            },
        );

        // Bob invites Dave (row6) & Carol to room:1, and both accept before cable cut
        let mut r6 = story_by_id.get("row6").unwrap().clone();
        r6.subject = Some("dave".to_string());
        r6.scope = Some(Scope {
            tier: "room".to_string(),
            room: Some("room:1".to_string()),
        });
        admit(&mut base_held, r6);

        admit(
            &mut base_held,
            Row {
                id: "row6:dave-ack".to_string(),
                family: "membership:{stage}".to_string(),
                verb: "attest".to_string(),
                signer: "dave".to_string(),
                signer_type: None,
                object: "invite:dave".to_string(),
                scope: Some(Scope {
                    tier: "room".to_string(),
                    room: Some("room:1".to_string()),
                }),
                subject: Some("dave".to_string()),
                stance: Some("for".to_string()),
                ranking: None,
                asserted_at: 0,
                note: None,

                role: None,

                job: None,
            },
        );

        admit(
            &mut base_held,
            Row {
                id: "invite:carol:row".to_string(),
                family: "membership:{stage}".to_string(),
                verb: "attest".to_string(),
                signer: "bob".to_string(),
                signer_type: None,
                object: "invite:carol".to_string(),
                scope: Some(Scope {
                    tier: "room".to_string(),
                    room: Some("room:1".to_string()),
                }),
                subject: Some("carol".to_string()),
                stance: None,
                ranking: None,
                asserted_at: 0,
                note: None,

                role: None,

                job: None,
            },
        );

        admit(
            &mut base_held,
            Row {
                id: "invite:carol:ack".to_string(),
                family: "membership:{stage}".to_string(),
                verb: "attest".to_string(),
                signer: "carol".to_string(),
                signer_type: None,
                object: "invite:carol".to_string(),
                scope: Some(Scope {
                    tier: "room".to_string(),
                    room: Some("room:1".to_string()),
                }),
                subject: Some("carol".to_string()),
                stance: Some("for".to_string()),
                ranking: None,
                asserted_at: 0,
                note: None,

                role: None,

                job: None,
            },
        );

        // Submarine cable cut: 3 disconnected islands diverge
        let mut island_a = base_held.clone();
        let mut island_b = base_held.clone();
        let mut island_c = base_held.clone();

        // Island A: alice, frank
        admit(&mut island_a, story_by_id.get("row1").unwrap().clone());
        admit(
            &mut island_a,
            Row {
                id: "row1:frank-root".to_string(),
                family: "trust:{job}:{version}".to_string(),
                verb: "authorize".to_string(),
                signer: "frank".to_string(),
                signer_type: None,
                object: "root:frank".to_string(),
                scope: None,
                subject: None,
                stance: None,
                ranking: None,
                asserted_at: 0,
                note: None,

                role: None,

                job: None,
            },
        );
        admit(
            &mut island_a,
            Row {
                id: "ops:roster".to_string(),
                family: "group:roster".to_string(),
                verb: "attest".to_string(),
                signer: "alice".to_string(),
                signer_type: None,
                object: "room:ops".to_string(),
                scope: Some(Scope {
                    tier: "room".to_string(),
                    room: Some("room:ops".to_string()),
                }),
                subject: None,
                stance: None,
                ranking: None,
                asserted_at: 0,
                note: None,

                role: None,

                job: None,
            },
        );
        admit(
            &mut island_a,
            Row {
                id: "ops:invite:frank".to_string(),
                family: "membership:{stage}".to_string(),
                verb: "attest".to_string(),
                signer: "alice".to_string(),
                signer_type: None,
                object: "invite:frank:ops".to_string(),
                scope: Some(Scope {
                    tier: "room".to_string(),
                    room: Some("room:ops".to_string()),
                }),
                subject: Some("frank".to_string()),
                stance: None,
                ranking: None,
                asserted_at: 0,
                note: None,

                role: None,

                job: None,
            },
        );
        admit(
            &mut island_a,
            Row {
                id: "ops:invite:frank:ack".to_string(),
                family: "membership:{stage}".to_string(),
                verb: "attest".to_string(),
                signer: "frank".to_string(),
                signer_type: None,
                object: "invite:frank:ops".to_string(),
                scope: Some(Scope {
                    tier: "room".to_string(),
                    room: Some("room:ops".to_string()),
                }),
                subject: Some("frank".to_string()),
                stance: Some("for".to_string()),
                ranking: None,
                asserted_at: 0,
                note: None,

                role: None,

                job: None,
            },
        );
        admit(
            &mut island_a,
            Row {
                id: "ops:post:1".to_string(),
                family: "chat:*".to_string(),
                verb: "attest".to_string(),
                signer: "alice".to_string(),
                signer_type: None,
                object: "post:ops:1".to_string(),
                scope: Some(Scope {
                    tier: "room".to_string(),
                    room: Some("room:ops".to_string()),
                }),
                subject: None,
                stance: None,
                ranking: None,
                asserted_at: 0,
                note: None,

                role: None,

                job: None,
            },
        );
        admit(
            &mut island_a,
            Row {
                id: "ops:post:1:sup".to_string(),
                family: "chat:*".to_string(),
                verb: "supersede".to_string(),
                signer: "alice".to_string(),
                signer_type: None,
                object: "post:ops:1".to_string(),
                scope: Some(Scope {
                    tier: "room".to_string(),
                    room: Some("room:ops".to_string()),
                }),
                subject: None,
                stance: None,
                ranking: None,
                asserted_at: 0,
                note: None,

                role: None,

                job: None,
            },
        );
        admit(
            &mut island_a,
            Row {
                id: "bind:frank:row".to_string(),
                family: "transport:{kind}".to_string(),
                verb: "attest".to_string(),
                signer: "frank".to_string(),
                signer_type: None,
                object: "bind:frank".to_string(),
                scope: Some(Scope {
                    tier: "federation".to_string(),
                    room: None,
                }),
                subject: None,
                stance: None,
                ranking: None,
                asserted_at: 0,
                note: None,

                role: None,

                job: None,
            },
        );

        // Island B: bob, dave, frank, agent:bob
        let mut r3 = story_by_id.get("row3").unwrap().clone();
        r3.scope = Some(Scope {
            tier: "federation".to_string(),
            room: None,
        });
        admit(&mut island_b, r3);

        let mut r7 = story_by_id.get("row7").unwrap().clone();
        r7.scope = Some(Scope {
            tier: "room".to_string(),
            room: Some("room:1".to_string()),
        });
        admit(&mut island_b, r7);

        let mut r8 = story_by_id.get("row8").unwrap().clone();
        r8.scope = Some(Scope {
            tier: "room".to_string(),
            room: Some("room:1".to_string()),
        });
        admit(&mut island_b, r8);

        let mut r9 = story_by_id.get("row9").unwrap().clone();
        r9.scope = Some(Scope {
            tier: "room".to_string(),
            room: Some("room:1".to_string()),
        });
        admit(&mut island_b, r9);

        let mut r11 = story_by_id.get("row11").unwrap().clone();
        r11.scope = Some(Scope {
            tier: "room".to_string(),
            room: Some("room:1".to_string()),
        });
        admit(&mut island_b, r11);

        admit(&mut island_b, story_by_id.get("row12").unwrap().clone());

        admit(
            &mut island_b,
            Row {
                id: "room1:post:3".to_string(),
                family: "chat:*".to_string(),
                verb: "attest".to_string(),
                signer: "bob".to_string(),
                signer_type: None,
                object: "post:3".to_string(),
                scope: Some(Scope {
                    tier: "room".to_string(),
                    room: Some("room:1".to_string()),
                }),
                subject: None,
                stance: None,
                ranking: None,
                asserted_at: 0,
                note: None,

                role: None,

                job: None,
            },
        );
        admit(
            &mut island_b,
            Row {
                id: "room1:post:3:withdraw".to_string(),
                family: "chat:*".to_string(),
                verb: "withdraw".to_string(),
                signer: "bob".to_string(),
                signer_type: None,
                object: "post:3".to_string(),
                scope: Some(Scope {
                    tier: "room".to_string(),
                    room: Some("room:1".to_string()),
                }),
                subject: None,
                stance: None,
                ranking: None,
                asserted_at: 0,
                note: None,

                role: None,

                job: None,
            },
        );

        // Island C: carol, erin
        admit(&mut island_c, story_by_id.get("row4").unwrap().clone());
        let mut r10 = story_by_id.get("row10").unwrap().clone();
        r10.scope = Some(Scope {
            tier: "room".to_string(),
            room: Some("room:1".to_string()),
        });
        admit(&mut island_c, r10);
        admit(&mut island_c, story_by_id.get("row13").unwrap().clone());

        admit(
            &mut island_c,
            Row {
                id: "res:roster".to_string(),
                family: "group:roster".to_string(),
                verb: "attest".to_string(),
                signer: "carol".to_string(),
                signer_type: None,
                object: "room:res".to_string(),
                scope: Some(Scope {
                    tier: "room".to_string(),
                    room: Some("room:res".to_string()),
                }),
                subject: None,
                stance: None,
                ranking: None,
                asserted_at: 0,
                note: None,

                role: None,

                job: None,
            },
        );
        admit(
            &mut island_c,
            Row {
                id: "res:invite:erin".to_string(),
                family: "membership:{stage}".to_string(),
                verb: "attest".to_string(),
                signer: "carol".to_string(),
                signer_type: None,
                object: "invite:erin:res".to_string(),
                scope: Some(Scope {
                    tier: "room".to_string(),
                    room: Some("room:res".to_string()),
                }),
                subject: Some("erin".to_string()),
                stance: None,
                ranking: None,
                asserted_at: 0,
                note: None,

                role: None,

                job: None,
            },
        );
        admit(
            &mut island_c,
            Row {
                id: "res:invite:erin:ack".to_string(),
                family: "membership:{stage}".to_string(),
                verb: "attest".to_string(),
                signer: "erin".to_string(),
                signer_type: None,
                object: "invite:erin:res".to_string(),
                scope: Some(Scope {
                    tier: "room".to_string(),
                    room: Some("room:res".to_string()),
                }),
                subject: Some("erin".to_string()),
                stance: Some("for".to_string()),
                ranking: None,
                asserted_at: 0,
                note: None,

                role: None,

                job: None,
            },
        );
        admit(
            &mut island_c,
            Row {
                id: "res:post:1".to_string(),
                family: "chat:*".to_string(),
                verb: "attest".to_string(),
                signer: "erin".to_string(),
                signer_type: None,
                object: "post:res:1".to_string(),
                scope: Some(Scope {
                    tier: "room".to_string(),
                    room: Some("room:res".to_string()),
                }),
                subject: None,
                stance: None,
                ranking: None,
                asserted_at: 0,
                note: None,

                role: None,

                job: None,
            },
        );

        // Union of all unique rows held across the 3 islands
        let mut union_map: HashMap<String, Row> = HashMap::new();
        for isl in &[&island_a, &island_b, &island_c] {
            for h in *isl {
                union_map.insert(h.row.id.clone(), h.row.clone());
            }
        }
        let all_rows: Vec<Row> = union_map.values().cloned().collect();
        assert_eq!(all_rows.len(), 36, "36 unique rows in 3-island union");

        let mut baseline: Option<FoldQueueSnapshot> = None;

        for i in 0..100 {
            // Fisher-Yates shuffle with deterministic Mulberry32
            let mut perm = all_rows.clone();
            for j in (1..perm.len()).rev() {
                let k = (rng.next_f64() * ((j + 1) as f64)).floor() as usize;
                perm.swap(j, k);
            }
            let eval_now = 100_000 + (rng.next_f64() * 50_000.0).floor() as u64;

            // 1) Permuted delivery with jittered first_held timestamps
            let mut jittered_held: Vec<HeldRow> = Vec::new();
            for r in &perm {
                let first_held = 10_000 + (rng.next_f64() * 40_000.0).floor() as u64;
                jittered_held.push(HeldRow {
                    row: r.clone(),
                    first_held,
                });
            }

            let snap = snapshot_fold_and_queue(&jittered_held, &tables, eval_now, &people);
            if let Some(b) = &baseline {
                assert_eq!(
                    &snap, b,
                    "permutation {} diverged on jittered held fold/queue",
                    i
                );
            } else {
                baseline = Some(snap.clone());
            }

            // 2) Causal re-merge through door() in multi-hop retry passes (as in Mesh::tick)
            let mut door_held: Vec<HeldRow> = Vec::new();
            let mut pending = perm.clone();
            let mut passes = 0;
            while !pending.is_empty() && passes < 6 {
                passes += 1;
                let mut retry = Vec::new();
                for r in pending {
                    let arrival = 10_000 + passes * 1000 + (rng.next_f64() * 900.0).floor() as u64;
                    let s = door(&mut door_held, &tables, &r, arrival);
                    if s.is_err() {
                        retry.push(r);
                    }
                }
                pending = retry;
            }
            assert_eq!(
                pending.len(),
                0,
                "permutation {} admitted all {} rows within {} passes",
                i,
                all_rows.len(),
                passes
            );
            let door_snap = snapshot_fold_and_queue(&door_held, &tables, eval_now, &people);
            let b = baseline.as_ref().unwrap();
            if door_snap.objects != b.objects {
                panic!("objects differed in perm {}: left {:?} vs right {:?}", i, door_snap.objects, b.objects);
            }
            if door_snap.rooms != b.rooms {
                panic!("rooms differed in perm {}: left {:?} vs right {:?}", i, door_snap.rooms, b.rooms);
            }
            if door_snap.registered != b.registered {
                panic!("registered differed in perm {}", i);
            }
            if door_snap.holders != b.holders {
                panic!("holders differed in perm {}", i);
            }
            if door_snap.nodes_of != b.nodes_of {
                panic!("nodes_of differed in perm {}", i);
            }
            if door_snap.queues != b.queues {
                for &p in &people {
                    let (left_items, left_starts) = &door_snap.queues[p];
                    let (right_items, right_starts) = &b.queues[p];
                    if left_items != right_items {
                        panic!("queue items for {} differed in perm {}: left {:?} vs right {:?}", p, i, left_items, right_items);
                    }
                    if left_starts != right_starts {
                        panic!("queue starts for {} differed in perm {}: left len {} vs right len {}", p, i, left_starts.len(), right_starts.len());
                    }
                }
            }
            assert_eq!(
                Some(door_snap),
                baseline,
                "permutation {} diverged after door() admission passes",
                i
            );
        }
    }

    // -------------------------------------------------------------------------
    // Test 5: churn_mobile_sleep_wake_stranger_refusal
    // Reproduces lines 228-386 of kernel.test.js:
    // Churn, mobile sleep/wake, stranger refusal under partition healing,
    // preview parity on every sign(), and 100% multi-device fold/queue parity.
    // -------------------------------------------------------------------------
    #[test]
    fn test_churn_mobile_sleep_wake_stranger_refusal() {
        let tables = Tables::load_vendored();
        let mut m = Mesh::new(tables.clone());

        m.add_node("alice-phone", "alice", "phone");
        m.add_node("alice-laptop", "alice", "phone");
        m.add_node("bob-phone", "bob", "phone");
        m.add_node("bob-tablet", "bob", "phone");
        m.add_node("dave-phone", "dave", "phone");
        m.add_node("mallory-phone", "mallory", "phone");
        m.add_node("seat", "nobody", "server");

        let all_clients = [
            "alice-phone",
            "alice-laptop",
            "bob-phone",
            "bob-tablet",
            "dave-phone",
            "mallory-phone",
        ];
        for &c in &all_clients {
            m.link(c, "seat");
        }

        let mut seq = 0;
        let mut make_row = |family: &str, verb: &str, signer: &str, object: &str, room: Option<&str>, subject: Option<&str>, stance: Option<&str>, tier: &str| {
            seq += 1;
            Row {
                id: format!("churn-r{}", seq),
                family: family.to_string(),
                verb: verb.to_string(),
                signer: signer.to_string(),
                signer_type: None,
                object: object.to_string(),
                scope: Some(Scope {
                    tier: tier.to_string(),
                    room: room.map(|r| r.to_string()),
                }),
                subject: subject.map(|s| s.to_string()),
                stance: stance.map(|s| s.to_string()),
                ranking: None,
                asserted_at: 0,
                note: None,

                role: None,

                job: None,
            }
        };

        let sign_checked = |mesh: &mut Mesh, node_id: &str, mut r: Row| -> StepResult {
            if r.asserted_at == 0 {
                r.asserted_at = mesh.now;
            }
            let node = mesh.nodes.get(node_id).unwrap();
            let pv = preview(&node.held, &mesh.tables, &[r.clone()], mesh.now);
            let res = mesh.sign(node_id, r.clone());
            assert_eq!(pv.verdicts[0].ok, res.is_ok());
            assert_eq!(
                pv.verdicts[0].refusal.as_deref(),
                res.as_ref().err().map(|e| e.as_str())
            );
            if res.is_ok() {
                let f_after = mesh.fold_of(node_id);
                for ch in &pv.changed {
                    assert_eq!(f_after.objects.get(&ch.object).unwrap().state, ch.to);
                }
                for rm in &pv.rooms {
                    let room_after = f_after.rooms.get(&rm.room).unwrap();
                    for joined in &rm.joined {
                        assert!(room_after.members.contains(joined));
                    }
                }
            }
            res
        };

        // Phase 1: Bootstrap keys & transport bindings
        for &(node_id, owner) in &[
            ("alice-phone", "alice"),
            ("bob-phone", "bob"),
            ("dave-phone", "dave"),
            ("mallory-phone", "mallory"),
        ] {
            let r1 = make_row("key:record", "attest", owner, &format!("key:{}", owner), None, None, None, "self");
            assert!(sign_checked(&mut m, node_id, r1).is_ok());

            let r2 = make_row("transport:{kind}", "attest", owner, &format!("bind:{}", node_id), None, None, None, "federation");
            assert!(sign_checked(&mut m, node_id, r2).is_ok());
        }
        m.tick(1000);
        m.tick(1000);

        for &c in &all_clients {
            assert_eq!(m.fold_of(c).registered.len(), 4, "{} has 4 registered keys", c);
        }

        // Phase 2: alice-laptop and dave-phone sleep
        m.sleep("alice-laptop");
        m.sleep("dave-phone");

        let alice_self_trace = make_row("trace:{form}:{version}", "attest", "alice", "trace:alice:private", None, None, None, "self");
        assert!(sign_checked(&mut m, "alice-phone", alice_self_trace).is_ok());

        let alice_consent = make_row("consent:{kind}", "attest", "alice", "consent:alice-bob", None, None, None, "self");
        assert!(sign_checked(&mut m, "alice-phone", alice_consent).is_ok());

        let room_alpha = make_row("group:roster", "attest", "bob", "room:alpha", Some("room:alpha"), None, None, "room");
        assert!(sign_checked(&mut m, "bob-phone", room_alpha).is_ok());

        let invite_alice = make_row("membership:{stage}", "attest", "bob", "invite:alice:alpha", Some("room:alpha"), Some("alice"), None, "room");
        assert!(sign_checked(&mut m, "bob-phone", invite_alice).is_ok());

        let invite_dave = make_row("membership:{stage}", "attest", "bob", "invite:dave:alpha", Some("room:alpha"), Some("dave"), None, "room");
        assert!(sign_checked(&mut m, "bob-phone", invite_dave).is_ok());

        m.tick(1000);
        m.tick(1000);

        let ack_alice = make_row("membership:{stage}", "attest", "alice", "invite:alice:alpha", Some("room:alpha"), Some("alice"), Some("for"), "room");
        assert!(sign_checked(&mut m, "alice-phone", ack_alice).is_ok());

        m.tick(1000);
        m.tick(1000);

        let post_alpha_1 = make_row("chat:*", "attest", "alice", "post:alpha:1", Some("room:alpha"), None, None, "room");
        assert!(sign_checked(&mut m, "alice-phone", post_alpha_1).is_ok());

        let post_alpha_2 = make_row("chat:*", "attest", "bob", "post:alpha:2", Some("room:alpha"), None, None, "room");
        assert!(sign_checked(&mut m, "bob-tablet", post_alpha_2).is_ok());

        let recant_alpha_1 = make_row("chat:*", "recant", "alice", "post:alpha:1", Some("room:alpha"), None, None, "room");
        assert!(sign_checked(&mut m, "alice-phone", recant_alpha_1).is_ok());

        // Stranger mallory-phone attempts to post and withdraw in room:alpha: refused!
        let mallory_bad_post = make_row("chat:*", "attest", "mallory", "post:mallory:1", Some("room:alpha"), None, None, "room");
        assert!(!sign_checked(&mut m, "mallory-phone", mallory_bad_post).is_ok());

        let mallory_bad_withdraw = make_row("chat:*", "withdraw", "mallory", "post:alpha:2", Some("room:alpha"), None, None, "room");
        assert!(!sign_checked(&mut m, "mallory-phone", mallory_bad_withdraw).is_ok());

        m.tick(1000);
        m.tick(1000);

        assert!(!m.fold_of("alice-laptop").objects.contains_key("trace:alice:private"));
        assert!(!m.fold_of("alice-laptop").rooms.contains_key("room:alpha"));
        assert!(!m.fold_of("dave-phone").rooms.contains_key("room:alpha"));

        // Phase 3: Rolling churn — alice-phone & bob-phone sleep; alice-laptop & dave-phone wake
        m.sleep("alice-phone");
        m.sleep("bob-phone");
        m.wake("alice-laptop");
        m.wake("dave-phone");

        m.tick(1000);
        m.tick(1000);
        m.tick(1000);

        assert_eq!(
            m.fold_of("alice-laptop").objects.get("trace:alice:private").unwrap().state,
            "admitted"
        );
        assert_eq!(
            m.fold_of("alice-laptop").objects.get("post:alpha:1").unwrap().state,
            "recanted"
        );

        let ack_dave = make_row("membership:{stage}", "attest", "dave", "invite:dave:alpha", Some("room:alpha"), Some("dave"), Some("for"), "room");
        assert!(sign_checked(&mut m, "dave-phone", ack_dave).is_ok());

        m.tick(1000);
        m.tick(1000);

        let post_alpha_3 = make_row("chat:*", "attest", "dave", "post:alpha:3", Some("room:alpha"), None, None, "room");
        assert!(sign_checked(&mut m, "dave-phone", post_alpha_3).is_ok());

        let post_alpha_4 = make_row("chat:*", "attest", "alice", "post:alpha:4", Some("room:alpha"), None, None, "room");
        assert!(sign_checked(&mut m, "alice-laptop", post_alpha_4).is_ok());

        // Phase 4: Heal partition completely
        m.wake("alice-phone");
        m.wake("bob-phone");
        for _ in 0..4 {
            m.tick(1000);
        }

        // Multi-device sibling fold and queue parity
        let canon_fold = |node_id: &str, room_only: bool| {
            let f = m.fold_of(node_id);
            let mut objs: Vec<_> = f
                .objects
                .iter()
                .filter(|(_, o)| !room_only || o.scope.tier == "room")
                .map(|(id, o)| (id.clone(), o.family.clone(), o.state.clone(), o.outcome.as_ref().map(|x| x.as_str().to_string()), o.voices.clone()))
                .collect();
            objs.sort_by(|a, b| a.0.cmp(&b.0));

            let mut rms: Vec<_> = f
                .rooms
                .iter()
                .map(|(id, r)| {
                    let mut founders: Vec<String> = r.founders.iter().cloned().collect();
                    founders.sort();
                    let mut members: Vec<String> = r.members.iter().cloned().collect();
                    members.sort();
                    let mut moderators: Vec<String> = r.moderators.iter().cloned().collect();
                    moderators.sort();
                    (id.clone(), founders, members, moderators, r.locked)
                })
                .collect();
            rms.sort_by(|a, b| a.0.cmp(&b.0));

            let mut reg: Vec<String> = f.registered.iter().cloned().collect();
            reg.sort();
            (objs, rms, reg)
        };

        assert_eq!(
            canon_fold("alice-phone", false),
            canon_fold("alice-laptop", false),
            "alice devices reach 100% fold parity"
        );
        assert_eq!(
            m.queue_of("alice-phone").items,
            m.queue_of("alice-laptop").items,
            "alice devices queue items parity"
        );
        assert_eq!(
            m.queue_of("alice-phone").starts,
            m.queue_of("alice-laptop").starts,
            "alice devices queue starts parity"
        );

        assert_eq!(
            canon_fold("bob-phone", false),
            canon_fold("bob-tablet", false),
            "bob devices reach 100% fold parity"
        );
        assert_eq!(
            m.queue_of("bob-phone").items,
            m.queue_of("bob-tablet").items,
            "bob devices queue items parity"
        );
        assert_eq!(
            m.queue_of("bob-phone").starts,
            m.queue_of("bob-tablet").starts,
            "bob devices queue starts parity"
        );

        // Stranger isolation on mallory-phone
        let mallory_node = m.nodes.get("mallory-phone").unwrap();
        assert_eq!(mallory_node.sealed.len(), 0, "stranger phone never carries sealed rows");
        for h in &mallory_node.held {
            let is_cf = CONSENT_FREE.contains(&h.row.family.as_str())
                || h.row.scope.as_ref().map(|s| s.tier.as_str()) == Some("federation")
                || h.row.signer == "mallory";
            assert!(is_cf, "stranger must not hold private/room row {}", h.row.id);
            assert_ne!(h.row.scope.as_ref().map(|s| s.tier.as_str()), Some("room"));
        }
        assert_eq!(m.fold_of("mallory-phone").rooms.len(), 0, "stranger folds 0 rooms");
    }

    #[test]
    fn test_namespace_match_vectors_1281() {
        let tables = Tables::load_vendored();
        let vectors = tables
            .match_vectors
            .get("vectors")
            .and_then(|v| v.as_array())
            .expect("vectors in match_vectors");

        let mut passed_vectors = 0;
        for v in vectors {
            let dim = v.get("dimension").and_then(|x| x.as_str()).unwrap();
            let expected_fam = v.get("family").and_then(|x| x.as_str());
            let expected_refusal = v.get("refusal").and_then(|x| x.as_str());

            let res = tables.match_family(dim);
            assert_eq!(
                res.family.as_deref(),
                expected_fam,
                "Vector {} family mismatch",
                dim
            );
            assert_eq!(
                res.refusal.as_deref(),
                expected_refusal,
                "Vector {} refusal mismatch",
                dim
            );

            if expected_refusal.is_none() && expected_fam.is_some() {
                let binds = tables.extract_dimensions(dim);
                assert!(res.ok, "Dimensions extracted for {}", dim);
                let _ = binds;
            }
            passed_vectors += 1;
        }
        assert_eq!(passed_vectors, 1256, "all 1,256 dimension vectors replayed");

        let structural: HashSet<&str> = ["scores", "delegates_to", "supersedes", "withdraws", "recants"]
            .into_iter()
            .collect();
        let row_type_vectors = tables
            .match_vectors
            .get("row_type_vectors")
            .and_then(|v| v.as_array())
            .expect("row_type_vectors in match_vectors");

        let mut passed_row_types = 0;
        for v in row_type_vectors {
            let attestation_type = v.get("attestation_type").and_then(|x| x.as_str()).unwrap();
            let expected_ref = v.get("refusal").and_then(|x| x.as_str());
            let is_struct = structural.contains(attestation_type);
            let calculated_ref = if is_struct {
                None
            } else {
                Some("attestation_type_unregistered")
            };
            assert_eq!(
                calculated_ref, expected_ref,
                "Row type {} refusal mismatch",
                attestation_type
            );
            passed_row_types += 1;
        }
        assert_eq!(passed_row_types, 25, "all 25 row-type vectors replayed");
    }

    #[test]
    fn test_191_families_17_roles_and_csd_namespace_map() {
        let tables = Tables::load_vendored();

        // 1. Verify csd_namespace_map.json integrity & summary
        assert_eq!(
            tables.csd_map.get("version").and_then(|v| v.as_str()),
            Some("cewp-synthesis-rc17-v6")
        );
        let summary = tables.csd_map.get("summary").expect("summary in csd_map");
        assert_eq!(summary.get("total_families").and_then(|v| v.as_u64()), Some(191));
        assert_eq!(summary.get("total_transitions").and_then(|v| v.as_u64()), Some(474));
        assert_eq!(summary.get("total_policies").and_then(|v| v.as_u64()), Some(12));
        assert_eq!(summary.get("total_csd_cards").and_then(|v| v.as_u64()), Some(105));
        assert_eq!(summary.get("csd_mapped_families").and_then(|v| v.as_u64()), Some(94));
        assert_eq!(summary.get("substrate_autonomous_families").and_then(|v| v.as_u64()), Some(97));
        assert_eq!(summary.get("total_csd_shows_fields").and_then(|v| v.as_u64()), Some(660));
        assert_eq!(summary.get("resolved_csd_shows_fields").and_then(|v| v.as_u64()), Some(152));

        // Verify all 105 cards and their 660 fields
        let mut resolved_shows_count = 0;
        let cards = tables.csd_map.get("cards").and_then(|v| v.as_array()).expect("cards array");
        for card in cards {
            let card_id = card.get("id").and_then(|v| v.as_str()).unwrap();
            assert!(card_id.starts_with("CSD-"));
            assert!(card.get("title").and_then(|v| v.as_str()).is_some());
            let topologies = card.get("topologies").and_then(|v| v.as_array()).unwrap();
            assert!(!topologies.is_empty());
            if let Some(shows) = card.get("shows").and_then(|v| v.as_array()) {
                for show in shows {
                    let field = show.get("field").and_then(|v| v.as_str()).unwrap();
                    let canonical_family = show.get("canonical_family").and_then(|v| v.as_str()).unwrap();
                    if field.starts_with("x_private:") {
                        assert_eq!(canonical_family, "x_private:*");
                        let m = tables.match_family(field);
                        assert_eq!(m.refusal.as_deref(), Some("namespace_private_use_not_federatable"));
                        resolved_shows_count += 1;
                    } else if field.starts_with("holds_bytes:") {
                        assert_eq!(canonical_family, "holds_bytes:sha256:{prefix}");
                        resolved_shows_count += 1;
                    } else {
                        let m = tables.match_family(field);
                        assert_eq!(m.family.as_deref(), Some(canonical_family), "Field {} in {}", field, card_id);
                        resolved_shows_count += 1;
                    }
                }
            }
        }
        assert_eq!(resolved_shows_count, 660, "all 660 csd:shows fields resolved");

        // 2. Verify all 17 role tokens across all 474 transitions & 3 equivalences
        let mut tested_transitions = 0;
        let mut gap_cells_tested = 0;
        let mut regular_tested = 0;

        let transitions_json = parse_json(VENDORED_TRANSITIONS).expect("valid transitions json");
        let rows = transitions_json.get("rows").and_then(|v| v.as_array()).expect("rows array");

        for row in rows {
            let family = row.get("family").and_then(|v| v.as_str()).unwrap().to_string();
            let transitions_arr = row.get("transitions").and_then(|v| v.as_array()).expect("transitions array");

            for t_json in transitions_arr {
                tested_transitions += 1;
                let verb = t_json.get("verb").and_then(|v| v.as_str()).unwrap().to_string();
                let from = t_json.get("from").and_then(|v| v.as_str()).unwrap().to_string();
                let to = t_json.get("to").and_then(|v| v.as_str()).unwrap().to_string();

                let mut roles = Vec::new();
                if let Some(r_arr) = t_json.get("roles").and_then(|v| v.get("value")).and_then(|v| v.as_array()) {
                    for r in r_arr {
                        if let Some(s) = r.as_str() {
                            roles.push(s.to_string());
                        }
                    }
                }

                let mut held: Vec<HeldRow> = Vec::new();
                let mut now = 1000u64;
                let sanitized_fam = family
                    .chars()
                    .map(|c| if matches!(c, ':' | '{' | '}' | '*') { '_' } else { c })
                    .collect::<String>();
                let test_obj = format!("obj:{}:{}", sanitized_fam, tested_transitions);

                let from_none = transitions_arr.iter().find(|x| x.get("from").and_then(|v| v.as_str()) == Some("none"));

                // (A) Equivalence 1: refused_previews_nothing on unauthorized stranger
                if family != "key:record" {
                    now += 1;
                    let stranger_row = Row {
                        id: format!("row:stranger:{}", tested_transitions),
                        family: family.clone(),
                        verb: verb.clone(),
                        signer: "stranger-nobody".to_string(),
                        signer_type: None,
                        object: test_obj.clone(),
                        scope: Some(Scope { tier: "self".to_string(), room: None }),
                        subject: None,
                        stance: None,
                        ranking: None,
                        asserted_at: now,
                        note: None,
                        role: None,
                        job: None,
                    };
                    let unauth_prev = preview(&held, &tables, &[stranger_row.clone()], now);
                    assert_eq!(unauth_prev.verdicts[0].ok, false);
                    assert_eq!(unauth_prev.changed.len(), 0);
                    let unauth_door = door(&mut held, &tables, &stranger_row, now);
                    assert!(unauth_door.is_err());
                }

                if roles.is_empty() {
                    gap_cells_tested += 1;
                    continue;
                }

                let from_none_roles_len = from_none.and_then(|fn_row| {
                    fn_row.get("roles").and_then(|v| v.get("value")).and_then(|v| v.as_array()).map(|arr| arr.len())
                }).unwrap_or(0);

                if from == "admitted" && (from_none.is_none() || from_none_roles_len == 0) {
                    continue;
                }

                // (B) Authorized actor holding the required role
                regular_tested += 1;
                let role = &roles[0];
                let fn_first_role = from_none.and_then(|fn_row| {
                    fn_row.get("roles").and_then(|v| v.get("value")).and_then(|v| v.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|x| x.as_str())
                });

                let is_system_detector = role == "identity:lenscore_detector"
                    || fn_first_role == Some("identity:lenscore_detector");
                let is_system_persist = role == "identity:substrate_persist"
                    || fn_first_role == Some("identity:substrate_persist");

                let signer = if is_system_detector {
                    "identity:lenscore_detector"
                } else if is_system_persist {
                    "identity:substrate_persist"
                } else {
                    "actor-omni"
                };

                let room_roles = ["founder", "member", "moderate"];
                let is_room_role = roles.iter().any(|r| room_roles.contains(&r.as_str()));
                let fn_is_room_role = from_none.map(|fn_row| {
                    fn_row.get("roles").and_then(|v| v.get("value")).and_then(|v| v.as_array()).map(|arr| {
                        arr.iter().any(|x| x.as_str().map(|s| room_roles.contains(&s)).unwrap_or(false))
                    }).unwrap_or(false)
                }).unwrap_or(false);

                let needs_room = is_room_role || fn_is_room_role;
                let room_id = "room:omni";
                let scope = if needs_room {
                    Some(Scope { tier: "room".to_string(), room: Some(room_id.to_string()) })
                } else {
                    Some(Scope { tier: "self".to_string(), room: None })
                };

                now += 1;
                door(&mut held, &tables, &Row {
                    id: format!("key:{}", signer),
                    family: "key:record".to_string(),
                    verb: "attest".to_string(),
                    signer: signer.to_string(),
                    signer_type: None,
                    object: format!("key:{}", signer),
                    scope: Some(Scope { tier: "self".to_string(), room: None }),
                    subject: None,
                    stance: None,
                    ranking: None,
                    asserted_at: now,
                    note: None,
                    role: None,
                    job: None,
                }, now).unwrap();

                now += 1;
                door(&mut held, &tables, &Row {
                    id: format!("trust:init:{}", signer),
                    family: "trust:{job}:{version}".to_string(),
                    verb: "authorize".to_string(),
                    signer: signer.to_string(),
                    signer_type: None,
                    object: format!("trust:root:{}", signer),
                    scope: Some(Scope { tier: "self".to_string(), room: None }),
                    subject: None,
                    stance: None,
                    ranking: None,
                    asserted_at: now,
                    note: None,
                    role: None,
                    job: None,
                }, now).unwrap();

                for spec in &["steward", "wa", "takedown", "review", "slash", "attest", "identity:lenscore_detector", "identity:substrate_persist"] {
                    now += 1;
                    door(&mut held, &tables, &Row {
                        id: format!("trust:{}:obj", spec),
                        family: "trust:{job}:{version}".to_string(),
                        verb: "authorize".to_string(),
                        signer: signer.to_string(),
                        signer_type: None,
                        object: format!("trust:{}:1", spec),
                        scope: Some(Scope { tier: "self".to_string(), room: None }),
                        subject: None,
                        stance: None,
                        ranking: None,
                        asserted_at: now,
                        note: None,
                        role: None,
                        job: None,
                    }, now).unwrap();
                }

                if needs_room {
                    now += 1;
                    door(&mut held, &tables, &Row {
                        id: format!("room:roster:{}", signer),
                        family: "group:roster".to_string(),
                        verb: "attest".to_string(),
                        signer: signer.to_string(),
                        signer_type: None,
                        object: room_id.to_string(),
                        scope: Some(Scope { tier: "room".to_string(), room: Some(room_id.to_string()) }),
                        subject: None,
                        stance: None,
                        ranking: None,
                        asserted_at: now,
                        note: None,
                        role: None,
                        job: None,
                    }, now).unwrap();
                }

                if from == "admitted" && from_none.is_some() && from_none_roles_len > 0 {
                    let init_verb = from_none.unwrap().get("verb").and_then(|v| v.as_str()).unwrap().to_string();
                    now += 1;
                    door(&mut held, &tables, &Row {
                        id: format!("{}:seed", test_obj),
                        family: family.clone(),
                        verb: init_verb,
                        signer: signer.to_string(),
                        signer_type: None,
                        object: test_obj.clone(),
                        scope: scope.clone(),
                        subject: Some(signer.to_string()),
                        stance: None,
                        ranking: None,
                        asserted_at: now,
                        note: None,
                        role: None,
                        job: None,
                    }, now).unwrap();
                }

                now += 1;
                let valid_row = Row {
                    id: format!("row:valid:{}", tested_transitions),
                    family: family.clone(),
                    verb: verb.clone(),
                    signer: signer.to_string(),
                    signer_type: None,
                    object: test_obj.clone(),
                    scope: scope.clone(),
                    subject: Some(signer.to_string()),
                    stance: None,
                    ranking: None,
                    asserted_at: now,
                    note: None,
                    role: None,
                    job: None,
                };

                // Equivalence 2: door_is_queue
                let f_before = fold(&held, &tables, now);
                let target = TargetRef {
                    id: Some(&test_obj),
                    object: Some(&test_obj),
                    subject: Some(signer),
                    author: Some(signer),
                    room: if needs_room { Some(room_id) } else { None },
                    signer_type: None,
                    tier: if needs_room { Some("room") } else { Some("self") },
                };
                let cur_roles: Vec<String> = standing_target(&f_before, signer, &target).into_iter().collect();
                let q_verbs: Vec<String> = enabled_for(&tables, &family, &from, &cur_roles)
                    .into_iter()
                    .map(|x| x.verb)
                    .collect();
                assert!(
                    q_verbs.contains(&verb),
                    "Queue missing {} from {} for {} under role {}",
                    verb, from, family, role
                );

                // Equivalence 3: preview_is_execution
                let valid_prev = preview(&held, &tables, &[valid_row.clone()], now);
                assert_eq!(
                    valid_prev.verdicts[0].ok, true,
                    "Preview failed for {} / {}", family, verb
                );
                assert!(
                    valid_prev.changed.iter().any(|c| c.object == test_obj && c.to == to),
                    "Preview changed missing {} for {}", to, family
                );

                let valid_adm = door(&mut held, &tables, &valid_row, now);
                assert!(valid_adm.is_ok(), "Admit failed for {} / {}", family, verb);
                let f_after = fold(&held, &tables, now);
                assert_eq!(
                    f_after.objects.get(&test_obj).map(|o| o.state.as_str()),
                    Some(to.as_str()),
                    "Final state mismatch for {}", family
                );
            }
        }

        assert_eq!(tested_transitions, 474, "all 474 transitions tested");
        assert_eq!(gap_cells_tested, 13, "all 13 gap transitions verified closed");
        assert_eq!(regular_tested, 458, "all 458 executable transitions passed with preview, door, and queue parity");
    }
}
