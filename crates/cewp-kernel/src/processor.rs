//! The CC §4.5.13 state processor: fold, standing, enabled_for, step, door, queue, preview.

use crate::tables::{policy_row, transition_of, Tables};
use crate::types::{
    Ballot, ChangedObject, FoldState, HaltState, HeldRow, Missing, ObjectState, Outcome, Pending,
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

        let t = transition_of(tables, &fam_key, &row.verb, &o.state);
        if let Some(t) = t {
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
        let mut for_voices: HashSet<String> = HashSet::new();
        for_voices.insert(opener);
        let mut against_voices: HashSet<String> = HashSet::new();
        let mut ballots: Vec<Ballot> = Vec::new();

        for h in &o.rows {
            let r = &h.row;
            if r.stance.as_deref() == Some("for") {
                for_voices.insert(r.signer.clone());
            }
            if r.stance.as_deref() == Some("against") {
                against_voices.insert(r.signer.clone());
                for_voices.remove(&r.signer);
            }
            if let Some(ranking) = &r.ranking {
                ballots.push(Ballot {
                    signer: r.signer.clone(),
                    ranking: ranking.clone(),
                });
            }
        }

        let base: u64 = if let Some(rid) = &o.scope.room {
            if let Some(rm) = rooms.get(rid) {
                (rm.members.len() + rm.founders.len()).max(2) as u64
            } else {
                2
            }
        } else {
            2
        };

        let window_secs = p.window_secs.unwrap_or(0);
        let window_end = o.pending.as_ref().unwrap().since + window_secs * 1000;

        let met = |frac: Option<crate::types::Fraction>, n: usize| -> bool {
            if let Some(f) = frac {
                (n as u64) * f.den >= f.num * base
            } else {
                false
            }
        };

        let mut outcome: Option<Outcome> = None;
        let mut missing: Option<Missing> = None;

        match p.kind.as_str() {
            "two_path" => {
                if met(p.enact, for_voices.len()) {
                    outcome = Some(Outcome::Enacted);
                } else if met(p.cancel, against_voices.len()) {
                    outcome = Some(Outcome::Cancelled);
                } else if now >= window_end {
                    outcome = Some(match p.at_window.as_deref() {
                        Some("enacted") => Outcome::Enacted,
                        _ => Outcome::Cancelled,
                    });
                } else {
                    let needed = if let Some(enact) = p.enact {
                        let num_val = (enact.num * base + enact.den - 1) / enact.den;
                        (num_val as i64) - (for_voices.len() as i64)
                    } else {
                        0
                    };
                    missing = Some(Missing {
                        voices: Some(needed),
                        until: Some(window_end),
                        ballots: None,
                    });
                }
            }
            "mirror" => {
                if met(p.cancel, against_voices.len()) {
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
                let frac = p.enact.unwrap_or(crate::types::Fraction { num: 1, den: 2 });
                if met(Some(frac), for_voices.len()) {
                    outcome = Some(Outcome::Enacted);
                } else {
                    let num_val = (frac.num * base + frac.den - 1) / frac.den;
                    let needed = (num_val as i64) - (for_voices.len() as i64);
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

        let mut sorted_for: Vec<String> = for_voices.into_iter().collect();
        sorted_for.sort();
        let mut sorted_against: Vec<String> = against_voices.into_iter().collect();
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
                if parts.len() > 1 && !parts[1].contains('{') {
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

    if halt.latched && !halt.confirmed {
        if let Some(since) = halt.since {
            if now.saturating_sub(since) > 86400 * 1000 {
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
            }
            if r.founders.contains(person) || r.members.contains(person) {
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

    // --- Specialized Roles (Deriving all 17 constitutional role tokens) ---
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

    // steward, wa, takedown, review, slash
    if let Some(person_roles) = f.roles_of.get(person) {
        for job in &["steward", "wa", "takedown", "review", "slash"] {
            if person_roles.contains(*job) {
                roles.insert(job.to_string());
            }
        }
    }

    // system identities: identity:lenscore_detector, identity:substrate_persist
    let person_roles = f.roles_of.get(person);
    if person == "identity:lenscore_detector"
        || f.system_identities.contains("identity:lenscore_detector")
        || person_roles.map(|r| r.contains("identity:lenscore_detector") || r.contains("lenscore_detector")).unwrap_or(false)
    {
        roles.insert("identity:lenscore_detector".to_string());
    }
    if person == "identity:substrate_persist"
        || f.system_identities.contains("identity:substrate_persist")
        || person_roles.map(|r| r.contains("identity:substrate_persist") || r.contains("substrate_persist")).unwrap_or(false)
    {
        roles.insert("identity:substrate_persist".to_string());
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
    let state = o.map(|x| x.state.as_str()).unwrap_or("none");
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
    };
    let mut roles = standing_target(&f, &row.signer, &target);

    // founding: a roster's first version seats a listed member iff that member signed it, no owner gate
    if o.is_none()
        && (fam_key == "group:roster" || row.family == "group:roster")
        && row.verb == "attest"
        && f.registered.contains(&row.signer)
    {
        roles.insert("founder".to_string());
        roles.insert("member".to_string());
    }

    let mut t = transition_of(tables, &fam_key, &row.verb, state);
    if t.is_none() && o.is_some() && o.unwrap().pending.is_some() && (row.stance.is_some() || row.ranking.is_some()) {
        t = Some(Transition {
            verb: "voice".to_string(),
            from: state.to_string(),
            to: state.to_string(),
            roles: vec!["member".to_string(), "subject".to_string(), "holder".to_string()],
            policy: o.unwrap().pending.as_ref().unwrap().policy.kind.clone(),
            policy_rows: Vec::new(),
            widens: false,
        });
    }

    let t = match t {
        Some(t) => t,
        None => return Err(Refusal::NoTransition),
    };

    if t.roles.is_empty() {
        return Err(Refusal::Closed);
    }
    if !t.roles.iter().any(|r| roles.contains(r)) {
        return Err(Refusal::NoStanding);
    }

    let self_key = (fam_key == "key:record" || row.family == "key:record") && row.verb == "attest";
    let is_system = row.signer == "identity:substrate_persist" || row.signer == "identity:lenscore_detector";
    if !self_key && !is_system && !f.registered.contains(&row.signer) {
        return Err(Refusal::Unregistered);
    }

    if let Some(scope) = &row.scope {
        if scope.tier == "room" {
            let room_id = scope.room.as_deref().unwrap_or("");
            let r = f.rooms.get(room_id);
            let in_room = r
                .map(|rm| rm.members.contains(&row.signer) || rm.founders.contains(&row.signer))
                .unwrap_or(false);
            let invitee = (fam_key == "membership:{stage}" || row.family == "membership:{stage}")
                && row.subject.as_deref() == Some(&row.signer);

            if r.is_none() && fam_key != "group:roster" && row.family != "group:roster" {
                return Err(Refusal::NoSuchRoom);
            }
            if r.is_some() && !in_room && !invitee {
                return Err(Refusal::SubjectOutsideAudience);
            }
            if let Some(subject) = &row.subject {
                if let Some(rm) = r {
                    if !(rm.members.contains(subject) || rm.founders.contains(subject))
                        && fam_key != "membership:{stage}"
                        && row.family != "membership:{stage}"
                    {
                        return Err(Refusal::SubjectOutsideAudience);
                    }
                }
            }
        }
    }

    if f.halt.latched && row.signer_type.as_deref() == Some("agent") {
        return Err(Refusal::Halted);
    }

    let policy = policy_row(tables, &t).cloned();
    Ok(Plan {
        row: row.clone(),
        transition: t,
        first_held: now,
        policy,
    })
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
        };
        let roles: Vec<String> = standing_target(&f, person, &target).into_iter().collect();
        for t in enabled_for(tables, &o.family, &o.state, &roles) {
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
    let global_roles: HashSet<String> = standing_target(&f, person, &target_none);

    for family in &tables.family_order {
        if let Some(ts) = tables.families.get(family) {
            for t in ts {
                if t.from == "none" && !t.roles.is_empty() && t.roles.iter().any(|r| global_roles.contains(r)) {
                    starts.push(QueueStart {
                        kind: "choice".to_string(),
                        family: family.clone(),
                        verb: t.verb.clone(),
                        to: t.to.clone(),
                        policy: t.policy.clone(),
                        confirm: t.widens,
                        found: None,
                        room: None,
                    });
                }
            }
        }
    }

    if global_roles.contains("registered") {
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
            ..Default::default()
        };
        let roles: HashSet<String> = standing_target(&f, person, &target_room);
        if !roles.contains("member") {
            continue;
        }
        for family in &tables.family_order {
            if let Some(ts) = tables.families.get(family) {
                for t in ts {
                    if t.from == "none" && !t.roles.is_empty() && t.roles.iter().any(|x| roles.contains(x)) {
                        starts.push(QueueStart {
                            kind: "choice".to_string(),
                            family: family.clone(),
                            verb: t.verb.clone(),
                            to: t.to.clone(),
                            policy: t.policy.clone(),
                            confirm: t.widens,
                            found: None,
                            room: Some(room_id.clone()),
                        });
                    }
                }
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
        let o = &after.objects[obj_id];
        let b = before.objects.get(obj_id);
        match b {
            None => {
                changed.push(ChangedObject {
                    object: obj_id.clone(),
                    family: o.family.clone(),
                    from: "none".to_string(),
                    to: o.state.clone(),
                    outcome: o.outcome.clone(),
                    pending: o.missing.clone(),
                });
            }
            Some(before_o) => {
                if before_o.state != o.state || before_o.outcome != o.outcome {
                    changed.push(ChangedObject {
                        object: obj_id.clone(),
                        family: o.family.clone(),
                        from: before_o.state.clone(),
                        to: o.state.clone(),
                        outcome: o.outcome.clone(),
                        pending: o.missing.clone(),
                    });
                }
            }
        }
    }

    let mut rooms = Vec::new();
    for (id, r) in &after.rooms {
        let b = before.rooms.get(id);
        let joined: Vec<String> = r
            .members
            .iter()
            .filter(|m| b.map(|prev| !prev.members.contains(*m)).unwrap_or(true))
            .cloned()
            .collect();
        let left: Vec<String> = b
            .map(|prev| {
                prev.members
                    .iter()
                    .filter(|m| !r.members.contains(*m))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        let locked = r.locked && !b.map(|prev| prev.locked).unwrap_or(false);

        if !joined.is_empty() || !left.is_empty() || locked {
            rooms.push(RoomDiff {
                room: id.clone(),
                joined,
                left,
                locked,
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
        rooms,
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
