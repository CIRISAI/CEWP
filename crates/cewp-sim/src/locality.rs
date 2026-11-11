//! Locality as an output: run acts over the social graph and measure how many stay home
//! (twin package `people`, spec in `FSD/twin/people.md`).
//!
//! Definitions match the scale engine (`kernel-wasm/src/lib.rs` out slots 16 and 17):
//! an act is *local* when it has an audience and every delivery lands in the signer's
//! locality; the act-locality share is local acts over acts with an audience. The delivery
//! share (local deliveries over deliveries) is reported beside it, because the two differ
//! by a lot once rooms are large. A consent-free act goes, as in the scale engine, to one seat
//! in the signer's locality and nine seats drawn across the world by population.
//!
//! Load at a seat and at a relay: a *seat* homes `seat_homed` persons of one locality (the
//! page places one seat per ~2,000 nodes) and sees every act signed by, or addressed to a room
//! holding, one of them; a *relay* is an interior member of the largest public room's relay
//! tree and forwards every act in that room. Both are counted per UTC second.

use crate::activity::{Act, ActivityParams, Generator, Reach, ScopeChoice, SyncEvent};
use crate::micro::Rng;
use crate::social::{Geography, GraphStats, Placement, RoomKind, Scope, SocialParams, SocialWorld, NO_ROOM, ROOM_KINDS};

/// Remote seats a consent-free row replicates to (kernel-wasm `tick`, kind 2: nine canonical seats).
pub const CONSENT_FREE_REMOTE_SEATS: u32 = 9;

/// Per-second load summary.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadStats {
    /// Mean arrivals per second.
    pub mean: f64,
    /// Median.
    pub p50: f64,
    /// 95th percentile.
    pub p95: f64,
    /// 99th percentile.
    pub p99: f64,
    /// 99.9th percentile.
    pub p999: f64,
    /// Largest second.
    pub max: f64,
    /// Index of dispersion of 60-s window counts.
    pub dispersion_60s: f64,
}

impl LoadStats {
    /// Summarise a per-second count series.
    #[must_use]
    pub fn of(series: &[u32]) -> Self {
        if series.is_empty() {
            return Self { mean: 0.0, p50: 0.0, p95: 0.0, p99: 0.0, p999: 0.0, max: 0.0, dispersion_60s: 0.0 };
        }
        let mut s: Vec<u32> = series.to_vec();
        s.sort_unstable();
        let q = |x: f64| f64::from(s[(((s.len() - 1) as f64) * x).round() as usize]);
        let mean = series.iter().map(|&x| f64::from(x)).sum::<f64>() / series.len() as f64;
        let w: Vec<f64> = series.chunks(60).filter(|c| c.len() == 60).map(|c| c.iter().map(|&x| f64::from(x)).sum()).collect();
        let wm = w.iter().sum::<f64>() / w.len().max(1) as f64;
        let wv = w.iter().map(|x| (x - wm).powi(2)).sum::<f64>() / (w.len().max(2) - 1) as f64;
        Self { mean, p50: q(0.5), p95: q(0.95), p99: q(0.99), p999: q(0.999), max: q(1.0), dispersion_60s: if wm > 0.0 { wv / wm } else { 0.0 } }
    }

    /// Largest second over the mean second.
    #[must_use]
    pub fn peak_to_mean(&self) -> f64 {
        if self.mean > 0.0 { self.max / self.mean } else { 0.0 }
    }
}

/// What one run measured.
#[derive(Debug, Clone, PartialEq)]
pub struct Measurement {
    /// Acts signed.
    pub acts: u64,
    /// Acts by scope.
    pub acts_by_scope: [u64; 4],
    /// Acts with an audience, by scope.
    pub audience_by_scope: [u64; 4],
    /// Local acts (every delivery in the signer's locality), by scope.
    pub local_by_scope: [u64; 4],
    /// Acts whose every delivery stayed in the signer's region.
    pub region_local_acts: u64,
    /// Deliveries.
    pub deliveries: u64,
    /// Deliveries inside the signer's locality.
    pub local_deliveries: u64,
    /// Deliveries inside the signer's region.
    pub region_local_deliveries: u64,
    /// Acts that rode a synchronized event.
    pub event_acts: u64,
    /// Cascade replies (depth > 0).
    pub reply_acts: u64,
    /// Per signer locality: (acts with audience, local acts).
    pub per_locality: Vec<(u64, u64)>,
    /// Per UTC hour of the horizon: (acts with audience, local acts).
    pub per_utc_hour: Vec<(u64, u64)>,
    /// Per signer local hour: (acts with audience, local acts).
    pub per_local_hour: [(u64, u64); 24],
    /// Load at the seat.
    pub seat: LoadStats,
    /// Load at the relay.
    pub relay: LoadStats,
    /// Persons homed on the seat.
    pub seat_homed: usize,
    /// Members of the relay's room.
    pub relay_room_size: usize,
}

impl Measurement {
    /// Acts with an audience.
    #[must_use]
    pub fn audience_acts(&self) -> u64 {
        self.audience_by_scope.iter().sum()
    }
    /// Local acts over acts with an audience (the page's figure).
    #[must_use]
    pub fn act_locality(&self) -> f64 {
        ratio(self.local_by_scope.iter().sum(), self.audience_acts())
    }
    /// Local deliveries over deliveries.
    #[must_use]
    pub fn delivery_locality(&self) -> f64 {
        ratio(self.local_deliveries, self.deliveries)
    }
    /// Region-local acts over acts with an audience.
    #[must_use]
    pub fn act_region_locality(&self) -> f64 {
        ratio(self.region_local_acts, self.audience_acts())
    }
    /// Region-local deliveries over deliveries.
    #[must_use]
    pub fn delivery_region_locality(&self) -> f64 {
        ratio(self.region_local_deliveries, self.deliveries)
    }
    /// Act-locality share of one scope.
    #[must_use]
    pub fn scope_locality(&self, s: Scope) -> f64 {
        ratio(self.local_by_scope[s as usize], self.audience_by_scope[s as usize])
    }
    /// Share of acts with no audience.
    #[must_use]
    pub fn no_audience_share(&self) -> f64 {
        1.0 - ratio(self.audience_acts(), self.acts)
    }
    /// Act-locality across localities with at least `min` acts with an audience:
    /// (p10, p50, p90, min, max, localities counted).
    #[must_use]
    pub fn locality_spread(&self, min: u64) -> (f64, f64, f64, f64, f64, usize) {
        spread(&self.per_locality, min)
    }
    /// Act-locality across UTC hours of the horizon: (p10, p50, p90, min, max, hours).
    #[must_use]
    pub fn hour_spread(&self, min: u64) -> (f64, f64, f64, f64, f64, usize) {
        spread(&self.per_utc_hour, min)
    }
}

fn ratio(a: u64, b: u64) -> f64 {
    if b == 0 { 0.0 } else { a as f64 / b as f64 }
}

fn spread(v: &[(u64, u64)], min: u64) -> (f64, f64, f64, f64, f64, usize) {
    let mut s: Vec<f64> = v.iter().filter(|x| x.0 >= min).map(|x| ratio(x.1, x.0)).collect();
    if s.is_empty() {
        return (0.0, 0.0, 0.0, 0.0, 0.0, 0);
    }
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let q = |x: f64| s[(((s.len() - 1) as f64) * x).round() as usize];
    (q(0.1), q(0.5), q(0.9), s[0], s[s.len() - 1], s.len())
}

/// The seat and relay a run watches.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Watch {
    /// Seat's locality.
    pub seat_locality: u16,
    /// Persons homed on the seat (capped at the locality's population).
    pub seat_homed: usize,
    /// The relay's room.
    pub relay_room: u32,
}

impl Watch {
    /// The most populous locality's seat (2,000 homed, as the page) and the largest public room.
    #[must_use]
    pub fn default_for(w: &SocialWorld) -> Self {
        let seat_locality = (0..w.localities()).max_by_key(|&k| (w.locality_pop[k], std::cmp::Reverse(k))).unwrap_or(0) as u16;
        let relay_room = (0..w.rooms() as u32)
            .filter(|&r| w.room_kind[r as usize] == RoomKind::Public)
            .max_by_key(|&r| (w.members(r).len(), std::cmp::Reverse(r)))
            .unwrap_or(NO_ROOM);
        Self { seat_locality, seat_homed: 2000, relay_room }
    }
}

/// Run `p` over `w` for `horizon_s` and measure.
#[must_use]
pub fn measure(w: &SocialWorld, p: &ActivityParams, horizon_s: f64, seed: u64, forced: &[SyncEvent], watch: Watch) -> Measurement {
    let g = Generator::new(w, p, seed);
    let n_loc = w.localities();
    let secs = horizon_s.ceil() as usize;
    let hours = (horizon_s / 3600.0).ceil() as usize;
    let mut homed = vec![false; w.persons.len()];
    let mut seat_homed = 0;
    for (i, q) in w.persons.iter().enumerate() {
        if q.locality == watch.seat_locality && seat_homed < watch.seat_homed {
            homed[i] = true;
            seat_homed += 1;
        }
    }
    let touches: Vec<bool> = (0..w.rooms() as u32).map(|r| w.members(r).iter().any(|&m| homed[m as usize])).collect();
    let loc_cum: Vec<f64> = w.locality_pop.iter().scan(0.0, |s, &x| { *s += f64::from(x); Some(*s) }).collect();
    let mut seat_s = vec![0u32; secs];
    let mut relay_s = vec![0u32; secs];
    let mut m = Measurement {
        acts: 0,
        acts_by_scope: [0; 4],
        audience_by_scope: [0; 4],
        local_by_scope: [0; 4],
        region_local_acts: 0,
        deliveries: 0,
        local_deliveries: 0,
        region_local_deliveries: 0,
        event_acts: 0,
        reply_acts: 0,
        per_locality: vec![(0, 0); n_loc],
        per_utc_hour: vec![(0, 0); hours],
        per_local_hour: [(0, 0); 24],
        seat: LoadStats::of(&[]),
        relay: LoadStats::of(&[]),
        seat_homed,
        relay_room_size: if watch.relay_room == NO_ROOM { 0 } else { w.members(watch.relay_room).len() },
    };
    let mut r = Rng::new(seed ^ 0x6D65_6173);
    let total = loc_cum.last().copied().unwrap_or(0.0);
    let mut sink = |a: &Act| {
        let s = &w.persons[a.signer as usize];
        let (sl, sg) = (s.locality, s.region);
        m.acts += 1;
        m.acts_by_scope[a.scope as usize] += 1;
        if a.from_event {
            m.event_acts += 1;
        }
        if a.depth > 0 {
            m.reply_acts += 1;
        }
        let (aud, local, reg) = if a.room != NO_ROOM {
            let size = w.members(a.room).len() as u64 - 1; // the signer is a member
            let (mut local, mut reg) = (0u64, 0u64);
            for &(l, c) in w.room_localities(a.room) {
                if l == sl {
                    local += u64::from(c);
                }
                if w.region_of_locality[l as usize] == sg {
                    reg += u64::from(c);
                }
            }
            (size, local - 1, reg - 1)
        } else if a.scope == Scope::ConsentFree {
            let (mut local, mut reg) = (1u64, 1u64);
            for _ in 0..CONSENT_FREE_REMOTE_SEATS {
                let x = r.f() * total;
                let k = loc_cum.partition_point(|&c| c <= x).min(n_loc - 1);
                if k == sl as usize {
                    local += 1;
                }
                if w.region_of_locality[k] == sg {
                    reg += 1;
                }
            }
            (1 + u64::from(CONSENT_FREE_REMOTE_SEATS), local, reg)
        } else {
            (0, 0, 0)
        };
        let lh = ((a.t + f64::from(s.lon) / 360.0 * 86_400.0) / 3600.0).rem_euclid(24.0) as usize % 24;
        let uh = ((a.t / 3600.0) as usize).min(hours.saturating_sub(1));
        if aud > 0 {
            let is_local = u64::from(local == aud);
            m.audience_by_scope[a.scope as usize] += 1;
            m.local_by_scope[a.scope as usize] += is_local;
            m.region_local_acts += u64::from(reg == aud);
            m.deliveries += aud;
            m.local_deliveries += local;
            m.region_local_deliveries += reg;
            m.per_locality[sl as usize].0 += 1;
            m.per_locality[sl as usize].1 += is_local;
            m.per_utc_hour[uh].0 += 1;
            m.per_utc_hour[uh].1 += is_local;
            m.per_local_hour[lh].0 += 1;
            m.per_local_hour[lh].1 += is_local;
        }
        let sec = (a.t as usize).min(secs - 1);
        if homed[a.signer as usize] || (a.room != NO_ROOM && touches[a.room as usize]) {
            seat_s[sec] += 1;
        }
        if a.room == watch.relay_room && a.room != NO_ROOM {
            relay_s[sec] += 1;
        }
    };
    g.run(horizon_s, seed, forced, &mut sink);
    m.seat = LoadStats::of(&seat_s);
    m.relay = LoadStats::of(&relay_s);
    m
}

// ------------------------------------------------------------------ the report

/// Report options.
#[derive(Debug, Clone, PartialEq)]
pub struct ReportOptions {
    /// Persons in the world.
    pub persons: usize,
    /// Days in the main runs.
    pub days: f64,
    /// Days in each sweep run.
    pub sweep_days: f64,
    /// Seed.
    pub seed: u64,
}

impl Default for ReportOptions {
    fn default() -> Self {
        Self { persons: 20_000, days: 7.0, sweep_days: 1.0, seed: 20_261_110 }
    }
}

fn pct(x: f64) -> String {
    format!("{:.1}%", 100.0 * x)
}

fn locality_row(name: &str, m: &Measurement) -> String {
    let total = m.acts.max(1) as f64;
    format!(
        "| {name} | {} | {} | {} | {} | {} | {} / {} / {} / {} | {} / {} / {} / {} |\n",
        pct(m.act_locality()),
        pct(m.delivery_locality()),
        pct(m.act_region_locality()),
        pct(m.delivery_region_locality()),
        pct(m.no_audience_share()),
        pct(m.scope_locality(Scope::Family)),
        pct(m.scope_locality(Scope::Community)),
        pct(m.scope_locality(Scope::ConsentFree)),
        pct(m.scope_locality(Scope::Public)),
        pct(m.acts_by_scope[0] as f64 / total),
        pct(m.acts_by_scope[1] as f64 / total),
        pct(m.acts_by_scope[2] as f64 / total),
        pct(m.acts_by_scope[3] as f64 / total),
    )
}

const LOCALITY_HEAD: &str = "| run | act locality | delivery locality | act region | delivery region | no audience | act locality by scope (fam / comm / c-free / pub) | realised mix (fam / comm / c-free / pub) |\n|---|---|---|---|---|---|---|---|\n";

fn load_row(name: &str, l: &LoadStats) -> String {
    format!(
        "| {name} | {:.2} | {:.0} | {:.0} | {:.0} | {:.0} | {:.0} | {:.1} | {:.1} |\n",
        l.mean, l.p50, l.p95, l.p99, l.p999, l.max, l.peak_to_mean(), l.dispersion_60s
    )
}

const LOAD_HEAD: &str = "| process | mean/s | p50 | p95 | p99 | p99.9 | max | peak/mean | D(60 s) |\n|---|---|---|---|---|---|---|---|---|\n";

/// Onset at 20:30 local in `locality` on day `day`.
fn evening_onset(w: &SocialWorld, locality: u16, day: f64) -> f64 {
    let (mut s, mut c) = (0.0, 0.0);
    for q in &w.persons {
        if q.locality == locality {
            s += f64::from(q.lon);
            c += 1.0;
        }
    }
    let lon = if c > 0.0 { s / c } else { 0.0 };
    day * 86_400.0 + (20.5 * 3600.0 - lon / 360.0 * 86_400.0).rem_euclid(86_400.0)
}

/// The full report as Markdown: graph statistics against bands, emergent locality next to the
/// page's input figure, sweeps, and load at a seat and a relay.
#[must_use]
pub fn report(geo: &Geography, geo_name: &str, o: &ReportOptions) -> String {
    let sp = SocialParams::default();
    let w = SocialWorld::build(geo, &sp, o.persons, o.seed, Placement::Placed);
    let g = w.ties(&sp, o.seed);
    let gs = GraphStats::of(&w, &g, 5000, o.seed);
    let horizon = o.days * 86_400.0;
    let watch = Watch::default_for(&w);
    let mut out = String::new();
    out.push_str(&format!(
        "# people: emergent locality and workload bursts\n\nGeography: {geo_name} ({} localities, {} regions). Persons {}, households {}, rooms {}. Seed {}. Main runs {} days, sweeps {} day(s). Every number below is a MODEL output of MODEL/FRONTIER inputs (FSD/twin/people.md); none is MEASURED.\n\n",
        w.localities(),
        geo.regions(),
        w.persons.len(),
        w.households,
        w.rooms(),
        o.seed,
        o.days,
        o.sweep_days
    ));

    out.push_str("## Graph statistics against the bands stated before the run\n\n| statistic | value | band | in band | band source |\n|---|---|---|---|---|\n");
    for (b, v, ok) in gs.against_bands() {
        out.push_str(&format!("| {} | {:.3} | {} .. {} | {} | {} |\n", b.name, v, b.lo, b.hi, if ok { "yes" } else { "NO" }, b.source));
    }
    out.push_str(&format!(
        "\nDegree: mean {:.1}, median {:.0}, p90 {:.0}, p99 {:.0}, max {:.0}, CV {:.2}, isolated {}. Ties {}. Modularity by region {:.3}.\n\n",
        gs.mean_degree, gs.median_degree, gs.p90_degree, gs.p99_degree, gs.max_degree, gs.degree_cv, pct(gs.isolated_share), gs.ties, gs.modularity_region
    ));

    out.push_str("## Rooms by mechanism\n\n| kind | rooms | mean members | largest | members in the room's majority locality |\n|---|---|---|---|---|\n");
    for k in ROOM_KINDS {
        let rs: Vec<u32> = (0..w.rooms() as u32).filter(|&r| w.room_kind[r as usize] == k).collect();
        let mem: usize = rs.iter().map(|&r| w.members(r).len()).sum();
        let maj: u64 = rs.iter().map(|&r| w.room_localities(r).iter().map(|x| u64::from(x.1)).max().unwrap_or(0)).sum();
        let largest = rs.iter().map(|&r| w.members(r).len()).max().unwrap_or(0);
        out.push_str(&format!("| {} | {} | {:.1} | {} | {} |\n", k.as_str(), rs.len(), mem as f64 / rs.len().max(1) as f64, largest, pct(maj as f64 / mem.max(1) as f64)));
    }

    // main runs
    let mpois = measure(&w, &ActivityParams::poisson(), horizon, o.seed, &[], watch);
    let mburst = measure(&w, &ActivityParams::default(), horizon, o.seed, &[], watch);
    let mut tie = ActivityParams::default();
    tie.scope_choice = ScopeChoice::TieWeighted;
    let mtie = measure(&w, &tie, horizon, o.seed, &[], watch);
    let mut np = sp.clone();
    np.persons_per_household = 1.0;
    np.household_size_sd = 0.0;
    let wnull = SocialWorld::build(geo, &np, o.persons, o.seed, Placement::Placeless);
    let mnull = measure(&wnull, &ActivityParams::poisson(), o.sweep_days * 86_400.0, o.seed, &[], Watch::default_for(&wnull));

    out.push_str("\n## Locality: the page's input figure next to what the graph produces\n\n");
    out.push_str("The page: families drawn 100% in the signer's locality, community rooms drawn 90% local, act mix 65/30/4/1, so its act locality is 0.65 + 0.30 × 0.90 = 0.92 (DERIVED from its inputs). README states ~92%; FSD §4.4 Expected regime states 65% (~90% incl. local community).\n\n");
    out.push_str(LOCALITY_HEAD);
    out.push_str("| page today (input arithmetic) | 92.0% | n/a | n/a | n/a | n/a | 100% / 90% / 0% / 0% | 65% / 30% / 4% / 1% |\n");
    out.push_str(&locality_row("graph, Poisson, fixed mix", &mpois));
    out.push_str(&locality_row("graph, bursty, fixed mix", &mburst));
    out.push_str(&locality_row("graph, bursty, tie-weighted", &mtie));
    out.push_str(&locality_row("null: placeless rooms, Poisson", &mnull));
    let (a, b, c, d, e, n) = mburst.locality_spread(200);
    out.push_str(&format!("\nAcross signer localities (bursty, fixed mix; {n} localities with >= 200 acts with an audience): p10 {}, p50 {}, p90 {}, min {}, max {}.\n", pct(a), pct(b), pct(c), pct(d), pct(e)));
    let (a, b, c, d, e, n) = mburst.hour_spread(1000);
    out.push_str(&format!("Across UTC hours ({n} hours): p10 {}, p50 {}, p90 {}, min {}, max {}.\n", pct(a), pct(b), pct(c), pct(d), pct(e)));
    out.push_str("By signer local hour (act locality): ");
    for (h, x) in mburst.per_local_hour.iter().enumerate() {
        out.push_str(&format!("{h:02}h {} ", pct(ratio(x.1, x.0))));
    }
    out.push_str(&format!(
        "\nBursty run: {} acts, {} replies ({}), {} event acts ({}).\n",
        mburst.acts,
        mburst.reply_acts,
        pct(ratio(mburst.reply_acts, mburst.acts)),
        mburst.event_acts,
        pct(ratio(mburst.event_acts, mburst.acts))
    ));

    // sweeps
    let sweep = |label: &str, vals: &[f64], set: &dyn Fn(&mut SocialParams, f64)| -> String {
        let mut s = format!("\n### Sweep: {label} (Poisson, fixed mix, {} day(s))\n\n| {label} | act locality | delivery locality | act region | family act locality | community act locality | cross-locality ties |\n|---|---|---|---|---|---|---|\n", o.sweep_days);
        for &v in vals {
            let mut p = SocialParams::default();
            set(&mut p, v);
            let wv = SocialWorld::build(geo, &p, o.persons, o.seed, Placement::Placed);
            let gv = GraphStats::of(&wv, &wv.ties(&p, o.seed), 500, o.seed);
            let mv = measure(&wv, &ActivityParams::poisson(), o.sweep_days * 86_400.0, o.seed, &[], Watch::default_for(&wv));
            s.push_str(&format!(
                "| {v} | {} | {} | {} | {} | {} | {} |\n",
                pct(mv.act_locality()),
                pct(mv.delivery_locality()),
                pct(mv.act_region_locality()),
                pct(mv.scope_locality(Scope::Family)),
                pct(mv.scope_locality(Scope::Community)),
                pct(gv.cross_locality_tie_share)
            ));
        }
        s
    };
    out.push_str("\n## How locality moves with the inputs that set it\n");
    out.push_str(&sweep("diaspora_share", &[0.0, 0.06, 0.12, 0.25, 0.5], &|p, v| p.diaspora_share = v));
    out.push_str(&sweep("interest_local_share", &[1.0, 0.9, 0.7, 0.5, 0.3, 0.1], &|p, v| {
        p.interest_local_share = v;
        p.interest_regional_share = 0.6 * (1.0 - v);
    }));
    out.push_str(&sweep("public_local_share", &[0.9, 0.4, 0.0], &|p, v| {
        p.public_local_share = v;
        p.public_regional_share = 0.5 * (1.0 - v);
    }));
    out.push_str(&sweep("kin_households_mean", &[1.0, 2.0, 4.0, 8.0], &|p, v| p.kin_households_mean = v));

    // load
    let seat_t0 = evening_onset(&w, watch.seat_locality, 2.0);
    let relay_t0 = 4.0 * 86_400.0 + 20.5 * 3600.0;
    let flash = [
        SyncEvent { t0: seat_t0, reach: Reach::Locality(watch.seat_locality), participation: 0.5, acts_per_participant: 3.0, decay_s: 300.0 },
        SyncEvent { t0: relay_t0, reach: Reach::World, participation: 0.3, acts_per_participant: 3.0, decay_s: 300.0 },
    ];
    let flash: Vec<SyncEvent> = flash.into_iter().filter(|e| e.t0 < horizon).collect();
    let mflash = measure(&w, &ActivityParams::default(), horizon, o.seed, &flash, watch);
    out.push_str(&format!(
        "\n## Load per second at a seat and at a relay\n\nSeat: {} persons homed in the most populous locality (index {}); counts every act signed by one of them or posted in a room holding one. Relay: interior member of the largest public room ({} members); counts every act posted in it. Flash crowd: the bursty process plus a match in the seat's locality at 20:30 local on day 3 (half the locality joins, 3 acts each, 300 s decay) and a viral post worldwide at 20:30 UTC on day 5 (30% join): FRONTIER stress inputs, not calibrated.\n\n### Seat\n\n",
        mpois.seat_homed, watch.seat_locality, mpois.relay_room_size
    ));
    out.push_str(LOAD_HEAD);
    out.push_str(&load_row("Poisson (today)", &mpois.seat));
    out.push_str(&load_row("bursty", &mburst.seat));
    out.push_str(&load_row("bursty + flash crowd", &mflash.seat));
    out.push_str("\n### Relay\n\n");
    out.push_str(LOAD_HEAD);
    out.push_str(&load_row("Poisson (today)", &mpois.relay));
    out.push_str(&load_row("bursty", &mburst.relay));
    out.push_str(&load_row("bursty + flash crowd", &mflash.relay));
    out.push_str(&format!(
        "\nActs: Poisson {}, bursty {} ({:+.2}% vs Poisson), bursty + flash {}.\n",
        mpois.acts,
        mburst.acts,
        100.0 * (mburst.acts as f64 / mpois.acts.max(1) as f64 - 1.0),
        mflash.acts
    ));
    out
}

/// `cewp-cli people [--persons N] [--days D] [--sweep-days D] [--seed S] [--geo PATH|stand-in]`.
///
/// # Errors
/// A bad argument or an unreadable geography.
pub fn cli(args: &[String]) -> Result<String, String> {
    let mut o = ReportOptions::default();
    let mut geo_path = "docs/globe.js".to_string();
    let mut i = 0;
    while i < args.len() {
        let val = args.get(i + 1).ok_or_else(|| format!("{} needs a value", args[i]))?;
        let num = || val.parse::<f64>().map_err(|e| format!("{}: {e}", args[i]));
        match args[i].as_str() {
            "--persons" => o.persons = num()? as usize,
            "--days" => o.days = num()?,
            "--sweep-days" => o.sweep_days = num()?,
            "--seed" => o.seed = num()? as u64,
            "--geo" => geo_path = val.clone(),
            other => return Err(format!("people: unknown option {other}")),
        }
        i += 2;
    }
    if geo_path == "stand-in" {
        let geo = Geography::stand_in(o.seed, 181, 16, 9);
        return Ok(report(&geo, "generated stand-in (MODEL)", &o));
    }
    let src = std::fs::read_to_string(&geo_path).map_err(|e| format!("{geo_path}: {e} (run from the repo root, or pass --geo stand-in)"))?;
    let geo = Geography::from_globe_js(&src)?;
    Ok(report(&geo, &format!("the page's population grid ({geo_path} POP_GRID_B64)"), &o))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(w: &SocialWorld, p: &ActivityParams) -> Measurement {
        measure(w, p, 86_400.0, 3, &[], Watch::default_for(w))
    }

    #[test]
    fn measurement_is_deterministic_by_seed() {
        let w = SocialWorld::build(&Geography::stand_in(1, 12, 3, 3), &SocialParams::default(), 1500, 5, Placement::Placed);
        let a = run(&w, &ActivityParams::default());
        let b = run(&w, &ActivityParams::default());
        assert_eq!(a, b);
    }

    /// No tie leaves a locality (no diaspora, no remote work, every interest group and public
    /// room locality-bound) and no consent-free acts: every act and delivery is local.
    #[test]
    fn zero_cross_locality_ties_measure_full_locality() {
        let mut p = SocialParams::default();
        p.diaspora_share = 0.0;
        p.work_cross_locality_share = 0.0;
        p.interest_local_share = 1.0;
        p.interest_regional_share = 0.0;
        p.public_local_share = 1.0;
        p.public_regional_share = 0.0;
        let w = SocialWorld::build(&Geography::stand_in(2, 15, 3, 3), &p, 2000, 6, Placement::Placed);
        let mut a = ActivityParams::default();
        a.mix = [0.65, 0.30, 0.0, 0.05];
        a.event_mix = [0.5, 0.3, 0.0, 0.2];
        let m = run(&w, &a);
        assert!(m.audience_acts() > 10_000);
        assert_eq!(m.act_locality(), 1.0);
        assert_eq!(m.delivery_locality(), 1.0);
    }

    /// Rooms that ignore geography over 10 equal localities: a delivery is local with
    /// probability ~1/10. 3,000 persons put each locality's share within ~0.006 of 0.1, and
    /// deliveries number in the millions, so 0.02 is several sd wide.
    #[test]
    fn placeless_rooms_measure_one_over_localities() {
        let mut p = SocialParams::default();
        p.persons_per_household = 1.0;
        p.household_size_sd = 0.0;
        let l = 10;
        let w = SocialWorld::build(&Geography::uniform(l, 2), &p, 3000, 7, Placement::Placeless);
        let mut a = ActivityParams::poisson();
        a.mix = [0.65, 0.30, 0.0, 0.05];
        let m = run(&w, &a);
        assert!((m.delivery_locality() - 0.1).abs() < 0.02, "{}", m.delivery_locality());
        // an act is local only if every member is: far below 1/L for rooms of several members
        assert!(m.act_locality() < 0.1, "{}", m.act_locality());
    }

    #[test]
    fn load_stats_of_constant_series() {
        let l = LoadStats::of(&vec![3u32; 600]);
        assert_eq!((l.mean, l.p50, l.p999, l.max), (3.0, 3.0, 3.0, 3.0));
        assert_eq!(l.dispersion_60s, 0.0);
        assert_eq!(l.peak_to_mean(), 1.0);
    }

    #[test]
    fn report_runs_small() {
        let o = ReportOptions { persons: 600, days: 1.0, sweep_days: 0.1, seed: 1 };
        let s = report(&Geography::stand_in(1, 12, 3, 3), "stand-in", &o);
        assert!(s.contains("act locality"));
        assert!(s.contains("| Poisson (today) |"));
    }
}
