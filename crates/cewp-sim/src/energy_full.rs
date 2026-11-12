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
    AiEdgeRatio = "edge.ai_efficiency_ratio", "x data-centre energy per unit of work", [1.0, 3.0, 10.0], Frontier, 2026,
        "AI the person still wants, run on their own devices: accelerators in a data centre are far more efficient per unit of inference";
    AiWantedShare = "ai.wanted_inference_share", "share of the estate's AI energy", [0.2, 0.5, 0.8], Frontier, 2021,
        "accounting only (it splits the removed AI into what vanishes and today's wanted inference, which the mesh's own CIRISProxy-shaped inference replaces; it does not move the net): inference people actually ask for, against ranking, ads and training on harvested data: UPSTREAM-0 rows 18-25 (ML 10-15% of one operator's energy; 10:20:70 experimentation:training:inference for one fleet; the 0.5-5.5 kWh harvesting bracket); no published split";
    InteractionsPerPersonDay = "ai.interactions_per_person_day", "agent interactions", [1.0, 5.0, 30.0], Frontier, 2026,
        "how often a person asks their agent for something; no figure (CIRISProxy's aggregate counters could measure it: see the measurement request)";
    CallsPerInteraction = "ai.llm_calls_per_interaction", "LLM calls", [12.0, 30.0, 70.0], Derived, 2026,
        "STATED by the project, not measured: CIRISProxy README (CIRISAI/CIRISProxy 6f2f640) '1 credit = 1 user interaction (12-70 LLM calls typical)'";
    TokensPerCall = "ai.tokens_per_call", "tokens (prompt + completion)", [500.0, 2000.0, 6000.0], Frontier, 2026,
        "agent calls carry long prompts (context, constitution, history) and short completions; CIRISProxy's litellm_config disables reasoning on DeepInfra Qwen (measured there: 1,193 tokens to 29 for one answer)";
    WhPer1kTokens = "ai.wh_per_1k_tokens.mid_size_open_weight", "Wh per 1,000 tokens", [0.01, 0.05, 0.3], Frontier, 2026,
        "the model class CIRISProxy routes: mid-size open-weight models (Qwen 3.6/3.5 35B-A3B, about 3B active; Qwen 3.6 27B; Llama 4 Scout/Maverick, 17B active; gpt-oss-20b), not frontier-scale, served by DeepInfra, OpenRouter, Groq and Together at data-centre utilisation; a few 70B dense models are also in the config; no sourced figure";
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

/// How "entirely" treats AI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiCase {
    /// The CIRIS design (Eric: "a hybrid, we use openrouter with ZDR"): inference people ask for
    /// stays in data centres at data-centre efficiency (neither removed nor relocated), scaled by
    /// the demand multiple; the AI that exists to harvest, profile, rank and advertise, and
    /// training on retained user data, vanishes (zero data retention removes that loop).
    Hybrid,
    /// Counterfactual: all AI relocates onto members' devices at the AI edge ratio.
    AllOnDevices,
    /// AI out of scope: the AI part of the estate is neither removed nor added.
    NoAi,
}

impl AiCase {
    /// All three, the design first.
    pub const ALL: [Self; 3] = [Self::Hybrid, Self::AllOnDevices, Self::NoAi];
    /// Name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Hybrid => "HYBRID (the design: hosted inference via a zero-retention router)",
            Self::AllOnDevices => "all AI on devices (counterfactual)",
            Self::NoAi => "no AI in scope",
        }
    }
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
    /// Hybrid only: share of the wanted inference served on members' devices (0 in the design;
    /// small local models are plausible for some tasks).
    pub on_device: f64,
}

impl FullParams {
    /// Defaults with the household pass's additions.
    #[must_use]
    pub fn defaults(a: &Additions, case: AiCase) -> Self {
        Self { v: FULL_SPECS.iter().map(|s| s.mid).collect(), add_gateway: a.gateway[1], add_box: a.new_box[1], new_box_share: 0.0, case, on_device: 0.0 }
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
    /// Hybrid: the wanted inference still served in data centres (at the demand multiple).
    pub inference_kept: f64,
    /// What the mesh adds (A + B).
    pub additions: f64,
    /// `additions + relocated_at_edge - removed`: negative is a saving.
    pub net: f64,
}

/// The hybrid's AI demand, kWh per person-year: interactions per day x 365.25 x calls per
/// interaction x tokens per call x Wh per 1,000 tokens.
#[must_use]
pub fn ai_demand_kwh(p: &FullParams) -> f64 {
    p.get(F::InteractionsPerPersonDay) * 365.25 * p.get(F::CallsPerInteraction) * p.get(F::TokensPerCall) / 1000.0 * p.get(F::WhPer1kTokens) / 1000.0
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
    match p.case {
        AiCase::NoAi => {}
        AiCase::AllOnDevices => {
            removed += ai;
            at_edge += ai * p.get(F::AiEdgeRatio);
        }
        AiCase::Hybrid => {
            // all of today's AI leaves its current form: the harvesting part vanishes; the
            // wanted inference comes back at the demand multiple, in data centres or (share
            // `on_device`) on members' devices
            removed += ai;
            vanished += ai * (1.0 - p.get(F::AiWantedShare));
            // the mesh's own inference, through a CIRISProxy-shaped router: interactions x calls
            // x tokens x energy per token for the routed model class (kWh per person-year)
            let wanted = ai_demand_kwh(p);
            kept = wanted * (1.0 - p.on_device);
            at_edge += wanted * p.on_device * p.get(F::AiEdgeRatio);
        }
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
    if case != AiCase::Hybrid {
        return corners(a, case, new_box_share);
    }
    corners_fixed(a, case, new_box_share, Some(m))
}

fn corners_fixed(a: &Additions, case: AiCase, new_box_share: f64, m: Option<f64>) -> (f64, f64, Vec<(&'static str, f64)>) {
    let used = |f: F| match f {
        F::AiEdgeRatio => case == AiCase::AllOnDevices,
        F::AiWantedShare | F::CallsPerInteraction | F::TokensPerCall | F::WhPer1kTokens => case == AiCase::Hybrid,
        F::InteractionsPerPersonDay => case == AiCase::Hybrid && m.is_none(),
        _ => true,
    };
    let free: Vec<F> = F::ALL.iter().copied().filter(|&f| FULL_SPECS[f as usize].lo < FULL_SPECS[f as usize].hi && used(f)).collect();
    let n = free.len() + 2;
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
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
            "on_device" => p.on_device = x,
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
        "new_box_share" | "on_device" => 1.0,
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
    s.push_str("## Result\n\n| AI case | agent interactions per person-day | new-box share of homes | estate per user | removed (of which vanishes) | relocated at the edge | inference kept in data centres | mesh additions | net (default) | net over every corner | sign |\n|---|---|---|---|---|---|---|---|---|---|---|\n");
    for case in AiCase::ALL {
        let demands: &[f64] = if case == AiCase::Hybrid { &[1.0, 5.0, 30.0] } else { &[5.0] };
        for &m in demands {
            for nb in [0.0, 0.25, 1.0] {
                let mut p = FullParams::defaults(&a, case);
                p.new_box_share = nb;
                p.set(F::InteractionsPerPersonDay, m);
                let r = eval(&p);
                let (lo, hi, _) = corners_at(&a, case, nb, m);
                let sign = if hi < 0.0 { "SAVING over the whole input box" } else if lo > 0.0 { "COST over the whole input box" } else { "open: the box spans zero" };
                s.push_str(&format!("| {} | {} | {} | {:.1} | {:.1} ({:.1}) | {:.1} | {:.1} | {:.2} | {:.1} | [{:.1}, {:.1}] | {} |\n", case.as_str(), m, nb, r.estate_per_user, r.removed, r.vanished, r.relocated_at_edge, r.inference_kept, r.additions, r.net, lo, hi, sign));
            }
        }
    }
    // the hybrid's AI term on its own, with its interval (corners of its four inputs)
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    let rows4 = [F::InteractionsPerPersonDay, F::CallsPerInteraction, F::TokensPerCall, F::WhPer1kTokens];
    for mask in 0u32..16 {
        let mut p = FullParams::defaults(&a, AiCase::Hybrid);
        for (i, &f) in rows4.iter().enumerate() {
            let sp = &FULL_SPECS[f as usize];
            p.set(f, if mask >> i & 1 == 1 { sp.hi } else { sp.lo });
        }
        let v = ai_demand_kwh(&p);
        lo = lo.min(v);
        hi = hi.max(v);
    }
    let pd = FullParams::defaults(&a, AiCase::Hybrid);
    let per_interaction_wh = pd.get(F::CallsPerInteraction) * pd.get(F::TokensPerCall) / 1000.0 * pd.get(F::WhPer1kTokens);
    s.push_str(&format!("\n**The hybrid's AI term** (interactions per day x 365.25 x LLM calls per interaction x tokens per call x Wh per 1,000 tokens for the routed model class): {:.2} kWh per person-year at the defaults ({:.1} Wh per agent interaction), [{:.3}, {:.0}] over its four inputs' corners. The calls per interaction (12-70) are STATED by CIRISProxy's README, not measured.\n", ai_demand_kwh(&pd), per_interaction_wh, lo, hi));
    let be = break_even(&a, AiCase::Hybrid, "ai.interactions_per_person_day");
    s.push_str(&format!("\n**How many agent interactions per person per day the saving can carry** (others at default, no new boxes): {}. ZDR removes retention and training on the conversation at the provider; it does not remove the inference energy, which is this term.\n", be.map_or("more than the scan reaches".to_string(), |x| format!("{x:.1}"))));
    s.push_str("\nUnder the hybrid the theory no longer turns on how efficient a device is at AI: the inference people want stays in data centres at data-centre efficiency (the AI edge ratio enters only through the on-device lever, 0 in the design).\n\nHybrid, the on-device lever (share of wanted inference served on members' devices, AI edge ratio 3, no new boxes):\n\n| on-device share | net (default) |\n|---|---|\n");
    for d in [0.0, 0.1, 0.25, 0.5, 1.0] {
        let mut p = FullParams::defaults(&a, AiCase::Hybrid);
        p.on_device = d;
        s.push_str(&format!("| {d} | {:.1} |\n", eval(&p).net));
    }
    for case in AiCase::ALL {
        let (lo, hi, arg) = corners(&a, case, 0.0);
        s.push_str(&format!("\n{}, no new boxes: best corner {:.1}; worst corner (nearest zero from the saving side when the default saves, else the farthest from a saving) {:.1} kWh per person-year at ", case.as_str(), lo, hi));
        s.push_str(&arg.iter().map(|(n, v)| format!("`{n}` = {v:.3}")).collect::<Vec<_>>().join(", "));
        s.push_str(".\n");
    }
    s.push_str("\n## Break-even for this test (one input, others at default; no new boxes)\n\n| input | HYBRID | all AI on devices | no AI |\n|---|---|---|---|\n");
    let names: Vec<&str> = ["estate.vanish_share", "edge.efficiency_ratio", "estate.stays_share", "new_box_share", "ai.interactions_per_person_day", "ai.llm_calls_per_interaction", "ai.tokens_per_call", "ai.wh_per_1k_tokens.mid_size_open_weight", "ai.wanted_inference_share", "estate.ai_share", "on_device", "edge.ai_efficiency_ratio", "additions"].to_vec();
    for n in names {
        let f = |c| break_even(&a, c, n).map_or("none in range".to_string(), |x| format!("{x:.3}"));
        let label = match n {
            "new_box_share" => "share of homes with a new always-on box",
            "additions" => "mesh additions (kWh per person-year)",
            "on_device" => "share of wanted inference served on devices",
            x => x,
        };
        s.push_str(&format!("| `{label}` | {} | {} | {} |\n", f(AiCase::Hybrid), f(AiCase::AllOnDevices), f(AiCase::NoAi)));
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
        // AI in at a high edge ratio costs more than AI out
        let mut p = FullParams::defaults(&adds(), AiCase::AllOnDevices);
        p.set(F::AiEdgeRatio, 10.0);
        let q = FullParams::defaults(&adds(), AiCase::NoAi);
        assert!(eval(&p).net > eval(&q).net);
    }

    #[test]
    fn the_hybrid_keeps_wanted_inference_in_data_centres() {
        let a = adds();
        let mut p = FullParams::defaults(&a, AiCase::Hybrid);
        let r = eval(&p);
        assert!((r.inference_kept - ai_demand_kwh(&p)).abs() < 1e-12);
        // 5 a day x 365.25 x 30 calls x 2,000 tokens x 0.05 Wh per 1k = 5.48 kWh
        assert!((ai_demand_kwh(&p) - 5.479).abs() < 0.01);
        // the device AI ratio does not move the design (on-device share 0)
        let n0 = r.net;
        p.set(F::AiEdgeRatio, 10.0);
        assert!((eval(&p).net - n0).abs() < 1e-12);
        // more inference demand costs more
        p.set(F::InteractionsPerPersonDay, 30.0);
        assert!(eval(&p).net > n0, "more agent interactions cost more");
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
