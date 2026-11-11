//! The check that every figure on the public twin page has a ledger entry.
//!
//! The page marks a figure in one of three ways:
//! - static text: `<span class="fig" data-claim="<id>">…</span>`;
//! - a live value written into an element: the element carries `data-claim="<id>"`, or
//!   `data-evidence="input"` when it only echoes a control the visitor set, or
//!   `data-evidence="figs"` when its content is built from `fig()` calls;
//! - a value inside a template: `${fig("<id>", value)}`.
//!
//! The scan (std only, no JS engine) reads `docs/globe.js`, `docs/tiling.js`, `docs/evidence/evidence.js`
//! and the two HTML pages
//! and reports:
//! 1. a `data-claim` / `fig()` id with no ledger entry, and a ledger entry the page never shows;
//! 2. an element written with `.textContent` / `.innerHTML` that carries no claim (unless it is a
//!    control listed in [`UI_IDS`] with its reason);
//! 4. a globe layer (`docs/layers/*.js`, registered as `window.CEWP_LAYERS["<id>"] = { id: "<id>", cls: "<CLASS>", … }`)
//!    without a ledger entry `layer-<id>`, or whose declared class differs from that entry's class,
//!    and a `layer-*` entry no layer registers;
//! 3. inside the regions marked `// evidence:scan` … `// evidence:end` in `globe.js` and
//!    `evidence.js`: a digit in
//!    visible text outside a figure, unless its token is a name listed in [`NON_FIGURES`] or a
//!    citation (section, issue, version, commit, tier name); and a `${…}` interpolation outside a
//!    `fig()` call that reads anything but the identifiers in [`LABEL_IDENTS`].
//!
//! What it cannot see: numbers spelled out in words, numbers concatenated with `+` into a string
//! (scanned regions use templates for that reason), tooltip (`title=`) text, canvas text drawn
//! with `fillText`, and the `<head>` (title, meta description).

use crate::page_ledger::PAGE_CLAIMS;

/// Elements written as text that are controls or story chrome, not figures, with the reason.
pub const UI_IDS: &[(&str, &str)] = &[
    ("play", "run / pause button label"),
    ("eng", "engine toggle label (WASM / JS)"),
    ("story-act-badge", "story step counter (step n of 8)"),
    ("story-title", "story step title"),
    ("story-btn-play", "story play / pause glyph"),
    ("story-line", "story caption: its text comes from the scanned STORY_CAPTIONS / MICRO_BEATS regions"),
    ("sf-chip-tg", "menu toggle label"),
    ("focal-chip", "street readout: its template is a scanned region (updateFocalChip)"),
    ("node-card", "device card: its template is a scanned region (renderNodeCard)"),
    ("claim-card", "the evidence card itself: renders ledger text"),
    ("tier-now", "tier readout: its template is a scanned region (updateTierNow)"),
];

/// Tokens with digits that are names, not figures, with the reason.
pub const NON_FIGURES: &[(&str, &str)] = &[
    ("1:1", "the R5 tier's name (one node per device)"),
    ("1-hop", "the CC 5.4.3 term for a shaped link (one direct hop)"),
    ("6-Peer", "name of the six-peer kernel demo page"),
    ("CSD/4", "name of the topology catalogue"),
    ("110m", "Natural Earth dataset scale name"),
    ("50m", "Natural Earth dataset scale name"),
    ("10m", "Natural Earth dataset scale name"),
    ("1:10m", "Natural Earth dataset scale name"),
    ("2022", "dataset name: NOAA ETOPO 2022"),
    ("H3ERE", "name of the agent reasoning-trace format"),
    ("1/8", "initial story step counter before the script runs"),
    ("6-peer", "name of the six-peer kernel demo page"),
    ("seat-1", "the production serve seat's name in the FSD (aggregates only)"),
    ("IPv6", "protocol name"),
    ("P2P", "abbreviation: peer to peer"),
    ("pre-v44", "a version reference (edge before v44)"),
];

/// Identifiers a `${…}` interpolation may read outside a `fig()` call: labels, enum lookups and
/// markup fragments that are themselves built from scanned templates.
pub const LABEL_IDENTS: &[&str] = &[
    // lookups into label tables and the values that index them
    "MK", "MK_COL", "MDCLASS", "MACT", "MLINK", "MCLASS", "MROUTER", "BS_LABEL", "BS_SHORT", "BS_TITLE", "KEY_PLANE", "CAP_BINDS",
    "microBootstrap", "microDesign", "microStory", "kind", "act", "cls", "k", "sc", "asked", "bound", "nodeKind", "dclass",
    // device index labels (#3) and link openers: names in the generated world, not quantities
    "i", "o", "opener", "by",
    // fragments built from scanned templates or label strings
    "state", "reach", "relayWhy", "shapesWhy", "repl", "capHtml", "tr", "rows", "join", "kindsTxt", "scale", "hm", "dev", "f", "NF",
    "unmetered", "charging", "claimed", "mailbox", "old", "yes", "chipSub", "seatsBtn", "here", "tierTxt", "cellTxt", "sizeTxt", "lockTxt",
    "kneeTxt", "c", "kmTxt", "s", "world", "timing", "engineTxt", "map", "seatKm",
    // read only as a condition that decides whether a fig()-wrapped line shows
    "holding", "pending",
    // story helpers whose numeric parts are wrapped in fig() where they are built
    "helperPhrase", "homesOf", "ap", "MSTORY", "route1", "viaSeat", "outcome", "ci",
    // JS keywords and literals that can appear in a conditional interpolation
    "true", "false", "null", "undefined",
];

/// One problem found by the scan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// File (relative to `docs/`).
    pub file: String,
    /// 1-based line.
    pub line: usize,
    /// What is wrong.
    pub what: String,
}

impl std::fmt::Display for Finding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "docs/{}:{}: {}", self.file, self.line, self.what)
    }
}

fn line_of(src: &str, byte: usize) -> usize {
    src[..byte.min(src.len())].bytes().filter(|&b| b == b'\n').count() + 1
}

/// Every `data-claim="…"` and `fig("…"` id in `src`, with its byte offset.
#[must_use]
pub fn claim_refs(src: &str) -> Vec<(String, usize)> {
    let mut out = Vec::new();
    for pat in ["data-claim=\"", "fig(\""] {
        let mut from = 0;
        while let Some(p) = src[from..].find(pat) {
            let s = from + p + pat.len();
            // `fig(` must be the helper, not the tail of another identifier
            let ok = pat != "fig(\"" || !src[..from + p].ends_with(|c: char| c.is_ascii_alphanumeric() || c == '_' || c == '$' || c == '.');
            if let Some(e) = src[s..].find('"') {
                // `data-claim="${id}"` inside the fig() helper itself is not a reference
                if ok && !src[s..s + e].contains('$') {
                    out.push((src[s..s + e].to_string(), s));
                }
                from = s + e;
            } else {
                break;
            }
        }
    }
    out
}

/// The globe layers a script registers: `CEWP_LAYERS["<id>"] = { id: "<id>", cls: "<CLASS>", … }`.
/// Returns (key, id field, cls field, offset); a field that is not a string literal comes back empty.
#[must_use]
pub fn layer_regs(src: &str) -> Vec<(String, String, String, usize)> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(p) = src[from..].find("CEWP_LAYERS[") {
        let at = from + p;
        let k = at + "CEWP_LAYERS[".len();
        from = k;
        let key = lit_at(src, k).unwrap_or_default();
        // only an assignment registers: `] = {`
        let Some(close) = src[k..].find(']') else { break };
        let rest = src[k + close + 1..].trim_start();
        if !rest.starts_with('=') || rest.starts_with("==") {
            continue;
        }
        let Some(open) = src[k + close..].find('{').map(|o| k + close + o) else { break };
        let end = balanced_end(src, open, src.len(), b'{', b'}');
        let body = &src[open..end];
        let field = |name: &str| -> String {
            for sep in [": ", ":"] {
                let pat = format!("{name}{sep}");
                let mut f = 0;
                while let Some(q) = body[f..].find(&pat) {
                    let i = f + q;
                    let before = body[..i].chars().next_back().unwrap_or(' ');
                    if !(before.is_ascii_alphanumeric() || before == '_') {
                        return lit_at(body, i + pat.len()).unwrap_or_default();
                    }
                    f = i + pat.len();
                }
            }
            String::new()
        };
        out.push((key, field("id"), field("cls"), at));
        from = end.max(k);
    }
    out
}

/// The string literal starting at `i` (`"…"` or `'…'`), if there is one.
fn lit_at(src: &str, i: usize) -> Option<String> {
    let q = *src.as_bytes().get(i)?;
    if q != b'"' && q != b'\'' {
        return None;
    }
    let e = src[i + 1..].find(q as char)?;
    Some(src[i + 1..i + 1 + e].to_string())
}

/// Findings for the globe layers in `layers` (file name, source).
#[must_use]
pub fn check_layers(layers: &[(String, String)]) -> Vec<Finding> {
    let mut out = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for (f, s) in layers {
        for (key, id, cls, at) in layer_regs(s) {
            let line = line_of(s, at);
            let mut bad = |what: String| out.push(Finding { file: f.clone(), line, what });
            if key.is_empty() || id.is_empty() || cls.is_empty() {
                bad("layer registration needs string literals for its key, `id:` and `cls:`".into());
                continue;
            }
            if key != id {
                bad(format!("layer registered as '{}' declares id '{}'", key, id));
            }
            seen.insert(id.clone());
            match crate::page_ledger::lookup(&format!("layer-{}", id)) {
                None => bad(format!("layer '{}' has no ledger entry 'layer-{}'", id, id)),
                Some(c) if c.class.as_str() != cls => bad(format!("layer '{}' declares {} but its ledger entry says {}", id, cls, c.class.as_str())),
                Some(_) => {}
            }
        }
    }
    for c in PAGE_CLAIMS {
        if let Some(id) = c.id.strip_prefix("layer-") {
            if !seen.contains(id) {
                out.push(Finding { file: "layers/".into(), line: 0, what: format!("ledger entry '{}' names a layer no file in docs/layers registers", c.id) });
            }
        }
    }
    out
}

/// Ids of elements the scripts write as text, with the offset of the write.
#[must_use]
pub fn written_ids(src: &str) -> Vec<(String, usize)> {
    let mut out = Vec::new();
    let pat = "$(\"";
    let mut from = 0;
    while let Some(p) = src[from..].find(pat) {
        let s = from + p + pat.len();
        let Some(e) = src[s..].find('"') else { break };
        let id = &src[s..s + e];
        let rest = src[s + e..].trim_start_matches(['"', ')']);
        if rest.starts_with(".textContent") || rest.starts_with(".innerHTML") {
            out.push((id.to_string(), s));
        }
        from = s + e;
    }
    // tiling.js-style: `const el = document.getElementById("x")` then `el.textContent = …`
    let pat = "getElementById(\"";
    let mut from = 0;
    while let Some(p) = src[from..].find(pat) {
        let s = from + p + pat.len();
        let Some(e) = src[s..].find('"') else { break };
        let mut span_end = (s + 400).min(src.len());
        while !src.is_char_boundary(span_end) {
            span_end -= 1;
        }
        let span = &src[s..span_end];
        if span.contains("el.textContent") || span.contains("el.innerHTML") {
            out.push((src[s..s + e].to_string(), s));
        }
        from = s + e;
    }
    out
}

/// The opening tag of the element with `id="<id>"` in `src`, if any.
#[must_use]
pub fn opening_tag<'a>(src: &'a str, id: &str) -> Option<&'a str> {
    let needle = format!("id=\"{}\"", id);
    let p = src.find(&needle)?;
    let start = src[..p].rfind('<')?;
    let end = p + src[p..].find('>')?;
    Some(&src[start..=end])
}

/// A piece of a scanned region.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Piece {
    /// Text of a string or template literal; `seq` groups the pieces of one literal.
    Text {
        /// The literal's text (escapes undone).
        text: String,
        /// Byte offset in the source.
        at: usize,
        /// Literal number.
        seq: usize,
    },
    /// A `${…}` interpolation (outside any `fig()` call) of literal `seq`.
    Interp {
        /// The expression between `${` and `}`.
        expr: String,
        /// Byte offset in the source.
        at: usize,
        /// Literal number.
        seq: usize,
    },
}

/// Lexes JavaScript between the `// evidence:scan` and `// evidence:end` markers into literal text
/// and interpolations, skipping comments and whole `fig(…)` calls.
#[must_use]
pub fn scan_regions(src: &str) -> Vec<Piece> {
    let mut out = Vec::new();
    let mut seq = 0usize;
    let mut from = 0;
    while let Some(p) = src[from..].find("// evidence:scan") {
        let start = from + p;
        let body = src[start..].find('\n').map_or(src.len(), |n| start + n + 1);
        let end = src[body..].find("// evidence:end").map_or(src.len(), |n| body + n);
        lex_code(src, body, end, &mut out, &mut seq);
        from = end.max(start + 1);
    }
    out
}

fn lex_code(src: &str, mut i: usize, end: usize, out: &mut Vec<Piece>, seq: &mut usize) {
    let b = src.as_bytes();
    while i < end {
        let c = b[i];
        if c == b'/' && i + 1 < end && b[i + 1] == b'/' {
            i = src[i..end].find('\n').map_or(end, |n| i + n + 1);
        } else if c == b'/' && i + 1 < end && b[i + 1] == b'*' {
            i = src[i + 2..end].find("*/").map_or(end, |n| i + 2 + n + 2);
        } else if c == b'"' || c == b'\'' {
            let close = string_end(b, i, end, c);
            *seq += 1;
            out.push(Piece::Text { text: unescape(&src[i + 1..close]), at: i + 1, seq: *seq });
            i = close + 1;
        } else if c == b'`' {
            i = lex_template(src, i, end, out, seq);
        } else if src[i..end].starts_with("fig(") && (i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_' || b[i - 1] == b'.' || b[i - 1] == b'$')) {
            i = balanced_end(src, i + 3, end, b'(', b')');
        } else {
            i += 1;
        }
    }
}

fn string_end(b: &[u8], open: usize, end: usize, q: u8) -> usize {
    let mut j = open + 1;
    while j < end {
        if b[j] == b'\\' {
            j += 2;
            continue;
        }
        if b[j] == q || b[j] == b'\n' {
            return j;
        }
        j += 1;
    }
    end
}

/// Index just past the bracket that closes the one at `open` (strings and templates skipped).
fn balanced_end(src: &str, open: usize, end: usize, lo: u8, hi: u8) -> usize {
    let b = src.as_bytes();
    let mut depth = 0i32;
    let mut j = open;
    while j < end {
        let c = b[j];
        if c == b'"' || c == b'\'' {
            j = string_end(b, j, end, c) + 1;
            continue;
        }
        if c == b'`' {
            j = template_end(src, j, end) + 1;
            continue;
        }
        if c == lo {
            depth += 1;
        } else if c == hi {
            depth -= 1;
            if depth == 0 {
                return j + 1;
            }
        }
        j += 1;
    }
    end
}

/// Index of the backtick that closes the template opened at `open`.
fn template_end(src: &str, open: usize, end: usize) -> usize {
    let b = src.as_bytes();
    let mut j = open + 1;
    while j < end {
        match b[j] {
            b'\\' => j += 2,
            b'`' => return j,
            b'$' if j + 1 < end && b[j + 1] == b'{' => j = balanced_end(src, j + 1, end, b'{', b'}'),
            _ => j += 1,
        }
    }
    end
}

fn lex_template(src: &str, open: usize, end: usize, out: &mut Vec<Piece>, seq: &mut usize) -> usize {
    let b = src.as_bytes();
    *seq += 1;
    let my = *seq;
    let mut j = open + 1;
    let mut text_start = j;
    while j < end {
        match b[j] {
            b'\\' => j += 2,
            b'`' => {
                out.push(Piece::Text { text: unescape(&src[text_start..j]), at: text_start, seq: my });
                return j + 1;
            }
            b'$' if j + 1 < end && b[j + 1] == b'{' => {
                out.push(Piece::Text { text: unescape(&src[text_start..j]), at: text_start, seq: my });
                let close = balanced_end(src, j + 1, end, b'{', b'}');
                let expr = &src[j + 2..close - 1];
                out.push(Piece::Interp { expr: expr.to_string(), at: j + 2, seq: my });
                // literals inside the expression are scanned too, fig() calls skipped
                lex_code(src, j + 2, close - 1, out, seq);
                j = close;
                text_start = j;
            }
            _ => j += 1,
        }
    }
    end
}

fn unescape(s: &str) -> String {
    s.replace("\\n", " ").replace("\\'", "'").replace("\\\"", "\"").replace("\\`", "`")
}

fn decode_entities(s: &str) -> String {
    s.replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").replace("&minus;", "-").replace("&nbsp;", " ")
}

/// Whether a token with a digit is a citation or a listed name rather than a figure.
#[must_use]
pub fn is_citation_or_name(tok: &str) -> bool {
    if NON_FIGURES.iter().any(|(n, _)| *n == tok) {
        return true;
    }
    let digits_dots = |s: &str| !s.is_empty() && s.split('.').all(|p| !p.is_empty() && p.bytes().all(|c| c.is_ascii_digit()));
    // CC / FSD section: 5.4.3, 4.5.13, §4.2, §4.6.6
    if let Some(rest) = tok.strip_prefix('§') {
        if digits_dots(rest) {
            return true;
        }
    }
    if digits_dots(tok) && tok.matches('.').count() >= 2 {
        return true;
    }
    // issue: #939, leviculum#100, CIRISEdge#939
    if let Some(h) = tok.rfind('#') {
        let (head, num) = (&tok[..h], &tok[h + 1..]);
        if !num.is_empty() && num.bytes().all(|c| c.is_ascii_digit()) && head.bytes().all(|c| c.is_ascii_alphabetic()) {
            return true;
        }
    }
    // version: v45, v45.0.0, v0.2
    if let Some(rest) = tok.strip_prefix('v') {
        if digits_dots(rest) {
            return true;
        }
    }
    // tier and catalogue names: R0..R5, T5, A1, L0
    let tb = tok.as_bytes();
    let name_len_ok = match tb.first() {
        Some(b'R' | b'A' | b'L') => tb.len() == 2,
        Some(b'T') => (2..=3).contains(&tb.len()),
        _ => false,
    };
    if name_len_ok && tb[1..].iter().all(|c| c.is_ascii_digit()) {
        return true;
    }
    // commit: 7 to 40 lowercase hex with at least one letter
    if (7..=40).contains(&tok.len()) && tok.bytes().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()) && tok.bytes().any(|c| c.is_ascii_alphabetic()) {
        return true;
    }
    false
}

fn trim_token(t: &str) -> &str {
    let t = t.strip_suffix("'s").or_else(|| t.strip_suffix("’s")).unwrap_or(t);
    t.trim_matches(|c: char| "()[]{},;:!?·\"'.…–—=≤≥<>~+−*×→/".contains(c))
}

/// Visible-text digits outside a figure, and interpolations that read a value, in one literal's
/// pieces (in order). Tag state carries across the pieces of one template.
fn check_literal(pieces: &[&Piece], src: &str, file: &str, out: &mut Vec<Finding>) {
    let mut in_tag = false;
    let mut tag = String::new();
    let mut skip: Option<(String, usize)> = None; // (tag name, depth) of a figure being skipped
    let mut visible = String::new();
    let mut vis_at = 0usize;
    let flush = |visible: &mut String, at: usize, out: &mut Vec<Finding>| {
        let text = decode_entities(visible);
        for raw in text.split(|c: char| c.is_whitespace()) {
            let tok = trim_token(raw);
            if tok.bytes().any(|c| c.is_ascii_digit()) && !is_citation_or_name(tok) {
                out.push(Finding { file: file.into(), line: line_of(src, at), what: format!("figure '{}' in visible text has no data-claim (wrap it in <span class=\"fig\" data-claim=…> or fig())", tok) });
            }
        }
        visible.clear();
    };
    for p in pieces {
        match p {
            Piece::Text { text, at, .. } => {
                for (bi, ch) in text.char_indices() {
                    if in_tag {
                        tag.push(ch);
                        if ch == '>' {
                            in_tag = false;
                            let t = tag.trim_start_matches('<');
                            let closing = t.starts_with('/');
                            let name: String = t.trim_start_matches('/').chars().take_while(|c| c.is_ascii_alphanumeric()).collect();
                            if let Some((sn, d)) = skip.as_mut() {
                                if *sn == name {
                                    if closing {
                                        *d -= 1;
                                    } else if !t.ends_with("/>") {
                                        *d += 1;
                                    }
                                }
                                if *d == 0 {
                                    skip = None;
                                }
                            } else if !closing && (t.contains("data-claim=") || t.contains("class=\"inp\"") || t.contains("data-evidence=")) {
                                skip = Some((name, 1));
                            }
                            tag.clear();
                            visible.push(' ');
                        }
                    } else if ch == '<' {
                        flush(&mut visible, vis_at, out);
                        in_tag = true;
                        tag.push('<');
                    } else if skip.is_none() {
                        if visible.trim().is_empty() {
                            vis_at = at + bi;
                        }
                        visible.push(ch);
                    }
                }
            }
            Piece::Interp { expr, at, .. } => {
                if in_tag || skip.is_some() {
                    continue;
                }
                visible.push(' ');
                for id in idents(&strip_literals_and_figs(expr)) {
                    if !LABEL_IDENTS.contains(&id.as_str()) {
                        out.push(Finding { file: file.into(), line: line_of(src, *at), what: format!("interpolation `${{{}}}` reads '{}' outside fig(): wrap the value in fig(\"<claim>\", …)", expr.trim(), id) });
                        break;
                    }
                }
            }
        }
    }
    flush(&mut visible, vis_at, out);
}

/// The expression with string/template literals blanked and `fig(…)` calls removed.
fn strip_literals_and_figs(expr: &str) -> String {
    let b = expr.as_bytes();
    let mut o = String::new();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c == b'"' || c == b'\'' {
            i = string_end(b, i, b.len(), c) + 1;
            o.push(' ');
        } else if c == b'`' {
            i = template_end(expr, i, b.len()) + 1;
            o.push(' ');
        } else if expr[i..].starts_with("fig(") && (i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_' || b[i - 1] == b'.')) {
            i = balanced_end(expr, i + 3, b.len(), b'(', b')');
            o.push(' ');
        } else {
            o.push(c as char);
            i += 1;
        }
    }
    o
}

fn idents(s: &str) -> Vec<String> {
    let mut v = Vec::new();
    let mut cur = String::new();
    let mut prev_digit = false;
    for ch in s.chars().chain(std::iter::once(' ')) {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '$' {
            if cur.is_empty() && ch.is_ascii_digit() {
                prev_digit = true;
            }
            if !prev_digit {
                cur.push(ch);
            }
        } else {
            if !cur.is_empty() {
                v.push(std::mem::take(&mut cur));
            }
            prev_digit = false;
        }
    }
    v
}

/// Scans one source for figures in its scanned regions.
#[must_use]
pub fn scan_figures(src: &str, file: &str) -> Vec<Finding> {
    let pieces = scan_regions(src);
    let mut out = Vec::new();
    let mut i = 0;
    while i < pieces.len() {
        let s = match &pieces[i] {
            Piece::Text { seq, .. } | Piece::Interp { seq, .. } => *seq,
        };
        let group: Vec<&Piece> = pieces[i..].iter().take_while(|p| matches!(p, Piece::Text { seq, .. } | Piece::Interp { seq, .. } if *seq == s)).collect();
        i += group.len().max(1);
        check_literal(&group, src, file, &mut out);
    }
    out
}

/// Runs every check over the page in `docs/`.
#[must_use]
pub fn check_page(docs: &std::path::Path) -> Vec<Finding> {
    let read = |f: &str| std::fs::read_to_string(docs.join(f)).unwrap_or_default();
    let files = [
        ("globe.js", read("globe.js")),
        ("tiling.js", read("tiling.js")),
        ("evidence/evidence.js", read("evidence/evidence.js")),
        ("globe.html", read("globe.html")),
        ("index.html", read("index.html")),
    ];
    let mut out = Vec::new();
    for (f, s) in &files {
        if s.is_empty() {
            out.push(Finding { file: (*f).into(), line: 0, what: "file missing or empty".into() });
        }
    }
    let globe = &files[0].1;
    // 1. ids resolve, and every entry is shown
    let mut seen = std::collections::BTreeSet::new();
    for (f, s) in &files {
        for (id, at) in claim_refs(s) {
            if crate::page_ledger::lookup(&id).is_none() {
                out.push(Finding { file: (*f).into(), line: line_of(s, at), what: format!("data-claim '{}' has no ledger entry", id) });
            }
            seen.insert(id);
        }
    }
    for c in PAGE_CLAIMS {
        if !seen.contains(c.id) && !c.id.starts_with("layer-") {
            out.push(Finding { file: "globe.js".into(), line: 0, what: format!("ledger entry '{}' is not shown anywhere on the page", c.id) });
        }
    }
    // 2. written elements carry a claim
    for (f, s) in files.iter().take(3) {
        for (id, at) in written_ids(s) {
            if UI_IDS.iter().any(|(u, _)| *u == id) {
                continue;
            }
            match opening_tag(globe, &id) {
                Some(tag) if tag.contains("data-claim=") || tag.contains("data-evidence=") => {}
                Some(_) => out.push(Finding { file: (*f).into(), line: line_of(s, at), what: format!("element #{} is written as text but its tag has no data-claim / data-evidence", id) }),
                None => out.push(Finding { file: (*f).into(), line: line_of(s, at), what: format!("element #{} is written as text but its markup was not found", id) }),
            }
        }
    }
    // 3. scanned regions
    if !globe.contains("// evidence:scan") {
        out.push(Finding { file: "globe.js".into(), line: 0, what: "no '// evidence:scan' region: the page's text is not checked".into() });
    }
    out.extend(scan_figures(globe, "globe.js"));
    out.extend(scan_figures(&files[2].1, "evidence/evidence.js"));
    // 4. globe layers
    let mut layers = Vec::new();
    if let Ok(rd) = std::fs::read_dir(docs.join("layers")) {
        let mut names: Vec<_> = rd.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().into_owned()).filter(|n| n.ends_with(".js")).collect();
        names.sort();
        for n in names {
            let f = format!("layers/{}", n);
            layers.push((f.clone(), read(&f)));
        }
    }
    out.extend(check_layers(&layers));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page_ledger::docs_dir;

    fn figs(js: &str) -> Vec<String> {
        scan_figures(js, "t.js").into_iter().map(|f| f.what).collect()
    }

    #[test]
    fn page_scan_flags_a_bare_figure_and_passes_a_wrapped_one() {
        let bad = "// evidence:scan\nconst a = `<p>server 7 KB/s</p>`;\n// evidence:end\n";
        assert_eq!(figs(bad).len(), 1, "{:?}", figs(bad));
        let good = "// evidence:scan\nconst a = `<p>server <span class=\"fig\" data-claim=\"budget-class-rates\">7 KB/s <b>x</b></span> ok</p>`;\n// evidence:end\n";
        assert!(figs(good).is_empty(), "{:?}", figs(good));
    }

    #[test]
    fn page_scan_outside_a_region_is_not_checked() {
        assert!(figs("const a = `<p>7 KB/s</p>`;").is_empty());
    }

    #[test]
    fn page_scan_interpolations_must_be_wrapped() {
        let bad = "// evidence:scan\nx.innerHTML = `Bob lives ${Math.round(c.span)} m away`;\n// evidence:end\n";
        assert_eq!(figs(bad).len(), 1, "{:?}", figs(bad));
        let good = "// evidence:scan\nx.innerHTML = `Bob lives ${fig(\"micro-generated\", Math.round(c.span) + \" m\")} away, ${MK[kind]}`;\n// evidence:end\n";
        assert!(figs(good).is_empty(), "{:?}", figs(good));
        let attr = "// evidence:scan\nx.innerHTML = `<button title=\"${n} things\">go</button>`;\n// evidence:end\n";
        assert!(figs(attr).is_empty(), "attribute text is not visible text: {:?}", figs(attr));
    }

    #[test]
    fn page_scan_citations_are_not_figures() {
        for t in ["5.4.3", "4.5.13", "§4.2", "#939", "leviculum#100", "v45", "v45.0.0", "R5", "T14", "c580298", "1:1", "1-hop"] {
            assert!(is_citation_or_name(t), "{t}");
        }
        for t in ["7", "2.25", "1.16", "33,000", "R55", "1,349", "abc1234X"] {
            assert!(!is_citation_or_name(t), "{t}");
        }
    }

    #[test]
    fn page_scan_written_elements_need_a_claim() {
        let src = "<b id=\"a\" data-claim=\"x\">–</b><b id=\"b\">–</b>\n$(\"a\").textContent = 1; $(\"b\").textContent = 2; $(\"b\").style.width = 3;";
        let w = written_ids(src);
        assert_eq!(w.iter().map(|(i, _)| i.as_str()).collect::<Vec<_>>(), ["a", "b"]);
        assert!(opening_tag(src, "a").unwrap().contains("data-claim"));
        assert!(!opening_tag(src, "b").unwrap().contains("data-claim"));
    }

    #[test]
    fn page_scan_layers_need_an_entry_of_their_class() {
        let src = "window.CEWP_LAYERS[\"cohort-load\"] = { id: \"cohort-load\", title: \"Load\", cls: \"MODEL\", last: {} };\nif (CEWP_LAYERS[\"x\"] === 1) {}\nwindow.CEWP_LAYERS[k] = { id: k, cls: \"MODEL\" };";
        let r = layer_regs(src);
        assert_eq!(r.len(), 2, "{:?}", r);
        assert_eq!((r[0].0.as_str(), r[0].1.as_str(), r[0].2.as_str()), ("cohort-load", "cohort-load", "MODEL"));
        assert!(r[1].0.is_empty() && r[1].1.is_empty());
        let found: Vec<String> = check_layers(&[("layers/t.js".into(), src.into())]).into_iter().map(|f| f.what).collect();
        assert!(found.iter().any(|w| w.contains("no ledger entry 'layer-cohort-load'")), "{:?}", found);
        assert!(found.iter().any(|w| w.contains("string literals")), "{:?}", found);
    }

    #[test]
    fn page_every_figure_has_a_ledger_entry() {
        let found = check_page(&docs_dir());
        assert!(found.is_empty(), "{} page figure(s) without a ledger entry:\n{}", found.len(), found.iter().map(|f| f.to_string()).collect::<Vec<_>>().join("\n"));
    }
}
