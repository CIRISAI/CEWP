//! Built form: the per-class numbers a leaf's buildings and streets are drawn from (widths, depths, storeys,
//! dwellings, gaps, setbacks, the village's share of barns, street widths, the dense fabric's plot rules), as ONE
//! table indexed by the street's form class, read through the runtime door (`crate::door`).
//!
//! The built-in rows are the numbers the layout code has always drawn with (MODEL, visual rules; no source): with no
//! table loaded every layout is byte-identical to what the `match class` arms gave (crates/cewp-sim/tests/
//! layout_xcheck.rs and the wasm half, docs/geo/layout_xcheck.test.js, check every digest). A row also states the
//! ORDER its random draws are taken in, so a value that is fixed (no draw) or drawn keeps every later draw where it
//! was; a loaded table publishes numbers only and never the order.
//!
//! Lookup, for a leaf of form class k at climate class z and region g: the loaded table's row for (k, z, g) if it
//! publishes one, else its (k, z, *) row, else its (k, *, g) row (each a "class x zone row"), else its (k, *, *) row
//! (the class's "world row"), else the built-in row. The leaf records which (`FormSource`), so the page can say what
//! a street's form came from. No regional value is in this file: no table of built form exists yet.
//!
//! Street spacing is not in a row: neighbouring leaves place the streets that cross their shared side from either side
//! (`tiling::local_crossings`), so a spacing that changed with a leaf's zone would break where they meet. Its numbers
//! are the default table's world constants (`SPACING`), read by `tiling::street_spacing_m`, and a loaded table cannot
//! change them.

use crate::micro::{DensityClass, Rng};

/// The street's form class: the four density classes, with the top of the apartment class laid out as dense fabric
/// (`tiling::DENSE_FABRIC_KM2`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FormClass {
    /// A main road, a few lanes, scattered houses and barns.
    Village = 0,
    /// Detached houses.
    Suburban = 1,
    /// Terraced houses.
    Terraced = 2,
    /// Apartment blocks on street frontage.
    Blocks = 3,
    /// Plots packed between lanes.
    DenseFabric = 4,
}

impl FormClass {
    /// Every class, in table order.
    pub const ALL: [Self; 5] = [Self::Village, Self::Suburban, Self::Terraced, Self::Blocks, Self::DenseFabric];
    /// The form class of a leaf classed `class` at `density_km2` people per km².
    #[must_use]
    pub fn of(class: DensityClass, density_km2: f64) -> Self {
        match class {
            DensityClass::UrbanCore if density_km2 >= crate::tiling::DENSE_FABRIC_KM2 => Self::DenseFabric,
            DensityClass::UrbanCore => Self::Blocks,
            DensityClass::Urban => Self::Terraced,
            DensityClass::Suburban => Self::Suburban,
            DensityClass::Village => Self::Village,
        }
    }
    /// The name a table row keys on.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Village => "village",
            Self::Suburban => "suburban",
            Self::Terraced => "terraced",
            Self::Blocks => "blocks",
            Self::DenseFabric => "dense",
        }
    }
    fn from_key(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.key() == s)
    }
}

/// A number a row draws: fixed when `lo == hi` (no random draw is taken), else uniform in [lo, hi).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Num {
    /// Low end (the value when fixed).
    pub lo: f64,
    /// High end.
    pub hi: f64,
}

impl Num {
    const fn fixed(v: f64) -> Self {
        Self { lo: v, hi: v }
    }
    const fn range(lo: f64, hi: f64) -> Self {
        Self { lo, hi }
    }
    /// The value, drawing only when the row gives a range.
    pub fn draw(self, rng: &mut Rng) -> f64 {
        if self.lo.to_bits() == self.hi.to_bits() { self.lo } else { rng.range(self.lo, self.hi) }
    }
}

/// A whole number a row draws: `base + below(span)`, fixed at `base` when `span <= 1` (no draw).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Whole {
    /// The least value.
    pub base: u16,
    /// How many values from `base` up.
    pub span: u16,
}

impl Whole {
    const fn fixed(v: u16) -> Self {
        Self { base: v, span: 1 }
    }
    /// The value, drawing only when `span > 1`.
    pub fn draw(self, rng: &mut Rng) -> u16 {
        if self.span <= 1 { self.base } else { self.base + rng.below(usize::from(self.span)) as u16 }
    }
}

/// Dwellings in a building on street frontage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Units {
    /// Drawn as a whole number.
    Count(Whole),
    /// One flat per `m` metres of frontage (at least one), on every storey (apartment blocks).
    PerWidth(f64),
}

/// One draw of a frontage row, named so a row can state the order its draws are taken in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    /// Frontage width.
    Width,
    /// Depth from the street.
    Depth,
    /// Storeys.
    Storeys,
    /// Dwellings.
    Units,
    /// Gap to the next building along the street.
    Gap,
    /// Setback from the street's edge.
    Setback,
}

/// The numbers one kind of building on street frontage is drawn from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Draws {
    /// Frontage width, m.
    pub width: Num,
    /// Depth, m.
    pub depth: Num,
    /// Storeys.
    pub storeys: Whole,
    /// Dwellings.
    pub units: Units,
    /// Gap to the next building, m.
    pub gap: Num,
    /// Setback from the street's edge, m.
    pub setback: Num,
    /// The order the draws are taken in (the code's, kept so the built-in rows reproduce the layouts exactly).
    pub order: [Field; 6],
}

/// One building as drawn: (width, depth, storeys, dwellings, gap, setback).
pub type Drawn = (f64, f64, u8, u16, f64, f64);

impl Draws {
    /// Draw one building.
    pub fn draw(&self, rng: &mut Rng) -> Drawn {
        let (mut w, mut d, mut s, mut u, mut g, mut b) = (0.0, 0.0, 0u16, 0u16, 0.0, 0.0);
        for f in self.order {
            match f {
                Field::Width => w = self.width.draw(rng),
                Field::Depth => d = self.depth.draw(rng),
                Field::Storeys => s = self.storeys.draw(rng),
                Field::Units => {
                    if let Units::Count(c) = self.units {
                        u = c.draw(rng);
                    }
                }
                Field::Gap => g = self.gap.draw(rng),
                Field::Setback => b = self.setback.draw(rng),
            }
        }
        let floors = s.min(255) as u8;
        if let Units::PerWidth(m) = self.units {
            u = (w / m).round().max(1.0) as u16 * u16::from(floors);
        }
        (w, d, floors, u, g, b)
    }
}

/// A frontage class's row: its street width, where along a street the first building starts, its buildings, and the
/// share of them drawn as barns (outbuildings, no dwelling) with their own numbers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrontageRow {
    /// Street width, m (main roads keep `tiling::MAIN_ROAD_M`).
    pub street_m: f64,
    /// Distance along the street to the first building, m.
    pub first: Num,
    /// Homes.
    pub house: Draws,
    /// The barns' share and numbers (a chance is drawn per building wherever a share is given).
    pub barn: Option<(f64, Draws)>,
}

/// Dense fabric's row: plots packed between lanes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DenseRow {
    /// Lane width, m.
    pub street_m: f64,
    /// Plot width along the row, m.
    pub plot_w: Num,
    /// Row depth, m.
    pub plot_d: Num,
    /// The smallest plot side tried where the drawn one does not fit, m.
    pub plot_min: f64,
    /// Gap between plots and between rows, m.
    pub gap: Num,
    /// Fewest storeys.
    pub min_storeys: u8,
    /// Metres of plot width per flat on a storey.
    pub flat_w: f64,
}

/// A row of the table: frontage or dense fabric.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FormRow {
    /// Buildings along the streets.
    Frontage(FrontageRow),
    /// Plots between lanes.
    Dense(DenseRow),
}

impl FormRow {
    /// The street (or lane) width, m.
    #[must_use]
    pub const fn street_m(&self) -> f64 {
        match self {
            Self::Frontage(r) => r.street_m,
            Self::Dense(r) => r.street_m,
        }
    }
}

const ORDER_PLAIN: [Field; 6] = [Field::Width, Field::Depth, Field::Storeys, Field::Units, Field::Gap, Field::Setback];

/// The built-in rows, by `FormClass` (MODEL, visual rules; no source). These are the numbers of the `match class` arms
/// they replaced, with each arm's draw order.
pub const BUILT_IN: [FormRow; 5] = [
    // village: 18 % barns (14 to 22 m by 10 to 14, one storey, no dwelling), else houses of one or two storeys
    FormRow::Frontage(FrontageRow {
        street_m: 5.0,
        first: Num::range(2.0, 10.0),
        house: Draws { width: Num::range(8.5, 13.0), depth: Num::range(8.0, 11.0), storeys: Whole { base: 1, span: 2 }, units: Units::Count(Whole::fixed(1)), gap: Num::range(14.0, 50.0), setback: Num::range(6.0, 13.0), order: ORDER_PLAIN },
        barn: Some((0.18, Draws { width: Num::range(14.0, 22.0), depth: Num::range(10.0, 14.0), storeys: Whole::fixed(1), units: Units::Count(Whole::fixed(0)), gap: Num::range(12.0, 40.0), setback: Num::range(8.0, 14.0), order: ORDER_PLAIN })),
    }),
    // suburban: detached houses of one or two storeys, one dwelling each
    FormRow::Frontage(FrontageRow {
        street_m: 6.0,
        first: Num::range(2.0, 10.0),
        house: Draws { width: Num::range(9.5, 13.0), depth: Num::range(8.5, 12.0), storeys: Whole { base: 1, span: 2 }, units: Units::Count(Whole::fixed(1)), gap: Num::range(9.0, 15.0), setback: Num::range(7.0, 11.0), order: ORDER_PLAIN },
        barn: None,
    }),
    // terraced: two or three storeys, one or two dwellings, 0.4 m apart, 3 m back
    FormRow::Frontage(FrontageRow {
        street_m: 8.0,
        first: Num::range(2.0, 10.0),
        house: Draws { width: Num::range(6.5, 9.0), depth: Num::range(8.5, 11.0), storeys: Whole { base: 2, span: 2 }, units: Units::Count(Whole { base: 1, span: 2 }), gap: Num::fixed(0.4), setback: Num::fixed(3.0), order: [Field::Storeys, Field::Width, Field::Depth, Field::Units, Field::Gap, Field::Setback] },
        barn: None,
    }),
    // apartment blocks: 4 to 7 storeys, a flat per 9 m of frontage on each
    FormRow::Frontage(FrontageRow {
        street_m: 8.0,
        first: Num::range(2.0, 10.0),
        house: Draws { width: Num::range(16.0, 28.0), depth: Num::range(11.0, 15.0), storeys: Whole { base: 4, span: 4 }, units: Units::PerWidth(9.0), gap: Num::range(0.5, 3.0), setback: Num::fixed(1.5), order: [Field::Width, Field::Storeys, Field::Depth, Field::Gap, Field::Setback, Field::Units] },
        barn: None,
    }),
    // dense fabric: plots of 8 to 13 m by 8 to 12, 1 to 2.5 m apart, between 4 m lanes; at least 3 storeys, a flat per 6 m
    FormRow::Dense(DenseRow { street_m: crate::tiling::DENSE_LANE_M, plot_w: Num::range(8.0, 13.0), plot_d: Num::range(8.0, 12.0), plot_min: 8.0, gap: Num::range(1.0, 2.5), min_storeys: crate::tiling::DENSE_MIN_FLOORS, flat_w: 6.0 }),
];

/// The default table's world constants for street spacing (see the module note): spacing = k / (households per km²)^e,
/// clamped to [min, max] metres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spacing {
    /// Numerator, m.
    pub k: f64,
    /// Exponent.
    pub e: f64,
    /// Least spacing, m.
    pub min: f64,
    /// Greatest spacing, m.
    pub max: f64,
}

/// About 90 m in a dense core, 230 m suburban, 1.4 km in open country (MODEL, a visual rule; no source).
pub const SPACING: Spacing = Spacing { k: 3400.0, e: 0.4, min: 60.0, max: 3000.0 };

/// Which row a leaf's form came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FormSource {
    /// The built-in row (no table loaded, or the table publishes nothing for the class).
    #[default]
    BuiltIn = 0,
    /// The loaded table's row for the class with no zone or region.
    World = 1,
    /// The loaded table's row for the class at the leaf's climate class and/or region.
    Zone = 2,
}

/// A row's key: the class, and a climate class (1..30) and region (0..8) or any.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct RowKey {
    /// Form class.
    pub class: FormClass,
    /// Climate class, or None for any.
    pub climate: Option<u8>,
    /// Region, or None for any.
    pub region: Option<u8>,
}

/// A loaded built-form table: complete rows by key.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FormTable {
    /// Rows, each complete (a field a row leaves out is its class's world row's, else the built-in's).
    pub rows: Vec<(RowKey, FormRow)>,
}

impl FormTable {
    /// True when some row is keyed on a climate class (then a layout reads the climate slot too).
    #[must_use]
    pub fn keys_climate(&self) -> bool {
        self.rows.iter().any(|(k, _)| k.climate.is_some())
    }

    /// The row for a leaf and where it came from (the module note's order).
    #[must_use]
    pub fn pick(&self, class: FormClass, climate: u8, region: u8) -> (FormRow, FormSource) {
        let z = (climate != 0).then_some(climate);
        let find = |c: Option<u8>, g: Option<u8>| self.rows.iter().find(|(k, _)| k.class == class && k.climate == c && k.region == g).map(|r| r.1);
        if z.is_some() {
            if let Some(r) = find(z, Some(region)).or_else(|| find(z, None)) {
                return (r, FormSource::Zone);
            }
        }
        if let Some(r) = find(None, Some(region)) {
            return (r, FormSource::Zone);
        }
        if let Some(r) = find(None, None) {
            return (r, FormSource::World);
        }
        (BUILT_IN[class as usize], FormSource::BuiltIn)
    }

    /// Parse the door's text form (FORM-1): lines `row <class> <climate symbol | code | *> <region 0..8 | *> field=value
    /// ...`, `#` comments. Fields: `street`, `first`, `width`, `depth`, `gap`, `setback` (lo,hi or one value), `storeys`
    /// and `units` (base,span), `units=per_width:<m>`, `barn` (a share; 0 or `none` for none), and for dense fabric
    /// `plot_w`, `plot_d`, `plot_min`, `gap`, `min_storeys`, `flat_w`. A field left out is the class's world row's in this
    /// table, else the built-in's. Climate symbols are APPEARANCE-0's legend (`CLIMATE_SYMBOLS`).
    ///
    /// # Errors
    /// The line and what is wrong with it.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut raw: Vec<(RowKey, Vec<(String, String)>)> = Vec::new();
        for (n, line) in text.lines().enumerate() {
            let line = line.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let mut it = line.split_whitespace();
            if it.next() != Some("row") {
                return Err(format!("line {}: not a row", n + 1));
            }
            let class = it.next().and_then(FormClass::from_key).ok_or(format!("line {}: class", n + 1))?;
            let climate = match it.next() {
                Some("*") => None,
                Some(s) => Some(climate_code(s).ok_or(format!("line {}: climate class {s}", n + 1))?),
                None => return Err(format!("line {}: climate class", n + 1)),
            };
            let region = match it.next() {
                Some("*") => None,
                Some(s) => Some(s.parse::<u8>().ok().filter(|&g| g < 9).ok_or(format!("line {}: region {s}", n + 1))?),
                None => return Err(format!("line {}: region", n + 1)),
            };
            let fields = it.map(|f| f.split_once('=').map(|(a, b)| (a.to_string(), b.to_string())).ok_or(format!("line {}: field {f}", n + 1))).collect::<Result<Vec<_>, _>>()?;
            let key = RowKey { class, climate, region };
            if raw.iter().any(|r| r.0 == key) {
                return Err(format!("line {}: a second row for the same key", n + 1));
            }
            raw.push((key, fields));
        }
        // world rows first (a keyed row fills from its class's world row)
        let mut rows: Vec<(RowKey, FormRow)> = Vec::new();
        for pass in [true, false] {
            for (key, fields) in &raw {
                let world = key.climate.is_none() && key.region.is_none();
                if world != pass {
                    continue;
                }
                let base = rows.iter().find(|(k, _)| k.class == key.class && k.climate.is_none() && k.region.is_none()).map_or(BUILT_IN[key.class as usize], |r| r.1);
                rows.push((*key, apply(base, fields)?));
            }
        }
        rows.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(Self { rows })
    }
}

fn num(v: &str) -> Result<Num, String> {
    let p: Vec<f64> = v.split(',').map(|x| x.parse::<f64>().map_err(|_| format!("number {v}"))).collect::<Result<_, _>>()?;
    match p.as_slice() {
        [a] if a.is_finite() && *a >= 0.0 => Ok(Num::fixed(*a)),
        [a, b] if a.is_finite() && b.is_finite() && *a >= 0.0 && b >= a => Ok(Num::range(*a, *b)),
        _ => Err(format!("range {v}")),
    }
}

fn whole(v: &str) -> Result<Whole, String> {
    let p: Vec<u16> = v.split(',').map(|x| x.parse::<u16>().map_err(|_| format!("whole number {v}"))).collect::<Result<_, _>>()?;
    match p.as_slice() {
        [a] => Ok(Whole::fixed(*a)),
        [a, b] if *b >= 1 && u32::from(*a) + u32::from(*b) <= 256 => Ok(Whole { base: *a, span: *b }),
        _ => Err(format!("base,span {v}")),
    }
}

fn scalar(v: &str) -> Result<f64, String> {
    v.parse::<f64>().ok().filter(|x| x.is_finite() && *x >= 0.0).ok_or(format!("number {v}"))
}

fn apply(base: FormRow, fields: &[(String, String)]) -> Result<FormRow, String> {
    match base {
        FormRow::Frontage(mut r) => {
            for (k, v) in fields {
                let h = &mut r.house;
                match k.as_str() {
                    "street" => r.street_m = scalar(v)?,
                    "first" => r.first = num(v)?,
                    "width" => h.width = num(v)?,
                    "depth" => h.depth = num(v)?,
                    "gap" => h.gap = num(v)?,
                    "setback" => h.setback = num(v)?,
                    "storeys" => h.storeys = whole(v)?,
                    "units" => h.units = if let Some(m) = v.strip_prefix("per_width:") { Units::PerWidth(scalar(m)?.max(1.0)) } else { Units::Count(whole(v)?) },
                    "barn" => {
                        let s = if v == "none" { 0.0 } else { scalar(v)? };
                        if s > 1.0 {
                            return Err(format!("barn share {v}"));
                        }
                        r.barn = if s == 0.0 { None } else { Some((s, r.barn.map_or(BARN_DEFAULT, |b| b.1))) };
                    }
                    _ => return Err(format!("field {k} is not a frontage field")),
                }
            }
            Ok(FormRow::Frontage(r))
        }
        FormRow::Dense(mut r) => {
            for (k, v) in fields {
                match k.as_str() {
                    "street" => r.street_m = scalar(v)?,
                    "plot_w" => r.plot_w = num(v)?,
                    "plot_d" => r.plot_d = num(v)?,
                    "plot_min" => r.plot_min = scalar(v)?.max(1.0),
                    "gap" => r.gap = num(v)?,
                    "min_storeys" => r.min_storeys = whole(v)?.base.clamp(1, 60) as u8,
                    "flat_w" => r.flat_w = scalar(v)?.max(1.0),
                    _ => return Err(format!("field {k} is not a dense-fabric field")),
                }
            }
            Ok(FormRow::Dense(r))
        }
    }
}

const BARN_DEFAULT: Draws = match BUILT_IN[0] {
    FormRow::Frontage(FrontageRow { barn: Some((_, b)), .. }) => b,
    _ => panic!("the village row has barns"),
};

/// APPEARANCE-0's class symbols, code 1..=30 at index code - 1 (CIRISHolon's legend).
pub const CLIMATE_SYMBOLS: [&str; 30] = [
    "Af", "Am", "Aw", "BWh", "BWk", "BSh", "BSk", "Csa", "Csb", "Csc", "Cwa", "Cwb", "Cwc", "Cfa", "Cfb", "Cfc", "Dsa", "Dsb", "Dsc", "Dsd", "Dwa", "Dwb", "Dwc", "Dwd", "Dfa", "Dfb", "Dfc", "Dfd", "ET", "EF",
];

/// A climate class's code from its symbol or its number.
#[must_use]
pub fn climate_code(s: &str) -> Option<u8> {
    if let Ok(n) = s.parse::<u8>() {
        return (1..=30).contains(&n).then_some(n);
    }
    CLIMATE_SYMBOLS.iter().position(|&x| x == s).map(|i| i as u8 + 1)
}

// ------------------------------------------------------------------ the generator seam

/// A vintage band: buildings built from `from` to `to` (years, inclusive).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Vintage {
    /// First year.
    pub from: u16,
    /// Last year.
    pub to: u16,
}

/// What a built-form generator is asked, once per building on street frontage: the key the next generator takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormQuery {
    /// The US state (2-digit federal code; 0 none: the country's row), as `door::Door::state_of` decides it.
    pub state: u8,
    /// Climate class 1..30, 0 unknown.
    pub climate: u8,
    /// The twin's density class of the street.
    pub class: FormClass,
    /// A vintage band, if one is known (the twin has no building-age table).
    pub vintage: Option<Vintage>,
    /// The building's seed (from the leaf's seed, the street, the side and the building's place along it; never the
    /// layout's own random stream, which a generator must not move).
    pub seed: u64,
}

/// Where a generated field's numbers were pooled from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Pool {
    /// The state's own rows.
    #[default]
    Own = 0,
    /// Its census division's rows.
    Division = 1,
    /// Borrowed from elsewhere (another division or the country).
    Borrowed = 2,
}

/// The pooling level of each field of a draw, so the page can say where each came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Pooling {
    /// Frontage width and depth.
    pub footprint: Pool,
    /// Storeys.
    pub storeys: Pool,
    /// Dwellings.
    pub units: Pool,
    /// Gap and setback.
    pub spacing: Pool,
}

/// One building as a generator draws it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BuildingDraw {
    /// Frontage width, m.
    pub width: f64,
    /// Depth, m.
    pub depth: f64,
    /// Storeys.
    pub storeys: u8,
    /// Dwellings.
    pub units: u16,
    /// Gap to the next building, m.
    pub gap: f64,
    /// Setback, m.
    pub setback: f64,
    /// Where each field came from.
    pub pooling: Pooling,
}

/// A built-form generator compiled into the kernel (as `holon_scene`'s household and device generators are). The
/// layout asks it only when `present`; an absent answer leaves the row's draw in place.
pub trait FormGenerator {
    /// True when a generator is compiled in.
    fn present(&self) -> bool;
    /// One building's form, or why there is none.
    ///
    /// # Errors
    /// `Absent` with a reason: the layout then draws the building from the row.
    fn draw(&self, q: &FormQuery) -> Result<BuildingDraw, Absent>;
}

/// Why a generator gave no building.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Absent(pub &'static str);

/// No built-form generator ships (CIRISHolon's first was refused by its own tests): every call is absent.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoGenerator;

impl FormGenerator for NoGenerator {
    fn present(&self) -> bool {
        false
    }
    fn draw(&self, _: &FormQuery) -> Result<BuildingDraw, Absent> {
        Err(Absent("no built-form generator ships"))
    }
}

/// What a leaf's form was read from, recorded on its layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FormUse {
    /// Which row.
    pub source: FormSource,
    /// The leaf's climate class (0 unknown or not read).
    pub climate: u8,
    /// The leaf's region.
    pub region: u8,
    /// The leaf's US state, read only when a generator is compiled in (`door::Door::state_of`).
    pub state: crate::door::StateKey,
    /// The door slots the layout read and their versions (`door::Stamp`).
    pub stamp: crate::door::Stamp,
    /// Buildings a generator drew.
    pub generated: u32,
    /// Where the first generated building's fields were pooled from, and whether a later one differed.
    pub pooling: Option<(Pooling, bool)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_table_fills_from_its_world_row_then_the_built_in_and_picks_most_specific_first() {
        let t = FormTable::parse("row village * * gap=20,30\nrow village Cfb * storeys=2,3\nrow village Cfb 4 street=7\nrow blocks * 2 storeys=5,2 # a regional row\n").unwrap();
        let FormRow::Frontage(w) = t.pick(FormClass::Village, 0, 0).0 else { panic!() };
        assert_eq!(t.pick(FormClass::Village, 0, 0).1, FormSource::World);
        assert_eq!(w.house.gap, Num::range(20.0, 30.0));
        assert_eq!(w.house.width, Num::range(8.5, 13.0), "built-in where the world row says nothing");
        let (z, s) = t.pick(FormClass::Village, 15, 1);
        assert_eq!(s, FormSource::Zone);
        let FormRow::Frontage(z) = z else { panic!() };
        assert_eq!((z.house.storeys, z.house.gap), (Whole { base: 2, span: 3 }, Num::range(20.0, 30.0)), "the zone row fills from the world row");
        let FormRow::Frontage(zg) = t.pick(FormClass::Village, 15, 4).0 else { panic!() };
        assert!((zg.street_m - 7.0).abs() < 1e-12);
        assert_eq!(t.pick(FormClass::Blocks, 15, 2).1, FormSource::Zone);
        assert_eq!(t.pick(FormClass::Blocks, 15, 3), (BUILT_IN[3], FormSource::BuiltIn));
        assert_eq!(t.pick(FormClass::Suburban, 15, 4), (BUILT_IN[1], FormSource::BuiltIn));
        assert!(t.keys_climate());
        for bad in ["row town * * width=1", "row village Xx * width=1", "row village * 9 width=1", "row village * * plot_w=3", "row dense * * width=3", "row village * * width=5,2", "row village * *\nrow village * *"] {
            assert!(FormTable::parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn the_stub_generator_is_absent() {
        let q = FormQuery { state: 39, climate: 25, class: FormClass::Suburban, vintage: Some(Vintage { from: 1950, to: 1979 }), seed: 1 };
        assert!(!NoGenerator.present());
        assert_eq!(NoGenerator.draw(&q), Err(Absent("no built-form generator ships")));
    }

    #[test]
    fn climate_symbols_are_the_legend() {
        assert_eq!((climate_code("Af"), climate_code("Cfb"), climate_code("EF"), climate_code("30"), climate_code("0"), climate_code("Xy")), (Some(1), Some(15), Some(30), Some(30), None, None));
    }
}
