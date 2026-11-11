//! Who acts when, and in which room (twin package `people`, spec in `FSD/twin/people.md`).
//!
//! Two processes over one [`SocialWorld`]:
//!
//! * [`Process::Poisson`]: today's generator, unchanged in law. Each person signs at
//!   `λ(t) = μ_diurnal(t_local) · acts_per_person_day / 86 400` (FSD §4.1, the same
//!   `workload::compute_arrival_rate` with one act per event), independently.
//! * [`Process::Bursty`]: the same mean rate, redistributed. Per-person activity is lognormal
//!   (normalised to mean 1); the day has a weekly rhythm and a later weekend clock; every act is
//!   an immigrant of a self-exciting (Hawkes) cascade whose replies come from the room's members
//!   after heavy-tailed delays; and a share of the budget arrives as exogenous synchronized
//!   events (a match in one locality, news in a region, a viral post worldwide) that hit a
//!   correlated set of people at once.
//!
//! Mean preservation is by construction: background immigrants run at
//! `(1 - branching) (1 - event_share)` of the rate, events supply `(1 - branching) event_share`,
//! and each immigrant brings `1 / (1 - branching)` acts on average. `tests::mean_rate_matches_poisson`
//! checks it over a week.
//!
//! An act's room comes from the signer's rooms in the graph. With [`ScopeChoice::FixedMix`] the
//! scope is drawn from the page's mix (65 / 30 / 4 / 1) and only the room inside the scope comes
//! from the graph; with [`ScopeChoice::TieWeighted`] the scope itself follows from per-kind
//! weights over the rooms a person actually has.

use crate::micro::{ParamSpec, Rng, Tag};
use crate::social::{RoomKind, Scope, SocialWorld, NO_ROOM};
use crate::workload::compute_arrival_rate;

/// Upper bound of `diurnal_multiplier` (0.75 + 1.05 + 0.5 at cos = 1), with margin.
const DIURNAL_MAX: f64 = 2.31;

/// Arrival law.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Process {
    /// Today's inhomogeneous Poisson (diurnal only).
    Poisson,
    /// Heterogeneous, weekly, self-exciting, with synchronized events.
    Bursty,
}

/// How an act's scope is chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeChoice {
    /// The page's fixed scope mix; the graph picks the room inside the scope.
    FixedMix,
    /// Per-kind weights over the signer's own rooms; the scope mix is an output.
    TieWeighted,
}

/// Activity parameters. Defaults and classes are in [`ACTIVITY_PARAM_SPECS`].
#[derive(Debug, Clone, PartialEq)]
pub struct ActivityParams {
    /// Arrival law.
    pub process: Process,
    /// Scope choice.
    pub scope_choice: ScopeChoice,
    /// Flat clock (no diurnal or weekly rhythm): tests of dispersion only.
    pub flat_rhythm: bool,
    /// Mean acts a person signs per day.
    pub acts_per_person_day: f64,
    /// Scope mix (family, community, consent-free, public).
    pub mix: [f64; 4],
    /// In a family act, the share that goes to the household room (else a kin room).
    pub family_household_share: f64,
    /// Community room weights by kind (work, school, neighbourhood, interest).
    pub community_kind_weight: [f64; 4],
    /// Local work / school hours (start, end) on weekdays.
    pub work_hours: (f64, f64),
    /// Weight of work and school rooms outside those hours, relative to inside.
    pub off_hours_work_affinity: f64,
    /// TieWeighted: weight per room kind in `ROOM_KINDS` order, then consent-free.
    pub tie_weights: [f64; 8],
    /// Day-of-week multipliers, Monday first (mean 1).
    pub weekly: [f64; 7],
    /// Weekend clock shift, hours later.
    pub weekend_shift_h: f64,
    /// Weekday at t = 0 (0 = Monday).
    pub start_weekday: u32,
    /// Lognormal sigma of per-person activity (mean 1).
    pub activity_sigma: f64,
    /// Mean replies an act begets (Hawkes branching ratio, < 1).
    pub branching: f64,
    /// Reply delay Lomax scale, seconds.
    pub reply_scale_s: f64,
    /// Reply delay Lomax tail index.
    pub reply_alpha: f64,
    /// Longest reply delay, seconds (longer draws are redrawn).
    pub reply_max_s: f64,
    /// Share of the act budget that arrives as synchronized events.
    pub event_share: f64,
    /// Event reach shares (locality, region, world).
    pub event_level_shares: [f64; 3],
    /// Share of the reached population that joins, per reach.
    pub event_participation: [f64; 3],
    /// Mean acts per participant (Poisson).
    pub event_acts_per_participant: f64,
    /// Mean delay of a participant's acts after onset (exponential), seconds.
    pub event_decay_s: f64,
    /// Scope mix of event acts (FixedMix).
    pub event_mix: [f64; 4],
}

impl Default for ActivityParams {
    fn default() -> Self {
        Self {
            process: Process::Bursty,
            scope_choice: ScopeChoice::FixedMix,
            flat_rhythm: false,
            acts_per_person_day: 180.0,
            mix: [0.65, 0.30, 0.04, 0.01],
            family_household_share: 0.5,
            community_kind_weight: [1.0, 1.0, 0.5, 1.0],
            work_hours: (8.0, 18.0),
            off_hours_work_affinity: 0.15,
            tie_weights: [0.25, 0.25, 0.12, 0.04, 0.06, 0.21, 0.03, 0.04],
            weekly: [1.02, 1.02, 1.02, 1.02, 1.02, 0.97, 0.93],
            weekend_shift_h: 1.5,
            start_weekday: 0,
            activity_sigma: 1.0,
            branching: 0.5,
            reply_scale_s: 30.0,
            reply_alpha: 1.5,
            reply_max_s: 21_600.0,
            event_share: 0.05,
            event_level_shares: [0.6, 0.3, 0.1],
            event_participation: [0.3, 0.2, 0.1],
            event_acts_per_participant: 3.0,
            event_decay_s: 300.0,
            event_mix: [0.45, 0.30, 0.0, 0.25],
        }
    }
}

impl ActivityParams {
    /// Today's generator: Poisson, fixed mix, no weekly rhythm, no cascades, no events.
    #[must_use]
    pub fn poisson() -> Self {
        Self { process: Process::Poisson, ..Self::default() }
    }
}

/// Numeric activity parameters with class and source (arrays listed by element).
pub const ACTIVITY_PARAM_SPECS: &[ParamSpec] = &[
    ParamSpec { name: "acts_per_person_day", default: 180.0, tag: Tag::Model, source: "86,400 / micro.rs msg_mean_interval_s (480 s, one awake device per person); scales seat counts, not locality" },
    ParamSpec { name: "mix_family", default: 0.65, tag: Tag::Model, source: "the page's act mix (globe.js tick args 650/300/40, public the remaining 10 per mille; kernel README)" },
    ParamSpec { name: "mix_community", default: 0.30, tag: Tag::Model, source: "the page's act mix" },
    ParamSpec { name: "mix_consent_free", default: 0.04, tag: Tag::Model, source: "the page's act mix" },
    ParamSpec { name: "mix_public", default: 0.01, tag: Tag::Model, source: "the page's act mix" },
    ParamSpec { name: "family_household_share", default: 0.5, tag: Tag::Frontier, source: "household room vs kin room inside a family act; no data" },
    ParamSpec { name: "community_weight_work", default: 1.0, tag: Tag::Frontier, source: "relative weight of a work room in a community act" },
    ParamSpec { name: "community_weight_school", default: 1.0, tag: Tag::Frontier, source: "relative weight of a school room" },
    ParamSpec { name: "community_weight_neighbourhood", default: 0.5, tag: Tag::Frontier, source: "relative weight of a neighbourhood room" },
    ParamSpec { name: "community_weight_interest", default: 1.0, tag: Tag::Frontier, source: "relative weight of an interest room" },
    ParamSpec { name: "work_hours_start", default: 8.0, tag: Tag::Model, source: "local weekday work/school hours; kind of source: time-use surveys (not loaded)" },
    ParamSpec { name: "work_hours_end", default: 18.0, tag: Tag::Model, source: "as above" },
    ParamSpec { name: "off_hours_work_affinity", default: 0.15, tag: Tag::Model, source: "work and school rooms stay partly active off hours" },
    ParamSpec { name: "tie_weight_household", default: 0.25, tag: Tag::Frontier, source: "TieWeighted share of a person's acts to the household; kind of source: messaging-recipient surveys; no data" },
    ParamSpec { name: "tie_weight_kin", default: 0.25, tag: Tag::Frontier, source: "TieWeighted, kin rooms" },
    ParamSpec { name: "tie_weight_work", default: 0.12, tag: Tag::Frontier, source: "TieWeighted, work rooms" },
    ParamSpec { name: "tie_weight_school", default: 0.04, tag: Tag::Frontier, source: "TieWeighted, school rooms" },
    ParamSpec { name: "tie_weight_neighbourhood", default: 0.06, tag: Tag::Frontier, source: "TieWeighted, neighbourhood rooms" },
    ParamSpec { name: "tie_weight_interest", default: 0.21, tag: Tag::Frontier, source: "TieWeighted, interest rooms" },
    ParamSpec { name: "tie_weight_public", default: 0.03, tag: Tag::Frontier, source: "TieWeighted, public rooms" },
    ParamSpec { name: "tie_weight_consent_free", default: 0.04, tag: Tag::Model, source: "TieWeighted, consent-free rows (kept at the page's 4%)" },
    ParamSpec { name: "weekly_weekday", default: 1.02, tag: Tag::Model, source: "Mon-Fri multiplier; weekly mean is 1; kind of source: published messaging / traffic weekly curves (not loaded)" },
    ParamSpec { name: "weekly_saturday", default: 0.97, tag: Tag::Model, source: "Saturday multiplier" },
    ParamSpec { name: "weekly_sunday", default: 0.93, tag: Tag::Model, source: "Sunday multiplier" },
    ParamSpec { name: "weekend_shift_h", default: 1.5, tag: Tag::Model, source: "weekend clock runs later (diurnal curve shifted, mean unchanged)" },
    ParamSpec { name: "activity_sigma", default: 1.0, tag: Tag::Model, source: "per-person activity lognormal (heavy-tailed users); human-dynamics literature reports strongly heterogeneous rates" },
    ParamSpec { name: "branching", default: 0.5, tag: Tag::Frontier, source: "mean replies per act (Hawkes branching ratio); no data" },
    ParamSpec { name: "reply_scale_s", default: 30.0, tag: Tag::Model, source: "reply delay Lomax scale" },
    ParamSpec { name: "reply_alpha", default: 1.5, tag: Tag::Model, source: "reply delay tail index; human-dynamics literature reports power-law inter-event times with exponents near 1-2 (e.g. Barabasi 2005, quoted from memory)" },
    ParamSpec { name: "reply_max_s", default: 21_600.0, tag: Tag::Model, source: "replies after 6 h are not part of the cascade" },
    ParamSpec { name: "event_share", default: 0.05, tag: Tag::Frontier, source: "share of acts that ride synchronized events; no data" },
    ParamSpec { name: "event_share_locality", default: 0.6, tag: Tag::Frontier, source: "share of events whose reach is one locality (a match)" },
    ParamSpec { name: "event_share_region", default: 0.3, tag: Tag::Frontier, source: "share reaching one region (news)" },
    ParamSpec { name: "event_share_world", default: 0.1, tag: Tag::Frontier, source: "share reaching the world (a viral post)" },
    ParamSpec { name: "event_participation_locality", default: 0.3, tag: Tag::Frontier, source: "share of a locality joining its event" },
    ParamSpec { name: "event_participation_region", default: 0.2, tag: Tag::Frontier, source: "share of a region joining" },
    ParamSpec { name: "event_participation_world", default: 0.1, tag: Tag::Frontier, source: "share of the world joining" },
    ParamSpec { name: "event_acts_per_participant", default: 3.0, tag: Tag::Frontier, source: "acts a participant signs about the event" },
    ParamSpec { name: "event_decay_s", default: 300.0, tag: Tag::Model, source: "mean delay of event acts after onset (instant rise, exponential decay)" },
    ParamSpec { name: "event_mix_family", default: 0.45, tag: Tag::Frontier, source: "event acts' scope mix: family" },
    ParamSpec { name: "event_mix_community", default: 0.30, tag: Tag::Frontier, source: "event acts: community" },
    ParamSpec { name: "event_mix_consent_free", default: 0.0, tag: Tag::Frontier, source: "event acts: consent-free" },
    ParamSpec { name: "event_mix_public", default: 0.25, tag: Tag::Frontier, source: "event acts: public" },
];

/// One signed act.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Act {
    /// UTC seconds from the start.
    pub t: f64,
    /// Signer.
    pub signer: u32,
    /// Scope.
    pub scope: Scope,
    /// Room, or [`NO_ROOM`] (consent-free, or a public act by someone who follows none).
    pub room: u32,
    /// Generation in its cascade (0 = immigrant).
    pub depth: u16,
    /// Rides a synchronized event.
    pub from_event: bool,
}

/// Where a synchronized event lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reach {
    /// One locality.
    Locality(u16),
    /// One region.
    Region(u8),
    /// Everyone.
    World,
}

/// A synchronized event.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SyncEvent {
    /// Onset, UTC seconds.
    pub t0: f64,
    /// Who it reaches.
    pub reach: Reach,
    /// Share of the reached who join.
    pub participation: f64,
    /// Mean acts per participant.
    pub acts_per_participant: f64,
    /// Mean delay after onset, seconds.
    pub decay_s: f64,
}

/// Generator state over one world.
#[derive(Debug, Clone)]
pub struct Generator<'a> {
    w: &'a SocialWorld,
    p: ActivityParams,
    /// Per-person activity weight (mean 1).
    pub weight: Vec<f64>,
    by_loc_off: Vec<u32>,
    by_loc: Vec<u32>,
}

fn poisson(r: &mut Rng, mean: f64) -> u32 {
    if mean <= 0.0 {
        return 0;
    }
    if mean > 30.0 {
        return (mean + mean.sqrt() * r.gauss()).round().max(0.0) as u32;
    }
    let l = (-mean).exp();
    let (mut k, mut q) = (0u32, 1.0f64);
    loop {
        q *= r.f();
        if q <= l {
            return k;
        }
        k += 1;
    }
}

fn pick(weights: &[f64], u: f64) -> Option<usize> {
    let total: f64 = weights.iter().sum();
    if total <= 0.0 {
        return None;
    }
    let mut x = u * total;
    for (i, &w) in weights.iter().enumerate() {
        if x < w {
            return Some(i);
        }
        x -= w;
    }
    weights.iter().rposition(|&w| w > 0.0)
}

impl<'a> Generator<'a> {
    /// Build over `w`; per-person weights come from `seed`.
    #[must_use]
    pub fn new(w: &'a SocialWorld, p: &ActivityParams, seed: u64) -> Self {
        let n = w.persons.len();
        let mut r = Rng::new(seed ^ 0x7765_6967);
        let mut weight: Vec<f64> = (0..n).map(|_| (p.activity_sigma * r.gauss()).exp()).collect();
        let mean = weight.iter().sum::<f64>() / n.max(1) as f64;
        for x in &mut weight {
            *x /= mean;
        }
        let n_loc = w.localities();
        let mut by_loc_off = vec![0u32; n_loc + 1];
        for q in &w.persons {
            by_loc_off[q.locality as usize + 1] += 1;
        }
        for k in 0..n_loc {
            by_loc_off[k + 1] += by_loc_off[k];
        }
        let mut pos = by_loc_off.clone();
        let mut by_loc = vec![0u32; n];
        for (i, q) in w.persons.iter().enumerate() {
            by_loc[pos[q.locality as usize] as usize] = i as u32;
            pos[q.locality as usize] += 1;
        }
        Self { w, p: p.clone(), weight, by_loc_off, by_loc }
    }

    /// Local solar seconds of `person` at UTC `t`.
    #[must_use]
    pub fn local_secs(&self, person: u32, t: f64) -> f64 {
        t + f64::from(self.w.persons[person as usize].lon) / 360.0 * 86_400.0
    }

    fn weekday(&self, t_local: f64) -> usize {
        ((t_local / 86_400.0).floor() as i64 + i64::from(self.p.start_weekday)).rem_euclid(7) as usize
    }

    /// Rate multiplier at local time (diurnal × weekly, mean 1 over a week).
    #[must_use]
    pub fn rhythm(&self, t_local: f64) -> f64 {
        if self.p.flat_rhythm {
            return 1.0;
        }
        match self.p.process {
            // today's law: `workload::compute_arrival_rate` with one act per event
            Process::Poisson => compute_arrival_rate(86_400.0, 1.0, t_local),
            Process::Bursty => {
                let dow = self.weekday(t_local);
                let shift = if dow >= 5 { self.p.weekend_shift_h * 3600.0 } else { 0.0 };
                self.p.weekly[dow] * compute_arrival_rate(86_400.0, 1.0, t_local - shift)
            }
        }
    }

    fn work_affinity(&self, t_local: f64) -> f64 {
        let dow = self.weekday(t_local);
        let h = (t_local / 3600.0).rem_euclid(24.0);
        if dow < 5 && h >= self.p.work_hours.0 && h < self.p.work_hours.1 {
            1.0
        } else {
            self.p.off_hours_work_affinity
        }
    }

    /// Choose scope and room for an immigrant act by `s` at UTC `t`.
    #[must_use]
    pub fn choose(&self, r: &mut Rng, s: u32, t: f64, event: bool) -> (Scope, u32) {
        let rooms = self.w.rooms_of(s);
        let kind = |rm: u32| self.w.room_kind[rm as usize];
        let size = |rm: u32| self.w.members(rm).len();
        let household = rooms.iter().copied().find(|&rm| kind(rm) == RoomKind::Household).unwrap_or(NO_ROOM);
        let aff = self.work_affinity(self.local_secs(s, t));
        let kind_w = |k: RoomKind| -> f64 {
            match k {
                RoomKind::Work => self.p.community_kind_weight[0] * aff,
                RoomKind::School => self.p.community_kind_weight[1] * aff,
                RoomKind::Neighbourhood => self.p.community_kind_weight[2],
                RoomKind::Interest => self.p.community_kind_weight[3],
                _ => 0.0,
            }
        };
        match self.p.scope_choice {
            ScopeChoice::FixedMix => {
                let mix = if event { &self.p.event_mix } else { &self.p.mix };
                let scope = match pick(mix, r.f()) {
                    Some(0) | None => Scope::Family,
                    Some(1) => Scope::Community,
                    Some(2) => Scope::ConsentFree,
                    _ => Scope::Public,
                };
                let room = match scope {
                    Scope::Family => {
                        let kin: Vec<u32> = rooms.iter().copied().filter(|&rm| kind(rm) == RoomKind::Kin).collect();
                        let k = if kin.is_empty() { NO_ROOM } else { kin[r.below(kin.len())] };
                        let first_household = r.chance(self.p.family_household_share);
                        let (a, b) = if first_household { (household, k) } else { (k, household) };
                        if a != NO_ROOM && size(a) > 1 {
                            a
                        } else if b != NO_ROOM && size(b) > 1 {
                            b
                        } else {
                            household
                        }
                    }
                    Scope::Community => {
                        let ws: Vec<f64> = rooms.iter().map(|&rm| kind_w(kind(rm))).collect();
                        // no community room: the page falls back to the family room
                        pick(&ws, r.f()).map_or(household, |i| rooms[i])
                    }
                    Scope::ConsentFree => NO_ROOM,
                    Scope::Public => {
                        let pubs: Vec<u32> = rooms.iter().copied().filter(|&rm| kind(rm) == RoomKind::Public).collect();
                        if pubs.is_empty() { NO_ROOM } else { pubs[r.below(pubs.len())] }
                    }
                };
                (scope, room)
            }
            ScopeChoice::TieWeighted => {
                let mut count = [0u32; 7];
                for &rm in rooms {
                    count[kind(rm) as usize] += 1;
                }
                let mut ws: Vec<f64> = rooms
                    .iter()
                    .map(|&rm| {
                        let k = kind(rm);
                        let a = if matches!(k, RoomKind::Work | RoomKind::School) { aff } else { 1.0 };
                        self.p.tie_weights[k as usize] * a / f64::from(count[k as usize])
                    })
                    .collect();
                ws.push(self.p.tie_weights[7]);
                match pick(&ws, r.f()) {
                    Some(i) if i < rooms.len() => (kind(rooms[i]).scope(), rooms[i]),
                    _ => (Scope::ConsentFree, NO_ROOM),
                }
            }
        }
    }

    fn reply_delay(&self, r: &mut Rng) -> f64 {
        let a = self.p.reply_alpha.max(0.1);
        for _ in 0..16 {
            let d = self.p.reply_scale_s * ((1.0 - r.f()).powf(-1.0 / a) - 1.0);
            if d <= self.p.reply_max_s {
                return d;
            }
        }
        self.p.reply_max_s
    }

    /// Emit `act` and, under Bursty, its cascade of replies inside the horizon.
    fn cascade(&self, r: &mut Rng, act: Act, horizon: f64, sink: &mut impl FnMut(&Act)) {
        let mut stack = vec![act];
        while let Some(a) = stack.pop() {
            sink(&a);
            if self.p.process != Process::Bursty || a.room == NO_ROOM || a.depth >= 1000 {
                continue;
            }
            let m = self.w.members(a.room);
            if m.len() < 2 {
                continue;
            }
            for _ in 0..poisson(r, self.p.branching) {
                let t = a.t + self.reply_delay(r);
                if t >= horizon {
                    continue;
                }
                let mut s = m[r.below(m.len())];
                if s == a.signer {
                    s = m[(m.iter().position(|&x| x == a.signer).unwrap_or(0) + 1 + r.below(m.len() - 1)) % m.len()];
                }
                stack.push(Act { t, signer: s, scope: a.scope, room: a.room, depth: a.depth + 1, from_event: a.from_event });
            }
        }
    }

    /// Expected immigrant acts one random event brings at defaults (reach drawn by share,
    /// locality and region ∝ population).
    #[must_use]
    pub fn expected_acts_per_event(&self) -> f64 {
        let n = self.w.persons.len() as f64;
        if n == 0.0 {
            return 0.0;
        }
        let loc2: f64 = self.w.locality_pop.iter().map(|&x| f64::from(x).powi(2)).sum::<f64>() / n;
        let mut reg = vec![0f64; self.w.region_of_locality.iter().map(|&g| g as usize + 1).max().unwrap_or(1)];
        for q in &self.w.persons {
            reg[q.region as usize] += 1.0;
        }
        let reg2: f64 = reg.iter().map(|x| x * x).sum::<f64>() / n;
        let pops = [loc2, reg2, n];
        (0..3).map(|i| self.p.event_level_shares[i] * self.p.event_participation[i] * pops[i]).sum::<f64>() * self.p.event_acts_per_participant
    }

    /// The background events over `[0, horizon)`, seeded.
    #[must_use]
    pub fn events(&self, horizon: f64, seed: u64) -> Vec<SyncEvent> {
        if self.p.process != Process::Bursty || self.p.event_share <= 0.0 || self.w.persons.is_empty() {
            return Vec::new();
        }
        let mut r = Rng::new(seed ^ 0x6576_656E);
        let n = self.w.persons.len() as f64;
        let budget = self.p.event_share * (1.0 - self.p.branching) * n * self.p.acts_per_person_day / 86_400.0 * horizon;
        let per = self.expected_acts_per_event();
        if per <= 0.0 {
            return Vec::new();
        }
        let count = poisson(&mut r, budget / per);
        (0..count)
            .map(|_| {
                let t0 = r.f() * horizon;
                let who = &self.w.persons[r.below(self.w.persons.len())];
                let level = pick(&self.p.event_level_shares, r.f()).unwrap_or(2);
                let reach = match level {
                    0 => Reach::Locality(who.locality),
                    1 => Reach::Region(who.region),
                    _ => Reach::World,
                };
                SyncEvent { t0, reach, participation: self.p.event_participation[level], acts_per_participant: self.p.event_acts_per_participant, decay_s: self.p.event_decay_s }
            })
            .collect()
    }

    fn reached(&self, reach: Reach) -> Vec<u32> {
        match reach {
            Reach::Locality(k) => self.by_loc[self.by_loc_off[k as usize] as usize..self.by_loc_off[k as usize + 1] as usize].to_vec(),
            Reach::Region(g) => (0..self.w.persons.len() as u32).filter(|&i| self.w.persons[i as usize].region == g).collect(),
            Reach::World => (0..self.w.persons.len() as u32).collect(),
        }
    }

    /// Generate every act in `[0, horizon)` into `sink` (not in time order). `forced` events are
    /// added on top of the background ones (a stress scenario: they add acts beyond the mean).
    pub fn run(&self, horizon: f64, seed: u64, forced: &[SyncEvent], sink: &mut impl FnMut(&Act)) {
        let mut r = Rng::new(seed ^ 0x7275_6E21);
        let base = self.p.acts_per_person_day / 86_400.0;
        let scale = match self.p.process {
            Process::Poisson => 1.0,
            Process::Bursty => (1.0 - self.p.branching) * (1.0 - self.p.event_share),
        };
        let wmax = match self.p.process {
            Process::Poisson => 1.0,
            Process::Bursty => self.p.weekly.iter().copied().fold(0.0, f64::max),
        };
        let peak = if self.p.flat_rhythm { 1.0 } else { DIURNAL_MAX * wmax };
        for s in 0..self.w.persons.len() as u32 {
            let w = if self.p.process == Process::Bursty { self.weight[s as usize] } else { 1.0 };
            let lmax = base * scale * w * peak;
            if lmax <= 0.0 {
                continue;
            }
            let mut t = 0.0;
            loop {
                t += -(1.0 - r.f()).ln() / lmax;
                if t >= horizon {
                    break;
                }
                let lam = base * scale * w * self.rhythm(self.local_secs(s, t));
                if r.f() * lmax < lam {
                    let (scope, room) = self.choose(&mut r, s, t, false);
                    self.cascade(&mut r, Act { t, signer: s, scope, room, depth: 0, from_event: false }, horizon, sink);
                }
            }
        }
        let mut evs = self.events(horizon, seed);
        evs.extend_from_slice(forced);
        for e in evs {
            for s in self.reached(e.reach) {
                if !r.chance(e.participation) {
                    continue;
                }
                for _ in 0..poisson(&mut r, e.acts_per_participant) {
                    let t = e.t0 - e.decay_s * (1.0 - r.f()).ln();
                    if t >= horizon {
                        continue;
                    }
                    let (scope, room) = self.choose(&mut r, s, t, true);
                    self.cascade(&mut r, Act { t, signer: s, scope, room, depth: 0, from_event: true }, horizon, sink);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::social::{Geography, Placement, SocialParams};

    const WEEK: f64 = 7.0 * 86_400.0;

    fn world(n: usize) -> SocialWorld {
        SocialWorld::build(&Geography::stand_in(5, 24, 3, 5), &SocialParams::default(), n, 11, Placement::Placed)
    }

    #[test]
    fn specs_match_defaults() {
        let d = ActivityParams::default();
        let get = |name: &str| ACTIVITY_PARAM_SPECS.iter().find(|s| s.name == name).unwrap().default;
        assert_eq!(get("acts_per_person_day"), d.acts_per_person_day);
        assert_eq!([get("mix_family"), get("mix_community"), get("mix_consent_free"), get("mix_public")], d.mix);
        assert_eq!(get("branching"), d.branching);
        assert_eq!(get("event_share"), d.event_share);
        assert_eq!([get("weekly_weekday"), get("weekly_saturday"), get("weekly_sunday")], [d.weekly[0], d.weekly[5], d.weekly[6]]);
        assert!((d.weekly.iter().sum::<f64>() / 7.0 - 1.0).abs() < 1e-12, "weekly rhythm has mean 1");
        assert!((d.tie_weights.iter().sum::<f64>() - 1.0).abs() < 1e-12);
        assert_eq!(
            [get("event_mix_family"), get("event_mix_community"), get("event_mix_consent_free"), get("event_mix_public")],
            d.event_mix
        );
    }

    #[test]
    fn run_is_deterministic_by_seed() {
        let w = world(400);
        let g = Generator::new(&w, &ActivityParams::default(), 3);
        let collect = |seed| {
            let mut v = Vec::new();
            g.run(86_400.0, seed, &[], &mut |a: &Act| v.push(*a));
            v
        };
        assert_eq!(collect(9), collect(9));
        assert_ne!(collect(9), collect(10));
    }

    #[test]
    fn rhythm_has_mean_one_over_a_week() {
        let w = world(50);
        for p in [ActivityParams::poisson(), ActivityParams::default()] {
            let g = Generator::new(&w, &p, 1);
            let steps = 7 * 24 * 60;
            let dt = WEEK / steps as f64;
            let m: f64 = (0..steps).map(|i| g.rhythm((i as f64 + 0.5) * dt)).sum::<f64>() / steps as f64;
            assert!((m - 1.0).abs() < 1e-3, "{m}");
        }
    }

    /// The bursty process keeps today's mean rate. Over one week, 2,000 persons at 180 acts a day
    /// expect 2,520,000 acts. Poisson's count has sd ~1,600 (0.06%). The bursty count's spread is
    /// dominated by events: at 5% of the budget the week holds a few hundred events of a few
    /// hundred acts each (sd well under 0.5%), and cascades add a factor ~(1-n)^-3/2 on the
    /// Poisson spread. A 1.5% band is therefore several sd wide and not widened to pass.
    #[test]
    fn mean_rate_matches_poisson() {
        let w = world(2000);
        let expect = 2000.0 * 180.0 * 7.0;
        for p in [ActivityParams::poisson(), ActivityParams::default()] {
            let g = Generator::new(&w, &p, 21);
            let mut n = 0u64;
            g.run(WEEK, 21, &[], &mut |_| n += 1);
            let rel = (n as f64 - expect) / expect;
            assert!(rel.abs() < 0.015, "{:?}: {n} vs {expect} ({rel:+.4})", p.process);
        }
    }

    fn dispersion(p: &ActivityParams, seed: u64) -> f64 {
        let w = world(1000);
        let g = Generator::new(&w, p, seed);
        let horizon = 2.0 * 86_400.0;
        let mut bins = vec![0f64; (horizon / 60.0) as usize];
        g.run(horizon, seed, &[], &mut |a| bins[(a.t / 60.0) as usize] += 1.0);
        let m = bins.iter().sum::<f64>() / bins.len() as f64;
        let v = bins.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (bins.len() - 1) as f64;
        v / m
    }

    /// Flat clock, 60-s windows. Poisson's index of dispersion is 1 (sampling sd over 2,880
    /// windows ~ sqrt(2/2880) ~ 0.026); the bursty process clusters acts, so it must sit well above.
    #[test]
    fn bursty_is_overdispersed_poisson_is_not() {
        let mut pp = ActivityParams::poisson();
        pp.flat_rhythm = true;
        let mut pb = ActivityParams::default();
        pb.flat_rhythm = true;
        let dp = dispersion(&pp, 5);
        let db = dispersion(&pb, 5);
        assert!((dp - 1.0).abs() < 0.1, "poisson D = {dp}");
        assert!(db > 2.0, "bursty D = {db}");
    }

    #[test]
    fn fixed_mix_scopes_follow_the_mix() {
        let w = world(1500);
        let g = Generator::new(&w, &ActivityParams::poisson(), 2);
        let mut c = [0f64; 4];
        let mut total = 0.0;
        g.run(86_400.0, 2, &[], &mut |a| {
            c[a.scope as usize] += 1.0;
            total += 1.0;
        });
        for (i, want) in [0.65, 0.30, 0.04, 0.01].iter().enumerate() {
            assert!((c[i] / total - want).abs() < 0.01, "scope {i}: {}", c[i] / total);
        }
    }

    #[test]
    fn forced_event_concentrates_in_time_and_place() {
        let w = world(2000);
        let mut p = ActivityParams::default();
        p.event_share = 0.0;
        let g = Generator::new(&w, &p, 4);
        let k = (0..w.localities()).max_by_key(|&k| w.locality_pop[k]).unwrap() as u16;
        let e = SyncEvent { t0: 40_000.0, reach: Reach::Locality(k), participation: 0.5, acts_per_participant: 3.0, decay_s: 300.0 };
        let (mut base, mut with) = (0u64, 0u64);
        g.run(86_400.0, 4, &[], &mut |a| if a.t >= 40_000.0 && a.t < 40_600.0 { base += 1 });
        g.run(86_400.0, 4, &[e], &mut |a| if a.t >= 40_000.0 && a.t < 40_600.0 { with += 1 });
        let expect_extra = 0.5 * 3.0 * f64::from(w.locality_pop[k as usize]) * (1.0 - (-2.0f64).exp());
        assert!(with as f64 - base as f64 > 0.5 * expect_extra, "{base} -> {with}, expected ~+{expect_extra}");
    }
}
