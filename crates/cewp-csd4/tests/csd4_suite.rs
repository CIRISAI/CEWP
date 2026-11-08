//! Comprehensive CSD/4 Integration Test Suite.
//!
//! Validates:
//! - All 20 CSD/4 Topologies (`T0` through `T19`) metadata and properties.
//! - Transient state windows (T0, T3, T5, T8, T10).
//! - Topology Realizability Rules (`R1`–`R7`) with valid topologies and planted violations.
//! - Card Refusal Rules (`R1`–`R5`), specifically `R5_ATOMS_NEVER_COHOLD`.
//! - Planner subset-search and folding optimization (`T0` into `T2`).
//! - Cross-Plane Spine Invariants (`X1`–`X5`) with valid states and breaker faults.
//! - Coverage Qualification Report (`122` promised / `156` vocabulary / `242` state table) + 6 durable atoms.
//! - Three-Mode runner contracts (`Formal`, `Sim`, `Native`), `rows.jsonl` export, and `MODEL_DRIFT`.

use std::collections::BTreeSet;
use cewp_csd4::{
    TopologyId, TOPOLOGIES, AWAITING_T20_DURABLE_ATOMS,
    TOTAL_PROMISED_ATOMS, TOTAL_VOCABULARY_ATOMS, TOTAL_CONSTITUTIONAL_STATE_TABLE,
    CATALOGUE_VERSION,
    check_topology_realizability, check_csd_card, TopologyRefusal, CardRefusal,
    CsdCardDecl, CardTestDecl,
    plan_tests, CsdTestRequest, PlannerOptions,
    CoverageReport,
    ExecutionMode, FederationRow, export_rows_jsonl, CheckpointSummary,
    check_inv_x1_latch_does_not_block_serving,
    check_inv_x2_evicted_key_authority_ends,
    check_inv_x3_replication_needs_consent_and_revocation_is_boundary,
    check_inv_x4_local_tier_never_leaves_node,
    check_inv_x5_leave_root_atomic,
    SpineViolation, HaltServingState, KeyRosterState, ConsentReplicationState,
    RowTransmissionState, LeaveRootState, RowTier,
};

// ── 1. All 20 Topologies (T0–T19) Verification ────────────────────────────

#[test]
fn test_all_20_topologies_exist_and_pass_realizability() {
    assert_eq!(TOPOLOGIES.len(), 20);
    assert_eq!(TopologyId::ALL.len(), 20);

    for (idx, id) in TopologyId::ALL.iter().enumerate() {
        assert_eq!(*id as usize, idx);
        let topo = id.get();
        assert_eq!(topo.id, *id);
        assert!(!topo.constitutional_name.is_empty());
        assert!(!topo.server_name.is_empty());
        assert!(!topo.title.is_empty());
        assert!(!topo.steps.is_empty());
        assert!(!topo.trace.is_empty());
        assert!(topo.cost > 0);

        // Every topology must pass R1–R7 realizability rules
        let realizable_result = check_topology_realizability(topo);
        assert!(
            realizable_result.is_ok(),
            "Topology {} failed realizability: {:?}",
            id,
            realizable_result
        );
    }
}

// ── 2. Transient State Windows Verification ────────────────────────────────

#[test]
fn test_transient_windows_t0_t3_t5_t8_t10() {
    // T0: node.unclaimed at [0, 1), node.claimed_sole at [1, end)
    let t0 = TopologyId::T0.get();
    let t0_unclaimed = t0.semantic_window("node.unclaimed").expect("T0 node.unclaimed");
    assert_eq!(t0_unclaimed.start, 0);
    assert_eq!(t0_unclaimed.end, 1);
    let t0_claimed_sole = t0.semantic_window("node.claimed_sole").expect("T0 node.claimed_sole");
    assert_eq!(t0_claimed_sole.start, 1);
    assert!(t0_claimed_sole.end >= t0.steps.len());

    // T3: device.join_requested at [3, 4), device.join_approved at [4, end)
    let t3 = TopologyId::T3.get();
    let t3_req = t3.semantic_window("device.join_requested").expect("T3 device.join_requested");
    assert_eq!(t3_req.start, 3);
    assert_eq!(t3_req.end, 4);
    let t3_app = t3.semantic_window("device.join_approved").expect("T3 device.join_approved");
    assert_eq!(t3_app.start, 4);
    assert!(t3_app.end >= t3.steps.len());

    // T5: member.applicant_pending at [4, 5)
    let t5 = TopologyId::T5.get();
    let t5_pending = t5.semantic_window("member.applicant_pending").expect("T5 member.applicant_pending");
    assert_eq!(t5_pending.start, 4);
    assert_eq!(t5_pending.end, 5);

    // T8: root.halted at [1, 2), root.halt_confirmed at [2, 3), root.resumed at [3, end)
    let t8 = TopologyId::T8.get();
    let t8_halted = t8.semantic_window("root.halted").expect("T8 root.halted");
    assert_eq!(t8_halted.start, 1);
    assert_eq!(t8_halted.end, 2);
    let t8_conf = t8.semantic_window("root.halt_confirmed").expect("T8 root.halt_confirmed");
    assert_eq!(t8_conf.start, 2);
    assert_eq!(t8_conf.end, 3);
    let t8_res = t8.semantic_window("root.resumed").expect("T8 root.resumed");
    assert_eq!(t8_res.start, 3);
    assert!(t8_res.end >= t8.steps.len());

    // T10: self.compromised_device at [3, 4), self.quorum_evicted at [4, end), self.lone_recovery_pending at [5, 6)
    let t10 = TopologyId::T10.get();
    let t10_comp = t10.semantic_window("self.compromised_device").expect("T10 self.compromised_device");
    assert_eq!(t10_comp.start, 3);
    assert_eq!(t10_comp.end, 4);
    let t10_evict = t10.semantic_window("self.quorum_evicted").expect("T10 self.quorum_evicted");
    assert_eq!(t10_evict.start, 4);
    assert!(t10_evict.end >= t10.steps.len());
    let t10_lone = t10.semantic_window("self.lone_recovery_pending").expect("T10 self.lone_recovery_pending");
    assert_eq!(t10_lone.start, 5);
    assert_eq!(t10_lone.end, 6);
}

// ── 3. Topology Realizability Rules R1–R7 with Planted Violations ──────────

#[test]
fn test_topology_realizability_planted_faults() {
    // Plant R6: kind = "family"
    let mut bad_t0 = TopologyId::T0.get().clone();
    let bad_root = cewp_csd4::RootDecl {
        id: "R_BAD",
        kind: "family", // Illegal kind
        holders: 1,
        quorum: 1,
        founders_seated: 0,
        founders_conferrable: 0,
        founders_node_bearing: 0,
        custody: "none",
        lifecycle_verdict: "rooted",
    };
    bad_t0.roots = Box::leak(Box::new([bad_root]));
    let err_r6 = check_topology_realizability(&bad_t0).unwrap_err();
    assert!(matches!(err_r6, TopologyRefusal::R6_ROOT_ALGORITHM { .. }));

    // Plant R1: Canonical holds unknown root
    let mut bad_t1 = TopologyId::T1.get().clone();
    let bad_canon = cewp_csd4::CanonicalDecl {
        id: "C_UNKNOWN",
        holds: "NON_EXISTENT_ROOT",
        serves: &[],
    };
    bad_t1.canonicals = Box::leak(Box::new([bad_canon]));
    let err_r1 = check_topology_realizability(&bad_t1).unwrap_err();
    assert!(matches!(err_r1, TopologyRefusal::R1_REF_CLOSURE { .. }));

    // Plant R3: Person accepts root but owned node does not
    let mut bad_t2 = TopologyId::T2.get().clone();
    let bad_person = cewp_csd4::PersonDecl {
        id: "charlie",
        owns: Box::leak(Box::new(["A"])),
        joins: &[],
        accepts: Some("NON_MATCHING_ROOT"),
    };
    bad_t2.persons = Box::leak(Box::new([bad_person]));
    let err_r3 = check_topology_realizability(&bad_t2).unwrap_err();
    assert!(matches!(err_r3, TopologyRefusal::R3_ACTOR_GROUNDING { .. }));
}

// ── 4. Card Refusal Rules R1–R5 (Planted Faults & R5_ATOMS_NEVER_COHOLD) ──

#[test]
fn test_card_refusal_rules_r1_to_r5() {
    // Clean base card for CSD-095
    let clean = CsdCardDecl {
        card_id: "CSD-095".to_string(),
        catalogue_version: Some(CATALOGUE_VERSION),
        topology: Some(TopologyId::T10),
        stage: Some("sketched".to_string()),
        derives: vec!["5a".to_string()],
        tests: vec![
            CardTestDecl {
                name: "add_held".to_string(),
                at: vec![
                    "key.membership.proposed_add".to_string(),
                    "device.join_requested".to_string(),
                ],
                probe_node: Some("A2".to_string()),
                probe_path: Some("/v1/self/roster/proposals".to_string()),
                probe_status: Some(200),
                probe_used_yaml_on: false,
            },
        ],
        has_layer_graph: false,
    };
    assert!(check_csd_card(&clean).is_ok());

    // Plant R1: Unknown atom
    let mut plant_r1 = clean.clone();
    plant_r1.tests[0].at = vec!["key.membership.evictd".to_string()]; // Typo
    assert!(matches!(
        check_csd_card(&plant_r1),
        Err(CardRefusal::R1_UNKNOWN_ATOM { .. })
    ));

    // Plant R2: Missing derives in building stage
    let mut plant_r2 = clean.clone();
    plant_r2.stage = Some("building".to_string());
    plant_r2.derives = vec![];
    assert!(matches!(
        check_csd_card(&plant_r2),
        Err(CardRefusal::R2_DERIVES_OR_TOPOLOGY_MISMATCH { .. })
    ));

    // Plant R3: Version behind
    let mut plant_r3 = clean.clone();
    plant_r3.catalogue_version = Some(2);
    assert!(matches!(
        check_csd_card(&plant_r3),
        Err(CardRefusal::R3_VERSION_BEHIND { .. })
    ));

    // Plant R4: Probe uses `on:` instead of `node:`
    let mut plant_r4 = clean.clone();
    plant_r4.tests[0].probe_used_yaml_on = true;
    assert!(matches!(
        check_csd_card(&plant_r4),
        Err(CardRefusal::R4_UNCOVERED_DERIVES { .. })
    ));

    // Plant R5: Atoms never co-hold simultaneously in T10
    // key.membership.evicted and key.membership.proposed_add never co-hold simultaneously
    let mut plant_r5 = clean.clone();
    plant_r5.tests[0].at = vec![
        "key.membership.evicted".to_string(),
        "key.membership.proposed_add".to_string(),
    ];
    let err_r5 = check_csd_card(&plant_r5).unwrap_err();
    match err_r5 {
        CardRefusal::R5_ATOMS_NEVER_COHOLD { card_id, test_name, atoms } => {
            assert_eq!(card_id, "CSD-095");
            assert_eq!(test_name, "add_held");
            assert_eq!(atoms, vec!["key.membership.evicted", "key.membership.proposed_add"]);
        }
        _ => panic!("Expected R5_ATOMS_NEVER_COHOLD error, got {:?}", err_r5),
    }
}

// ── 5. Planner Subset-Search & Folding Optimization (T0 into T2) ──────────

#[test]
fn test_planner_subset_search_cost_and_t0_into_t2_folding() {
    let requests = vec![
        // T0 single-node tests
        CsdTestRequest::new("CSD-118", "first_run", Some(TopologyId::T0), &["node.unclaimed"]),
        CsdTestRequest::new("CSD-118", "declared", Some(TopologyId::T0), &["node.claimed"]),
        // T2 pair room tests
        CsdTestRequest::new("CSD-091", "conversation", Some(TopologyId::T2), &["contact.reachable", "room.pair.keyed"]),
        CsdTestRequest::new("CSD-091", "outsider", Some(TopologyId::T2), &["person.outsider"]),
    ];

    let plan = plan_tests(&requests, &PlannerOptions::default());

    // Cost minimization must choose 1 build (T2) instead of 2 builds (T0 + T2)
    assert_eq!(plan.cost.builds, 1, "Must require exactly 1 build due to folding");
    assert_eq!(plan.entries, vec![TopologyId::T2]);

    // T0 tests are folded into T2
    assert_eq!(plan.reselected.get("CSD-118/first_run"), Some(&"T0 → T2".to_string()));
    assert_eq!(plan.reselected.get("CSD-118/declared"), Some(&"T0 → T2".to_string()));
    assert_eq!(plan.folded_into.get("CSD-118/first_run"), Some(&TopologyId::T2));
    assert_eq!(plan.folded_into.get("CSD-118/declared"), Some(&TopologyId::T2));

    // The T2 build cost is 58 (4 nodes * 10 + 18 steps)
    assert_eq!(plan.cost.node_step_cost, TopologyId::T2.get().cost);

    // Verify checkpoints:
    // CSD-118/first_run is scheduled at T2's "nodes" step
    // CSD-118/declared is scheduled at T2's "person:alice" step
    let t2_schedule = plan.schedule.get(&TopologyId::T2).unwrap();
    assert!(t2_schedule.get("nodes").unwrap().contains(&"CSD-118/first_run".to_string()));
    assert!(t2_schedule.get("person:alice").unwrap().contains(&"CSD-118/declared".to_string()));
}

// ── 6. Cross-Plane Spine Invariants (X1–X5) ───────────────────────────────

#[test]
fn test_cross_plane_spine_invariants_x1_to_x5() {
    // Inv_X1: LatchDoesNotBlockServing
    let valid_x1 = HaltServingState {
        step: 10,
        node_id: "NodeA".to_string(),
        halt_latch: true,
        serving_enabled: true,
        replication_enabled: true,
        standing_changed: false,
        agent_acting: false,
        latch_blocks_serve_breaker: false,
    };
    assert!(check_inv_x1_latch_does_not_block_serving(&valid_x1).is_ok());

    let mut breaker_x1 = valid_x1.clone();
    breaker_x1.serving_enabled = false;
    assert!(matches!(
        check_inv_x1_latch_does_not_block_serving(&breaker_x1),
        Err(SpineViolation::X1_LatchBlocksServing { .. })
    ));

    // Inv_X2: EvictedKeyAuthorityEnds
    let mut live_keys = BTreeSet::new();
    live_keys.insert("owner_key".to_string());
    let mut evicted_keys = BTreeSet::new();
    evicted_keys.insert("stolen_phone".to_string());

    let valid_x2 = KeyRosterState {
        step: 15,
        live_keys: live_keys.clone(),
        evicted_keys: evicted_keys.clone(),
        acting_key: Some("owner_key".to_string()),
    };
    assert!(check_inv_x2_evicted_key_authority_ends(&valid_x2).is_ok());

    let mut breach_x2 = valid_x2.clone();
    breach_x2.acting_key = Some("stolen_phone".to_string());
    assert!(matches!(
        check_inv_x2_evicted_key_authority_ends(&breach_x2),
        Err(SpineViolation::X2_EvictedKeyExercisedAuthority { .. })
    ));

    // Inv_X3: ReplicationNeedsConsentAndRevocationIsBoundary
    let valid_x3 = ConsentReplicationState {
        step: 30,
        peer_id: "alice".to_string(),
        has_active_consent: false,
        revocation_step: Some(25),
        row_id: 1001,
        row_created_step: 20, // Before revocation -> allowed
        row_standing_admitted: true,
        is_replicated: false,
    };
    assert!(check_inv_x3_replication_needs_consent_and_revocation_is_boundary(&valid_x3).is_ok());

    let mut breach_x3_no_consent = valid_x3.clone();
    breach_x3_no_consent.is_replicated = true;
    assert!(matches!(
        check_inv_x3_replication_needs_consent_and_revocation_is_boundary(&breach_x3_no_consent),
        Err(SpineViolation::X3_ReplicationLacksConsent { .. })
    ));

    let mut breach_x3 = valid_x3.clone();
    breach_x3.is_replicated = true;
    breach_x3.has_active_consent = true;
    breach_x3.row_created_step = 28; // After revocation -> refusal boundary
    assert!(matches!(
        check_inv_x3_replication_needs_consent_and_revocation_is_boundary(&breach_x3),
        Err(SpineViolation::X3_ReplicationCrossedRevocationBoundary { .. })
    ));

    // Inv_X4: LocalTierNeverLeavesNode
    let valid_x4 = RowTransmissionState {
        step: 8,
        node_id: "NodeB".to_string(),
        row_id: 55,
        tier: RowTier::Federation,
        is_transmitted_over_network: true,
        has_valid_signature: true,
    };
    assert!(check_inv_x4_local_tier_never_leaves_node(&valid_x4).is_ok());

    let mut breach_x4 = valid_x4.clone();
    breach_x4.tier = RowTier::Local;
    assert!(matches!(
        check_inv_x4_local_tier_never_leaves_node(&breach_x4),
        Err(SpineViolation::X4_LocalTierRowLeaked { .. })
    ));

    // Inv_X5: LeaveRootAtomic
    let valid_x5 = LeaveRootState {
        step: 40,
        root_id: "HA".to_string(),
        prior_subscribed: true,
        post_subscribed: false,
        prior_latch: true,
        post_latch: false,
        historical_rows_retained: true,
    };
    assert!(check_inv_x5_leave_root_atomic(&valid_x5).is_ok());

    let mut breach_x5 = valid_x5.clone();
    breach_x5.post_latch = true; // Failed to lift latch
    assert!(matches!(
        check_inv_x5_leave_root_atomic(&breach_x5),
        Err(SpineViolation::X5_LatchNotLiftedWithSubscriptionFall { .. })
    ));
}

// ── 7. Coverage Qualification Report ──────────────────────────────────────

#[test]
fn test_coverage_qualification_exact_formatting_and_durability_atoms() {
    let report = CoverageReport::from_topologies(&[TopologyId::T0, TopologyId::T1, TopologyId::T2]);
    let qual = report.format_qualification();

    // Must match: (atoms_exercised / 122 promised / 156 vocabulary / 242 constitutional_state_table)
    let expected_suffix = format!(
        "/ {} promised / {} vocabulary / {} constitutional_state_table)",
        TOTAL_PROMISED_ATOMS, TOTAL_VOCABULARY_ATOMS, TOTAL_CONSTITUTIONAL_STATE_TABLE
    );
    assert!(qual.ends_with(&expected_suffix), "Qualification string mismatch: {}", qual);

    // Must list the 6 file.durable.* atoms awaiting T20 (CIRISServer#809)
    assert_eq!(AWAITING_T20_DURABLE_ATOMS.len(), 6);
    let full_text = report.format_full_report();
    for atom in AWAITING_T20_DURABLE_ATOMS {
        assert!(
            full_text.contains(atom),
            "Full report missing awaiting durable atom: {}",
            atom
        );
    }
}

// ── 8. Three-Mode Runner Contract, rows.jsonl & MODEL_DRIFT ───────────────

#[test]
fn test_three_mode_runner_and_rows_jsonl_and_model_drift() {
    // Mode checks
    assert_eq!(ExecutionMode::from_str("formal").unwrap(), ExecutionMode::Formal);
    assert_eq!(ExecutionMode::from_str("sim").unwrap(), ExecutionMode::Sim);
    assert_eq!(ExecutionMode::from_str("native").unwrap(), ExecutionMode::Native);

    // rows.jsonl export
    let rows = vec![
        FederationRow {
            rowid: 1,
            dimension: "attestation".to_string(),
            cohort_scope: "federation".to_string(),
            admitting_key_id: "key_root".to_string(),
            attesting_key_id: "key_signer".to_string(),
            payload_hash: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
            signature: "sig_abc".to_string(),
            admitted_step: 1,
        },
    ];
    let (jsonl, digest) = export_rows_jsonl(&rows);
    assert!(jsonl.contains("\"rowid\":1"));
    assert_eq!(digest.len(), 64);

    // Checkpoint MODEL_DRIFT witness
    let checkpoint = CheckpointSummary::new(
        TopologyId::T0,
        "nodes",
        0,
        &["node.unclaimed", "root.none"],
        &[("node.unclaimed", true), ("root.none", true)],
    );
    assert!(checkpoint.check_model_drift().is_ok());

    let mut drift_checkpoint = checkpoint.clone();
    drift_checkpoint.live_witness_readings.insert("node.unclaimed".to_string(), false);
    let drift_err = drift_checkpoint.check_model_drift().unwrap_err();
    assert_eq!(drift_err.atom, "node.unclaimed");
    assert_eq!(drift_err.model_expected, true);
    assert_eq!(drift_err.live_witness, false);
}

// ── 9. CSD Namespace Map Comprehensive Integration ────────────────────────

#[test]
fn test_csd_namespace_map_full_integration() {
    use cewp_csd4::CsdNamespaceMap;

    let map = CsdNamespaceMap::load_vendored();

    // 1. Summary statistics check
    assert_eq!(map.summary.total_families, 191);
    assert_eq!(map.summary.total_transitions, 474);
    assert_eq!(map.summary.total_policies, 12);
    assert_eq!(map.summary.total_csd_cards, 105);
    assert_eq!(map.summary.csd_mapped_families, 94);
    assert_eq!(map.summary.substrate_autonomous_families, 97);
    assert_eq!(map.summary.total_csd_shows_fields, 660);
    assert_eq!(map.summary.resolved_csd_shows_fields, 152);
    assert_eq!(map.summary.canonical_resolved_shows_fields, 152);
    assert_eq!(map.summary.refused_private_namespace_fields, 505);
    assert_eq!(map.summary.refused_retired_holds_bytes_fields, 3);
    assert_eq!(map.summary.total_match_vectors, 1291);
    assert_eq!(map.summary.valid_canonical_vectors, 783);
    assert_eq!(map.summary.refused_private_namespace_vectors, 505);
    assert_eq!(map.summary.refused_retired_holds_bytes_vectors, 3);

    // 2. Collections sizing
    assert_eq!(map.cards.len(), 105);
    assert_eq!(map.families.len(), 191);
    assert_eq!(map.topologies.len(), 20);

    // 3. Category count check (94 UI surface + 97 substrate autonomous)
    let ui_count = map.families.iter().filter(|f| f.category == "csd_ui_surface").count();
    let sub_count = map.families.iter().filter(|f| f.category == "substrate_autonomous").count();
    assert_eq!(ui_count, 94);
    assert_eq!(sub_count, 97);

    // 4. Transitions summation
    let total_transitions: usize = map.families.iter().map(|f| f.transitions_count).sum();
    assert_eq!(total_transitions, 474);

    // 5. Aliases resolution
    assert_eq!(map.resolve_csd_shows("consent_request"), Some("consent"));
    assert_eq!(map.resolve_csd_shows("directory"), Some("directory:{entry_type}:{key}"));
    assert_eq!(map.resolve_csd_shows("trust:attestation:device"), Some("trust:{job}:{version}"));

    // 6. All 660 csd:shows fields resolution
    let mut resolved_count = 0;
    for card in &map.cards {
        for show in &card.shows {
            let res = map.resolve_csd_shows(&show.field);
            assert!(
                res.is_some(),
                "Field '{}' on card '{}' failed to resolve",
                show.field,
                card.id
            );
            assert_eq!(
                res.unwrap(),
                show.canonical_family.as_str(),
                "Field '{}' on card '{}' resolved to '{}', expected '{}'",
                show.field,
                card.id,
                res.unwrap(),
                show.canonical_family
            );
            resolved_count += 1;
        }
    }
    assert_eq!(resolved_count, 660, "Expected all 660 shows fields to be validated");

    // 7. Every topology T0–T19 mapping
    for id in TopologyId::ALL {
        let cards = map.cards_for_topology(id);
        let families = map.families_for_topology(id);

        assert!(!cards.is_empty(), "Topology {} should map to CSD cards", id);
        if id != TopologyId::T9 {
            assert!(!families.is_empty(), "Topology {} should map to namespace families", id);
        }

        // Verify card_by_id and family_by_pattern consistency
        for card in &cards {
            assert_eq!(map.card_by_id(&card.id).map(|c| &c.id), Some(&card.id));
        }
        for fam in &families {
            assert_eq!(map.family_by_pattern(&fam.pattern).map(|f| &f.pattern), Some(&fam.pattern));
        }
    }

    // Specific topology checks
    let t0_cards = map.cards_for_topology(TopologyId::T0);
    assert_eq!(t0_cards.len(), 30);
    let t0_fams = map.families_for_topology(TopologyId::T0);
    assert_eq!(t0_fams.len(), 49);

    let t1_cards = map.cards_for_topology(TopologyId::T1);
    assert_eq!(t1_cards.len(), 15);
    let t1_fams = map.families_for_topology(TopologyId::T1);
    assert_eq!(t1_fams.len(), 19);

    let t10_cards = map.cards_for_topology(TopologyId::T10);
    assert_eq!(t10_cards.len(), 4);
    assert!(t10_cards.iter().any(|c| c.id == "CSD-095"));
}
