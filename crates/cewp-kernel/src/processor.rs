//! The CC §4.5.13 state processor: fold, standing, enabled_for, enabled, step, door, queue, preview.

use crate::tables::{expected_active, policy_row, transition_of, Tables};
use crate::types::{
    Ballot, ChangedObject, FoldState, Fraction, HaltState, HeldRow, Missing, ObjectState, Outcome, Pending,
    Plan, PreviewImpact, PreviewResult, PreviewVerdict, Queue, QueueItem, QueueStart, Refusal, RoomDiff,
    RoomState, Row, Scope, StepResult, TargetRef, Transition, Voices,
};
use std::collections::{HashMap, HashSet};

struct Duty {
    #[allow(dead_code)]
    from: String,
    to: String,
    #[allow(dead_code)]
    scope: String,
    room: Option<String>,
}

pub fn irv(ballots: &[Ballot]) -> Option<String> {
    if ballots.is_empty() {
        return None;
    }
    let mut seen: HashMap<String, Vec<String>> = HashMap::new();
    for b in ballots {
        seen.insert(b.signer.clone(), b.ranking.clone());
    }

    let mut alive: HashSet<String> = HashSet::new();
    for ranking in seen.values() {
        for c in ranking {
            alive.insert(c.clone());
        }
    }

    while alive.len() > 1 {
        let mut tally: HashMap<String, usize> = HashMap::new();
        for c in &alive {
            tally.insert(c.clone(), 0);
        }
        for ranking in seen.values() {
            if let Some(first) = ranking.iter().find(|c| alive.contains(*c)) {
                if let Some(count) = tally.get_mut(first) {
                    *count += 1;
                }
            }
        }

        let total: usize = tally.values().sum();
        for (c, n) in &tally {
            if *n * 2 > total {
                return Some(c.clone());
            }
        }

        let min_val = match tally.values().min() {
            Some(m) => *m,
            None => break,
        };

        let mut losers: Vec<String> = tally
            .iter()
            .filter(|(_, n)| **n == min_val)
            .map(|(c, _)| c.clone())
            .collect();
        losers.sort();

        if let Some(loser) = losers.first() {
            alive.remove(loser);
        } else {
            break;
        }
    }

    let mut remaining: Vec<String> = alive.into_iter().collect();
    remaining.sort();
    remaining.into_iter().next()
}

pub fn fold(held: &[HeldRow], tables: &Tables, now: u64) -> FoldState {
    let mut rows: Vec<HeldRow> = held.to_vec();
    rows.sort_by(|a, b| {
        a.row
            .asserted_at
            .cmp(&b.row.asserted_at)
            .then_with(|| a.row.id.cmp(&b.row.id))
    });

    let mut objects: HashMap<String, ObjectState> = HashMap::new();
    let mut object_order: Vec<String> = Vec::new();
    let mut registered: HashSet<String> = HashSet::new();
    let mut rooms: HashMap<String, RoomState> = HashMap::new();
    let mut room_order: Vec<String> = Vec::new();
    let mut holders: HashSet<String> = HashSet::new();
    let mut nodes_of: HashMap<String, HashSet<String>> = HashMap::new();
    let mut duties: Vec<Duty> = Vec::new();
    let mut halt = HaltState {
        latched: false,
        by: None,
        since: None,
        confirmed: false,
    };
    let mut roles_of: HashMap<String, HashSet<String>> = HashMap::new();
    let mut system_identities: HashSet<String> = HashSet::new();
    let mut owners_of: HashMap<String, HashSet<String>> = HashMap::new();
    let mut attesters: HashSet<String> = HashSet::new();
    let mut device_to_fed_id: HashMap<String, String> = HashMap::new();

    let mut add_role = |person: &str, role: &str| {
        if !person.is_empty() && !role.is_empty() {
            roles_of.entry(person.to_string()).or_default().insert(role.to_string());
        }
    };

    for h in &rows {
        let row = &h.row;
        let mut fam_key = row.family.clone();
        let mut binds = HashMap::new();
        if !tables.families.contains_key(&fam_key) {
            let m = tables.match_family(&row.family);
            if let Some(ref fam) = m.family {
                fam_key = fam.clone();
                binds = m.binds;
            }
        }

        // Fold self:roster rows so device keys map to their fedID (CC §4.5.13)
        if row.family.starts_with("self:roster") || row.family == "self:roster" || fam_key.starts_with("self_roster") {
            if let Some(fed) = &row.subject {
                device_to_fed_id.insert(row.signer.clone(), fed.clone());
            }
        }

        if !objects.contains_key(&row.object) {
            let o = ObjectState {
                id: row.object.clone(),
                family: fam_key.clone(),
                raw_family: Some(row.family.clone()),
                binds,
                state: "none".to_string(),
                rows: Vec::new(),
                scope: row
                    .scope
                    .clone()
                    .unwrap_or(Scope {
                        tier: "self".to_string(),
                        room: None,
                    }),
                author: row.signer.clone(),
                subject: row.subject.clone(),
                pending: None,
                outcome: None,
                missing: None,
                voices: None,
                last: None,
            };
            objects.insert(row.object.clone(), o);
            object_order.push(row.object.clone());
        }

        let o = objects.get_mut(&row.object).unwrap();
        o.rows.push(h.clone());

        let is_vote = row.stance.is_some()
            || row.ranking.is_some()
            || row.verb == "voice"
            || row.verb == "assent"
            || row.verb == "raised"
            || row.verb == "object";

        let t = if o.state == "none" && is_vote {
            None
        } else {
            transition_of(tables, &fam_key, &row.verb, &o.state)
        };

        if let Some(t) = t {
            if o.state == "none" {
                o.author = row.signer.clone();
                o.subject = row.subject.clone();
                if let Some(s) = &row.scope {
                    o.scope = s.clone();
                }
            }
            let p = policy_row(tables, &t);
            match p {
                None => {
                    o.state = t.to.clone();
                    o.last = Some(h.clone());
                }
                Some(pol) => {
                    o.state = t.to.clone();
                    o.last = Some(h.clone());
                    o.pending = Some(Pending {
                        policy: pol.clone(),
                        transition: t.clone(),
                        since: h.first_held,
                        opener: row.signer.clone(),
                    });
                }
            }
        }
    }

    // policy folds
    for obj_id in &object_order {
        let o = objects.get_mut(obj_id).unwrap();
        if o.pending.is_none() {
            continue;
        }

        let p = o.pending.as_ref().unwrap().policy.clone();
        let opener = o.pending.as_ref().unwrap().opener.clone();
        let mut ballots: Vec<Ballot> = Vec::new();

        let get_fed = |signer: &str| -> String {
            device_to_fed_id.get(signer).cloned().unwrap_or_else(|| signer.to_string())
        };

        let opener_fed = get_fed(&opener);
        let mut fed_for: HashSet<String> = HashSet::new();
        let mut fed_against: HashSet<String> = HashSet::new();
        let mut fed_disagree: HashSet<String> = HashSet::new();

        fed_for.insert(opener_fed);

        for h in &o.rows {
            let r = &h.row;
            let fed = get_fed(&r.signer);
            let is_approval = r.stance.as_deref() == Some("for")
                || (r.family.starts_with("objection") && r.verb == "assent");
            let is_objection = r.stance.as_deref() == Some("against")
                || r.family == "objection"
                || (r.family.starts_with("objection") && (r.verb == "raised" || r.verb == "object"));

            if is_approval && is_objection {
                fed_disagree.insert(fed.clone());
            } else if is_approval {
                if fed_against.contains(&fed) {
                    fed_disagree.insert(fed.clone());
                }
                fed_for.insert(fed.clone());
            } else if is_objection {
                if fed_for.contains(&fed) {
                    fed_disagree.insert(fed.clone());
                }
                fed_against.insert(fed.clone());
            }

            if let Some(ranking) = &r.ranking {
                ballots.push(Ballot {
                    signer: r.signer.clone(),
                    ranking: ranking.clone(),
                });
            }
        }

        // CC Rule: Disagreeing live rows remove a fedID from both for and against tallies (no voice)
        for fed in &fed_disagree {
            fed_for.remove(fed);
            fed_against.remove(fed);
        }

        let for_count = fed_for.len() as u64;
        let against_count = fed_against.len() as u64;

        // Base law: compute base from policy domain
        let base: u64 = if p.corridor.as_deref() == Some("self") || p.family.starts_with("self_roster") {
            let fed = get_fed(&opener);
            let live_keys = device_to_fed_id.iter().filter(|(_, f_id)| *f_id == &fed).count().max(1) as u64;
            live_keys
        } else if let Some(rid) = &o.scope.room {
            if let Some(rm) = rooms.get(rid) {
                let raw_b = (rm.members.len() + rm.founders.len()).max(2) as u64;
                if p.base.as_deref() == Some("active") {
                    expected_active(raw_b)
                } else {
                    raw_b
                }
            } else {
                2
            }
        } else if p.family == "accord_halt" {
            holders.len().max(1) as u64
        } else {
            2
        };

        // Minimum voice rule (CC 1.13.3)
        let min_count = if p.corridor.as_deref() == Some("self") || p.family.starts_with("self_roster") || p.family == "accord_halt" {
            1u64
        } else {
            2u64
        };

        let window_secs = p.window_secs.unwrap_or(0);
        let window_end = o.pending.as_ref().unwrap().since + window_secs * 1000;

        // Thresholds
        // Enact is strictly more: (num * base) / den + 1
        let ev = p.enact.map(|e| (e.num * base) / e.den + 1).unwrap_or(1);
        let cv = p.cancel.map(|c| min_count.max((c.num * base + c.den - 1) / c.den));
        let fv = p.floor.map(|f| min_count.max((f.num * base + f.den - 1) / f.den));

        let mut outcome: Option<Outcome> = None;
        let mut missing: Option<Missing> = None;

        match p.kind.as_str() {
            "two_path" => {
                if for_count >= ev && !p.irreversible {
                    outcome = Some(Outcome::Enacted);
                } else if now < window_end {
                    outcome = None;
                    let needed = (ev as i64) - (for_count as i64);
                    missing = Some(Missing {
                        voices: Some(needed),
                        until: Some(window_end),
                        ballots: None,
                    });
                } else if cv.is_some() && against_count >= cv.unwrap() {
                    outcome = Some(Outcome::Cancelled);
                } else if for_count >= ev {
                    outcome = Some(if p.irreversible {
                        Outcome::EnactedIrreversible
                    } else {
                        Outcome::Enacted
                    });
                } else if p.reverse_path && fv.is_some() && for_count >= fv.unwrap() {
                    outcome = Some(Outcome::Enacted);
                } else {
                    outcome = Some(Outcome::Cancelled);
                }
            }
            "mirror" => {
                if cv.is_some() && against_count >= cv.unwrap() {
                    outcome = Some(Outcome::Reversed);
                } else if now >= window_end {
                    outcome = Some(Outcome::Stands);
                } else {
                    outcome = Some(Outcome::Acting);
                    missing = Some(Missing {
                        voices: None,
                        until: Some(window_end),
                        ballots: None,
                    });
                }
            }
            "cosigned" => {
                let frac = p.enact.unwrap_or(Fraction { num: 1, den: 2 });
                let required = (frac.num * base) / frac.den + 1;
                if for_count >= required {
                    outcome = Some(Outcome::Enacted);
                } else {
                    let needed = (required as i64) - (for_count as i64);
                    missing = Some(Missing {
                        voices: Some(needed),
                        until: None,
                        ballots: None,
                    });
                }
            }
            "ranked" => {
                if now >= window_end {
                    outcome = irv(&ballots).map(Outcome::Seated);
                } else {
                    missing = Some(Missing {
                        voices: None,
                        until: Some(window_end),
                        ballots: Some(ballots.len()),
                    });
                }
            }
            _ => {}
        }

        let mut sorted_for: Vec<String> = fed_for.into_iter().collect();
        sorted_for.sort();
        let mut sorted_against: Vec<String> = fed_against.into_iter().collect();
        sorted_against.sort();

        o.outcome = outcome;
        o.missing = missing;
        o.voices = Some(Voices {
            for_voices: sorted_for,
            against_voices: sorted_against,
        });
    }

    // standing folds across families
    for obj_id in &object_order {
        let o = &objects[obj_id];
        let live = o.state == "admitted";

        if o.family == "key:record" && live {
            registered.insert(o.author.clone());
        }
        if o.family == "transport:{kind}" && live {
            nodes_of.entry(o.author.clone()).or_default().insert(o.id.clone());
        }
        if o.family == "trust:{job}:{version}" && live {
            holders.insert(o.author.clone());
            for h in &o.rows {
                if h.row.stance.as_deref() == Some("for") {
                    holders.insert(h.row.signer.clone());
                }
            }
            let mut job = o.binds.get("job").cloned();
            if job.as_ref().map(|j| j.contains('{')).unwrap_or(true) {
                if let Some(ref raw) = o.raw_family {
                    let parts: Vec<&str> = raw.split(':').collect();
                    if parts.len() > 1 && !parts[1].contains('{') {
                        job = Some(parts[1].to_string());
                    }
                }
            }
            if job.as_ref().map(|j| j.contains('{')).unwrap_or(true) {
                let parts: Vec<&str> = o.id.split(':').collect();
                if parts.len() > 2 && !parts[1].contains('{') {
                    job = Some(parts[1..parts.len() - 1].join(":"));
                } else if parts.len() > 1 && !parts[1].contains('{') {
                    job = Some(parts[1].to_string());
                }
            }
            if job.as_ref().map(|j| j.contains('{')).unwrap_or(true) {
                if let Some(first_row) = o.rows.first() {
                    job = first_row.row.role.clone().or_else(|| first_row.row.job.clone());
                }
            }
            if let Some(j) = job {
                add_role(&o.author, &j);
                if j == "attest" {
                    attesters.insert(o.author.clone());
                }
            }
        }
        if (o.family.starts_with("role:") || o.family == "role:{job}") && live {
            let mut job = o.binds.get("job").cloned();
            if job.is_none() {
                if let Some(ref raw) = o.raw_family {
                    let parts: Vec<&str> = raw.split(':').collect();
                    if parts.len() > 1 {
                        job = Some(parts[1].to_string());
                    }
                }
            }
            if job.is_none() {
                let parts: Vec<&str> = o.id.split(':').collect();
                if parts.len() > 1 {
                    job = Some(parts[1].to_string());
                }
            }
            if let Some(j) = job {
                add_role(&o.author, &j);
            }
        }
        if o.family == "ownership:{relation}:{target_kind}:{version}" && live {
            let target = o.subject.as_ref().unwrap_or(&o.id);
            owners_of.entry(target.clone()).or_default().insert(o.author.clone());
        }
        if o.family == "group:roster" && live {
            let rid = o.scope.room.as_deref().unwrap_or(o.id.as_str());
            if !rooms.contains_key(rid) {
                rooms.insert(
                    rid.to_string(),
                    RoomState {
                        id: rid.to_string(),
                        founders: HashSet::new(),
                        members: HashSet::new(),
                        removed: HashSet::new(),
                        moderators: HashSet::new(),
                        locked: false,
                    },
                );
                room_order.push(rid.to_string());
            }
            rooms.get_mut(rid).unwrap().founders.insert(o.author.clone());
        }
        if o.family == "membership:{stage}" && o.subject.is_some() {
            if let Some(rid) = &o.scope.room {
                if !rooms.contains_key(rid) {
                    rooms.insert(
                        rid.clone(),
                        RoomState {
                            id: rid.clone(),
                            founders: HashSet::new(),
                            members: HashSet::new(),
                            removed: HashSet::new(),
                            moderators: HashSet::new(),
                            locked: false,
                        },
                    );
                    room_order.push(rid.clone());
                }
                let subject = o.subject.as_ref().unwrap();
                let r = rooms.get_mut(rid).unwrap();
                if live && o.outcome == Some(Outcome::Enacted) {
                    r.members.insert(subject.clone());
                }
                if o.state == "withdrawn" {
                    r.members.remove(subject);
                    r.removed.insert(subject.clone());
                }
            }
        }
        if (o.family == "delegates_to" || o.family == "self:delegates_to") && live {
            if let Some(subject) = &o.subject {
                duties.push(Duty {
                    from: o.author.clone(),
                    to: subject.clone(),
                    scope: "moderate".to_string(),
                    room: o.scope.room.clone(),
                });
            }
        }
        if o.family.starts_with("accord:invoke:constitutional:") && live {
            halt.latched = true;
            halt.by = Some(o.author.clone());
            halt.since = o.last.as_ref().map(|l| l.first_held);
            let for_len = o.voices.as_ref().map(|v| v.for_voices.len()).unwrap_or(0);
            halt.confirmed = for_len * 2 > holders.len().max(1);
        }
        if o.family == "ballot:moderator" {
            if let Some(Outcome::Seated(seated)) = &o.outcome {
                if let Some(rid) = &o.scope.room {
                    if !rooms.contains_key(rid) {
                        rooms.insert(
                            rid.clone(),
                            RoomState {
                                id: rid.clone(),
                                founders: HashSet::new(),
                                members: HashSet::new(),
                                removed: HashSet::new(),
                                moderators: HashSet::new(),
                                locked: false,
                            },
                        );
                        room_order.push(rid.clone());
                    }
                    rooms.get_mut(rid).unwrap().moderators.insert(seated.clone());
                }
            }
        }
        if o.family == "system:*" && live {
            system_identities.insert(o.author.clone());
        }
    }

    for r in rooms.values_mut() {
        for f in &r.founders {
            r.moderators.insert(f.clone());
        }
        for d in &duties {
            if d.room.as_deref() == Some(&r.id) {
                r.moderators.insert(d.to.clone());
            }
        }
        r.locked = r.moderators.is_empty();
    }

    // Halt duration read from policy table window_secs
    let halt_window_ms = tables
        .policies
        .get("accord_halt")
        .and_then(|p| p.window_secs)
        .unwrap_or(86400)
        * 1000;

    if halt.latched && !halt.confirmed {
        if let Some(since) = halt.since {
            if now.saturating_sub(since) > halt_window_ms {
                halt.latched = false;
            }
        }
    }

    FoldState {
        objects,
        object_order,
        registered,
        rooms,
        room_order,
        holders,
        nodes_of,
        halt,
        roles_of,
        system_identities,
        owners_of,
        attesters,
        now,
    }
}

pub fn standing_target(
    f: &FoldState,
    person: &str,
    target: &TargetRef,
) -> HashSet<String> {
    let mut roles = HashSet::new();
    if f.registered.contains(person) {
        roles.insert("registered".to_string());
    }
    if f.holders.contains(person) {
        roles.insert("holder".to_string());
    }

    let o = target
        .id
        .and_then(|id| f.objects.get(id))
        .or_else(|| target.object.and_then(|id| f.objects.get(id)));
    let room_id = o.and_then(|obj| obj.scope.room.as_deref()).or(target.room);

    if let Some(rid) = room_id {
        if let Some(r) = f.rooms.get(rid) {
            if r.founders.contains(person) {
                roles.insert("founder".to_string());
                roles.insert("member".to_string());
                roles.insert("moderate".to_string());
            }
            if r.members.contains(person) {
                roles.insert("member".to_string());
            }
            if r.moderators.contains(person) {
                roles.insert("moderate".to_string());
            }
        }
    }

    let is_author = o.map(|obj| obj.author == person).unwrap_or(false)
        || target.author == Some(person);
    let subj = o.and_then(|obj| obj.subject.as_deref()).or(target.subject);

    if o.is_some() || target.author.is_some() || target.subject.is_some() {
        if is_author {
            roles.insert("author".to_string());
            roles.insert("self".to_string());
        }
        if subj == Some(person) {
            roles.insert("subject".to_string());
        }
    } else {
        roles.insert("self".to_string());
    }

    // group:roster founding bootstrap:
    // When target/object is group:roster and state is none:
    // registered person has founder + member standing.
    let is_group_roster = o.map(|obj| obj.family == "group:roster").unwrap_or(false)
        || target.id.map(|id| id.starts_with("room:")).unwrap_or(false);
    if is_group_roster && f.registered.contains(person) {
        roles.insert("founder".to_string());
        roles.insert("member".to_string());
    }

    // --- Specialized Roles (Deriving constitutional role tokens from held rows, no hard-coded authority roster) ---
    let target_id = o.map(|obj| obj.id.as_str()).or(target.id).or(target.object);
    let is_target_owner = target_id
        .and_then(|tid| f.owners_of.get(tid))
        .map(|s| s.contains(person))
        .unwrap_or(false)
        || subj
            .and_then(|s| f.owners_of.get(s))
            .map(|s| s.contains(person))
            .unwrap_or(false);
    let is_room_founder = room_id
        .and_then(|rid| f.rooms.get(rid))
        .map(|r| r.founders.contains(person))
        .unwrap_or(false);

    if subj == Some(person) || is_target_owner || is_room_founder || is_author {
        roles.insert("owner".to_string());
    }

    // attest role
    if f.attesters.contains(person)
        || f.roles_of.get(person).map(|r| r.contains("attest")).unwrap_or(false)
        || f.holders.contains(person)
    {
        roles.insert("attest".to_string());
    }

    // roles from held trust / role rows
    if let Some(person_roles) = f.roles_of.get(person) {
        for r in person_roles {
            roles.insert(r.clone());
        }
    }

    roles
}

pub fn standing(f: &FoldState, person: &str, object: &str) -> Vec<String> {
    let target = if object.is_empty() {
        TargetRef::default()
    } else if f.objects.contains_key(object) {
        TargetRef {
            id: Some(object),
            object: Some(object),
            ..Default::default()
        }
    } else if f.rooms.contains_key(object) || object.starts_with("room:") {
        TargetRef {
            room: Some(object),
            tier: Some("room"),
            ..Default::default()
        }
    } else {
        TargetRef {
            id: Some(object),
            object: Some(object),
            ..Default::default()
        }
    };
    let roles = standing_target(f, person, &target);
    let mut sorted: Vec<String> = roles.into_iter().collect();
    sorted.sort();
    sorted
}

pub fn enabled_for(tables: &Tables, family: &str, state: &str, roles: &[String]) -> Vec<Transition> {
    let mut fam_key = family;
    let resolved;
    if !tables.families.contains_key(fam_key) {
        let m = tables.match_family(family);
        if let Some(ref fam) = m.family {
            if tables.families.contains_key(fam) {
                resolved = fam.clone();
                fam_key = &resolved;
            }
        }
    }
    let ts = match tables.families.get(fam_key) {
        Some(ts) => ts,
        None => return Vec::new(),
    };
    let rs: HashSet<&str> = roles.iter().map(|s| s.as_str()).collect();
    ts.iter()
        .filter(|t| {
            t.from == state
                && !t.roles.is_empty()
                && (t.roles.iter().any(|r| rs.contains(r.as_str()))
                    || (t.roles.iter().any(|r| r == "registered") && !rs.is_empty() && rs.contains("registered")))
        })
        .cloned()
        .collect()
}

/// Unified enablement checking: the single gate for both the queue and the door (Queue == Door).
pub fn enabled(
    f: &FoldState,
    tables: &Tables,
    person: &str,
    target: &TargetRef,
    family_filter: Option<&str>,
) -> Vec<Transition> {
    // 1. Halt Gate: checked against role state / charter rather than self-declared only
    let is_agent = target.signer_type == Some("agent")
        || f.roles_of.get(person).map(|r| r.contains("agent")).unwrap_or(false)
        || f.objects.values().any(|o| {
            o.family.starts_with("agent:charter")
                && o.subject.as_deref() == Some(person)
                && o.state == "admitted"
        });

    if f.halt.latched && is_agent {
        return Vec::new();
    }

    // 2. Registration Gate:
    let is_reg = f.registered.contains(person);

    let target_id = target.id.or(target.object);
    let o = target_id.and_then(|id| f.objects.get(id));

    let mut out = Vec::new();

    if let Some(o) = o {
        // Family mismatch check:
        if let Some(req_fam) = family_filter {
            let m = tables.match_family(req_fam);
            let fam_key = m.family.as_deref().unwrap_or(req_fam);
            if o.family != fam_key {
                return Vec::new();
            }
        }

        // Room audience check:
        let is_room_audience = o.scope.tier == "room" || target.tier == Some("room");
        if is_room_audience {
            let room_id = o.scope.room.as_deref().or(target.room);
            if let Some(rid) = room_id {
                let r = f.rooms.get(rid);
                if r.is_none() && o.family != "group:roster" {
                    return Vec::new();
                }
                if let Some(rm) = r {
                    let in_room = rm.members.contains(person) || rm.founders.contains(person);
                    let invitee = o.family == "membership:{stage}" && o.subject.as_deref() == Some(person);
                    if !in_room && !invitee {
                        return Vec::new();
                    }
                    if let Some(subj) = target.subject {
                        let subj_in_room = rm.members.contains(subj) || rm.founders.contains(subj);
                        if !subj_in_room && o.family != "membership:{stage}" {
                            return Vec::new();
                        }
                    }
                }
            }
        }

        let roles = standing_target(f, person, target);
        if !is_reg && o.family != "key:record" {
            return Vec::new();
        }

        if let Some(ts) = tables.families.get(&o.family) {
            for t in ts {
                if t.from == o.state && !t.roles.is_empty() && t.roles.iter().any(|r| roles.contains(r)) {
                    out.push(t.clone());
                }
            }
        }

        // Synthesized vote transition for pending policy objects
        if o.pending.is_some() && (roles.contains("member") || roles.contains("subject") || roles.contains("holder")) {
            let pol = o.pending.as_ref().unwrap().policy.kind.clone();
            let vote_roles = vec!["member".to_string(), "subject".to_string(), "holder".to_string()];
            for v_verb in &["voice", "assent", "raised", "object"] {
                out.push(Transition {
                    verb: v_verb.to_string(),
                    from: o.state.clone(),
                    to: o.state.clone(),
                    roles: vote_roles.clone(),
                    policy: pol.clone(),
                    policy_rows: Vec::new(),
                    widens: false,
                });
            }
        }
    } else {
        // Starts (transitions from "none")
        let is_room_audience = target.tier == Some("room");
        if is_room_audience {
            if let Some(rid) = target.room {
                let r = f.rooms.get(rid);
                let is_roster = family_filter
                    .map(|f| tables.match_family(f).family.as_deref() == Some("group:roster"))
                    .unwrap_or(false);
                if r.is_none() && !is_roster {
                    return Vec::new();
                }
                if let Some(rm) = r {
                    let in_room = rm.members.contains(person) || rm.founders.contains(person);
                    let invitee = family_filter
                        .map(|f| tables.match_family(f).family.as_deref() == Some("membership:{stage}"))
                        .unwrap_or(false)
                        && target.subject == Some(person);
                    if !in_room && !invitee && !is_roster {
                        return Vec::new();
                    }
                }
            }
        }

        let roles = standing_target(f, person, target);

        let resolved_fam = family_filter.map(|req_fam| {
            tables.match_family(req_fam).family.unwrap_or_else(|| req_fam.to_string())
        });
        let families_to_check: Vec<&str> = if let Some(ref fam) = resolved_fam {
            vec![fam.as_str()]
        } else {
            tables.family_order.iter().map(|s| s.as_str()).collect()
        };

        for fam in families_to_check {
            if !is_reg && fam != "key:record" {
                continue;
            }
            if let Some(ts) = tables.families.get(fam) {
                for t in ts {
                    if t.from == "none" && !t.roles.is_empty() && t.roles.iter().any(|r| roles.contains(r)) {
                        out.push(t.clone());
                    }
                }
            }
        }

        // Out-of-order votes on uncreated objects (CC CIRISPersist#1198)
        if is_reg && target.id.is_some() {
            let vote_roles = vec!["member".to_string(), "subject".to_string(), "holder".to_string()];
            for v_verb in &["voice", "assent", "raised", "object"] {
                out.push(Transition {
                    verb: v_verb.to_string(),
                    from: "none".to_string(),
                    to: "none".to_string(),
                    roles: vote_roles.clone(),
                    policy: "two_path".to_string(),
                    policy_rows: Vec::new(),
                    widens: false,
                });
            }
        }
    }

    out
}

pub fn step(held: &[HeldRow], tables: &Tables, row: &Row, now: u64) -> StepResult {
    if held.iter().any(|h| h.row.id == row.id) {
        return Err(Refusal::AlreadyHeld);
    }

    let f = fold(held, tables, now);

    // Resolve wire family
    let mut fam_key = row.family.clone();
    if !tables.families.contains_key(&fam_key) {
        let m = tables.match_family(&row.family);
        if let Some(ref refusal) = m.refusal {
            return Err(Refusal::from(refusal.as_str()));
        }
        if let Some(ref fam) = m.family {
            fam_key = fam.clone();
        }
    }

    let o = f.objects.get(&row.object);
    if let Some(o) = o {
        if o.family != fam_key {
            return Err(Refusal::FamilyMismatch);
        }
    }

    let target = TargetRef {
        id: Some(&row.object),
        object: Some(&row.object),
        subject: row.subject.as_deref(),
        author: Some(&row.signer),
        room: row.scope.as_ref().and_then(|s| s.room.as_deref()),
        signer_type: row.signer_type.as_deref(),
        tier: row.scope.as_ref().map(|s| s.tier.as_str()),
    };

    let is_vote = row.stance.is_some()
        || row.ranking.is_some()
        || row.verb == "voice"
        || row.verb == "assent"
        || row.verb == "raised"
        || row.verb == "object";

    let enabled_transitions = enabled(&f, tables, &row.signer, &target, Some(&row.family));
    let t = enabled_transitions.into_iter().find(|x| x.verb == row.verb || (x.verb == "voice" && is_vote));

    if let Some(t) = t {
        let policy = policy_row(tables, &t).cloned();
        return Ok(Plan {
            row: row.clone(),
            transition: t,
            first_held: now,
            policy,
        });
    }

    // Determine exact refusal:
    let is_agent = row.signer_type.as_deref() == Some("agent")
        || f.roles_of.get(&row.signer).map(|r| r.contains("agent")).unwrap_or(false)
        || f.objects.values().any(|obj| {
            obj.family.starts_with("agent:charter")
                && obj.subject.as_deref() == Some(&row.signer)
                && obj.state == "admitted"
        });
    if f.halt.latched && is_agent {
        return Err(Refusal::Halted);
    }
    if let Some(o) = o {
        if o.family != fam_key {
            return Err(Refusal::FamilyMismatch);
        }
    }
    let is_reg = f.registered.contains(&row.signer);
    if !is_reg && fam_key != "key:record" {
        return Err(Refusal::Unregistered);
    }
    let is_room_tier = row.scope.as_ref().map(|s| s.tier.as_str()) == Some("room");
    if is_room_tier {
        if let Some(rid) = row.scope.as_ref().and_then(|s| s.room.as_deref()) {
            let r = f.rooms.get(rid);
            if r.is_none() && fam_key != "group:roster" {
                return Err(Refusal::NoSuchRoom);
            }
            if let Some(rm) = r {
                let in_room = rm.members.contains(&row.signer) || rm.founders.contains(&row.signer);
                let invitee = fam_key == "membership:{stage}" && row.subject.as_deref() == Some(&row.signer);
                if !in_room && !invitee {
                    return Err(Refusal::SubjectOutsideAudience);
                }
                if let Some(subj) = &row.subject {
                    let subj_in = rm.members.contains(subj) || rm.founders.contains(subj);
                    if !subj_in && fam_key != "membership:{stage}" {
                        return Err(Refusal::SubjectOutsideAudience);
                    }
                }
            }
        }
    }
    let all_ts = tables.families.get(&fam_key);
    let trans_by_verb = all_ts.map(|ts| ts.iter().filter(|x| x.verb == row.verb).collect::<Vec<_>>()).unwrap_or_default();
    if trans_by_verb.is_empty() && !is_vote {
        return Err(Refusal::NoTransition);
    }
    if trans_by_verb.iter().any(|x| x.roles.is_empty()) {
        return Err(Refusal::Closed);
    }
    Err(Refusal::NoStanding)
}

pub fn commit(held: &mut Vec<HeldRow>, plan: Plan) {
    if let Some(pos) = held.iter().position(|h| h.row.id == plan.row.id) {
        held[pos] = HeldRow {
            row: plan.row,
            first_held: plan.first_held,
        };
    } else {
        held.push(HeldRow {
            row: plan.row,
            first_held: plan.first_held,
        });
    }
}

pub fn door(held: &mut Vec<HeldRow>, tables: &Tables, row: &Row, now: u64) -> StepResult {
    let s = step(held, tables, row, now);
    if let Ok(plan) = &s {
        commit(held, plan.clone());
    }
    s
}

pub fn queue(held: &[HeldRow], tables: &Tables, person: &str, now: u64) -> Queue {
    let f = fold(held, tables, now);
    let mut items = Vec::new();

    for obj_id in &f.object_order {
        let o = &f.objects[obj_id];
        let target = TargetRef {
            id: Some(obj_id),
            object: Some(obj_id),
            subject: o.subject.as_deref(),
            author: Some(&o.author),
            room: o.scope.room.as_deref(),
            signer_type: None,
            tier: Some(&o.scope.tier),
        };
        for t in enabled(&f, tables, person, &target, None) {
            items.push(QueueItem {
                kind: "choice".to_string(),
                object: o.id.clone(),
                family: o.family.clone(),
                verb: t.verb,
                to: t.to,
                policy: t.policy,
                confirm: t.widens,
            });
        }
    }

    let mut starts = Vec::new();
    let target_none = TargetRef::default();

    for family in &tables.family_order {
        for t in enabled(&f, tables, person, &target_none, Some(family)) {
            starts.push(QueueStart {
                kind: "choice".to_string(),
                family: family.clone(),
                verb: t.verb,
                to: t.to,
                policy: t.policy,
                confirm: t.widens,
                found: None,
                room: None,
            });
        }
    }

    if f.registered.contains(person) {
        if let Some(t) = transition_of(tables, "group:roster", "attest", "none") {
            starts.push(QueueStart {
                kind: "choice".to_string(),
                family: "group:roster".to_string(),
                verb: "attest".to_string(),
                to: t.to,
                policy: t.policy,
                confirm: t.widens,
                found: Some(true),
                room: None,
            });
        }
    }

    for room_id in &f.room_order {
        let target_room = TargetRef {
            room: Some(room_id),
            tier: Some("room"),
            ..Default::default()
        };
        for family in &tables.family_order {
            for t in enabled(&f, tables, person, &target_room, Some(family)) {
                starts.push(QueueStart {
                    kind: "choice".to_string(),
                    family: family.clone(),
                    verb: t.verb,
                    to: t.to,
                    policy: t.policy,
                    confirm: t.widens,
                    found: None,
                    room: Some(room_id.clone()),
                });
            }
        }
    }

    Queue {
        items,
        starts,
        fold: f,
    }
}

pub fn preview(held: &[HeldRow], tables: &Tables, rows: &[Row], now: u64) -> PreviewResult {
    let before = fold(held, tables, now);
    let mut copy: Vec<HeldRow> = held.to_vec();
    let mut verdicts = Vec::new();

    for row in rows {
        let s = step(&copy, tables, row, now);
        let ok = s.is_ok();
        let plan = s.as_ref().ok().cloned();
        let refusal = s.as_ref().err().map(|r| r.to_string());
        if let Ok(plan) = s {
            commit(&mut copy, plan);
        }
        verdicts.push(PreviewVerdict {
            row: row.id.clone(),
            ok,
            plan,
            refusal,
        });
    }

    let after = fold(&copy, tables, now);

    let mut changed = Vec::new();
    for obj_id in &after.object_order {
        let o_after = &after.objects[obj_id];
        let o_before = before.objects.get(obj_id);
        match o_before {
            None => {
                changed.push(ChangedObject {
                    object: obj_id.clone(),
                    family: o_after.family.clone(),
                    from: "none".to_string(),
                    to: o_after.state.clone(),
                    outcome: o_after.outcome.clone(),
                    pending: o_after.missing.clone(),
                });
            }
            Some(o_prev) => {
                if o_prev.state != o_after.state || o_prev.outcome != o_after.outcome {
                    changed.push(ChangedObject {
                        object: obj_id.clone(),
                        family: o_after.family.clone(),
                        from: o_prev.state.clone(),
                        to: o_after.state.clone(),
                        outcome: o_after.outcome.clone(),
                        pending: o_after.missing.clone(),
                    });
                }
            }
        }
    }

    let mut room_diffs = Vec::new();
    for room_id in &after.room_order {
        let r_after = &after.rooms[room_id];
        let r_before = before.rooms.get(room_id);

        let mut joined = Vec::new();
        let mut left = Vec::new();

        for m in &r_after.members {
            if !r_before.map(|b| b.members.contains(m)).unwrap_or(false) {
                joined.push(m.clone());
            }
        }
        if let Some(rb) = r_before {
            for m in &rb.members {
                if !r_after.members.contains(m) {
                    left.push(m.clone());
                }
            }
        }

        joined.sort();
        left.sort();

        if !joined.is_empty() || !left.is_empty() || r_before.map(|b| b.locked != r_after.locked).unwrap_or(true) {
            room_diffs.push(RoomDiff {
                room: room_id.clone(),
                joined,
                left,
                locked: r_after.locked,
            });
        }
    }

    let widens = verdicts
        .iter()
        .any(|v| v.ok && v.plan.as_ref().map(|p| p.transition.widens).unwrap_or(false));

    let mut audience: Vec<String> = rows
        .iter()
        .map(|r| {
            r.scope
                .as_ref()
                .map(|s| s.tier.clone())
                .unwrap_or_else(|| "self".to_string())
        })
        .collect();
    audience.sort();
    audience.dedup();

    let undo: Vec<bool> = verdicts
        .iter()
        .filter(|v| v.ok)
        .map(|v| {
            let plan = v.plan.as_ref().unwrap();
            transition_of(tables, &plan.row.family, "withdraw", &plan.transition.to).is_some()
        })
        .collect();

    PreviewResult {
        verdicts,
        changed,
        rooms: room_diffs,
        impact: PreviewImpact {
            direction: if widens {
                "widens".to_string()
            } else {
                "same".to_string()
            },
            audience,
            undo,
        },
    }
}
