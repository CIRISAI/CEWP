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

use crate::micro::{rects_overlap, seg_dist, Building, DensityClass, MicroParams, Rng, Road, P};
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
        (s * M_PER_DEG * lat.to_radians().cos().max(0.01), s * M_PER_DEG)
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
        LeafContext { budget, l2_nb, edges, chain }
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
}

impl LeafContext {
    /// Flat form for the kernel door (32 numbers): budget, 4 L2 neighbours, per side level, own
    /// budget, other budget, present, then the 11 chain budgets.
    #[must_use]
    pub fn to_f64(&self) -> [f64; 32] {
        let mut v = [0.0; 32];
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
        c
    }
}

// ------------------------------------------------------------------ edges

/// Street spacing (metres) for a household density (households per km²): about 90 m in a dense
/// core, 230 m suburban, 1.4 km in open country. MODEL, a visual rule; no CIRIS source.
#[must_use]
pub fn street_spacing_m(hh_per_km2: f64) -> f64 {
    (3400.0 / (hh_per_km2.max(0.01)).powf(0.4)).clamp(60.0, 3000.0)
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
        (x0 as f64, s, deg * M_PER_DEG * lat.to_radians().cos().max(0.01))
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

// ------------------------------------------------------------------ leaf layout

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
}

impl LeafLayout {
    /// Leaf-local metres of an absolute lattice point (x in the leaf's column frame).
    #[must_use]
    pub fn to_m(id: HolonId, x: f64, y: f64) -> (f64, f64) {
        let (y0, x0, y1, x1) = id.bounds_units();
        let (xc, yc) = ((x0 + x1) as f64 / 2.0, (y0 + y1) as f64 / 2.0);
        let (lat, _) = id.centre_deg();
        let k = M_PER_DEG / UNITS_PER_DEG as f64;
        ((x - xc) * k * lat.to_radians().cos().max(0.01), (y - yc) * k)
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
            eat(u64::from(b.floors) << 16 | u64::from(b.units));
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
        let mut out = Self { id, w, h, households: ctx.budget, density_km2, class, roads: Vec::new(), buildings: Vec::new(), ports: Vec::new() };
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
        let wd = |main: bool| if main { 12.0 } else { width };
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
        out.place_buildings(&mut rng);
        out.fit_households(&mut rng);
        out
    }

    fn fits(&self, b: &Building) -> bool {
        // the footprint's corners stay inside the leaf (so no building ever meets a neighbour's)
        let (ca, sa) = (b.angle.cos(), b.angle.sin());
        for (u, v) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            let (px, py) = (b.cx + ca * u * b.w / 2.0 - sa * v * b.d / 2.0, b.cy + sa * u * b.w / 2.0 + ca * v * b.d / 2.0);
            if px.abs() > self.w / 2.0 - 1.0 || py.abs() > self.h / 2.0 - 1.0 {
                return false;
            }
        }
        for r in &self.roads {
            if seg_dist(b.cx, b.cy, r) < r.width / 2.0 + 0.5 * b.w.min(b.d) + 1.0 {
                return false;
            }
        }
        !self.buildings.iter().any(|o| rects_overlap(b, o, 0.6))
    }

    fn place_buildings(&mut self, rng: &mut Rng) {
        if self.households == 0 {
            return;
        }
        let roads = self.roads.clone();
        for r in &roads {
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
                    let b = Building { cx: px - uy * off * side, cy: py + ux * off * side, w, d, angle, floors, units };
                    if self.fits(&b) {
                        self.buildings.push(b);
                    }
                    s += w + gap;
                }
            }
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
                // no street frontage held a home: one block at a free spot near the centre
                let side = ((want as f64).sqrt() * 6.0).clamp(10.0, (self.w.min(self.h) - 4.0).max(4.0));
                let mut placed = false;
                for k in 0..24 {
                    let a = k as f64 * 0.7;
                    let rr = k as f64 * 0.04 * self.w.min(self.h);
                    let b = Building { cx: rr * a.cos(), cy: rr * a.sin(), w: side, d: side * 0.7, angle: 0.0, floors: 1, units: 0 };
                    if self.fits(&b) {
                        self.buildings.push(b);
                        placed = true;
                        break;
                    }
                }
                if !placed {
                    self.buildings.push(Building { cx: 0.0, cy: 0.0, w: 6.0, d: 5.0, angle: 0.0, floors: 1, units: 0 });
                }
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
