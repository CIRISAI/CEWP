//! Writes (`--write`) or checks the page's claims ledger, `docs/evidence/claims.json` and
//! `docs/evidence/claims.js`, from `cewp_evidence::page_ledger::PAGE_CLAIMS`, then runs the page
//! scan. Exits non-zero when an entry is refused, a file is stale, or the page shows a figure with
//! no entry.
//!
//!     cargo run -p cewp-evidence --example page_ledger -- --write
//!     cargo run -p cewp-evidence --example page_ledger -- --docs <another tree>/docs

use cewp_evidence::page_ledger::{docs_dir, ledger_js, ledger_json, validate_ledger, GENERATE_CMD};
use cewp_evidence::page_scan::check_page;

fn main() {
    let write = std::env::args().any(|a| a == "--write");
    let mut bad = false;
    if let Err(errs) = validate_ledger() {
        for e in errs {
            eprintln!("refused: {e}");
        }
        std::process::exit(1);
    }
    let dir = docs_dir().join("evidence");
    for (name, body) in [("claims.json", ledger_json()), ("claims.js", ledger_js())] {
        let path = dir.join(name);
        if write {
            std::fs::create_dir_all(&dir).expect("create docs/evidence");
            std::fs::write(&path, body).expect("write ledger");
            println!("wrote {}", path.display());
        } else if std::fs::read_to_string(&path).ok().as_deref() != Some(body.as_str()) {
            eprintln!("stale: {} (run `{}`)", path.display(), GENERATE_CMD);
            bad = true;
        }
    }
    // `--docs <dir>` scans another tree (an integration checkout) against this ledger
    let args: Vec<String> = std::env::args().collect();
    let docs = args.iter().position(|a| a == "--docs").and_then(|i| args.get(i + 1)).map(std::path::PathBuf::from).unwrap_or_else(docs_dir);
    let found = check_page(&docs);
    for f in &found {
        eprintln!("{f}");
    }
    if !found.is_empty() {
        eprintln!("{} page figure(s) without a ledger entry", found.len());
        bad = true;
    }
    if bad {
        std::process::exit(1);
    }
    println!("ledger ok: every figure on the page has an entry");
}
