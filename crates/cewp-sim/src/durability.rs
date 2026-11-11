//! Durability as a measured outcome of placement across failure domains (REVIEW.md §3 item 7;
//! spec `FSD/twin/failures.md`).
//!
//! # What is constitutional and what is this module's MODEL
//!
//! Constitutional (CIRISConstitution `constitution/part_6_the_coherence_mathematics.md`):
//! - CC 6.1.5: a durable item carries an owner-chosen target holder count; the swarm replicates
//!   the item's sealed chunks to that count across the cohort's live nodes, rarest chunk first;
//!   recovery is "pull the chunks from any holder". Chunks are held whole and sealed (CC 5.4.2;
//!   `chunk_custody.rs`).
//! - CC 6.1.5.1: target and floors are counted in holders per item; minimum viable `>= 1`; the
//!   CC pins no default number ("the defaults of this edition are the reference's shipped
//!   values"). Its informative reliability line `1 - (1 - q)^h` is for independent holders and
//!   says "the correlation-discount discipline of this Part applies before any such figure is
//!   quoted".
//! - CC 6.1.2.1.2: the correlated-source discount `k_eff = k / (1 + rho_bar (k - 1))`, `rho_bar`
//!   clamped to `[0, 1]`; fully correlated sources count as one. This module applies that
//!   formula to holders.
//!
//! Not constitutional, and stated here because `chunk_custody.rs` and FSD §3.3 read otherwise:
//! `N_EFF_MIN = 3` and `R9_HOLDER_FLOOR = 9` do not appear in CC 6.1.2.1.2. That section's
//! gates are over aggregation-tier SOURCES (`n_eff * 2 >= source_count`, pinned
//! `min_ratio = 0.5`; `max_source_multiplicity * n_min <= source_count`, `n_min` default 2),
//! not over chunk holders, and "R9" names the residual of CC 8.3.1, not nine holders. This
//! module uses neither constant as a custody rule; it reports the twin's `R9_HOLDER_FLOOR` only
//! as one of the holder counts swept.
//!
//! MODEL (this module's choices): which nodes are eligible holders, how holders are placed
//! ([`Placement`]), whether a holder keeps the whole object or each chunk is placed on its own
//! ([`Granularity`]), and the per-axis correlation weights of [`RhoWeights`]. The chunk size is
//! CIRISEdge's (`src/group_content/store.rs` `CHUNK_BYTES` = 256 KiB at edge 310aa7e), which the
//! code comment there says is not a CC constant.

use crate::failure::{
    evaluate, mix, paired_with, sample_event, AsnKind, Axis, Condition, Dist, DomainWorld, Event, FailureDomainSource,
    FailureParams, NodeKind, Probe, ScenarioKind, State, AXES, F, SCENARIOS,
};
use crate::micro::Rng;
use std::collections::BTreeMap;

/// CIRISEdge `src/group_content/store.rs` `CHUNK_BYTES` (producer's choice, not CC).
pub const CHUNK_BYTES: u64 = 256 * 1024;

/// The review's "10 GB object", read as 10^10 bytes (MODEL reading).
pub const REVIEW_OBJECT_BYTES: u64 = 10_000_000_000;

/// Chunks in an object.
#[must_use]
pub const fn chunks_for(bytes: u64) -> u64 {
    bytes.div_ceil(CHUNK_BYTES)
}

/// How a chunk's holders are chosen (MODEL; the CC says "across the cohort's live nodes,
/// rarest chunk first" and names no domain rule).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Placement {
    /// Distinct holders uniformly over the eligible live nodes: the "replica count" reading.
    Uniform,
    /// Each holder is from the publisher's metro with probability `local_share`, else from
    /// anywhere else: a popular object is held by the audience that pulled it, and audiences
    /// are local (FSD 4.4 locality share 0.65).
    AudienceLocal {
        /// Probability a holder is local.
        local_share: f64,
    },
    /// Holders spread over distinct access networks first (round-robin when holders outnumber
    /// networks), one uniform node per network: what a domain-aware placer could do. The CC
    /// neither requires nor forbids it.
    DomainSpread,
}

impl Placement {
    /// Name.
    #[must_use]
    pub fn label(self) -> String {
        match self {
            Self::Uniform => "uniform".into(),
            Self::AudienceLocal { local_share } => format!("audience_local({local_share})"),
            Self::DomainSpread => "domain_spread".into(),
        }
    }
}

/// What one holder keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Granularity {
    /// Every holder keeps every chunk: the object survives while one holder does.
    WholeObject,
    /// Each chunk's holders are drawn independently: every chunk must survive.
    PerChunk {
        /// Chunks in the object.
        chunks: u64,
    },
}

/// Eligible holders of one placement, with their state.
#[derive(Debug, Clone, Copy)]
pub struct HolderPool<'a> {
    /// Access-network group of each eligible holder.
    pub group: &'a [u32],
    /// Holder is in the publisher's locality.
    pub local: &'a [bool],
    /// Holder serves now.
    pub avail: &'a [bool],
}

/// P(`h` distinct draws from `e` nodes, `a` of them available, are all unavailable). Draws
/// beyond `e` are capped (every node holds). Zero draws: 1 (nothing holds it).
#[must_use]
pub fn hyp_all_lost(e: u64, a: u64, h: u64) -> f64 {
    let h = h.min(e);
    let bad = e - a.min(e);
    if bad < h {
        return 0.0;
    }
    let mut p = 1.0;
    for i in 0..h {
        p *= (bad - i) as f64 / (e - i) as f64;
    }
    p
}

fn binom_pmf(n: u64, p: f64) -> Vec<f64> {
    let mut v = vec![0.0; n as usize + 1];
    let mut c = 1.0f64;
    for k in 0..=n {
        v[k as usize] = c * p.powi(k as i32) * (1.0 - p).powi((n - k) as i32);
        c = c * (n - k) as f64 / (k + 1) as f64;
    }
    v
}

fn groups(pool: &HolderPool<'_>) -> Vec<(u64, u64)> {
    let mut m: BTreeMap<u32, (u64, u64)> = BTreeMap::new();
    for (i, &g) in pool.group.iter().enumerate() {
        let e = m.entry(g).or_insert((0, 0));
        e.0 += 1;
        e.1 += u64::from(pool.avail[i]);
    }
    m.into_values().collect()
}

fn choose(n: u64, k: u64) -> f64 {
    let k = k.min(n - k.min(n));
    let mut c = 1.0;
    for i in 0..k {
        c = c * (n - i) as f64 / (i + 1) as f64;
    }
    c
}

/// Exact P(a chunk's every holder is unavailable) given the state, integrating over placement.
#[must_use]
pub fn p_chunk_lost(pool: &HolderPool<'_>, placement: Placement, h: u32) -> f64 {
    let h = u64::from(h);
    match placement {
        Placement::Uniform => {
            let e = pool.avail.len() as u64;
            let a = pool.avail.iter().filter(|&&x| x).count() as u64;
            hyp_all_lost(e, a, h)
        }
        Placement::AudienceLocal { local_share } => {
            let (mut el, mut al, mut er, mut ar) = (0u64, 0u64, 0u64, 0u64);
            for (i, &l) in pool.local.iter().enumerate() {
                let a = u64::from(pool.avail[i]);
                if l {
                    el += 1;
                    al += a;
                } else {
                    er += 1;
                    ar += a;
                }
            }
            binom_pmf(h, local_share.clamp(0.0, 1.0))
                .iter()
                .enumerate()
                .map(|(l, &w)| {
                    let lk = (l as u64).min(el);
                    w * hyp_all_lost(el, al, lk) * hyp_all_lost(er, ar, h - lk)
                })
                .sum()
        }
        Placement::DomainSpread => {
            let gs = groups(pool);
            let n = gs.len() as u64;
            if n == 0 {
                return 1.0;
            }
            let base = h / n;
            let rem = (h % n) as usize;
            // dp[j]: sum over ways to give `j` groups one extra holder of the product of
            // per-group all-lost probabilities.
            let mut dp = vec![0.0f64; rem + 1];
            dp[0] = 1.0;
            for &(e, a) in &gs {
                let f0 = hyp_all_lost(e, a, base);
                let f1 = hyp_all_lost(e, a, base + 1);
                for j in (0..=rem).rev() {
                    dp[j] = dp[j] * f0 + if j > 0 { dp[j - 1] * f1 } else { 0.0 };
                }
            }
            dp[rem] / choose(n, rem as u64)
        }
    }
}

fn frac_subsets_all_lost(avail: &[bool], k: usize) -> f64 {
    let n = avail.len();
    let k = k.min(n);
    let (mut all, mut lost) = (0u64, 0u64);
    for mask in 0u32..(1u32 << n) {
        if mask.count_ones() as usize != k {
            continue;
        }
        all += 1;
        if (0..n).all(|i| mask & (1 << i) == 0 || !avail[i]) {
            lost += 1;
        }
    }
    lost as f64 / all as f64
}

/// [`p_chunk_lost`] by brute-force enumeration of holder sets (tiny pools only, < 20 nodes).
#[must_use]
pub fn enumerate_p_chunk_lost(pool: &HolderPool<'_>, placement: Placement, h: u32) -> f64 {
    let h = h as usize;
    match placement {
        Placement::Uniform => frac_subsets_all_lost(pool.avail, h),
        Placement::AudienceLocal { local_share } => {
            let loc: Vec<bool> = (0..pool.avail.len()).filter(|&i| pool.local[i]).map(|i| pool.avail[i]).collect();
            let rest: Vec<bool> = (0..pool.avail.len()).filter(|&i| !pool.local[i]).map(|i| pool.avail[i]).collect();
            let mut s = 0.0;
            for l in 0..=h {
                let lk = l.min(loc.len());
                let w = choose(h as u64, l as u64) * local_share.powi(l as i32) * (1.0 - local_share).powi((h - l) as i32);
                s += w * frac_subsets_all_lost(&loc, lk) * frac_subsets_all_lost(&rest, h - lk);
            }
            s
        }
        Placement::DomainSpread => {
            let mut ids: Vec<u32> = pool.group.to_vec();
            ids.sort_unstable();
            ids.dedup();
            let n = ids.len();
            let base = h / n;
            let rem = h % n;
            let members = |g: u32| -> Vec<bool> {
                (0..pool.avail.len()).filter(|&i| pool.group[i] == g).map(|i| pool.avail[i]).collect()
            };
            let (mut tot, mut cnt) = (0.0, 0u64);
            for mask in 0u32..(1u32 << n) {
                if mask.count_ones() as usize != rem {
                    continue;
                }
                cnt += 1;
                let mut prod = 1.0;
                for (j, &g) in ids.iter().enumerate() {
                    let take = base + usize::from(mask & (1 << j) != 0);
                    prod *= if take == 0 { 1.0 } else { frac_subsets_all_lost(&members(g), take) };
                }
                tot += prod;
            }
            tot / cnt as f64
        }
    }
}

/// P(the object is NOT fully recoverable) from the per-chunk loss probability, computed
/// without cancellation so tails near zero keep their digits.
#[must_use]
pub fn p_not_full(p_lost: f64, g: Granularity) -> f64 {
    match g {
        Granularity::WholeObject => p_lost,
        Granularity::PerChunk { chunks } => {
            if p_lost >= 1.0 {
                1.0
            } else {
                -(chunks as f64 * (-p_lost).ln_1p()).exp_m1()
            }
        }
    }
}

/// P(the object is fully recoverable) from the per-chunk loss probability.
#[must_use]
pub fn p_full(p_lost: f64, g: Granularity) -> f64 {
    match g {
        Granularity::WholeObject => 1.0 - p_lost,
        Granularity::PerChunk { chunks } => {
            if p_lost >= 1.0 {
                0.0
            } else {
                (chunks as f64 * (-p_lost).ln_1p()).exp()
            }
        }
    }
}

// ------------------------------------------------------------------ effective replicas

/// The correlated-source discount of CC 6.1.2.1.2: `k / (1 + rho_bar (k - 1))`, `rho_bar`
/// clamped to `[0, 1]` (asserted anti-correlation never mints credit).
#[must_use]
pub fn k_eff(k: usize, rho_bar: f64) -> f64 {
    if k == 0 {
        return 0.0;
    }
    let r = if rho_bar.is_nan() { 1.0 } else { rho_bar.clamp(0.0, 1.0) };
    k as f64 / (1.0 + r * (k as f64 - 1.0))
}

/// Per-axis shared-fate weights for the structural pairwise correlation (MODEL: a pair sharing
/// a household shares power, router, line and usually the person, so it saturates at 1; the
/// rest are judgement, to be replaced by co-failure rates once incident data is approved).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RhoWeights {
    /// Weight per axis, in [`AXES`] order.
    pub w: [f64; 8],
}

impl Default for RhoWeights {
    fn default() -> Self {
        // household, operator, asn, metro, power, country, cloud_region, cloud_provider
        Self { w: [0.6, 0.3, 0.25, 0.1, 0.2, 0.1, 0.4, 0.1] }
    }
}

/// Structural correlation of two holders: the clamped sum of the weights of the axes they share.
#[must_use]
pub fn rho_pair<S: FailureDomainSource + ?Sized>(src: &S, a: usize, b: usize, rw: &RhoWeights) -> f64 {
    let mut s = 0.0;
    for (k, ax) in AXES.iter().enumerate() {
        if let (Some(x), Some(y)) = (src.domain(a, *ax), src.domain(b, *ax)) {
            if x == y {
                s += rw.w[k];
            }
        }
    }
    s.min(1.0)
}

/// Mean pairwise structural correlation of a holder set.
#[must_use]
pub fn rho_bar<S: FailureDomainSource + ?Sized>(src: &S, holders: &[usize], rw: &RhoWeights) -> f64 {
    let k = holders.len();
    if k < 2 {
        return 0.0;
    }
    let mut s = 0.0;
    for i in 0..k {
        for j in (i + 1)..k {
            s += rho_pair(src, holders[i], holders[j], rw);
        }
    }
    s / (k * (k - 1) / 2) as f64
}

/// Effective independent replicas of a holder set (CC 6.1.2.1.2 formula, MODEL weights).
#[must_use]
pub fn k_eff_structural<S: FailureDomainSource + ?Sized>(src: &S, holders: &[usize], rw: &RhoWeights) -> f64 {
    k_eff(holders.len(), rho_bar(src, holders, rw))
}

/// Kish participation ratio of holders over one axis' domains, integer floor over fixed-point
/// masses exactly as the CC 6.1.2.1.2 `n_eff` (each holder's mass is one unit of its domain).
/// Holders with no domain on the axis count as their own singleton domain.
#[must_use]
pub fn axis_n_eff<S: FailureDomainSource + ?Sized>(src: &S, holders: &[usize], axis: Axis) -> u32 {
    let mut m: BTreeMap<(bool, u64), u64> = BTreeMap::new();
    for &h in holders {
        let key = match src.domain(h, axis) {
            Some(d) => (true, d),
            None => (false, h as u64),
        };
        *m.entry(key).or_insert(0) += crate::chunk_custody::PPM_SCALE;
    }
    let masses: Vec<u64> = m.into_values().collect();
    crate::chunk_custody::effective_source_count_fixed_point(&masses)
}

/// The binding (smallest) axis `n_eff` and its axis.
#[must_use]
pub fn min_axis_n_eff<S: FailureDomainSource + ?Sized>(src: &S, holders: &[usize]) -> (u32, Axis) {
    let mut best = (u32::MAX, Axis::Household);
    for ax in AXES {
        let v = axis_n_eff(src, holders, ax);
        if v < best.0 {
            best = (v, ax);
        }
    }
    best
}

/// Draw one holder set from a placement (distinct holders; for reporting k_eff).
pub fn sample_holders(pool: &HolderPool<'_>, placement: Placement, h: u32, r: &mut Rng) -> Vec<usize> {
    let n = pool.group.len();
    let mut out: Vec<usize> = Vec::new();
    let h = (h as usize).min(n);
    match placement {
        Placement::Uniform => {
            let mut idx: Vec<usize> = (0..n).collect();
            for _ in 0..h {
                out.push(idx.swap_remove(r.below(idx.len())));
            }
        }
        Placement::AudienceLocal { local_share } => {
            let mut loc: Vec<usize> = (0..n).filter(|&i| pool.local[i]).collect();
            let mut rest: Vec<usize> = (0..n).filter(|&i| !pool.local[i]).collect();
            for _ in 0..h {
                let from_loc = (r.chance(local_share) && !loc.is_empty()) || rest.is_empty();
                let v = if from_loc { &mut loc } else { &mut rest };
                out.push(v.swap_remove(r.below(v.len())));
            }
        }
        Placement::DomainSpread => {
            let mut by: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
            for i in 0..n {
                by.entry(pool.group[i]).or_default().push(i);
            }
            let mut gs: Vec<Vec<usize>> = by.into_values().collect();
            while out.len() < h {
                // A random order of the groups that still have nodes, one holder each.
                let mut order: Vec<usize> = (0..gs.len()).filter(|&g| !gs[g].is_empty()).collect();
                if order.is_empty() {
                    break;
                }
                while !order.is_empty() && out.len() < h {
                    let g = order.swap_remove(r.below(order.len()));
                    let v = &mut gs[g];
                    out.push(v.swap_remove(r.below(v.len())));
                }
            }
        }
    }
    out
}

// ------------------------------------------------------------------ pools on a DomainWorld

/// Eligible holders of an object published by `publisher` (person): holder-kind nodes up at
/// publish time (`base`). Returns node ids, groups (access network) and locality flags.
#[must_use]
pub fn world_pool(w: &DomainWorld, base: &State, publisher: usize) -> (Vec<usize>, Vec<u32>, Vec<bool>) {
    let metro = w.persons[publisher].metro;
    let mut ids = Vec::new();
    let mut grp = Vec::new();
    let mut loc = Vec::new();
    for (i, n) in w.nodes.iter().enumerate() {
        if n.kind.holder_eligible() && base.up[i] {
            ids.push(i);
            grp.push(w.normal_asn(n));
            loc.push(n.metro == metro);
        }
    }
    (ids, grp, loc)
}

/// The harness probe: mean over drawn publishers of P(full recovery now) and of the per-chunk
/// loss probability.
#[must_use]
pub fn probe_publishers(w: &DomainWorld, base: &State, st: &State, probe: &Probe, seed: u64) -> (f64, f64) {
    let mut r = Rng::new(seed);
    let g = Granularity::PerChunk { chunks: chunks_for(probe.bytes) };
    let (mut d, mut c) = (0.0, 0.0);
    let n = probe.publishers.max(1);
    for _ in 0..n {
        let pb = r.below(w.persons.len());
        let (ids, grp, loc) = world_pool(w, base, pb);
        let avail: Vec<bool> = ids.iter().map(|&i| st.serves_mainland(w, i)).collect();
        let pool = HolderPool { group: &grp, local: &loc, avail: &avail };
        let pl = p_chunk_lost(&pool, probe.placement, probe.holders);
        c += pl;
        d += p_not_full(pl, g);
    }
    (d / n as f64, c / n as f64)
}

/// Empirical mean pairwise correlation of the availability of `holders` over draws of the
/// scenario library (each draw one scenario, kinds in rotation). DERIVED from MODEL inputs.
/// A holder that never changes state across the draws has no defined correlation and pairs
/// with it count as 0.
#[must_use]
pub fn rho_empirical(w: &DomainWorld, p: &FailureParams, holders: &[usize], draws: usize, seed: u64) -> f64 {
    let k = holders.len();
    if k < 2 || draws == 0 {
        return 0.0;
    }
    let mut x = vec![vec![0.0f64; draws]; k];
    let mut r = Rng::new(seed);
    for d in 0..draws {
        let ev = sample_event(w, p, SCENARIOS[d % SCENARIOS.len()], &mut r);
        let evs = [ev];
        let st = evaluate(w, p, &Condition { events: &evs, t_h: p.get(F::EvalHour), removed: None, rejoin: true });
        for (j, &h) in holders.iter().enumerate() {
            x[j][d] = f64::from(u8::from(st.serves_mainland(w, h)));
        }
    }
    let mean: Vec<f64> = x.iter().map(|v| v.iter().sum::<f64>() / draws as f64).collect();
    let var: Vec<f64> = x.iter().zip(&mean).map(|(v, m)| v.iter().map(|a| (a - m) * (a - m)).sum::<f64>()).collect();
    let mut s = 0.0;
    for i in 0..k {
        for j in (i + 1)..k {
            if var[i] > 0.0 && var[j] > 0.0 {
                let cov: f64 = (0..draws).map(|d| (x[i][d] - mean[i]) * (x[j][d] - mean[j])).sum();
                s += cov / (var[i] * var[j]).sqrt();
            }
        }
    }
    s / (k * (k - 1) / 2) as f64
}

// ------------------------------------------------------------------ the review's question

/// One configuration of the review's question.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReviewConfig {
    /// Placement.
    pub placement: Placement,
    /// Holders (target holder count).
    pub holders: u32,
    /// Granularity.
    pub granularity: Granularity,
}

/// Distribution over seeds for one configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct ReviewRow {
    /// Configuration.
    pub cfg: ReviewConfig,
    /// P(NOT fully recoverable) while the two ISPs are down and 30% of home nodes are gone.
    pub during: Dist,
    /// P(NOT fully recoverable) after the ISPs return (only the permanent 30% loss remains).
    pub after: Dist,
    /// Bytes held per node kind (`NodeKind as usize`) by one drawn holder set, mean over seeds
    /// (HOOKS.md 1; DERIVED: holders x object bytes, split by the drawn holders' kinds).
    pub bytes_held_by_kind: [f64; 5],
    /// Structural effective replicas of a drawn holder set (CC 6.1.2.1.2 discount, MODEL weights).
    pub k_eff: Dist,
    /// Binding per-axis Kish `n_eff` of a drawn holder set.
    pub axis_n_eff: Dist,
}

/// Inputs of the named scenario "a popular 10 GB object after the simultaneous loss of two
/// major ISPs and 30% of home nodes".
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReviewScenario {
    /// Major ISPs lost (drawn from the largest `major_pool` fixed ISPs).
    pub isps: usize,
    /// Pool of "major" ISPs.
    pub major_pool: usize,
    /// Share of home-class nodes (home nodes, laptops) permanently lost.
    pub home_loss: f64,
    /// Publisher drawn among customers of the lost ISPs (else among everyone).
    pub publisher_on_lost_isp: bool,
    /// Object bytes.
    pub bytes: u64,
}

impl Default for ReviewScenario {
    fn default() -> Self {
        Self { isps: 2, major_pool: 4, home_loss: 0.30, publisher_on_lost_isp: false, bytes: REVIEW_OBJECT_BYTES }
    }
}

/// Run the named scenario for each configuration over `seeds` seeds. The world's states are
/// computed once per seed and shared by every configuration.
#[must_use]
pub fn review_question(
    w: &DomainWorld,
    p: &FailureParams,
    sc: &ReviewScenario,
    cfgs: &[ReviewConfig],
    seeds: usize,
) -> Vec<ReviewRow> {
    let rw = RhoWeights::default();
    let mut during = vec![Vec::new(); cfgs.len()];
    let mut after = vec![Vec::new(); cfgs.len()];
    let mut keff = vec![Vec::new(); cfgs.len()];
    let mut axn = vec![Vec::new(); cfgs.len()];
    let mut held = vec![[0.0f64; 5]; cfgs.len()];
    let majors = w.fixed_isps_by_size();
    let base = evaluate(w, p, &Condition { events: &[], t_h: p.get(F::EvalHour), removed: None, rejoin: true });
    for s in 0..seeds as u64 {
        let mut r = Rng::new(mix(mix(w.seed, 0x10_6B), s));
        let mut pool: Vec<u32> = majors.iter().copied().take(sc.major_pool.max(sc.isps)).collect();
        let mut ev = Event::manual(ScenarioKind::IspOutage, r.next_u64(), f64::INFINITY);
        while ev.asns.len() < sc.isps && !pool.is_empty() {
            ev.asns.push(pool.swap_remove(r.below(pool.len())));
        }
        ev.scope = format!("fixed asns {:?}", ev.asns);
        let mut removed = vec![false; w.nodes.len()];
        for (i, n) in w.nodes.iter().enumerate() {
            if n.kind.home_class() && r.chance(sc.home_loss) {
                removed[i] = true;
            }
        }
        let evs = [ev];
        let t = p.get(F::EvalHour);
        let st_during = evaluate(w, p, &Condition { events: &evs, t_h: t, removed: Some(&removed), rejoin: true });
        let st_after = evaluate(w, p, &Condition { events: &[], t_h: t, removed: Some(&removed), rejoin: true });
        let lost_isps = &evs[0].asns;
        let publisher = if sc.publisher_on_lost_isp {
            let cands: Vec<usize> = (0..w.persons.len())
                .filter(|&q| lost_isps.contains(&w.nodes[w.persons[q].phone as usize].fixed_asn))
                .collect();
            if cands.is_empty() { r.below(w.persons.len()) } else { cands[r.below(cands.len())] }
        } else {
            r.below(w.persons.len())
        };
        let (ids, grp, loc) = world_pool(w, &base, publisher);
        let av_d: Vec<bool> = ids.iter().map(|&i| st_during.serves_mainland(w, i)).collect();
        let av_a: Vec<bool> = ids.iter().map(|&i| st_after.serves_mainland(w, i)).collect();
        let pd = HolderPool { group: &grp, local: &loc, avail: &av_d };
        let pa = HolderPool { group: &grp, local: &loc, avail: &av_a };
        for (c, cfg) in cfgs.iter().enumerate() {
            during[c].push(p_not_full(p_chunk_lost(&pd, cfg.placement, cfg.holders), cfg.granularity));
            after[c].push(p_not_full(p_chunk_lost(&pa, cfg.placement, cfg.holders), cfg.granularity));
            let mut hr = Rng::new(mix(s, c as u64));
            let set: Vec<usize> = sample_holders(&pd, cfg.placement, cfg.holders, &mut hr).into_iter().map(|j| ids[j]).collect();
            for &h in &set {
                held[c][w.nodes[h].kind as usize] += sc.bytes as f64 / seeds.max(1) as f64;
            }
            keff[c].push(k_eff_structural(w, &set, &rw));
            axn[c].push(f64::from(min_axis_n_eff(w, &set).0));
        }
    }
    cfgs.iter()
        .enumerate()
        .map(|(c, cfg)| ReviewRow {
            cfg: *cfg,
            during: Dist::of(&during[c]),
            after: Dist::of(&after[c]),
            k_eff: Dist::of(&keff[c]),
            axis_n_eff: Dist::of(&axn[c]),
            bytes_held_by_kind: held[c],
        })
        .collect()
}

/// The sweep the report prints: three placements x holders {9, 30, 100} x both granularities.
#[must_use]
pub fn review_sweep() -> Vec<ReviewConfig> {
    let chunks = chunks_for(REVIEW_OBJECT_BYTES);
    let mut v = Vec::new();
    for placement in [Placement::Uniform, Placement::AudienceLocal { local_share: 0.65 }, Placement::DomainSpread] {
        for holders in [crate::chunk_custody::R9_HOLDER_FLOOR, crate::chunk_custody::DEFAULT_TARGET_HOLDERS, 100] {
            for granularity in [Granularity::WholeObject, Granularity::PerChunk { chunks }] {
                v.push(ReviewConfig { placement, holders, granularity });
            }
        }
    }
    v
}

/// Domain-vs-random durability for one hand-built ISP outage (used by the spec's claim C2).
#[must_use]
pub fn isp_outage_pair(w: &DomainWorld, p: &FailureParams, asn: u32, seed: u64, probe: &Probe) -> crate::failure::PairedRun {
    debug_assert_eq!(w.asns[asn as usize].kind, AsnKind::Fixed);
    let mut ev = Event::manual(ScenarioKind::IspOutage, seed, f64::INFINITY);
    ev.asns.push(asn);
    paired_with(w, p, ev, seed, probe)
}

/// Kinds counted as home holders in reports.
#[must_use]
pub fn holder_kinds() -> [NodeKind; 3] {
    [NodeKind::HomeNode, NodeKind::Laptop, NodeKind::Hosted]
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Labels(Vec<[Option<u64>; 8]>);
    impl FailureDomainSource for Labels {
        fn node_count(&self) -> usize {
            self.0.len()
        }
        fn domain(&self, node: usize, axis: Axis) -> Option<u64> {
            self.0[node][AXES.iter().position(|a| *a == axis).unwrap_or(0)]
        }
    }

    /// Nine holders behind one ISP (all else distinct) against nine behind nine ISPs.
    #[test]
    fn nine_on_one_isp_count_fewer_than_nine_on_nine() {
        let mk = |same_isp: bool| {
            Labels(
                (0..9u64)
                    .map(|i| {
                        [Some(i), Some(i), Some(if same_isp { 0 } else { i }), Some(i), Some(i), Some(i), None, None]
                    })
                    .collect(),
            )
        };
        let one = mk(true);
        let nine = mk(false);
        let hs: Vec<usize> = (0..9).collect();
        let rw = RhoWeights::default();
        let k1 = k_eff_structural(&one, &hs, &rw);
        let k9 = k_eff_structural(&nine, &hs, &rw);
        assert!((k9 - 9.0).abs() < 1e-12, "independent holders count fully: {k9}");
        assert!((k1 - 9.0 / (1.0 + 0.25 * 8.0)).abs() < 1e-12, "one-ISP: {k1}");
        assert!(k1 < k9);
        assert_eq!(min_axis_n_eff(&one, &hs), (1, Axis::Asn));
        assert_eq!(min_axis_n_eff(&nine, &hs).0, 9);
        // One household: fully correlated, counts as one (CC 6.1.2.1.2).
        let house = Labels((0..9u64).map(|i| [Some(0), Some(i), Some(0), Some(0), Some(0), Some(0), None, None]).collect());
        assert!((k_eff_structural(&house, &hs, &rw) - 1.0).abs() < 1e-12);
        assert!((k_eff(5, -0.5) - 5.0).abs() < 1e-12, "negative rho is clamped, never credits more than k");
    }

    /// The same check on availability under the scenario library: nine home holders on the
    /// largest ISP co-fail more than nine on nine different ISPs.
    #[test]
    fn one_isp_holders_co_fail_more_in_the_scenario_library() {
        let mut p = FailureParams::default();
        p.set(F::Nodes, 3_000.0);
        let w = DomainWorld::build(11, &p);
        let base = evaluate(&w, &p, &Condition { events: &[], t_h: 1.0, removed: None, rejoin: true });
        let homes: Vec<usize> = (0..w.nodes.len())
            .filter(|&i| w.nodes[i].kind == NodeKind::HomeNode && base.serves_mainland(&w, i))
            .collect();
        let big = w.fixed_isps_by_size()[0];
        let one: Vec<usize> = homes.iter().copied().filter(|&i| w.nodes[i].fixed_asn == big).take(9).collect();
        let mut seen = Vec::new();
        let mut spread = Vec::new();
        for &i in &homes {
            let a = w.nodes[i].fixed_asn;
            if !seen.contains(&a) && seen.len() < 9 {
                seen.push(a);
                spread.push(i);
            }
        }
        assert_eq!((one.len(), spread.len()), (9, 9));
        let r1 = rho_empirical(&w, &p, &one, 180, 5);
        let r9 = rho_empirical(&w, &p, &spread, 180, 5);
        assert!(r1 > r9 + 0.1, "one ISP rho {r1} vs spread rho {r9}");
        assert!(k_eff(9, r1) < k_eff(9, r9));
    }

    /// The named review scenario is deterministic by seed and orders placements the way the
    /// spec says: at nine holders per chunk, uniform placement loses the object more often than
    /// domain spread; whole-object holding is never worse than per-chunk.
    #[test]
    fn review_question_is_deterministic_and_ordered() {
        let mut p = FailureParams::default();
        p.set(F::Nodes, 3_000.0);
        let w = DomainWorld::build(3, &p);
        let chunks = chunks_for(REVIEW_OBJECT_BYTES);
        let cfgs: Vec<ReviewConfig> = [Placement::Uniform, Placement::DomainSpread]
            .iter()
            .flat_map(|&placement| {
                [Granularity::WholeObject, Granularity::PerChunk { chunks }]
                    .map(|granularity| ReviewConfig { placement, holders: 9, granularity })
            })
            .collect();
        let sc = ReviewScenario::default();
        let a = review_question(&w, &p, &sc, &cfgs, 40);
        assert_eq!(a, review_question(&w, &p, &sc, &cfgs, 40));
        assert!(a[1].during.mean > a[3].during.mean, "uniform {} vs spread {}", a[1].during.mean, a[3].during.mean);
        for pair in a.chunks(2) {
            assert!(pair[0].during.mean <= pair[1].during.mean);
            assert!(pair[0].after.mean <= pair[0].during.mean + 1e-15);
        }
        assert!(a[2].k_eff.p50 > a[0].k_eff.p50, "spread holders are more independent");
    }

    #[test]
    fn hypergeometric_edges() {
        assert_eq!(hyp_all_lost(10, 0, 3), 1.0);
        assert_eq!(hyp_all_lost(10, 8, 3), 0.0);
        assert_eq!(hyp_all_lost(5, 1, 0), 1.0);
        assert!((hyp_all_lost(10, 5, 2) - (5.0 / 10.0) * (4.0 / 9.0)).abs() < 1e-15);
        assert_eq!(hyp_all_lost(3, 1, 9), 0.0, "every node holds when holders exceed nodes");
        assert_eq!(chunks_for(REVIEW_OBJECT_BYTES), 38_147);
    }

    /// The exact formulas against brute-force holder-set enumeration, on every state of a tiny
    /// pool.
    #[test]
    fn exact_formula_matches_holder_enumeration() {
        let group = [0u32, 0, 0, 1, 1, 2, 3];
        let local = [true, true, false, false, true, false, false];
        let placements = [Placement::Uniform, Placement::AudienceLocal { local_share: 0.65 }, Placement::DomainSpread];
        for mask in 0u32..(1 << group.len()) {
            let avail: Vec<bool> = (0..group.len()).map(|i| mask & (1 << i) != 0).collect();
            let pool = HolderPool { group: &group, local: &local, avail: &avail };
            for pl in placements {
                for h in 1..=6u32 {
                    let a = p_chunk_lost(&pool, pl, h);
                    let b = enumerate_p_chunk_lost(&pool, pl, h);
                    assert!((a - b).abs() < 1e-12, "{pl:?} h={h} mask={mask:b}: {a} vs {b}");
                }
            }
        }
    }

    /// The durability estimator (Monte Carlo over failure states, exact over placement) against
    /// exhaustive enumeration of every failure state on a tiny world: 4 access networks that
    /// fail independently with q, 8 nodes that fail independently with p.
    #[test]
    fn estimator_agrees_with_exhaustive_enumeration() {
        let group = [0u32, 0, 0, 1, 1, 2, 2, 3];
        let local = [true, true, true, false, false, false, true, false];
        let (q, pn) = (0.3, 0.2);
        let gran = Granularity::PerChunk { chunks: 3 };
        for pl in [Placement::Uniform, Placement::AudienceLocal { local_share: 0.65 }, Placement::DomainSpread] {
            for h in [2u32, 3] {
                // Exhaustive: every (network state, node state), brute-force holder sets.
                let (mut ex, mut ex2) = (0.0, 0.0);
                for gm in 0u32..16 {
                    let pg: f64 = (0..4).map(|g| if gm & (1 << g) != 0 { q } else { 1.0 - q }).product();
                    for nm in 0u32..256 {
                        let pnm: f64 = (0..8).map(|i| if nm & (1 << i) != 0 { pn } else { 1.0 - pn }).product();
                        let avail: Vec<bool> =
                            (0..8).map(|i| nm & (1 << i) == 0 && gm & (1 << group[i]) == 0).collect();
                        let pool = HolderPool { group: &group, local: &local, avail: &avail };
                        let v = p_full(enumerate_p_chunk_lost(&pool, pl, h), gran);
                        ex += pg * pnm * v;
                        ex2 += pg * pnm * v * v;
                    }
                }
                // Estimator: seeded draws of the failure state, exact placement formula.
                let n = 100_000;
                let mut r = Rng::new(42);
                let mut est = 0.0;
                for _ in 0..n {
                    let gdown: Vec<bool> = (0..4).map(|_| r.chance(q)).collect();
                    let avail: Vec<bool> = (0..8).map(|i| !r.chance(pn) && !gdown[group[i] as usize]).collect();
                    let pool = HolderPool { group: &group, local: &local, avail: &avail };
                    est += p_full(p_chunk_lost(&pool, pl, h), gran);
                }
                est /= n as f64;
                let sd = ((ex2 - ex * ex).max(0.0) / n as f64).sqrt();
                assert!((est - ex).abs() <= 4.0 * sd + 1e-12, "{pl:?} h={h}: est {est} exact {ex} sd {sd}");
            }
        }
    }

    #[test]
    fn per_chunk_is_never_better_than_whole_object() {
        for pl in [0.0, 1e-9, 1e-6, 0.01, 0.5, 1.0] {
            let g = Granularity::PerChunk { chunks: 38_147 };
            assert!((p_not_full(pl, g) - (1.0 - p_full(pl, g))).abs() < 1e-9);
            assert!(p_full(pl, Granularity::PerChunk { chunks: 38_147 }) <= p_full(pl, Granularity::WholeObject) + 1e-15);
        }
    }
}
