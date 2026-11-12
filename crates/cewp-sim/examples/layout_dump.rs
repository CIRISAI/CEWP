//! Native layout digests for a context file (one leaf per line: set, packed id, 33 context numbers,
//! anything after is ignored): prints "id digest_lo digest_hi". With `--roads <id>`, that leaf's roads.
use cewp_sim::micro::MicroParams;
use cewp_sim::tiling::{HolonId, LeafContext, LeafLayout};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let text = std::fs::read_to_string(&a[1]).expect("file");
    let want: Option<u64> = a.iter().position(|x| x == "--roads").map(|i| a[i + 1].parse().unwrap());
    for line in text.lines() {
        let v: Vec<f64> = line.split_whitespace().map(|x| x.parse().unwrap()).collect();
        let id = v[1] as u64;
        if want.is_some_and(|w| w != id) { continue; }
        let l = LeafLayout::build(HolonId::unpack(id).unwrap(), &LeafContext::from_f64(&v[2..35]), &MicroParams::default());
        let d = l.digest();
        println!("{id} {} {}", d & 0xFFFF_FFFF, d >> 32);
        if want.is_some() { for r in &l.roads { println!("road {:?}", [r.x0, r.y0, r.x1, r.y1, r.width]); } println!("w {} h {} density {}", l.w, l.h, l.density_km2); }
    }
}
