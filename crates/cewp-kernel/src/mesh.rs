//! Multi-node mesh simulator: nodes as cells, rows as signals.
//!
//! One tick offers held/sealed rows to neighbors in the audience.

use crate::processor::{door, fold, preview, queue};
use crate::tables::Tables;
use crate::types::{FoldState, HeldRow, PreviewResult, Queue, Refusal, Row, StepResult};
use std::collections::{HashMap, HashSet};

pub const CONSENT_FREE: &[&str] = &[
    "key:record",
    "transport:{kind}",
    // rc19: a person's device binding (transport:{kind} is now an infrastructure node's)
    "identity:occurrence",
    "consent:{kind}",
    "accord:invoke:constitutional:{halt_id}",
    "trust:{job}:{version}",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealedEntry {
    pub row: Row,
    pub origin: String,
}

#[derive(Debug, Clone)]
pub struct MeshNode {
    pub id: String,
    pub owner: String,
    pub kind: String,
    pub held: Vec<HeldRow>,
    pub sealed: HashMap<String, SealedEntry>,
    pub sealed_order: Vec<String>,
    pub offered: HashSet<String>,
    pub carrier: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshMove {
    pub from: String,
    pub to: String,
    pub row: String,
    pub ok: bool,
    pub sealed: bool,
    pub refusal: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MeshLogEntry {
    pub t: u64,
    pub node: String,
    pub row: String,
    pub admitted: bool,
    pub refusal: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Mesh {
    pub tables: Tables,
    pub nodes: HashMap<String, MeshNode>,
    pub node_order: Vec<String>,
    pub links: HashSet<String>,
    pub now: u64,
    pub log: Vec<MeshLogEntry>,
}

impl Mesh {
    pub fn new(tables: Tables) -> Self {
        Self {
            tables,
            nodes: HashMap::new(),
            node_order: Vec::new(),
            links: HashSet::new(),
            now: 0,
            log: Vec::new(),
        }
    }

    pub fn add_node(&mut self, id: &str, owner: &str, kind: &str) -> &mut MeshNode {
        let carrier = kind == "server";
        let node = MeshNode {
            id: id.to_string(),
            owner: owner.to_string(),
            kind: kind.to_string(),
            held: Vec::new(),
            sealed: HashMap::new(),
            sealed_order: Vec::new(),
            offered: HashSet::new(),
            carrier,
        };
        self.nodes.insert(id.to_string(), node);
        if !self.node_order.iter().any(|x| x == id) {
            self.node_order.push(id.to_string());
        }
        self.nodes.get_mut(id).unwrap()
    }

    pub fn link(&mut self, a: &str, b: &str) {
        let key = if a < b {
            format!("{}|{}", a, b)
        } else {
            format!("{}|{}", b, a)
        };
        self.links.insert(key);
    }

    pub fn unlink(&mut self, a: &str, b: &str) {
        let key = if a < b {
            format!("{}|{}", a, b)
        } else {
            format!("{}|{}", b, a)
        };
        self.links.remove(&key);
    }

    pub fn sleep(&mut self, node_id: &str) {
        self.links.retain(|l| {
            let mut parts = l.split('|');
            let a = parts.next().unwrap_or("");
            let b = parts.next().unwrap_or("");
            a != node_id && b != node_id
        });
    }

    pub fn wake(&mut self, node_id: &str) {
        self.link(node_id, "seat");
    }

    pub fn neighbours(&self, id: &str) -> Vec<String> {
        let mut out = Vec::new();
        for l in &self.links {
            let mut parts = l.split('|');
            let a = parts.next().unwrap_or("");
            let b = parts.next().unwrap_or("");
            if a == id {
                out.push(b.to_string());
            } else if b == id {
                out.push(a.to_string());
            }
        }
        out.sort();
        out
    }

    pub fn audience(&self, origin_node_id: &str, row: &Row) -> Option<HashSet<String>> {
        let node = self.nodes.get(origin_node_id)?;
        let f = fold(&node.held, &self.tables, self.now);
        let tier = row.scope.as_ref().map(|s| s.tier.as_str()).unwrap_or("self");
        let m = self.tables.match_family(&row.family);
        let fam_key = m.family.as_deref().unwrap_or(&row.family);

        if tier == "federation"
            || CONSENT_FREE.contains(&fam_key)
            || CONSENT_FREE.contains(&row.family.as_str())
        {
            return None; // everyone may carry it
        }

        if tier == "self" {
            let mut who = HashSet::new();
            who.insert(row.signer.clone());
            return Some(who);
        }

        let mut who = HashSet::new();
        let room_id = row.scope.as_ref().and_then(|s| s.room.as_deref());
        if let Some(rid) = room_id {
            if let Some(r) = f.rooms.get(rid) {
                for m in &r.members {
                    who.insert(m.clone());
                }
                for fo in &r.founders {
                    who.insert(fo.clone());
                }
            }
        }
        if let Some(subject) = &row.subject {
            who.insert(subject.clone());
        }

        // the subject of a live proposal sees the room's rows: a first join sees a fresh roster
        for obj_id in &f.object_order {
            let o = &f.objects[obj_id];
            if o.family == "membership:{stage}"
                && o.scope.room.as_deref() == room_id
                && o.state == "admitted"
            {
                if let Some(subject) = &o.subject {
                    who.insert(subject.clone());
                }
            }
        }

        Some(who)
    }

    pub fn sign(&mut self, node_id: &str, mut row: Row) -> StepResult {
        if row.asserted_at == 0 {
            row.asserted_at = self.now;
        }
        let (res, row_id) = {
            let node = match self.nodes.get_mut(node_id) {
                Some(n) => n,
                None => return Err(Refusal::Other("unknown_node".to_string())),
            };
            let s = door(&mut node.held, &self.tables, &row, self.now);
            (s, row.id.clone())
        };

        match &res {
            Ok(_) => {
                self.log.push(MeshLogEntry {
                    t: self.now,
                    node: node_id.to_string(),
                    row: row_id,
                    admitted: true,
                    refusal: None,
                });
            }
            Err(e) => {
                self.log.push(MeshLogEntry {
                    t: self.now,
                    node: node_id.to_string(),
                    row: row_id,
                    admitted: false,
                    refusal: Some(e.to_string()),
                });
            }
        }

        res
    }

    pub fn tick(&mut self, dt: u64) -> Vec<MeshMove> {
        self.now += dt;
        let mut moves = Vec::new();

        let node_ids = self.node_order.clone();
        for node_id in node_ids {
            let neighbors = self.neighbours(&node_id);
            for nb in neighbors {
                let carried: Vec<(Row, String)> = {
                    let node = &self.nodes[&node_id];
                    let mut c = Vec::new();
                    for h in &node.held {
                        c.push((h.row.clone(), node.id.clone()));
                    }
                    for sid in &node.sealed_order {
                        if let Some(s) = node.sealed.get(sid) {
                            c.push((s.row.clone(), s.origin.clone()));
                        }
                    }
                    c
                };

                for (row, origin) in carried {
                    let key = format!("{}>{}", row.id, nb);
                    {
                        let node = &self.nodes[&node_id];
                        let other = &self.nodes[&nb];
                        if node.offered.contains(&key)
                            || other.held.iter().any(|h| h.row.id == row.id)
                            || other.sealed.contains_key(&row.id)
                        {
                            continue;
                        }
                    }

                    let aud = self.audience(&origin, &row);
                    let other_carrier = self.nodes[&nb].carrier;
                    let other_owner = self.nodes[&nb].owner.clone();
                    let other_id = self.nodes[&nb].id.clone();

                    if let Some(aud_set) = aud {
                        if !aud_set.contains(&other_owner) {
                            if !other_carrier || other_id == origin {
                                continue;
                            }
                            // Carrier seat carries it sealed
                            let other_node = self.nodes.get_mut(&nb).unwrap();
                            if !other_node.sealed.contains_key(&row.id) {
                                other_node.sealed_order.push(row.id.clone());
                            }
                            other_node.sealed.insert(
                                row.id.clone(),
                                SealedEntry {
                                    row: row.clone(),
                                    origin,
                                },
                            );
                            self.nodes.get_mut(&node_id).unwrap().offered.insert(key);
                            moves.push(MeshMove {
                                from: node_id.clone(),
                                to: nb.clone(),
                                row: row.id.clone(),
                                ok: true,
                                sealed: true,
                                refusal: None,
                            });
                            continue;
                        }
                    }

                    let s = {
                        let other_node = self.nodes.get_mut(&nb).unwrap();
                        door(&mut other_node.held, &self.tables, &row, self.now)
                    };

                    if s.is_ok() {
                        self.nodes.get_mut(&node_id).unwrap().offered.insert(key);
                    }
                    moves.push(MeshMove {
                        from: node_id.clone(),
                        to: nb.clone(),
                        row: row.id.clone(),
                        ok: s.is_ok(),
                        sealed: false,
                        refusal: s.err().map(|r| r.to_string()),
                    });
                }
            }
        }

        moves
    }

    pub fn fold_of(&self, node_id: &str) -> FoldState {
        let node = self.nodes.get(node_id).expect("node not found");
        fold(&node.held, &self.tables, self.now)
    }

    pub fn queue_of(&self, node_id: &str) -> Queue {
        let node = self.nodes.get(node_id).expect("node not found");
        queue(&node.held, &self.tables, &node.owner, self.now)
    }

    pub fn preview_at(&self, node_id: &str, rows: &[Row]) -> PreviewResult {
        let node = self.nodes.get(node_id).expect("node not found");
        preview(&node.held, &self.tables, rows, self.now)
    }
}
