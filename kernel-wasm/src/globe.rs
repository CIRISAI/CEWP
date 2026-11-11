//! What the globe draws, computed here: the simulation's clock and sun, the flows one tick moved
//! between localities, one room's CC 6.1.6 member relay tree (cewp_sim::alm), one room's chunk
//! custody (cewp_sim::chunk_custody) and a seat's load across device counts (cewp_sim::capacity).
//!
//! Every buffer is flat (f64 or u32), read by the page through `globe_buf_ptr` / `globe_buf_len`.
//! The page draws these numbers and nothing it invents. Classes, as in FSD §8.2:
//!   clock / sun position .......... MODEL (circular orbit, no equation of time; day of year is a
//!                                    scenario parameter)
//!   diurnal multiplier ............ MODEL (cewp_sim::workload::diurnal_multiplier, FSD §4.1)
//!   flows, locality acts .......... DERIVED (counted inside `tick`, the same events as out[5..8])
//!   relay tree .................... DERIVED by alm.rs from MODEL inputs (uplink per device class,
//!                                    RTT from distance, every member trusts every member at depth 1)
//!   custody ....................... DERIVED by chunk_custody.rs from the room's membership
//!   seat curve .................... as `cap_seat` (calibrated to seat-1; MEASURED/DERIVED/MODEL per
//!                                    capacity.rs COST_SPECS)

use cewp_sim::alm::{
    compute_alm_topology_verified_with_publisher, CapacityVerification, Epoch, ReachabilityObservation,
    ReceiverLayerPolicy, RelayCapacity, SignedRelayCapacity, StreamId, TrustGrant, VerifiedCapacityAd,
    VerifiedTopologyInputSnapshot, DEFAULT_SIM_FANOUT_CAP, MAX_USEFUL_RTT_MS,
};
use cewp_sim::capacity::{self, CostParams, ReadSet, RoundShape, Scenario, SeatSelect};
use cewp_sim::chunk_custody::{classify, effective_source_count, NoiseFloorState};
use cewp_sim::workload::{diurnal_multiplier, SECONDS_PER_DAY};

/// Localities the population grid carries (the `tick` locality table is this size).
pub const NLOC: usize = 181;
/// Largest value `diurnal_multiplier` returns (its 2-harmonic peak, ~2.30 at 20:30 local); the
/// acceptance draw divides by it so the busiest hour is always accepted.
pub const DIURNAL_PEAK: f64 = 2.30;

pub struct GlobeState {
    /// Sim time at the start of the next tick, seconds since the sim epoch (00:00 UTC, day `doy`).
    pub t0: f64,
    /// Sim seconds one tick spans; 0 = the legacy spacing of 137 s per act.
    pub span: f64,
    /// Day of year for the sun's declination (MODEL scenario parameter).
    pub doy: f64,
    /// 1 = signers are drawn in proportion to their local diurnal multiplier, and receivers are marked
    /// in proportion to it (rx_mark). On by default; clock_set(.., 0) turns it off.
    pub diurnal: bool,
    /// Cross-locality deliveries of the last tick, NLOC × NLOC row-major (from, to).
    pub flows: Vec<u32>,
    /// Acts signed per locality in the last tick (signer's locality).
    pub loc_acts: Vec<u32>,
    /// Signers rejected by the diurnal draw in the last tick (redrawn, so `acts` is unchanged).
    pub redraws: u32,
    pub sun: Vec<f64>,
    pub cent: Vec<f64>,
    pub alm_members: Vec<u32>,
    pub alm_edges: Vec<u32>,
    pub alm_unrooted: Vec<u32>,
    pub alm_member_f: Vec<f64>,
    pub custody: Vec<f64>,
    pub custody_holders: Vec<u32>,
    pub cap_curve: Vec<f64>,
    pub diurnal_bins: Vec<f64>,
    cost: Option<CostParams>,
}

impl GlobeState {
    /// The state a page starts with (diurnal on).
    #[must_use]
    pub fn fresh() -> Self {
        Self {
            t0: 43200.0,
            span: 0.0,
            doy: 314.0,
            diurnal: true,
            flows: vec![0; NLOC * NLOC],
            loc_acts: vec![0; NLOC],
            redraws: 0,
            sun: vec![0.0; 8],
            cent: Vec::new(),
            alm_members: Vec::new(),
            alm_edges: Vec::new(),
            alm_unrooted: Vec::new(),
            alm_member_f: Vec::new(),
            custody: Vec::new(),
            custody_holders: Vec::new(),
            cap_curve: Vec::new(),
            diurnal_bins: Vec::new(),
            cost: None,
        }
    }
}

static mut GLOBE: Option<GlobeState> = None;

pub fn globe() -> &'static mut GlobeState {
    // SAFETY: wasm32-unknown-unknown runs this module on one thread; the page calls in turn.
    unsafe {
        let g = &mut *std::ptr::addr_of_mut!(GLOBE);
        g.get_or_insert_with(GlobeState::fresh)
    }
}

/// Sim time of act `i` of `acts` in the tick about to run. With no span set this is the legacy
/// spacing (43,200 s + 137 s per act), so a page that never calls `clock_set` sees the old tick.
#[inline]
pub fn act_time(g: &GlobeState, i: u32, acts: u32) -> f64 {
    if g.span > 0.0 { g.t0 + g.span * (i as f64) / (acts.max(1) as f64) } else { g.t0 + (i as f64) * 137.0 }
}

/// Local solar seconds at longitude `lon_deg` for sim time `t`.
#[inline]
pub fn local_sec(t: f64, lon_deg: f64) -> f64 {
    (t + (lon_deg / 360.0) * SECONDS_PER_DAY).rem_euclid(SECONDS_PER_DAY)
}

/// The subsolar point (degrees) at sim time `t` on day `doy`: declination from a circular orbit
/// (23.44° tilt, solstice on day 172), longitude from UTC with no equation of time. MODEL.
pub fn subsolar(t: f64, doy: f64) -> (f64, f64) {
    let day = doy + (t / SECONDS_PER_DAY).floor();
    let decl = 23.44 * (2.0 * std::f64::consts::PI * (day - 172.0) / 365.25).cos();
    let utc_h = t.rem_euclid(SECONDS_PER_DAY) / 3600.0;
    let lon = (-15.0 * (utc_h - 12.0) + 540.0).rem_euclid(360.0) - 180.0;
    (decl, lon)
}

/// Set the sim clock for the next `tick`: start time `t_s` (sim seconds), the span one tick covers
/// `span_s` (0 = legacy spacing), the day of year, and whether signers follow the diurnal wave.
#[no_mangle]
pub extern "C" fn clock_set(t_s: f64, span_s: f64, doy: f64, diurnal: u32) {
    let g = globe();
    g.t0 = t_s;
    g.span = span_s.max(0.0);
    g.doy = doy;
    g.diurnal = diurnal != 0;
}

/// The sun at sim time `t_s`: fills buffer 0 with [subsolar lat°, subsolar lon°, UTC hour, day of
/// year, diurnal multiplier at the subsolar point, diurnal peak, 0, 0]. Returns the length.
#[no_mangle]
pub extern "C" fn clock_sun(t_s: f64) -> u32 {
    let g = globe();
    let (la, lo) = subsolar(t_s, g.doy);
    let day = g.doy + (t_s / SECONDS_PER_DAY).floor();
    g.sun = vec![la, lo, t_s.rem_euclid(SECONDS_PER_DAY) / 3600.0, day, diurnal_multiplier(local_sec(t_s, lo)), DIURNAL_PEAK, 0.0, 0.0];
    g.sun.len() as u32
}

/// The diurnal multiplier (workload.rs) at sim time `t_s` for `bins` longitudes spaced evenly from
/// -180° (bin centres). Buffer 1. Returns the length.
#[no_mangle]
pub extern "C" fn clock_diurnal_bins(t_s: f64, bins: u32) -> u32 {
    let g = globe();
    let b = bins.clamp(1, 1440) as usize;
    g.diurnal_bins.clear();
    for i in 0..b {
        let lon = -180.0 + 360.0 * (i as f64 + 0.5) / b as f64;
        g.diurnal_bins.push(diurnal_multiplier(local_sec(t_s, lon)));
    }
    b as u32
}

/// Called by `tick` before its first act.
pub fn tick_begin(g: &mut GlobeState) {
    for v in g.flows.iter_mut() { *v = 0; }
    for v in g.loc_acts.iter_mut() { *v = 0; }
    g.redraws = 0;
}

/// Called by `tick` for every delivery counted in out[6] or out[7].
#[inline]
pub fn flow(g: &mut GlobeState, from: usize, to: usize) {
    if from < NLOC && to < NLOC {
        g.flows[from * NLOC + to] = g.flows[from * NLOC + to].saturating_add(1);
    }
}

/// The diurnal acceptance draw for a signer at longitude `lon` at sim time `t`: true = signs now.
/// `u` is a uniform draw in 0..1000 from the tick's own stream.
#[inline]
pub fn diurnal_accept(t: f64, lon: f64, u: u32) -> bool {
    let p = diurnal_multiplier(local_sec(t, lon)) / DIURNAL_PEAK;
    (u as f64) < p * 1000.0
}

/// How brightly a receiving node is marked in the activity buffer: `base` scaled by its local
/// diurnal multiplier over the peak (workload.rs, MODEL) while the diurnal switch is on, so a device
/// in its night shows little live delivery (asleep: it takes the rows later, from its mailbox or its
/// next round); `base` with the switch off. Display only: no out slot and no draw depends on it.
#[inline]
pub fn rx_mark(g: &GlobeState, t: f64, lon: f64, base: u8) -> u8 {
    if !g.diurnal {
        return base;
    }
    let w = diurnal_multiplier(local_sec(t, lon)) / DIURNAL_PEAK;
    (f64::from(base) * w.clamp(0.0, 1.0)).round() as u8
}

/// Mean position of each locality's nodes (unit-vector mean, so it is right across the
/// antimeridian). Buffer 2, f64 stride 3: lat°, lon°, node count. Returns the locality count.
#[no_mangle]
pub extern "C" fn locality_centroids(lat: *const f32, lon: *const f32, metro: *const u16, n: u32) -> u32 {
    let g = globe();
    let mut acc = vec![[0f64; 4]; NLOC];
    unsafe {
        for i in 0..n as usize {
            let k = *metro.add(i) as usize;
            if k >= NLOC { continue; }
            let (a, o) = ((*lat.add(i) as f64).to_radians(), (*lon.add(i) as f64).to_radians());
            acc[k][0] += a.cos() * o.sin();
            acc[k][1] += a.sin();
            acc[k][2] += a.cos() * o.cos();
            acc[k][3] += 1.0;
        }
    }
    g.cent.clear();
    for v in acc {
        let r = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        if v[3] == 0.0 || r == 0.0 { g.cent.extend_from_slice(&[0.0, 0.0, 0.0]); continue; }
        g.cent.extend_from_slice(&[(v[1] / r).asin().to_degrees(), v[0].atan2(v[2]).to_degrees(), v[3]]);
    }
    NLOC as u32
}

fn km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let (a1, o1, a2, o2) = (lat1.to_radians(), lon1.to_radians(), lat2.to_radians(), lon2.to_radians());
    let d = (((a2 - a1) / 2.0).sin().powi(2) + a1.cos() * a2.cos() * ((o2 - o1) / 2.0).sin().powi(2)).sqrt().min(1.0);
    2.0 * 6371.0 * d.asin()
}

/// splitmix64, for the MODEL inputs below (device class per member)
fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// MODEL device classes for relay-tree inputs: (name code, uplink Mbit/s, max subscribers it offers).
/// 0 phone on cellular, 1 phone on Wi-Fi, 2 laptop, 3 home node. Shares 30/30/15/25 %.
/// Uplinks are round MODEL figures for a residential/mobile uplink, not measurements.
const ALM_CLASS: [(f32, u16); 4] = [(2.0, 1), (8.0, 2), (12.0, 4), (20.0, 8)];
fn alm_class(seed: u64, node: u32) -> u32 {
    let u = mix(seed ^ (node as u64).wrapping_mul(0x2545_F491_4F6C_DD1D)) % 100;
    if u < 30 { 0 } else if u < 60 { 1 } else if u < 75 { 2 } else { 3 }
}
/// MODEL round-trip time between two members: 10 ms of access plus fibre at 200 km/ms over a path
/// 1.6× the great circle, both ways.
pub fn model_rtt_ms(k: f64) -> u32 { (10.0 + 2.0 * 1.6 * k / 200.0).round() as u32 }

/// One room's member relay tree, computed by `cewp_sim::alm::compute_alm_topology_verified_with_publisher`.
/// Members are the first `max_members` of room `room` (plus `publisher` if it is a member, as root).
/// Inputs are MODEL (see ALM_CLASS, model_rtt_ms; every member grants every other at depth 1).
/// Buffers: 3 members (u32 node index), 4 edges (u32 stride 3: parent member ix, child member ix,
/// child depth), 5 unrooted members (u32 member ix), 6 member facts (f64 stride 4: class, uplink
/// Mbit/s, depth (-1 unrooted), children). Returns the edge count.
#[no_mangle]
pub extern "C" fn alm_room(
    lat: *const f32, lon: *const f32, room_off: *const u32, room_mem: *const u32,
    room: u32, publisher: u32, max_members: u32, seed_lo: u32,
) -> u32 {
    let g = globe();
    g.alm_members.clear(); g.alm_edges.clear(); g.alm_unrooted.clear(); g.alm_member_f.clear();
    let seed = seed_lo as u64 | 0x5EED_0000_0000;
    let members: Vec<u32> = unsafe {
        let a = *room_off.add(room as usize) as usize;
        let b = *room_off.add(room as usize + 1) as usize;
        let mut v: Vec<u32> = (a..b).map(|i| *room_mem.add(i)).collect();
        if let Some(p) = v.iter().position(|&m| m == publisher) { v.swap(0, p); }
        v.truncate(max_members.clamp(2, 96) as usize);
        v
    };
    if members.len() < 2 { return 0; }
    let pos: Vec<(f64, f64)> = members.iter().map(|&m| unsafe { (*lat.add(m as usize) as f64, *lon.add(m as usize) as f64) }).collect();
    let id = |i: usize| format!("m{:03}", i);
    let stream = StreamId([7u8; 32]);
    let mut ads = Vec::new();
    let mut grants = Vec::new();
    let mut obs = Vec::new();
    for (i, &m) in members.iter().enumerate() {
        let c = alm_class(seed, m) as usize;
        let (up, subs) = ALM_CLASS[c];
        ads.push(VerifiedCapacityAd {
            ad: SignedRelayCapacity {
                advertiser_key_id: id(i),
                capacity: RelayCapacity::new(up, 1, subs, ReceiverLayerPolicy::UNCAPPED, 1),
                stream_id: stream.clone(),
                epoch: Epoch(1),
                signature_ed25519_base64: String::new(),
                signature_ml_dsa_65_base64: String::new(),
            },
            verification: CapacityVerification::HybridSignatureValid,
        });
        for j in 0..members.len() {
            if i == j { continue; }
            grants.push(TrustGrant { granter_peer_id: id(i), grantee_peer_id: id(j), granted_at_unix_ms: 1, chain_depth: 1, weight: TrustGrant::canonical_weight_for_depth(1) });
            obs.push(ReachabilityObservation { from_peer_id: id(i), to_peer_id: id(j), observed_rtt_ms: model_rtt_ms(km(pos[i].0, pos[i].1, pos[j].0, pos[j].1)), observed_at_unix_ms: 1 });
        }
    }
    let snap = VerifiedTopologyInputSnapshot { capacity_ads: ads, trust_grants: grants, reachability_observations: obs, locality_id: String::new(), snapshot_epoch_id: 1 };
    let topo = compute_alm_topology_verified_with_publisher(&snap, &id(0));
    let ix = |s: &str| s[1..].parse::<usize>().unwrap_or(usize::MAX);
    let n = members.len();
    let mut parent = vec![usize::MAX; n];
    let mut children = vec![0u32; n];
    for e in &topo.tree {
        let (p, c) = (ix(&e.parent_peer_id), ix(&e.child_peer_id));
        if p < n && c < n && parent[c] == usize::MAX { parent[c] = p; children[p] += 1; }
    }
    let mut depth = vec![-1i32; n];
    depth[0] = 0;
    for _ in 0..n {
        for c in 0..n { if depth[c] < 0 && parent[c] < n && depth[parent[c]] >= 0 { depth[c] = depth[parent[c]] + 1; } }
    }
    for c in 0..n {
        if parent[c] < n { g.alm_edges.extend_from_slice(&[parent[c] as u32, c as u32, depth[c].max(0) as u32]); }
        else if c != 0 { g.alm_unrooted.push(c as u32); }
    }
    for (i, &m) in members.iter().enumerate() {
        let cl = alm_class(seed, m);
        g.alm_member_f.extend_from_slice(&[cl as f64, ALM_CLASS[cl as usize].0 as f64, depth[i] as f64, children[i] as f64]);
    }
    g.alm_members = members;
    (g.alm_edges.len() / 3) as u32
}

/// One room's whole-chunk custody (cewp_sim::chunk_custody): its members are the chunk's holders.
/// `present_pm` is the share of holders reachable (permille; 1000 = all), drawn per holder from
/// `seed_lo` (the failures package will replace this draw). Buffer 7 (f64): [members, present,
/// target, min viable, state (0 Full, 1 Partial, 2 EnvelopeOnly), Kish n_eff over holders' locality
/// masses, distinct localities, mean km from the holders' centroid, max km]. Buffer 8 (u32 stride 2):
/// holder node index, reachable (1/0). `target`/`min_viable` are the tick's own (20, 6).
#[no_mangle]
pub extern "C" fn custody_room(
    lat: *const f32, lon: *const f32, metro: *const u16, room_off: *const u32, room_mem: *const u32,
    room: u32, present_pm: u32, seed_lo: u32,
) -> u32 {
    let g = globe();
    g.custody.clear(); g.custody_holders.clear();
    let (target, min_viable) = (20u32, 6u32);
    let mut per_loc = vec![0u64; NLOC];
    let mut present = 0u32;
    let mut pts: Vec<(f64, f64)> = Vec::new();
    let members: Vec<u32> = unsafe {
        let a = *room_off.add(room as usize) as usize;
        let b = *room_off.add(room as usize + 1) as usize;
        (a..b).map(|i| *room_mem.add(i)).collect()
    };
    for &m in &members {
        let up = (mix((seed_lo as u64) << 32 ^ m as u64) % 1000) < present_pm as u64;
        g.custody_holders.extend_from_slice(&[m, up as u32]);
        if !up { continue; }
        present += 1;
        unsafe {
            let k = *metro.add(m as usize) as usize;
            if k < NLOC { per_loc[k] += 1; }
            pts.push((*lat.add(m as usize) as f64, *lon.add(m as usize) as f64));
        }
    }
    let masses: Vec<f64> = per_loc.iter().filter(|&&c| c > 0).map(|&c| c as f64).collect();
    let n_eff = if masses.is_empty() { 0 } else { effective_source_count(&masses) };
    let (mut cx, mut cy, mut cz) = (0.0, 0.0, 0.0);
    for &(a, o) in &pts { let (a, o) = (a.to_radians(), o.to_radians()); cx += a.cos() * o.sin(); cy += a.sin(); cz += a.cos() * o.cos(); }
    let r = (cx * cx + cy * cy + cz * cz).sqrt().max(1e-12);
    let (cla, clo) = ((cy / r).asin().to_degrees(), cx.atan2(cz).to_degrees());
    let mut sum = 0.0; let mut mx: f64 = 0.0;
    for &(a, o) in &pts { let d = km(cla, clo, a, o); sum += d; mx = mx.max(d); }
    let state = match classify(present, target, min_viable) { NoiseFloorState::Full => 0.0, NoiseFloorState::Partial => 1.0, NoiseFloorState::EnvelopeOnly => 2.0 };
    g.custody = vec![members.len() as f64, present as f64, target as f64, min_viable as f64, state, n_eff as f64, masses.len() as f64, if pts.is_empty() { 0.0 } else { sum / pts.len() as f64 }, mx];
    g.custody.len() as u32
}

/// A seat's load across `n` device counts spaced geometrically from `d0` to `d1`, under the same
/// arguments as `cap_seat` (shape, read, seats, select, cand). Buffer 9, f64 stride 4: devices,
/// rho cpu, rho permits, round latency s (-1 = unbounded). No knee is reported here.
#[no_mangle]
pub extern "C" fn cap_curve(d0: f64, d1: f64, n: u32, shape: u32, read: u32, seats: u32, select: u32, cand: u32) -> u32 {
    let g = globe();
    let p = g.cost.get_or_insert_with(CostParams::calibrated).clone();
    let shape = match shape { 1 => RoundShape::RequiredSet, 2 => RoundShape::Digest, 3 => RoundShape::Stretched, _ => RoundShape::WholePlane };
    let read = if read == 1 { ReadSet::AfterReadMin } else { ReadSet::AsShipped };
    g.cap_curve.clear();
    let n = n.clamp(2, 256);
    let (a, b) = (d0.max(1.0), d1.max(d0.max(1.0) * 1.0001));
    for i in 0..n {
        let d = a * (b / a).powf(i as f64 / (n - 1) as f64);
        let mut sc = Scenario::one_seat(d, shape, read);
        sc.seats = seats.max(1);
        sc.select = if select == 1 { SeatSelect::HomeOne } else { SeatSelect::AllSeats };
        if cand == 1 { sc = sc.with_candidate(); }
        let o = capacity::run(&p, &sc);
        let lat = o.seat_load.round_latency_s;
        g.cap_curve.extend_from_slice(&[d, o.seat_load.rho_cpu, o.seat_load.rho_permit, if lat.is_finite() { lat } else { -1.0 }]);
    }
    n
}

/// Globe buffers: 0 sun (f64), 1 diurnal bins (f64), 2 locality centroids (f64), 3 relay-tree
/// members (u32), 4 tree edges (u32), 5 unrooted (u32), 6 member facts (f64), 7 custody (f64),
/// 8 custody holders (u32), 9 seat curve (f64), 10 flows (u32, NLOC×NLOC), 11 locality acts (u32),
/// 12 tick facts (u32: [diurnal redraws, NLOC]).
#[no_mangle]
pub extern "C" fn globe_buf_ptr(which: u32) -> u32 {
    let g = globe();
    (match which {
        0 => g.sun.as_ptr() as usize,
        1 => g.diurnal_bins.as_ptr() as usize,
        2 => g.cent.as_ptr() as usize,
        3 => g.alm_members.as_ptr() as usize,
        4 => g.alm_edges.as_ptr() as usize,
        5 => g.alm_unrooted.as_ptr() as usize,
        6 => g.alm_member_f.as_ptr() as usize,
        7 => g.custody.as_ptr() as usize,
        8 => g.custody_holders.as_ptr() as usize,
        9 => g.cap_curve.as_ptr() as usize,
        10 => g.flows.as_ptr() as usize,
        11 => g.loc_acts.as_ptr() as usize,
        _ => {
            // SAFETY: one thread (see `globe`)
            unsafe { let f = &mut *std::ptr::addr_of_mut!(TICK_FACTS); f[0] = g.redraws; f[1] = NLOC as u32; f.as_ptr() as usize }
        }
    }) as u32
}
static mut TICK_FACTS: [u32; 2] = [0; 2];

/// Length in elements of a globe buffer (see `globe_buf_ptr`).
#[no_mangle]
pub extern "C" fn globe_buf_len(which: u32) -> u32 {
    let g = globe();
    (match which {
        0 => g.sun.len(), 1 => g.diurnal_bins.len(), 2 => g.cent.len(), 3 => g.alm_members.len(),
        4 => g.alm_edges.len(), 5 => g.alm_unrooted.len(), 6 => g.alm_member_f.len(), 7 => g.custody.len(),
        8 => g.custody_holders.len(), 9 => g.cap_curve.len(), 10 => g.flows.len(), 11 => g.loc_acts.len(),
        _ => 2,
    }) as u32
}

/// Fan-out bound and RTT ceiling the tree was computed under (alm.rs constants), for the legend.
#[no_mangle]
pub extern "C" fn alm_bounds(which: u32) -> u32 {
    match which { 0 => DEFAULT_SIM_FANOUT_CAP as u32, _ => MAX_USEFUL_RTT_MS }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    // the kernel state is one global (as on the page); tests that touch it take turns
    static TURN: Mutex<()> = Mutex::new(());

    #[test]
    fn subsolar_noon_utc_is_greenwich_and_solstice_declination() {
        let (d, lo) = subsolar(43200.0, 172.0);
        assert!((d - 23.44).abs() < 1e-9);
        assert!(lo.abs() < 1e-9);
        let (_, lo6) = subsolar(6.0 * 3600.0, 172.0);
        assert!((lo6 - 90.0).abs() < 1e-9);
    }

    #[test]
    fn legacy_spacing_without_clock_set() {
        let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
        let g = globe();
        g.t0 = 43200.0; g.span = 0.0;
        assert_eq!(act_time(g, 3, 100), 43200.0 + 3.0 * 137.0);
        g.span = 600.0;
        assert_eq!(act_time(g, 50, 100), 43200.0 + 300.0);
        g.span = 0.0;
    }

    #[test]
    fn diurnal_is_on_by_default_and_night_receivers_mark_less() {
        let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
        let g = globe();
        let was = g.diurnal;
        g.diurnal = true;
        // workload.rs peaks at 20:30 local and is lowest before dawn: at 00:00 UTC, longitude -52.5 is at
        // 20:30 local and longitude 75 at 05:00 local
        let evening = rx_mark(g, 0.0, -52.5, 70);
        let predawn = rx_mark(g, 0.0, 75.0, 70);
        assert!(predawn < evening / 4 && evening == 70, "evening {evening} pre-dawn {predawn}");
        g.diurnal = false;
        assert_eq!(rx_mark(g, 43_200.0, 180.0, 70), 70);
        g.diurnal = was;
        let fresh = GlobeState::fresh();
        assert!(fresh.diurnal, "the switch is on unless a caller turns it off");
    }

    #[test]
    fn diurnal_peak_bounds_the_multiplier() {
        for s in 0..(24 * 60) { assert!(diurnal_multiplier(s as f64 * 60.0) <= DIURNAL_PEAK + 1e-9); }
    }

    #[test]
    fn alm_room_tree_is_rooted_at_the_publisher_and_acyclic() {
        let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
        let lat: Vec<f32> = (0..40).map(|i| 40.0 + (i % 7) as f32).collect();
        let lon: Vec<f32> = (0..40).map(|i| -5.0 + (i % 11) as f32 * 2.0).collect();
        let off = [0u32, 40];
        let mem: Vec<u32> = (0..40).collect();
        let e = alm_room(lat.as_ptr(), lon.as_ptr(), off.as_ptr(), mem.as_ptr(), 0, 5, 40, 9) as usize;
        let g = globe();
        assert_eq!(g.alm_members[0], 5);
        assert!(e > 0);
        let n = g.alm_members.len();
        let mut parent = vec![usize::MAX; n];
        for k in 0..e { parent[g.alm_edges[k * 3 + 1] as usize] = g.alm_edges[k * 3] as usize; }
        assert_eq!(parent[0], usize::MAX);
        for c in 0..n { let mut x = c; let mut hops = 0; while parent[x] != usize::MAX { x = parent[x]; hops += 1; assert!(hops <= n); } }
        // every child's recorded depth is its parent's plus one
        for k in 0..e { let (p, d) = (g.alm_edges[k * 3] as usize, g.alm_edges[k * 3 + 2]); let dp = g.alm_member_f[p * 4 + 2]; assert_eq!(d as f64, dp + 1.0); }
        // no parent exceeds the fan-out bound
        for i in 0..n { assert!(g.alm_member_f[i * 4 + 3] <= DEFAULT_SIM_FANOUT_CAP as f64); }
    }

    #[test]
    fn custody_counts_holders_and_classifies() {
        let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
        let lat = [10.0f32, 10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 0.0];
        let lon = [0.0f32; 8];
        let metro = [0u16, 0, 1, 2, 3, 4, 5, 6];
        let off = [0u32, 8];
        let mem: Vec<u32> = (0..8).collect();
        custody_room(lat.as_ptr(), lon.as_ptr(), metro.as_ptr(), off.as_ptr(), mem.as_ptr(), 0, 1000, 1);
        let c = &globe().custody;
        assert_eq!(c[0], 8.0); assert_eq!(c[1], 8.0);
        assert_eq!(c[4], 1.0); // 6 <= 8 < 20: Partial
        assert_eq!(c[6], 7.0); // seven localities
        custody_room(lat.as_ptr(), lon.as_ptr(), metro.as_ptr(), off.as_ptr(), mem.as_ptr(), 0, 0, 1);
        assert_eq!(globe().custody[1], 0.0);
        assert_eq!(globe().custody[4], 2.0);
    }
}
