//! A seeded social graph over generated households and the localities of the population grid
//! (twin package `people`, spec in `FSD/twin/people.md`).
//!
//! The scale page (`docs/globe.js` `buildWorld`) draws every family in the signer's locality and
//! every community room 90% local, then signs acts 65% family / 30% community / 4% consent-free /
//! 1% public. Its locality figure (~92%, README) is therefore `0.65 + 0.30 × 0.90`: an input.
//! This module builds the rooms from named mechanisms instead (households, extended family with a
//! diaspora, workplace teams, school classes, neighbourhood rooms, interest groups, public
//! broadcast rooms), each with a placement rule, so that locality is measured on the result
//! (`crate::locality`). The placement rules are themselves inputs; what changes is that each one
//! is a quantity a real dataset could replace (a migration stock, commuting flows, a social
//! connectedness index), not the answer.
//!
//! No person here corresponds to a real person. Households are drawn per grid cell from coarse
//! population weights only; the grid is the page's own (`POP_GRID_B64`), read through
//! [`Geography::from_globe_js`], or a generated stand-in.

use crate::micro::{ParamSpec, Rng, Tag};

/// Sentinel for "no room" (a consent-free act has no room).
pub const NO_ROOM: u32 = u32::MAX;

// ------------------------------------------------------------------ parameters

macro_rules! social_params {
    ($( $field:ident = $def:expr, $tag:ident, $src:literal; )*) => {
        /// Named social-graph parameters (FSD §8.2 tags in [`SOCIAL_PARAM_SPECS`]).
        #[derive(Debug, Clone, PartialEq)]
        pub struct SocialParams { $( #[doc = $src] pub $field: f64, )* }
        impl Default for SocialParams {
            fn default() -> Self { Self { $( $field: $def, )* } }
        }
        /// Every social-graph parameter with its default, class and source.
        pub const SOCIAL_PARAM_SPECS: &[ParamSpec] = &[
            $( ParamSpec { name: stringify!($field), default: $def, tag: Tag::$tag, source: $src }, )*
        ];
    };
}

social_params! {
    persons_per_household = 2.5, Model, "same default and draw as micro.rs persons_per_household: round(2.5 + sd·N(0,1)) clamped to 1..6; kind of source: UN household size and composition tables (not loaded)";
    household_size_sd = 1.1, Model, "spread of the household size draw, as micro.rs uses (1.1); 0 with persons_per_household 1 gives single-person households";
    student_share = 0.18, Model, "share of persons in a school class; kind of source: UN World Population Prospects age structure times enrolment (not loaded)";
    worker_share = 0.45, Model, "share of persons in a workplace team; kind of source: ILO employment-to-population ratio (not loaded)";
    kin_households_mean = 4.0, Model, "households in one extended-family room including its founder (1 + Poisson(mean - 1)); no source in the repo";
    diaspora_share = 0.12, Model, "share of a kin room's other households living outside the founder's locality; kind of source: UN DESA International Migrant Stock (about 3.6% of people live outside their birth country, quoted from memory, not checked against the table) plus internal migration between localities; a migration stock table would replace it";
    diaspora_abroad_share = 0.35, Model, "of diaspora households, the share in another world region; the twin has no country layer, so another region stands in for abroad";
    work_team_median = 8.0, Model, "median workplace team room size (lognormal); firm sizes are heavy-tailed (Zipf-like in firm-size literature) but a team room is not the firm";
    work_team_sigma = 0.8, Model, "lognormal spread of team room sizes";
    work_team_max = 200.0, Model, "largest team room";
    work_cross_locality_share = 0.05, Model, "share of workers whose team sits in another locality (remote work, long commutes); kind of source: census commuting flows (not loaded)";
    work_cross_region_share = 0.25, Model, "of cross-locality workers, the share whose team is in another region";
    class_size = 25.0, Model, "school class room size; kind of source: OECD / UNESCO class-size tables (not loaded)";
    community_join_share = 0.5, Model, "share of households in a neighbourhood room (= micro.rs community_join_share)";
    neighbourhood_room_households = 30.0, Model, "households per neighbourhood room: the micro patch holds ~60 households with half in one street room";
    interest_groups_per_person = 2.0, Frontier, "interest-group memberships per person (Poisson); no data in the repo";
    interest_group_mean_size = 20.0, Frontier, "mean members per interest group (sizes heavy-tailed by interest_pareto_alpha)";
    interest_local_share = 0.5, Frontier, "share of interest-group joins into a group bound to the person's locality; nothing in the repo or loaded data supports a value; the main lever, swept in the report";
    interest_regional_share = 0.3, Frontier, "share of joins into a group bound to the person's region; the rest (0.2) are global";
    interest_pareto_alpha = 1.5, Model, "Pareto tail of interest-group popularity (heavy-tailed group sizes)";
    public_follows_per_person = 3.0, Frontier, "public broadcast rooms a person follows (Poisson)";
    public_room_mean_size = 200.0, Model, "mean audience per public room at this world's size (rooms = persons × follows / mean)";
    public_local_share = 0.4, Frontier, "share of follows into a locality-bound public room (local news, local channels)";
    public_regional_share = 0.3, Frontier, "share of follows into a region-bound public room; the rest (0.3) are global";
    public_pareto_alpha = 1.1, Model, "Pareto tail of public-room audience (follower counts are heavy-tailed; kind of source: platform follower-count distributions)";
    clique_cap = 12.0, Model, "rooms up to this size make every pair of members a tie";
    large_group_ties = 6.0, Model, "in a larger room each member ties to this many random co-members (a room is not a clique of friendships); public follows make no tie";
}

/// Whether rooms respect geography. `Placeless` is the null model: every non-household room
/// draws its members uniformly from the whole world, so a tie is local with probability
/// about 1 / localities (equal localities).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    /// Mechanisms place members by the parameters above.
    Placed,
    /// Every room ignores geography.
    Placeless,
}

// ------------------------------------------------------------------ geography

/// One population-grid cell.
#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    /// Centre latitude, degrees.
    pub lat: f32,
    /// Centre longitude, degrees (sets the cell's solar clock).
    pub lon: f32,
    /// Population weight.
    pub weight: f64,
    /// Locality index.
    pub locality: u16,
}

/// Cells grouped into localities grouped into regions.
#[derive(Debug, Clone, PartialEq)]
pub struct Geography {
    /// The cells.
    pub cells: Vec<Cell>,
    /// Region of each locality.
    pub region_of_locality: Vec<u8>,
}

impl Geography {
    /// Number of localities.
    #[must_use]
    pub fn localities(&self) -> usize {
        self.region_of_locality.len()
    }

    /// Number of regions.
    #[must_use]
    pub fn regions(&self) -> usize {
        self.region_of_locality.iter().map(|&r| r as usize + 1).max().unwrap_or(0)
    }

    /// `n` equal localities of one cell each, spread over longitude, `regions` regions in turn.
    /// A test geography (MODEL).
    #[must_use]
    pub fn uniform(n: usize, regions: usize) -> Self {
        let cells = (0..n)
            .map(|k| Cell { lat: 0.0, lon: -180.0 + 360.0 * (k as f32 + 0.5) / n as f32, weight: 1.0, locality: k as u16 })
            .collect();
        let region_of_locality = (0..n).map(|k| (k % regions.max(1)) as u8).collect();
        Self { cells, region_of_locality }
    }

    /// A generated stand-in for the page's grid (MODEL): `n_loc` localities of `cells_per`
    /// cells, lognormal locality weights (sigma 1.2), `regions` regions in longitude bands.
    #[must_use]
    pub fn stand_in(seed: u64, n_loc: usize, cells_per: usize, regions: usize) -> Self {
        let mut r = Rng::new(seed ^ 0x6765_6F67);
        let mut cells = Vec::new();
        let mut region_of_locality = Vec::new();
        for k in 0..n_loc {
            let lon0 = -170.0 + 340.0 * r.f();
            let lat0 = -40.0 + 100.0 * r.f();
            let w = (1.2 * r.gauss()).exp();
            region_of_locality.push((((lon0 + 180.0) / 360.0 * regions as f64) as usize).min(regions - 1) as u8);
            for _ in 0..cells_per {
                cells.push(Cell {
                    lat: (lat0 + r.range(-3.0, 3.0)) as f32,
                    lon: (lon0 + r.range(-3.0, 3.0)) as f32,
                    weight: w * (0.5 * r.gauss()).exp(),
                    locality: k as u16,
                });
            }
        }
        Self { cells, region_of_locality }
    }

    /// Parse the page's population-grid binary (`POP_GRID_B64` decoded): u16 cells, u16
    /// localities, one region byte per locality, pad to even, then per cell i16 lat·100,
    /// i16 lon·100, u16 weight, u8 locality, u8 meta (little-endian), as `initPopGrid` reads it.
    ///
    /// # Errors
    /// A short or inconsistent buffer.
    pub fn from_pop_grid(b: &[u8]) -> Result<Self, String> {
        let u16at = |o: usize| -> Result<u16, String> {
            b.get(o..o + 2).map(|s| u16::from_le_bytes([s[0], s[1]])).ok_or_else(|| "pop grid: short".to_string())
        };
        let n_cells = u16at(0)? as usize;
        let n_locs = u16at(2)? as usize;
        let region_of_locality = b.get(4..4 + n_locs).ok_or("pop grid: short header")?.to_vec();
        let mut off = 4 + n_locs;
        if off % 2 == 1 {
            off += 1;
        }
        let mut cells = Vec::with_capacity(n_cells);
        for _ in 0..n_cells {
            let lat = u16at(off)? as i16 as f32 * 0.01;
            let lon = u16at(off + 2)? as i16 as f32 * 0.01;
            let w = u16at(off + 4)? as f64;
            let loc = *b.get(off + 6).ok_or("pop grid: short cell")? as u16;
            if loc as usize >= n_locs {
                return Err(format!("pop grid: cell locality {loc} >= {n_locs}"));
            }
            cells.push(Cell { lat, lon, weight: w, locality: loc });
            off += 8;
        }
        Ok(Self { cells, region_of_locality })
    }

    /// Read the grid out of the page source (`docs/globe.js`, `const POP_GRID_B64="..."`).
    ///
    /// # Errors
    /// No grid literal, bad base64, or a bad grid.
    pub fn from_globe_js(src: &str) -> Result<Self, String> {
        let key = "POP_GRID_B64=\"";
        let a = src.find(key).ok_or("globe.js: no POP_GRID_B64")? + key.len();
        let len = src[a..].find('"').ok_or("globe.js: unterminated POP_GRID_B64")?;
        Self::from_pop_grid(&base64_decode(&src[a..a + len])?)
    }
}

/// Standard base64 (RFC 4648, padding optional).
///
/// # Errors
/// A character outside the alphabet.
pub fn base64_decode(s: &str) -> Result<Vec<u8>, String> {
    let val = |c: u8| -> Result<u32, String> {
        Ok(match c {
            b'A'..=b'Z' => (c - b'A') as u32,
            b'a'..=b'z' => (c - b'a' + 26) as u32,
            b'0'..=b'9' => (c - b'0' + 52) as u32,
            b'+' => 62,
            b'/' => 63,
            _ => return Err(format!("base64: bad byte {c}")),
        })
    };
    let bytes: Vec<u8> = s.bytes().filter(|&c| c != b'=' && !c.is_ascii_whitespace()).collect();
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    for chunk in bytes.chunks(4) {
        let mut acc = 0u32;
        for (i, &c) in chunk.iter().enumerate() {
            acc |= val(c)? << (18 - 6 * i);
        }
        let n = chunk.len();
        out.push((acc >> 16) as u8);
        if n > 2 {
            out.push((acc >> 8) as u8);
        }
        if n > 3 {
            out.push(acc as u8);
        }
    }
    Ok(out)
}

// ------------------------------------------------------------------ the world

/// What a person does in the day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// In a school class.
    Student,
    /// In a workplace team.
    Worker,
    /// Neither.
    Other,
}

/// One generated person.
#[derive(Debug, Clone, PartialEq)]
pub struct Person {
    /// Grid cell.
    pub cell: u32,
    /// Locality of the cell.
    pub locality: u16,
    /// Region of the locality.
    pub region: u8,
    /// Household index.
    pub household: u32,
    /// Day role.
    pub role: Role,
    /// Longitude (solar clock).
    pub lon: f32,
}

/// The mechanism a room comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum RoomKind {
    /// Co-resident household.
    Household = 0,
    /// Extended family across households (diaspora included).
    Kin = 1,
    /// Workplace team.
    Work = 2,
    /// School class.
    School = 3,
    /// Neighbourhood room.
    Neighbourhood = 4,
    /// Interest group, not bound to place unless drawn local.
    Interest = 5,
    /// Public broadcast room.
    Public = 6,
}

/// Every room kind in index order.
pub const ROOM_KINDS: [RoomKind; 7] = [
    RoomKind::Household,
    RoomKind::Kin,
    RoomKind::Work,
    RoomKind::School,
    RoomKind::Neighbourhood,
    RoomKind::Interest,
    RoomKind::Public,
];

/// The scope an act carries (the page's four act kinds).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Scope {
    /// self / family.
    Family = 0,
    /// A community room.
    Community = 1,
    /// A consent-free row (goes to serve seats).
    ConsentFree = 2,
    /// A public room.
    Public = 3,
}

impl RoomKind {
    /// The act scope a row in this room carries.
    #[must_use]
    pub const fn scope(self) -> Scope {
        match self {
            Self::Household | Self::Kin => Scope::Family,
            Self::Work | Self::School | Self::Neighbourhood | Self::Interest => Scope::Community,
            Self::Public => Scope::Public,
        }
    }
    /// Lower-case name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Household => "household",
            Self::Kin => "kin",
            Self::Work => "work",
            Self::School => "school",
            Self::Neighbourhood => "neighbourhood",
            Self::Interest => "interest",
            Self::Public => "public",
        }
    }
}

/// Persons, households and rooms, with both membership directions as CSR arrays.
#[derive(Debug, Clone, PartialEq)]
pub struct SocialWorld {
    /// Region of each locality.
    pub region_of_locality: Vec<u8>,
    /// The persons.
    pub persons: Vec<Person>,
    /// Number of households.
    pub households: usize,
    /// Kind of each room.
    pub room_kind: Vec<RoomKind>,
    /// Room → members offsets.
    pub room_off: Vec<u32>,
    /// Room members (sorted per room, distinct).
    pub room_mem: Vec<u32>,
    /// Room → (locality, members there) offsets.
    pub room_loc_off: Vec<u32>,
    /// Per room, members per locality, sorted by locality.
    pub room_loc: Vec<(u16, u32)>,
    /// Person → rooms offsets.
    pub person_room_off: Vec<u32>,
    /// Rooms of each person.
    pub person_rooms: Vec<u32>,
    /// Persons per locality.
    pub locality_pop: Vec<u32>,
}

fn poisson(r: &mut Rng, mean: f64) -> u32 {
    if mean <= 0.0 {
        return 0;
    }
    if mean > 30.0 {
        return (mean + mean.sqrt() * r.gauss()).round().max(0.0) as u32;
    }
    let l = (-mean).exp();
    let (mut k, mut p) = (0u32, 1.0f64);
    loop {
        p *= r.f();
        if p <= l {
            return k;
        }
        k += 1;
    }
}

/// Index into a cumulative weight table by a uniform draw.
fn pick_cum(cum: &[f64], u: f64) -> usize {
    let x = u * cum.last().copied().unwrap_or(0.0);
    cum.partition_point(|&c| c <= x).min(cum.len().saturating_sub(1))
}

fn cumulative(w: impl Iterator<Item = f64>) -> Vec<f64> {
    let mut s = 0.0;
    w.map(|x| {
        s += x.max(0.0);
        s
    })
    .collect()
}

fn shuffle<T>(r: &mut Rng, v: &mut [T]) {
    for i in (1..v.len()).rev() {
        v.swap(i, r.below(i + 1));
    }
}

/// Chunk a list into rooms whose sizes come from `size`.
fn chunk_rooms(list: &[u32], mut size: impl FnMut() -> usize, out: &mut Vec<Vec<u32>>) {
    let mut i = 0;
    while i < list.len() {
        let s = size().max(1);
        let j = (i + s).min(list.len());
        out.push(list[i..j].to_vec());
        i = j;
    }
}

impl SocialWorld {
    /// Build `n` persons over `geo` with `p`, seeded. Each mechanism draws from its own
    /// sub-stream, so changing one parameter does not reshuffle the others.
    #[must_use]
    pub fn build(geo: &Geography, p: &SocialParams, n: usize, seed: u64, placement: Placement) -> Self {
        let placeless = placement == Placement::Placeless;
        let n_loc = geo.localities();
        let n_reg = geo.regions().max(1);
        let stream = |salt: u64| Rng::new(seed.wrapping_mul(0x9E37_79B9).wrapping_add(salt));

        // households and persons
        let mut r = stream(1);
        let cell_cum = cumulative(geo.cells.iter().map(|c| c.weight));
        let mut persons: Vec<Person> = Vec::with_capacity(n);
        let mut hh_members: Vec<Vec<u32>> = Vec::new();
        while persons.len() < n {
            let c = pick_cum(&cell_cum, r.f());
            let cell = &geo.cells[c];
            let size = (p.persons_per_household + p.household_size_sd * r.gauss()).round().clamp(1.0, 6.0) as usize;
            let h = hh_members.len() as u32;
            let mut mem = Vec::new();
            for _ in 0..size.min(n - persons.len()) {
                let u = r.f();
                let role = if u < p.student_share {
                    Role::Student
                } else if u < p.student_share + p.worker_share {
                    Role::Worker
                } else {
                    Role::Other
                };
                mem.push(persons.len() as u32);
                persons.push(Person {
                    cell: c as u32,
                    locality: cell.locality,
                    region: geo.region_of_locality[cell.locality as usize],
                    household: h,
                    role,
                    lon: cell.lon,
                });
            }
            hh_members.push(mem);
        }
        let n_hh = hh_members.len();
        let mut locality_pop = vec![0u32; n_loc];
        for q in &persons {
            locality_pop[q.locality as usize] += 1;
        }
        // households by locality (sorted by cell, so chunks are neighbours first)
        let mut hh_by_loc: Vec<Vec<u32>> = vec![Vec::new(); n_loc];
        for (h, m) in hh_members.iter().enumerate() {
            hh_by_loc[persons[m[0] as usize].locality as usize].push(h as u32);
        }
        for v in &mut hh_by_loc {
            v.sort_by_key(|&h| persons[hh_members[h as usize][0] as usize].cell);
        }
        let loc_cum = cumulative(locality_pop.iter().map(|&x| x as f64));
        let mut loc_in_region: Vec<Vec<u16>> = vec![Vec::new(); n_reg];
        for k in 0..n_loc {
            if locality_pop[k] > 0 {
                loc_in_region[geo.region_of_locality[k] as usize].push(k as u16);
            }
        }
        let reg_cum: Vec<Vec<f64>> = loc_in_region.iter().map(|ls| cumulative(ls.iter().map(|&k| locality_pop[k as usize] as f64))).collect();
        // a locality other than `own`, drawn ∝ persons, in the same region or not
        let other_locality = |r: &mut Rng, own: usize, same_region: bool| -> Option<usize> {
            let g = geo.region_of_locality[own] as usize;
            for _ in 0..64 {
                let k = if same_region {
                    if loc_in_region[g].len() < 2 {
                        return None;
                    }
                    loc_in_region[g][pick_cum(&reg_cum[g], r.f())] as usize
                } else {
                    pick_cum(&loc_cum, r.f())
                };
                if k != own && (same_region || geo.region_of_locality[k] as usize != g) && !hh_by_loc[k].is_empty() {
                    return Some(k);
                }
            }
            None
        };

        let mut rooms: Vec<(RoomKind, Vec<u32>)> = Vec::new();
        for m in &hh_members {
            rooms.push((RoomKind::Household, m.clone()));
        }

        // extended family
        let mut r = stream(2);
        for h in 0..n_hh {
            let own = persons[hh_members[h][0] as usize].locality as usize;
            let extra = poisson(&mut r, (p.kin_households_mean - 1.0).max(0.0));
            let mut hs = vec![h as u32];
            for _ in 0..extra {
                let k = if placeless {
                    None
                } else if r.chance(p.diaspora_share) {
                    let abroad = r.chance(p.diaspora_abroad_share);
                    other_locality(&mut r, own, !abroad).or_else(|| other_locality(&mut r, own, abroad))
                } else {
                    Some(own)
                };
                let pick = match k {
                    Some(k) => hh_by_loc[k][r.below(hh_by_loc[k].len())],
                    None => r.below(n_hh) as u32,
                };
                hs.push(pick);
            }
            hs.sort_unstable();
            hs.dedup();
            if hs.len() > 1 {
                rooms.push((RoomKind::Kin, hs.iter().flat_map(|&x| hh_members[x as usize].iter().copied()).collect()));
            }
        }

        // workplace teams, grouped by the team's locality
        let mut r = stream(3);
        let key = |k: usize| if placeless { 0 } else { k };
        let mut by_work: Vec<Vec<u32>> = vec![Vec::new(); n_loc];
        for (i, q) in persons.iter().enumerate() {
            if q.role != Role::Worker {
                continue;
            }
            let own = q.locality as usize;
            let k = if !placeless && r.chance(p.work_cross_locality_share) {
                let cross_region = r.chance(p.work_cross_region_share);
                other_locality(&mut r, own, !cross_region).unwrap_or(own)
            } else {
                own
            };
            by_work[key(k)].push(i as u32);
        }
        for list in &mut by_work {
            shuffle(&mut r, list);
            if !placeless {
                list.sort_by_key(|&i| persons[i as usize].cell);
            }
            let mut rs = Rng::new(r.next_u64());
            let mut out = Vec::new();
            chunk_rooms(list, || (p.work_team_median * (p.work_team_sigma * rs.gauss()).exp()).round().clamp(2.0, p.work_team_max) as usize, &mut out);
            rooms.extend(out.into_iter().map(|m| (RoomKind::Work, m)));
        }

        // school classes, by locality, neighbours first
        let mut r = stream(4);
        let mut by_school: Vec<Vec<u32>> = vec![Vec::new(); n_loc];
        for (i, q) in persons.iter().enumerate() {
            if q.role == Role::Student {
                by_school[key(q.locality as usize)].push(i as u32);
            }
        }
        for list in &mut by_school {
            shuffle(&mut r, list);
            if !placeless {
                list.sort_by_key(|&i| persons[i as usize].cell);
            }
            let mut out = Vec::new();
            chunk_rooms(list, || p.class_size.round().max(1.0) as usize, &mut out);
            rooms.extend(out.into_iter().map(|m| (RoomKind::School, m)));
        }

        // neighbourhood rooms: joining households of a locality, neighbours first
        let mut r = stream(5);
        let mut by_hood: Vec<Vec<u32>> = vec![Vec::new(); n_loc];
        for (k, hs) in hh_by_loc.iter().enumerate() {
            for &h in hs {
                if r.chance(p.community_join_share) {
                    by_hood[key(k)].push(h);
                }
            }
        }
        for list in &mut by_hood {
            if placeless {
                shuffle(&mut r, list);
            }
            let mut out = Vec::new();
            chunk_rooms(list, || p.neighbourhood_room_households.round().max(1.0) as usize, &mut out);
            for hs in out {
                rooms.push((RoomKind::Neighbourhood, hs.iter().flat_map(|&h| hh_members[h as usize].iter().copied()).collect()));
            }
        }

        // interest groups and public rooms: pooled by reach
        let pools = |salt: u64, per_person: f64, mean_size: f64, local: f64, regional: f64, alpha: f64, kind: RoomKind, rooms: &mut Vec<(RoomKind, Vec<u32>)>| {
            let mut r = stream(salt);
            // pool index: locality k, then n_loc + region g, then n_loc + n_reg (global)
            let n_pool = n_loc + n_reg + 1;
            let mut joins: Vec<(u32, u32)> = Vec::new();
            let mut demand = vec![0u32; n_pool];
            for (i, q) in persons.iter().enumerate() {
                for _ in 0..poisson(&mut r, per_person) {
                    let u = r.f();
                    let pool = if placeless {
                        n_loc + n_reg
                    } else if u < local {
                        q.locality as usize
                    } else if u < local + regional {
                        n_loc + q.region as usize
                    } else {
                        n_loc + n_reg
                    };
                    demand[pool] += 1;
                    joins.push((i as u32, pool as u32));
                }
            }
            let mut base = vec![0usize; n_pool];
            let mut cums: Vec<Vec<f64>> = Vec::with_capacity(n_pool);
            let first = rooms.len();
            for (pool, &d) in demand.iter().enumerate() {
                base[pool] = rooms.len() - first;
                let count = if d == 0 { 0 } else { ((f64::from(d) / mean_size).round() as usize).max(1) };
                cums.push(cumulative((0..count).map(|_| (1.0 - r.f()).powf(-1.0 / alpha))));
                for _ in 0..count {
                    rooms.push((kind, Vec::new()));
                }
            }
            for (i, pool) in joins {
                let c = &cums[pool as usize];
                let j = pick_cum(c, r.f());
                rooms[first + base[pool as usize] + j].1.push(i);
            }
        };
        pools(6, p.interest_groups_per_person, p.interest_group_mean_size, p.interest_local_share, p.interest_regional_share, p.interest_pareto_alpha, RoomKind::Interest, &mut rooms);
        pools(7, p.public_follows_per_person, p.public_room_mean_size, p.public_local_share, p.public_regional_share, p.public_pareto_alpha, RoomKind::Public, &mut rooms);

        // flatten
        let mut room_kind = Vec::with_capacity(rooms.len());
        let mut room_off = vec![0u32];
        let mut room_mem = Vec::new();
        let mut room_loc_off = vec![0u32];
        let mut room_loc: Vec<(u16, u32)> = Vec::new();
        for (kind, mut m) in rooms {
            m.sort_unstable();
            m.dedup();
            let mut locs: Vec<u16> = m.iter().map(|&i| persons[i as usize].locality).collect();
            locs.sort_unstable();
            let start = *room_loc_off.last().unwrap() as usize;
            for l in locs {
                let fresh = room_loc.len() == start;
                match room_loc.last_mut() {
                    Some(last) if !fresh && last.0 == l => last.1 += 1,
                    _ => room_loc.push((l, 1)),
                }
            }
            room_loc_off.push(room_loc.len() as u32);
            room_mem.extend_from_slice(&m);
            room_off.push(room_mem.len() as u32);
            room_kind.push(kind);
        }
        let mut cnt = vec![0u32; n + 1];
        for &m in &room_mem {
            cnt[m as usize + 1] += 1;
        }
        for i in 0..n {
            cnt[i + 1] += cnt[i];
        }
        let mut person_rooms = vec![0u32; room_mem.len()];
        let mut pos = cnt.clone();
        for rm in 0..room_kind.len() {
            for &m in &room_mem[room_off[rm] as usize..room_off[rm + 1] as usize] {
                person_rooms[pos[m as usize] as usize] = rm as u32;
                pos[m as usize] += 1;
            }
        }
        Self {
            region_of_locality: geo.region_of_locality.clone(),
            persons,
            households: n_hh,
            room_kind,
            room_off,
            room_mem,
            room_loc_off,
            room_loc,
            person_room_off: cnt,
            person_rooms,
            locality_pop,
        }
    }

    /// Members of a room.
    #[must_use]
    pub fn members(&self, room: u32) -> &[u32] {
        &self.room_mem[self.room_off[room as usize] as usize..self.room_off[room as usize + 1] as usize]
    }

    /// Members of a room per locality.
    #[must_use]
    pub fn room_localities(&self, room: u32) -> &[(u16, u32)] {
        &self.room_loc[self.room_loc_off[room as usize] as usize..self.room_loc_off[room as usize + 1] as usize]
    }

    /// Rooms a person is in.
    #[must_use]
    pub fn rooms_of(&self, person: u32) -> &[u32] {
        &self.person_rooms[self.person_room_off[person as usize] as usize..self.person_room_off[person as usize + 1] as usize]
    }

    /// Number of rooms.
    #[must_use]
    pub fn rooms(&self) -> usize {
        self.room_kind.len()
    }

    /// Number of localities.
    #[must_use]
    pub fn localities(&self) -> usize {
        self.region_of_locality.len()
    }

    /// Person ties from room co-membership (public follows excluded), as an undirected CSR
    /// graph with sorted neighbour lists. Rooms up to `clique_cap` are cliques; in a larger room
    /// each member ties to `large_group_ties` random co-members.
    #[must_use]
    pub fn ties(&self, p: &SocialParams, seed: u64) -> TieGraph {
        let mut r = Rng::new(seed ^ 0x7469_6573);
        let cap = p.clique_cap.max(2.0) as usize;
        let k = p.large_group_ties.max(0.0) as usize;
        let mut e: Vec<u64> = Vec::new();
        let pack = |a: u32, b: u32| if a < b { (u64::from(a) << 32) | u64::from(b) } else { (u64::from(b) << 32) | u64::from(a) };
        for rm in 0..self.rooms() {
            if self.room_kind[rm] == RoomKind::Public {
                continue;
            }
            let m = self.members(rm as u32);
            if m.len() < 2 {
                continue;
            }
            if m.len() <= cap {
                for i in 0..m.len() {
                    for j in i + 1..m.len() {
                        e.push(pack(m[i], m[j]));
                    }
                }
            } else {
                for &a in m {
                    for _ in 0..k {
                        let b = m[r.below(m.len())];
                        if a != b {
                            e.push(pack(a, b));
                        }
                    }
                }
            }
        }
        e.sort_unstable();
        e.dedup();
        let n = self.persons.len();
        let mut off = vec![0u32; n + 1];
        for &x in &e {
            off[(x >> 32) as usize + 1] += 1;
            off[(x & 0xFFFF_FFFF) as usize + 1] += 1;
        }
        for i in 0..n {
            off[i + 1] += off[i];
        }
        let mut adj = vec![0u32; off[n] as usize];
        let mut pos = off.clone();
        for &x in &e {
            let (a, b) = ((x >> 32) as usize, (x & 0xFFFF_FFFF) as usize);
            adj[pos[a] as usize] = b as u32;
            pos[a] += 1;
            adj[pos[b] as usize] = a as u32;
            pos[b] += 1;
        }
        for i in 0..n {
            adj[off[i] as usize..off[i + 1] as usize].sort_unstable();
        }
        TieGraph { off, adj, edges: e.len() }
    }
}

// ------------------------------------------------------------------ graph statistics

/// Undirected person-tie graph (CSR, sorted neighbour lists).
#[derive(Debug, Clone, PartialEq)]
pub struct TieGraph {
    /// Offsets.
    pub off: Vec<u32>,
    /// Neighbours.
    pub adj: Vec<u32>,
    /// Undirected edge count.
    pub edges: usize,
}

impl TieGraph {
    /// Neighbours of `i`.
    #[must_use]
    pub fn nb(&self, i: usize) -> &[u32] {
        &self.adj[self.off[i] as usize..self.off[i + 1] as usize]
    }
    /// Degree of `i`.
    #[must_use]
    pub fn degree(&self, i: usize) -> usize {
        (self.off[i + 1] - self.off[i]) as usize
    }
}

/// Statistics of a tie graph (outputs).
#[derive(Debug, Clone, PartialEq)]
pub struct GraphStats {
    /// Persons.
    pub persons: usize,
    /// Undirected ties.
    pub ties: usize,
    /// Mean degree.
    pub mean_degree: f64,
    /// Median degree.
    pub median_degree: f64,
    /// 90th percentile degree.
    pub p90_degree: f64,
    /// 99th percentile degree.
    pub p99_degree: f64,
    /// Largest degree.
    pub max_degree: f64,
    /// Coefficient of variation of degree.
    pub degree_cv: f64,
    /// Share of persons with no tie.
    pub isolated_share: f64,
    /// Mean local clustering coefficient over persons of degree >= 2 (sampled).
    pub clustering: f64,
    /// Newman modularity of the partition by locality.
    pub modularity_locality: f64,
    /// Newman modularity of the partition by region.
    pub modularity_region: f64,
    /// Degree assortativity (Pearson over edge ends).
    pub assortativity: f64,
    /// Share of ties whose ends are in different localities.
    pub cross_locality_tie_share: f64,
    /// Share of ties whose ends are in different regions (the twin's stand-in for abroad).
    pub cross_region_tie_share: f64,
}

fn quantile_sorted(v: &[f64], q: f64) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    let i = ((v.len() - 1) as f64 * q).round() as usize;
    v[i.min(v.len() - 1)]
}

impl GraphStats {
    /// Compute over `g` on `w`; clustering averages over up to `sample` persons (seeded).
    #[must_use]
    pub fn of(w: &SocialWorld, g: &TieGraph, sample: usize, seed: u64) -> Self {
        let n = w.persons.len();
        let mut deg: Vec<f64> = (0..n).map(|i| g.degree(i) as f64).collect();
        let m2 = (2 * g.edges) as f64;
        let mean = if n > 0 { m2 / n as f64 } else { 0.0 };
        let var = deg.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / n.max(1) as f64;
        let isolated = deg.iter().filter(|&&d| d == 0.0).count() as f64 / n.max(1) as f64;

        // clustering
        let mut r = Rng::new(seed ^ 0x636C_7573);
        let cand: Vec<usize> = (0..n).filter(|&i| g.degree(i) >= 2).collect();
        let take = sample.min(cand.len());
        let mut csum = 0.0;
        for s in 0..take {
            let i = if take == cand.len() { cand[s] } else { cand[r.below(cand.len())] };
            let nb = g.nb(i);
            let mut links = 0usize;
            for (a_i, &a) in nb.iter().enumerate() {
                let na = g.nb(a as usize);
                // count b in nb after a that are neighbours of a (sorted merge)
                let rest = &nb[a_i + 1..];
                let (mut x, mut y) = (0, 0);
                while x < rest.len() && y < na.len() {
                    match rest[x].cmp(&na[y]) {
                        std::cmp::Ordering::Less => x += 1,
                        std::cmp::Ordering::Greater => y += 1,
                        std::cmp::Ordering::Equal => {
                            links += 1;
                            x += 1;
                            y += 1;
                        }
                    }
                }
            }
            let d = nb.len() as f64;
            csum += 2.0 * links as f64 / (d * (d - 1.0));
        }
        let clustering = if take > 0 { csum / take as f64 } else { 0.0 };

        // modularity, assortativity, crossing shares
        let n_loc = w.localities();
        let n_reg = w.region_of_locality.iter().map(|&x| x as usize + 1).max().unwrap_or(1);
        let (mut in_loc, mut in_reg) = (0.0, 0.0);
        let mut dl = vec![0.0f64; n_loc];
        let mut dr = vec![0.0f64; n_reg];
        let (mut sxy, mut sx, mut sx2) = (0.0, 0.0, 0.0);
        for i in 0..n {
            let (li, ri) = (w.persons[i].locality as usize, w.persons[i].region as usize);
            dl[li] += deg[i];
            dr[ri] += deg[i];
            for &j in g.nb(i) {
                let j = j as usize;
                if w.persons[j].locality as usize == li {
                    in_loc += 1.0;
                }
                if w.persons[j].region as usize == ri {
                    in_reg += 1.0;
                }
                let (a, b) = (deg[i], deg[j]);
                sxy += a * b;
                sx += a;
                sx2 += a * a;
            }
        }
        let q = |inside: f64, d: &[f64]| if m2 > 0.0 { inside / m2 - d.iter().map(|x| (x / m2).powi(2)).sum::<f64>() } else { 0.0 };
        let assort = if m2 > 0.0 {
            let (exy, ex, ex2) = (sxy / m2, sx / m2, sx2 / m2);
            let den = ex2 - ex * ex;
            if den > 0.0 { (exy - ex * ex) / den } else { 0.0 }
        } else {
            0.0
        };
        deg.sort_by(|a, b| a.partial_cmp(b).unwrap());
        Self {
            persons: n,
            ties: g.edges,
            mean_degree: mean,
            median_degree: quantile_sorted(&deg, 0.5),
            p90_degree: quantile_sorted(&deg, 0.9),
            p99_degree: quantile_sorted(&deg, 0.99),
            max_degree: deg.last().copied().unwrap_or(0.0),
            degree_cv: if mean > 0.0 { var.sqrt() / mean } else { 0.0 },
            isolated_share: isolated,
            clustering,
            modularity_locality: q(in_loc, &dl),
            modularity_region: q(in_reg, &dr),
            assortativity: assort,
            cross_locality_tie_share: if m2 > 0.0 { 1.0 - in_loc / m2 } else { 0.0 },
            cross_region_tie_share: if m2 > 0.0 { 1.0 - in_reg / m2 } else { 0.0 },
        }
    }
}

/// A band a realistic graph should fall in, stated before any run.
#[derive(Debug, Clone, Copy)]
pub struct Band {
    /// Statistic name (a [`GraphStats`] field).
    pub name: &'static str,
    /// Lower bound.
    pub lo: f64,
    /// Upper bound.
    pub hi: f64,
    /// Class and the kind of source.
    pub source: &'static str,
}

/// Bands for a strong-tie personal network (MODEL bands: literature values quoted from memory,
/// not read from the papers in this session; each names the kind of measurement).
pub const BANDS: &[Band] = &[
    Band { name: "mean_degree", lo: 10.0, hi: 150.0, source: "MODEL: personal-network size studies put active networks at tens to ~150 (Dunbar-scale); a strong-tie graph sits inside that" },
    Band { name: "degree_cv", lo: 0.5, hi: 3.0, source: "MODEL: degree distributions of communication networks are right-skewed (lognormal / truncated power law), CV well above Poisson's" },
    Band { name: "clustering", lo: 0.10, hi: 0.60, source: "MODEL: mobile-call and online friendship graphs report mean local clustering ~0.1-0.6 depending on degree (e.g. Onnela et al. 2007; Ugander et al. 2011)" },
    Band { name: "assortativity", lo: 0.0, hi: 0.40, source: "MODEL: social networks mix assortatively by degree, r > 0 (Newman 2002/2003), unlike technological networks" },
    Band { name: "modularity_locality", lo: 0.30, hi: 0.95, source: "MODEL: communities recovered from phone networks follow geography closely; with ~181 localities a locality partition should be strongly modular" },
    Band { name: "cross_locality_tie_share", lo: 0.03, hi: 0.37, source: "MODEL: Facebook Social Connectedness Index work reports ~63% of US users' friends within 100 miles (Bailey et al. 2018), so <= ~37% beyond; a twin locality is far wider than 100 miles and strong ties are more local" },
    Band { name: "cross_region_tie_share", lo: 0.005, hi: 0.10, source: "MODEL: most ties are domestic; ~3.6% of people are international migrants (UN DESA 2020, quoted from memory)" },
];

impl GraphStats {
    /// Value of a named statistic.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<f64> {
        Some(match name {
            "mean_degree" => self.mean_degree,
            "degree_cv" => self.degree_cv,
            "clustering" => self.clustering,
            "assortativity" => self.assortativity,
            "modularity_locality" => self.modularity_locality,
            "modularity_region" => self.modularity_region,
            "cross_locality_tie_share" => self.cross_locality_tie_share,
            "cross_region_tie_share" => self.cross_region_tie_share,
            _ => return None,
        })
    }

    /// Each band with this graph's value and whether it falls inside.
    #[must_use]
    pub fn against_bands(&self) -> Vec<(Band, f64, bool)> {
        BANDS
            .iter()
            .map(|b| {
                let v = self.get(b.name).unwrap_or(f64::NAN);
                (*b, v, v >= b.lo && v <= b.hi)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn small(placement: Placement, p: &SocialParams, seed: u64) -> SocialWorld {
        SocialWorld::build(&Geography::stand_in(3, 20, 4, 4), p, 3000, seed, placement)
    }

    #[test]
    fn spec_defaults_match_struct() {
        let d = SocialParams::default();
        assert_eq!(SOCIAL_PARAM_SPECS.len(), 27);
        assert_eq!(SOCIAL_PARAM_SPECS[0].name, "persons_per_household");
        assert_eq!(SOCIAL_PARAM_SPECS[0].default, d.persons_per_household);
        // the household draw agrees with the micro tier's default
        let micro = crate::micro::PARAM_SPECS.iter().find(|s| s.name == "persons_per_household").unwrap();
        assert_eq!(micro.default, d.persons_per_household);
        let join = crate::micro::PARAM_SPECS.iter().find(|s| s.name == "community_join_share").unwrap();
        assert_eq!(join.default, d.community_join_share);
    }

    #[test]
    fn base64_roundtrip_known() {
        assert_eq!(base64_decode("TWFu").unwrap(), b"Man");
        assert_eq!(base64_decode("TWE=").unwrap(), b"Ma");
        assert_eq!(base64_decode("TQ==").unwrap(), b"M");
        assert!(base64_decode("T*").is_err());
    }

    #[test]
    fn page_grid_parses() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/globe.js");
        let src = std::fs::read_to_string(path).expect("docs/globe.js is in the repo");
        let g = Geography::from_globe_js(&src).unwrap();
        assert_eq!(g.cells.len(), 4559);
        assert_eq!(g.localities(), 181);
        assert_eq!(g.regions(), 9);
    }

    #[test]
    fn build_is_deterministic_by_seed() {
        let p = SocialParams::default();
        let a = small(Placement::Placed, &p, 7);
        let b = small(Placement::Placed, &p, 7);
        let c = small(Placement::Placed, &p, 8);
        assert_eq!(a, b);
        assert_ne!(a.room_mem, c.room_mem);
        let (ga, gb) = (a.ties(&p, 7), b.ties(&p, 7));
        assert_eq!(GraphStats::of(&a, &ga, 500, 1), GraphStats::of(&b, &gb, 500, 1));
    }

    #[test]
    fn csr_is_consistent() {
        let p = SocialParams::default();
        let w = small(Placement::Placed, &p, 1);
        assert_eq!(w.persons.len(), 3000);
        for rm in 0..w.rooms() as u32 {
            let total: u32 = w.room_localities(rm).iter().map(|x| x.1).sum();
            assert_eq!(total as usize, w.members(rm).len());
            for &m in w.members(rm) {
                assert!(w.rooms_of(m).contains(&rm));
            }
        }
        // every person has a household room
        for i in 0..w.persons.len() as u32 {
            assert!(w.rooms_of(i).iter().any(|&r| w.room_kind[r as usize] == RoomKind::Household));
        }
    }

    #[test]
    fn no_cross_ties_when_everything_is_local() {
        let mut p = SocialParams::default();
        p.diaspora_share = 0.0;
        p.work_cross_locality_share = 0.0;
        p.interest_local_share = 1.0;
        p.interest_regional_share = 0.0;
        let w = small(Placement::Placed, &p, 2);
        let s = GraphStats::of(&w, &w.ties(&p, 2), 300, 2);
        assert_eq!(s.cross_locality_tie_share, 0.0);
    }

    #[test]
    fn placeless_ties_cross_at_one_minus_one_over_l() {
        let mut p = SocialParams::default();
        p.persons_per_household = 1.0;
        p.household_size_sd = 0.0;
        let l = 10;
        let w = SocialWorld::build(&Geography::uniform(l, 2), &p, 5000, 4, Placement::Placeless);
        let s = GraphStats::of(&w, &w.ties(&p, 4), 300, 4);
        // a tie between two uniform draws over 10 near-equal localities is local w.p. ~0.1;
        // 5000 persons over 10 cells by multinomial puts each share within ~0.005 of 0.1
        assert!((s.cross_locality_tie_share - 0.9).abs() < 0.02, "{}", s.cross_locality_tie_share);
    }
}
