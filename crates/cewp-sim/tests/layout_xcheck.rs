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

/// The built-form table's identity set (tools/holon/form_identity.mjs): synthetic leaves at every form class, their
/// digests written by the kernel before the per-class numbers moved into `builtform::BUILT_IN`. With no table loaded
/// every layout is the same, number for number, and `build` is `build_with` an empty door.
#[test]
fn with_no_table_loaded_every_form_class_lays_out_as_before() {
    use cewp_sim::builtform::{FormClass, FormSource};
    let text = include_str!("form_identity.txt");
    let (mut n, mut bad, mut classes) = (0, Vec::new(), std::collections::BTreeSet::new());
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let v: Vec<f64> = line.split_whitespace().map(|x| x.parse().unwrap()).collect();
        let id = HolonId::unpack(v[0] as u64).unwrap();
        let ctx = LeafContext::from_f64(&v[1..34]);
        let l = LeafLayout::build(id, &ctx, &MicroParams::default());
        let d = l.digest();
        if (d & 0xFFFF_FFFF) as f64 != v[34] || (d >> 32) as f64 != v[35] {
            bad.push(v[0] as u64);
        }
        assert_eq!(l, LeafLayout::build_with(id, &ctx, &MicroParams::default(), &cewp_sim::door::Door::default()));
        assert_eq!((l.form.source, l.form.stamp.read, l.form.stamp.versions), (FormSource::BuiltIn, 0b10, [0, 0, 0, 0]), "read the form slot, absent");
        assert!(l.tones.len() == l.buildings.len() && l.tones.iter().all(|&t| t == 0), "every roof neutral while no generator ships");
        classes.insert(FormClass::of(l.class, l.density_km2));
        n += 1;
    }
    assert_eq!(classes.len(), 5, "every form class: {classes:?}");
    assert!(n == 600 && bad.is_empty(), "{} of {n} leaves differ: {:?}", bad.len(), &bad[..bad.len().min(5)]);
}
