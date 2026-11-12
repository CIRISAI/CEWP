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
    re * re * CELL_DEG * d * (crate::detmath::sin(n * d) - crate::detmath::sin(s * d))
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
    /// The twin's region (0..9, docs/geo/regions.json) of the 1.5-degree cell holding cell (r, c), if
    /// known; the in-cell seam then reads the regional UNEVEN-0 row where one is published.
    fn region(&self, _r: usize, _c: usize) -> Option<u8> {
        None
    }
    /// The in-cell seam: true divides a cell's people over its pixels by CIRISHolon's UNEVEN-0
    /// ([`incell`]); false spreads them evenly (the legacy rule, "no certificate").
    fn incell(&self) -> bool {
        false
    }
    /// Where the cell's town is, if a Natural Earth populated place lies in cell (r, c): its position
    /// in pixel units from the cell's north-west corner (0..15 east, 0..15 south). The in-cell seam
    /// grows the densest pixels around it; without one, toward the most peopled neighbours.
    fn anchor(&self, _r: usize, _c: usize) -> Option<(f64, f64)> {
        None
    }
}

/// The in-cell seam (FSD/twin/holon-intake.md section 2): a 1/8-degree cell's people divided over its
/// 15 x 15 pixels of 30 arc-seconds (about 0.9 km) by CIRISHolon's UNEVEN-0 rule
/// (`holon_scene::world::incell::divide`, grain 1, each cell once), with a table swap: the regional
/// row where the cell's region publishes one, else the world row. Which pixel is the town is not in
/// the table; the twin's display rule (MODEL) gives the densest classes the pixels that score highest:
/// the 1/8-degree people field interpolated at the pixel (bilinear over the cell and its eight
/// neighbours), so towns lean toward dense neighbours and join across cell borders, plus a pull toward
/// the cell's largest Natural Earth populated place if it has one ([`Cells::anchor`]); ties to the lower
/// pixel index. The per-class counts and people are the table's exactly.
pub mod incell {
    use super::{Cells, NX, NY};
    use crate::holon_tables::UNEVEN0_REGIONAL;
    use holon_scene::apportion::apportion;
    use holon_scene::seed::{child_seed, Stream};
    use holon_scene::world::incell::{PEOPLE_PPM, TILES_PPM};

    /// Pixels per cell side (1/8 degree / 30 arc-seconds).
    pub const SIDE: usize = 15;
    /// Pixels per cell.
    pub const N: usize = SIDE * SIDE;
    /// The seed every cell's draw descends from.
    pub const INCELL_SEED: u64 = 0x5EED_0E0E_1CE1_1000;
    /// Lower bounds of the cell classes 1..=5, people per km2 of the cell's whole area (classes.json).
    pub const CLASS_KM2: [f64; 5] = [0.0, 250.0, 1_500.0, 6_000.0, 12_000.0];

    /// Which table a cell read.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Source {
        /// The cell's region publishes shares for its class.
        Regional(u8),
        /// The world row: no region known, or fewer than 30 cells of the class in the region.
        World,
    }

    /// A cell's class 1..=5 from its people per km2 (0 under the floor).
    #[must_use]
    pub fn class_of(people: u32, r: usize) -> u8 {
        let d = f64::from(people) / super::cell_km2(r);
        if people == 0 || d < super::FLOOR_KM2 {
            return 0;
        }
        (CLASS_KM2.iter().rposition(|&b| d >= b).unwrap_or(0) + 1) as u8
    }

    /// The tile classes and people of a cell's tiles by the table, in the generator's order (class 0
    /// first, then each class's tiles): CIRISHolon's divide() without its shuffle, with the table as
    /// a parameter. Returns (class, people) per tile, and how many tiles each class holds.
    #[must_use]
    pub fn divide_table(tiles_ppm: &[u64; 6], people_ppm: &[u64; 6], people: u64, n_tiles: usize, seed: u64) -> Vec<(u8, u64)> {
        let tile_counts = apportion(n_tiles as u64, tiles_ppm);
        let mut class_people = apportion(people, people_ppm);
        for k in 0..6 {
            if class_people[k] > 0 && tile_counts[k] == 0 {
                let to = (0..6)
                    .filter(|&j| tile_counts[j] > 0 && j != 0)
                    .min_by_key(|&j| (j as i64 - k as i64).abs())
                    .or_else(|| (0..6).find(|&j| tile_counts[j] > 0))
                    .expect("some class holds a tile");
                class_people[to] += class_people[k];
                class_people[k] = 0;
            }
        }
        let mut out = Vec::with_capacity(n_tiles);
        for k in 0..6 {
            let n = tile_counts[k] as usize;
            if n == 0 {
                continue;
            }
            let mut w = Stream::new(child_seed(seed, k as u64));
            let weights: Vec<u64> = (0..n).map(|_| 50 + w.below(100)).collect();
            for p in apportion(class_people[k], &weights) {
                out.push((k as u8, p));
            }
        }
        out
    }

    /// The table row a cell reads, and which.
    #[must_use]
    pub fn row(class: u8, region: Option<u8>) -> ([u64; 6], [u64; 6], Source) {
        let k = usize::from(class.clamp(1, 5)) - 1;
        if let Some(rg) = region {
            if let Some((t, p)) = UNEVEN0_REGIONAL.get(usize::from(rg)).and_then(|r| r[k]) {
                return (t, p, Source::Regional(rg));
            }
        }
        (TILES_PPM[k], PEOPLE_PPM[k], Source::World)
    }

    /// The people of each pixel of cell (r, c), row-major from the cell's north-west pixel, summing
    /// exactly to the cell's people; and the pixel's tile class (255 for a cell nobody lives in).
    #[must_use]
    pub fn pixels<C: Cells + ?Sized>(cells: &C, r: usize, c: usize) -> ([u32; N], [u8; N], Source) {
        let p = cells.people(r, c);
        let class = class_of(p, r);
        let mut people = [0u32; N];
        let mut cls = [255u8; N];
        if class == 0 {
            return (people, cls, Source::World);
        }
        let (t, pp, src) = row(class, cells.region(r, c));
        let tiles = divide_table(&t, &pp, u64::from(p), N, child_seed(INCELL_SEED, (r * NX + c) as u64));
        // the ranking (MODEL display rule): the 1/8-degree people field interpolated at each pixel's centre
        // (bilinear over the cell and its eight neighbours' centres), so dense tiles lean toward dense neighbours
        // and towns join across cell borders; plus a pull toward the cell's populated place, if it has one
        let side = SIDE as f64;
        let nb = |dr: i64, dc: i64| -> f64 {
            let rr = r as i64 + dr;
            if rr < 0 || rr >= NY as i64 {
                return 0.0;
            }
            f64::from(cells.people(rr as usize, (c as i64 + dc).rem_euclid(NX as i64) as usize))
        };
        let mut g = [[0.0_f64; 3]; 3];
        for (a, row) in g.iter_mut().enumerate() {
            for (b, v) in row.iter_mut().enumerate() {
                *v = nb(a as i64 - 1, b as i64 - 1);
            }
        }
        let field = |x: f64, y: f64| {
            // x, y in pixel units from the cell's north-west corner; cell centres at 7.5 + 15 k
            let (u, v) = ((x - side / 2.0) / side, (y - side / 2.0) / side); // -0.5..0.5 within the cell
            let (bx, fx) = if u < 0.0 { (0, u + 1.0) } else { (1, u) };
            let (by, fy) = if v < 0.0 { (0, v + 1.0) } else { (1, v) };
            let top = g[by][bx] * (1.0 - fx) + g[by][bx + 1] * fx;
            let bot = g[by + 1][bx] * (1.0 - fx) + g[by + 1][bx + 1] * fx;
            top * (1.0 - fy) + bot * fy
        };
        let anchor = cells.anchor(r, c);
        let peak = g.iter().flatten().fold(0.0_f64, |a, &b| a.max(b)).max(1.0);
        let score = |i: usize| {
            let (x, y) = ((i % SIDE) as f64 + 0.5, (i / SIDE) as f64 + 0.5);
            let mut s = field(x, y) / peak;
            if let Some((ax, ay)) = anchor {
                let d2 = (x - ax) * (x - ax) + (y - ay) * (y - ay);
                s += 1.5 / (1.0 + d2 / 4.0);
            }
            s
        };
        let mut order: Vec<usize> = (0..N).collect();
        order.sort_by(|&a, &b| score(b).total_cmp(&score(a)).then(a.cmp(&b)));
        // densest class first takes the nearest pixels; empty tiles last
        let mut at = 0;
        for k in (0..6u8).rev() {
            for &(cl, pe) in tiles.iter().filter(|t| t.0 == k) {
                let i = order[at];
                people[i] = pe as u32;
                cls[i] = cl;
                at += 1;
            }
        }
        (people, cls, src)
    }
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
            let cc = c.rem_euclid(NX as i64) as usize;
            let p = cells.people(r, cc);
            if p == 0 {
                continue;
            }
            if !cells.incell() || (fx >= 1.0 && fy >= 1.0) {
                sum += (f64::from(p) * 1000.0 * fx * fy).round() as u64;
                continue;
            }
            // a cell the holon only partly covers: its pixels' people, each by the share of the pixel covered
            let (px, _, _) = incell::pixels(cells, r, cc);
            let pd = CELL_DEG / incell::SIDE as f64;
            for (i, &q) in px.iter().enumerate() {
                if q == 0 {
                    continue;
                }
                let (pr, pc) = ((i / incell::SIDE) as f64, (i % incell::SIDE) as f64);
                let ptop = top - pr * pd;
                let pleft = left + pc * pd;
                let gy = (ptop.min(la1) - (ptop - pd).max(la0)) / pd;
                let gx = ((pleft + pd).min(lo1) - pleft.max(lo0)) / pd;
                if gx > 0.0 && gy > 0.0 {
                    sum += (f64::from(q) * 1000.0 * gx * gy).round() as u64;
                }
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

/// The household seam (FSD/twin/holon-intake.md section 5): CIRISHolon's household generator
/// (HOUSEGEN-1, `holon_scene::household::generate`) is called once per L4 tile (0.01 degree, about
/// 1.1 km, the level nearest its 1 km and tract references; an L3 leaf is its own tile) with the
/// tile's people, its own class and a reference; the tile's households go whole to its leaves by a
/// cumulative midpoint over the leaves' apportioned people, so households sum exactly at the tile
/// and at the leaves, and a leaf's people then follow its households (the tile total stays exact).
/// The legacy rule (round(people / 2.5) households at the leaf, sizes drawn and apportioned, "no
/// certificate") is kept as the labelled fallback and never tuned.
pub mod households {
    use super::{split, Cells, Pop, World};
    use crate::tiling::{is_leaf, HolonId};
    pub use holon_scene::household::Reference;
    use holon_scene::verdict::Verdict;

    /// The leaf rule's target (tiling.js TL_TARGET): households, which above the tile are the
    /// chain's people / 2.5, so the rule is "split while a holon holds more than about 150 people".
    pub const LEAF_TARGET: u64 = 60;
    /// The tile level.
    pub const TILE_LEVEL: u8 = 4;
    /// Lower bounds of CIRISHolon's household classes 1..=5 (village, suburban, terraced, apartment
    /// blocks, dense fabric), people per km2 of the tile (classes.json, the same as the twin's).
    pub const CLASS_KM2: [f64; 5] = [0.0, 250.0, 1_500.0, 6_000.0, 12_000.0];

    /// The tile holding a leaf.
    #[must_use]
    pub fn tile_of(leaf: HolonId) -> HolonId {
        if leaf.level <= TILE_LEVEL { leaf } else { leaf.ancestor(TILE_LEVEL) }
    }

    /// CIRISHolon's class 1..=5 of a tile from its people per km2 (the twin's classes are 0 apartment
    /// blocks .. 3 village with dense fabric above 12,000 inside class 0; this is the explicit map).
    #[must_use]
    pub fn holon_class(people_per_km2: f64) -> u8 {
        (CLASS_KM2.iter().rposition(|&b| people_per_km2 >= b).unwrap_or(0) + 1) as u8
    }

    /// The leaves under `tile` in `children()` order, with their apportioned people (the tree's rule:
    /// L3 or deeper and at most [`LEAF_TARGET`] households, or the last level).
    #[must_use]
    pub fn leaves_under<C: Cells>(cells: &C, tile: HolonId, pop: Pop) -> Vec<(HolonId, u64)> {
        let mut out = Vec::new();
        let mut stack = vec![(tile, pop)];
        while let Some((h, p)) = stack.pop() {
            if is_leaf(h, p.households, LEAF_TARGET) {
                out.push((h, p.people));
                continue;
            }
            for k in split(cells, h, p).into_iter().rev() {
                stack.push(k);
            }
        }
        out
    }

    /// A verdict as the page shows it.
    #[must_use]
    pub const fn verdict_code(v: Verdict) -> u8 {
        match v {
            Verdict::Certified => 0,
            Verdict::Refused => 1,
            Verdict::Unresolved => 2,
        }
    }

    /// What a tile's call gave.
    #[derive(Debug, Clone)]
    pub struct Tile {
        /// The tile.
        pub id: HolonId,
        /// Its people (exact, the chain's).
        pub people: u64,
        /// CIRISHolon's class 1..=5.
        pub class: u8,
        /// The reference read.
        pub reference: Reference,
        /// Whether the tile was inside the reference's country (false while the twin has no country test).
        pub in_country: bool,
        /// Each leaf under the tile, in order, with its households' sizes.
        pub leaves: Vec<(HolonId, Vec<u8>)>,
        /// Verdict (0 certified, 1 refused, 2 unresolved) and reason: households per unit, then composition.
        pub per_unit: (u8, &'static str),
        /// See `per_unit`.
        pub composition: (u8, &'static str),
        /// The notice that travels with the output.
        pub notice: &'static str,
    }

    /// FNV-1a over the sizes of a fixed grid of generator calls (both references, the five classes,
    /// people 0..=2,000 in steps, seeds 0..4): the same number natively and in wasm proves the page runs
    /// the generator the certificate describes (docs/geo/holon/housegen.test.js, popchain test).
    #[must_use]
    pub fn generator_digest() -> u64 {
        let mut h: u64 = 0xCBF2_9CE4_8422_2325;
        for reference in [Reference::Germany2022, Reference::UnitedStates2024] {
            for class in 1..=5u8 {
                for people in (0..=2_000u64).step_by(37) {
                    for seed in 0..4u64 {
                        for s in holon_scene::household::generate(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ people, people, class, reference, true).sizes {
                            h ^= u64::from(s);
                            h = h.wrapping_mul(0x0100_0000_01B3);
                        }
                        h ^= 0xFF;
                        h = h.wrapping_mul(0x0100_0000_01B3);
                    }
                }
            }
        }
        h
    }

    /// Generate tile `tile`'s households with CIRISHolon's generator and split them over its leaves.
    /// `reference` and `in_country` are the caller's MODEL choice (see the kernel's reference rule).
    #[must_use]
    pub fn generate<C: Cells>(cells: &C, world: &World, tile: HolonId, reference: Reference, in_country: bool) -> Tile {
        let pop = super::pop_of(cells, world, tile);
        let class = holon_class(pop.people as f64 / tile.area_km2().max(1e-9));
        let h = holon_scene::household::generate(tile.seed(), pop.people, class, reference, in_country);
        let leaves = leaves_under(cells, tile, pop);
        // cumulative midpoint: household i (sizes in the generator's order) goes to the leaf whose
        // interval of apportioned people holds 2 * (people before it) + size, in doubled units
        let mut ends = Vec::with_capacity(leaves.len());
        let mut acc = 0u64;
        for &(_, p) in &leaves {
            acc += p;
            ends.push(2 * acc);
        }
        let mut per: Vec<Vec<u8>> = vec![Vec::new(); leaves.len()];
        let (mut before, mut li) = (0u64, 0usize);
        for &s in &h.sizes {
            let mid = 2 * before + u64::from(s);
            while li + 1 < ends.len() && mid > ends[li] {
                li += 1;
            }
            if !per.is_empty() {
                per[li].push(s);
            }
            before += u64::from(s);
        }
        Tile {
            id: tile,
            people: pop.people,
            class,
            reference,
            in_country,
            leaves: leaves.iter().map(|x| x.0).zip(per).collect(),
            per_unit: (verdict_code(h.per_unit.verdict), h.per_unit.reason),
            composition: (verdict_code(h.composition.verdict), h.composition.reason),
            notice: h.notice,
        }
    }
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

    /// The same grid with the in-cell seam on, in region `region`.
    struct Uneven(Option<u8>);
    impl Cells for Uneven {
        fn people(&self, r: usize, c: usize) -> u32 {
            Synthetic.people(r, c)
        }
        fn region(&self, _r: usize, _c: usize) -> Option<u8> {
            self.0
        }
        fn incell(&self) -> bool {
            true
        }
    }

    #[test]
    fn incell_divide_is_holons_on_the_world_table() {
        // the twin's divide (table as a parameter, no shuffle) gives CIRISHolon's divide()'s tiles: per
        // class the same count and the same people, for every class, seed and size tried
        use holon_scene::world::incell::{divide, PEOPLE_PPM, TILES_PPM};
        for class in 1..=5u8 {
            for &people in &[0u64, 7, 225, 1_000, 31_337, 4_000_000] {
                for seed in [0u64, 1, 42, 0xDEAD_BEEF] {
                    let k = usize::from(class) - 1;
                    let mine = incell::divide_table(&TILES_PPM[k], &PEOPLE_PPM[k], people, incell::N, seed);
                    let theirs = divide(class, people, incell::N, seed);
                    for cl in 0..6u8 {
                        let mut a: Vec<u64> = mine.iter().filter(|t| t.0 == cl).map(|t| t.1).collect();
                        let mut b: Vec<u64> = theirs.iter().filter(|t| t.class == cl).map(|t| t.people).collect();
                        a.sort_unstable();
                        b.sort_unstable();
                        assert_eq!(a, b, "class {class} people {people} seed {seed} tile class {cl}");
                    }
                    assert_eq!(mine.iter().map(|t| t.1).sum::<u64>(), people);
                }
            }
        }
    }

    #[test]
    fn incell_pixels_sum_to_the_cell_and_the_town_faces_the_city() {
        let (r, c) = (340, 1508); // the cell south of the city centre
        for region in [None, Some(2), Some(0)] {
            let (px, cls, src) = incell::pixels(&Uneven(region), r, c);
            assert_eq!(px.iter().map(|&p| u64::from(p)).sum::<u64>(), u64::from(Synthetic.people(r, c)));
            if region == Some(2) {
                assert_eq!(src, incell::Source::Regional(2));
            }
            // the city is north: the densest pixels sit in the cell's northern rows
            let dense_row: usize = (0..incell::N).max_by_key(|&i| (cls[i], px[i], std::cmp::Reverse(i))).unwrap() / incell::SIDE;
            assert!(dense_row < incell::SIDE / 2, "dense pixel in row {dense_row}");
        }
        // with a populated place in the cell, the densest pixel sits beside it
        struct Placed;
        impl Cells for Placed {
            fn people(&self, r: usize, c: usize) -> u32 {
                Synthetic.people(r, c)
            }
            fn incell(&self) -> bool {
                true
            }
            fn anchor(&self, _r: usize, _c: usize) -> Option<(f64, f64)> {
                Some((13.5, 13.5)) // the south-east corner, away from the city
            }
        }
        let (px, cls, _) = incell::pixels(&Placed, r, c);
        let top = (0..incell::N).max_by_key(|&i| (cls[i], px[i], std::cmp::Reverse(i))).unwrap();
        assert!(top % incell::SIDE >= 10 && top / incell::SIDE >= 10, "dense pixel {top} is by the place");
        // apartment blocks in North America publish no shares: the world row stands in
        assert_eq!(incell::row(4, Some(0)).2, incell::Source::World);
    }

    #[test]
    fn popchain_sums_exactly_with_the_incell_seam_on() {
        let cells = Uneven(Some(2));
        let w = World::new(&cells, 330..350, 7_000_000);
        let mut h = HolonId::at(0, 47.6, 8.5);
        let mut pop = w.root(h);
        let mut uneven = false;
        for _ in 0..9 {
            let kids = split(&cells, h, pop);
            assert_eq!(kids.iter().map(|k| k.1.people).sum::<u64>(), pop.people, "{h:?} people");
            assert_eq!(kids.iter().map(|k| k.1.households).sum::<u64>(), pop.households, "{h:?} households");
            let legacy = split(&Synthetic, h, pop);
            uneven |= legacy.iter().zip(&kids).any(|(a, b)| a.1.people != b.1.people);
            let (k, p) = *kids.iter().max_by_key(|k| (k.1.people, std::cmp::Reverse(k.0.pack()))).unwrap();
            assert_eq!(pop_of(&cells, &w, k), p);
            h = k;
            pop = p;
        }
        assert!(uneven, "the seam must change some split below the cell");
    }

    #[test]
    fn the_generator_digest_is_the_committed_one() {
        // docs/geo/holon/housegen.test.js checks the shipped wasm gives the same number
        let want = include_str!("../../../docs/geo/holon/housegen_digest.txt").trim().parse::<u64>().unwrap();
        assert_eq!(households::generator_digest(), want);
    }

    #[test]
    fn household_classes_map_explicitly_and_tiles_split_whole_households_exactly() {
        use households::*;
        for (d, c) in [(0.5, 1), (249.9, 1), (250.0, 2), (1_499.0, 2), (1_500.0, 3), (5_999.0, 3), (6_000.0, 4), (11_999.0, 4), (12_000.0, 5), (90_000.0, 5)] {
            assert_eq!(holon_class(d), c, "{d} per km2");
        }
        let cells = Uneven(Some(2));
        let w = World::new(&cells, 330..350, 7_000_000);
        for (la, lo) in [(47.6, 8.5), (47.58, 8.47), (47.7, 8.62)] {
            let leaf_tile = tile_of(HolonId::at(9, la, lo));
            for reference in [Reference::Germany2022, Reference::UnitedStates2024] {
                let t = generate(&cells, &w, leaf_tile, reference, false);
                let people: u64 = t.leaves.iter().flat_map(|l| l.1.iter()).map(|&s| u64::from(s)).sum();
                assert_eq!(people, t.people, "the tile's households hold its people exactly");
                assert!(t.leaves.iter().all(|l| l.0.ancestor(TILE_LEVEL) == t.id));
                assert_eq!(t.per_unit.0, 2, "outside the country read: unresolved");
                // a leaf's people follow its households and stay near its apportioned people
                let pop = pop_of(&cells, &w, t.id);
                for ((id, sizes), (id2, apportioned)) in t.leaves.iter().zip(leaves_under(&cells, t.id, pop)) {
                    assert_eq!(*id, id2);
                    let got: u64 = sizes.iter().map(|&s| u64::from(s)).sum();
                    assert!(got.abs_diff(apportioned) <= 16, "{id:?}: {got} people from households, {apportioned} apportioned");
                }
                // same seed, same answer
                let again = generate(&cells, &w, leaf_tile, reference, false);
                assert_eq!(again.leaves, t.leaves);
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
