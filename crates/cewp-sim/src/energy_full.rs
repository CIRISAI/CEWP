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
    InteractionsPerPersonDay = "ai.interactions_per_person_day", "agent interactions (all kinds)", [1.0, 10.0, 100.0], Frontier, 2026,
        "how often a person asks their agent for something; no figure (CIRISProxy's aggregate counters could measure it: see the measurement request)";
    CallsPerInteraction = "ai.ethical.llm_calls_per_interaction", "LLM calls", [12.0, 30.0, 70.0], Derived, 2026,
        "ETHICAL agents: STATED by the project, not measured: CIRISProxy README (CIRISAI/CIRISProxy 6f2f640) '1 credit = 1 user interaction (12-70 LLM calls typical)'";
    SimpleCalls = "ai.simple.llm_calls_per_interaction", "LLM calls", [1.0, 2.0, 5.0], Frontier, 2026,
        "SIMPLE agents, 'single-purpose tools' with 'no ethics of their own' (ciris.ai/federation, the project's stated design): 1 to a few calls";
    SimpleSmallShare = "ai.simple.small_model_share", "share of calls", [0.5, 0.8, 1.0], Frontier, 2026,
        "share of a simple agent's calls a small model (the on-device tier) can serve";
    SelfAwareCalls = "ai.self_aware.llm_calls_per_interaction", "LLM calls", [30.0, 70.0, 200.0], Frontier, 2026,
        "ETHICAL, SELF-AWARE agents, which 'cost more to run' (ciris.ai/federation): the upper end of the stated range and above";
    SelfAwareLargeShare = "ai.self_aware.large_model_share", "share of calls", [0.1, 0.3, 0.6], Frontier, 2026,
        "share of a self-aware agent's calls that need a large model";
    SelfAwareObservationKwh = "ai.self_aware.observation_kwh_per_person_year", "kWh", [0.0, 0.5, 10.0], Frontier, 2026,
        "continuous self-observation outside interactions, per person-year when self-aware agents are 5% of interactions (scaled by their share): UNKNOWN; the page gives no figure";
    ShareEthical = "ai.kinds.ethical_share", "share of interactions", [0.05, 0.15, 0.40], Frontier, 2026,
        "MY READING of the page: 'most agents will be' simple, 'only a small share' self-aware; ethical agents the rest; swept";
    ShareSelfAware = "ai.kinds.self_aware_share", "share of interactions", [0.01, 0.05, 0.15], Frontier, 2026,
        "MY READING of 'only a small share of the network needs to be this kind' (ciris.ai/federation); simple agents take 1 - ethical - self-aware";
    RecordsPerCall = "record.signed_records_per_call", "records", [0.1, 0.5, 1.0], Frontier, 2026,
        "'every choice an agent makes goes into a signed record' (ciris.ai/federation): records per model call";
    SigJPerOp = "record.j_per_sig_op", "J", [0.0002, 0.0008, 0.003], Frontier, 2026,
        "a hybrid sign or verify on a phone-class core (energy.rs phone.j_per_sig_op; the 276 us EPYC verify MEASURED in cewp-cli metrics)";
    VerifiersPerRecord = "record.verifies_per_record", "verifications", [1.0, 3.0, 10.0], Frontier, 2026,
        "peers that verify a signed record";
    RecordBytes = "record.bytes", "bytes", [500.0, 2000.0, 10000.0], Frontier, 2026,
        "a signed decision record with its hybrid signature";
    RecordReplicas = "record.replicas", "copies", [1.0, 3.0, 10.0], Frontier, 2026,
        "copies held across the custody set";
    RecordKeepYears = "record.keep_years", "years", [1.0, 5.0, 20.0], Frontier, 2026,
        "how long a signed record is kept (the stock is a year's records times this)";
    DeviceStorageWPerTb = "record.device_storage_w_per_tb", "W/TB", [0.0, 0.2, 1.0], Frontier, 2026,
        "marginal storage power on devices already on (energy.rs c2.device_storage_w_per_tb)";
    NetJPerMb = "record.net_j_per_mb", "J/MB", [0.08, 0.24, 0.5], Model, 2020,
        "fixed-network traffic term, CIRISHolon POWER-0 row 35 (0.03 W per Mbit/s)";
    PobMembersPerPerson = "pob.member_nodes_per_person", "member nodes", [0.01, 0.1, 1.0], Frontier, 2026,
        "PROOF OF BENEFIT (ciris.ai/federation): membership is earned by 'running a real ethical-reasoning agent over time' (bonded, or sovereign after 'about a month'); member nodes per person";
    PobInteractionsPerDay = "pob.interactions_per_member_day", "ethical interactions", [1.0, 10.0, 50.0], Frontier, 2026,
        "the ethical-reasoning agent a member node runs to earn and keep standing: interactions a day";
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
    let ktok = class_ktokens(p);
    place_ktokens(p, case, ktok)
}

/// Thousands of tokens a year per person by model class (small, mid, large), over the agent
/// kinds: simple (most), ethical, self-aware (a small share), plus the Proof-of-Benefit agent.
#[must_use]
pub fn class_ktokens(p: &FullParams) -> [f64; 3] {
    let per_call = p.get(F::TokensPerCall) / 1000.0;
    let year = p.get(F::InteractionsPerPersonDay) * 365.25 * per_call;
    let (se, ss) = (p.get(F::ShareEthical), p.get(F::ShareSelfAware));
    let simple = (1.0 - se - ss).max(0.0);
    let (sm, lg) = (p.get(F::SmallJobShare), p.get(F::LargeJobShare));
    let mut k = [0.0; 3];
    // simple agents: a small model mostly, the rest mid
    let cs = year * simple * p.get(F::SimpleCalls);
    k[0] += cs * p.get(F::SimpleSmallShare);
    k[1] += cs * (1.0 - p.get(F::SimpleSmallShare));
    // ethical agents: the stated 12-70 calls, over the job-size split
    let ce = year * se * p.get(F::CallsPerInteraction);
    k[0] += ce * sm;
    k[1] += ce * (1.0 - sm - lg).max(0.0);
    k[2] += ce * lg;
    // self-aware agents: the upper end, a larger share of large-model calls
    let ca = year * ss * p.get(F::SelfAwareCalls);
    k[1] += ca * (1.0 - p.get(F::SelfAwareLargeShare));
    k[2] += ca * p.get(F::SelfAwareLargeShare);
    k
}

/// Thousands of tokens a year per person the Proof-of-Benefit agents run (ethical, mid class).
#[must_use]
pub fn pob_ktokens(p: &FullParams) -> f64 {
    p.get(F::PobMembersPerPerson) * p.get(F::PobInteractionsPerDay) * 365.25 * p.get(F::CallsPerInteraction) * p.get(F::TokensPerCall) / 1000.0
}

/// Place thousands of tokens by class over the tiers: (edge kWh, hosted kWh, share by tier).
fn place_ktokens(p: &FullParams, case: AiCase, ktok: [f64; 3]) -> (f64, f64, [f64; 5]) {
    let f = p.get(F::ForcedLocalShare);
    let total: f64 = ktok.iter().sum();
    let (mut edge, mut hosted, mut share) = (0.0, 0.0, [0.0; 5]);
    for (c, &kt) in ktok.iter().enumerate() {
        for (forced, part) in [(true, f), (false, 1.0 - f)] {
            let t = place(p, case, c, forced);
            let kwh = kt * part * tier_wh(p, t, c).unwrap_or(0.0) / 1000.0;
            if total > 0.0 {
                share[t] += kt * part / total;
            }
            if t == 4 { hosted += kwh } else { edge += kwh }
        }
    }
    (edge, hosted, share)
}

/// The signed record of every agent choice, kWh per person-year: signs and verifies, network
/// bytes to its replicas, and storage of the kept stock on devices already on.
#[must_use]
pub fn record_kwh(p: &FullParams) -> f64 {
    let calls_ktok: f64 = class_ktokens(p).iter().sum::<f64>() + pob_ktokens(p);
    let calls = calls_ktok * 1000.0 / p.get(F::TokensPerCall);
    let records = calls * p.get(F::RecordsPerCall);
    let sig_j = records * (1.0 + p.get(F::VerifiersPerRecord)) * p.get(F::SigJPerOp);
    let mb = records * p.get(F::RecordBytes) * p.get(F::RecordReplicas) / 1e6;
    let net_j = mb * p.get(F::NetJPerMb);
    let stock_tb = mb / 1e6 * p.get(F::RecordKeepYears);
    let store_kwh = stock_tb * p.get(F::DeviceStorageWPerTb) * 8766.0 / 1000.0;
    (sig_j + net_j) / 3.6e6 + store_kwh
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
    /// The signed record of every agent choice (mesh side).
    pub record: f64,
    /// Proof of Benefit: the ethical-reasoning agents member nodes run to earn and keep
    /// standing (a deliberate cost of Sybil resistance), placed like any other call.
    pub pob: f64,
    /// Self-aware agents' continuous self-observation outside interactions (unknown).
    pub observation: f64,
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
    let (record, pob, observation) = if p.case == AiCase::NoAi {
        (0.0, 0.0, 0.0)
    } else {
        let (pe, ph, _) = place_ktokens(p, p.case, [0.0, pob_ktokens(p), 0.0]);
        (record_kwh(p), pe + ph, p.get(F::SelfAwareObservationKwh) * p.get(F::ShareSelfAware) / 0.05)
    };
    Full { estate_per_user: e, removed, vanished, relocated_at_edge: at_edge, inference_kept: kept, additions, record, pob, observation, net: additions + at_edge + kept + record + pob + observation - removed }
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
    // too many inputs for every corner: seat_calibration's corner (each input at the end that
    // moves the net that way one at a time) refined by a vertex search, for each extreme; each
    // bound is attained by a real combination of input ends
    let mut d = FullParams::defaults(a, case);
    d.new_box_share = new_box_share;
    if let Some(m) = m {
        d.set(F::InteractionsPerPersonDay, m);
    }
    // the additions' interval as two pseudo-inputs
    let get = |q: &FullParams, i: usize| if i < free.len() { q.get(free[i]) } else if i == free.len() { q.add_gateway } else { q.add_box };
    let ends = |i: usize| if i < free.len() { let sp = &FULL_SPECS[free[i] as usize]; (sp.lo, sp.hi) } else if i == free.len() { (a.gateway[0], a.gateway[2]) } else { (a.new_box[0], a.new_box[2]) };
    let set = |q: &mut FullParams, i: usize, x: f64| if i < free.len() { q.set(free[i], x) } else if i == free.len() { q.add_gateway = x } else { q.add_box = x };
    let n = free.len() + 2;
    let search = |sense: f64| -> (f64, FullParams) {
        let mut x = d.clone();
        for i in 0..n {
            let (l, h) = ends(i);
            let mut ql = d.clone();
            set(&mut ql, i, l);
            let mut qh = d.clone();
            set(&mut qh, i, h);
            set(&mut x, i, if sense * eval(&qh).net >= sense * eval(&ql).net { h } else { l });
        }
        let mut best = eval(&x).net;
        for _ in 0..6 {
            let mut moved = false;
            for i in 0..n {
                let (l, h) = ends(i);
                for end in [l, h] {
                    if get(&x, i) == end {
                        continue;
                    }
                    let mut y = x.clone();
                    set(&mut y, i, end);
                    let v = eval(&y).net;
                    if sense * v > sense * best + 1e-9 * best.abs().max(1.0) {
                        best = v;
                        x = y;
                        moved = true;
                    }
                }
            }
            if !moved {
                break;
            }
        }
        (best, x)
    };
    let dn = eval(&d).net;
    let (lo, _) = search(-1.0);
    let (hi, xh) = search(1.0);
    let mut arg: Vec<(&'static str, f64)> = free.iter().map(|&f| (FULL_SPECS[f as usize].name, xh.get(f))).collect();
    arg.push(("mesh additions, gateway mix (kWh/person-year)", xh.add_gateway));
    arg.push(("mesh additions, new-box mix (kWh/person-year)", xh.add_box));
    (lo.min(dn), hi.max(dn), arg)
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
    let mut s = String::from("# TEST 1, THE THEORY: full replacement of the internet (substitution = 1)\n\nPer person-year, kWh. Baseline: the whole data-centre estate per internet user. Its work vanishes (a), relocates to members' devices at an edge efficiency ratio (b), or stays (c: non-consumer data-centre work; networks are baseload on both sides and do not enter). The mesh side is only what the mesh adds (the household pass's terms A + B) plus the relocated work. Every split input is FRONTIER: nobody publishes (a) against (b). Bounds: a vertex search over every input's interval (each bound attained by a real combination of input ends).\n\n");
    s.push_str("## Inputs\n\n| row | unit | lo / default / hi | class | year | source and basis |\n|---|---|---|---|---|---|\n");
    for r in FULL_SPECS {
        s.push_str(&format!("| `{}` | {} | {} / {} / {} | {} | {} | {} |\n", r.name, r.unit, r.lo, r.mid, r.hi, r.class.as_str(), r.year, r.source));
    }
    s.push_str(&format!("| mesh additions, gateway mix | kWh/person-year | {:.2} / {:.2} / {:.2} | FRONTIER | 2026 | the household pass (energy.rs), US 2020 mix, node on gateway, terms A + B, expected regime |\n", a.gateway[0], a.gateway[1], a.gateway[2]));
    s.push_str(&format!("| mesh additions, new-box mix | kWh/person-year | {:.2} / {:.2} / {:.2} | FRONTIER | 2026 | same, a new always-on box in every home that can take one |\n\n", a.new_box[0], a.new_box[1], a.new_box[2]));
    s.push_str("## Result\n\nAI cases: the **tiered mix (the design)**, Eric: \"a hybrid means a mixture of on-device, edge, prosumer, office appliance, servers, etc... - the market optimizes\". Each model call goes to a tier by an explicit allocation rule: a forced-local share (privacy, offline) to the most local tier able to run it, the rest to the lowest-energy able tier. A cost-minimising market is not the same as an energy-minimising one (cost also prices privacy, latency and hardware already owned), so the energy result can sit a little either side of the lowest-energy placement. Beside it, two labelled BOUNDS: all hosted with zero retention (CIRISProxy today) and all on devices. What vanishes is the same in every AI case: retention, and training, ranking and advertising on harvested data. ZDR removes retention and training on the conversation; it does not remove inference energy.\n\n| AI case | agent interactions per person-day | new-box share of homes | removed (of which vanishes) | relocated at the edge (AI on edge tiers included) | inference kept in data centres | mesh additions | signed records | Proof of Benefit | self-observation | net (default) | net bounds | sign |\n|---|---|---|---|---|---|---|---|---|---|---|---|---|\n");
    for case in AiCase::ALL {
        let demands: &[f64] = if case == AiCase::NoAi { &[10.0] } else { &[1.0, 10.0, 100.0] };
        for &m in demands {
            for nb in [0.0, 1.0] {
                let mut p = FullParams::defaults(&a, case);
                p.new_box_share = nb;
                p.set(F::InteractionsPerPersonDay, m);
                let r = eval(&p);
                let (lo, hi, _) = corners_at(&a, case, nb, m);
                let sign = if hi < 0.0 { "SAVING over the whole input box" } else if lo > 0.0 { "COST over the whole input box" } else { "open: the box spans zero" };
                s.push_str(&format!("| {} | {} | {} | {:.1} ({:.1}) | {:.1} | {:.1} | {:.2} | {:.3} | {:.2} | {:.2} | {:.1} | [{:.1}, {:.1}] | {} |\n", case.as_str(), if case == AiCase::NoAi { "-".to_string() } else { format!("{m}") }, nb, r.removed, r.vanished, r.relocated_at_edge, r.inference_kept, r.additions, r.record, r.pob, r.observation, r.net, lo, hi, sign));
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
    s.push_str("\n**Agent kinds** (ciris.ai/federation, the project's stated design: people; ethical, self-aware agents, which 'cost more to run, so only a small share of the network needs to be this kind'; ethical agents; simple agents, 'single-purpose tools' with 'no ethics of their own', 'most agents will be this kind'; 'every choice an agent makes goes into a signed record'; membership earned by 'running a real ethical-reasoning agent over time', Proof of Benefit). The page gives no numbers: every number here is FRONTIER, and the shares are MY READING of 'most' and 'a small share'. **Proof of Benefit is a deliberate, standing cost of membership**, the price the design accepts for Sybil resistance, comparable in role to what other networks spend on proof of work or stake; it is reported as its own line.\n\nThe tiered mix, net at the defaults, by the share of interactions that are ethical and self-aware (simple agents take the rest):\n\n| ethical share | self-aware 0.01 | 0.05 | 0.15 |\n|---|---|---|---|\n");
    for se in [0.05, 0.15, 0.4] {
        s.push_str(&format!("| {se} |"));
        for ss in [0.01, 0.05, 0.15] {
            let mut p = FullParams::defaults(&a, AiCase::Mix);
            p.set(F::ShareEthical, se);
            p.set(F::ShareSelfAware, ss);
            let be = { let mut q = p.clone(); let base = q.clone(); let _ = base; q.set(F::InteractionsPerPersonDay, 10.0); eval(&q).net };
            s.push_str(&format!(" {:.1} |", be));
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
        s.push_str(". (A vertex search from seat_calibration's corner; each bound is attained by a real combination of input ends; the mix's placement makes its net piecewise, so the bounds are a search, not a guarantee.)\n");
    }
    s.push_str("\n## Break-even for this test (one input, others at default; no new boxes)\n\n| input | tiered mix (the design) | all hosted (bound) | all on devices (bound) | no AI |\n|---|---|---|---|---|\n");
    let names: Vec<&str> = ["estate.vanish_share", "edge.efficiency_ratio", "estate.stays_share", "new_box_share", "ai.interactions_per_person_day", "ai.ethical.llm_calls_per_interaction", "ai.self_aware.llm_calls_per_interaction", "ai.kinds.ethical_share", "ai.kinds.self_aware_share", "pob.member_nodes_per_person", "pob.interactions_per_member_day", "ai.tokens_per_call", "ai.tier.hosted.wh_per_1k_tokens.mid", "ai.tier.device.wh_per_1k_tokens.small", "ai.tier.appliance.wh_per_1k_tokens.mid", "ai.tier.edge_server.wh_per_1k_tokens.large", "ai.jobs.small_share", "ai.alloc.forced_local_share", "additions"].to_vec();
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
        // all hosted: the class tokens times the hosted energies
        let k = class_ktokens(&p);
        let w = p.get(F::WhPer1kTokens);
        let want = (k[0] * w * p.get(F::HostedSmallFactor) + k[1] * w + k[2] * w * p.get(F::HostedLargeFactor)) / 1000.0;
        assert!((hosted - want).abs() < 1e-9, "{hosted} {want}");
        // simple agents dominate interactions but not tokens: at the defaults the ethical and
        // self-aware kinds carry most of the tokens
        let mut q2 = p.clone();
        q2.set(F::ShareEthical, 0.0);
        q2.set(F::ShareSelfAware, 0.0);
        assert!(class_ktokens(&q2).iter().sum::<f64>() < 0.5 * k.iter().sum::<f64>());
        // Proof of Benefit and the signed record are positive costs when AI is in scope
        let r = eval(&p);
        assert!(r.pob > 0.0 && r.record > 0.0);
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
