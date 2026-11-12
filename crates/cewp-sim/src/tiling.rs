//! Holon tiling of the globe: the lattice the page zooms through from the 1.5° population grid
//! down to single streets (FSD §4.6.6).
//!
//! Every holon is a lat/lon cell on one global integer lattice, so the same id names the same
//! ground in Rust and in the page (`docs/holon.js` carries the same arithmetic):
//!
//! | level | cell      | children per side |
//! |-------|-----------|-------------------|
//! | L0    | 1.5°      | (population grid) |
//! | L1    | 0.3°      | 5                 |
//! | L2    | 0.06°     | 5                 |
//! | L3    | 0.02°     | 3 (the page's street tile, `TILE_DEG`) |
//! | L4..  | 0.02°/2ᵏ  | 2 (adaptive quadtree) |
//!
//! Below L3 a holon splits while it holds more households than one street view shows
//! (`P::TargetHouseholds`), so depth is a pure function of the budget. A parent's households go to
//! its children by integer largest-remainder apportionment over weights (the caller's land and
//! place weight × a value-noise field on the global lattice), so the children always sum to the
//! parent exactly and nothing depends on the order holons are generated or evicted.
//!
//! Streets are lines each cell inherits from its parent: an L2 cell draws arterials, every cell
//! below adds local streets for its own density. Streets cross a holon's edge only at the edge's
//! crossings: the lines of the lowest cell holding both sides, or, across a tile boundary, a hash
//! of the two cells at the SHALLOWER side's level; a deeper holon's own streets stay inside it.
//! Everything here is procedural: no real road or building appears below region scale.

use crate::micro::{footprint_meets_road, rects_overlap, Building, DensityClass, MicroParams, Rng, Road, NO_STREET, P};
use std::collections::BTreeMap;

/// Lattice units per degree: one unit is 0.02° / 512, the L12 cell (about 4.3 m north-south).
pub const UNITS_PER_DEG: i64 = 25_600;
/// The street-tile level (the page's 0.02° `TILE_DEG` tile, same `(ti, tj)`).
pub const L3: u8 = 3;
/// Deepest level: a holon at L12 is a leaf whatever its budget.
pub const MAX_LEVEL: u8 = 12;
/// Cell side in lattice units, by level.
pub const STEP: [i64; 13] = [38_400, 7_680, 1_536, 512, 256, 128, 64, 32, 16, 8, 4, 2, 1];
/// Children per side when a cell of level `l - 1` splits into level `l`.
pub const RATIO: [i64; 13] = [1, 5, 5, 3, 2, 2, 2, 2, 2, 2, 2, 2, 2];
/// Lattice units around a parallel.
pub const LON_UNITS: i64 = 360 * UNITS_PER_DEG;
/// Lattice units from the equator to a pole.
pub const LAT_UNITS_HALF: i64 = 90 * UNITS_PER_DEG;
/// Metres per degree, the flat-patch scale `MicroWorld` uses.
pub const M_PER_DEG: f64 = 111_320.0;
/// The tiling's global seed (every holon's seed mixes this with its id).
pub const HOLON_SEED: u64 = 0xC125_4010_0E1D_7A11;

const I_OFF: i64 = 1 << 22;
const J_OFF: i64 = 1 << 23;

/// A side of a cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Dir {
    /// North.
    N = 0,
    /// East.
    E = 1,
    /// South.
    S = 2,
    /// West.
    W = 3,
}

impl Dir {
    /// All four, in export order.
    pub const ALL: [Dir; 4] = [Dir::N, Dir::E, Dir::S, Dir::W];
    /// The opposite side.
    #[must_use]
    pub const fn flip(self) -> Self {
        match self {
            Dir::N => Dir::S,
            Dir::E => Dir::W,
            Dir::S => Dir::N,
            Dir::W => Dir::E,
        }
    }
    /// True for the east-west running edges (north and south sides).
    #[must_use]
    pub const fn horizontal(self) -> bool {
        matches!(self, Dir::N | Dir::S)
    }
}

/// splitmix64 finaliser.
#[must_use]
pub fn mix64(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// One cell of the lattice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HolonId {
    /// Level, 0..=12.
    pub level: u8,
    /// Row: the cell spans latitude units `[i·step, (i+1)·step)`.
    pub i: i32,
    /// Column, wrapped to `[-cols/2, cols/2)`.
    pub j: i32,
}

impl HolonId {
    /// Columns around the globe at a level.
    #[must_use]
    pub const fn cols(level: u8) -> i64 {
        LON_UNITS / STEP[level as usize]
    }
    /// Rows from the equator to a pole at a level.
    #[must_use]
    pub const fn rows_half(level: u8) -> i64 {
        LAT_UNITS_HALF / STEP[level as usize]
    }
    fn wrap_j(level: u8, j: i64) -> i32 {
        let n = Self::cols(level);
        ((j + n / 2).rem_euclid(n) - n / 2) as i32
    }
    /// The cell at `(level, i, j)`, `j` wrapped; `None` past a pole.
    #[must_use]
    pub fn new(level: u8, i: i64, j: i64) -> Option<Self> {
        if level > MAX_LEVEL {
            return None;
        }
        let h = Self::rows_half(level);
        if i < -h || i >= h {
            return None;
        }
        Some(Self { level, i: i as i32, j: Self::wrap_j(level, j) })
    }
    /// The cell holding a point. Latitude is clamped inside the poles; longitude wraps.
    #[must_use]
    pub fn at(level: u8, lat: f64, lon: f64) -> Self {
        let s = STEP[level as usize];
        let y = (lat * UNITS_PER_DEG as f64).floor() as i64;
        let x = (lon * UNITS_PER_DEG as f64).floor() as i64;
        let h = Self::rows_half(level);
        let i = y.div_euclid(s).clamp(-h, h - 1);
        Self { level, i: i as i32, j: Self::wrap_j(level, x.div_euclid(s)) }
    }
    /// The id as one integer below 2^51 (exact in a JavaScript number):
    /// `level·2^47 + (i + 2^22)·2^24 + (j + 2^23)`.
    #[must_use]
    pub fn pack(self) -> u64 {
        ((self.level as u64) << 47) | (((self.i as i64 + I_OFF) as u64) << 24) | ((self.j as i64 + J_OFF) as u64)
    }
    /// Inverse of [`HolonId::pack`]; `None` for a value no cell packs to.
    #[must_use]
    pub fn unpack(v: u64) -> Option<Self> {
        let level = (v >> 47) as u8;
        let i = ((v >> 24) & ((1 << 23) - 1)) as i64 - I_OFF;
        let j = (v & ((1 << 24) - 1)) as i64 - J_OFF;
        let id = Self::new(level, i, j)?;
        (id.pack() == v).then_some(id)
    }
    /// Cell side in lattice units.
    #[must_use]
    pub const fn step(self) -> i64 {
        STEP[self.level as usize]
    }
    /// The containing cell one level up (`None` at L0).
    #[must_use]
    pub fn parent(self) -> Option<Self> {
        if self.level == 0 {
            return None;
        }
        let r = RATIO[self.level as usize];
        Some(Self { level: self.level - 1, i: (self.i as i64).div_euclid(r) as i32, j: (self.j as i64).div_euclid(r) as i32 })
    }
    /// The containing cell at `level` (itself when `level >= self.level`).
    #[must_use]
    pub fn ancestor(self, level: u8) -> Self {
        let mut a = self;
        while a.level > level {
            a = a.parent().expect("level > 0");
        }
        a
    }
    /// Children, row by row from the south-west corner (empty at `MAX_LEVEL`).
    #[must_use]
    pub fn children(self) -> Vec<Self> {
        if self.level >= MAX_LEVEL {
            return Vec::new();
        }
        let r = RATIO[self.level as usize + 1];
        let mut v = Vec::with_capacity((r * r) as usize);
        for a in 0..r {
            for b in 0..r {
                v.push(Self { level: self.level + 1, i: (self.i as i64 * r + a) as i32, j: (self.j as i64 * r + b) as i32 });
            }
        }
        v
    }
    /// The cell across a side at the same level (`None` past a pole).
    #[must_use]
    pub fn neighbour(self, d: Dir) -> Option<Self> {
        let (di, dj) = match d {
            Dir::N => (1, 0),
            Dir::E => (0, 1),
            Dir::S => (-1, 0),
            Dir::W => (0, -1),
        };
        Self::new(self.level, self.i as i64 + di, self.j as i64 + dj)
    }
    /// South, west, north, east bounds in lattice units (`x` in this cell's own column frame).
    #[must_use]
    pub fn bounds_units(self) -> (i64, i64, i64, i64) {
        let s = self.step();
        (self.i as i64 * s, self.j as i64 * s, (self.i as i64 + 1) * s, (self.j as i64 + 1) * s)
    }
    /// South, west, north, east bounds in degrees.
    #[must_use]
    pub fn bounds_deg(self) -> (f64, f64, f64, f64) {
        let (y0, x0, y1, x1) = self.bounds_units();
        let k = UNITS_PER_DEG as f64;
        (y0 as f64 / k, x0 as f64 / k, y1 as f64 / k, x1 as f64 / k)
    }
    /// Centre in degrees.
    #[must_use]
    pub fn centre_deg(self) -> (f64, f64) {
        let (a, b, c, d) = self.bounds_deg();
        ((a + c) / 2.0, (b + d) / 2.0)
    }
    /// Width (east-west, at the centre's latitude) and height in metres, on the flat-patch scale.
    #[must_use]
    pub fn size_m(self) -> (f64, f64) {
        let s = self.step() as f64 / UNITS_PER_DEG as f64;
        let (lat, _) = self.centre_deg();
        (s * M_PER_DEG * crate::detmath::cos_deg(lat).max(0.01), s * M_PER_DEG)
    }
    /// Area in km².
    #[must_use]
    pub fn area_km2(self) -> f64 {
        let (w, h) = self.size_m();
        w * h / 1e6
    }
    /// This holon's seed under the global seed.
    #[must_use]
    pub fn seed(self) -> u64 {
        mix64(HOLON_SEED ^ self.pack().wrapping_mul(0xD6E8_FEB8_6659_FD93))
    }
}

// ------------------------------------------------------------------ fields

fn lattice_hash(gx: i64, gy: i64, salt: u64) -> u32 {
    (mix64((gx as u64).wrapping_mul(0x9E37_79B1_85EB_CA87) ^ (gy as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F) ^ salt) >> 54) as u32
}

/// Value noise in `[0, 1024]` on the global integer lattice: corners every `period` units
/// (a divisor of `LON_UNITS`, so the field wraps at the antimeridian), smoothstep-bilinear in
/// integer arithmetic. Continuous across every holon edge because it is one global field.
#[must_use]
pub fn value_noise(x: i64, y: i64, period: i64, salt: u64) -> u32 {
    let cols = LON_UNITS / period;
    let gx = x.div_euclid(period);
    let gy = y.div_euclid(period);
    let fx = x.rem_euclid(period) * 1024 / period;
    let fy = y.rem_euclid(period) * 1024 / period;
    let s = |t: i64| t * t * (3 * 1024 - 2 * t) / (1024 * 1024);
    let (sx, sy) = (s(fx), s(fy));
    let wx = |g: i64| g.rem_euclid(cols);
    let c00 = lattice_hash(wx(gx), gy, salt) as i64;
    let c10 = lattice_hash(wx(gx + 1), gy, salt) as i64;
    let c01 = lattice_hash(wx(gx), gy + 1, salt) as i64;
    let c11 = lattice_hash(wx(gx + 1), gy + 1, salt) as i64;
    let a = c00 * (1024 - sx) + c10 * sx;
    let b = c01 * (1024 - sx) + c11 * sx;
    ((a * (1024 - sy) + b * sy) / (1024 * 1024)) as u32
}

/// The noise factor a child's weight carries, in `[256, 1279]`: two octaves keyed to the
/// parent's cell size, so neighbouring children (in any parent) vary smoothly.
#[must_use]
pub fn child_noise(child: HolonId) -> u64 {
    let s = child.step();
    let (y0, x0, _, _) = child.bounds_units();
    let (x, y) = (x0 + s / 2, y0 + s / 2);
    let p = STEP[child.level.saturating_sub(1) as usize];
    let n1 = value_noise(x, y, p, 0x5EED_0001) as u64;
    let n2 = value_noise(x, y, (p * 4).min(STEP[0]), 0x5EED_0002) as u64;
    256 + (n1 * 3 + n2) / 4
}

/// Largest-remainder apportionment: `total` split in proportion to `weights`, summing exactly to
/// `total`; remainders tie toward the lower index. All-zero weights split evenly.
#[must_use]
pub fn apportion(total: u64, weights: &[u64]) -> Vec<u64> {
    if weights.is_empty() {
        return Vec::new();
    }
    let sum: u128 = weights.iter().map(|&w| w as u128).sum();
    let ws: Vec<u128> = if sum == 0 { vec![1; weights.len()] } else { weights.iter().map(|&w| w as u128).collect() };
    let sum: u128 = ws.iter().sum();
    let mut out: Vec<u64> = ws.iter().map(|&w| (total as u128 * w / sum) as u64).collect();
    let given: u64 = out.iter().sum();
    let mut rem: Vec<(u128, usize)> = ws.iter().enumerate().map(|(k, &w)| (total as u128 * w % sum, k)).collect();
    rem.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    for &(_, k) in rem.iter().take((total - given) as usize) {
        out[k] += 1;
    }
    out
}

/// A parent's households over its children: `ext` is the caller's weight per child (land share ×
/// populated-place kernel, quantised), in [`HolonId::children`] order.
#[must_use]
pub fn split(parent: HolonId, budget: u64, ext: &[u32]) -> Vec<(HolonId, u64)> {
    let kids = parent.children();
    assert_eq!(kids.len(), ext.len(), "one weight per child");
    let w: Vec<u64> = kids.iter().zip(ext).map(|(c, &e)| e as u64 * child_noise(*c)).collect();
    kids.into_iter().zip(apportion(budget, &w)).collect()
}

/// A holon is a leaf at L3 or below once it holds no more than `target` households (or at L12).
#[must_use]
pub fn is_leaf(id: HolonId, budget: u64, target: u64) -> bool {
    id.level >= L3 && (budget <= target || id.level >= MAX_LEVEL)
}

// ------------------------------------------------------------------ tree (reference resolver)

/// The tree over caller-supplied inputs: `root(l0) -> households` and `ext(child) -> weight`.
/// Budgets are memoised; [`HolonTree::evict`] drops any of them and they come back identical.
/// The page keeps the same tree in JavaScript and calls [`split`] through the kernel.
pub struct HolonTree<R: Fn(HolonId) -> u64, W: Fn(HolonId) -> u32> {
    root: R,
    ext: W,
    /// Households a street view shows (`P::TargetHouseholds`).
    pub target: u64,
    cache: BTreeMap<HolonId, u64>,
}

impl<R: Fn(HolonId) -> u64, W: Fn(HolonId) -> u32> HolonTree<R, W> {
    /// A tree over the two input functions.
    pub fn new(root: R, ext: W, target: u64) -> Self {
        Self { root, ext, target, cache: BTreeMap::new() }
    }
    /// Households in a cell.
    pub fn budget(&mut self, id: HolonId) -> u64 {
        if id.level == 0 {
            return (self.root)(id);
        }
        if let Some(&b) = self.cache.get(&id) {
            return b;
        }
        let p = id.parent().expect("level > 0");
        let pb = self.budget(p);
        let ext: Vec<u32> = p.children().iter().map(|&c| (self.ext)(c)).collect();
        let mut mine = 0;
        for (c, b) in split(p, pb, &ext) {
            if c == id {
                mine = b;
            }
            self.cache.insert(c, b);
        }
        mine
    }
    /// Drop memoised budgets (all of them with `None`).
    pub fn evict(&mut self, id: Option<HolonId>) {
        match id {
            Some(i) => {
                self.cache.remove(&i);
            }
            None => self.cache.clear(),
        }
    }
    /// Memoised entries.
    #[must_use]
    pub fn cached(&self) -> usize {
        self.cache.len()
    }
    /// True if the cell is a leaf of the tiling.
    pub fn leaf(&mut self, id: HolonId) -> bool {
        let b = self.budget(id);
        is_leaf(id, b, self.target)
    }
    /// The leaf holding a point.
    pub fn leaf_at(&mut self, lat: f64, lon: f64) -> HolonId {
        let mut id = HolonId::at(L3, lat, lon);
        while !self.leaf(id) {
            id = HolonId::at(id.level + 1, lat, lon);
        }
        id
    }
    /// The leaf that is `cell` or an ancestor of it, if any (`None` when `cell` is split).
    pub fn leaf_over(&mut self, cell: HolonId) -> Option<HolonId> {
        for l in L3..=cell.level {
            let a = cell.ancestor(l);
            if self.leaf(a) {
                return Some(a);
            }
        }
        None
    }
    /// What a leaf needs from outside itself to lay out its streets.
    pub fn context(&mut self, leaf: HolonId) -> LeafContext {
        let budget = self.budget(leaf);
        let a2 = leaf.ancestor(2);
        let mut l2_nb = [0u64; 4];
        let mut edges = [EdgeCtx::default(); 4];
        for d in Dir::ALL {
            l2_nb[d as usize] = a2.neighbour(d).map_or(0, |n| self.budget(n));
            let Some(nb) = leaf.neighbour(d) else { continue };
            // the neighbour side's leaf, if it is no deeper than this one
            let level = self.leaf_over(nb).map_or(leaf.level, |y| y.level);
            let own = leaf.ancestor(level);
            let other = own.neighbour(d).expect("same-level neighbour exists");
            edges[d as usize] = EdgeCtx { level, own_budget: self.budget(own), other_budget: self.budget(other), present: true };
        }
        let mut chain = [0u64; 11];
        for l in 2..=leaf.level {
            chain[(l - 2) as usize] = self.budget(leaf.ancestor(l));
        }
        LeafContext { budget, l2_nb, edges, chain, people: 0 }
    }
}

/// One side of a leaf as the neighbourhood decides it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EdgeCtx {
    /// The level the crossings of this side are drawn at: the shallower of the two sides.
    pub level: u8,
    /// Households in this leaf's ancestor at `level`.
    pub own_budget: u64,
    /// Households in the cell across the side at `level`.
    pub other_budget: u64,
    /// False past a pole.
    pub present: bool,
}

/// Everything outside a leaf its layout reads. The page builds the same record from its tree.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LeafContext {
    /// Households in the leaf.
    pub budget: u64,
    /// Households in the four L2 cells around the leaf's L2 cell (N, E, S, W).
    pub l2_nb: [u64; 4],
    /// The leaf's four sides (N, E, S, W).
    pub edges: [EdgeCtx; 4],
    /// Households in the leaf's ancestors from L2 down to the leaf (`chain[k]` is level 2 + k).
    pub chain: [u64; 11],
    /// People in the leaf (popchain), so its households' persons sum to them; 0 = not given.
    pub people: u64,
}

impl LeafContext {
    /// Flat form for the kernel door (33 numbers): budget, 4 L2 neighbours, per side level, own
    /// budget, other budget, present, then the 11 chain budgets, then the leaf's people.
    #[must_use]
    pub fn to_f64(&self) -> [f64; 33] {
        let mut v = [0.0; 33];
        v[0] = self.budget as f64;
        for k in 0..4 {
            v[1 + k] = self.l2_nb[k] as f64;
            let e = self.edges[k];
            v[5 + 4 * k] = e.level as f64;
            v[6 + 4 * k] = e.own_budget as f64;
            v[7 + 4 * k] = e.other_budget as f64;
            v[8 + 4 * k] = if e.present { 1.0 } else { 0.0 };
        }
        for k in 0..11 {
            v[21 + k] = self.chain[k] as f64;
        }
        v[32] = self.people as f64;
        v
    }
    /// Inverse of [`LeafContext::to_f64`].
    #[must_use]
    pub fn from_f64(v: &[f64]) -> Self {
        let g = |k: usize| v.get(k).copied().unwrap_or(0.0).max(0.0) as u64;
        let mut c = Self { budget: g(0), ..Self::default() };
        for k in 0..4 {
            c.l2_nb[k] = g(1 + k);
            c.edges[k] = EdgeCtx { level: g(5 + 4 * k).min(MAX_LEVEL as u64) as u8, own_budget: g(6 + 4 * k), other_budget: g(7 + 4 * k), present: g(8 + 4 * k) != 0 };
        }
        for k in 0..11 {
            c.chain[k] = g(21 + k);
        }
        c.people = g(32);
        c
    }
}

// ------------------------------------------------------------------ edges

/// Street spacing (metres) for a household density (households per km²): about 90 m in a dense
/// core, 230 m suburban, 1.4 km in open country. MODEL, a visual rule; no CIRIS source.
#[must_use]
pub fn street_spacing_m(hh_per_km2: f64) -> f64 {
    (3400.0 / crate::detmath::pow(hh_per_km2.max(0.01), 0.4)).clamp(60.0, 3000.0)
}

/// A street crossing a side, at an absolute lattice position along it (longitude units for a
/// north or south side, latitude units for an east or west side).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Port {
    /// Which side of the leaf.
    pub dir: Dir,
    /// Position along the side, lattice units.
    pub pos: f64,
    /// A main road (handed down from the L3 tile).
    pub main: bool,
}

fn pair_key(a: HolonId, b: HolonId, salt: u64) -> u64 {
    let (p, q) = if a.pack() <= b.pack() { (a.pack(), b.pack()) } else { (b.pack(), a.pack()) };
    mix64(mix64(p ^ salt) ^ q.wrapping_mul(0x9E37_79B9_7F4A_7C15))
}

fn edge_span(a: HolonId, d: Dir) -> (f64, f64, f64) {
    // (start, length) along the side in units, and its length in metres
    let (y0, x0, y1, _) = a.bounds_units();
    let s = a.step() as f64;
    let deg = s / UNITS_PER_DEG as f64;
    if d.horizontal() {
        let lat = if d == Dir::N { y1 } else { y0 } as f64 / UNITS_PER_DEG as f64;
        (x0 as f64, s, deg * M_PER_DEG * crate::detmath::cos_deg(lat).max(0.01))
    } else {
        (y0 as f64, s, deg * M_PER_DEG)
    }
}

/// Local street crossings on side `d` of cell `a`, whose neighbour across it holds `bud_b`.
/// Symmetric: the neighbour asking about its opposite side gets the same positions.
#[must_use]
pub fn local_crossings(a: HolonId, d: Dir, bud_a: u64, bud_b: u64) -> Vec<f64> {
    let Some(b) = a.neighbour(d) else { return Vec::new() };
    if bud_a == 0 || bud_b == 0 {
        return Vec::new();
    }
    let dens = (bud_a + bud_b) as f64 / (a.area_km2() + b.area_km2());
    let (start, len_u, len_m) = edge_span(a, d);
    let mut r = Rng::new(pair_key(a, b, 0xED6E));
    let n = (len_m / street_spacing_m(dens) + r.f() * 0.6 - 0.3).round().max(0.0) as u64;
    let mut v = Vec::with_capacity(n as usize);
    for k in 0..n {
        let t = (k as f64 + 0.5 + (r.f() - 0.5) * 0.6) / n as f64;
        // quantised to 1/65536 of the side: both sides compute the same exact value
        let q = (t * 65_536.0).floor().clamp(1.0, 65_535.0);
        v.push(start + q * len_u / 65_536.0);
    }
    v
}

/// Arterial spacing (metres): eight local blocks, clamped to 0.7–8 km. MODEL, a visual rule.
#[must_use]
pub fn arterial_spacing_m(hh_per_km2: f64) -> f64 {
    (street_spacing_m(hh_per_km2) * 8.0).clamp(700.0, 8000.0)
}

/// One street line of a cell: its position in lattice units (x for a north-south street, y for
/// an east-west one) and whether it is an arterial.
pub type Line = (f64, bool);

/// Street lines of a cell. An L2 cell (0.06°) draws its arterials; every cell below keeps the
/// lines of its parent that run through it and adds local lines in the widest gaps until its own
/// density's spacing is met, so streets and arterials run straight on through holons of any
/// depth. `chain[k]` holds the households at level 2 + k of the cell's ancestry (`ns` picks the
/// north-south lines).
#[must_use]
pub fn cell_lines(id: HolonId, chain: &[u64], ns: bool) -> Vec<Line> {
    let mut lines: Vec<Line> = Vec::new();
    for l in 2..=id.level {
        let c = id.ancestor(l);
        let (y0, x0, y1, x1) = c.bounds_units();
        let (lo, hi) = if ns { (x0 as f64, x1 as f64) } else { (y0 as f64, y1 as f64) };
        lines.retain(|v| v.0 > lo && v.0 < hi);
        let b = chain.get((l - 2) as usize).copied().unwrap_or(0);
        if b == 0 {
            lines.clear();
            continue;
        }
        let (w, h) = c.size_m();
        let len = if ns { w } else { h };
        let dens = b as f64 / c.area_km2();
        let main = l == 2;
        let want = if main {
            // an arterial for every ~1,500 households at most: open country has none
            ((len / arterial_spacing_m(dens)).round() as usize).min((b / 1500) as usize)
        } else {
            // a handful of homes needs a lane, not a grid
            ((len / street_spacing_m(dens)).round() as usize).min((b as f64 / 3.0).ceil() as usize)
        };
        let mut r = Rng::new(c.seed() ^ if ns { 0x4E53 } else { 0x4557 });
        while lines.len() < want {
            let mut best = (0.0, lo);
            let mut prev = lo;
            for v in lines.iter().map(|v| v.0).chain(std::iter::once(hi)) {
                if v - prev > best.0 {
                    best = (v - prev, prev);
                }
                prev = v;
            }
            let v = best.1 + best.0 * (0.5 + (r.f() - 0.5) * 0.4);
            let at = lines.partition_point(|x| x.0 < v);
            lines.insert(at, (v, main));
        }
    }
    lines
}

/// The streets crossing side `d` of leaf `id`, as (position, arterial).
/// - Inside an L2 cell they are the lines of the lowest cell holding both sides; a few local ones
///   are dropped by the pair's hash, so streets also end in T-junctions.
/// - Across an L2 boundary the arterials are those of the side with fewer (the lower id on a tie).
/// - Across an L3 tile boundary, local crossings are also drawn from a hash of the two cells at the
///   shallower side's level, clear of the lines already crossing there.
#[must_use]
pub fn side_crossings(id: HolonId, d: Dir, ctx: &LeafContext) -> Vec<Line> {
    let e = ctx.edges[d as usize];
    if !e.present || e.own_budget == 0 || e.other_budget == 0 {
        return Vec::new();
    }
    let ns = d.horizontal();
    let a = id.ancestor(e.level);
    let Some(b) = a.neighbour(d) else { return Vec::new() };
    let (y0, x0, y1, x1) = a.bounds_units();
    let (lo, hi) = if ns { (x0 as f64, x1 as f64) } else { (y0 as f64, y1 as f64) };
    let mut m = e.level;
    while m > 2 && a.ancestor(m) != b.ancestor(m) {
        m -= 1;
    }
    let key = pair_key(a, b, 0x7EE);
    let mut out: Vec<Line> = if a.ancestor(m) == b.ancestor(m) {
        cell_lines(a.ancestor(m), &ctx.chain, ns).into_iter().filter(|v| v.0 > lo && v.0 < hi && (v.1 || mix64(key ^ v.0.to_bits()) % 100 >= 6)).collect()
    } else {
        let (o2, t2) = (a.ancestor(2), b.ancestor(2));
        let mine = cell_lines(o2, &ctx.chain[..1], ns);
        let theirs = cell_lines(t2, &[ctx.l2_nb[d as usize]], ns);
        let pick = if mine.len() < theirs.len() || (mine.len() == theirs.len() && o2.pack() < t2.pack()) { mine } else { theirs };
        pick.into_iter().filter(|v| v.0 > lo && v.0 < hi).collect()
    };
    if m < L3 || a.ancestor(m) != b.ancestor(m) {
        let k_m = M_PER_DEG / UNITS_PER_DEG as f64;
        let near: Vec<f64> = out.iter().map(|v| v.0).collect();
        for p in local_crossings(a, d, e.own_budget, e.other_budget) {
            if p > lo && p < hi && near.iter().all(|&q| (p - q).abs() * k_m >= 30.0) {
                out.push((p, false));
            }
        }
        out.sort_by(|x, y| x.0.total_cmp(&y.0));
    }
    out
}

/// Arterials crossing side `d` of holon `id` (level 2 or deeper; 0 above, where no arterial is
/// laid): the arterials of the L2 lattice whose position lies in the side's half-open interval,
/// whether or not a leaf draws them (an empty leaf draws no street, the lattice still runs through
/// it). Across an L2 boundary the arterials are those of the side with fewer, the rule
/// [`side_crossings`] draws by, so the holon across the side counts the same lines; the children
/// along a side split its interval, so their counts sum to it (CIRISHolon's edge law). `l2_budget`
/// gives an L2 cell's households.
#[must_use]
pub fn arterial_crossings(id: HolonId, d: Dir, l2_budget: &mut impl FnMut(HolonId) -> u64) -> u64 {
    if id.level < 2 {
        return 0;
    }
    let Some(b) = id.neighbour(d) else { return 0 };
    let ns = d.horizontal();
    let (y0, x0, y1, x1) = id.bounds_units();
    let (lo, hi) = if ns { (x0 as f64, x1 as f64) } else { (y0 as f64, y1 as f64) };
    let (o2, t2) = (id.ancestor(2), b.ancestor(2));
    let mine = cell_lines(o2, &[l2_budget(o2)], ns);
    let lines = if o2 == t2 {
        mine
    } else {
        let theirs = cell_lines(t2, &[l2_budget(t2)], ns);
        if mine.len() < theirs.len() || (mine.len() == theirs.len() && o2.pack() < t2.pack()) { mine } else { theirs }
    };
    lines.iter().filter(|v| v.1 && v.0 >= lo && v.0 < hi).count() as u64
}

/// One side of a split for the sums file: the parent's arterial crossings on side `d`, the holon
/// across it (same level), and the crossings of the children along that side, in `children()`
/// order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SideSum {
    /// The side.
    pub side: Dir,
    /// Crossings on the parent's side.
    pub parent: u64,
    /// The holon across the side, if any.
    pub peer: Option<HolonId>,
    /// Crossings of each child along the side.
    pub children: Vec<u64>,
}

/// The four sides of `parent`'s split ([`arterial_crossings`] on the parent and on the children
/// along each side).
#[must_use]
pub fn split_crossings(parent: HolonId, l2_budget: &mut impl FnMut(HolonId) -> u64) -> Vec<SideSum> {
    let kids = parent.children();
    Dir::ALL
        .iter()
        .map(|&d| {
            let along: Vec<u64> = kids
                .iter()
                .filter(|k| k.neighbour(d).is_none_or(|n| n.ancestor(parent.level) != parent))
                .map(|&k| arterial_crossings(k, d, l2_budget))
                .collect();
            SideSum { side: d, parent: arterial_crossings(parent, d, l2_budget), peer: parent.neighbour(d), children: along }
        })
        .collect()
}

// ------------------------------------------------------------------ leaf layout

/// What meets a street's end, or where two streets meet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum JunctionKind {
    /// The street leaves the leaf through a side (it runs on in the neighbour).
    Exit = 0,
    /// The street ends inside the leaf with nothing across it.
    DeadEnd = 1,
    /// One street ends on another.
    Tee = 2,
    /// Two streets cross.
    Cross = 3,
    /// Both streets end where they meet.
    Corner = 4,
}

/// A point of the street graph: two streets meeting, or one street's open end.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Junction {
    /// East, leaf-local metres.
    pub x: f64,
    /// North, leaf-local metres.
    pub y: f64,
    /// Its kind.
    pub kind: JunctionKind,
    /// The street (index into the leaf's roads); for a meeting, the north-south one.
    pub a: u32,
    /// The other street of a meeting; [`NO_STREET`] for an exit or a dead end.
    pub b: u32,
}

/// A street: one of the leaf's roads, named by the leaf and its index.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Street {
    /// The leaf holding it (the street's id is `(leaf, index)`).
    pub leaf: HolonId,
    /// Its index in the leaf's roads.
    pub index: u32,
    /// An arterial (main road width).
    pub main: bool,
    /// What its start and its end meet.
    pub ends: [JunctionKind; 2],
}

/// The leaf's streets as a graph: streets, the junctions where they meet or end, and (on the
/// buildings, `Building::street`) which street each building and so each household is on.
#[derive(Debug, Clone, PartialEq)]
pub struct StreetGraph {
    /// Streets, in road order.
    pub streets: Vec<Street>,
    /// Meetings first (in street order), then open ends.
    pub junctions: Vec<Junction>,
}

impl StreetGraph {
    /// The graph's segments: each street cut at every junction on it, as (node a, node b, length m),
    /// with nodes numbered by first appearance (junction points and street ends).
    #[must_use]
    pub fn segments(&self, roads: &[Road]) -> (usize, Vec<(usize, usize, f64)>) {
        let mut ids: BTreeMap<(i64, i64), usize> = BTreeMap::new();
        let mut node = |x: f64, y: f64| {
            let k = ((x * 1e4).round() as i64, (y * 1e4).round() as i64);
            let n = ids.len();
            *ids.entry(k).or_insert(n)
        };
        let mut segs = Vec::new();
        for st in &self.streets {
            let r = roads[st.index as usize];
            let (dx, dy) = (r.x1 - r.x0, r.y1 - r.y0);
            let len = (dx * dx + dy * dy).sqrt();
            if len <= 0.0 {
                continue;
            }
            let mut ts: Vec<f64> = vec![0.0, 1.0];
            for j in self.junctions.iter().filter(|j| j.a == st.index || j.b == st.index) {
                ts.push((((j.x - r.x0) * dx + (j.y - r.y0) * dy) / (len * len)).clamp(0.0, 1.0));
            }
            ts.sort_by(f64::total_cmp);
            ts.dedup_by(|a, b| (*a - *b).abs() * len < 1e-6);
            for w in ts.windows(2) {
                let a = node(r.x0 + w[0] * dx, r.y0 + w[0] * dy);
                let b = node(r.x0 + w[1] * dx, r.y0 + w[1] * dy);
                segs.push((a, b, (w[1] - w[0]) * len));
            }
        }
        (ids.len(), segs)
    }

    /// Edge betweenness of every segment (shortest paths by length between every pair of nodes,
    /// Brandes; each unordered pair counted once), in [`StreetGraph::segments`] order.
    #[must_use]
    pub fn segment_betweenness(&self, roads: &[Road]) -> Vec<f64> {
        let (n, segs) = self.segments(roads);
        let mut adj: Vec<Vec<(usize, f64, usize)>> = vec![Vec::new(); n];
        for (e, &(a, b, l)) in segs.iter().enumerate() {
            adj[a].push((b, l, e));
            adj[b].push((a, l, e));
        }
        let mut cb = vec![0.0; segs.len()];
        for s in 0..n {
            // Dijkstra from s, keeping every shortest predecessor
            let mut dist = vec![f64::INFINITY; n];
            let mut sigma = vec![0.0_f64; n];
            let mut pred: Vec<Vec<(usize, usize)>> = vec![Vec::new(); n];
            let mut order = Vec::with_capacity(n);
            let mut done = vec![false; n];
            dist[s] = 0.0;
            sigma[s] = 1.0;
            while let Some(v) = (0..n).filter(|&v| !done[v] && dist[v].is_finite()).min_by(|&a, &b| dist[a].total_cmp(&dist[b]).then(a.cmp(&b))) {
                done[v] = true;
                order.push(v);
                for &(w, l, e) in &adj[v] {
                    let d = dist[v] + l;
                    if d < dist[w] - 1e-9 {
                        dist[w] = d;
                        sigma[w] = sigma[v];
                        pred[w] = vec![(v, e)];
                    } else if (d - dist[w]).abs() <= 1e-9 && !done[w] {
                        sigma[w] += sigma[v];
                        pred[w].push((v, e));
                    }
                }
            }
            let mut delta = vec![0.0; n];
            for &w in order.iter().rev() {
                for &(v, e) in &pred[w] {
                    let c = sigma[v] / sigma[w] * (1.0 + delta[w]);
                    cb[e] += c;
                    delta[v] += c;
                }
            }
        }
        cb.iter().map(|c| c / 2.0).collect()
    }

    /// Gini coefficient of segment betweenness (0 = traffic spread evenly over the segments, 1 = all
    /// on one): how unevenly shortest paths concentrate on the street network (CIRISHolon's street
    /// reference reads the same statistic). 0 for fewer than two segments.
    #[must_use]
    pub fn betweenness_gini(&self, roads: &[Road]) -> f64 {
        let mut v = self.segment_betweenness(roads);
        let n = v.len();
        let total: f64 = v.iter().sum();
        if n < 2 || total <= 0.0 {
            return 0.0;
        }
        v.sort_by(f64::total_cmp);
        let weighted: f64 = v.iter().enumerate().map(|(i, x)| (2.0 * (i as f64 + 1.0) - n as f64 - 1.0) * x).sum();
        weighted / (n as f64 * total)
    }
}

/// Main road width, metres (an arterial, every class).
pub const MAIN_ROAD_M: f64 = 12.0;

/// People per km² from which an apartment-block leaf is laid out as dense mid- and high-rise fabric
/// (MODEL): narrow lanes instead of 8 m streets, plots filling the leaf rather than lining its
/// streets, and the leaf's households shared over the buildings that fit, so storeys follow
/// density (`floors = ceil(households per building / flats per floor)`, at least
/// [`DENSE_MIN_FLOORS`]). Below it, apartment blocks keep 4 to 7 storeys on street frontage.
pub const DENSE_FABRIC_KM2: f64 = 12_000.0;
/// Lane width in dense fabric, metres (MODEL: Old Dhaka, Kowloon and Cairo lanes are 2 to 5 m).
pub const DENSE_LANE_M: f64 = 4.0;
/// Fewest storeys a dense-fabric building is drawn with (MODEL).
pub const DENSE_MIN_FLOORS: u8 = 3;

/// How far every building stays inside its leaf's sides, metres: half the widest road plus the
/// one-metre clearance a building keeps from any road.
pub const SIDE_INSET: f64 = MAIN_ROAD_M / 2.0 + 1.0;

/// A leaf's streets and buildings, in metres east and north of its centre.
#[derive(Debug, Clone, PartialEq)]
pub struct LeafLayout {
    /// The leaf.
    pub id: HolonId,
    /// Width, metres (east-west).
    pub w: f64,
    /// Height, metres (north-south).
    pub h: f64,
    /// Households (the leaf's budget, exactly).
    pub households: u64,
    /// People (from the context; 0 = not given): the focal world apportions them over the households.
    pub people: u64,
    /// People per km² the layout was classed at.
    pub density_km2: f64,
    /// Its street class.
    pub class: DensityClass,
    /// Streets.
    pub roads: Vec<Road>,
    /// Buildings (their `units` sum to `households`).
    pub buildings: Vec<Building>,
    /// Where streets leave the leaf.
    pub ports: Vec<Port>,
    /// True when no spot in the leaf clears its streets for even one small house.
    pub unplaced: bool,
}

impl LeafLayout {
    /// Leaf-local metres of an absolute lattice point (x in the leaf's column frame).
    #[must_use]
    pub fn to_m(id: HolonId, x: f64, y: f64) -> (f64, f64) {
        let (y0, x0, y1, x1) = id.bounds_units();
        let (xc, yc) = ((x0 + x1) as f64 / 2.0, (y0 + y1) as f64 / 2.0);
        let (lat, _) = id.centre_deg();
        let k = M_PER_DEG / UNITS_PER_DEG as f64;
        ((x - xc) * k * crate::detmath::cos_deg(lat).max(0.01), (y - yc) * k)
    }

    /// The street graph of the leaf: every pair of a north-south and an east-west road that meet
    /// is a junction (cross, tee or corner, by which of the two ends there); every road end that
    /// meets nothing is an exit on a side or a dead end inside.
    #[must_use]
    pub fn street_graph(&self) -> StreetGraph {
        const EPS: f64 = 1e-6;
        let within = |v: f64, a: f64, b: f64| v >= a.min(b) - EPS && v <= a.max(b) + EPS;
        let mut ends: Vec<[Option<JunctionKind>; 2]> = vec![[None, None]; self.roads.len()];
        let mut junctions = Vec::new();
        let vertical = |r: &Road| (r.x0 - r.x1).abs() < EPS;
        for (i, v) in self.roads.iter().enumerate() {
            if !vertical(v) {
                continue;
            }
            for (j, h) in self.roads.iter().enumerate() {
                if vertical(h) || (h.y0 - h.y1).abs() >= EPS {
                    continue;
                }
                let (px, py) = (v.x0, h.y0);
                if !within(py, v.y0, v.y1) || !within(px, h.x0, h.x1) {
                    continue;
                }
                let ev = [(py - v.y0).abs() < EPS, (py - v.y1).abs() < EPS];
                let eh = [(px - h.x0).abs() < EPS, (px - h.x1).abs() < EPS];
                let (v_end, h_end) = (ev[0] || ev[1], eh[0] || eh[1]);
                let kind = match (v_end, h_end) {
                    (false, false) => JunctionKind::Cross,
                    (true, true) => JunctionKind::Corner,
                    _ => JunctionKind::Tee,
                };
                for k in 0..2 {
                    if ev[k] {
                        ends[i][k] = Some(kind);
                    }
                    if eh[k] {
                        ends[j][k] = Some(kind);
                    }
                }
                junctions.push(Junction { x: px, y: py, kind, a: i as u32, b: j as u32 });
            }
        }
        let on_side = |x: f64, y: f64| (x.abs() - self.w / 2.0).abs() < EPS || (y.abs() - self.h / 2.0).abs() < EPS;
        let mut streets = Vec::with_capacity(self.roads.len());
        for (i, r) in self.roads.iter().enumerate() {
            let mut kinds = [JunctionKind::DeadEnd; 2];
            for (k, (x, y)) in [(r.x0, r.y0), (r.x1, r.y1)].into_iter().enumerate() {
                kinds[k] = ends[i][k].unwrap_or_else(|| {
                    let kind = if on_side(x, y) { JunctionKind::Exit } else { JunctionKind::DeadEnd };
                    junctions.push(Junction { x, y, kind, a: i as u32, b: NO_STREET });
                    kind
                });
            }
            streets.push(Street { leaf: self.id, index: i as u32, main: r.width >= MAIN_ROAD_M, ends: kinds });
        }
        StreetGraph { streets, junctions }
    }

    /// Give every building placed off a frontage (dense fabric, a block on a free spot) the street
    /// nearest its centre (ties to the lower index); a leaf with no road leaves them on none.
    fn link_streets(&mut self) {
        let roads = &self.roads;
        for b in self.buildings.iter_mut().filter(|b| b.street == NO_STREET) {
            let mut best = (f64::INFINITY, NO_STREET);
            for (i, r) in roads.iter().enumerate() {
                let (dx, dy) = (r.x1 - r.x0, r.y1 - r.y0);
                let l2 = dx * dx + dy * dy;
                let t = if l2 > 0.0 { (((b.cx - r.x0) * dx + (b.cy - r.y0) * dy) / l2).clamp(0.0, 1.0) } else { 0.0 };
                let (qx, qy) = (r.x0 + t * dx - b.cx, r.y0 + t * dy - b.cy);
                let d = qx * qx + qy * qy;
                if d < best.0 {
                    best = (d, i as u32);
                }
            }
            b.street = best.1;
        }
    }

    /// FNV-1a over every number in the layout: equal hashes mean byte-identical layouts.
    #[must_use]
    pub fn digest(&self) -> u64 {
        let mut h: u64 = 0xCBF2_9CE4_8422_2325;
        let mut eat = |b: u64| {
            for k in 0..8 {
                h ^= (b >> (8 * k)) & 0xFF;
                h = h.wrapping_mul(0x0100_0000_01B3);
            }
        };
        eat(self.id.pack());
        eat(self.households);
        for r in &self.roads {
            for v in [r.x0, r.y0, r.x1, r.y1, r.width] {
                eat(v.to_bits());
            }
        }
        for b in &self.buildings {
            for v in [b.cx, b.cy, b.w, b.d, b.angle] {
                eat(v.to_bits());
            }
            eat(u64::from(b.floors) << 16 | u64::from(b.units) | u64::from(b.street) << 32);
        }
        for p in &self.ports {
            eat(p.dir as u64 | (p.main as u64) << 8);
            eat(p.pos.to_bits());
        }
        h
    }

    /// Lay out a leaf from its context. A pure function of `(id, ctx, params)`.
    #[must_use]
    pub fn build(id: HolonId, ctx: &LeafContext, params: &MicroParams) -> Self {
        let (w, h) = id.size_m();
        let pph = params.get(P::PersonsPerHousehold).max(1.0);
        let area = (w * h / 1e6).max(1e-9);
        let density_km2 = ctx.budget as f64 * pph / area;
        let class = DensityClass::of(density_km2, params);
        let rng = Rng::new(id.seed());
        let mut out = Self { id, w, h, households: ctx.budget, people: ctx.people, density_km2, class, roads: Vec::new(), buildings: Vec::new(), ports: Vec::new(), unplaced: false };
        let (hw, hh) = (w / 2.0, h / 2.0);

        // ports: where streets cross each side (the shallower side's rule; see side_crossings)
        let (y0, x0, y1, x1) = id.bounds_units();
        for d in Dir::ALL {
            // the crossings are drawn along the shallower cell's whole side; this leaf keeps its stretch
            let (lo, hi) = if d.horizontal() { (x0 as f64, x1 as f64) } else { (y0 as f64, y1 as f64) };
            for (pos, main) in side_crossings(id, d, ctx) {
                if pos > lo && pos < hi {
                    out.ports.push(Port { dir: d, pos, main });
                }
            }
        }
        out.ports.sort_by(|a, b| a.dir.cmp(&b.dir).then(a.pos.total_cmp(&b.pos)));

        // streets: the leaf's own lines, run on to a side where that side has a port there, and a
        // stub in from every port that is not on a line; the grid closes over all of them
        let width = match class {
            DensityClass::UrbanCore if density_km2 >= DENSE_FABRIC_KM2 => DENSE_LANE_M,
            DensityClass::UrbanCore | DensityClass::Urban => 8.0,
            DensityClass::Suburban => 6.0,
            DensityClass::Village => 5.0,
        };
        let has = |dir: Dir, v: f64| out.ports.iter().any(|p| p.dir == dir && p.pos.to_bits() == v.to_bits());
        let mut xs: Vec<Line> = cell_lines(id, &ctx.chain, true);
        let mut ys: Vec<Line> = cell_lines(id, &ctx.chain, false);
        for p in &out.ports {
            let v = if p.dir.horizontal() { &mut xs } else { &mut ys };
            if !v.iter().any(|q| q.0.to_bits() == p.pos.to_bits()) {
                v.push((p.pos, p.main));
            }
        }
        xs.sort_by(|a, b| a.0.total_cmp(&b.0));
        ys.sort_by(|a, b| a.0.total_cmp(&b.0));
        let xm: Vec<f64> = xs.iter().map(|&(x, _)| Self::to_m(id, x, 0.0).0).collect();
        let ym: Vec<f64> = ys.iter().map(|&(y, _)| Self::to_m(id, 0.0, y).1).collect();
        let wd = |main: bool| if main { MAIN_ROAD_M } else { width };
        let mut roads = Vec::new();
        for (k, &(x, main)) in xs.iter().enumerate() {
            let top = if has(Dir::N, x) { Some(hh) } else { ym.last().copied() };
            let bot = if has(Dir::S, x) { Some(-hh) } else { ym.first().copied() };
            let (top, bot) = match (top, bot) {
                (Some(a), Some(b)) => (a, b),
                (Some(a), None) => (a, 0.0),
                (None, Some(b)) => (0.0, b),
                (None, None) => continue,
            };
            if top - bot > 1e-9 {
                roads.push(Road { x0: xm[k], y0: bot, x1: xm[k], y1: top, width: wd(main) });
            }
        }
        for (k, &(y, main)) in ys.iter().enumerate() {
            let east = if has(Dir::E, y) { Some(hw) } else { xm.last().copied() };
            let west = if has(Dir::W, y) { Some(-hw) } else { xm.first().copied() };
            let (east, west) = match (east, west) {
                (Some(a), Some(b)) => (a, b),
                (Some(a), None) => (a, 0.0),
                (None, Some(b)) => (0.0, b),
                (None, None) => continue,
            };
            if east - west > 1e-9 {
                roads.push(Road { x0: west, y0: ym[k], x1: east, y1: ym[k], width: wd(main) });
            }
        }
        let mut rng = rng;
        out.roads = roads;
        if class == DensityClass::UrbanCore && density_km2 >= DENSE_FABRIC_KM2 {
            out.place_dense_fabric(&mut rng);
        } else {
            out.place_buildings(&mut rng);
        }
        out.fit_households(&mut rng);
        out.link_streets();
        out
    }

    /// Dense fabric (see [`DENSE_FABRIC_KM2`]): plots of 8 to 13 m by 8 to 12 m on a grid that
    /// fills the leaf, 1 to 2.5 m apart, wherever they clear the lanes; then the leaf's households
    /// are shared evenly over them and each building gets the storeys its flats need.
    fn place_dense_fabric(&mut self, rng: &mut Rng) {
        if self.households == 0 {
            return;
        }
        // a side keeps the full SIDE_INSET only where a main road crosses it (a neighbour's main road
        // may run beside it); elsewhere plots come to a lane's half-width of the leaf's edge
        let inset = |dir: Dir| if self.ports.iter().any(|p| p.dir == dir && p.main) { SIDE_INSET } else { DENSE_LANE_M / 2.0 + 0.5 };
        let (x0, x1) = (-self.w / 2.0 + inset(Dir::W), self.w / 2.0 - inset(Dir::E));
        let (y0, y1) = (-self.h / 2.0 + inset(Dir::S), self.h / 2.0 - inset(Dir::N));
        let fits = |l: &Self, b: &Building| {
            b.cx - b.w / 2.0 >= x0 && b.cx + b.w / 2.0 <= x1 && b.cy - b.d / 2.0 >= y0 && b.cy + b.d / 2.0 <= y1
                && !l.roads.iter().any(|r| footprint_meets_road(b, r, 0.5))
                && !l.buildings.iter().any(|o| rects_overlap(b, o, 0.6))
        };
        // a plot that meets a lane slides on a metre and tries again, and a row that held none moves
        // up a metre, so the plots pack up to every lane rather than skipping the strip beside it
        // (Dhaka's centre leaf held one building when a miss skipped a whole plot)
        // and a plot or row that does not fit at its drawn size tries the smallest size first
        let mut y = y0;
        while y < y1 {
            let drawn = rng.range(8.0, 12.0);
            let mut placed = false;
            let mut row_d = drawn;
            for d in [drawn, 8.0] {
                let mut x = x0;
                while x < x1 {
                    let drawn_w = rng.range(8.0, 13.0);
                    let at = |w: f64| Building { cx: x + w / 2.0, cy: y + d / 2.0, w, d, angle: 0.0, floors: DENSE_MIN_FLOORS, units: 1, street: NO_STREET };
                    let hit = [drawn_w, 8.0].into_iter().map(at).find(|b| fits(self, b));
                    if let Some(b) = hit {
                        x += b.w + rng.range(1.0, 2.5);
                        self.buildings.push(b);
                        placed = true;
                    } else {
                        x += 1.0;
                    }
                }
                row_d = d;
                if placed {
                    break;
                }
            }
            y += if placed { row_d + rng.range(1.0, 2.5) } else { 1.0 };
        }
        if self.buildings.is_empty() {
            return; // fit_households places one block on the first free spot
        }
        let n = self.buildings.len() as u64;
        let (each, extra) = (self.households / n, self.households % n);
        let mut k = 0;
        self.buildings.retain(|_| {
            k += 1;
            each > 0 || (k as u64) <= extra
        });
        for (k, b) in self.buildings.iter_mut().enumerate() {
            let units = each + u64::from((k as u64) < extra);
            b.units = units.min(u64::from(u16::MAX)) as u16;
            let per_floor = (b.w / 6.0).round().max(1.0) as u64;
            b.floors = (units.div_ceil(per_floor).min(60) as u8).max(DENSE_MIN_FLOORS);
        }
    }

    fn fits(&self, b: &Building) -> bool {
        // the footprint's corners stay SIDE_INSET inside the leaf: no building meets a neighbour's,
        // and none meets a street of the neighbour's that runs beside the shared side (a street
        // this leaf cannot see, whose paving reaches at most half a main road's width across)
        let (ca, sa) = (crate::detmath::cos(b.angle), crate::detmath::sin(b.angle));
        for (u, v) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            let (px, py) = (b.cx + ca * u * b.w / 2.0 - sa * v * b.d / 2.0, b.cy + sa * u * b.w / 2.0 + ca * v * b.d / 2.0);
            if px.abs() > self.w / 2.0 - SIDE_INSET || py.abs() > self.h / 2.0 - SIDE_INSET {
                return false;
            }
        }
        // the whole footprint clears every road's paving by a metre (main roads included)
        if self.roads.iter().any(|r| footprint_meets_road(b, r, 1.0)) {
            return false;
        }
        !self.buildings.iter().any(|o| rects_overlap(b, o, 0.6))
    }

    fn place_buildings(&mut self, rng: &mut Rng) {
        if self.households == 0 {
            return;
        }
        let roads = self.roads.clone();
        for (ri, r) in roads.iter().enumerate() {
            let (dx, dy) = (r.x1 - r.x0, r.y1 - r.y0);
            let len = (dx * dx + dy * dy).sqrt();
            if len < 1.0 {
                continue;
            }
            let (ux, uy) = (dx / len, dy / len);
            let angle = uy.atan2(ux);
            for side in [-1.0f64, 1.0] {
                let mut s = rng.range(2.0, 10.0);
                while s < len {
                    let (w, d, floors, units, gap, setback) = match self.class {
                        DensityClass::UrbanCore => {
                            let w = rng.range(16.0, 28.0);
                            let floors = 4 + rng.below(4) as u8;
                            let per = (w / 9.0).round().max(1.0) as u16;
                            (w, rng.range(11.0, 15.0), floors, per * u16::from(floors), rng.range(0.5, 3.0), 1.5)
                        }
                        DensityClass::Urban => {
                            let floors = 2 + rng.below(2) as u8;
                            (rng.range(6.5, 9.0), rng.range(8.5, 11.0), floors, 1 + rng.below(2) as u16, 0.4, 3.0)
                        }
                        DensityClass::Suburban => (rng.range(9.5, 13.0), rng.range(8.5, 12.0), 1 + rng.below(2) as u8, 1, rng.range(9.0, 15.0), rng.range(7.0, 11.0)),
                        DensityClass::Village => {
                            if rng.chance(0.18) {
                                (rng.range(14.0, 22.0), rng.range(10.0, 14.0), 1, 0, rng.range(12.0, 40.0), rng.range(8.0, 14.0))
                            } else {
                                (rng.range(8.5, 13.0), rng.range(8.0, 11.0), 1 + rng.below(2) as u8, 1, rng.range(14.0, 50.0), rng.range(6.0, 13.0))
                            }
                        }
                    };
                    let mid = s + w / 2.0;
                    if mid > len {
                        break;
                    }
                    let off = r.width / 2.0 + setback + d / 2.0;
                    let (px, py) = (r.x0 + ux * mid, r.y0 + uy * mid);
                    let b = Building { cx: px - uy * off * side, cy: py + ux * off * side, w, d, angle, floors, units, street: ri as u32 };
                    if self.fits(&b) {
                        self.buildings.push(b);
                    }
                    s += w + gap;
                }
            }
        }
    }

    /// The first spot a block fits, scanning a grid over the leaf nearest the centre first and
    /// shrinking the block from `full` down to one small house.
    fn free_spot(&self, full: f64) -> Option<Building> {
        let mut side = full;
        loop {
            let (w, d) = (side, (side * 0.7).max(5.0));
            let step = (side / 2.0).max(2.0);
            let (nx, ny) = ((self.w / 2.0 / step).floor() as i64, (self.h / 2.0 / step).floor() as i64);
            let mut cands: Vec<(f64, f64)> = Vec::new();
            for i in -nx..=nx {
                for j in -ny..=ny {
                    cands.push((i as f64 * step, j as f64 * step));
                }
            }
            cands.sort_by(|a, b| (a.0 * a.0 + a.1 * a.1).total_cmp(&(b.0 * b.0 + b.1 * b.1)).then(a.0.total_cmp(&b.0)).then(a.1.total_cmp(&b.1)));
            for (cx, cy) in cands {
                for angle in [0.0, std::f64::consts::FRAC_PI_2] {
                    let b = Building { cx, cy, w, d, angle, floors: 1, units: 0, street: NO_STREET };
                    if self.fits(&b) {
                        return Some(b);
                    }
                }
            }
            if side <= 6.0 {
                return None;
            }
            side = (side * 0.7).max(6.0);
        }
    }

    /// Make the dwellings equal the budget exactly: flats gain or lose units, houses beyond the
    /// budget become empty lots, and a budget the streets cannot hold stacks into the largest blocks.
    fn fit_households(&mut self, rng: &mut Rng) {
        let want = self.households;
        let mut have: u64 = self.buildings.iter().map(|b| u64::from(b.units)).sum();
        if have > want {
            let flats = matches!(self.class, DensityClass::UrbanCore | DensityClass::Urban);
            while have > want {
                let homes: Vec<usize> = (0..self.buildings.len()).filter(|&k| self.buildings[k].units > 0).collect();
                let k = homes[rng.below(homes.len())];
                if flats && self.buildings[k].units > 1 {
                    self.buildings[k].units -= 1;
                    have -= 1;
                } else {
                    have -= u64::from(self.buildings[k].units);
                    self.buildings.remove(k);
                }
            }
            // removing a whole house can undershoot: top back up below
        }
        if have < want {
            if !self.buildings.iter().any(|b| b.units > 0) {
                // no street frontage held a home: one block on the first free spot, nearest the
                // centre first, shrinking until it fits between the streets
                let full = ((want as f64).sqrt() * 6.0).clamp(10.0, (self.w.min(self.h) - 4.0).max(4.0));
                let spot = self.free_spot(full);
                self.unplaced = spot.is_none();
                self.buildings.push(spot.unwrap_or(Building { cx: 0.0, cy: 0.0, w: 6.0, d: 5.0, angle: 0.0, floors: 1, units: 0, street: NO_STREET }));
                let last = self.buildings.len() - 1;
                self.buildings[last].units = 1;
                have += 1;
            }
            let mut homes: Vec<usize> = (0..self.buildings.len()).filter(|&k| self.buildings[k].units > 0).collect();
            homes.sort_by(|&a, &b| {
                let (x, y) = (&self.buildings[a], &self.buildings[b]);
                (y.w * y.d).total_cmp(&(x.w * x.d)).then(a.cmp(&b))
            });
            let mut k = 0;
            while have < want {
                let b = &mut self.buildings[homes[k % homes.len()]];
                b.units += 1;
                let per = ((b.w / 9.0).round().max(1.0)) as u16;
                b.floors = b.floors.max(b.units.div_ceil(per).min(60) as u8);
                have += 1;
                k += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_leaf_holds_a_street_graph_and_every_household_a_street() {
        let p = MicroParams::default();
        let mut kinds = [0usize; 5];
        for &(la, lo, dens) in &[(47.3769, 8.5417, 2_500.0), (39.2, -77.25, 1_100.0), (46.3, 2.3, 60.0), (30.0444, 31.2357, 37_000.0), (23.7104, 90.4074, 30_000.0)] {
            for level in [5_u8, 6, 7] {
                let id = HolonId::at(level, la, lo);
                let (w, h) = id.size_m();
                let ctx = LeafContext { budget: ((dens * w * h / 1e6) / 2.5).round().clamp(1.0, 400.0) as u64, chain: [3_000; 11], ..LeafContext::default() };
                let l = LeafLayout::build(id, &ctx, &p);
                let g = l.street_graph();
                assert_eq!(g.streets.len(), l.roads.len());
                // every street end is exactly one junction's: a meeting names it, else an exit or a dead end
                for s in &g.streets {
                    assert_eq!(s.leaf, id);
                    let named = g.junctions.iter().filter(|j| j.a == s.index || j.b == s.index).count();
                    let open = s.ends.iter().filter(|k| matches!(k, JunctionKind::Exit | JunctionKind::DeadEnd)).count();
                    assert!(named >= open, "{id:?} street {}", s.index);
                }
                for j in &g.junctions {
                    kinds[j.kind as usize] += 1;
                    assert_eq!(j.b == NO_STREET, matches!(j.kind, JunctionKind::Exit | JunctionKind::DeadEnd));
                }
                // buildings with dwellings all sit on a street when the leaf has one; summed by street, the households
                if !l.roads.is_empty() {
                    assert!(l.buildings.iter().all(|b| (b.street as usize) < l.roads.len()), "{id:?}");
                }
                let by_street: u64 = l.buildings.iter().map(|b| u64::from(b.units)).sum();
                assert_eq!(by_street, l.households, "{id:?}");
            }
        }
        assert!(kinds[JunctionKind::Cross as usize] + kinds[JunctionKind::Tee as usize] > 0, "{kinds:?}");
    }

    #[test]
    fn betweenness_is_brandes_on_a_known_graph_and_gini_is_bounded() {
        // a plus sign: two crossing streets of 20 m make 4 segments of 10 m meeting at one cross;
        // every pair of the 5 nodes takes one path, so each arm carries the 4 pairs through its end
        let id = HolonId::at(9, 47.3769, 8.5417);
        let (w, h) = id.size_m();
        let mut l = LeafLayout { id, w, h, households: 0, people: 0, density_km2: 0.0, class: DensityClass::Village, roads: Vec::new(), buildings: Vec::new(), ports: Vec::new(), unplaced: false };
        l.roads = vec![Road { x0: 0.0, y0: -10.0, x1: 0.0, y1: 10.0, width: 5.0 }, Road { x0: -10.0, y0: 0.0, x1: 10.0, y1: 0.0, width: 5.0 }];
        let g = l.street_graph();
        assert_eq!(g.junctions.iter().filter(|j| j.kind == JunctionKind::Cross).count(), 1);
        let (n, segs) = g.segments(&l.roads);
        assert_eq!((n, segs.len()), (5, 4));
        assert_eq!(g.segment_betweenness(&l.roads), vec![4.0; 4]);
        assert!(g.betweenness_gini(&l.roads).abs() < 1e-12, "a symmetric plus spreads evenly");
        // a comb: one spine with teeth concentrates paths on the spine
        l.roads = vec![Road { x0: -30.0, y0: 0.0, x1: 30.0, y1: 0.0, width: 5.0 }];
        for x in [-20.0, 0.0, 20.0] {
            l.roads.push(Road { x0: x, y0: 0.0, x1: x, y1: 10.0, width: 5.0 });
        }
        let gc = l.street_graph().betweenness_gini(&l.roads);
        assert!(gc > 0.1 && gc < 1.0, "comb gini {gc}");
    }

    #[test]
    fn arterial_crossings_sum_along_a_side_and_agree_across_it() {
        // households per L2 cell from a smooth field (any budget will do: the law is structural)
        let mut l2 = |c: HolonId| -> u64 { 2_000 + (c.pack() % 97) * 300 };
        let mut checked = (0, 0);
        for &(la, lo) in &[(47.3769, 8.5417), (30.0444, 31.2357), (23.7104, 90.4074), (40.7831, -73.9712)] {
            for level in 2..=6 {
                let p = HolonId::at(level, la, lo);
                for s in split_crossings(p, &mut l2) {
                    assert_eq!(s.children.iter().sum::<u64>(), s.parent, "{p:?} side {:?}", s.side);
                    let peer = s.peer.expect("not at a pole");
                    assert_eq!(arterial_crossings(peer, s.side.flip(), &mut l2), s.parent, "{p:?} and {peer:?} disagree");
                    checked.0 += 1;
                    checked.1 += s.parent;
                }
            }
        }
        assert!(checked.1 > 0, "some side must carry an arterial ({checked:?})");
    }

    #[test]
    fn dense_fabric_fills_the_leaf_and_storeys_follow_density() {
        // a leaf holding 60 households at 40,000 and at 10,000 people per km2
        let p = MicroParams::default();
        let id = HolonId::at(9, 23.7, 90.4);
        let (w, h) = id.size_m();
        let area = w * h / 1e6;
        let pph = p.get(P::PersonsPerHousehold);
        let mk = |dens: f64| { let ctx = LeafContext { budget: (dens * area / pph).round() as u64, ..LeafContext::default() }; LeafLayout::build(id, &ctx, &p) };
        let dense = mk(80_000.0);
        assert!(dense.density_km2 >= DENSE_FABRIC_KM2, "{}", dense.density_km2);
        let units: u64 = dense.buildings.iter().map(|b| u64::from(b.units)).sum();
        assert_eq!(units, dense.households, "every household has a flat");
        assert!(dense.buildings.iter().all(|b| b.floors >= DENSE_MIN_FLOORS));
        assert!(dense.roads.iter().all(|r| r.width <= DENSE_LANE_M || r.width >= MAIN_ROAD_M), "lanes, not 8 m streets");
        let denser = mk(160_000.0);
        let mean = |l: &LeafLayout| l.buildings.iter().map(|b| f64::from(b.floors)).sum::<f64>() / l.buildings.len().max(1) as f64;
        assert!(mean(&denser) > mean(&dense), "storeys follow density: {} vs {}", mean(&denser), mean(&dense));
    }

    /// A synthetic world for the witnesses: land everywhere except a sea patch, people dense
    /// around a "town" and thin elsewhere (no real place: the inputs are made up here).
    fn tree(target: u64) -> HolonTree<impl Fn(HolonId) -> u64, impl Fn(HolonId) -> u32> {
        let root = |id: HolonId| {
            let (lat, lon) = id.centre_deg();
            let town = (-((lat - 10.0).powi(2) + (lon - 20.0).powi(2)) / 2.0).exp();
            let edge = (-((lat - 10.0).powi(2) + (lon - 179.0).powi(2)) / 2.0).exp() + (-((lat - 10.0).powi(2) + (lon + 179.0).powi(2)) / 2.0).exp();
            ((town + edge) * 4.0e6 + 2.0e4) as u64
        };
        let ext = |id: HolonId| {
            let (lat, lon) = id.centre_deg();
            if lat > 10.4 && lon > 20.4 && lon < 20.7 {
                0 // sea
            } else {
                let d = ((lat - 10.0).powi(2) + (lon - 20.0).powi(2)).sqrt().min(((lat - 10.0).powi(2) + (lon.abs() - 179.0).powi(2)).sqrt());
                (1000.0 * (-d * 20.0).exp() + 50.0) as u32
            }
        };
        HolonTree::new(root, ext, target)
    }

    fn params() -> MicroParams {
        MicroParams::default()
    }

    #[test]
    fn ids_round_trip_and_wrap() {
        for lvl in 0..=MAX_LEVEL {
            for &(lat, lon) in &[(0.0, 0.0), (-0.001, -0.001), (89.999, 179.999), (-90.0, -180.0), (47.3769, 8.5417), (-33.9, 151.2), (10.0, 180.0), (10.0, -180.0)] {
                let id = HolonId::at(lvl, lat, lon);
                assert_eq!(HolonId::unpack(id.pack()), Some(id));
                assert!(id.pack() < 1 << 51, "fits a JavaScript number exactly");
                // membership is decided on the quantised point floor(deg × 25600), as in the page: a
                // decimal like 151.2 whose double sits a hair below the lattice line belongs below it
                let (y0, x0, y1, x1) = id.bounds_units();
                let y = ((lat * UNITS_PER_DEG as f64).floor() as i64).clamp(-LAT_UNITS_HALF, LAT_UNITS_HALF - 1);
                let x = ((lon * UNITS_PER_DEG as f64).floor() as i64 + LON_UNITS / 2).rem_euclid(LON_UNITS) - LON_UNITS / 2;
                assert!(y >= y0 && y < y1, "{lvl} {lat} in [{y0},{y1})");
                assert!(x >= x0 && x < x1, "{lvl} {lon} in [{x0},{x1})");
            }
        }
        // 180°E and 180°W are the same column
        assert_eq!(HolonId::at(5, 10.0, 180.0), HolonId::at(5, 10.0, -180.0));
        // east of the last column is the first
        let last = HolonId::at(L3, 10.0, 179.995);
        assert_eq!(last.neighbour(Dir::E), Some(HolonId::at(L3, 10.0, -179.995)));
        // past a pole there is no neighbour
        assert_eq!(HolonId::at(0, 89.0, 0.0).neighbour(Dir::N), None);
        // L3 is the page's tile: (ti, tj) = floor(lat / 0.02), floor(lon / 0.02)
        let t = HolonId::at(L3, 47.3769, 8.5417);
        assert_eq!((t.i, t.j), ((47.3769f64 / 0.02).floor() as i32, (8.5417f64 / 0.02).floor() as i32));
    }

    #[test]
    fn parents_and_children_agree() {
        for lvl in 1..=MAX_LEVEL {
            let id = HolonId::at(lvl, -12.345, 179.99);
            let p = id.parent().unwrap();
            assert!(p.children().contains(&id), "level {lvl}");
            for c in p.children() {
                assert_eq!(c.parent(), Some(p));
            }
            assert_eq!(HolonId::at(lvl - 1, -12.345, 179.99), p);
        }
    }

    /// The vectors the page's `docs/holon.js` is checked against (tools/holon/id_test.mjs).
    #[test]
    fn id_vectors_match_the_page() {
        let tsv = include_str!("../../../tools/holon/id_vectors.tsv");
        let mut n = 0;
        for line in tsv.lines().filter(|l| !l.starts_with('#') && !l.is_empty()) {
            let f: Vec<&str> = line.split('\t').collect();
            let (lvl, lat, lon, want): (u8, f64, f64, u64) = (f[0].parse().unwrap(), f[1].parse().unwrap(), f[2].parse().unwrap(), f[3].parse().unwrap());
            assert_eq!(HolonId::at(lvl, lat, lon).pack(), want, "{line}");
            n += 1;
        }
        assert!(n >= 20);
    }

    #[test]
    fn apportion_is_exact() {
        assert_eq!(apportion(10, &[1, 1, 1]), vec![4, 3, 3]);
        assert_eq!(apportion(0, &[5, 7]), vec![0, 0]);
        assert_eq!(apportion(7, &[0, 0]), vec![4, 3]);
        assert_eq!(apportion(1_000_003, &[3, 0, 9, 1]).iter().sum::<u64>(), 1_000_003);
        assert_eq!(apportion(5, &[0, 2, 0]), vec![0, 5, 0]);
    }

    #[test]
    fn noise_is_continuous_and_wraps() {
        let p = STEP[3];
        // across a lattice corner the field moves by at most one step's worth
        for x in -2000..2000 {
            let a = value_noise(x, 77, p, 1) as i64;
            let b = value_noise(x + 1, 77, p, 1) as i64;
            assert!((a - b).abs() <= 8, "x={x} {a} {b}");
        }
        assert_eq!(value_noise(-LON_UNITS / 2, 500, p, 9), value_noise(LON_UNITS / 2, 500, p, 9));
    }

    /// Witness: children always sum to their parent, from L0 down to the leaves.
    #[test]
    fn parent_sums_hold() {
        let mut t = tree(60);
        let mut stack = vec![HolonId::at(0, 10.0, 20.0), HolonId::at(0, 10.0, 179.5)];
        let mut leaves = 0;
        let mut checked = 0;
        while let Some(id) = stack.pop() {
            let b = t.budget(id);
            if is_leaf(id, b, 60) {
                leaves += 1;
                continue;
            }
            let kids = id.children();
            let sum: u64 = kids.iter().map(|&c| t.budget(c)).sum();
            assert_eq!(sum, b, "{id:?}");
            checked += 1;
            // follow the densest child and one more, so the walk reaches the deep levels
            let mut ks: Vec<(u64, HolonId)> = kids.iter().map(|&c| (t.budget(c), c)).collect();
            ks.sort();
            stack.extend(ks.iter().rev().take(2).map(|k| k.1));
        }
        assert!(checked > 20 && leaves > 2, "{checked} {leaves}");
    }

    /// Witness: depth is a pure function of the budget, and leaves hold no more than the target.
    #[test]
    fn depth_follows_budget() {
        let mut t = tree(60);
        let town = t.leaf_at(10.01, 20.01);
        let country = t.leaf_at(10.0 + 7.0, 20.0 + 7.0);
        assert!(town.level > country.level, "{town:?} {country:?}");
        assert!(t.budget(town) <= 60);
        assert!(town.level > L3 + 2);
        let parent_b = t.budget(town.parent().unwrap());
        assert!(parent_b > 60);
    }

    /// Witness: generate, evict, regenerate gives the identical layout.
    #[test]
    fn evict_and_regenerate_is_identical() {
        let mut t = tree(60);
        let p = params();
        let leaf = t.leaf_at(10.013, 20.017);
        let a = LeafLayout::build(leaf, &t.context(leaf), &p);
        t.evict(None);
        assert_eq!(t.cached(), 0);
        let b = LeafLayout::build(leaf, &t.context(leaf), &p);
        assert_eq!(a.digest(), b.digest());
        assert_eq!(a, b);
        assert!(!a.roads.is_empty() && !a.buildings.is_empty());
    }

    /// Witness: the order cells are generated in changes nothing (A then B vs B then A).
    #[test]
    fn generation_order_is_irrelevant() {
        let p = params();
        let mut t1 = tree(60);
        let a1 = t1.leaf_at(10.05, 20.05);
        let b1 = t1.leaf_at(9.93, 19.91);
        let la1 = LeafLayout::build(a1, &t1.context(a1), &p);
        let lb1 = LeafLayout::build(b1, &t1.context(b1), &p);
        let mut t2 = tree(60);
        let b2 = t2.leaf_at(9.93, 19.91);
        let lb2 = LeafLayout::build(b2, &t2.context(b2), &p);
        let a2 = t2.leaf_at(10.05, 20.05);
        let la2 = LeafLayout::build(a2, &t2.context(a2), &p);
        assert_eq!((a1, b1), (a2, b2));
        assert_eq!(la1.digest(), la2.digest());
        assert_eq!(lb1.digest(), lb2.digest());
    }

    /// Witness: a zoom out to L0 and back in lands on the byte-identical leaf.
    #[test]
    fn zoom_round_trip_is_identical() {
        let p = params();
        let mut t = tree(60);
        let leaf = t.leaf_at(10.002, 20.003);
        let before = LeafLayout::build(leaf, &t.context(leaf), &p);
        // zoom out: everything below L1 is dropped, other regions are visited
        t.evict(None);
        let _ = t.leaf_at(12.5, 24.5);
        let _ = t.leaf_at(10.0, -179.9);
        let back = t.leaf_at(10.002, 20.003);
        assert_eq!(back, leaf);
        let after = LeafLayout::build(back, &t.context(back), &p);
        assert_eq!(before.digest(), after.digest());
    }

    /// Witness: the leaf's households equal its budget exactly.
    #[test]
    fn layout_holds_the_budget() {
        let p = params();
        let mut t = tree(60);
        for &(lat, lon) in &[(10.0, 20.0), (10.3, 20.2), (12.0, 23.0), (16.0, 26.0), (10.0, 179.995)] {
            let leaf = t.leaf_at(lat, lon);
            let l = LeafLayout::build(leaf, &t.context(leaf), &p);
            let units: u64 = l.buildings.iter().map(|b| u64::from(b.units)).sum();
            assert_eq!(units, t.budget(leaf), "{leaf:?}");
        }
    }

    fn on_side(l: &LeafLayout, d: Dir, x: f64, y: f64) -> bool {
        let e = 1e-6;
        match d {
            Dir::N => (y - l.h / 2.0).abs() < e,
            Dir::S => (y + l.h / 2.0).abs() < e,
            Dir::E => (x - l.w / 2.0).abs() < e,
            Dir::W => (x + l.w / 2.0).abs() < e,
        }
    }

    /// Every road stays inside its leaf, and every road end on a side sits on one of its ports.
    fn check_inside(l: &LeafLayout) {
        for r in &l.roads {
            for (x, y) in [(r.x0, r.y0), (r.x1, r.y1)] {
                assert!(x.abs() <= l.w / 2.0 + 1e-6 && y.abs() <= l.h / 2.0 + 1e-6, "{:?} road leaves the leaf: {x} {y}", l.id);
                for d in Dir::ALL {
                    if on_side(l, d, x, y) {
                        let pos_m = |p: &Port| if d.horizontal() { LeafLayout::to_m(l.id, p.pos, 0.0).0 } else { LeafLayout::to_m(l.id, 0.0, p.pos).1 };
                        let at = if d.horizontal() { x } else { y };
                        assert!(l.ports.iter().any(|p| p.dir == d && (pos_m(p) - at).abs() < 1e-6), "{:?} road meets side {d:?} at {at} with no port", l.id);
                    }
                }
            }
        }
    }

    /// Witness: neighbouring leaves (of equal or different depth) meet with the same crossings,
    /// so no street runs into a neighbour that has no street there.
    #[test]
    fn neighbours_meet_at_the_same_crossings() {
        let p = params();
        let mut t = tree(60);
        let mut pairs = 0;
        let mut mixed = 0;
        let mut ports = 0;
        let mut arterials = 0;
        // walk a strip of leaves across the town's edge (depths change along it) and over the
        // antimeridian
        for &(lat, lon0, lon1) in &[(10.011, 19.95, 20.08), (10.311, 20.1, 20.45), (10.011, 179.97, 180.03)] {
            let mut lon = lon0;
            while lon < lon1 {
                let a = t.leaf_at(lat, lon);
                let la = LeafLayout::build(a, &t.context(a), &p);
                check_inside(&la);
                for d in Dir::ALL {
                    let Some(nb) = a.neighbour(d) else { continue };
                    // every leaf across this side
                    let (y0, x0, y1, x1) = a.bounds_units();
                    let probe: Vec<(f64, f64)> = (0..16)
                        .map(|k| {
                            let f = (k as f64 + 0.5) / 16.0;
                            let (u, v) = match d {
                                Dir::N => (x0 as f64 + f * (x1 - x0) as f64, y1 as f64 + 0.5),
                                Dir::S => (x0 as f64 + f * (x1 - x0) as f64, y0 as f64 - 0.5),
                                Dir::E => (x1 as f64 + 0.5, y0 as f64 + f * (y1 - y0) as f64),
                                Dir::W => (x0 as f64 - 0.5, y0 as f64 + f * (y1 - y0) as f64),
                            };
                            (v / UNITS_PER_DEG as f64, u / UNITS_PER_DEG as f64)
                        })
                        .collect();
                    let _ = nb;
                    let mut seen = Vec::new();
                    for (plat, plon) in probe {
                        let b = t.leaf_at(plat, plon);
                        if seen.contains(&b) {
                            continue;
                        }
                        seen.push(b);
                        let lb = LeafLayout::build(b, &t.context(b), &p);
                        // the shared stretch of the side, in this side's position frame
                        let (by0, bx0, by1, bx1) = b.bounds_units();
                        let (lo, hi) = if d.horizontal() { (x0.max(bx0) as f64, x1.min(bx1) as f64) } else { (y0.max(by0) as f64, y1.min(by1) as f64) };
                        let mine: Vec<(u64, bool)> = la.ports.iter().filter(|q| q.dir == d && q.pos > lo && q.pos < hi).map(|q| (q.pos.to_bits(), q.main)).collect();
                        let theirs: Vec<(u64, bool)> = lb.ports.iter().filter(|q| q.dir == d.flip() && q.pos > lo && q.pos < hi).map(|q| (q.pos.to_bits(), q.main)).collect();
                        assert_eq!(mine, theirs, "{a:?} {d:?} vs {b:?}");
                        pairs += 1;
                        ports += mine.len();
                        arterials += mine.iter().filter(|m| m.1).count();
                        if a.level != b.level {
                            mixed += 1;
                        }
                    }
                }
                let (_, _, _, x1) = a.bounds_units();
                lon = (x1 as f64 / UNITS_PER_DEG as f64) + 1e-7 + if lon >= 180.0 { 360.0 } else { 0.0 };
                if a.j < 0 && lon < 0.0 && lon0 > 0.0 {
                    lon += 360.0;
                }
            }
        }
        assert!(pairs > 100 && mixed > 10 && ports > 50 && arterials > 0, "pairs {pairs} mixed {mixed} ports {ports} arterials {arterials}");
    }

    /// The leaves along the three witness strips: across the dense town, across the town's edge
    /// (depths change along it), and over the antimeridian.
    fn strip_leaves<R: Fn(HolonId) -> u64, E: Fn(HolonId) -> u32>(t: &mut HolonTree<R, E>) -> Vec<HolonId> {
        let mut out = Vec::new();
        for &(lat, lon0, lon1) in &[(10.011, 19.95, 20.08), (10.311, 20.1, 20.45), (10.011, 179.97, 180.03)] {
            let mut lon = lon0;
            while lon < lon1 {
                let a = t.leaf_at(lat, lon);
                out.push(a);
                let (_, _, _, x1) = a.bounds_units();
                lon = (x1 as f64 / UNITS_PER_DEG as f64) + 1e-7 + if lon >= 180.0 { 360.0 } else { 0.0 };
                if a.j < 0 && lon < 0.0 && lon0 > 0.0 {
                    lon += 360.0;
                }
            }
        }
        out
    }

    /// A road of leaf `b`, moved into leaf `a`'s metre frame (through the lattice, wrapping the
    /// antimeridian).
    fn road_in(a: HolonId, b: HolonId, r: &Road) -> Road {
        let k = M_PER_DEG / UNITS_PER_DEG as f64;
        let units = |id: HolonId, x: f64, y: f64| {
            let (y0, x0, y1, x1) = id.bounds_units();
            let (xc, yc) = ((x0 + x1) as f64 / 2.0, (y0 + y1) as f64 / 2.0);
            let (lat, _) = id.centre_deg();
            (xc + x / (k * crate::detmath::cos_deg(lat).max(0.01)), yc + y / k)
        };
        let (_, ax0, _, ax1) = a.bounds_units();
        let axc = (ax0 + ax1) as f64 / 2.0;
        let wrap = |x: f64| {
            let span = LON_UNITS as f64;
            x - span * ((x - axc) / span).round()
        };
        let (u0, v0) = units(b, r.x0, r.y0);
        let (u1, v1) = units(b, r.x1, r.y1);
        let (x0, y0) = LeafLayout::to_m(a, wrap(u0), v0);
        let (x1, y1) = LeafLayout::to_m(a, wrap(u1), v1);
        Road { x0, y0, x1, y1, width: r.width }
    }

    /// The leaves touching `a` across a side or a corner (probed just outside it).
    fn touching<R: Fn(HolonId) -> u64, E: Fn(HolonId) -> u32>(t: &mut HolonTree<R, E>, a: HolonId) -> Vec<HolonId> {
        let (y0, x0, y1, x1) = a.bounds_units();
        let (y0, x0, y1, x1) = (y0 as f64, x0 as f64, y1 as f64, x1 as f64);
        let mut pts = Vec::new();
        for k in 0..16 {
            let f = (k as f64 + 0.5) / 16.0;
            let (u, v) = (x0 + f * (x1 - x0), y0 + f * (y1 - y0));
            pts.extend([(u, y1 + 0.5), (u, y0 - 0.5), (x1 + 0.5, v), (x0 - 0.5, v)]);
        }
        pts.extend([(x0 - 0.5, y0 - 0.5), (x1 + 0.5, y0 - 0.5), (x0 - 0.5, y1 + 0.5), (x1 + 0.5, y1 + 0.5)]);
        let mut seen: Vec<HolonId> = Vec::new();
        for (u, v) in pts {
            let b = t.leaf_at(v / UNITS_PER_DEG as f64, u / UNITS_PER_DEG as f64);
            if b != a && !seen.contains(&b) {
                seen.push(b);
            }
        }
        seen
    }

    /// Witness: no building footprint (dwelling or not) meets any road, main or local, its own
    /// leaf's or a touching leaf's, in every leaf of the dense, mixed and antimeridian strips; and
    /// every leaf still holds exactly its households.
    #[test]
    fn no_footprint_meets_a_road() {
        let p = params();
        let mut t = tree(60);
        let leaves = strip_leaves(&mut t);
        let (mut checked, mut mains, mut homes, mut foreign) = (0usize, 0usize, 0u64, 0usize);
        for &a in &leaves {
            let la = LeafLayout::build(a, &t.context(a), &p);
            assert!(!la.unplaced, "{a:?}: no spot clears the streets");
            let units: u64 = la.buildings.iter().map(|b| u64::from(b.units)).sum();
            assert_eq!(units, t.budget(a), "{a:?}");
            homes += units;
            let mut roads = la.roads.clone();
            mains += la.roads.iter().filter(|r| r.width >= MAIN_ROAD_M).count();
            for b in touching(&mut t, a) {
                let lb = LeafLayout::build(b, &t.context(b), &p);
                foreign += lb.roads.len();
                roads.extend(lb.roads.iter().map(|r| road_in(a, b, r)));
            }
            let own = la.roads.len();
            for f in &la.buildings {
                for (k, r) in roads.iter().enumerate() {
                    let whose = if k < own { "its own" } else { "a touching leaf's" };
                    assert!(!footprint_meets_road(f, r, 0.0), "{a:?} ({} x {} m): building {f:?} meets {whose} road {r:?}", la.w, la.h);
                    checked += 1;
                }
            }
        }
        let deepest = leaves.iter().map(|l| l.level).max().unwrap_or(0);
        eprintln!("footprints: leaves {} (deepest L{deepest}) mains {mains} homes {homes} foreign roads {foreign} footprint-road pairs {checked}", leaves.len());
        assert!(leaves.len() > 40 && mains > 10 && homes > 1000 && foreign > 100, "leaves {} mains {mains} homes {homes} foreign {foreign} pairs {checked}", leaves.len());
    }

    /// The focal world built on a leaf holds exactly the leaf's households, on the leaf's streets.
    #[test]
    fn focal_world_sits_on_its_leaf() {
        let p = params();
        let mut t = tree(60);
        let leaf = t.leaf_at(10.004, 20.006);
        let l = LeafLayout::build(leaf, &t.context(leaf), &p);
        let w = crate::micro::MicroWorld::from_leaf(&l, p);
        assert_eq!(w.households.len() as u64, t.budget(leaf));
        assert_eq!(w.roads, l.roads);
        assert_eq!(w.buildings.len(), l.buildings.len());
        assert!(w.nodes.len() > 1);
        let (lat, lon) = leaf.centre_deg();
        assert_eq!((w.lat0, w.lon0), (lat, lon));
    }
}
