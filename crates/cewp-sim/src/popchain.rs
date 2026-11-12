//! The population chain of the street tier: one population, one split, in Rust, read by the page
//! through kernel-wasm (`pop_load`, `pop_root`, `pop_split`) and by native tests through a fixture.
//!
//! Input: GHS-POP R2023A 2020 people per 1/8-degree cell (`docs/geo/gd/pop1`, conserved to the
//! 1/2-degree totals by `docs/geo/geodata.js`), 2,880 x 1,440 cells, row 0 at the north. An
//! external published estimate (pop-grid-source, MODEL).
//!
//! Integers from the first step, so every split sums exactly:
//! 1. Each cell's people is rounded to an integer, and a cell under [`FLOOR_KM2`] people per km² of
//!    cell area counts zero ([`cell_people`]).
//! 2. The world: the cohort tier's GHS total ([`World::new`]'s `world_people`, the same people the
//!    1.5-degree grid keeps) is apportioned over the 28,800 level-0 holons (1.5 degrees, each exactly
//!    12 x 12 cells) by their floored cell sums, so the street tier carries the same total and drops
//!    nobody twice; the world's households are `round(world people / 2.5)`, apportioned over the
//!    level-0 holons by their people.
//! 3. A split: a holon's people go to its children by the people each child's rectangle overlaps
//!    (milli-persons, each cell's people x the overlapped share of the cell, uniform within a cell);
//!    its households go to its children by the people each child was just given (households follow
//!    people). Both are [`apportion`]: largest remainder, ties to the lower index, a zero weight gets
//!    zero unless all are zero.
//!
//! So at every holon, for every kind, the children sum to the parent with zero tolerance, and a
//! holon's households are its people / 2.5 within the rounding of the steps above it.

use crate::tiling::{apportion, HolonId, STEP, UNITS_PER_DEG as UPD};

/// Cells per row and rows of the level-1 GHS-POP grid (1/8 degree).
pub const NX: usize = 2_880;
/// Rows.
pub const NY: usize = 1_440;
/// Degrees per cell.
pub const CELL_DEG: f64 = 0.125;
/// Below this many people per km² of cell area a cell contributes nobody (MODEL).
pub const FLOOR_KM2: f64 = 1.0;
/// Persons per household for the world's household total (MODEL; micro.rs PersonsPerHousehold).
pub const PPH_NUM: u64 = 5;
/// Persons per household, denominator (2.5 = 5 / 2).
pub const PPH_DEN: u64 = 2;
/// Level-0 holons per row (1.5 degrees: 240) and rows (120).
pub const L0_NX: usize = 240;
/// Rows of level-0 holons.
pub const L0_NY: usize = 120;

/// km² of a cell in row `r` (sphere of radius 6,371.0088 km, as geodata.js).
#[must_use]
pub fn cell_km2(r: usize) -> f64 {
    let (re, d) = (6_371.008_8_f64, std::f64::consts::PI / 180.0);
    let n = 90.0 - r as f64 * CELL_DEG;
    let s = 90.0 - (r + 1) as f64 * CELL_DEG;
    re * re * CELL_DEG * d * ((n * d).sin() - (s * d).sin())
}

/// A cell's people as the chain counts them: rounded, and zero under the floor.
#[must_use]
pub fn cell_people(p: f32, r: usize) -> u32 {
    let p = f64::from(p);
    if p.is_nan() || p <= 0.0 || p / cell_km2(r) < FLOOR_KM2 {
        return 0;
    }
    p.round().min(f64::from(u32::MAX)) as u32
}

/// Where the cell people come from: the full grid on the page, a fixture in tests.
pub trait Cells {
    /// Integer people of cell (row, column); column wraps.
    fn people(&self, r: usize, c: usize) -> u32;
}

/// The full grid, people already integer ([`cell_people`]).
pub struct Grid {
    /// Row-major, `NX * NY`.
    pub cells: Vec<u32>,
}

impl Grid {
    /// From the level-1 floats, row-major.
    #[must_use]
    pub fn from_f32(p1: &[f32]) -> Self {
        let mut cells = vec![0u32; NX * NY];
        for r in 0..NY.min(p1.len() / NX) {
            for c in 0..NX {
                cells[r * NX + c] = cell_people(p1[r * NX + c], r);
            }
        }
        Self { cells }
    }
}

impl Cells for Grid {
    fn people(&self, r: usize, c: usize) -> u32 {
        self.cells[r * NX + c % NX]
    }
}

/// Integer degrees -> (row, column) range of the cells a holon covers, and the overlap weights.
/// A holon's rectangle in holon units (UNITS_PER_DEG a degree): (y0, x0, y1, x1).
fn overlap_milli<C: Cells>(cells: &C, h: HolonId) -> u64 {
    let (y0, x0, y1, x1) = h.bounds_units();
    let upd = UPD as f64;
    let (la0, lo0, la1, lo1) = (y0 as f64 / upd, x0 as f64 / upd, y1 as f64 / upd, x1 as f64 / upd);
    let r0 = ((90.0 - la1) / CELL_DEG).floor().max(0.0) as usize;
    let r1 = (((90.0 - la0) / CELL_DEG).ceil() as usize).min(NY).saturating_sub(1);
    let c0 = ((lo0 + 180.0) / CELL_DEG).floor() as i64;
    let c1 = ((lo1 + 180.0) / CELL_DEG).ceil() as i64 - 1;
    let mut sum = 0u64;
    for r in r0..=r1 {
        let top = 90.0 - r as f64 * CELL_DEG;
        let fy = (top.min(la1) - (top - CELL_DEG).max(la0)) / CELL_DEG;
        if fy <= 0.0 {
            continue;
        }
        for c in c0..=c1 {
            let left = -180.0 + c as f64 * CELL_DEG;
            let fx = ((left + CELL_DEG).min(lo1) - left.max(lo0)) / CELL_DEG;
            if fx <= 0.0 {
                continue;
            }
            let p = cells.people(r, c.rem_euclid(NX as i64) as usize);
            if p > 0 {
                sum += (f64::from(p) * 1000.0 * fx * fy).round() as u64;
            }
        }
    }
    sum
}

/// The people and households a holon holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Pop {
    /// People.
    pub people: u64,
    /// Households.
    pub households: u64,
}

/// The world's level-0 holons, apportioned from one total.
pub struct World {
    /// People per level-0 holon, row-major from the north (row = 89.999 down), [`L0_NX`] columns.
    pub l0: Vec<Pop>,
    /// The world's people (as given) and households.
    pub total: Pop,
    /// The floored, rounded cell people summed over the world, before the apportionment.
    pub cells_total: u64,
    /// Cells above the floor.
    pub cells_peopled: u64,
}

fn l0_index(h: HolonId) -> Option<usize> {
    if h.level != 0 {
        return None;
    }
    let (y0, x0, _, _) = h.bounds_units();
    let row = ((90 * UPD - y0) / STEP[0]) as usize - 1;
    let col = ((x0 + 180 * UPD) / STEP[0]) as usize;
    (row < L0_NY && col < L0_NX).then_some(row * L0_NX + col)
}

impl World {
    /// Apportion `world_people` (the cohort tier's GHS total; 0 = the floored cell total itself) over
    /// the level-0 holons by their floored cell sums, and the world's households over them by people.
    #[must_use]
    pub fn new<C: Cells>(cells: &C, rows: std::ops::Range<usize>, world_people: u64) -> Self {
        let mut w = vec![0u64; L0_NX * L0_NY];
        let (mut total, mut peopled) = (0u64, 0u64);
        for r in rows {
            for c in 0..NX {
                let p = u64::from(cells.people(r, c));
                if p > 0 {
                    w[(r / 12) * L0_NX + c / 12] += p;
                    total += p;
                    peopled += 1;
                }
            }
        }
        let people = if world_people > 0 { world_people } else { total };
        let households = (people * PPH_DEN * 2 + PPH_NUM) / (PPH_NUM * 2); // round(people / 2.5)
        let pp = apportion(people, &w);
        let hh = apportion(households, &pp);
        let l0 = pp.iter().zip(&hh).map(|(&people, &households)| Pop { people, households }).collect();
        Self { l0, total: Pop { people, households }, cells_total: total, cells_peopled: peopled }
    }

    /// A level-0 holon's population.
    #[must_use]
    pub fn root(&self, h: HolonId) -> Pop {
        l0_index(h).map_or(Pop::default(), |i| self.l0[i])
    }
}

/// The children of `parent` holding `pop`: people by the people their rectangles overlap, households
/// by the people they were given. In `parent.children()` order.
#[must_use]
pub fn split<C: Cells>(cells: &C, parent: HolonId, pop: Pop) -> Vec<(HolonId, Pop)> {
    let kids = parent.children();
    let w: Vec<u64> = kids.iter().map(|&k| overlap_milli(cells, k)).collect();
    let pp = apportion(pop.people, &w);
    let hh = apportion(pop.households, &pp);
    kids.into_iter().zip(pp.into_iter().zip(hh)).map(|(k, (people, households))| (k, Pop { people, households })).collect()
}

/// A holon's population, descending from its level-0 ancestor (generating only the path).
#[must_use]
pub fn pop_of<C: Cells>(cells: &C, world: &World, h: HolonId) -> Pop {
    let mut path = vec![h];
    while let Some(p) = path.last().and_then(|x| x.parent()) {
        path.push(p);
    }
    path.reverse();
    let mut pop = world.root(path[0]);
    for w in path.windows(2) {
        let kids = split(cells, w[0], pop);
        pop = kids.iter().find(|(k, _)| *k == w[1]).map_or(Pop::default(), |x| x.1);
    }
    pop
}

/// Persons per household in a leaf: `people` apportioned over its households with the drawn
/// household sizes as weights, so the leaf's persons are exactly its people.
#[must_use]
pub fn persons_per_household(people: u64, drawn_sizes: &[u64]) -> Vec<u64> {
    apportion(people, drawn_sizes)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A synthetic grid: a city of 30,000 per km² at one spot, a village cell, empty cells around.
    struct Synthetic;
    impl Cells for Synthetic {
        fn people(&self, r: usize, c: usize) -> u32 {
            let (cr, cc) = (339, 1508); // about 47.6 N, 8.5 E
            let d = (r as i64 - cr).abs().max((c as i64 - cc).abs());
            match d {
                0 => cell_people(5_000_000.0, r),
                1 => cell_people(400_000.0, r),
                2 => cell_people(20_000.0, r),
                3 => cell_people(50.0, r), // under the floor: about 0.4 per km2
                _ => 0,
            }
        }
    }

    #[test]
    fn popchain_every_split_sums_exactly_down_to_the_street() {
        let w = World::new(&Synthetic, 330..350, 7_000_000);
        assert_eq!(w.l0.iter().map(|p| p.people).sum::<u64>(), w.total.people);
        assert_eq!(w.l0.iter().map(|p| p.households).sum::<u64>(), w.total.households);
        assert_eq!(w.total.households, 2_800_000);
        let mut h = HolonId::at(0, 47.6, 8.5);
        let mut pop = w.root(h);
        assert!(pop.people > 0);
        for _ in 0..9 {
            let kids = split(&Synthetic, h, pop);
            assert_eq!(kids.iter().map(|k| k.1.people).sum::<u64>(), pop.people, "{h:?} people");
            assert_eq!(kids.iter().map(|k| k.1.households).sum::<u64>(), pop.households, "{h:?} households");
            let (k, p) = *kids.iter().max_by_key(|k| (k.1.people, std::cmp::Reverse(k.0.pack()))).unwrap();
            assert_eq!(pop_of(&Synthetic, &w, k), p, "random access lands on the same holon");
            h = k;
            pop = p;
        }
    }

    #[test]
    fn popchain_cells_under_the_floor_hold_nobody() {
        assert_eq!(cell_people(50.0, 339), 0);
        let w = World::new(&Synthetic, 330..350, 0);
        // the ring of floor cells adds nobody to the world
        assert_eq!(w.cells_total, 5_000_000 + 8 * 400_000 + 16 * 20_000);
    }

    #[test]
    fn popchain_persons_follow_people_exactly() {
        let sizes = [3u64, 1, 6, 2, 2];
        let p = persons_per_household(17, &sizes);
        assert_eq!(p.iter().sum::<u64>(), 17);
        assert!(p.iter().zip(&sizes).all(|(&a, &s)| (a as f64 - 17.0 * s as f64 / 14.0).abs() <= 1.0));
    }
}
