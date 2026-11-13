//! Vendored constitutional tables, policy mappings, and CC RC17 namespace matcher.

use crate::json::{parse, JsonValue};
use crate::types::{Fraction, MatchResult, Policy, Rule, Transition};
use std::collections::{HashMap, HashSet};

pub const VENDORED_VERSION: &str = include_str!("../../../docs/kernel/tables/VERSION");
pub const VENDORED_TRANSITIONS: &str = include_str!("../../../docs/kernel/tables/transitions.json");
pub const VENDORED_GOVERNANCE_POLICY: &str = include_str!("../../../docs/kernel/tables/governance_policy.json");
pub const VENDORED_GOVERNANCE_POLICY_VECTORS: &str = include_str!("../../../docs/kernel/tables/governance_policy_vectors.json");
pub const VENDORED_MESH_STORY_TRACE: &str = include_str!("../../../docs/kernel/tables/mesh_story_trace.json");
pub const VENDORED_NAMESPACE_REGISTRY: &str = include_str!("../../../docs/kernel/tables/namespace_registry.json");
pub const VENDORED_NAMESPACE_MATCH_VECTORS: &str = include_str!("../../../docs/kernel/tables/namespace_match_vectors.json");
pub const VENDORED_CSD_NAMESPACE_MAP: &str = include_str!("../../../docs/kernel/tables/csd_namespace_map.json");
/// rc19: expiries, clocks and reactions (durations as named constants). Read into types; none is evaluated yet.
pub const VENDORED_REACTIONS: &str = include_str!("../../../docs/kernel/tables/reactions.json");
/// rc19: the family model transitions.json is derived from; the kernel reads its fold cells (the group:roster fold).
pub const VENDORED_FAMILY_MODEL: &str = include_str!("../../../docs/kernel/tables/family_model.json");

/// One reaction of reactions.json: an expiry rule, a named clock or a reaction of one family, kept as published.
#[derive(Debug, Clone)]
pub struct Reaction {
    pub family: String,
    pub kind: String,
    pub cc_ref: Vec<String>,
    pub value: JsonValue,
}

fn strs(v: Option<&JsonValue>) -> Vec<String> {
    v.and_then(|x| x.as_array()).map(|a| a.iter().filter_map(|e| e.as_str().map(str::to_string)).collect()).unwrap_or_default()
}

/// The rc19 cells of one transition from its JSON object (types.rs `TransitionCells`).
fn transition_cells(t: &JsonValue) -> crate::types::TransitionCells {
    let mut c = crate::types::TransitionCells::default();
    const KNOWN: [&str; 10] = ["verb", "from", "to", "roles", "policy", "widens", "undo", "confirm", "guard", "change"];
    if let Some(o) = t.as_object() {
        for (k, _) in o {
            if !KNOWN.contains(&k.as_str()) {
                c.unknown.push(k.clone());
            }
        }
    }
    let known_sub = |cell: &str, keys: &[&str], c: &mut crate::types::TransitionCells| {
        if let Some(o) = t.get(cell).and_then(|v| v.as_object()) {
            for (k, _) in o {
                if !keys.contains(&k.as_str()) {
                    c.unknown.push(format!("{cell}.{k}"));
                }
            }
        }
    };
    known_sub("roles", &["value", "source", "cc_ref", "stages", "conditions", "cites"], &mut c);
    known_sub("policy", &["value", "source", "cc_ref", "count", "cases", "stages", "cites"], &mut c);
    known_sub("widens", &["value", "source", "cc_ref", "when", "cites"], &mut c);
    known_sub("confirm", &["value", "source", "cc_ref", "when", "cites"], &mut c);
    known_sub("undo", &["value", "source", "cc_ref", "via", "policy", "cites"], &mut c);
    known_sub("guard", &["value", "source", "cc_ref", "cites"], &mut c);
    known_sub("change", &["value", "source", "cc_ref", "cites"], &mut c);
    if let Some(st) = t.get("roles").and_then(|v| v.get("stages")).and_then(|v| v.as_object()) {
        c.role_stages = st.iter().map(|(k, v)| (k.clone(), strs(Some(v)))).collect();
    }
    c.role_conditions = strs(t.get("roles").and_then(|v| v.get("conditions")));
    c.widens_when = t.get("widens").and_then(|v| v.get("when")).and_then(|v| v.as_str()).map(str::to_string);
    c.confirm = t.get("confirm").and_then(|v| Some((v.get("value")?.as_bool()?, v.get("when").and_then(|w| w.as_str()).map(str::to_string))));
    c.undo = t.get("undo").and_then(|v| {
        Some((v.get("value")?.as_bool()?, v.get("via").and_then(|w| w.as_str()).map(str::to_string), v.get("policy").and_then(|w| w.as_str()).map(str::to_string)))
    });
    if let Some(g) = t.get("guard").and_then(|v| v.get("value")).and_then(|v| v.as_array()) {
        for e in g {
            match (e.get("guard").and_then(|v| v.as_str()), e.get("reach").and_then(|v| v.as_str())) {
                (Some(text), Some(reach)) => c.guards.push((text.to_string(), reach.to_string())),
                _ => c.unknown.push(format!("guard item {}", e.to_json_string())),
            }
        }
    }
    if let Some(g) = t.get("change").and_then(|v| v.get("value")).and_then(|v| v.as_array()) {
        for e in g {
            match e.get("item").and_then(|v| v.as_str()) {
                Some(text) => c.changes.push(text.to_string()),
                None => c.unknown.push(format!("change item {}", e.to_json_string())),
            }
        }
    }
    let pol = t.get("policy");
    c.policy_count = pol.and_then(|v| v.get("count")).and_then(|v| v.as_str()).map(str::to_string);
    if let Some(cs) = pol.and_then(|v| v.get("cases")).and_then(|v| v.as_array()) {
        for e in cs {
            c.policy_cases.push((
                e.get("when").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                e.get("policy").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                strs(e.get("rows")),
            ));
        }
    }
    if let Some(st) = pol.and_then(|v| v.get("stages")).and_then(|v| v.as_object()) {
        for (k, e) in st {
            c.policy_stages.push((k.clone(), e.get("policy").and_then(|v| v.as_str()).unwrap_or("").to_string(), strs(e.get("rows"))));
        }
    }
    c
}

// ---------------------------------------------------------------- CC RC17 dimension matcher

pub fn is_hex(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

pub fn is_hex64(s: &str) -> bool {
    s.len() == 64 && is_hex(s)
}

pub fn is_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| matches!(b, b'0'..=b'9'))
}

pub fn is_currency(s: &str) -> bool {
    s.len() == 3 && s.bytes().all(|b| matches!(b, b'A'..=b'Z'))
}

pub fn is_vocab(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let bytes = s.as_bytes();
    if !matches!(bytes[0], b'a'..=b'z' | b'0'..=b'9') {
        return false;
    }
    bytes[1..].iter().all(|&b| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'_' | b'.' | b'-'))
}

pub fn is_literal(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let bytes = s.as_bytes();
    if !matches!(bytes[0], b'a'..=b'z' | b'0'..=b'9') {
        return false;
    }
    bytes[1..].iter().all(|&b| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'_'))
}

pub fn is_version(s: &str) -> bool {
    if !s.starts_with('v') {
        return false;
    }
    let rest = &s[1..];
    if rest.is_empty() {
        return false;
    }
    let parts: Vec<&str> = rest.split('.').collect();
    parts.iter().all(|p| !p.is_empty() && p.bytes().all(|b| matches!(b, b'0'..=b'9')))
}

pub fn is_version_like(s: &str) -> bool {
    if !s.starts_with('v') && !s.starts_with('V') {
        return false;
    }
    let rest = &s[1..];
    if rest.is_empty() {
        return false;
    }
    let parts: Vec<&str> = rest.split('.').collect();
    parts.iter().all(|p| !p.is_empty() && p.bytes().all(|b| matches!(b, b'0'..=b'9')))
}

pub fn is_version_attempt(s: &str) -> bool {
    if !s.starts_with('v') && !s.starts_with('V') {
        return false;
    }
    let bytes = s.as_bytes();
    if bytes.len() < 2 || !matches!(bytes[1], b'0'..=b'9') {
        return false;
    }
    bytes[2..].iter().all(|&b| matches!(b, b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z' | b'_' | b'.' | b'-'))
}

pub fn is_bcp47_lang_code(s: &str) -> bool {
    if s == "i-default" || s == "i-enochian" || s == "i-mingo" || s == "cel-gaulish" || s == "zh-min" {
        return true;
    }
    if s.starts_with("x-") {
        let parts: Vec<&str> = s[2..].split('-').collect();
        return !parts.is_empty()
            && parts
                .iter()
                .all(|p| !p.is_empty() && p.len() <= 8 && p.bytes().all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9')));
    }
    let parts: Vec<&str> = s.split('-').collect();
    if parts.is_empty() {
        return false;
    }
    let mut idx = 0;
    let lang = parts[idx];
    if !(lang.len() == 2 || lang.len() == 3) || !lang.bytes().all(|b| matches!(b, b'a'..=b'z')) {
        return false;
    }
    idx += 1;
    // optional script: [A-Z][a-z]{3}
    if idx < parts.len() {
        let p = parts[idx];
        if p.len() == 4
            && matches!(p.as_bytes()[0], b'A'..=b'Z')
            && p.as_bytes()[1..].iter().all(|b| matches!(b, b'a'..=b'z'))
        {
            idx += 1;
        }
    }
    // optional region: [A-Z]{2} | [0-9]{3}
    if idx < parts.len() {
        let p = parts[idx];
        if (p.len() == 2 && p.bytes().all(|b| matches!(b, b'A'..=b'Z')))
            || (p.len() == 3 && p.bytes().all(|b| matches!(b, b'0'..=b'9')))
        {
            idx += 1;
        }
    }
    // optional variants: (?:[a-z0-9]{5,8}|[0-9][a-z0-9]{3})
    while idx < parts.len() {
        let p = parts[idx];
        let is_var = (p.len() >= 5
            && p.len() <= 8
            && p.bytes().all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9')))
            || (p.len() == 4
                && matches!(p.as_bytes()[0], b'0'..=b'9')
                && p.bytes().all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9')));
        if is_var {
            idx += 1;
        } else {
            break;
        }
    }
    // optional extensions: [a-wy-z0-9](-[a-z0-9]{2,8})+
    while idx < parts.len() {
        let p = parts[idx];
        if p.len() == 1 && matches!(p.as_bytes()[0], b'a'..=b'w' | b'y'..=b'z' | b'0'..=b'9') {
            idx += 1;
            let start_ext = idx;
            while idx < parts.len() {
                let sub = parts[idx];
                if sub.len() >= 2
                    && sub.len() <= 8
                    && sub.bytes().all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9'))
                {
                    idx += 1;
                } else {
                    break;
                }
            }
            if idx == start_ext {
                return false;
            }
        } else {
            break;
        }
    }
    // optional privateuse: -x(-[a-z0-9]{1,8})+
    if idx < parts.len() && parts[idx] == "x" {
        idx += 1;
        let start_priv = idx;
        while idx < parts.len() {
            let sub = parts[idx];
            if !sub.is_empty()
                && sub.len() <= 8
                && sub.bytes().all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9'))
            {
                idx += 1;
            } else {
                break;
            }
        }
        if idx == start_priv {
            return false;
        }
    }
    idx == parts.len()
}

fn match_pattern(pattern: &str, got: &str) -> bool {
    if pattern == "^[0-9]+$" {
        is_digits(got)
    } else if pattern == "^[0-9a-f]{64}$" {
        is_hex64(got)
    } else if pattern == "^[0-9a-f]+$" {
        is_hex(got)
    } else if pattern == "^[A-Z]{3}$" {
        is_currency(got)
    } else if pattern.starts_with("^v[0-9]+") {
        is_version(got)
    } else if pattern.starts_with("^(?:[a-z]{2,3}") || pattern.contains("cel-gaulish") {
        is_bcp47_lang_code(got)
    } else {
        true
    }
}

#[derive(Debug, Clone)]
pub struct RegistrySegment {
    pub segment: String,
    pub class: String,
    pub pattern: Option<String>,
    pub values: Vec<String>,
    pub open: bool,
    pub multi: bool,
}

#[derive(Debug, Clone)]
pub struct RegistryFamily {
    pub prefix: String,
    pub segments: Vec<RegistrySegment>,
    pub leaves: Vec<String>,
    pub leaves_closed: bool,
    pub index: usize,
}

#[derive(Debug, Clone)]
pub struct RefusalTokens {
    pub case_malformed: &'static str,
    pub family_unregistered: &'static str,
    pub missing_version_segment: &'static str,
    pub private_use_not_federatable: &'static str,
    pub vocab_value_unregistered: &'static str,
}

impl Default for RefusalTokens {
    fn default() -> Self {
        Self {
            case_malformed: "namespace_dimension_case_malformed",
            family_unregistered: "namespace_family_unregistered",
            missing_version_segment: "missing_version_segment",
            private_use_not_federatable: "namespace_private_use_not_federatable",
            vocab_value_unregistered: "namespace_vocab_value_unregistered",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Rules {
    pub private: String,
    pub version_required: bool,
    pub exempt: HashSet<String>,
    pub tokens: RefusalTokens,
    pub external: HashMap<String, String>,
    pub reserved_stems: Vec<String>,
    pub closed_stems: Vec<String>,
    pub families: Vec<RegistryFamily>,
    pub family_map: HashMap<String, RegistryFamily>,
}

impl Default for Rules {
    fn default() -> Self {
        Self {
            private: "x_private:".to_string(),
            version_required: true,
            exempt: HashSet::new(),
            tokens: RefusalTokens::default(),
            external: HashMap::new(),
            reserved_stems: Vec::new(),
            closed_stems: Vec::new(),
            families: Vec::new(),
            family_map: HashMap::new(),
        }
    }
}

impl Rules {
    pub fn from_json(manifest: &JsonValue) -> Self {
        let meta = manifest.get("_meta");
        let cr = meta.and_then(|m| m.get("case_rule"));
        let private = meta
            .and_then(|m| m.get("private_use_prefix"))
            .and_then(|v| v.as_str())
            .unwrap_or("x_private:")
            .to_string();

        let vs = cr.and_then(|c| c.get("version_segment"));
        let version_required = vs
            .and_then(|v| v.get("required"))
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let mut exempt = HashSet::new();
        if let Some(arr) = vs.and_then(|v| v.get("exempt")).and_then(|v| v.as_array()) {
            for item in arr {
                if let Some(s) = item.as_str() {
                    exempt.insert(s.to_string());
                }
            }
        }

        let tokens = RefusalTokens::default();

        let mut external = HashMap::new();
        if let Some(ext_map) = cr.and_then(|c| c.get("external_standards")).and_then(|v| v.as_object()) {
            for (name, spec) in ext_map {
                if let Some(pat) = spec.get("pattern").and_then(|v| v.as_str()) {
                    external.insert(name.clone(), pat.to_string());
                }
            }
        }

        let mut reserved_stems = Vec::new();
        if let Some(stems_arr) = cr.and_then(|c| c.get("reserved_stems")).and_then(|v| v.as_array()) {
            for s in stems_arr {
                if let Some(stem) = s.get("stem").and_then(|v| v.as_str()) {
                    reserved_stems.push(stem.to_string());
                }
            }
        }

        let mut families = Vec::new();
        let mut family_map = HashMap::new();
        if let Some(f_arr) = manifest.get("families").and_then(|v| v.as_array()) {
            for (idx, f) in f_arr.iter().enumerate() {
                let prefix = f.get("prefix").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let leaves_closed = f.get("leaves_closed").and_then(|v| v.as_bool()).unwrap_or(false);

                let mut leaves = Vec::new();
                if let Some(l_arr) = f.get("leaves").and_then(|v| v.as_array()) {
                    for l in l_arr {
                        if let Some(s) = l.as_str() {
                            leaves.push(s.to_string());
                        }
                    }
                }

                let mut segments = Vec::new();
                if let Some(s_arr) = f.get("segments").and_then(|v| v.as_array()) {
                    for seg in s_arr {
                        let segment = seg.get("segment").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let class = seg.get("class").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let pattern = seg.get("pattern").and_then(|v| v.as_str()).map(|s| s.to_string());
                        let open = seg.get("open").and_then(|v| v.as_bool()).unwrap_or(false);
                        let multi = seg.get("multi").and_then(|v| v.as_bool()).unwrap_or(false);

                        let mut values = Vec::new();
                        if let Some(v_arr) = seg.get("values").and_then(|v| v.as_array()) {
                            for val in v_arr {
                                if let Some(s) = val.as_str() {
                                    values.push(s.to_string());
                                }
                            }
                        }

                        segments.push(RegistrySegment {
                            segment,
                            class,
                            pattern,
                            values,
                            open,
                            multi,
                        });
                    }
                }

                let reg_fam = RegistryFamily {
                    prefix: prefix.clone(),
                    segments,
                    leaves,
                    leaves_closed,
                    index: idx,
                };
                families.push(reg_fam.clone());
                family_map.insert(prefix, reg_fam);
            }
        }

        let mut cs = HashSet::new();
        for f in &families {
            if f.leaves_closed {
                if let Some(last) = f.segments.last() {
                    if last.class == "wildcard" {
                        if f.prefix.ends_with('*') {
                            cs.insert(f.prefix[..f.prefix.len() - 1].to_string());
                        } else {
                            cs.insert(f.prefix.clone());
                        }
                    } else {
                        let first = f.prefix.split(':').next().unwrap_or("");
                        cs.insert(format!("{}:", first));
                    }
                }
            }
        }
        let mut closed_stems: Vec<String> = cs.into_iter().collect();
        closed_stems.sort();

        Self {
            private,
            version_required,
            exempt,
            tokens,
            external,
            reserved_stems,
            closed_stems,
            families,
            family_map,
        }
    }
}

fn score(fam: &RegistryFamily) -> (usize, usize, usize) {
    let literals = fam.segments.iter().filter(|s| s.class == "literal").count();
    let wildcard = fam.segments.last().map(|s| s.class == "wildcard").unwrap_or(false);
    (literals, if wildcard { 0 } else { 1 }, fam.segments.len())
}

fn check_seg(seg: &RegistrySegment, got: &str, rules: &Rules) -> Option<&'static str> {
    if got.is_empty() {
        return Some(rules.tokens.case_malformed);
    }
    if seg.segment == "{version}" {
        return if is_version(got) {
            None
        } else {
            Some(rules.tokens.case_malformed)
        };
    }
    let cls = seg.class.as_str();
    let name = seg.segment.trim_start_matches('{').trim_end_matches('}');
    if cls == "vocab" {
        if !is_vocab(got) {
            return Some(rules.tokens.case_malformed);
        }
        if !seg.values.is_empty() && !seg.open && !seg.values.iter().any(|v| v == got) {
            return Some(rules.tokens.vocab_value_unregistered);
        }
        return None;
    }
    if cls == "hex" {
        let ok = if let Some(ref p) = seg.pattern {
            match_pattern(p, got)
        } else {
            is_hex(got)
        };
        return if ok {
            None
        } else {
            Some(rules.tokens.case_malformed)
        };
    }
    if cls == "external" {
        let ok = if let Some(ref p) = seg.pattern {
            match_pattern(p, got)
        } else if let Some(p) = rules.external.get(name) {
            match_pattern(p, got)
        } else {
            true
        };
        return if ok {
            None
        } else {
            Some(rules.tokens.case_malformed)
        };
    }
    if let Some(ref p) = seg.pattern {
        if !match_pattern(p, got) {
            return Some(rules.tokens.case_malformed);
        }
    }
    None
}

fn match_segments(
    fam: &RegistryFamily,
    parts: &[&str],
    rules: &Rules,
) -> (bool, HashMap<String, String>, Option<&'static str>) {
    let segs = &fam.segments;
    if segs.is_empty() {
        return (false, HashMap::new(), None);
    }
    let wildcard = segs.last().map(|s| s.class == "wildcard").unwrap_or(false);
    let fixed = if wildcard {
        &segs[..segs.len() - 1]
    } else {
        &segs[..]
    };
    let multi_idx = fixed.iter().position(|s| s.multi);
    let extra: isize = if wildcard {
        if parts.len() < fixed.len() + 1 {
            return (false, HashMap::new(), None);
        }
        0
    } else if let Some(_) = multi_idx {
        let ext = parts.len() as isize - fixed.len() as isize;
        if ext < 0 {
            return (false, HashMap::new(), None);
        }
        ext
    } else if parts.len() != fixed.len() {
        return (false, HashMap::new(), None);
    } else {
        0
    };

    let mut binds = HashMap::new();
    let mut refusal: Option<&'static str> = None;
    let mut pi = 0;

    for (i, seg) in fixed.iter().enumerate() {
        if Some(i) == multi_idx {
            let count = 1 + extra as usize;
            let span = &parts[pi..pi + count];
            pi += count;
            for got in span {
                if refusal.is_none() {
                    refusal = check_seg(seg, got, rules);
                }
            }
            let key = seg.segment.trim_start_matches('{').trim_end_matches('}');
            binds.insert(key.to_string(), span.join(":"));
            continue;
        }
        let got = parts[pi];
        pi += 1;
        if seg.class == "literal" {
            if seg.segment != got {
                return (false, HashMap::new(), None);
            }
            continue;
        }
        if refusal.is_none() {
            refusal = check_seg(seg, got, rules);
        }
        let key = seg.segment.trim_start_matches('{').trim_end_matches('}');
        binds.insert(key.to_string(), got.to_string());
    }

    let tail = &parts[pi..];
    for (k, got) in tail.iter().enumerate() {
        let last = pi + k == parts.len() - 1;
        if got.is_empty()
            || !is_vocab(got)
            || is_version_like(got)
            || (last && is_version_attempt(got))
        {
            if refusal.is_none() {
                refusal = Some(rules.tokens.case_malformed);
            }
        }
    }
    if !tail.is_empty() {
        binds.insert("*".to_string(), tail.join(":"));
    }
    (true, binds, refusal)
}

fn mutated_tail(seg: &str) -> bool {
    (is_version_like(seg) && !is_version(seg)) || is_version_attempt(seg)
}

#[derive(Debug, Clone)]
struct Candidate {
    sc: (usize, usize, usize),
    prefix: String,
    binds: HashMap<String, String>,
    refusal: Option<&'static str>,
    has_version: bool,
    index: usize,
}

fn resolve(rules: &Rules, parts: &[&str]) -> Option<Candidate> {
    let versioned = parts.last().map(|p| is_version(p)).unwrap_or(false);
    let mut candidates = Vec::new();

    for fam in &rules.families {
        let ends_version = fam
            .segments
            .last()
            .map(|s| s.segment == "{version}")
            .unwrap_or(false);

        if ends_version && !versioned {
            let mut headless = fam.clone();
            headless.segments.pop();
            let (ok, binds, ref1) = match_segments(&headless, parts, rules);
            if ok {
                candidates.push(Candidate {
                    sc: score(fam),
                    prefix: fam.prefix.clone(),
                    binds,
                    refusal: ref1,
                    has_version: false,
                    index: fam.index,
                });
            }
            let (ok2, binds2, ref2) = match_segments(fam, parts, rules);
            if ok2 {
                candidates.push(Candidate {
                    sc: score(fam),
                    prefix: fam.prefix.clone(),
                    binds: binds2,
                    refusal: ref2,
                    has_version: true,
                    index: fam.index,
                });
            }
        } else if ends_version || !versioned {
            let (ok, binds, mut ref1) = match_segments(fam, parts, rules);
            if ok {
                if !versioned && parts.last().map(|p| mutated_tail(p)).unwrap_or(false) {
                    if ref1.is_none() {
                        ref1 = Some(rules.tokens.case_malformed);
                    }
                }
                candidates.push(Candidate {
                    sc: score(fam),
                    prefix: fam.prefix.clone(),
                    binds,
                    refusal: ref1,
                    has_version: ends_version,
                    index: fam.index,
                });
            }
        } else if parts.len() > 1 {
            let (ok, binds, ref1) = match_segments(fam, &parts[..parts.len() - 1], rules);
            if ok {
                candidates.push(Candidate {
                    sc: score(fam),
                    prefix: fam.prefix.clone(),
                    binds,
                    refusal: ref1,
                    has_version: true,
                    index: fam.index,
                });
            }
        }
    }

    if candidates.is_empty() && versioned {
        for fam in &rules.families {
            let ends_version = fam
                .segments
                .last()
                .map(|s| s.segment == "{version}")
                .unwrap_or(false);
            if ends_version {
                continue;
            }
            let (ok, binds, ref1) = match_segments(fam, parts, rules);
            if ok {
                candidates.push(Candidate {
                    sc: score(fam),
                    prefix: fam.prefix.clone(),
                    binds,
                    refusal: ref1,
                    has_version: false,
                    index: fam.index,
                });
            }
        }
    }

    if candidates.is_empty() {
        return None;
    }

    candidates.sort_by(|a, b| {
        b.sc.0
            .cmp(&a.sc.0)
            .then_with(|| b.sc.1.cmp(&a.sc.1))
            .then_with(|| b.sc.2.cmp(&a.sc.2))
            .then_with(|| a.index.cmp(&b.index))
    });

    candidates.into_iter().next()
}

fn detect_malformed(rules: &Rules, parts: &[&str], clean_only: bool) -> Option<String> {
    let mut variants: Vec<Vec<String>> = Vec::new();
    let low: Vec<String> = parts.iter().map(|p| p.to_lowercase()).collect();
    if low.iter().zip(parts.iter()).any(|(l, p)| l.as_str() != *p) {
        variants.push(low.clone());
    }
    let n = parts.len();
    if n > 1 && (is_version_like(parts[n - 1]) || is_version_attempt(parts[n - 1])) {
        variants.push(parts[..n - 1].iter().map(|s| s.to_string()).collect());
        variants.push(low[..n - 1].to_vec());
        if n > 2 && is_version_like(parts[n - 2]) {
            variants.push(parts[..n - 2].iter().map(|s| s.to_string()).collect());
            variants.push(low[..n - 2].to_vec());
        }
    }
    for v in variants {
        let v_slices: Vec<&str> = v.iter().map(|s| s.as_str()).collect();
        if let Some(hit) = resolve(rules, &v_slices) {
            if hit.refusal.is_none() || !clean_only {
                return Some(hit.prefix);
            }
        }
    }
    None
}

pub fn match_family_with_rules(
    wire_namespace: &str,
    tables: &Tables,
) -> MatchResult {
    if wire_namespace.is_empty() {
        return MatchResult {
            family: None,
            binds: HashMap::new(),
            refusal: Some("namespace_dimension_case_malformed".to_string()),
            ok: false,
        };
    }

    // Fast path 1: explicit aliases from csd_namespace_map.json
    let target = tables
        .aliases
        .get(wire_namespace)
        .map(|s| s.as_str())
        .unwrap_or(wire_namespace);

    // Fast path 2: direct exact lookup in transitions table for template families
    if (target.contains('{') || target.ends_with('*')) && tables.families.contains_key(target) {
        return MatchResult {
            family: Some(target.to_string()),
            binds: HashMap::new(),
            refusal: None,
            ok: true,
        };
    }

    let rules = &tables.rules;

    if target.trim() != target {
        return MatchResult {
            family: None,
            binds: HashMap::new(),
            refusal: Some(rules.tokens.case_malformed.to_string()),
            ok: false,
        };
    }

    if target.starts_with(&rules.private) {
        return MatchResult {
            family: None,
            binds: HashMap::new(),
            refusal: Some(rules.tokens.private_use_not_federatable.to_string()),
            ok: false,
        };
    }

    let parts: Vec<&str> = target.split(':').collect();
    if parts.iter().any(|p| p.is_empty()) {
        return MatchResult {
            family: None,
            binds: HashMap::new(),
            refusal: Some(rules.tokens.case_malformed.to_string()),
            ok: false,
        };
    }

    let hit = resolve(rules, &parts);
    if hit.is_none() {
        if let Some(mutated) = detect_malformed(rules, &parts, false) {
            return MatchResult {
                family: Some(mutated),
                binds: HashMap::new(),
                refusal: Some(rules.tokens.case_malformed.to_string()),
                ok: false,
            };
        }
        let fenced: Vec<&str> = rules
            .reserved_stems
            .iter()
            .chain(rules.closed_stems.iter())
            .map(|s| s.as_str())
            .collect();
        if fenced.iter().any(|stem| target.starts_with(stem)) {
            return MatchResult {
                family: None,
                binds: HashMap::new(),
                refusal: Some(rules.tokens.family_unregistered.to_string()),
                ok: false,
            };
        }
        let low = target.to_lowercase();
        if low != target && fenced.iter().any(|stem| low.starts_with(stem)) {
            return MatchResult {
                family: None,
                binds: HashMap::new(),
                refusal: Some(rules.tokens.case_malformed.to_string()),
                ok: false,
            };
        }
        if rules.version_required && !is_version(parts[parts.len() - 1]) {
            if mutated_tail(parts[parts.len() - 1]) {
                return MatchResult {
                    family: None,
                    binds: HashMap::new(),
                    refusal: Some(rules.tokens.case_malformed.to_string()),
                    ok: false,
                };
            }
            return MatchResult {
                family: None,
                binds: HashMap::new(),
                refusal: Some(rules.tokens.missing_version_segment.to_string()),
                ok: false,
            };
        }
        if parts.len() >= 2 && is_version_like(parts[parts.len() - 2]) {
            return MatchResult {
                family: None,
                binds: HashMap::new(),
                refusal: Some(rules.tokens.case_malformed.to_string()),
                ok: false,
            };
        }
        if parts.len() < 2 {
            return MatchResult {
                family: None,
                binds: HashMap::new(),
                refusal: Some(rules.tokens.case_malformed.to_string()),
                ok: false,
            };
        }
        return MatchResult {
            family: None,
            binds: HashMap::new(),
            refusal: None,
            ok: true,
        };
    }

    let hit = hit.unwrap();
    let prefix = hit.prefix;
    let mut binds = hit.binds;
    let refusal = hit.refusal;
    let has_version = hit.has_version;

    let fam = match rules.family_map.get(&prefix) {
        Some(f) => f,
        None => {
            return MatchResult {
                family: Some(prefix),
                binds,
                refusal: refusal.map(|s| s.to_string()),
                ok: refusal.is_none(),
            };
        }
    };

    if let Some(ref_str) = refusal {
        if let Some(mutated) = detect_malformed(rules, &parts, true) {
            if mutated != prefix {
                return MatchResult {
                    family: Some(mutated),
                    binds: HashMap::new(),
                    refusal: Some(rules.tokens.case_malformed.to_string()),
                    ok: false,
                };
            }
        }
        return MatchResult {
            family: Some(prefix),
            binds,
            refusal: Some(ref_str.to_string()),
            ok: false,
        };
    }

    let versioned = is_version(parts[parts.len() - 1]);
    let last_seg_class = fam.segments.last().map(|s| s.class.as_str());
    if fam.leaves_closed && (last_seg_class == Some("wildcard") || last_seg_class == Some("vocab")) {
        let leaf_parts = if versioned && has_version {
            &parts[..parts.len() - 1]
        } else {
            &parts[..]
        };
        let ok_leaf = fam.leaves.iter().any(|l| {
            if let Some(lfam) = rules.family_map.get(l) {
                let (ok, _, _) = match_segments(lfam, leaf_parts, rules);
                ok
            } else {
                false
            }
        });
        if !ok_leaf {
            return MatchResult {
                family: Some(prefix),
                binds,
                refusal: Some(rules.tokens.family_unregistered.to_string()),
                ok: false,
            };
        }
    }

    if rules.version_required && !rules.exempt.contains(&prefix) && !has_version {
        return MatchResult {
            family: Some(prefix),
            binds,
            refusal: Some(rules.tokens.missing_version_segment.to_string()),
            ok: false,
        };
    }

    if has_version {
        binds.insert("version".to_string(), parts[parts.len() - 1].to_string());
    }

    MatchResult {
        family: Some(prefix),
        binds,
        refusal: None,
        ok: true,
    }
}

// ---------------------------------------------------------------- tables

/// Computes SHA-256 over a byte slice without external dependencies.
pub fn sha256_digest(input: &[u8]) -> String {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];

    let k: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
        0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
        0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
        0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
        0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];

    let mut data = input.to_vec();
    let bit_len = (input.len() as u64) * 8;
    data.push(0x80);
    while (data.len() + 8) % 64 != 0 {
        data.push(0x00);
    }
    data.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in data.chunks(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([chunk[i * 4], chunk[i * 4 + 1], chunk[i * 4 + 2], chunk[i * 4 + 3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }

        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut h_val = h[7];

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h_val.wrapping_add(s1).wrapping_add(ch).wrapping_add(k[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h_val = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(h_val);
    }

    use std::fmt::Write;
    let mut out = String::with_capacity(64);
    for val in h {
        write!(out, "{:08x}", val).unwrap();
    }
    out
}

pub fn parse_version_hashes(version_str: &str) -> HashMap<String, String> {
    let mut hashes = HashMap::new();
    let mut in_hashes = false;
    for line in version_str.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("SHA-256 Hashes:") {
            in_hashes = true;
            continue;
        }
        if in_hashes {
            if let Some((file, hash)) = trimmed.split_once(':') {
                let file = file.trim().to_string();
                let hash = hash.trim().to_string();
                if !file.is_empty() && !hash.is_empty() {
                    hashes.insert(file, hash);
                }
            }
        }
    }
    hashes
}

pub fn expected_active(n: u64) -> u64 {
    if n <= 12 {
        n
    } else {
        3.max(12 + (n - 12) / 30)
    }
}

pub fn parse_rule(val: Option<&JsonValue>) -> Rule {
    let val = match val {
        Some(v) => v,
        None => return Rule::None,
    };
    if let Some(s) = val.as_str() {
        let s = s.trim();
        if s.is_empty() || s == "—" || s == "-" {
            return Rule::None;
        }
        if s == "1/N" || s == "1/N fires" {
            return Rule::OneOfN;
        }
        if s == "1/2 confirms" {
            return Rule::StrictMajority;
        }
        if s.starts_with("more than 1/2 of the others") {
            return Rule::StrictMajorityOfOthers;
        }
        if s == "one_voter" {
            return Rule::Voices { count: 1 };
        }
        if s == "2 voices" {
            return Rule::Voices { count: 2 };
        }
        if s.ends_with("voices") || s.ends_with("voice") {
            let parts: Vec<&str> = s.split_whitespace().collect();
            if let Some(n) = parts.first().and_then(|x| x.parse::<u64>().ok()) {
                return Rule::Voices { count: n };
            }
        }
        if s.starts_with("role:") && s.contains(" at ") {
            let parts: Vec<&str> = s.split(" at ").collect();
            if parts.len() == 2 {
                let role_part = parts[0].trim_start_matches("role:");
                let roles: Vec<String> = role_part.split('|').map(|r| r.to_string()).collect();
                if let Ok(count) = parts[1].trim().parse::<u64>() {
                    return Rule::RoleAt { roles, count };
                }
            }
        }
        if s == "instant runoff over ranked choice of three" {
            return Rule::RankedIrv { choices: 3 };
        }
        if s.contains("every member of the root except the named party") {
            return Rule::AllOthers;
        }
        if s.contains("at least 2 of the self's live nodes") {
            return Rule::SelfNodesMin2Or1;
        }
        if s.contains("at least 2 witnesses from the prior seal") {
            return Rule::WitnessesPriorSeal;
        }
        if s.contains("each named party signs") {
            return Rule::AllNamedParties;
        }
        if s.contains("single Wise Authority or Steward signature") {
            return Rule::SingleRoleSignature {
                roles: vec!["wa".to_string(), "steward".to_string()],
            };
        }
        if s.contains('/') {
            let parts: Vec<&str> = s.split('/').collect();
            if parts.len() == 2 {
                if let (Ok(num), Ok(den)) = (parts[0].trim().parse::<u64>(), parts[1].trim().parse::<u64>()) {
                    return Rule::Fraction { num, den };
                }
            }
        }
        return Rule::Prose(s.to_string());
    }

    if let Some(kind) = val.get("kind").and_then(|v| v.as_str()) {
        match kind {
            "none" => return Rule::None,
            "one_of_n" => return Rule::OneOfN,
            "strict_majority" => return Rule::StrictMajority,
            "strict_majority_of_others" => return Rule::StrictMajorityOfOthers,
            "fraction" => {
                let num = val.get("num").and_then(|v| v.as_u64()).unwrap_or(0);
                let den = val.get("den").and_then(|v| v.as_u64()).unwrap_or(1);
                return Rule::Fraction { num, den };
            }
            "role_at" => {
                let mut roles = Vec::new();
                if let Some(arr) = val.get("roles").and_then(|v| v.as_array()) {
                    for r in arr {
                        if let Some(rs) = r.as_str() {
                            roles.push(rs.to_string());
                        }
                    }
                }
                let count = val.get("count").and_then(|v| v.as_u64()).unwrap_or(1);
                return Rule::RoleAt { roles, count };
            }
            "voices" => {
                let count = val.get("count").and_then(|v| v.as_u64()).unwrap_or(1);
                return Rule::Voices { count };
            }
            "ranked_irv" => {
                let choices = val.get("choices").and_then(|v| v.as_u64()).unwrap_or(3);
                return Rule::RankedIrv { choices };
            }
            _ => {}
        }
    }
    if let (Some(num), Some(den)) = (
        val.get("num").and_then(|v| v.as_u64()),
        val.get("den").and_then(|v| v.as_u64()),
    ) {
        return Rule::Fraction { num, den };
    }
    Rule::None
}

#[derive(Debug, Clone, PartialEq)]
pub struct PolarityResult {
    pub admissible: bool,
    pub class: Option<String>,
    pub refusal: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Tables {
    pub families: HashMap<String, Vec<Transition>>,
    pub family_order: Vec<String>,
    pub policies: HashMap<String, Policy>,
    pub defaults: HashMap<String, String>,
    pub version: String,
    pub story_trace: JsonValue,
    pub aliases: HashMap<String, String>,
    pub registry: JsonValue,
    pub match_vectors: JsonValue,
    pub csd_map: JsonValue,
    pub rules: Rules,
    pub min_count: HashMap<String, u64>,
    pub policy_vectors: JsonValue,
    /// rc19 reactions.json, as published (none evaluated: `unread_cells`).
    pub reactions: Vec<Reaction>,
    /// rc19 family_model.json fold clauses of group:roster (the roster fold), as published.
    pub roster_fold: Vec<String>,
    /// rc19 governance_policy.json `protocols.kinds`, as published.
    pub protocols: JsonValue,
}

impl Tables {
    pub fn load_vendored() -> Self {
        Self::load_all(
            VENDORED_VERSION,
            VENDORED_TRANSITIONS,
            VENDORED_GOVERNANCE_POLICY,
            VENDORED_MESH_STORY_TRACE,
            VENDORED_NAMESPACE_REGISTRY,
            VENDORED_NAMESPACE_MATCH_VECTORS,
            VENDORED_CSD_NAMESPACE_MAP,
            VENDORED_GOVERNANCE_POLICY_VECTORS,
        )
        .expect("Failed to load vendored CC §4.5.13 constitutional tables")
    }

    pub fn load_from_strings(
        version_str: &str,
        transitions_str: &str,
        policy_str: &str,
        trace_str: &str,
    ) -> Result<Self, String> {
        Self::load_all(
            version_str,
            transitions_str,
            policy_str,
            trace_str,
            VENDORED_NAMESPACE_REGISTRY,
            VENDORED_NAMESPACE_MATCH_VECTORS,
            VENDORED_CSD_NAMESPACE_MAP,
            VENDORED_GOVERNANCE_POLICY_VECTORS,
        )
    }

    pub fn load_all(
        version_str: &str,
        transitions_str: &str,
        policy_str: &str,
        trace_str: &str,
        registry_str: &str,
        match_vectors_str: &str,
        csd_map_str: &str,
        policy_vectors_str: &str,
    ) -> Result<Self, String> {
        let expected_hashes = parse_version_hashes(version_str);
        let files_to_verify = [
            ("transitions.json", transitions_str),
            ("governance_policy.json", policy_str),
            ("governance_policy_vectors.json", policy_vectors_str),
            ("mesh_story_trace.json", trace_str),
            ("namespace_registry.json", registry_str),
            ("namespace_match_vectors.json", match_vectors_str),
        ];
        for (filename, content) in &files_to_verify {
            if let Some(expected) = expected_hashes.get(*filename) {
                let computed = sha256_digest(content.as_bytes());
                if &computed != expected {
                    return Err(format!(
                        "SHA-256 digest mismatch for {}: expected {}, computed {}",
                        filename, expected, computed
                    ));
                }
            }
        }

        let transitions_json = parse(transitions_str).map_err(|e| format!("transitions.json parse error: {}", e))?;
        let policy_json = parse(policy_str).map_err(|e| format!("governance_policy.json parse error: {}", e))?;
        let trace_json = parse(trace_str).map_err(|e| format!("mesh_story_trace.json parse error: {}", e))?;
        let registry_json = parse(registry_str).map_err(|e| format!("namespace_registry.json parse error: {}", e))?;
        let match_vectors_json = parse(match_vectors_str).map_err(|e| format!("namespace_match_vectors.json parse error: {}", e))?;
        let csd_map_json = parse(csd_map_str).map_err(|e| format!("csd_namespace_map.json parse error: {}", e))?;
        let policy_vectors_json = parse(policy_vectors_str).map_err(|e| format!("governance_policy_vectors.json parse error: {}", e))?;

        let mut families = HashMap::new();
        let mut family_order = Vec::new();

        if let Some(rows) = transitions_json.get("rows").and_then(|v| v.as_array()) {
            for r in rows {
                let fam = r.get("family").and_then(|v| v.as_str()).unwrap_or("").to_string();
                if fam.is_empty() {
                    continue;
                }
                let mut ts = Vec::new();
                if let Some(t_arr) = r.get("transitions").and_then(|v| v.as_array()) {
                    for t in t_arr {
                        let verb = t.get("verb").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let from = t.get("from").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let to = t.get("to").and_then(|v| v.as_str()).unwrap_or("").to_string();

                        let mut roles = Vec::new();
                        if let Some(role_arr) = t.get("roles").and_then(|v| v.get("value")).and_then(|v| v.as_array()) {
                            for role in role_arr {
                                if let Some(s) = role.as_str() {
                                    roles.push(s.to_string());
                                }
                            }
                        }

                        let policy_val = t.get("policy").and_then(|v| v.get("value"));
                        let policy = policy_val
                            .and_then(|v| v.get("policy"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("selfAct")
                            .to_string();

                        let mut policy_rows = Vec::new();
                        if let Some(p_rows) = policy_val.and_then(|v| v.get("rows")).and_then(|v| v.as_array()) {
                            for pr in p_rows {
                                if let Some(s) = pr.as_str() {
                                    policy_rows.push(s.to_string());
                                }
                            }
                        }

                        let widens = t
                            .get("widens")
                            .and_then(|v| v.get("value"))
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);

                        ts.push(Transition {
                            verb,
                            from,
                            to,
                            roles,
                            policy,
                            policy_rows,
                            widens,
                            cells: transition_cells(t),
                        });
                    }
                }
                family_order.push(fam.clone());
                families.insert(fam, ts);
            }
        }

        let mut policies = HashMap::new();
        if let Some(fams) = policy_json.get("families").and_then(|v| v.as_array()) {
            for p in fams {
                let fam = p.get("family").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let kind = p.get("kind").and_then(|v| v.as_str()).unwrap_or("").to_string();

                let enact_rule = p.get("enact_rule")
                    .map(|v| parse_rule(Some(v)))
                    .unwrap_or_else(|| parse_rule(p.get("enact")));
                let cancel_rule = p.get("cancel_rule")
                    .map(|v| parse_rule(Some(v)))
                    .unwrap_or_else(|| parse_rule(p.get("cancel")));
                let floor_rule = p.get("floor_rule")
                    .map(|v| parse_rule(Some(v)))
                    .unwrap_or_else(|| parse_rule(p.get("floor")));

                let enact = match &enact_rule {
                    Rule::Fraction { num, den } => Some(Fraction { num: *num, den: *den }),
                    _ => p.get("enact").and_then(|v| {
                        let num = v.get("num")?.as_u64()?;
                        let den = v.get("den")?.as_u64()?;
                        Some(Fraction { num, den })
                    }),
                };

                let cancel = match &cancel_rule {
                    Rule::Fraction { num, den } => Some(Fraction { num: *num, den: *den }),
                    _ => p.get("cancel").and_then(|v| {
                        let num = v.get("num")?.as_u64()?;
                        let den = v.get("den")?.as_u64()?;
                        Some(Fraction { num, den })
                    }),
                };

                let floor = match &floor_rule {
                    Rule::Fraction { num, den } => Some(Fraction { num: *num, den: *den }),
                    _ => p.get("floor").and_then(|v| {
                        let num = v.get("num")?.as_u64()?;
                        let den = v.get("den")?.as_u64()?;
                        Some(Fraction { num, den })
                    }),
                };

                let irreversible = p.get("irreversible").and_then(|v| v.as_bool()).unwrap_or(false);
                let reverse_path = p.get("reverse_path").and_then(|v| v.as_bool()).unwrap_or(false);
                let cancel_over = p.get("cancel_over").and_then(|v| v.as_str()).map(|s| {
                    if s == "—" || s == "-" { "none".to_string() } else { s.to_string() }
                });
                let base_law = p.get("base_law").and_then(|v| v.as_str()).map(|s| s.to_string())
                    .or_else(|| policy_json.get("base_law").and_then(|b| b.get("rule")).and_then(|r| r.as_str()).map(|s| s.to_string()));
                let base = p.get("base").and_then(|v| v.as_str()).map(|s| s.to_string());
                let at_window = p.get("at_window").and_then(|v| v.as_str()).map(|s| {
                    if s == "—" || s == "-" { "none".to_string() } else { s.to_string() }
                });
                let window_secs = p.get("window_secs").and_then(|v| v.as_u64());
                let window_class = p.get("window_class").and_then(|v| v.as_str()).map(|s| s.to_string());
                let corridor = p.get("corridor").and_then(|v| v.as_str()).map(|s| s.to_string());
                let counted: Vec<(String, String)> = p
                    .get("counted")
                    .and_then(|v| v.as_object())
                    .map(|o| o.iter().map(|(k, v)| (k.clone(), v.get("value").and_then(|x| x.as_str()).unwrap_or("").to_string())).collect())
                    .unwrap_or_default();

                policies.insert(
                    fam.clone(),
                    Policy {
                        family: fam,
                        kind,
                        enact,
                        cancel,
                        floor,
                        enact_rule,
                        cancel_rule,
                        floor_rule,
                        irreversible,
                        reverse_path,
                        cancel_over,
                        base_law,
                        base,
                        at_window,
                        window_secs,
                        window_class,
                        corridor,
                        counted,
                    },
                );
            }
        }

        let mut defaults = HashMap::new();
        defaults.insert("two_path".to_string(), "default_standing".to_string());
        defaults.insert("mirror".to_string(), "moderation_action".to_string());
        defaults.insert("cosigned".to_string(), "custodian_replace".to_string());
        defaults.insert("ranked".to_string(), "moderator_election".to_string());

        let mut aliases = HashMap::new();
        if let Some(alias_obj) = csd_map_json.get("aliases").and_then(|v| v.as_object()) {
            for (k, v) in alias_obj {
                if let Some(target) = v.as_str() {
                    aliases.insert(k.clone(), target.to_string());
                }
            }
        }

        let rules = Rules::from_json(&registry_json);

        let mut min_count = HashMap::new();
        if let Some(mc_obj) = policy_json.get("min_count").and_then(|v| v.as_object()) {
            for (k, v) in mc_obj {
                if let Some(cnt) = v.as_u64() {
                    min_count.insert(k.clone(), cnt);
                }
            }
        }

        // rc19: reactions and the group:roster fold, read and checked against VERSION like the rest
        let expected_hashes2 = parse_version_hashes(version_str);
        for (filename, content) in [("reactions.json", VENDORED_REACTIONS), ("family_model.json", VENDORED_FAMILY_MODEL)] {
            if let Some(expected) = expected_hashes2.get(filename) {
                let computed = sha256_digest(content.as_bytes());
                if &computed != expected {
                    return Err(format!("SHA-256 digest mismatch for {}: expected {}, computed {}", filename, expected, computed));
                }
            }
        }
        let reactions_json = parse(VENDORED_REACTIONS).map_err(|e| format!("reactions.json parse error: {}", e))?;
        let reactions = reactions_json
            .get("reactions")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .map(|r| Reaction {
                        family: r.get("family").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        kind: r.get("kind").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        cc_ref: strs(r.get("cc_ref")),
                        value: r.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        let model_json = parse(VENDORED_FAMILY_MODEL).map_err(|e| format!("family_model.json parse error: {}", e))?;
        let roster_fold = model_json
            .get("rows")
            .and_then(|v| v.as_array())
            .and_then(|rows| rows.iter().find(|r| r.get("family").and_then(|f| f.as_str()) == Some("group:roster")))
            .and_then(|r| r.get("fold"))
            .and_then(|f| f.get("value"))
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|e| e.get("fold").and_then(|x| x.as_str()).map(str::to_string)).collect())
            .unwrap_or_default();
        let protocols = policy_json.get("protocols").cloned().unwrap_or(JsonValue::Null);

        Ok(Tables {
            families,
            family_order,
            policies,
            defaults,
            version: version_str.trim().to_string(),
            story_trace: trace_json,
            aliases,
            registry: registry_json,
            match_vectors: match_vectors_json,
            csd_map: csd_map_json,
            rules,
            min_count,
            policy_vectors: policy_vectors_json,
            reactions,
            roster_fold,
            protocols,
        })
    }

    /// Every table cell this kernel reads but does not evaluate, and every cell it does not know, one line each, sorted:
    /// pinned by a test (`unread_cells_are_listed`) so that a re-vendor which adds one, or a change that starts evaluating
    /// one, shows as a diff and is never silent. Lines starting UNKNOWN are shapes the parser does not know at all.
    #[must_use]
    pub fn unread_cells(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut agg: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
        for fam in &self.family_order {
            for t in &self.families[fam] {
                let c = &t.cells;
                for k in &c.unknown {
                    out.push(format!("UNKNOWN {fam} {}: {k}", t.verb));
                }
                for (g, reach) in &c.guards {
                    out.push(format!("guard {fam} {}: {g} [reach {reach}]", t.verb));
                }
                for ch in &c.changes {
                    out.push(format!("change {fam} {}: {ch}", t.verb));
                }
                if !c.role_conditions.is_empty() {
                    out.push(format!("roles.conditions {fam} {}: {}", t.verb, c.role_conditions.join(",")));
                }
                for (w, p, rows) in &c.policy_cases {
                    out.push(format!("policy.cases {fam} {}: when {w} -> {p} {}", t.verb, rows.join(",")));
                }
                for (st, p, rows) in &c.policy_stages {
                    if p != &t.policy {
                        out.push(format!("policy.stages {fam} {}: {st} -> {p} {}", t.verb, rows.join(",")));
                    }
                }
                if let Some(pc) = &c.policy_count {
                    *agg.entry(format!("policy.count (derivation of the policy cell, informational): {pc}")).or_default() += 1;
                }
                if let Some((_, Some(w))) = &c.confirm {
                    *agg.entry(format!("confirm.when {w} (the kernel has no confirmation step)")).or_default() += 1;
                }
                if let Some(w) = &c.widens_when {
                    *agg.entry(format!("widens.when {w} (the kernel reads the boolean only)")).or_default() += 1;
                }
                if let Some((true, via, pol)) = &c.undo {
                    *agg.entry(format!("undo via {} under {} (offered only as the table's own transition)", via.as_deref().unwrap_or("?"), pol.as_deref().unwrap_or("?"))).or_default() += 1;
                }
            }
        }
        let mut fams: Vec<&String> = self.policies.keys().collect();
        fams.sort();
        for f in fams {
            let p = &self.policies[f];
            for (col, val) in &p.counted {
                *agg.entry(format!("counted {col}={val}")).or_default() += 1;
            }
            for (cell, r) in [("enact", &p.enact_rule), ("cancel", &p.cancel_rule), ("floor", &p.floor_rule)] {
                if let crate::types::Rule::Prose(text) = r {
                    out.push(format!("rule prose {f} {cell}: {text}"));
                }
            }
        }
        for r in &self.reactions {
            *agg.entry(format!("reaction {} (none evaluated: no clock, expiry or reaction runs in this kernel)", r.kind)).or_default() += 1;
        }
        for f in &self.roster_fold {
            out.push(format!("roster fold (the kernel's room roster is its founders and seated members, no head chain): {f}"));
        }
        if let Some(k) = self.protocols.get("kinds").and_then(|v| v.as_array()) {
            for e in k {
                out.push(format!("protocol kind {} (policy protocols, not evaluated)", e.get("kind").and_then(|v| v.as_str()).unwrap_or("?")));
            }
        }
        for (k, n) in agg {
            out.push(format!("{k}: {n}"));
        }
        out.sort();
        out
    }

    pub fn match_family(&self, wire_namespace: &str) -> MatchResult {
        match_family_with_rules(wire_namespace, self)
    }

    pub fn extract_dimensions(&self, wire_namespace: &str) -> HashMap<String, String> {
        let m = self.match_family(wire_namespace);
        m.binds
    }

    pub fn check_polarity(&self, family_prefix: &str, score: f64) -> PolarityResult {
        let meta = match self.registry.get("_meta") {
            Some(m) => m,
            None => {
                return PolarityResult {
                    admissible: false,
                    class: None,
                    refusal: Some("namespace_family_unregistered".to_string()),
                }
            }
        };
        let pc = match meta.get("polarity_classes") {
            Some(p) => p,
            None => {
                return PolarityResult {
                    admissible: true,
                    class: None,
                    refusal: None,
                }
            }
        };

        let fam = self.registry.get("families")
            .and_then(|v| v.as_array())
            .and_then(|arr| {
                arr.iter().find(|f| f.get("prefix").and_then(|p| p.as_str()) == Some(family_prefix))
            });

        let fam = match fam {
            Some(f) => f,
            None => {
                let refusal = meta
                    .get("case_rule")
                    .and_then(|cr| cr.get("refusal_tokens"))
                    .and_then(|rt| rt.get("family_unregistered"))
                    .and_then(|s| s.as_str())
                    .unwrap_or("namespace_family_unregistered")
                    .to_string();
                return PolarityResult {
                    admissible: false,
                    class: None,
                    refusal: Some(refusal),
                };
            }
        };

        let raw_polarity = fam.get("polarity").and_then(|p| p.as_str()).unwrap_or("");
        let aliases = pc.get("_aliases");
        let name = aliases
            .and_then(|a| a.get(raw_polarity))
            .and_then(|s| s.as_str())
            .unwrap_or(raw_polarity);

        let cls = pc.get(name);
        let default_refusal = pc
            .get("refusal")
            .and_then(|r| r.as_str())
            .unwrap_or("score_outside_declared_polarity");

        let cls = match cls {
            Some(c) => c,
            None => {
                return PolarityResult {
                    admissible: true,
                    class: Some(name.to_string()),
                    refusal: None,
                }
            }
        };

        let kind = cls.get("kind").and_then(|k| k.as_str()).unwrap_or("");
        if kind == "per_leaf" {
            return PolarityResult {
                admissible: true,
                class: Some(name.to_string()),
                refusal: None,
            };
        }

        let ok = if kind == "points" {
            cls.get("points")
                .and_then(|p| p.as_array())
                .map(|arr| {
                    arr.iter().any(|val| {
                        if let Some(pv) = val.as_f64() {
                            (score - pv).abs() < 1e-9
                        } else if let Some(pv) = val.as_i64() {
                            (score - pv as f64).abs() < 1e-9
                        } else {
                            false
                        }
                    })
                })
                .unwrap_or(false)
        } else {
            let min = cls.get("min").and_then(|v| v.as_f64()).unwrap_or(-1.0);
            let max = cls.get("max").and_then(|v| v.as_f64()).unwrap_or(1.0);
            let min_inclusive = cls.get("min_inclusive").and_then(|v| v.as_bool()).unwrap_or(false);

            let lo_ok = score > min || (min_inclusive && (score - min).abs() < 1e-9);
            lo_ok && score <= max
        };

        PolarityResult {
            admissible: ok,
            class: Some(name.to_string()),
            refusal: if ok {
                None
            } else {
                Some(default_refusal.to_string())
            },
        }
    }
}

pub fn check_polarity(tables: &Tables, family_prefix: &str, score: f64) -> PolarityResult {
    tables.check_polarity(family_prefix, score)
}

pub fn transition_of(tables: &Tables, family: &str, verb: &str, state: &str) -> Option<Transition> {
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
    let ts = tables.families.get(fam_key)?;
    let res = ts.iter().find(|t| t.verb == verb && t.from == state).cloned();
    if res.is_none() && (verb == "voice" || verb == "assent" || verb == "raised" || verb == "object") {
        Some(Transition {
            verb: verb.to_string(),
            from: state.to_string(),
            to: state.to_string(),
            roles: vec!["member".to_string(), "subject".to_string(), "holder".to_string()],
            policy: "two_path".to_string(),
            policy_rows: Vec::new(),
            widens: false,
            ..Default::default()
        })
    } else {
        res
    }
}

pub fn policy_row<'a>(tables: &'a Tables, t: &Transition) -> Option<&'a Policy> {
    if t.policy == "selfAct" || t.policy == "none" {
        return None;
    }
    let default_val = String::new();
    let default_lookup = tables.defaults.get(&t.policy).unwrap_or(&default_val);
    let name = t.policy_rows.first().unwrap_or(default_lookup);
    if name.is_empty() {
        return None;
    }
    tables.policies.get(name)
}
