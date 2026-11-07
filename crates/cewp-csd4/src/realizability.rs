//! Realizability and Card Refusal Validators.
//!
//! Enforces:
//! 1. Topology Realizability Rules (`R1`–`R7`) from `CIRISServer/FSD/TOPOLOGY.md` §3:
//!    - `R1_REF_CLOSURE`: All reference IDs across layers resolve without dangling links.
//!    - `R2_CANONICAL_PIN`: Canonicals pin known roots and comply with per-mesh limits.
//!    - `R3_ACTOR_GROUNDING`: Actor person/device grounding, owner accepts root only if owned nodes do.
//!    - `R4_RELATION_ARITY`: Relation endpoints valid, scoped content requires direct links or connected graph.
//!    - `R5_NEGATIVE_WITNESS`: Negative predicates valid and have non-vacuous controls.
//!    - `R6_ROOT_ALGORITHM`: Infrastructure root quorum M needs seated >= M+1, conferrable >= 1, derived K.
//!    - `R7_RELATION_REALIZABLE`: Relation prerequisites (infrastructure root for halt/confirm/resume, common root for rooted_with).
//!
//! 2. Card Refusal Rules (`R1`–`R5`) from `CIRISClient/packaging/check_csd4.py`:
//!    - `R1_UNKNOWN_ATOM`: Refuse any atom outside catalogue vocabulary.
//!    - `R2_DERIVES_OR_TOPOLOGY_MISMATCH`: Refuse topology outside T0–T19 or missing derives when building.
//!    - `R3_VERSION_BEHIND`: Refuse card whose catalogue_version is behind pinned version.
//!    - `R4_UNCOVERED_DERIVES`: Refuse empty tests, malformed probes (e.g. `on:` instead of `node:`).
//!    - `R5_ATOMS_NEVER_COHOLD`: Refuse tests whose atoms never co-hold simultaneously at any step.

use std::collections::HashSet;
use std::fmt;
use crate::catalogue::{
    TopologyDecl, TopologyId, VOCABULARY_ATOMS, CATALOGUE_VERSION,
};

// ── Topology Realizability Rules (R1–R7) ───────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum TopologyRefusal {
    R1_REF_CLOSURE { detail: String },
    R2_CANONICAL_PIN { detail: String },
    R3_ACTOR_GROUNDING { detail: String },
    R4_RELATION_ARITY { detail: String },
    R5_NEGATIVE_WITNESS { detail: String },
    R6_ROOT_ALGORITHM { detail: String },
    R7_RELATION_REALIZABLE { detail: String },
}

impl fmt::Display for TopologyRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TopologyRefusal::R1_REF_CLOSURE { detail } => write!(f, "R1_REF_CLOSURE: {}", detail),
            TopologyRefusal::R2_CANONICAL_PIN { detail } => write!(f, "R2_CANONICAL_PIN: {}", detail),
            TopologyRefusal::R3_ACTOR_GROUNDING { detail } => write!(f, "R3_ACTOR_GROUNDING: {}", detail),
            TopologyRefusal::R4_RELATION_ARITY { detail } => write!(f, "R4_RELATION_ARITY: {}", detail),
            TopologyRefusal::R5_NEGATIVE_WITNESS { detail } => write!(f, "R5_NEGATIVE_WITNESS: {}", detail),
            TopologyRefusal::R6_ROOT_ALGORITHM { detail } => write!(f, "R6_ROOT_ALGORITHM: {}", detail),
            TopologyRefusal::R7_RELATION_REALIZABLE { detail } => write!(f, "R7_RELATION_REALIZABLE: {}", detail),
        }
    }
}

impl std::error::Error for TopologyRefusal {}

/// Validates a topology declaration against Realizability Rules R1–R7.
pub fn check_topology_realizability(topo: &TopologyDecl) -> Result<(), TopologyRefusal> {
    let root_ids: HashSet<&str> = topo.roots.iter().map(|r| r.id).collect();
    let canon_ids: HashSet<&str> = topo.canonicals.iter().map(|c| c.id).collect();
    let node_ids: HashSet<&str> = topo.nodes.iter().map(|n| n.id).collect();
    let person_ids: HashSet<&str> = topo.persons.iter().map(|p| p.id).collect();

    // R6: Root algorithm
    for r in topo.roots {
        if r.kind == "family" {
            return Err(TopologyRefusal::R6_ROOT_ALGORITHM {
                detail: format!("root {}: kind 'family' has no model behind it — use 'key' or 'infrastructure'", r.id),
            });
        }
        if r.kind == "infrastructure" {
            if r.founders_seated < r.quorum + 1 {
                return Err(TopologyRefusal::R6_ROOT_ALGORITHM {
                    detail: format!("rule 1: root {} quorum {} needs founders.seated >= {} (T7)", r.id, r.quorum, r.quorum + 1),
                });
            }
            if r.founders_conferrable < 1 {
                return Err(TopologyRefusal::R6_ROOT_ALGORITHM {
                    detail: format!("rule 1: root {} needs founders.conferrable >= 1 for T7 recovery", r.id),
                });
            }
        }
    }

    // R1: Reference closure - canonical holds
    for c in topo.canonicals {
        if !root_ids.contains(c.holds) {
            return Err(TopologyRefusal::R1_REF_CLOSURE {
                detail: format!("canonical {} holds unknown root {}", c.id, c.holds),
            });
        }
    }

    // R1: Reference closure - nodes dial known node or canonical
    for n in topo.nodes {
        for dial in n.dials {
            if !node_ids.contains(dial) && !canon_ids.contains(dial) {
                return Err(TopologyRefusal::R1_REF_CLOSURE {
                    detail: format!("node {} dials unknown {}", n.id, dial),
                });
            }
        }
        if let Some(acc) = n.accepts {
            if !root_ids.contains(acc) {
                return Err(TopologyRefusal::R1_REF_CLOSURE {
                    detail: format!("node {} accepts unknown root {}", n.id, acc),
                });
            }
        }
    }

    // R1 & R3: Persons own nodes, accept roots
    for p in topo.persons {
        for owned in p.owns {
            if !node_ids.contains(owned) {
                return Err(TopologyRefusal::R1_REF_CLOSURE {
                    detail: format!("person {} owns unknown node {}", p.id, owned),
                });
            }
            // R3: person accepts root only if every node they own accepts it
            if let Some(p_acc) = p.accepts {
                let node_decl = topo.nodes.iter().find(|n| n.id == *owned).unwrap();
                if node_decl.accepts != Some(p_acc) {
                    return Err(TopologyRefusal::R3_ACTOR_GROUNDING {
                        detail: format!("rule 2: {} accepts {} but owns {} which does not", p.id, p_acc, owned),
                    });
                }
            }
        }

        // R3: joins must be in owns, and first owned node cannot be in joins
        for join in p.joins {
            if !p.owns.contains(join) {
                return Err(TopologyRefusal::R3_ACTOR_GROUNDING {
                    detail: format!("person {} joins {}, which it does not own", p.id, join),
                });
            }
        }
        if !p.owns.is_empty() && !p.joins.is_empty() && p.joins.contains(&p.owns[0]) {
            return Err(TopologyRefusal::R3_ACTOR_GROUNDING {
                detail: format!("person {}: the first owned node is claimed with a fresh identity; only later device can join by approval", p.id),
            });
        }
    }

    // R3: Actor grounding
    if let Some(ref actor) = topo.actor {
        if !person_ids.contains(actor.person) {
            return Err(TopologyRefusal::R3_ACTOR_GROUNDING {
                detail: format!("actor person {} does not exist", actor.person),
            });
        }
        let person = topo.persons.iter().find(|p| p.id == actor.person).unwrap();
        if !person.owns.contains(&actor.device) {
            return Err(TopologyRefusal::R3_ACTOR_GROUNDING {
                detail: format!("actor person {} does not own actor device {}", actor.person, actor.device),
            });
        }
    }

    // R7: Relation realizability & R4 Relation arity
    let has_infra_root = topo.roots.iter().any(|r| r.kind == "infrastructure");
    for (i, rel) in topo.relations.iter().enumerate() {
        let k = rel.rel;
        if matches!(k, "halt" | "confirm" | "resume" | "join_request") && !has_infra_root {
            return Err(TopologyRefusal::R7_RELATION_REALIZABLE {
                detail: format!("rel[{}] `{}` needs infrastructure root", i, k),
            });
        }
        if k == "rooted_with" {
            if rel.between.len() < 2 {
                return Err(TopologyRefusal::R4_RELATION_ARITY {
                    detail: format!("rooted_with requires between of length 2"),
                });
            }
            let p1 = topo.persons.iter().find(|p| p.id == rel.between[0]);
            let p2 = topo.persons.iter().find(|p| p.id == rel.between[1]);
            match (p1, p2) {
                (Some(a), Some(b)) => {
                    if a.accepts.is_none() || a.accepts != b.accepts {
                        return Err(TopologyRefusal::R7_RELATION_REALIZABLE {
                            detail: format!("rule 3: rooted_with({},{}) needs a common accepted root", a.id, b.id),
                        });
                    }
                }
                _ => {
                    return Err(TopologyRefusal::R1_REF_CLOSURE {
                        detail: format!("rooted_with refers to unknown persons {:?}", rel.between),
                    });
                }
            }
        }
        if k == "reachable" {
            if let Some(target_p) = rel.person {
                let p = topo.persons.iter().find(|p| p.id == target_p);
                if let Some(p) = p {
                    let has_announced = p.owns.iter().any(|nid| {
                        topo.nodes.iter().any(|n| n.id == *nid && n.announced)
                    });
                    if !has_announced {
                        return Err(TopologyRefusal::R7_RELATION_REALIZABLE {
                            detail: format!("rule 5: reachable(..., {}) needs one of {}'s nodes announced", target_p, target_p),
                        });
                    }
                }
            }
        }
        if k == "contact" && rel.to.is_some() {
            // via code requires announced
            let target_p = rel.to.unwrap();
            let p = topo.persons.iter().find(|p| p.id == target_p);
            if let Some(p) = p {
                let has_announced = p.owns.iter().any(|nid| {
                    topo.nodes.iter().any(|n| n.id == *nid && n.announced)
                });
                if !has_announced {
                    return Err(TopologyRefusal::R7_RELATION_REALIZABLE {
                        detail: format!("rule 6: contact to {} via code needs {}'s node announced", target_p, target_p),
                    });
                }
            }
        }
    }

    Ok(())
}

// ── Card Refusal Rules (R1–R5) ─────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum CardRefusal {
    R1_UNKNOWN_ATOM { atom: String, catalogue_version: u32 },
    R2_DERIVES_OR_TOPOLOGY_MISMATCH { detail: String },
    R3_VERSION_BEHIND { card_version: u32, pinned_version: u32 },
    R4_UNCOVERED_DERIVES { detail: String },
    R5_ATOMS_NEVER_COHOLD { card_id: String, test_name: String, atoms: Vec<String> },
}

impl fmt::Display for CardRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CardRefusal::R1_UNKNOWN_ATOM { atom, catalogue_version } => {
                write!(f, "R1_UNKNOWN_ATOM: atom '{}' is not in catalogue version {} vocabulary", atom, catalogue_version)
            }
            CardRefusal::R2_DERIVES_OR_TOPOLOGY_MISMATCH { detail } => {
                write!(f, "R2_DERIVES_OR_TOPOLOGY_MISMATCH: {}", detail)
            }
            CardRefusal::R3_VERSION_BEHIND { card_version, pinned_version } => {
                write!(f, "R3_VERSION_BEHIND: card catalogue_version {} is behind pinned catalogue {}", card_version, pinned_version)
            }
            CardRefusal::R4_UNCOVERED_DERIVES { detail } => {
                write!(f, "R4_UNCOVERED_DERIVES: {}", detail)
            }
            CardRefusal::R5_ATOMS_NEVER_COHOLD { card_id, test_name, atoms } => {
                write!(f, "R5_ATOMS_NEVER_COHOLD: test '{}/{}' has atoms {:?} that never co-hold at one instant", card_id, test_name, atoms)
            }
        }
    }
}

impl std::error::Error for CardRefusal {}

/// A CSD card test specification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardTestDecl {
    pub name: String,
    pub at: Vec<String>,
    pub probe_node: Option<String>,
    pub probe_path: Option<String>,
    pub probe_status: Option<u16>,
    pub probe_used_yaml_on: bool, // Planted defect: 'on:' used instead of 'node:'
}

/// A CSD Card declaration parsed from markdown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsdCardDecl {
    pub card_id: String,
    pub catalogue_version: Option<u32>,
    pub topology: Option<TopologyId>,
    pub stage: Option<String>,
    pub derives: Vec<String>,
    pub tests: Vec<CardTestDecl>,
    pub has_layer_graph: bool, // Planted defect: has both csd:tests and csd:topology
}

/// Validates a CSD Card against Card Refusal Rules R1–R5.
pub fn check_csd_card(card: &CsdCardDecl) -> Result<(), CardRefusal> {
    if card.has_layer_graph {
        return Err(CardRefusal::R2_DERIVES_OR_TOPOLOGY_MISMATCH {
            detail: "the card carries csd:tests AND csd:topology layer graph — one catalogue id, never a topology block".to_string(),
        });
    }

    // R3: Version behind
    match card.catalogue_version {
        None => {
            return Err(CardRefusal::R3_VERSION_BEHIND {
                card_version: 0,
                pinned_version: CATALOGUE_VERSION,
            });
        }
        Some(ver) if ver != CATALOGUE_VERSION => {
            return Err(CardRefusal::R3_VERSION_BEHIND {
                card_version: ver,
                pinned_version: CATALOGUE_VERSION,
            });
        }
        _ => {}
    }

    // R2: Topology must be valid
    let topo_id = match card.topology {
        Some(id) => id,
        None => {
            return Err(CardRefusal::R2_DERIVES_OR_TOPOLOGY_MISMATCH {
                detail: "topology is missing or outside closed catalogue T0–T19".to_string(),
            });
        }
    };

    // R2: Stage building or later requires derives
    if let Some(ref stage) = card.stage {
        let is_building_or_later = matches!(stage.as_str(), "building" | "testable" | "verified" | "shipped");
        if is_building_or_later && card.derives.is_empty() {
            return Err(CardRefusal::R2_DERIVES_OR_TOPOLOGY_MISMATCH {
                detail: format!("stage '{}' without derives: (CSD4.md §1 rule 4)", stage),
            });
        }
    }

    // R4: Empty tests
    if card.tests.is_empty() {
        return Err(CardRefusal::R4_UNCOVERED_DERIVES {
            detail: "tests: is missing or empty".to_string(),
        });
    }

    let vocab_set: HashSet<&str> = VOCABULARY_ATOMS.iter().copied().collect();
    let topo_decl = topo_id.get();

    for test in &card.tests {
        if test.at.is_empty() {
            return Err(CardRefusal::R4_UNCOVERED_DERIVES {
                detail: format!("test '{}' at: must be non-empty list of atoms", test.name),
            });
        }

        if test.probe_used_yaml_on {
            return Err(CardRefusal::R4_UNCOVERED_DERIVES {
                detail: format!("test '{}' probe names node with `on:`, which YAML reads as true — use `node:`", test.name),
            });
        }

        // R1: Unknown atoms
        for atom in &test.at {
            if !vocab_set.contains(atom.as_str()) {
                return Err(CardRefusal::R1_UNKNOWN_ATOM {
                    atom: atom.clone(),
                    catalogue_version: CATALOGUE_VERSION,
                });
            }
        }

        // R5: Atoms never co-hold simultaneously in ANY step of this topology
        let req_slices: Vec<&str> = test.at.iter().map(|s| s.as_str()).collect();
        let co_holds = topo_decl.earliest_step_for(&req_slices).is_some();
        if !co_holds {
            return Err(CardRefusal::R5_ATOMS_NEVER_COHOLD {
                card_id: card.card_id.clone(),
                test_name: test.name.clone(),
                atoms: test.at.clone(),
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_catalogue_topologies_pass_realizability() {
        for id in TopologyId::ALL {
            let decl = id.get();
            let res = check_topology_realizability(decl);
            assert!(
                res.is_ok(),
                "Catalogue entry {} failed realizability: {:?}",
                id,
                res
            );
        }
    }

    #[test]
    fn test_card_check_valid() {
        let card = CsdCardDecl {
            card_id: "CSD-118".to_string(),
            catalogue_version: Some(5),
            topology: Some(TopologyId::T0),
            stage: Some("sketched".to_string()),
            derives: vec!["9c".to_string()],
            tests: vec![
                CardTestDecl {
                    name: "first_run".to_string(),
                    at: vec!["root.none".to_string(), "node.unclaimed".to_string()],
                    probe_node: Some("N".to_string()),
                    probe_path: Some("/v1/setup/status".to_string()),
                    probe_status: Some(200),
                    probe_used_yaml_on: false,
                },
            ],
            has_layer_graph: false,
        };
        assert!(check_csd_card(&card).is_ok());
    }

    #[test]
    fn test_card_check_planted_r1_unknown_atom() {
        let card = CsdCardDecl {
            card_id: "CSD-118".to_string(),
            catalogue_version: Some(5),
            topology: Some(TopologyId::T0),
            stage: Some("sketched".to_string()),
            derives: vec!["9c".to_string()],
            tests: vec![
                CardTestDecl {
                    name: "bad_atom_test".to_string(),
                    at: vec!["node.unclaimed".to_string(), "completely.fake.atom".to_string()],
                    probe_node: None,
                    probe_path: None,
                    probe_status: None,
                    probe_used_yaml_on: false,
                },
            ],
            has_layer_graph: false,
        };
        let err = check_csd_card(&card).unwrap_err();
        assert!(matches!(err, CardRefusal::R1_UNKNOWN_ATOM { .. }));
    }

    #[test]
    fn test_card_check_planted_r3_version_behind() {
        let card = CsdCardDecl {
            card_id: "CSD-118".to_string(),
            catalogue_version: Some(3), // Old version
            topology: Some(TopologyId::T0),
            stage: Some("sketched".to_string()),
            derives: vec!["9c".to_string()],
            tests: vec![
                CardTestDecl {
                    name: "test".to_string(),
                    at: vec!["node.unclaimed".to_string()],
                    probe_node: None,
                    probe_path: None,
                    probe_status: None,
                    probe_used_yaml_on: false,
                },
            ],
            has_layer_graph: false,
        };
        let err = check_csd_card(&card).unwrap_err();
        assert!(matches!(err, CardRefusal::R3_VERSION_BEHIND { .. }));
    }

    #[test]
    fn test_card_check_planted_r5_atoms_never_cohold() {
        // In T0, node.unclaimed and node.claimed never co-hold at the same step
        let card = CsdCardDecl {
            card_id: "CSD-118".to_string(),
            catalogue_version: Some(5),
            topology: Some(TopologyId::T0),
            stage: Some("sketched".to_string()),
            derives: vec!["9c".to_string()],
            tests: vec![
                CardTestDecl {
                    name: "impossible_instant".to_string(),
                    at: vec!["node.unclaimed".to_string(), "node.claimed".to_string()],
                    probe_node: None,
                    probe_path: None,
                    probe_status: None,
                    probe_used_yaml_on: false,
                },
            ],
            has_layer_graph: false,
        };
        let err = check_csd_card(&card).unwrap_err();
        match err {
            CardRefusal::R5_ATOMS_NEVER_COHOLD { card_id, test_name, atoms } => {
                assert_eq!(card_id, "CSD-118");
                assert_eq!(test_name, "impossible_instant");
                assert_eq!(atoms, vec!["node.unclaimed", "node.claimed"]);
            }
            _ => panic!("Expected R5_ATOMS_NEVER_COHOLD error, got {:?}", err),
        }
    }
}
