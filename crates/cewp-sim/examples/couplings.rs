//! Writes the couplings registry: `docs/geo/couplings.json` and the generated tables in
//! `FSD/twin/couplings.md` (between the markers). Without `--write` it prints the JSON.
use cewp_sim::couplings::{to_json, to_markdown_tables, BEGIN, END};

fn main() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    if !std::env::args().any(|a| a == "--write") {
        print!("{}", to_json());
        return;
    }
    std::fs::write(format!("{root}/docs/geo/couplings.json"), to_json()).expect("write couplings.json");
    let path = format!("{root}/FSD/twin/couplings.md");
    let md = std::fs::read_to_string(&path).expect("FSD/twin/couplings.md");
    let (a, b) = (md.find(BEGIN).expect("begin marker"), md.find(END).expect("end marker"));
    let out = format!("{}{BEGIN}\n{}{}", &md[..a], to_markdown_tables(), &md[b..]);
    std::fs::write(&path, out).expect("write couplings.md");
    println!("wrote docs/geo/couplings.json and FSD/twin/couplings.md");
}
