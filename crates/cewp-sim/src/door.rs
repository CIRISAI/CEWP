//! The runtime table door: how a CIRISHolon table reaches the kernel while the page runs (FSD/twin/holon-intake.md
//! section 7). Until now a table could only be compiled in (`holon_tables.rs`); through the door the page hands the
//! kernel a NAMED, VERSIONED table (its bytes, the sha256 the index declares, Holon's reference and the version), the
//! kernel checks the sha256 over the bytes it received, parses them into a typed table and holds it in the table's
//! slot. From then on a layout may read it.
//!
//! The rules the door keeps:
//! - A layout is a pure function of (leaf, context, parameters, the versions of the slots it read). Every layout records
//!   the slots it read and the version of each it saw (`Stamp`; version 0 = absent, and reading an absent slot is a
//!   read), so a cache keeps a layout exactly while those slots are at those versions: loading a table invalidates
//!   what read its slot and nothing else, and a layout made before a table arrived is never kept as a different
//!   answer after. The same set of tables gives the same world whatever order they arrived in.
//! - No table means the code's own constants (`builtform::BUILT_IN`), and the layout says so (`FormSource::BuiltIn`).
//! - Loading is in pieces (`begin`, `feed`, `finish`): the page feeds a large table a slice at a time between frames,
//!   and nothing is held or read until the whole has its declared sha256.
//!
//! Slots: `climate` (APPEARANCE-0's Köppen-Geiger class per 1/8-degree cell, 4,147,200 bytes, one per cell, in the
//! twin's cell order), `form` (a built-form table, `builtform::FormTable::parse`'s text; none exists yet), and `state` and
//! `state_count` (REGION-0's US state code per cell and its count of 25 sample points in any state; `Door::state_of`). A climate
//! class is not land: the kernel reads it only for leaves, and leaves exist only where the twin's population chain puts
//! people.

use crate::builtform::{FormClass, FormRow, FormSource, FormTable};
use crate::tiling::HolonId;

/// One table the kernel can hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    /// APPEARANCE-0: the Köppen-Geiger class of every 1/8-degree cell.
    Climate = 0,
    /// A built-form table.
    Form = 1,
    /// REGION-0: the US state code (2-digit federal code) of every 1/8-degree cell.
    State = 2,
    /// REGION-0: how many of each cell's 25 sample points lie in any state (a cell has a state only at 13 or more).
    StateCount = 3,
}

/// Number of slots.
pub const SLOTS: usize = 4;

/// The least count of a cell's 25 sample points in any state for the cell to take its state (REGION-0's note: 2,064 coded
/// cells have fewer; they are coast, Canada or Mexico).
pub const STATE_MIN_POINTS: u8 = 13;

impl Slot {
    /// The name the page loads the slot under.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Climate => "climate",
            Self::Form => "form",
            Self::State => "state",
            Self::StateCount => "state_count",
        }
    }
    /// The slot of a name.
    #[must_use]
    pub fn of(name: &str) -> Option<Self> {
        [Self::Climate, Self::Form, Self::State, Self::StateCount].into_iter().find(|s| s.name() == name)
    }
}

/// The slots a layout read (bit k for slot k) and the version it saw in each (0 = absent).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct Stamp {
    /// Bit k set: slot k was read.
    pub read: u8,
    /// The version of each slot at the read.
    pub versions: [u32; SLOTS],
}

impl Stamp {
    fn read(&mut self, s: Slot, v: u32) {
        self.read |= 1 << s as u8;
        self.versions[s as usize] = v;
    }
    /// True while every slot this stamp read is at the version it read.
    #[must_use]
    pub fn current(&self, door: &Door) -> bool {
        (0..SLOTS).all(|k| self.read & (1 << k) == 0 || self.versions[k] == door.versions()[k])
    }
}

/// What the page declares with a table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    /// The slot's name (`Slot::name`).
    pub name: String,
    /// CIRISHolon's reference, as the index names it (e.g. "APPEARANCE-0").
    pub reference: String,
    /// The table's version (1 or more).
    pub version: u32,
    /// The sha256 the index declares for the bytes handed in.
    pub sha256: [u8; 32],
}

impl Header {
    /// Parse the door's header text: four lines, `name`, `reference`, `version`, `sha256` (64 hex digits).
    ///
    /// # Errors
    /// `Refusal::Header` naming what is wrong.
    pub fn parse(text: &str) -> Result<Self, Refusal> {
        let l: Vec<&str> = text.lines().map(str::trim).collect();
        let [name, reference, version, sha] = l.as_slice() else { return Err(Refusal::Header("four lines: name, reference, version, sha256")) };
        let version = version.parse::<u32>().map_err(|_| Refusal::Header("version"))?;
        let sha = hex32(sha).ok_or(Refusal::Header("sha256: 64 hex digits"))?;
        Ok(Self { name: (*name).to_string(), reference: (*reference).to_string(), version, sha256: sha })
    }
}

fn hex32(s: &str) -> Option<[u8; 32]> {
    if s.len() != 64 {
        return None;
    }
    let mut o = [0u8; 32];
    for (k, b) in o.iter_mut().enumerate() {
        *b = u8::from_str_radix(s.get(2 * k..2 * k + 2)?, 16).ok()?;
    }
    Some(o)
}

/// Why the door refused a table. The slot keeps what it held before.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// The header is malformed.
    Header(&'static str),
    /// No slot has this name.
    UnknownName,
    /// Version 0 is reserved for "absent".
    VersionZero,
    /// `feed` or `finish` without `begin`.
    NotBegun,
    /// The bytes' sha256 is not the declared one.
    Sha256,
    /// The bytes do not parse as the slot's table.
    Parse(String),
}

impl Refusal {
    /// A negative code for the wasm door.
    #[must_use]
    pub const fn code(&self) -> i32 {
        match self {
            Self::Header(_) => -1,
            Self::UnknownName => -2,
            Self::VersionZero => -3,
            Self::NotBegun => -4,
            Self::Sha256 => -5,
            Self::Parse(_) => -6,
        }
    }
}

/// Cells of the twin's 1/8-degree grid: columns, rows.
pub const GRID: (usize, usize) = (2880, 1440);

/// A one-byte-per-cell slot's table (the climate class 1..30, the state code 1..56, the state count 0..25; 0 none).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Climate {
    cells: Vec<u8>,
}

/// The state slots hold the same shape of table.
pub type CellGrid = Climate;

impl Climate {
    fn parse(bytes: Vec<u8>, max: u8) -> Result<Self, Refusal> {
        if bytes.len() != GRID.0 * GRID.1 {
            return Err(Refusal::Parse(format!("{} bytes, not one per cell ({})", bytes.len(), GRID.0 * GRID.1)));
        }
        if let Some(b) = bytes.iter().find(|&&b| b > max) {
            return Err(Refusal::Parse(format!("value {b} above {max}")));
        }
        Ok(Self { cells: bytes })
    }
    /// The class of cell (r, c), r from the north.
    #[must_use]
    pub fn at_cell(&self, r: usize, c: usize) -> u8 {
        self.cells[r.min(GRID.1 - 1) * GRID.0 + c % GRID.0]
    }
}

/// What decided a leaf's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StateWhy {
    /// The cell's own state.
    Own = 0,
    /// No state table held: the country's row.
    #[default]
    NotLoaded = 1,
    /// The cell has no code (outside every state, or US land dropped at the map's scale: small islands, the westernmost
    /// Aleutians): the country's row.
    NoCode = 2,
    /// Fewer than 13 of the cell's 25 points lie in any state (coast, a border): the country's row.
    FewPoints = 3,
    /// The twin's own country mask does not put the cell in the United States: no state.
    NotUs = 4,
}

/// A leaf's US state key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StateKey {
    /// The state used (federal code), 0 for none.
    pub code: u8,
    /// The cell's code as published.
    pub raw: u8,
    /// The cell's sample points in any state (of 25).
    pub points: u8,
    /// What decided it.
    pub why: StateWhy,
}

/// The twin's cell of a point (the page's `geoCell` at level 1): row from the north, column from -180.
#[must_use]
pub fn cell_of(lat: f64, lon: f64) -> (usize, usize) {
    let s = 0.125;
    let c = ((((lon + 180.0).rem_euclid(360.0)) / s).floor() as usize).min(GRID.0 - 1);
    let r = ((90.0 - lat) / s).floor().clamp(0.0, (GRID.1 - 1) as f64) as usize;
    (r, c)
}

/// The cell a leaf keys on: its L3 ancestor's centre (the leaf itself above L3), so every leaf of a street tile shares
/// one climate class and one region.
#[must_use]
pub fn key_cell(id: HolonId) -> (usize, usize) {
    let t = if id.level >= 3 { id.ancestor(3) } else { id };
    let (la, lo) = t.centre_deg();
    cell_of(la, lo)
}

/// The twin's region (0..8) of a cell (`regions_table`, the rule popchain's in-cell seam uses).
#[must_use]
pub fn region_of_cell(r: usize, c: usize) -> u8 {
    crate::regions_table::REGION_OF_L0.get((r / 12) * 240 + (c / 12) % 240).copied().unwrap_or(0)
}

struct Pending {
    slot: Slot,
    header: Header,
    sha: Sha256,
    bytes: Vec<u8>,
}

/// The kernel's tables and the generator it may call.
pub struct Door {
    climate: Option<(Header, Climate)>,
    form: Option<(Header, FormTable)>,
    state: Option<(Header, CellGrid)>,
    state_count: Option<(Header, CellGrid)>,
    pending: Option<Pending>,
    /// The built-form generator (none exists yet: `NoGenerator`).
    pub generator: &'static (dyn crate::builtform::FormGenerator + Sync),
}

impl Default for Door {
    fn default() -> Self {
        Self { climate: None, form: None, state: None, state_count: None, pending: None, generator: &crate::builtform::NoGenerator }
    }
}

impl std::fmt::Debug for Door {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Door").field("versions", &self.versions()).field("generator", &self.generator.present()).finish()
    }
}

/// The door with nothing loaded: every layout reads the built-in rows.
pub static EMPTY: std::sync::LazyLock<Door> = std::sync::LazyLock::new(Door::default);

impl Door {
    /// An empty door that calls `generator` (a built-form generator compiled in).
    #[must_use]
    pub fn with_generator(generator: &'static (dyn crate::builtform::FormGenerator + Sync)) -> Self {
        Self { generator, ..Self::default() }
    }

    /// The version held in each slot (0 = absent).
    #[must_use]
    pub fn versions(&self) -> [u32; SLOTS] {
        let v = |h: Option<&Header>| h.map_or(0, |h| h.version);
        [v(self.climate.as_ref().map(|t| &t.0)), v(self.form.as_ref().map(|t| &t.0)), v(self.state.as_ref().map(|t| &t.0)), v(self.state_count.as_ref().map(|t| &t.0))]
    }

    /// The header a slot holds.
    #[must_use]
    pub fn header(&self, s: Slot) -> Option<&Header> {
        match s {
            Slot::Climate => self.climate.as_ref().map(|t| &t.0),
            Slot::Form => self.form.as_ref().map(|t| &t.0),
            Slot::State => self.state.as_ref().map(|t| &t.0),
            Slot::StateCount => self.state_count.as_ref().map(|t| &t.0),
        }
    }

    /// Start a table (a pending one is dropped).
    ///
    /// # Errors
    /// An unknown name or version 0.
    pub fn begin(&mut self, header: Header) -> Result<(), Refusal> {
        self.pending = None;
        let slot = Slot::of(&header.name).ok_or(Refusal::UnknownName)?;
        if header.version == 0 {
            return Err(Refusal::VersionZero);
        }
        // the climate grid's size is known: one allocation, no doubling while it arrives
        let bytes = if slot == Slot::Form { Vec::new() } else { Vec::with_capacity(GRID.0 * GRID.1) };
        self.pending = Some(Pending { slot, header, sha: Sha256::new(), bytes });
        Ok(())
    }

    /// More of the pending table's bytes.
    ///
    /// # Errors
    /// `NotBegun`.
    pub fn feed(&mut self, bytes: &[u8]) -> Result<usize, Refusal> {
        let p = self.pending.as_mut().ok_or(Refusal::NotBegun)?;
        p.sha.update(bytes);
        p.bytes.extend_from_slice(bytes);
        Ok(p.bytes.len())
    }

    /// Check the pending table's sha256, parse it and put it in its slot.
    ///
    /// # Errors
    /// `NotBegun`, `Sha256` or `Parse`; the slot keeps what it held.
    pub fn finish(&mut self) -> Result<Slot, Refusal> {
        let p = self.pending.take().ok_or(Refusal::NotBegun)?;
        if p.sha.finish() != p.header.sha256 {
            return Err(Refusal::Sha256);
        }
        match p.slot {
            Slot::Climate => self.climate = Some((p.header, Climate::parse(p.bytes, 30)?)),
            Slot::State => self.state = Some((p.header, CellGrid::parse(p.bytes, 56)?)),
            Slot::StateCount => self.state_count = Some((p.header, CellGrid::parse(p.bytes, 25)?)),
            Slot::Form => {
                let text = String::from_utf8(p.bytes).map_err(|_| Refusal::Parse("not UTF-8".into()))?;
                self.form = Some((p.header, FormTable::parse(&text).map_err(Refusal::Parse)?));
            }
        }
        Ok(p.slot)
    }

    /// `begin`, `feed` the whole, `finish`.
    ///
    /// # Errors
    /// As those.
    pub fn load(&mut self, header: Header, bytes: &[u8]) -> Result<Slot, Refusal> {
        self.begin(header)?;
        self.feed(bytes)?;
        self.finish()
    }

    /// Empty a slot (its version becomes 0).
    pub fn unload(&mut self, s: Slot) {
        match s {
            Slot::Climate => self.climate = None,
            Slot::Form => self.form = None,
            Slot::State => self.state = None,
            Slot::StateCount => self.state_count = None,
        }
    }

    /// The US state a leaf keys on (its L3 tile's centre cell), with what decided it: a code counts only with both state
    /// slots held, in a cell the twin's own country mask puts in the United States, with 13 or more of its 25 sample points
    /// in any state; otherwise none, the country's row stands in, and the reason is kept. (A leaf is on land by
    /// construction: leaves exist only where the population chain puts people.)
    #[must_use]
    pub fn state_of(&self, id: HolonId) -> StateKey {
        let t = if id.level >= 3 { id.ancestor(3) } else { id };
        let (la, lo) = t.centre_deg();
        let (r, c) = cell_of(la, lo);
        let (Some((_, codes)), Some((_, counts))) = (self.state.as_ref(), self.state_count.as_ref()) else {
            return StateKey { code: 0, raw: 0, points: 0, why: StateWhy::NotLoaded };
        };
        let (raw, points) = (codes.at_cell(r, c), counts.at_cell(r, c));
        let in_us = || crate::countries::country_near(la, lo, 0.15) == Some(840);
        let why = if raw == 0 {
            if in_us() { StateWhy::NoCode } else { StateWhy::NotUs }
        } else if points < STATE_MIN_POINTS {
            StateWhy::FewPoints
        } else if !in_us() {
            StateWhy::NotUs
        } else {
            StateWhy::Own
        };
        StateKey { code: if why == StateWhy::Own { raw } else { 0 }, raw, points, why }
    }

    /// The climate class at a point (0 with no climate table, or a cell with no class). Not land: see the module note.
    #[must_use]
    pub fn climate_at(&self, lat: f64, lon: f64) -> u8 {
        let (r, c) = cell_of(lat, lon);
        self.climate.as_ref().map_or(0, |t| t.1.at_cell(r, c))
    }

    /// The state code and sample-point count published for the cell at a point (0, 0 when not held).
    #[must_use]
    pub fn state_cell(&self, lat: f64, lon: f64) -> (u8, u8) {
        let (r, c) = cell_of(lat, lon);
        (self.state.as_ref().map_or(0, |t| t.1.at_cell(r, c)), self.state_count.as_ref().map_or(0, |t| t.1.at_cell(r, c)))
    }

    /// The climate class a leaf keys on (`key_cell`).
    #[must_use]
    pub fn climate_of(&self, id: HolonId) -> u8 {
        let (r, c) = key_cell(id);
        self.climate.as_ref().map_or(0, |t| t.1.at_cell(r, c))
    }

    /// The built-form row for leaf `id` of class `class`, with what was read: the form slot always (absent is a read),
    /// the climate slot only when the loaded form table keys on climate.
    #[must_use]
    pub fn form(&self, id: HolonId, class: FormClass) -> (FormRow, crate::builtform::FormUse) {
        let (r, c) = key_cell(id);
        let region = region_of_cell(r, c);
        let v = self.versions();
        let mut stamp = Stamp::default();
        stamp.read(Slot::Form, v[Slot::Form as usize]);
        // a generator's key is (state, climate, class, vintage, seed): with one compiled in, both state slots and the climate
        // slot are read
        let gen = self.generator.present();
        let state = if gen {
            stamp.read(Slot::State, v[Slot::State as usize]);
            stamp.read(Slot::StateCount, v[Slot::StateCount as usize]);
            self.state_of(id)
        } else {
            StateKey::default()
        };
        let keyed = self.form.as_ref().is_some_and(|t| t.1.keys_climate());
        let climate = if keyed || gen {
            stamp.read(Slot::Climate, v[Slot::Climate as usize]);
            self.climate_of(id)
        } else {
            0
        };
        let Some((_, t)) = self.form.as_ref() else {
            return (crate::builtform::BUILT_IN[class as usize], crate::builtform::FormUse { source: FormSource::BuiltIn, climate, region, state, stamp, generated: 0, pooling: None });
        };
        let (row, source) = t.pick(class, if keyed { climate } else { 0 }, region);
        (row, crate::builtform::FormUse { source, climate, region, state, stamp, generated: 0, pooling: None })
    }
}

/// SHA-256 (FIPS 180-4), fed in pieces.
#[derive(Clone)]
pub struct Sha256 {
    h: [u32; 8],
    buf: [u8; 64],
    n: usize,
    len: u64,
}

const K: [u32; 64] = [
    0x428a_2f98, 0x7137_4491, 0xb5c0_fbcf, 0xe9b5_dba5, 0x3956_c25b, 0x59f1_11f1, 0x923f_82a4, 0xab1c_5ed5, 0xd807_aa98, 0x1283_5b01, 0x2431_85be, 0x550c_7dc3, 0x72be_5d74, 0x80de_b1fe, 0x9bdc_06a7, 0xc19b_f174,
    0xe49b_69c1, 0xefbe_4786, 0x0fc1_9dc6, 0x240c_a1cc, 0x2de9_2c6f, 0x4a74_84aa, 0x5cb0_a9dc, 0x76f9_88da, 0x983e_5152, 0xa831_c66d, 0xb003_27c8, 0xbf59_7fc7, 0xc6e0_0bf3, 0xd5a7_9147, 0x06ca_6351, 0x1429_2967,
    0x27b7_0a85, 0x2e1b_2138, 0x4d2c_6dfc, 0x5338_0d13, 0x650a_7354, 0x766a_0abb, 0x81c2_c92e, 0x9272_2c85, 0xa2bf_e8a1, 0xa81a_664b, 0xc24b_8b70, 0xc76c_51a3, 0xd192_e819, 0xd699_0624, 0xf40e_3585, 0x106a_a070,
    0x19a4_c116, 0x1e37_6c08, 0x2748_774c, 0x34b0_bcb5, 0x391c_0cb3, 0x4ed8_aa4a, 0x5b9c_ca4f, 0x682e_6ff3, 0x748f_82ee, 0x78a5_636f, 0x84c8_7814, 0x8cc7_0208, 0x90be_fffa, 0xa450_6ceb, 0xbef9_a3f7, 0xc671_78f2,
];

impl Default for Sha256 {
    fn default() -> Self {
        Self::new()
    }
}

impl Sha256 {
    /// A fresh hash.
    #[must_use]
    pub const fn new() -> Self {
        Self { h: [0x6a09_e667, 0xbb67_ae85, 0x3c6e_f372, 0xa54f_f53a, 0x510e_527f, 0x9b05_688c, 0x1f83_d9ab, 0x5be0_cd19], buf: [0; 64], n: 0, len: 0 }
    }
    fn block(h: &mut [u32; 8], b: &[u8]) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([b[4 * i], b[4 * i + 1], b[4 * i + 2], b[4 * i + 3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }
        let [mut a, mut b2, mut c, mut d, mut e, mut f, mut g, mut hh] = *h;
        for i in 0..64 {
            let t1 = hh.wrapping_add(e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25)).wrapping_add((e & f) ^ (!e & g)).wrapping_add(K[i]).wrapping_add(w[i]);
            let t2 = (a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22)).wrapping_add((a & b2) ^ (a & c) ^ (b2 & c));
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b2;
            b2 = a;
            a = t1.wrapping_add(t2);
        }
        for (x, y) in h.iter_mut().zip([a, b2, c, d, e, f, g, hh]) {
            *x = x.wrapping_add(y);
        }
    }
    /// Hash more bytes.
    pub fn update(&mut self, mut data: &[u8]) {
        self.len = self.len.wrapping_add(data.len() as u64);
        if self.n > 0 {
            let k = (64 - self.n).min(data.len());
            self.buf[self.n..self.n + k].copy_from_slice(&data[..k]);
            self.n += k;
            data = &data[k..];
            if self.n < 64 {
                return;
            }
            let b = self.buf;
            Self::block(&mut self.h, &b);
            self.n = 0;
        }
        let mut chunks = data.chunks_exact(64);
        for b in &mut chunks {
            Self::block(&mut self.h, b);
        }
        let r = chunks.remainder();
        self.buf[..r.len()].copy_from_slice(r);
        self.n = r.len();
    }
    /// The digest.
    #[must_use]
    pub fn finish(mut self) -> [u8; 32] {
        let bits = self.len.wrapping_mul(8);
        self.update(&[0x80]);
        while self.n != 56 {
            self.update(&[0]);
        }
        self.update(&bits.to_be_bytes());
        let mut o = [0u8; 32];
        for (k, v) in self.h.iter().enumerate() {
            o[4 * k..4 * k + 4].copy_from_slice(&v.to_be_bytes());
        }
        o
    }
}

/// SHA-256 of a byte slice.
#[must_use]
pub fn sha256(bytes: &[u8]) -> [u8; 32] {
    let mut s = Sha256::new();
    s.update(bytes);
    s.finish()
}

/// Lower-case hex.
#[must_use]
pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_matches_the_standard_vectors_in_any_pieces() {
        assert_eq!(hex(&sha256(b"")), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
        assert_eq!(hex(&sha256(b"abc")), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        let m = b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";
        assert_eq!(hex(&sha256(m)), "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1");
        let big: Vec<u8> = (0..100_000u32).map(|i| (i * 7 + i / 13) as u8).collect();
        let whole = sha256(&big);
        for step in [1usize, 3, 63, 64, 65, 1000, 65_536] {
            let mut s = Sha256::new();
            for c in big.chunks(step) {
                s.update(c);
            }
            assert_eq!(s.finish(), whole, "pieces of {step}");
        }
    }

    fn header(name: &str, version: u32, bytes: &[u8]) -> Header {
        Header { name: name.into(), reference: "TEST".into(), version, sha256: sha256(bytes) }
    }

    #[test]
    fn the_door_refuses_a_wrong_sha_and_keeps_what_it_held() {
        let mut d = Door::default();
        let t = b"row village * * storeys=1".to_vec();
        assert_eq!(d.load(header("form", 3, &t), &t), Ok(Slot::Form));
        assert_eq!(d.versions(), [0, 3, 0, 0]);
        let mut h = header("form", 4, b"row village * * storeys=2");
        h.sha256[0] ^= 1;
        assert_eq!(d.load(h, b"row village * * storeys=2"), Err(Refusal::Sha256));
        assert_eq!(d.versions(), [0, 3, 0, 0], "the slot keeps the version it held");
        assert_eq!(d.load(header("weather", 1, b""), b""), Err(Refusal::UnknownName));
        assert_eq!(d.load(header("form", 0, b""), b""), Err(Refusal::VersionZero));
        assert_eq!(d.load(header("climate", 1, &[1, 2, 3]), &[1, 2, 3]).map_err(|e| e.code()), Err(-6));
        let h = Header::parse(&format!("form\nFORM-TEST\n7\n{}\n", hex(&sha256(&t)))).unwrap();
        assert_eq!((h.version, h.sha256), (7, sha256(&t)));
        assert!(Header::parse("form\nX\n7\nzz").is_err());
    }

    #[test]
    fn cells_are_the_pages() {
        // docs/geo/geodata.js geoCell at level 1
        assert_eq!(cell_of(90.0, -180.0), (0, 0));
        assert_eq!(cell_of(-90.0, 180.0), (1439, 0));
        assert_eq!(cell_of(51.51, -0.13), (307, 1438)); // CIRISHolon APPEARANCE-0 R1: London, cell 885,598
        assert_eq!(cell_of(-33.87, 151.21), (990, 2649)); // Sydney, cell 2,853,849
    }
}
