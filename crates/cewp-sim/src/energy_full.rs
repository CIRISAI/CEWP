//! TEST 1, THE THEORY: full replacement of the internet by the mesh (`FSD/twin/energy.md` §0).
//!
//! Eric: "the whole assumption is if we replaced the internet entirely with cewp"; "we do not
//! need massive datacenters". So substitution is 1 by definition, and the baseline is the WHOLE
//! data-centre estate the replaced internet would no longer need, per user. The estate's work
//! splits three ways, as explicit inputs because nobody publishes the split:
//!
//! * (a) VANISHES: harvesting, profiling, ranking, advertising and the duplication that serves
//!   them;
//! * (b) RELOCATES to members' devices (their own messages and media, storage with the mesh's
//!   bounded retention, compute they still want, AI if "entirely" includes it), costing energy
//!   at the edge at `edge efficiency ratio` times what it cost in a data centre (a phone or
//!   home node is less efficient per unit of work and has no PUE: the ratio is an input and may
//!   lie either side of 1);
//! * (c) STAYS regardless: data-centre work that is not the consumer internet (enterprise,
//!   science, government), and the access and core networks, which are baseload (the network is
//!   not in the estate figure, and it stays on both sides, so it does not enter).
//!
//! The mesh side counts only what the mesh ADDS (people use phones and run gateways regardless):
//! term A + B of the household pass (idle-schedule radio and wake cost, relaying, signatures,
//! the hosted seat's share, added devices under the host choice), plus the relocated work (b).
//!
//! Output per person-year in kWh: baseline removed, relocated work at the edge, mesh additions,
//! net, each with bounds over every input's interval (the net is multilinear in the inputs, so
//! its extremes lie at corners, and every corner is evaluated), the corner nearest zero, and a
//! break-even table. TEST 2 (the adoption path: one more household joining while the platforms
//! still run) is `energy.rs`.

use crate::energy::{self, Class, Counters, EnergyParams, Run, Scenario};
use crate::workload::ParameterRegime;

/// One input row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FullSpec {
    /// Name.
    pub name: &'static str,
    /// Unit.
    pub unit: &'static str,
    /// Low, default, high.
    pub lo: f64,
    /// Default.
    pub mid: f64,
    /// High.
    pub hi: f64,
    /// Class.
    pub class: Class,
    /// Year.
    pub year: u16,
    /// Source and basis.
    pub source: &'static str,
}

macro_rules! full_rows {
    ($( $idx:ident = $name:literal, $unit:literal, [$lo:expr, $mid:expr, $hi:expr], $cls:ident, $year:literal, $src:literal; )*) => {
        /// Index into [`FULL_SPECS`].
        #[allow(non_camel_case_types)]
        #[repr(usize)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum F { $( $idx, )* }
        /// The inputs of the full-replacement test.
        pub const FULL_SPECS: &[FullSpec] = &[ $( FullSpec { name: $name, unit: $unit, lo: $lo, mid: $mid, hi: $hi, class: Class::$cls, year: $year, source: $src }, )* ];
        impl F {
            /// Every row.
            pub const ALL: &'static [F] = &[ $( F::$idx, )* ];
        }
    };
}

full_rows! {
    EstateTwh = "estate.all_data_centres_twh_per_year", "TWh/year", [340.0, 415.0, 460.0], Model, 2024,
        "CIRISHolon POWER-0 row 43 (415 TWh, 2024: a model estimate of EVERY data-centre workload); interval ours (estimates of the order of 340 to 460 are commonly quoted; not re-read)";
    InternetUsers = "estate.internet_users_billion", "billion", [5.0, 5.5, 5.8], Frontier, 2024,
        "Holon's arithmetic: about 5.5 billion internet users, from its memory, not a read figure";
    AiShare = "estate.ai_share", "share", [0.05, 0.15, 0.30], Frontier, 2024,
        "the share of the estate that is AI training and inference: the fastest-growing part and the least clearly relocatable (UPSTREAM-0 row 0 includes generative AI); no published split";
    StaysShare = "estate.stays_share", "share of the non-AI estate", [0.2, 0.4, 0.6], Frontier, 2024,
        "(c) data-centre work that is not the consumer internet (enterprise, science, government) and stays whatever the internet runs on; no published split";
    VanishShare = "estate.vanish_share", "share of the consumer part", [0.2, 0.5, 0.8], Frontier, 2024,
        "(a) harvesting, profiling, ranking, advertising and the duplication that serves them: UPSTREAM-0 row 25 puts them at 0.5 to 5.5 of one operator's 5.5 kWh per person (9% to 100%), rows 18-24 at 10% to over 40% of server energy; NOBODY PUBLISHES (a) against (b)";
    EdgeRatio = "edge.efficiency_ratio", "x data-centre energy per unit of work", [0.3, 1.0, 3.0], Frontier, 2026,
        "(b) relocated non-AI work at the edge: storage and serving on devices already on cost only a marginal draw (below 1), compute on a phone or home node is less efficient per unit and has no PUE (above 1)";
    AiWantedShare = "ai.wanted_inference_share", "share of the estate's AI energy", [0.2, 0.5, 0.8], Frontier, 2021,
        "accounting only (it splits the removed AI into what vanishes and today's wanted inference, which the mesh's own CIRISProxy-shaped inference replaces; it does not move the net): inference people actually ask for, against ranking, ads and training on harvested data: UPSTREAM-0 rows 18-25 (ML 10-15% of one operator's energy; 10:20:70 experimentation:training:inference for one fleet; the 0.5-5.5 kWh harvesting bracket); no published split";
    InteractionsPerPersonDay = "ai.interactions_per_person_day", "agent interactions", [1.0, 5.0, 30.0], Frontier, 2026,
        "how often a person asks their agent for something; no figure (CIRISProxy's aggregate counters could measure it: see the measurement request)";
    CallsPerInteraction = "ai.llm_calls_per_interaction", "LLM calls", [12.0, 30.0, 70.0], Derived, 2026,
        "STATED by the project, not measured: CIRISProxy README (CIRISAI/CIRISProxy 6f2f640) '1 credit = 1 user interaction (12-70 LLM calls typical)'";
    TokensPerCall = "ai.tokens_per_call", "tokens (prompt + completion)", [500.0, 2000.0, 6000.0], Frontier, 2026,
        "agent calls carry long prompts (context, constitution, history) and short completions; CIRISProxy's litellm_config disables reasoning on DeepInfra Qwen (measured there: 1,193 tokens to 29 for one answer)";
    WhPer1kTokens = "ai.tier.hosted.wh_per_1k_tokens.mid", "Wh per 1,000 tokens", [0.01, 0.05, 0.3], Frontier, 2026,
        "hosted tier, zero retention (CIRISProxy today): the mid-size open-weight class it routes (Qwen 3.6/3.5 35B-A3B, about 3B active; Qwen 3.6 27B; Llama 4 Scout/Maverick, 17B active; gpt-oss-20b) at data-centre utilisation; no sourced figure";
    HostedSmallFactor = "ai.tier.hosted.small_class_factor", "x the mid class", [0.2, 0.3, 0.5], Model, 2026,
        "a small (1-4B) model per token against the mid class, hosted";
    HostedLargeFactor = "ai.tier.hosted.large_class_factor", "x the mid class", [2.0, 4.0, 8.0], Model, 2026,
        "a large (70B dense and up) model per token against the mid class, hosted";
    PhoneWhPer1k = "ai.tier.device.wh_per_1k_tokens.small", "Wh per 1,000 tokens (ADDED draw)", [0.02, 0.05, 0.2], Frontier, 2026,
        "phone or laptop on-device, small class only; hardware already on: only the added draw counts (mesh side)";
    ProsumerWhPer1k = "ai.tier.prosumer.wh_per_1k_tokens.mid", "Wh per 1,000 tokens (ADDED draw)", [0.05, 0.2, 0.6], Frontier, 2026,
        "a prosumer machine with a consumer GPU, up to the mid class; often on anyway; added draw (mesh side)";
    ApplianceWhPer1k = "ai.tier.appliance.wh_per_1k_tokens.mid", "Wh per 1,000 tokens (ADDED draw)", [0.03, 0.1, 0.4], Frontier, 2026,
        "an office or home appliance with an inference accelerator, up to the mid class; on all day; added draw (mesh side)";
    EdgeServerWhPer1k = "ai.tier.edge_server.wh_per_1k_tokens.large", "Wh per 1,000 tokens", [0.1, 0.4, 1.5], Frontier, 2026,
        "an edge or on-premises server, up to the large class, at low utilisation (its idle draw amortised into the figure; hardware added, mesh side)";
    SmallJobShare = "ai.jobs.small_share", "share of calls", [0.2, 0.5, 0.8], Frontier, 2026,
        "share of model calls a small on-device model can serve; no figure";
    LargeJobShare = "ai.jobs.large_share", "share of calls", [0.0, 0.05, 0.2], Frontier, 2026,
        "share of calls that need a large model; the rest need the mid class";
    ForcedLocalShare = "ai.alloc.forced_local_share", "share of calls", [0.2, 0.2, 0.2], Frontier, 2026,
        "the allocation rule's lever: share forced to the most local able tier for privacy or offline use; the rest go to the lowest-energy able tier; swept 0 to 1 in the report";
    PersonsPerHousehold = "household.persons", "persons", [2.5, 2.5, 2.5], Model, 2026,
        "the household pass's default (micro.rs); per person-year = household-hour figures x 8,766 / 2.5";
}

/// The mesh's additions per person-year, from the household pass (term A + B, expected regime,
/// stand-in presence; `lo` / `hi` from that pass's vertex bounds, PRESENCE-0 inside them).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Additions {
    /// Node on the existing gateway (fixed-line homes) and phones only (cellular-only homes):
    /// the US 2020 mix.
    pub gateway: [f64; 3],
    /// A new always-on box in every home that can take one.
    pub new_box: [f64; 3],
}

const J_HH_TO_KWH_PY: f64 = 8766.0 / 3.6e6;

impl Additions {
    /// Read from the household pass (Test 2's engine) with `seed`'s counters.
    #[must_use]
    pub fn from_household_pass(k: &Counters) -> Self {
        let p = EnergyParams::default();
        let ab = |s: Scenario| {
            let d = energy::delta(&p, k, s, ParameterRegime::Expected, Run::Long);
            let persons = p.get(energy::E::PersonsPerHousehold);
            let f = |x: f64| x * J_HH_TO_KWH_PY / persons;
            [f(d.bounds[0].lo + d.bounds[1].lo), f(d.point[0] + d.point[1]), f(d.bounds[0].hi + d.bounds[1].hi)]
        };
        Self { gateway: ab(Scenario::MixGateway), new_box: ab(Scenario::MixBox) }
    }
}

/// How "entirely" treats AI. Eric: "a hybrid means a mixture of on-device, edge, prosumer,
/// office appliance, servers, etc... - the market optimizes".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiCase {
    /// The design: each model call goes to a tier (phone/laptop, prosumer machine, office or
    /// home appliance, edge or on-premises server, hosted with zero retention) by the
    /// allocation rule.
    Mix,
    /// Bound: every call hosted with zero retention (CIRISProxy today).
    AllHosted,
    /// Bound: every call on the most local tier able to run it.
    AllOnDevices,
    /// AI out of scope.
    NoAi,
}

impl AiCase {
    /// All four, the design first.
    pub const ALL: [Self; 4] = [Self::Mix, Self::AllHosted, Self::AllOnDevices, Self::NoAi];
    /// Name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Mix => "tiered mix (the design)",
            Self::AllHosted => "all hosted, zero retention (bound)",
            Self::AllOnDevices => "all on devices (bound)",
            Self::NoAi => "no AI in scope",
        }
    }
}

/// The tiers.
pub const TIERS: [&str; 5] = ["phone/laptop on-device", "prosumer machine", "office or home appliance", "edge or on-premises server", "hosted, zero retention"];

/// Energy per 1,000 tokens of a job class (0 small, 1 mid, 2 large) at a tier, or `None` when the
/// tier cannot run it.
fn tier_wh(p: &FullParams, tier: usize, class: usize) -> Option<f64> {
    let mid = p.get(F::WhPer1kTokens);
    match (tier, class) {
        (0, 0) => Some(p.get(F::PhoneWhPer1k)),
        (1, 0 | 1) => Some(p.get(F::ProsumerWhPer1k) * if class == 0 { p.get(F::HostedSmallFactor) } else { 1.0 }),
        (2, 0 | 1) => Some(p.get(F::ApplianceWhPer1k) * if class == 0 { p.get(F::HostedSmallFactor) } else { 1.0 }),
        (3, _) => Some(p.get(F::EdgeServerWhPer1k) * [p.get(F::HostedSmallFactor) / p.get(F::HostedLargeFactor), 1.0 / p.get(F::HostedLargeFactor), 1.0][class]),
        (4, _) => Some(mid * [p.get(F::HostedSmallFactor), 1.0, p.get(F::HostedLargeFactor)][class]),
        _ => None,
    }
}

/// Where a call of `class` goes under `case`: the tier.
fn place(p: &FullParams, case: AiCase, class: usize, forced: bool) -> usize {
    let local = |c: usize| -> usize {
        // the most local able tier: the phone for small, the cheaper of prosumer and appliance
        // for mid, the edge server for large
        match c {
            0 => 0,
            1 => if tier_wh(p, 1, 1) <= tier_wh(p, 2, 1) { 1 } else { 2 },
            _ => 3,
        }
    };
    match case {
        AiCase::AllHosted => 4,
        AiCase::AllOnDevices => local(class),
        _ if forced => local(class),
        _ => (0..5).filter_map(|t| tier_wh(p, t, class).map(|w| (t, w))).min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(core::cmp::Ordering::Equal)).map_or(4, |x| x.0),
    }
}

/// The AI term split: (energy at the edge tiers, the mesh side; energy at the hosted tier, kept
/// in data centres), kWh per person-year, and the share of tokens at each tier.
#[must_use]
pub fn ai_split(p: &FullParams, case: AiCase) -> (f64, f64, [f64; 5]) {
    if case == AiCase::NoAi {
        return (0.0, 0.0, [0.0; 5]);
    }
    let mtok = p.get(F::InteractionsPerPersonDay) * 365.25 * p.get(F::CallsPerInteraction) * p.get(F::TokensPerCall) / 1000.0; // thousands of tokens a year
    let (sm, lg) = (p.get(F::SmallJobShare), p.get(F::LargeJobShare));
    let classes = [sm, (1.0 - sm - lg).max(0.0), lg];
    let f = p.get(F::ForcedLocalShare);
    let (mut edge, mut hosted, mut share) = (0.0, 0.0, [0.0; 5]);
    for (c, &w) in classes.iter().enumerate() {
        for (forced, part) in [(true, f), (false, 1.0 - f)] {
            let t = place(p, case, c, forced);
            let kwh = mtok * w * part * tier_wh(p, t, c).unwrap_or(0.0) / 1000.0;
            share[t] += w * part;
            if t == 4 { hosted += kwh } else { edge += kwh }
        }
    }
    (edge, hosted, share)
}

/// Values of every row (plus the mesh additions and the new-box share of homes).
#[derive(Debug, Clone, PartialEq)]
pub struct FullParams {
    v: Vec<f64>,
    /// Mesh additions, node on gateway mix (kWh per person-year).
    pub add_gateway: f64,
    /// Mesh additions, new-box mix.
    pub add_box: f64,
    /// Share of homes with a new always-on box (the host-choice lever, 0 to 1).
    pub new_box_share: f64,
    /// How AI is treated.
    pub case: AiCase,
}

impl FullParams {
    /// Defaults with the household pass's additions.
    #[must_use]
    pub fn defaults(a: &Additions, case: AiCase) -> Self {
        Self { v: FULL_SPECS.iter().map(|s| s.mid).collect(), add_gateway: a.gateway[1], add_box: a.new_box[1], new_box_share: 0.0, case }
    }
    /// Value.
    #[must_use]
    pub fn get(&self, f: F) -> f64 {
        self.v[f as usize]
    }
    /// Set.
    pub fn set(&mut self, f: F, x: f64) {
        self.v[f as usize] = x;
    }
}

/// The four quantities, kWh per person-year.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Full {
    /// The estate per user.
    pub estate_per_user: f64,
    /// Baseline removed: the data-centre work that leaves (vanishes or relocates).
    pub removed: f64,
    /// Of it, the work that vanishes.
    pub vanished: f64,
    /// The relocated work's energy at the edge.
    pub relocated_at_edge: f64,
    /// The AI calls placed at the hosted tier (zero retention): energy that stays in data centres.
    pub inference_kept: f64,
    /// What the mesh adds (A + B).
    pub additions: f64,
    /// `additions + relocated_at_edge - removed`: negative is a saving.
    pub net: f64,
}

/// The AI term under `case`, kWh per person-year (edge tiers plus hosted).
#[must_use]
pub fn ai_demand_kwh(p: &FullParams, case: AiCase) -> f64 {
    let (e, h, _) = ai_split(p, case);
    e + h
}

/// Evaluate.
#[must_use]
pub fn eval(p: &FullParams) -> Full {
    let e = p.get(F::EstateTwh) * 1e9 / (p.get(F::InternetUsers) * 1e9); // kWh per user-year
    let ai = e * p.get(F::AiShare);
    let consumer = (e - ai) * (1.0 - p.get(F::StaysShare));
    let vanished = consumer * p.get(F::VanishShare);
    let relocated = consumer - vanished;
    let mut removed = consumer;
    let mut at_edge = relocated * p.get(F::EdgeRatio);
    let mut kept = 0.0;
    let mut vanished = vanished;
    if p.case != AiCase::NoAi {
        // all of today's AI estate leaves its current form: the harvesting, ranking and
        // advertising part (and training on retained data) vanishes; today's wanted inference is
        // replaced by the mesh's own, placed over the tiers
        removed += ai;
        vanished += ai * (1.0 - p.get(F::AiWantedShare));
        let (edge, hosted, _) = ai_split(p, p.case);
        kept = hosted;
        at_edge += edge;
    }
    let additions = (1.0 - p.new_box_share) * p.add_gateway + p.new_box_share * p.add_box;
    Full { estate_per_user: e, removed, vanished, relocated_at_edge: at_edge, inference_kept: kept, additions, net: additions + at_edge + kept - removed }
}

/// Extremes of `net` over every corner of the free rows' intervals and the additions'
/// intervals: (min, max, the corner of the max as (row name, value) pairs). The net is
/// multilinear in each input (InternetUsers enters as 1/x, monotone), so a corner holds each
/// extreme.
#[must_use]
pub fn corners(a: &Additions, case: AiCase, new_box_share: f64) -> (f64, f64, Vec<(&'static str, f64)>) {
    corners_fixed(a, case, new_box_share, None)
}

/// [`corners`] with interactions per person per day fixed at `m` (excluded from the corners).
#[must_use]
pub fn corners_at(a: &Additions, case: AiCase, new_box_share: f64, m: f64) -> (f64, f64, Vec<(&'static str, f64)>) {
    if case == AiCase::NoAi {
        return corners(a, case, new_box_share);
    }
    corners_fixed(a, case, new_box_share, Some(m))
}

fn corners_fixed(a: &Additions, case: AiCase, new_box_share: f64, m: Option<f64>) -> (f64, f64, Vec<(&'static str, f64)>) {
    let used = |f: F| match f {
        F::InteractionsPerPersonDay => case != AiCase::NoAi && m.is_none(),
        F::EstateTwh | F::InternetUsers | F::AiShare | F::StaysShare | F::VanishShare | F::EdgeRatio | F::PersonsPerHousehold => true,
        // AI rows: only where AI is in scope; the shares and the hosted factors also matter to
        // every AI case, the tier energies only where the case can place a call there
        F::WhPer1kTokens => matches!(case, AiCase::Mix | AiCase::AllHosted),
        F::PhoneWhPer1k | F::ProsumerWhPer1k | F::ApplianceWhPer1k | F::EdgeServerWhPer1k => matches!(case, AiCase::Mix | AiCase::AllOnDevices),
        // accounting only: it does not move the net
        F::AiWantedShare => false,
        _ => case != AiCase::NoAi,
    };
    let free: Vec<F> = F::ALL.iter().copied().filter(|&f| FULL_SPECS[f as usize].lo < FULL_SPECS[f as usize].hi && used(f)).collect();
    let n = free.len() + 2;
    // the default point is in the hull too (the mix's placement makes its net piecewise)
    let mut d = FullParams::defaults(a, case);
    d.new_box_share = new_box_share;
    if let Some(m) = m {
        d.set(F::InteractionsPerPersonDay, m);
    }
    let dn = eval(&d).net;
    let (mut lo, mut hi) = (dn, dn);
    let mut arg = Vec::new();
    for mask in 0u32..(1 << n) {
        let mut p = FullParams::defaults(a, case);
        p.new_box_share = new_box_share;
        if let Some(m) = m {
            p.set(F::InteractionsPerPersonDay, m);
        }
        let mut set = Vec::new();
        for (i, &f) in free.iter().enumerate() {
            let s = &FULL_SPECS[f as usize];
            let x = if mask >> i & 1 == 1 { s.hi } else { s.lo };
            p.set(f, x);
            set.push((s.name, x));
        }
        p.add_gateway = if mask >> free.len() & 1 == 1 { a.gateway[2] } else { a.gateway[0] };
        p.add_box = if mask >> (free.len() + 1) & 1 == 1 { a.new_box[2] } else { a.new_box[0] };
        set.push(("mesh additions, gateway mix (kWh/person-year)", p.add_gateway));
        set.push(("mesh additions, new-box mix (kWh/person-year)", p.add_box));
        let v = eval(&p).net;
        lo = lo.min(v);
        if v > hi {
            hi = v;
            arg = set;
        }
    }
    (lo, hi, arg)
}

/// Value of one input (others at default) where the net is zero, if any in [0, 100 x hi].
#[must_use]
pub fn break_even(a: &Additions, case: AiCase, which: &str) -> Option<f64> {
    let base = FullParams::defaults(a, case);
    let f = |x: f64| {
        let mut p = base.clone();
        match which {
            "new_box_share" => p.new_box_share = x,
            "additions" => {
                p.add_gateway = x;
            }
            name => {
                let r = F::ALL.iter().copied().find(|&r| FULL_SPECS[r as usize].name == name).expect("row");
                p.set(r, x);
            }
        }
        eval(&p).net
    };
    let hi = match which {
        "new_box_share" => 1.0,
        "additions" => 1000.0,
        name => F::ALL.iter().map(|&r| &FULL_SPECS[r as usize]).find(|s| s.name == name).map_or(1.0, |s| s.hi * 100.0),
    };
    let steps = 2000;
    let mut prev = (1e-9, f(1e-9));
    for i in 1..=steps {
        let x = hi * f64::from(i) / f64::from(steps);
        let y = f(x);
        if prev.1.signum() != y.signum() {
            let (mut l, mut h) = (prev.0, x);
            for _ in 0..60 {
                let m = 0.5 * (l + h);
                if f(m).signum() == f(l).signum() { l = m } else { h = m }
            }
            return Some(0.5 * (l + h));
        }
        prev = (x, y);
    }
    None
}

/// The markdown report (`cewp-cli energy-full [seed]`).
#[must_use]
pub fn report(seed: u64) -> String {
    let k = Counters::from_hooks(seed);
    let a = Additions::from_household_pass(&k);
    let mut s = String::from("# TEST 1, THE THEORY: full replacement of the internet (substitution = 1)\n\nPer person-year, kWh. Baseline: the whole data-centre estate per internet user. Its work vanishes (a), relocates to members' devices at an edge efficiency ratio (b), or stays (c: non-consumer data-centre work; networks are baseload on both sides and do not enter). The mesh side is only what the mesh adds (the household pass's terms A + B) plus the relocated work. Every split input is FRONTIER: nobody publishes (a) against (b). Bounds: every corner of every input's interval (exact for this multilinear net).\n\n");
    s.push_str("## Inputs\n\n| row | unit | lo / default / hi | class | year | source and basis |\n|---|---|---|---|---|---|\n");
    for r in FULL_SPECS {
        s.push_str(&format!("| `{}` | {} | {} / {} / {} | {} | {} | {} |\n", r.name, r.unit, r.lo, r.mid, r.hi, r.class.as_str(), r.year, r.source));
    }
    s.push_str(&format!("| mesh additions, gateway mix | kWh/person-year | {:.2} / {:.2} / {:.2} | FRONTIER | 2026 | the household pass (energy.rs), US 2020 mix, node on gateway, terms A + B, expected regime |\n", a.gateway[0], a.gateway[1], a.gateway[2]));
    s.push_str(&format!("| mesh additions, new-box mix | kWh/person-year | {:.2} / {:.2} / {:.2} | FRONTIER | 2026 | same, a new always-on box in every home that can take one |\n\n", a.new_box[0], a.new_box[1], a.new_box[2]));
    s.push_str("## Result\n\nAI cases: the **tiered mix (the design)**, Eric: \"a hybrid means a mixture of on-device, edge, prosumer, office appliance, servers, etc... - the market optimizes\". Each model call goes to a tier by an explicit allocation rule: a forced-local share (privacy, offline) to the most local tier able to run it, the rest to the lowest-energy able tier. A cost-minimising market is not the same as an energy-minimising one (cost also prices privacy, latency and hardware already owned), so the energy result can sit a little either side of the lowest-energy placement. Beside it, two labelled BOUNDS: all hosted with zero retention (CIRISProxy today) and all on devices. What vanishes is the same in every AI case: retention, and training, ranking and advertising on harvested data. ZDR removes retention and training on the conversation; it does not remove inference energy.\n\n| AI case | agent interactions per person-day | new-box share of homes | removed (of which vanishes) | relocated at the edge (AI on edge tiers included) | inference kept in data centres | mesh additions | net (default) | net over every corner | sign |\n|---|---|---|---|---|---|---|---|---|---|\n");
    for case in AiCase::ALL {
        let demands: &[f64] = if case == AiCase::NoAi { &[5.0] } else { &[1.0, 5.0, 30.0] };
        for &m in demands {
            for nb in [0.0, 1.0] {
                let mut p = FullParams::defaults(&a, case);
                p.new_box_share = nb;
                p.set(F::InteractionsPerPersonDay, m);
                let r = eval(&p);
                let (lo, hi, _) = corners_at(&a, case, nb, m);
                let sign = if hi < 0.0 { "SAVING over the whole input box" } else if lo > 0.0 { "COST over the whole input box" } else { "open: the box spans zero" };
                s.push_str(&format!("| {} | {} | {} | {:.1} ({:.1}) | {:.1} | {:.1} | {:.2} | {:.1} | [{:.1}, {:.1}] | {} |\n", case.as_str(), if case == AiCase::NoAi { "-".to_string() } else { format!("{m}") }, nb, r.removed, r.vanished, r.relocated_at_edge, r.inference_kept, r.additions, r.net, lo, hi, sign));
            }
        }
    }
    s.push_str("\n**The AI term by case** (interactions per day x 365.25 x LLM calls per interaction x tokens per call, placed over the tiers; kWh per person-year at the defaults) and where the calls go:\n\n| AI case | AI term | of which on edge tiers (mesh side) | of which hosted (stays in data centres) |");
    for t in TIERS {
        s.push_str(&format!(" {t} |"));
    }
    s.push_str("\n|---|---|---|---|---|---|---|---|---|\n");
    for case in [AiCase::Mix, AiCase::AllHosted, AiCase::AllOnDevices] {
        let p = FullParams::defaults(&a, case);
        let (e, h, sh) = ai_split(&p, case);
        s.push_str(&format!("| {} | {:.2} | {:.2} | {:.2} |", case.as_str(), e + h, e, h));
        for x in sh {
            s.push_str(&format!(" {:.0}% |", 100.0 * x));
        }
        s.push('\n');
    }
    s.push_str("\n**How many agent interactions per person per day the saving can carry** (others at default, no new boxes):\n\n| AI case | interactions per day at zero |\n|---|---|\n");
    for case in [AiCase::Mix, AiCase::AllHosted, AiCase::AllOnDevices] {
        s.push_str(&format!("| {} | {} |\n", case.as_str(), break_even(&a, case, "ai.interactions_per_person_day").map_or("more than the scan reaches (3,000)".to_string(), |x| format!("{x:.1}"))));
    }
    s.push_str("\nThe tiered mix, the allocation rule's lever (forced-local share) and the job-size distribution (small-model share), net at the defaults, no new boxes:\n\n| forced-local share | small-model share 0.2 | 0.5 | 0.8 |\n|---|---|---|---|\n");
    for fl in [0.0, 0.2, 0.5, 1.0] {
        s.push_str(&format!("| {fl} |"));
        for sm in [0.2, 0.5, 0.8] {
            let mut p = FullParams::defaults(&a, AiCase::Mix);
            p.set(F::ForcedLocalShare, fl);
            p.set(F::SmallJobShare, sm);
            s.push_str(&format!(" {:.1} |", eval(&p).net));
        }
        s.push('\n');
    }
    for case in AiCase::ALL {
        let (lo, hi, arg) = corners(&a, case, 0.0);
        s.push_str(&format!("\n{}, no new boxes: best corner {:.1}; worst corner {:.1} kWh per person-year at ", case.as_str(), lo, hi));
        s.push_str(&arg.iter().map(|(n, v)| format!("`{n}` = {v:.3}")).collect::<Vec<_>>().join(", "));
        s.push_str(". (The mix places each call at the cheaper tier at each corner, so its net is not multilinear; the corners are a search there, not a guarantee.)\n");
    }
    s.push_str("\n## Break-even for this test (one input, others at default; no new boxes)\n\n| input | tiered mix (the design) | all hosted (bound) | all on devices (bound) | no AI |\n|---|---|---|---|---|\n");
    let names: Vec<&str> = ["estate.vanish_share", "edge.efficiency_ratio", "estate.stays_share", "new_box_share", "ai.interactions_per_person_day", "ai.llm_calls_per_interaction", "ai.tokens_per_call", "ai.tier.hosted.wh_per_1k_tokens.mid", "ai.tier.device.wh_per_1k_tokens.small", "ai.tier.appliance.wh_per_1k_tokens.mid", "ai.tier.edge_server.wh_per_1k_tokens.large", "ai.jobs.small_share", "ai.alloc.forced_local_share", "additions"].to_vec();
    for n in names {
        let f = |c| break_even(&a, c, n).map_or("none in range".to_string(), |x| format!("{x:.3}"));
        let label = match n {
            "new_box_share" => "share of homes with a new always-on box",
            "additions" => "mesh additions (kWh per person-year)",
            x => x,
        };
        s.push_str(&format!("| `{label}` | {} | {} | {} | {} |\n", f(AiCase::Mix), f(AiCase::AllHosted), f(AiCase::AllOnDevices), f(AiCase::NoAi)));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn adds() -> Additions {
        Additions { gateway: [0.5, 3.0, 8.0], new_box: [10.0, 22.0, 40.0] }
    }

    #[test]
    fn holons_arithmetic_and_the_box() {
        let p = FullParams::defaults(&adds(), AiCase::AllOnDevices);
        let r = eval(&p);
        // 415 TWh over 5.5 billion users: about 75 kWh per user-year (8.6 W)
        assert!((r.estate_per_user - 75.45).abs() < 0.1);
        assert!((r.removed - (r.vanished + (r.removed - r.vanished))).abs() < 1e-9);
        // a 5 W box per home of 2.5 is about 17.5 kWh per person-year
        assert!((5.0_f64 * 8766.0 / 1000.0 / 2.5 - 17.53).abs() < 0.01);
    }

    #[test]
    fn corners_bracket_the_default_and_move_the_right_way() {
        for ai in AiCase::ALL {
            let a = adds();
            let (lo, hi, _) = corners(&a, ai, 0.0);
            let n = eval(&FullParams::defaults(&a, ai)).net;
            assert!(lo <= n && n <= hi);
            let mut p = FullParams::defaults(&a, ai);
            let base = eval(&p).net;
            p.set(F::VanishShare, 0.8);
            assert!(eval(&p).net < base, "more work vanishing saves more");
            p.set(F::EdgeRatio, 3.0);
            let worse = eval(&p).net;
            p.set(F::EdgeRatio, 0.3);
            assert!(eval(&p).net < worse, "a more efficient edge saves more");
        }
        // all AI on devices at the dearest device tiers costs more than no AI
        let mut p = FullParams::defaults(&adds(), AiCase::AllOnDevices);
        p.set(F::ProsumerWhPer1k, 0.6);
        p.set(F::ApplianceWhPer1k, 0.4);
        p.set(F::InteractionsPerPersonDay, 30.0);
        let q = FullParams::defaults(&adds(), AiCase::NoAi);
        assert!(eval(&p).net > eval(&q).net);
    }

    #[test]
    fn the_mix_sits_between_its_bounds_and_places_by_energy() {
        let a = adds();
        let p = FullParams::defaults(&a, AiCase::Mix);
        let mix = ai_demand_kwh(&p, AiCase::Mix);
        let hosted = ai_demand_kwh(&p, AiCase::AllHosted);
        let devices = ai_demand_kwh(&p, AiCase::AllOnDevices);
        // with no forced-local share the mix is the cheapest placement, so no dearer than either
        let mut q = p.clone();
        q.set(F::ForcedLocalShare, 0.0);
        let free = ai_demand_kwh(&q, AiCase::Mix);
        assert!(free <= hosted + 1e-12 && free <= devices + 1e-12);
        // forcing calls local moves the mix toward the device bound
        let mut r = p.clone();
        r.set(F::ForcedLocalShare, 1.0);
        assert!((ai_demand_kwh(&r, AiCase::Mix) - devices).abs() < 1e-9);
        assert!(mix.is_finite());
        // all hosted at the defaults: 5 x 365.25 x 30 x 2,000 / 1,000 x (0.5 x 0.3 + 0.45 x 1 + 0.05 x 4) x 0.05 Wh
        let want = 5.0 * 365.25 * 30.0 * 2.0 * (0.5 * 0.3 + 0.45 + 0.05 * 4.0) * 0.05 / 1000.0;
        assert!((hosted - want).abs() < 1e-9, "{hosted} {want}");
        // tiers' shares sum to one
        let (_, _, sh) = ai_split(&p, AiCase::Mix);
        assert!((sh.iter().sum::<f64>() - 1.0).abs() < 1e-12);
        // more interactions cost more, in every AI case
        for c in [AiCase::Mix, AiCase::AllHosted, AiCase::AllOnDevices] {
            let mut x = FullParams::defaults(&a, c);
            let n0 = eval(&x).net;
            x.set(F::InteractionsPerPersonDay, 30.0);
            assert!(eval(&x).net > n0);
        }
    }

    #[test]
    fn break_even_is_a_zero() {
        let a = adds();
        if let Some(z) = break_even(&a, AiCase::NoAi, "edge.efficiency_ratio") {
            let mut p = FullParams::defaults(&a, AiCase::NoAi);
            p.set(F::EdgeRatio, z);
            assert!(eval(&p).net.abs() < 1e-6);
        }
    }
}
