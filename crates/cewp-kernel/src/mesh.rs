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
    /// Each room row's head, fixed when it is signed: the live roster and membership rows of its room on the signing
    /// node (the twin's rows carry no digest of their own, so the mesh keeps the head beside the row).
    pub heads: HashMap<String, Vec<String>>,
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
            heads: HashMap::new(),
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

    /// THE CARRY RULE, in one function (CC 5.2 "What a row carries"; formal/causal_kernel/route.py at 820a7a0): who a node
    /// may offer `row` to, and whether the row is inert there. An inert row carries nobody but its signers (no member,
    /// stranger or seat); it waits for its cure, which reaches it only under the cure's own audience, by this rule applied
    /// where the cure is held. A live row: `audience`. The same rule as docs/kernel/kernel.js `Mesh.carry`.
    pub fn carry(&self, origin_node_id: &str, row: &Row) -> (Option<HashSet<String>>, bool) {
        let Some(node) = self.nodes.get(origin_node_id) else { return (Some(HashSet::new()), false) };
        let f = fold(&node.held, &self.tables, self.now);
        if f.inert.contains_key(&row.id) {
            return (Some(std::iter::once(row.signer.clone()).collect()), true);
        }
        let roster_row = row.family == "group:roster" || row.family == "membership:{stage}";
        let room_tier = row.scope.as_ref().is_some_and(|s| s.tier == "room");
        let consent_free = CONSENT_FREE.contains(&row.family.as_str());
        if let (Some(_), false, true, false) = (self.heads.get(&row.id), roster_row, room_tier, consent_free) {
            // judged at the row's own head; an undetermined head carries to nobody yet
            return (Some(self.room_audience(node, row, &f, &mut HashSet::new()).unwrap_or_default()), false);
        }
        (self.audience_in(&f, row), false)
    }

    /// A ROOM ROW'S AUDIENCE (CC 5.2 "What a row carries"; formal/causal_kernel/route.py `closed` and Carry.lean at
    /// 2fb305c): the room's members at the row's own head, its signer and the key it names; then carriage: a concluded
    /// row of the same room (or by the same signer) that names it in its head, whose signer is already in its audience,
    /// adds that row's audience. Judged from the rows the carrier holds: a head not held whole is not judged (None).
    /// Roster and membership rows are the one ledger every member concludes and keep `audience_in`. As
    /// docs/kernel/kernel.js `roomAudience`.
    fn room_audience(&self, node: &MeshNode, row: &Row, f: &FoldState, seen: &mut HashSet<String>) -> Option<HashSet<String>> {
        if !seen.insert(row.id.clone()) {
            return Some(HashSet::new());
        }
        let head = self.heads.get(&row.id)?;
        let room = row.scope.as_ref().and_then(|s| s.room.clone()).unwrap_or_default();
        if !head.iter().all(|id| node.held.iter().any(|h| &h.row.id == id)) {
            return None;
        }
        let is_roster = |x: &Row| x.scope.as_ref().and_then(|s| s.room.as_deref()) == Some(room.as_str()) && (x.family == "group:roster" || x.family == "membership:{stage}");
        let at_head: Vec<HeldRow> = node.held.iter().filter(|h| !is_roster(&h.row) || head.contains(&h.row.id)).cloned().collect();
        let fh = fold(&at_head, &self.tables, self.now);
        let mut who: HashSet<String> = HashSet::new();
        if let Some(r) = fh.rooms.get(&room) {
            who.extend(r.members.iter().cloned());
            who.extend(r.founders.iter().cloned());
        }
        who.insert(row.signer.clone());
        if let Some(s) = &row.subject {
            who.insert(s.clone());
        }
        for h in &node.held {
            let d = &h.row;
            if d.id == row.id || f.inert.contains_key(&d.id) || !self.heads.get(&d.id).is_some_and(|dh| dh.contains(&row.id)) {
                continue;
            }
            let same_room = d.scope.as_ref().and_then(|s| s.room.as_deref()) == Some(room.as_str());
            if !(same_room || d.signer == row.signer) || !who.contains(&d.signer) {
                continue; // Carry.lean `may`, and the namer's signer already in the audience
            }
            if let Some(a) = self.room_audience(node, d, f, seen) {
                who.extend(a);
            }
        }
        Some(who)
    }

    /// The live roster and membership rows of `room` a node holds: the head a new room row names.
    pub fn head_of(&self, node_id: &str, room: &str) -> Vec<String> {
        let Some(node) = self.nodes.get(node_id) else { return Vec::new() };
        let f = fold(&node.held, &self.tables, self.now);
        let mut v: Vec<String> = node
            .held
            .iter()
            .map(|h| &h.row)
            .filter(|r| !f.inert.contains_key(&r.id) && r.scope.as_ref().and_then(|s| s.room.as_deref()) == Some(room) && (r.family == "group:roster" || r.family == "membership:{stage}"))
            .map(|r| r.id.clone())
            .collect();
        v.sort();
        v
    }

    pub fn audience(&self, origin_node_id: &str, row: &Row) -> Option<HashSet<String>> {
        let node = self.nodes.get(origin_node_id)?;
        let f = fold(&node.held, &self.tables, self.now);
        self.audience_in(&f, row)
    }

    fn audience_in(&self, f: &FoldState, row: &Row) -> Option<HashSet<String>> {
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
        if let Some(room) = row.scope.as_ref().filter(|s| s.tier == "room").and_then(|s| s.room.clone()) {
            if !self.heads.contains_key(&row.id) {
                let h = self.head_of(node_id, &room);
                self.heads.insert(row.id.clone(), h);
            }
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

                    let (aud, inert) = self.carry(&origin, &row);
                    let other_carrier = self.nodes[&nb].carrier;
                    let other_owner = self.nodes[&nb].owner.clone();
                    let other_id = self.nodes[&nb].id.clone();
                    if inert && !aud.as_ref().is_some_and(|a| a.contains(&other_owner)) {
                        continue; // an inert row goes to its signers only: no member, no seat
                    }

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

                    if s.is_ok() || s.as_ref().err().is_some_and(|r| r.is_waiting()) {
                        self.nodes.get_mut(&node_id).unwrap().offered.insert(key); // kept, live or inert
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
