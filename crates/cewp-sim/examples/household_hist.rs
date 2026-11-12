//! Household-size histogram from the twin's own code path, for CIRISHolon's HOUSEHOLD-0 campaign:
//! `docs/geo/household_hist.json`. Without `--write` it prints the JSON; `--check` regenerates and
//! compares with the committed file (exit 1 on a difference). `tests/household_hist.rs` runs the
//! same comparison, so the file cannot go stale silently.
//!
//! Nothing is re-implemented: every household comes out of `MicroWorld::from_leaf`, the call the
//! page makes through `kernel-wasm` `micro_build_leaf`, and its size is the `Household::persons`
//! that `micro_population` exports. The drawn sizes (before the leaf's people are apportioned over
//! them) come out of the same call on the same leaf with its people set to 0: `populate` then keeps
//! the drawn sizes as they are (`target_people == 0`), with the same seed and the same per-dwelling
//! streams. The leaf layout is synthetic (one building holding every dwelling, no streets): a
//! household's size depends on the leaf's seed, the dwelling's index, the leaf's people and its
//! dwelling count, never on the layout, the density class or the place (see FSD/twin/hhsize.md).
//!
//! `--crosscheck F` rebuilds, natively from this crate's source, every real leaf the page's kernel
//! built in `node docs/geo/household_hist_real.js --sample F` (one line per leaf: packed id, the
//! 33-slot context, the household count and each household's persons, the building count and each
//! dwelling building's units) with `LeafLayout::build` and `MicroWorld::from_leaf`, and compares:
//! every household's persons must agree leaf by leaf (exit 1 otherwise); leaves whose building
//! split differs are listed and counted beside it.
#![allow(dead_code)]
use cewp_sim::micro::{Building, DensityClass, MicroParams, MicroWorld, NO_STREET};
use cewp_sim::tiling::{HolonId, LeafContext, LeafLayout};

/// Leaves generated.
pub const LEAVES: i32 = 16_667;
/// Households per leaf.
pub const LEAF_HOUSEHOLDS: u16 = 60;
/// People per leaf.
pub const LEAF_PEOPLE: u64 = 150;
/// Level of the synthetic leaf ids (0.0025 degrees, about 280 m at the equator).
pub const LEVEL: u8 = 6;
/// The library code the committed file was generated with (the branch base; no library change).
pub const LIBRARY_COMMIT: &str = "943ab2d8337b5dc07cbf9c5961837f4386e466d5";
/// The command that writes the committed file.
pub const COMMAND: &str = "cargo run --release -p cewp-sim --example household_hist -- --write";
/// The committed file, relative to the repository root.
pub const OUT: &str = "docs/geo/household_hist.json";

/// Leaf `k`: id `HolonId { level: 6, i: 0, j: k }`, one building of 60 dwellings, `people` people.
fn leaf(k: i32, people: u64) -> LeafLayout {
    let id = HolonId { level: LEVEL, i: 0, j: k };
    let b = Building { cx: 0.0, cy: 0.0, w: 40.0, d: 20.0, angle: 0.0, floors: 6, units: LEAF_HOUSEHOLDS, street: NO_STREET };
    let density_km2 = 1_000.0;
    LeafLayout {
        id,
        w: 280.0,
        h: 280.0,
        households: u64::from(LEAF_HOUSEHOLDS),
        people,
        density_km2,
        class: DensityClass::of(density_km2, &MicroParams::default()),
        roads: Vec::new(),
        buildings: vec![b],
        ports: Vec::new(),
        unplaced: false,
    }
}

/// The persons of each household of leaf `k`, as the page's focal world holds them.
fn persons(k: i32, people: u64) -> Vec<u64> {
    let w = MicroWorld::from_leaf(&leaf(k, people), MicroParams::default());
    assert_eq!(w.households.len(), usize::from(LEAF_HOUSEHOLDS), "every dwelling is a household");
    w.households.iter().map(|h| u64::from(h.persons)).collect()
}

fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 { a } else { gcd(b, a % b) }
}

fn add(hist: &mut Vec<u64>, v: u64) {
    let v = v as usize;
    if hist.len() <= v {
        hist.resize(v + 1, 0);
    }
    hist[v] += 1;
}

fn hist_json(name: &str, h: &[u64]) -> String {
    let n: u64 = h.iter().sum();
    let p: u64 = h.iter().enumerate().map(|(s, &c)| s as u64 * c).sum();
    let g = gcd(p, n);
    let counts: Vec<String> = h.iter().map(u64::to_string).collect();
    format!(
        "  \"{name}\": {{\n    \"counts_by_size\": [{}],\n    \"households\": {n},\n    \"persons\": {p},\n    \"mean\": {{ \"persons\": {p}, \"households\": {n}, \"reduced\": \"{}/{}\", \"decimal\": \"{:.6}\" }},\n    \"max_size\": {},\n    \"size_0_households\": {},\n    \"over_6_households\": {}\n  }}",
        counts.join(", "),
        p / g,
        n / g,
        p as f64 / n as f64,
        h.iter().rposition(|&c| c > 0).unwrap_or(0),
        h[0],
        h.iter().skip(7).sum::<u64>(),
    )
}

/// The whole file.
#[must_use]
pub fn render() -> String {
    let (mut fin, mut drawn) = (vec![0u64; 1], vec![0u64; 1]);
    let mut delta: std::collections::BTreeMap<i64, u64> = std::collections::BTreeMap::new();
    for k in 0..LEAVES {
        let f = persons(k, LEAF_PEOPLE);
        let d = persons(k, 0);
        assert_eq!(f.iter().sum::<u64>(), LEAF_PEOPLE, "a leaf's persons are its people");
        for (&a, &b) in f.iter().zip(&d) {
            add(&mut fin, a);
            add(&mut drawn, b);
            *delta.entry(a as i64 - b as i64).or_insert(0) += 1;
        }
    }
    let m = fin.len().max(drawn.len());
    fin.resize(m, 0);
    drawn.resize(m, 0);
    // W1 between two distributions on the integers with the same count N: sum over k of
    // |F_a(k) - F_b(k)| with F the cumulative count, over N.
    let n: u64 = fin.iter().sum();
    let (mut ca, mut cb, mut w1) = (0i64, 0i64, 0u64);
    for s in 0..m {
        ca += fin[s] as i64;
        cb += drawn[s] as i64;
        w1 += (ca - cb).unsigned_abs();
    }
    let g = gcd(w1, n).max(1);
    let deltas: Vec<String> = delta.iter().map(|(d, c)| format!("\"{d}\": {c}")).collect();
    let changed: u64 = delta.iter().filter(|(&d, _)| d != 0).map(|(_, &c)| c).sum();
    format!(
        "{{\n  \"what\": \"household sizes (persons per household) of the CEWP twin's focal street leaf, from the twin's own code path (MicroWorld::from_leaf -> populate -> popchain::persons_per_household), MODEL\",\n  \"leaf_shape\": {{ \"leaves\": {LEAVES}, \"households_per_leaf\": {LEAF_HOUSEHOLDS}, \"people_per_leaf\": {LEAF_PEOPLE} }},\n  \"seed_scheme\": \"leaf k = 0..{last} is HolonId {{ level: {LEVEL}, i: 0, j: k }} (a synthetic id sequence); the world's seed is that id's HolonId::seed() as on the page; dwelling hi draws from Rng::new(seed ^ (hi + 1) * 0xA24B_AED4_963E_E407); MicroParams::default() (persons_per_household 2.5, the page never sets it)\",\n  \"conditioning\": \"none: one histogram; a household's size depends only on the leaf seed, its dwelling index, the leaf's people and its dwelling count (not density class, region or building type)\",\n  \"library_commit\": \"{LIBRARY_COMMIT}\",\n  \"command\": \"{COMMAND}\",\n{},\n{},\n  \"apportionment_effect\": {{\n    \"final_minus_drawn_counts\": {{ {} }},\n    \"households_changed\": {changed},\n    \"w1_drawn_final\": {{ \"numerator\": {w1}, \"denominator\": {n}, \"reduced\": \"{}/{}\", \"decimal\": \"{:.6}\" }}\n  }}\n}}\n",
        hist_json("final", &fin),
        hist_json("drawn", &drawn),
        deltas.join(", "),
        w1 / g,
        n / g,
        w1 as f64 / n as f64,
        last = LEAVES - 1,
    )
}

/// Native rebuild of the page's leaves (see the module doc). Returns (leaves, households, leaves
/// whose persons differ, leaves whose building split differs). Only the first fails the check: a
/// household's size never reads the layout; the building split is reported beside it.
fn crosscheck(file: &str) -> (u64, u64, u64, u64) {
    let text = std::fs::read_to_string(file).expect("read the sample");
    let (mut leaves, mut households, mut bad, mut layout) = (0u64, 0u64, 0u64, 0u64);
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let v: Vec<f64> = line.split_whitespace().map(|x| x.parse().expect("a number")).collect();
        let id = HolonId::unpack(v[0] as u64).expect("a packed id");
        let ctx = &v[1..34];
        let n = v[34] as usize;
        let per: Vec<u64> = v[35..35 + n].iter().map(|&x| x as u64).collect();
        let m = v[35 + n] as usize;
        let units: Vec<u64> = v[36 + n..36 + n + m].iter().map(|&x| x as u64).collect();
        let l = LeafLayout::build(id, &LeafContext::from_f64(ctx), &MicroParams::default());
        let w = MicroWorld::from_leaf(&l, MicroParams::default());
        let mine: Vec<u64> = w.households.iter().map(|h| u64::from(h.persons)).collect();
        let my_units: Vec<u64> = l.buildings.iter().filter(|b| b.units > 0).map(|b| u64::from(b.units)).collect();
        if mine != per {
            bad += 1;
            eprintln!("leaf {} (L{}): persons differ: native {mine:?}, kernel {per:?}", v[0], id.level);
        }
        if my_units != units {
            layout += 1;
            eprintln!("leaf {} (L{}): building split differs: native {my_units:?}, kernel {units:?}", v[0], id.level);
        }
        leaves += 1;
        households += n as u64;
    }
    (leaves, households, bad, layout)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Some(k) = args.iter().position(|a| a == "--crosscheck") {
        let (leaves, households, bad, layout) = crosscheck(args.get(k + 1).expect("--crosscheck FILE"));
        println!("crosscheck: {leaves} leaves, {households} households; persons differ on {bad} leaves, the building split on {layout}");
        std::process::exit(i32::from(bad > 0 || leaves == 0));
    }
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/geo/household_hist.json");
    let json = render();
    if std::env::args().any(|a| a == "--write") {
        std::fs::write(path, &json).expect("write household_hist.json");
        println!("wrote {OUT}");
    } else if std::env::args().any(|a| a == "--check") {
        let have = std::fs::read_to_string(path).expect("read household_hist.json");
        if have != json {
            eprintln!("{OUT} is stale: run {COMMAND}");
            std::process::exit(1);
        }
        println!("{OUT} is current");
    } else {
        print!("{json}");
    }
}
