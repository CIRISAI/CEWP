//! Ranked choice as one count (CC 4.5.13; cell names CC 6.1.9), as the constitution's model writes it
//! (formal/ranked_choice/ranked_choice_model.py): a SUMMARY, a count per ranking that merges by adding and is the only
//! part that touches ballots, and a FINISH, instant runoff over the summary alone, in whole numbers.
//!
//! Cells are named, never positioned: `option_id = H("ciris/poll-option/v1", poll, option)` and
//! `cell_id = H("ciris/poll-cell/v1", poll, j, option_id_1, .., option_id_j)` for a ranking of length j (j = 0 is the
//! "none" cell). H is SHA-256 over each field as a 4-byte big-endian length then its UTF-8 bytes, in lower-case hex. The
//! constitution fixes only that an entry is "named by a digest of the poll and the options it ranks, never by its
//! position" (CC 6.1.9); the labels are the store side's proposal and the framing this twin's choice, the same in
//! docs/kernel/kernel.js. The summary is a map cell_id -> count, merged by adding per key; a person's rankings are
//! resolved before adding (all the same: one ballot; any two differ: none).

use crate::tables::sha256_digest;
use std::collections::BTreeMap;

fn h(fields: &[&str]) -> String {
    let mut buf = Vec::new();
    for f in fields {
        buf.extend_from_slice(&(f.len() as u32).to_be_bytes());
        buf.extend_from_slice(f.as_bytes());
    }
    sha256_digest(&buf)
}

/// An option's id in a poll.
#[must_use]
pub fn option_id(poll: &str, option: &str) -> String {
    h(&["ciris/poll-option/v1", poll, option])
}

/// A ranking's cell id in a poll, from its options' ids best first (empty: the "none" cell).
#[must_use]
pub fn cell_id(poll: &str, option_ids: &[String]) -> String {
    let j = option_ids.len().to_string();
    let mut f: Vec<&str> = vec!["ciris/poll-cell/v1", poll, &j];
    f.extend(option_ids.iter().map(String::as_str));
    h(&f)
}

/// A summary: cell id -> count.
pub type Summary = BTreeMap<String, u64>;

/// The summary of (person, ranking) rows, and the persons whose ranking was refused (it repeats an option, names one not
/// offered, or ranks more than `ranks`): never counted.
#[must_use]
pub fn summary(poll: &str, options: &[String], rows: &[(String, Vec<String>)], ranks: usize) -> (Summary, Vec<String>) {
    let mut by: BTreeMap<&str, Vec<&Vec<String>>> = BTreeMap::new();
    for (p, r) in rows {
        let v = by.entry(p.as_str()).or_default();
        if !v.contains(&r) {
            v.push(r);
        }
    }
    let (mut cells, mut refused) = (Summary::new(), Vec::new());
    for (p, rs) in by {
        if rs.len() != 1 {
            continue; // a person whose ballots disagree has none
        }
        let r = rs[0];
        let mut seen = std::collections::BTreeSet::new();
        if r.len() > ranks || r.iter().any(|o| !options.contains(o) || !seen.insert(o)) {
            refused.push(p.to_string());
            continue;
        }
        let ids: Vec<String> = r.iter().map(|o| option_id(poll, o)).collect();
        *cells.entry(cell_id(poll, &ids)).or_default() += 1;
    }
    (cells, refused)
}

/// Two summaries added per key.
#[must_use]
pub fn merge(a: &Summary, b: &Summary) -> Summary {
    let mut c = a.clone();
    for (k, v) in b {
        *c.entry(k.clone()).or_default() += v;
    }
    c
}

/// The count's result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Finish {
    /// The winner and the rounds taken.
    Winner(String, usize),
    /// No option: no ballot still counting.
    NoWinner(usize),
    /// Fewer ballots than min(floor, options).
    BelowFloor,
    /// A cell the enumeration of rankings does not name.
    UnknownCell(String),
}

/// Instant runoff over a summary alone: the cells are resolved by enumerating the rankings (by length, then the declared
/// order), never by position; while no standing option holds more than half of the ballots still counting, the weakest
/// goes, a tie against the option later in the declared order; fewer ballots than min(floor, options) gives no winner.
#[must_use]
pub fn finish(poll: &str, options: &[String], cells: &Summary, ranks: usize, floor: u64) -> Finish {
    let ids: Vec<String> = options.iter().map(|o| option_id(poll, o)).collect();
    let mut by_cell: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    fn walk(prefix: &mut Vec<usize>, n: usize, ranks: usize, f: &mut dyn FnMut(&[usize])) {
        f(prefix);
        if prefix.len() == ranks {
            return;
        }
        for i in 0..n {
            if !prefix.contains(&i) {
                prefix.push(i);
                walk(prefix, n, ranks, f);
                prefix.pop();
            }
        }
    }
    walk(&mut Vec::new(), options.len(), ranks, &mut |p: &[usize]| {
        let pid: Vec<String> = p.iter().map(|&i| ids[i].clone()).collect();
        by_cell.insert(cell_id(poll, &pid), p.to_vec());
    });
    if let Some(k) = cells.keys().find(|k| !by_cell.contains_key(*k)) {
        return Finish::UnknownCell(k.clone());
    }
    let total: u64 = cells.iter().filter(|(k, _)| !by_cell[*k].is_empty()).map(|(_, n)| n).sum();
    if total < floor.min(options.len().max(1) as u64) {
        return Finish::BelowFloor;
    }
    let mut standing: Vec<usize> = (0..options.len()).collect();
    let mut rounds = 0;
    loop {
        let mut first = vec![0u64; options.len()];
        for (k, n) in cells {
            if let Some(&top) = by_cell[k].iter().find(|i| standing.contains(i)) {
                first[top] += n;
            }
        }
        let live: u64 = standing.iter().map(|&i| first[i]).sum();
        if live == 0 {
            return Finish::NoWinner(rounds);
        }
        let (mut lead, mut weak) = (standing[0], standing[0]);
        for &i in &standing {
            if first[i] > first[lead] || (first[i] == first[lead] && i < lead) {
                lead = i;
            }
            if first[i] < first[weak] || (first[i] == first[weak] && i > weak) {
                weak = i;
            }
        }
        if 2 * first[lead] > live || standing.len() == 1 {
            return Finish::Winner(options[lead].clone(), rounds);
        }
        standing.retain(|&i| i != weak);
        rounds += 1;
    }
}

/// The fold's election: (person, ranking) ballots; the declared order is `options` if given, else the offered options in
/// option-id order (a fixed rule: the twin's rows carry no worthiness order).
#[must_use]
pub fn elect(poll: &str, ballots: &[(String, Vec<String>)], options: Option<&[String]>, ranks: usize, floor: u64) -> Option<String> {
    if ballots.is_empty() {
        return None;
    }
    let opts: Vec<String> = match options {
        Some(o) => o.to_vec(),
        None => {
            let mut v: Vec<String> = ballots.iter().flat_map(|b| b.1.iter().cloned()).collect::<std::collections::BTreeSet<_>>().into_iter().collect();
            v.sort_by_key(|o| option_id(poll, o));
            v
        }
    };
    let (s, _) = summary(poll, &opts, ballots, ranks);
    match finish(poll, &opts, &s, ranks, floor) {
        Finish::Winner(w, _) => Some(w),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn o(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_string()).collect()
    }

    /// docs/kernel/ranked_choice_vectors.json: every profile of up to four ballots over three options, the winner and
    /// rounds of the constitution's model (tools/kernel/ranked_vectors.py).
    #[test]
    fn the_model_s_winner_and_rounds_on_every_profile() {
        let v = crate::json::parse(include_str!("../../../docs/kernel/ranked_choice_vectors.json")).unwrap();
        let opts: Vec<String> = v.get("options_declared").unwrap().as_array().unwrap().iter().map(|x| x.as_str().unwrap().to_string()).collect();
        let floor = v.get("floor").unwrap().as_u64().unwrap();
        let mut n = 0;
        for p in v.get("profiles").unwrap().as_array().unwrap() {
            let a = p.as_array().unwrap();
            let rounds = a[a.len() - 1].as_u64().unwrap() as usize;
            let winner = a[a.len() - 2].as_str().unwrap();
            let rows: Vec<(String, Vec<String>)> = a[..a.len() - 2].iter().enumerate().map(|(i, b)| (format!("p{i}"), b.as_str().unwrap().chars().map(|c| c.to_string()).collect())).collect();
            let (s, _) = summary("poll:x", &opts, &rows, 3);
            let got = finish("poll:x", &opts, &s, 3, floor);
            match got {
                Finish::Winner(w, r) => assert_eq!((w.as_str(), r), (winner, rounds), "{rows:?}"),
                _ => assert_eq!(winner, "", "{rows:?}: {got:?}"),
            }
            n += 1;
        }
        assert!(n > 3000);
        for c in v.get("persons").unwrap().as_array().unwrap() {
            let rows: Vec<(String, Vec<String>)> = c.get("rows").unwrap().as_array().unwrap().iter().map(|r| {
                let r = r.as_array().unwrap();
                (r[0].as_str().unwrap().to_string(), r[1].as_array().unwrap().iter().map(|x| x.as_str().unwrap().to_string()).collect())
            }).collect();
            assert_eq!(elect("poll:x", &rows, Some(&opts), 3, floor).as_deref(), c.get("winner").and_then(|w| w.as_str()));
        }
    }

    #[test]
    fn every_order_the_same_winner_and_parts_merge_to_the_whole() {
        let opts = o(&["a", "b", "c"]);
        let mut all: Vec<Vec<String>> = Vec::new();
        for a in &opts {
            all.push(vec![a.clone()]);
            for b in &opts {
                if b != a {
                    all.push(vec![a.clone(), b.clone()]);
                    for c in &opts {
                        if c != a && c != b {
                            all.push(vec![a.clone(), b.clone(), c.clone()]);
                        }
                    }
                }
            }
        }
        let mut seed: u64 = 7;
        let mut rnd = |n: usize| {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((seed >> 33) as usize) % n
        };
        for _ in 0..200 {
            let len = 2 + rnd(9);
            let rows: Vec<(String, Vec<String>)> = (0..len).map(|_| (format!("p{}", rnd(8)), all[rnd(all.len())].clone())).collect();
            let (whole, _) = summary("poll:y", &opts, &rows, 3);
            let w = finish("poll:y", &opts, &whole, 3, 2);
            for _ in 0..6 {
                let mut sh = rows.clone();
                for i in (1..sh.len()).rev() {
                    sh.swap(i, rnd(i + 1));
                }
                assert_eq!(finish("poll:y", &opts, &summary("poll:y", &opts, &sh, 3).0, 3, 2), w);
            }
            let (pa, pb): (Vec<_>, Vec<_>) = rows.iter().cloned().partition(|r| r.0.as_str() < "p4");
            let m = merge(&summary("poll:y", &opts, &pa, 3).0, &summary("poll:y", &opts, &pb, 3).0);
            assert_eq!(m, whole);
            assert_eq!(finish("poll:y", &opts, &m, 3, 2), w);
        }
    }

    #[test]
    fn disagreeing_ballots_count_for_none_and_ties_go_against_the_later_declared_option() {
        let opts = o(&["a", "b", "c"]);
        let r = |p: &str, v: &[&str]| (p.to_string(), o(v));
        // where the last-held ballot decided (the earlier irv): one answer in both orders
        let flip = vec![r("p1", &["a"]), r("p1", &["b"]), r("p2", &["b"]), r("p3", &["a"])];
        let mut rev = flip.clone();
        rev.reverse();
        assert_eq!(elect("poll:o", &flip, Some(&opts), 3, 2), Some("a".to_string()));
        assert_eq!(elect("poll:o", &rev, Some(&opts), 3, 2), Some("a".to_string()));
        // all three hold two first choices of six: the later declared option goes each round
        let tie = vec![r("q1", &["a", "b"]), r("q2", &["c", "a"]), r("q3", &["b"]), r("q4", &["b"]), r("q5", &["a", "c"]), r("q6", &["c", "b"])];
        assert_eq!(elect("poll:t", &tie, Some(&o(&["a", "b", "c"])), 3, 2), Some("a".to_string()));
        assert_eq!(elect("poll:t", &tie, Some(&o(&["c", "b", "a"])), 3, 2), Some("c".to_string()));
        let (s, refused) = summary("poll:z", &opts, &[r("x", &["a", "a"]), r("y", &["d"])], 3);
        assert!(s.is_empty());
        assert_eq!(refused, vec!["x".to_string(), "y".to_string()]);
        assert_ne!(cell_id("poll:1", &[option_id("poll:1", "a")]), cell_id("poll:2", &[option_id("poll:2", "a")]));
        assert_eq!(finish("poll:z", &opts, &[("00".to_string(), 3)].into_iter().collect(), 3, 2), Finish::UnknownCell("00".to_string()));
    }

    #[test]
    fn the_same_ids_as_the_page_kernel() {
        // docs/kernel/kernel.js optionId / cellId: SHA-256 over 4-byte big-endian length-prefixed UTF-8 fields
        assert_eq!(option_id("poll:1", "a"), crate::tables::sha256_digest(b"\0\0\0\x14ciris/poll-option/v1\0\0\0\x06poll:1\0\0\0\x01a"));
        // pinned in both kernels (docs/kernel/kernel.test.js): the same three ids
        let (a, b) = (option_id("poll:1", "a"), option_id("poll:1", "b"));
        assert_eq!(a, "8d089364168555d378185e44e1fce08bb398ce57683b8751204e49cc24cb0d45");
        assert_eq!(cell_id("poll:1", &[a, b]), "b2cbf297e33bb1b79b409476fe9e470077c83e770ff253c9a159a8832293557f");
        assert_eq!(cell_id("poll:1", &[]), "547f7d3f4590e4cc20594c09c08dc853fa69186689a15c8909025d602289eaa6");
    }
}
