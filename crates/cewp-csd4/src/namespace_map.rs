//! CSD Namespace Map loader, parser, and structured query interface.
//!
//! Exposes `/home/emoore/CEWP/docs/kernel/tables/csd_namespace_map.json`:
//! - Maps all 105 CSD cards (`CSD-001..132`)
//! - Maps all 20 topologies (`T0..T19`)
//! - Maps all 191 CC RC17 namespace families (`94` UI surface + `97` autonomous substrate)
//! - Maps all 474 transitions
//! - Maps all 12 governance policies
//! - Resolves all 660 `csd:shows` fields with 3 canonical aliases

use std::collections::BTreeMap;
use crate::catalogue::TopologyId;

/// Pinned vendored JSON text embedded at compile-time.
pub const VENDORED_CSD_NAMESPACE_MAP_JSON: &str =
    include_str!("../../../docs/kernel/tables/csd_namespace_map.json");

/// Summary statistics of the CSD namespace map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamespaceSummary {
    pub total_families: usize,
    pub total_transitions: usize,
    pub total_policies: usize,
    pub total_csd_cards: usize,
    pub csd_mapped_families: usize,
    pub substrate_autonomous_families: usize,
    pub total_csd_shows_fields: usize,
    pub resolved_csd_shows_fields: usize,
}

/// A `csd:shows` field mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsdShowsField {
    pub field: String,
    pub canonical_family: String,
    pub use_type: String,
    pub field_type: String,
}

/// A CSD card specification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsdCardInfo {
    pub id: String,
    pub title: String,
    pub domain: String,
    pub section: String,
    pub topologies: Vec<String>,
    pub state_atoms: Vec<String>,
    pub shows: Vec<CsdShowsField>,
    pub transitions_exercised: Vec<String>,
    pub governance_policies: Vec<String>,
}

/// A namespace family specification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamespaceFamilyInfo {
    pub pattern: String,
    pub authority: String,
    pub rule_ids: Vec<String>,
    pub transitions_count: usize,
    pub roles_required: Vec<String>,
    pub category: String, // "csd_ui_surface" or "substrate_autonomous"
    pub referenced_by_csd_cards: Vec<String>,
    pub topologies: Vec<String>,
}

/// Mapping of a topology to its associated cards, families, and state atoms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopologyMapping {
    pub cards: Vec<String>,
    pub families: Vec<String>,
    pub state_atoms: Vec<String>,
}

/// The complete CSD namespace map.
#[derive(Debug, Clone)]
pub struct CsdNamespaceMap {
    pub version: String,
    pub summary: NamespaceSummary,
    pub aliases: BTreeMap<String, String>,
    pub cards: Vec<CsdCardInfo>,
    pub families: Vec<NamespaceFamilyInfo>,
    pub topologies: BTreeMap<String, TopologyMapping>,
    card_index: BTreeMap<String, usize>,
    family_index: BTreeMap<String, usize>,
    shows_field_map: BTreeMap<String, String>,
}

impl CsdNamespaceMap {
    /// Loads and parses the vendored CSD namespace map JSON.
    pub fn load_vendored() -> Self {
        Self::parse_json(VENDORED_CSD_NAMESPACE_MAP_JSON)
            .expect("Failed to parse vendored csd_namespace_map.json")
    }

    /// Parses a JSON string into `CsdNamespaceMap`.
    pub fn parse_json(json_str: &str) -> Result<Self, String> {
        let val = parse_json_value(json_str)?;
        let obj = val.as_object().ok_or("Root is not a JSON object")?;

        let version = obj.get("version").and_then(|v| v.as_str()).unwrap_or("").to_string();

        let sum_obj = obj.get("summary").and_then(|v| v.as_object()).ok_or("Missing summary object")?;
        let summary = NamespaceSummary {
            total_families: sum_obj.get("total_families").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
            total_transitions: sum_obj.get("total_transitions").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
            total_policies: sum_obj.get("total_policies").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
            total_csd_cards: sum_obj.get("total_csd_cards").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
            csd_mapped_families: sum_obj.get("csd_mapped_families").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
            substrate_autonomous_families: sum_obj.get("substrate_autonomous_families").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
            total_csd_shows_fields: sum_obj.get("total_csd_shows_fields").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
            resolved_csd_shows_fields: sum_obj.get("resolved_csd_shows_fields").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
        };

        let mut aliases = BTreeMap::new();
        if let Some(alias_obj) = obj.get("aliases").and_then(|v| v.as_object()) {
            for (k, v) in alias_obj {
                if let Some(target) = v.as_str() {
                    aliases.insert(k.clone(), target.to_string());
                }
            }
        }

        let mut cards = Vec::new();
        let mut card_index = BTreeMap::new();
        let mut shows_field_map = BTreeMap::new();

        if let Some(cards_arr) = obj.get("cards").and_then(|v| v.as_array()) {
            for c_val in cards_arr {
                if let Some(c_obj) = c_val.as_object() {
                    let id = c_obj.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let title = c_obj.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let domain = c_obj.get("domain").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let section = c_obj.get("section").and_then(|v| v.as_str()).unwrap_or("").to_string();

                    let topologies = extract_string_array(c_obj.get("topologies"));
                    let state_atoms = extract_string_array(c_obj.get("state_atoms"));
                    let transitions_exercised = extract_string_array(c_obj.get("transitions_exercised"));
                    let governance_policies = extract_string_array(c_obj.get("governance_policies"));

                    let mut shows = Vec::new();
                    if let Some(shows_arr) = c_obj.get("shows").and_then(|v| v.as_array()) {
                        for s_val in shows_arr {
                            if let Some(s_obj) = s_val.as_object() {
                                let field = s_obj.get("field").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                let canonical_family = s_obj.get("canonical_family").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                let use_type = s_obj.get("use").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                let field_type = s_obj.get("type").and_then(|v| v.as_str()).unwrap_or("").to_string();

                                shows_field_map.insert(field.clone(), canonical_family.clone());
                                shows.push(CsdShowsField {
                                    field,
                                    canonical_family,
                                    use_type,
                                    field_type,
                                });
                            }
                        }
                    }

                    let idx = cards.len();
                    card_index.insert(id.clone(), idx);
                    cards.push(CsdCardInfo {
                        id,
                        title,
                        domain,
                        section,
                        topologies,
                        state_atoms,
                        shows,
                        transitions_exercised,
                        governance_policies,
                    });
                }
            }
        }

        let mut families = Vec::new();
        let mut family_index = BTreeMap::new();

        if let Some(fams_arr) = obj.get("families").and_then(|v| v.as_array()) {
            for f_val in fams_arr {
                if let Some(f_obj) = f_val.as_object() {
                    let pattern = f_obj.get("pattern").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let authority = f_obj.get("authority").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let rule_ids = extract_string_array(f_obj.get("rule_ids"));
                    let transitions_count = f_obj.get("transitions_count").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                    let roles_required = extract_string_array(f_obj.get("roles_required"));
                    let category = f_obj.get("category").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let referenced_by_csd_cards = extract_string_array(f_obj.get("referenced_by_csd_cards"));
                    let topologies = extract_string_array(f_obj.get("topologies"));

                    let idx = families.len();
                    family_index.insert(pattern.clone(), idx);
                    families.push(NamespaceFamilyInfo {
                        pattern,
                        authority,
                        rule_ids,
                        transitions_count,
                        roles_required,
                        category,
                        referenced_by_csd_cards,
                        topologies,
                    });
                }
            }
        }

        let mut topologies = BTreeMap::new();
        if let Some(topos_obj) = obj.get("topologies").and_then(|v| v.as_object()) {
            for (t_id, t_val) in topos_obj {
                if let Some(t_obj) = t_val.as_object() {
                    let cards_list = extract_string_array(t_obj.get("cards"));
                    let fams_list = extract_string_array(t_obj.get("families"));
                    let atoms_list = extract_string_array(t_obj.get("state_atoms"));
                    topologies.insert(t_id.clone(), TopologyMapping {
                        cards: cards_list,
                        families: fams_list,
                        state_atoms: atoms_list,
                    });
                }
            }
        }

        Ok(Self {
            version,
            summary,
            aliases,
            cards,
            families,
            topologies,
            card_index,
            family_index,
            shows_field_map,
        })
    }

    /// Returns cards associated with the given topology.
    pub fn cards_for_topology(&self, topo: TopologyId) -> Vec<&CsdCardInfo> {
        if let Some(mapping) = self.topologies.get(topo.as_str()) {
            mapping.cards.iter()
                .filter_map(|cid| self.card_by_id(cid))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Returns namespace families associated with the given topology.
    pub fn families_for_topology(&self, topo: TopologyId) -> Vec<&NamespaceFamilyInfo> {
        if let Some(mapping) = self.topologies.get(topo.as_str()) {
            mapping.families.iter()
                .filter_map(|pat| self.family_by_pattern(pat))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Returns a card by its identifier (e.g. "CSD-001", "CSD-118").
    pub fn card_by_id(&self, id: &str) -> Option<&CsdCardInfo> {
        self.card_index.get(id).map(|&idx| &self.cards[idx])
    }

    /// Returns a namespace family by its pattern (e.g. "accord:*", "consent:{kind}").
    pub fn family_by_pattern(&self, pattern: &str) -> Option<&NamespaceFamilyInfo> {
        self.family_index.get(pattern).map(|&idx| &self.families[idx])
    }

    /// Resolves a `csd:shows` field name to its canonical family pattern.
    ///
    /// Supports:
    /// - Explicit aliases (e.g. "consent_request" -> "consent", "directory" -> "directory:{entry_type}:{key}")
    /// - All 660 csd:shows fields mapped to their canonical_family
    /// - Direct family pattern pass-through
    pub fn resolve_csd_shows(&self, field: &str) -> Option<&str> {
        // 1. Explicit alias resolution
        if let Some(alias_target) = self.aliases.get(field) {
            return Some(alias_target.as_str());
        }
        // 2. Direct field-to-canonical-family mapping from shows blocks
        if let Some(canonical) = self.shows_field_map.get(field) {
            return Some(canonical.as_str());
        }
        // 3. Direct family pattern match
        if let Some(fam) = self.family_by_pattern(field) {
            return Some(fam.pattern.as_str());
        }
        None
    }
}

fn extract_string_array(val: Option<&JsonValue>) -> Vec<String> {
    match val.and_then(|v| v.as_array()) {
        Some(arr) => arr.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect(),
        None => Vec::new(),
    }
}

// ── Pure-std JSON Parser ───────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(BTreeMap<String, JsonValue>),
}

impl JsonValue {
    fn as_object(&self) -> Option<&BTreeMap<String, JsonValue>> {
        match self {
            JsonValue::Object(map) => Some(map),
            _ => None,
        }
    }

    fn as_array(&self) -> Option<&Vec<JsonValue>> {
        match self {
            JsonValue::Array(arr) => Some(arr),
            _ => None,
        }
    }

    fn as_str(&self) -> Option<&str> {
        match self {
            JsonValue::String(s) => Some(s),
            _ => None,
        }
    }

    fn as_u64(&self) -> Option<u64> {
        match self {
            JsonValue::Number(n) if *n >= 0.0 => Some(*n as u64),
            _ => None,
        }
    }
}

fn parse_json_value(input: &str) -> Result<JsonValue, String> {
    let bytes = input.as_bytes();
    let mut pos = 0;
    skip_ws(bytes, &mut pos);
    let val = parse_val(bytes, &mut pos)?;
    skip_ws(bytes, &mut pos);
    if pos < bytes.len() {
        return Err(format!("Trailing data at byte {}", pos));
    }
    Ok(val)
}

fn skip_ws(bytes: &[u8], pos: &mut usize) {
    while *pos < bytes.len() && matches!(bytes[*pos], b' ' | b'\t' | b'\r' | b'\n') {
        *pos += 1;
    }
}

fn parse_val(bytes: &[u8], pos: &mut usize) -> Result<JsonValue, String> {
    skip_ws(bytes, pos);
    if *pos >= bytes.len() {
        return Err("Unexpected EOF while parsing JSON".to_string());
    }
    match bytes[*pos] {
        b'{' => parse_obj(bytes, pos),
        b'[' => parse_arr(bytes, pos),
        b'"' => parse_str(bytes, pos).map(JsonValue::String),
        b't' | b'f' => parse_bool(bytes, pos).map(JsonValue::Bool),
        b'n' => parse_null(bytes, pos),
        b'-' | b'0'..=b'9' => parse_num(bytes, pos).map(JsonValue::Number),
        b => Err(format!("Unexpected character '{}' at pos {}", b as char, *pos)),
    }
}

fn parse_obj(bytes: &[u8], pos: &mut usize) -> Result<JsonValue, String> {
    *pos += 1; // '{'
    let mut map = BTreeMap::new();
    skip_ws(bytes, pos);
    if *pos < bytes.len() && bytes[*pos] == b'}' {
        *pos += 1;
        return Ok(JsonValue::Object(map));
    }

    loop {
        skip_ws(bytes, pos);
        if *pos >= bytes.len() || bytes[*pos] != b'"' {
            return Err(format!("Expected string key at pos {}", *pos));
        }
        let key = parse_str(bytes, pos)?;
        skip_ws(bytes, pos);
        if *pos >= bytes.len() || bytes[*pos] != b':' {
            return Err(format!("Expected ':' at pos {}", *pos));
        }
        *pos += 1; // ':'
        skip_ws(bytes, pos);
        let val = parse_val(bytes, pos)?;
        map.insert(key, val);

        skip_ws(bytes, pos);
        if *pos >= bytes.len() {
            return Err("Unexpected EOF in object".to_string());
        }
        if bytes[*pos] == b',' {
            *pos += 1;
        } else if bytes[*pos] == b'}' {
            *pos += 1;
            break;
        } else {
            return Err(format!("Expected ',' or '}}' at pos {}", *pos));
        }
    }
    Ok(JsonValue::Object(map))
}

fn parse_arr(bytes: &[u8], pos: &mut usize) -> Result<JsonValue, String> {
    *pos += 1; // '['
    let mut arr = Vec::new();
    skip_ws(bytes, pos);
    if *pos < bytes.len() && bytes[*pos] == b']' {
        *pos += 1;
        return Ok(JsonValue::Array(arr));
    }

    loop {
        skip_ws(bytes, pos);
        let val = parse_val(bytes, pos)?;
        arr.push(val);
        skip_ws(bytes, pos);
        if *pos >= bytes.len() {
            return Err("Unexpected EOF in array".to_string());
        }
        if bytes[*pos] == b',' {
            *pos += 1;
        } else if bytes[*pos] == b']' {
            *pos += 1;
            break;
        } else {
            return Err(format!("Expected ',' or ']' at pos {}", *pos));
        }
    }
    Ok(JsonValue::Array(arr))
}

fn parse_str(bytes: &[u8], pos: &mut usize) -> Result<String, String> {
    *pos += 1; // opening '"'
    let mut s = String::new();
    while *pos < bytes.len() {
        let b = bytes[*pos];
        if b == b'"' {
            *pos += 1;
            return Ok(s);
        } else if b == b'\\' {
            *pos += 1;
            if *pos >= bytes.len() {
                return Err("Unexpected EOF in string escape".to_string());
            }
            match bytes[*pos] {
                b'"' => s.push('"'),
                b'\\' => s.push('\\'),
                b'/' => s.push('/'),
                b'b' => s.push('\x08'),
                b'f' => s.push('\x0c'),
                b'n' => s.push('\n'),
                b'r' => s.push('\r'),
                b't' => s.push('\t'),
                b'u' => {
                    *pos += 1;
                    if *pos + 4 > bytes.len() {
                        return Err("Unexpected EOF in unicode escape".to_string());
                    }
                    let hex_str = std::str::from_utf8(&bytes[*pos..*pos + 4])
                        .map_err(|e| e.to_string())?;
                    let code = u32::from_str_radix(hex_str, 16)
                        .map_err(|e| format!("Invalid hex: {}", e))?;
                    let ch = char::from_u32(code).unwrap_or('\u{FFFD}');
                    s.push(ch);
                    *pos += 3; // +1 at loop step
                }
                esc => s.push(esc as char),
            }
            *pos += 1;
        } else {
            // Direct UTF-8 byte sequence
            let start = *pos;
            while *pos < bytes.len() && bytes[*pos] != b'"' && bytes[*pos] != b'\\' {
                *pos += 1;
            }
            let slice = std::str::from_utf8(&bytes[start..*pos])
                .map_err(|e| e.to_string())?;
            s.push_str(slice);
        }
    }
    Err("Unclosed string literal".to_string())
}

fn parse_bool(bytes: &[u8], pos: &mut usize) -> Result<bool, String> {
    if bytes[*pos..].starts_with(b"true") {
        *pos += 4;
        Ok(true)
    } else if bytes[*pos..].starts_with(b"false") {
        *pos += 5;
        Ok(false)
    } else {
        Err(format!("Expected boolean at pos {}", *pos))
    }
}

fn parse_null(bytes: &[u8], pos: &mut usize) -> Result<JsonValue, String> {
    if bytes[*pos..].starts_with(b"null") {
        *pos += 4;
        Ok(JsonValue::Null)
    } else {
        Err(format!("Expected null at pos {}", *pos))
    }
}

fn parse_num(bytes: &[u8], pos: &mut usize) -> Result<f64, String> {
    let start = *pos;
    if *pos < bytes.len() && bytes[*pos] == b'-' {
        *pos += 1;
    }
    while *pos < bytes.len() && (bytes[*pos].is_ascii_digit() || bytes[*pos] == b'.' || bytes[*pos] == b'e' || bytes[*pos] == b'E' || bytes[*pos] == b'+' || bytes[*pos] == b'-') {
        *pos += 1;
    }
    let slice = std::str::from_utf8(&bytes[start..*pos])
        .map_err(|e| e.to_string())?;
    slice.parse::<f64>().map_err(|e| format!("Invalid number '{}': {}", slice, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_vendored_summary() {
        let map = CsdNamespaceMap::load_vendored();

        assert_eq!(map.summary.total_families, 191);
        assert_eq!(map.summary.total_transitions, 474);
        assert_eq!(map.summary.total_policies, 12);
        assert_eq!(map.summary.total_csd_cards, 105);
        assert_eq!(map.summary.csd_mapped_families, 94);
        assert_eq!(map.summary.substrate_autonomous_families, 97);
        assert_eq!(map.summary.total_csd_shows_fields, 660);
        assert_eq!(map.summary.resolved_csd_shows_fields, 660);

        assert_eq!(map.cards.len(), 105);
        assert_eq!(map.families.len(), 191);
        assert_eq!(map.topologies.len(), 20);
    }

    #[test]
    fn test_aliases_resolution() {
        let map = CsdNamespaceMap::load_vendored();

        assert_eq!(map.resolve_csd_shows("consent_request"), Some("consent"));
        assert_eq!(map.resolve_csd_shows("directory"), Some("directory:{entry_type}:{key}"));
        assert_eq!(map.resolve_csd_shows("trust:attestation:device"), Some("trust:{job}:{version}"));
    }

    #[test]
    fn test_shows_field_resolution() {
        let map = CsdNamespaceMap::load_vendored();

        // Specific fields from csd_shows
        assert_eq!(map.resolve_csd_shows("accord:halt_status"), Some("accord:*"));
        assert_eq!(map.resolve_csd_shows("need:{domain}:{kind}"), Some("need:{domain}:{kind}"));
    }

    #[test]
    fn test_topologies_lookups() {
        let map = CsdNamespaceMap::load_vendored();

        for id in TopologyId::ALL {
            let cards = map.cards_for_topology(id);
            let families = map.families_for_topology(id);
            // Every topology maps to its cards
            assert!(!cards.is_empty(), "Topology {} should have cards", id);
            if id != TopologyId::T9 {
                assert!(!families.is_empty(), "Topology {} should have families", id);
            }
        }

        // T0 specifics
        let t0_cards = map.cards_for_topology(TopologyId::T0);
        assert_eq!(t0_cards.len(), 30);
        assert!(t0_cards.iter().any(|c| c.id == "CSD-118"));
    }
}
