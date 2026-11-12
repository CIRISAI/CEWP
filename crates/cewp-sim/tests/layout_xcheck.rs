//! Native half of the layout crosscheck: every leaf in docs/geo/layout_xcheck.txt lays out to the
//! stored digest (docs/geo/layout_xcheck.test.js checks the shipped wasm against the same digests,
//! so a certificate scored natively describes what the page draws).
use cewp_sim::micro::MicroParams;
use cewp_sim::tiling::{HolonId, LeafContext, LeafLayout};

#[test]
fn native_layouts_match_the_stored_digests() {
    let text = include_str!("../../../docs/geo/layout_xcheck.txt");
    let (mut n, mut bad) = (0, Vec::new());
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let v: Vec<f64> = line.split_whitespace().map(|x| x.parse().unwrap()).collect();
        let id = v[0] as u64;
        let l = LeafLayout::build(HolonId::unpack(id).unwrap(), &LeafContext::from_f64(&v[1..34]), &MicroParams::default());
        let d = l.digest();
        if (d & 0xFFFF_FFFF) as f64 != v[34] || (d >> 32) as f64 != v[35] {
            bad.push(id);
        }
        n += 1;
    }
    assert!(n >= 2000 && bad.is_empty(), "{} of {n} leaves differ: {:?}", bad.len(), &bad[..bad.len().min(5)]);
}
