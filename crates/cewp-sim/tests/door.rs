//! The runtime table door (crates/cewp-sim/src/door.rs) end to end, on SYNTHETIC tables only: a made-up climate grid and
//! a made-up built-form table that exist in this file and nowhere else (nothing here is shipped or describes a place).
use cewp_sim::builtform::{FormClass, FormSource};
use cewp_sim::door::{sha256, Door, Header, Slot, GRID};
use cewp_sim::micro::MicroParams;
use cewp_sim::tiling::{HolonId, LeafContext, LeafLayout};

fn leaves() -> Vec<(HolonId, LeafContext)> {
    include_str!("form_identity.txt")
        .lines()
        .filter(|l| !l.starts_with('#'))
        .step_by(2) // half the identity set: every form class, at debug-build speed
        .map(|line| {
            let v: Vec<f64> = line.split_whitespace().map(|x| x.parse().unwrap()).collect();
            (HolonId::unpack(v[0] as u64).unwrap(), LeafContext::from_f64(&v[1..34]))
        })
        .collect()
}

/// A synthetic climate grid: class 15 in every other 10-degree band of longitude, class 8 between, none above 80 N.
fn synthetic_climate() -> Vec<u8> {
    let mut g = vec![0u8; GRID.0 * GRID.1];
    for r in 80..GRID.1 {
        for c in 0..GRID.0 {
            g[r * GRID.0 + c] = if (c / 80) % 2 == 0 { 15 } else { 8 };
        }
    }
    g
}

const SYNTHETIC_FORM: &str = "# SYNTHETIC, for this test only: suburban streets in climate class 15 get three or four storeys\nrow suburban 15 * storeys=3,2\n";

fn header(name: &str, version: u32, bytes: &[u8]) -> Header {
    Header { name: name.into(), reference: "SYNTHETIC".into(), version, sha256: sha256(bytes) }
}

fn lay(door: &Door, (id, ctx): &(HolonId, LeafContext)) -> LeafLayout {
    LeafLayout::build_with(*id, ctx, &MicroParams::default(), door)
}

#[test]
fn a_synthetic_table_changes_one_class_in_one_zone_and_nothing_else() {
    let grid = synthetic_climate();
    let mut door = Door::default();
    assert_eq!(door.load(header("climate", 1, &grid), &grid), Ok(Slot::Climate));
    assert_eq!(door.load(header("form", 1, SYNTHETIC_FORM.as_bytes()), SYNTHETIC_FORM.as_bytes()), Ok(Slot::Form));
    let (mut changed, mut kept_in_zone) = (0, 0);
    for leaf in leaves() {
        let before = lay(&Door::default(), &leaf);
        let after = lay(&door, &leaf);
        let in_zone = FormClass::of(before.class, before.density_km2) == FormClass::Suburban && door.climate_of(leaf.0) == 15;
        if in_zone {
            assert_eq!(after.form.source, FormSource::Zone);
            assert_eq!((after.roads.clone(), after.ports.clone()), (before.roads.clone(), before.ports.clone()), "only the buildings' storeys may move");
            // every home has 3 storeys or more, but for the one block a leaf places when no frontage holds a home
            assert!(after.buildings.iter().filter(|b| b.units > 0 && b.floors < 3).count() <= 1, "homes have 3 or 4 storeys");
            if before.digest() != after.digest() {
                changed += 1;
            } else if before.buildings.iter().filter(|b| b.units > 0).count() > 1 {
                kept_in_zone += 1; // homes on the street that kept their storeys
            }
        } else {
            assert_eq!(after.digest(), before.digest(), "a leaf outside the class and zone is unchanged");
            assert_eq!(after.form.source, FormSource::BuiltIn);
        }
    }
    assert!(changed >= 10 && kept_in_zone == 0, "{changed} suburban leaves in the zone changed, {kept_in_zone} with homes on the street did not");
}

#[test]
fn the_same_tables_give_the_same_world_whatever_order_they_arrive_in() {
    let grid = synthetic_climate();
    let form = SYNTHETIC_FORM.as_bytes();
    let all = leaves();
    // a page's cache: layouts kept while every slot they read is at the version they read
    let mut door = Door::default();
    let mut cache: Vec<LeafLayout> = all.iter().map(|l| lay(&door, l)).collect();
    let refresh = |door: &Door, cache: &mut Vec<LeafLayout>| -> usize {
        let mut n = 0;
        for (k, l) in all.iter().enumerate() {
            if !cache[k].form.stamp.current(door) {
                cache[k] = lay(door, l);
                n += 1;
            }
        }
        n
    };
    // the climate arrives first: no layout read it (no form table keys on it), so nothing is rebuilt
    door.load(header("climate", 1, &grid), &grid).unwrap();
    assert_eq!(refresh(&door, &mut cache), 0, "loading the climate invalidates nothing that did not read it");
    // the form table arrives: every layout read the (absent) form slot
    door.load(header("form", 1, form), form).unwrap();
    assert_eq!(refresh(&door, &mut cache), all.len());
    // the other order, from scratch, and a fresh door with both: the same world, layout for layout
    let mut other = Door::default();
    other.load(header("form", 1, form), form).unwrap();
    let mut cache2: Vec<LeafLayout> = all.iter().map(|l| lay(&other, l)).collect();
    assert!(cache2.iter().all(|l| l.form.stamp.read == 0b11 && l.form.stamp.versions == [0, 1, 0, 0]), "a form table keyed on climate reads the climate slot, absent");
    other.load(header("climate", 1, &grid), &grid).unwrap();
    for (k, l) in all.iter().enumerate() {
        if !cache2[k].form.stamp.current(&other) {
            cache2[k] = lay(&other, l);
        }
    }
    let fresh: Vec<LeafLayout> = all.iter().map(|l| lay(&door, l)).collect();
    assert_eq!(cache, fresh);
    assert_eq!(cache2, fresh);
    // a new version of the form table invalidates every layout again; unloading returns the built-in world
    let v2 = b"row suburban 15 * storeys=3,2 # SYNTHETIC\nrow village * * barn=none";
    door.load(header("form", 2, v2), v2).unwrap();
    assert!(cache.iter().all(|l| !l.form.stamp.current(&door)));
    door.unload(Slot::Form);
    let back: Vec<u64> = all.iter().map(|l| lay(&door, l).digest()).collect();
    assert_eq!(back, all.iter().map(|l| lay(&Door::default(), l).digest()).collect::<Vec<_>>());
}

#[test]
fn a_generator_draw_replaces_the_rows_and_moves_no_other_draw() {
    use cewp_sim::builtform::{Absent, BuildingDraw, FormGenerator, FormQuery};
    // SYNTHETIC: a generator that answers for detached houses only, each a 10 by 9 m house of 2 storeys
    struct Fixed;
    impl FormGenerator for Fixed {
        fn present(&self) -> bool {
            true
        }
        fn draw(&self, q: &FormQuery) -> Result<BuildingDraw, Absent> {
            if q.class != FormClass::Suburban {
                return Err(Absent("outside this test's envelope"));
            }
            Ok(BuildingDraw { width: 10.0, depth: 9.0, storeys: 2, units: 1, gap: 12.0, setback: 9.0, pooling: Default::default() })
        }
    }
    static FIXED: Fixed = Fixed;
    let door = Door::with_generator(&FIXED);
    let (mut asked, mut drew, mut same) = (0, 0, 0);
    for leaf in leaves() {
        let before = lay(&Door::default(), &leaf);
        let after = lay(&door, &leaf);
        if FormClass::of(before.class, before.density_km2) == FormClass::Suburban {
            asked += 1;
            drew += usize::from(after.form.generated > 0);
            assert!(after.buildings.iter().filter(|b| b.units > 0 && b.floors < 2).count() <= 1, "the generator's houses have 2 storeys");
        } else {
            assert_eq!(after.digest(), before.digest(), "an absent answer leaves the row's draw");
            same += 1;
        }
    }
    assert!(asked > 10 && drew > 10 && same > 100, "{asked} {drew} {same}");
}
