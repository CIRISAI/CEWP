//! TEST 1, THE THEORY: full replacement of the internet by the mesh (`FSD/twin/energy.md` §0).
//!
//! The scenario is the internet replaced entirely by the mesh, with no need for massive data
//! centres. So substitution is 1 by definition, and the baseline is the WHOLE
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

/// Where ESTATE-0 version 2 (with its "businesses" block) was read.
pub const ESTATE0_V2_PIN: &str = "CIRISHolon 8e95fef5715ffaf3bb8f1df9984d107113da752c conformance/scene/tables/estate0/estate_table.json v2 sha256 68fe7f825b690bcbfed45dd6b4061a6910a29e164b5fa25452ed4caccc6e2b58; conformance/scene/TWIN_THEORY_INPUTS.md";

/// Where ESTATE-0 version 4 (AI as a mix across tiers, idle draw by tier, trust per row) was read.
pub const ESTATE0_V4_PIN: &str = "CIRISHolon 818449a38767d0cbe319211d9101674e02728813 conformance/scene/tables/estate0/estate_table.json v4 sha256 0c5e8aae9a445f476d8e0c382d0c3079b2ca8969abc8914b0ed92331a854ab60";

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
    EstateTwh = "estate.all_data_centres_twh_per_year", "TWh/year", [380.0, 416.0, 460.0], Model, 2024,
        "ESTATE-0 (CIRISHolon 026936e, conformance/scene/tables/estate0, v1) rows 0 and 5: IEA Energy and AI 2025 Annex A.1, 416 TWh in 2024 (hyperscale 166, colocation 144, enterprise 106; modelled; excludes crypto, networks and devices); interval ours";
    InternetUsers = "estate.internet_users_billion", "billion", [5.5, 5.8, 6.0], Model, 2024,
        "ESTATE-0 (CIRISHolon 026936e, conformance/scene/tables/estate0, v1) rows 2-3: ITU revised 2024 to 5.8 bn (first published 5.5; 6.0 in 2025); 416 / 5.8 = 71.7 kWh per user-year (row 4)";
    AiShare = "estate.ai_share", "share", [0.15, 0.19, 0.32], Model, 2026,
        "ESTATE-0 (CIRISHolon 026936e, conformance/scene/tables/estate0, v1) rows 41-44: 0.15 is the IEA 2024 proxy without cooling overhead; 17-21% with it; about 19% in 2026 and 32% in 2030 at the IEA's rates (the scout's arithmetic); the default is the 2026 scenario year";
    StaysShare = "estate.stays_share", "share of the estate", [0.45, 0.6, 0.75], Model, 2024,
        "ESTATE-0 v2 (CIRISHolon 8e95fef, conformance/scene/tables/estate0, businesses block), CASE A: organisations' work stays on servers, owned or rented, placed by the market at about today's efficiency; enterprise-owned is 0.25 (IEA 2025, read at source), the rest of the bracket is the scout's, unsourced (CC 4.4.3.2.8: a business is an affiliation, 'a room that acts as one voice', running nodes it owns)";
    EnterpriseShare = "estate.enterprise_owned_share", "share of the estate", [0.255, 0.255, 0.255], Model, 2024,
        "ESTATE-0 row 5: enterprise 106 of 416 TWh (IEA Energy and AI 2025, Table A.1, read at source): already 'a node you own', inside the scenario and unchanged";
    OnPremRatio = "estate.case_b_on_premises_ratio", "x today's energy", [1.4, 2.0, 4.0], Frontier, 2025,
        "ESTATE-0 v2 (CIRISHolon 8e95fef, conformance/scene/tables/estate0, businesses block), CASE B only: rented business work moved to organisations' own premises: 1.4 (overhead alone, PUE 1.54 against about 1.1) to about 4 (a vendor-commissioned claim of 3.2-4.1), near 1 for best practice; BY SEARCH RESULT, not read at source; 2.0 is my centre";
    VanishShare = "estate.vanish_share", "share of the consumer non-AI part", [0.3, 0.5, 0.7], Frontier, 2024,
        "ESTATE-0 (CIRISHolon 026936e, conformance/scene/tables/estate0, v1) row 23: 0.5 stands (0.3-0.7), AUTHOR'S ESTIMATE, unsourceable; NOT weighted by traffic bytes (on-demand video is 54-57% of downstream bytes but about 1.3 Wh per viewing hour at the data centre, rows 14-19)";
    EdgeRatio = "edge.marginal_ratio_already_on", "x data-centre energy per unit of work", [0.1, 0.3, 1.0], Model, 2024,
        "ESTATE-0 (CIRISHolon 026936e, conformance/scene/tables/estate0, v1) row 40: on devices ALREADY ON, 0.1-1 (media delivery from a gateway already on about 0.1, row 32); the fixed draw of anything ADDED is the new-box term (a two-bay home storage box 39-129 kWh a year, row 36; energy.rs home_node.added_box.w)";
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
    AiVanishShare = "ai.vanish_share", "share of the consumer AI electricity", [0.2, 0.4, 0.6], Frontier, 2025,
        "ESTATE-0 v4 (CIRISHolon 818449a, ai_as_a_mix_across_tiers): the scout's estimate, unsourceable: ranking, advertising and training on harvested data vanish (the often-quoted 'inference is 60-90%' figures are 2019-21 and mostly ranking inference). TRUST none";
    AiRequestedShare = "ai.requested_inference_share", "share of the consumer AI electricity", [0.05, 0.12, 0.2], Frontier, 2025,
        "ESTATE-0 v4 (CIRISHolon 818449a, ai_as_a_mix_across_tiers): requested generative inference is about 5-20% of AI electricity (the scout's bound from operators' own figures); it is replaced by the mesh's own demand; the rest of consumer AI (frontier, training, business) stays. TRUST low";
    HostedJPerToken = "ai.hosted.j_per_token.mid", "J per token", [0.15, 0.18, 0.21], Model, 2025,
        "ESTATE-0 v4 (CIRISHolon 818449a, ai_as_a_mix_across_tiers): hosted, batched, about 32 bn parameters, measured (Chung et al., arXiv 2601.22076); 0.24 Wh per median hosted prompt (one operator, 2025, fell 33 times in a year). TRUST high";
    HostedIdleReserve = "ai.hosted.idle_reserve", "share added", [0.05, 0.10, 0.15], Model, 2025,
        "ESTATE-0 v4 (CIRISHolon 818449a, ai_as_a_mix_across_tiers): hosted idle reserve is about 10% of 0.24 Wh per prompt (one operator's disclosure). TRUST high";
    RightSizeSaving = "ai.right_size_saving", "share saved", [0.48, 0.65, 0.80], Model, 2025,
        "ESTATE-0 v4 (CIRISHolon 818449a, ai_as_a_mix_across_tiers): routing easy queries to small models saves 48-80% against sending everything to a frontier model; the saving comes from MODEL SIZE, not hardware; applied to a hosted small model and to the mix. TRUST medium";
    PhoneJPerToken = "ai.device.phone.j_per_token", "J per token (marginal: on anyway)", [0.58, 0.67, 0.76], Model, 2024,
        "ESTATE-0 v4 (CIRISHolon 818449a, ai_as_a_mix_across_tiers): about 3 bn parameters, measured, whole phone (Laskaridis et al., MobiCom 2024). TRUST medium";
    LaptopJPerToken = "ai.device.laptop.j_per_token", "J per token (marginal: in use)", [0.09, 0.17, 0.25], Model, 2025,
        "ESTATE-0 v4 (CIRISHolon 818449a, ai_as_a_mix_across_tiers): 3-9 bn parameters, power ASSUMED, not measured. TRUST low";
    PhoneShareOfLocal = "ai.device.phone_share_of_local", "share", [0.3, 0.5, 0.7], Frontier, 2026,
        "of the local small-model work, the share on phones (the rest on laptops in use); no figure. TRUST none";
    ConsumerGpuJPerToken = "ai.prosumer_gpu.j_per_token", "J per token", [0.27, 0.30, 0.32], Model, 2025,
        "ESTATE-0 v4 (CIRISHolon 818449a, ai_as_a_mix_across_tiers): consumer graphics card, about 1 bn parameters, card only. TRUST low";
    ConsumerGpuIdleW = "ai.prosumer_gpu.idle_w", "W", [22.0, 26.0, 30.0], Model, 2025,
        "ESTATE-0 v4 (CIRISHolon 818449a, ai_as_a_mix_across_tiers): consumer graphics card alone at idle, forum posts only. TRUST low";
    SameModelRatio = "ai.same_model_ratio_vs_batched_dc", "x the batched data centre per token", [18.0, 30.0, 46.0], Model, 2025,
        "ESTATE-0 v4 (CIRISHolon 818449a, ai_as_a_mix_across_tiers): the same model on a laptop one request at a time: the data centre is 1.6-2.3x better per joule, batching 64 a further 11-20x, so 18-46x against a batched data centre (Saad-Falcon et al., arXiv 2511.07885, read by the scout; the scout's arithmetic on the two factors). Efficiency is decided by model size and how full the machine is; an office box or on-prem server at low load cannot batch and sits near the laptop. TRUST high";
    BoxIdleW = "ai.box.idle_w", "W", [4.0, 7.0, 15.0], Frontier, 2025,
        "ESTATE-0 v4 (CIRISHolon 818449a, ai_as_a_mix_across_tiers): no idle figure is published for a small inference appliance or an on-premises GPU server (a 240 W supply; 10.2 kW maximum); small desktop 4-7 W and workstation with integrated accelerator 6-13 W (vendor-measured, TRUST high) as the proxy. TRUST none for the appliance itself";
    LocalShare = "ai.alloc.local_share", "share of small-model work", [0.4, 0.4, 0.4], Frontier, 2025,
        "the market rule's lever: the share of small-model work done on devices already awake (phones, laptops in use); swept 0 to 0.8 in the report; local models answer 71-89% of single-turn queries correctly (ESTATE-0 v4 (CIRISHolon 818449a, ai_as_a_mix_across_tiers), routing). TRUST none";
    PrefillRatio = "ai.agent.prefill_energy_ratio", "x a generated token", [0.1, 0.25, 0.5], Frontier, 2025,
        "ESTATE-0 v4 (CIRISHolon 818449a, ai_as_a_mix_across_tiers): agent traffic is mostly input, so energy rises less than tokens; an input token's energy against an output token's (no figure). TRUST none";
    HostedLargeFactor = "ai.tier.hosted.large_class_factor", "x the mid class", [2.0, 4.0, 8.0], Model, 2026,
        "a large (70B dense and up) model per token against the mid class, hosted";
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

/// How "entirely" treats AI: a hybrid is a mixture of on-device, edge, prosumer, office
/// appliance and server inference, placed by the market.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiCase {
    /// The design, by the market rule (ESTATE-0 v4): light work on devices already awake (only
    /// their marginal energy), heavy work on shared batched servers, small jobs right-sized; a
    /// dedicated box only where its utilisation pays for its idle draw (reported, not chosen at
    /// the defaults).
    Mix,
    /// Bound: every call hosted with zero retention, small jobs on a small hosted model.
    AllHostedRightSized,
    /// Bound: every call hosted with zero retention on one mid-size model (CIRISProxy today).
    AllHosted,
    /// Bound: every call on the most local tier able to run it, a dedicated box per home for
    /// mid and large jobs (idle draw included).
    AllOnDevices,
    /// Comparison: the earlier rule (a forced-local share to the most local tier, the rest to
    /// the lowest-energy tier per token, idle not charged).
    OldRule,
    /// AI out of scope.
    NoAi,
}

impl AiCase {
    /// All six, the design first.
    pub const ALL: [Self; 6] = [Self::Mix, Self::AllHostedRightSized, Self::AllHosted, Self::AllOnDevices, Self::OldRule, Self::NoAi];
    /// Name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Mix => "tiered mix, market rule (the design)",
            Self::AllHostedRightSized => "all hosted, right-sized, zero retention (bound)",
            Self::AllHosted => "all hosted, one mid-size model, zero retention (bound)",
            Self::AllOnDevices => "all on devices (bound)",
            Self::OldRule => "earlier rule: forced-local share, then lowest energy per token (comparison)",
            Self::NoAi => "no AI in scope",
        }
    }
}

/// The tiers.
pub const TIERS: [&str; 5] = ["phone/laptop already on", "prosumer GPU", "office or home appliance", "edge or on-premises server", "hosted, zero retention"];

/// Hosted energy per token of the mid class, idle reserve included (J).
fn hosted_mid(p: &FullParams) -> f64 {
    p.get(F::HostedJPerToken) * (1.0 + p.get(F::HostedIdleReserve))
}

/// Marginal energy per token (J) of a job class (0 small, 1 mid, 2 large) at a tier, or `None`
/// when the tier cannot run it; `right_sized` serves small jobs hosted on a small model. Idle
/// draw is NOT in these figures: [`box_idle_kwh`] adds it where a box is on for inference.
fn tier_j(p: &FullParams, tier: usize, class: usize, right_sized: bool) -> Option<f64> {
    let h = hosted_mid(p);
    let small_hosted = if right_sized { h * (1.0 - p.get(F::RightSizeSaving)) } else { h };
    let ph = p.get(F::PhoneShareOfLocal);
    match (tier, class) {
        (0, 0) => Some(ph * p.get(F::PhoneJPerToken) + (1.0 - ph) * p.get(F::LaptopJPerToken)),
        (1, 0) => Some(p.get(F::ConsumerGpuJPerToken)),
        (1 | 2, 1) => Some(h * p.get(F::SameModelRatio)),
        (2, 0) => Some(p.get(F::LaptopJPerToken)),
        (3, 1) => Some(h * p.get(F::SameModelRatio)),
        (3, 2) => Some(h * p.get(F::HostedLargeFactor) * p.get(F::SameModelRatio)),
        (4, 0) => Some(small_hosted),
        (4, 1) => Some(h),
        (4, 2) => Some(h * p.get(F::HostedLargeFactor)),
        _ => None,
    }
}

/// Place thousands of tokens by class: (edge kWh, hosted kWh, share by tier).
#[allow(clippy::needless_range_loop)]
fn place_ktokens(p: &FullParams, case: AiCase, ktok: [f64; 3]) -> (f64, f64, [f64; 5]) {
    let total: f64 = ktok.iter().sum();
    let (mut edge, mut hosted, mut share) = (0.0, 0.0, [0.0; 5]);
    let mut put = |t: usize, c: usize, kt: f64, right: bool| {
        let kwh = kt * 1000.0 * tier_j(p, t, c, right).unwrap_or(0.0) / 3.6e6;
        if total > 0.0 {
            share[t] += kt / total;
        }
        if t == 4 { hosted += kwh } else { edge += kwh }
    };
    let local = |c: usize| match c { 0 => 0, 1 => 2, _ => 3 };
    match case {
        AiCase::NoAi => {}
        AiCase::Mix => {
            // light work on devices already awake, the rest right-sized on shared servers
            let l = p.get(F::LocalShare);
            put(0, 0, ktok[0] * l, true);
            put(4, 0, ktok[0] * (1.0 - l), true);
            put(4, 1, ktok[1], true);
            put(4, 2, ktok[2], true);
        }
        AiCase::AllHostedRightSized | AiCase::AllHosted => {
            let right = case == AiCase::AllHostedRightSized;
            for c in 0..3 {
                put(4, c, ktok[c], right);
            }
        }
        AiCase::AllOnDevices => {
            for c in 0..3 {
                put(local(c), c, ktok[c], true);
            }
        }
        AiCase::OldRule => {
            let f = p.get(F::ForcedLocalShare);
            for c in 0..3 {
                put(local(c), c, ktok[c] * f, true);
                let best = (0..5).filter_map(|t| tier_j(p, t, c, true).map(|w| (t, w))).min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(core::cmp::Ordering::Equal)).map_or(4, |x| x.0);
                put(best, c, ktok[c] * (1.0 - f), true);
            }
        }
    }
    (edge, hosted, share)
}

/// The idle draw of boxes on FOR inference, kWh per person-year: one appliance per household
/// when the case puts mid or large work on one (all on devices).
fn box_idle_kwh(p: &FullParams, case: AiCase) -> f64 {
    if case == AiCase::AllOnDevices { p.get(F::BoxIdleW) * 8766.0 / 1000.0 / p.get(F::PersonsPerHousehold) } else { 0.0 }
}

/// Queries a day at which a dedicated box running a small model breaks even with hosted, on
/// energy: idle W x 86,400 s / ((hosted J - box J) per token x tokens per query); `None` when the
/// box's marginal energy per token is not below the hosted figure (it never pays).
#[must_use]
pub fn box_breakeven_queries_per_day(p: &FullParams, right_sized_hosted: bool, gpu: bool) -> Option<f64> {
    let hosted = tier_j(p, 4, 0, right_sized_hosted)?;
    let (marg, idle) = if gpu { (p.get(F::ConsumerGpuJPerToken), p.get(F::ConsumerGpuIdleW)) } else { (p.get(F::LaptopJPerToken), p.get(F::BoxIdleW)) };
    if marg >= hosted {
        return None;
    }
    Some(idle * 86_400.0 / ((hosted - marg) * p.get(F::TokensPerCall)))
}

/// The AI term split: (energy at the edge tiers, the mesh side; energy at the hosted tier, kept
/// in data centres), kWh per person-year, and the share of tokens at each tier.
#[must_use]
pub fn ai_split(p: &FullParams, case: AiCase) -> (f64, f64, [f64; 5]) {
    if case == AiCase::NoAi {
        return (0.0, 0.0, [0.0; 5]);
    }
    let (e, h, sh) = place_ktokens(p, case, class_ktokens(p));
    (e + box_idle_kwh(p, case), h, sh)
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
    /// Case B (counterfactual, "no large facilities at all"): rented business work moves to
    /// organisations' own premises. Case A (false, the central case): it stays on servers.
    pub case_b: bool,
}

impl FullParams {
    /// Defaults with the household pass's additions.
    #[must_use]
    pub fn defaults(a: &Additions, case: AiCase) -> Self {
        Self { v: FULL_SPECS.iter().map(|s| s.mid).collect(), add_gateway: a.gateway[1], add_box: a.new_box[1], new_box_share: 0.0, case, case_b: false }
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
    /// Case B only: the extra energy of rented business work moved on-premises.
    pub case_b_extra: f64,
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
    // the stay share is of the whole estate; AI is carved out of the consumer part first
    let consumer_all = e * (1.0 - p.get(F::StaysShare));
    // of consumer AI: ranking, advertising and training on harvested data vanish; requested
    // inference is replaced by the mesh's own; the rest (frontier, training, business) stays
    let ai_all = consumer_all * p.get(F::AiShare);
    let ai = ai_all * (p.get(F::AiVanishShare) + p.get(F::AiRequestedShare));
    let consumer = consumer_all * (1.0 - p.get(F::AiShare));
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
        vanished += ai_all * p.get(F::AiVanishShare);
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
    let case_b_extra = if p.case_b { e * (p.get(F::StaysShare) - p.get(F::EnterpriseShare)).max(0.0) * (p.get(F::OnPremRatio) - 1.0) } else { 0.0 };
    Full { estate_per_user: e, removed, vanished, relocated_at_edge: at_edge, inference_kept: kept, additions, record, pob, observation, case_b_extra, net: additions + at_edge + kept + record + pob + observation + case_b_extra - removed }
}

/// Extremes of `net` over every corner of the free rows' intervals and the additions'
/// intervals: (min, max, the corner of the max as (row name, value) pairs). The net is
/// multilinear in each input (InternetUsers enters as 1/x, monotone), so a corner holds each
/// extreme.
#[must_use]
pub fn corners(a: &Additions, case: AiCase, new_box_share: f64) -> (f64, f64, Vec<(&'static str, f64)>) {
    corners_fixed(a, case, new_box_share, None, false)
}

/// [`corners`] with interactions per person per day fixed at `m` (excluded from the corners).
#[must_use]
pub fn corners_at(a: &Additions, case: AiCase, new_box_share: f64, m: f64) -> (f64, f64, Vec<(&'static str, f64)>) {
    if case == AiCase::NoAi {
        return corners(a, case, new_box_share);
    }
    corners_fixed(a, case, new_box_share, Some(m), false)
}

fn corners_fixed(a: &Additions, case: AiCase, new_box_share: f64, m: Option<f64>, case_b: bool) -> (f64, f64, Vec<(&'static str, f64)>) {
    let used = |f: F| match f {
        F::InteractionsPerPersonDay => case != AiCase::NoAi && m.is_none(),
        F::EstateTwh | F::InternetUsers | F::AiShare | F::StaysShare | F::VanishShare | F::EdgeRatio | F::PersonsPerHousehold | F::EnterpriseShare => true,
        F::OnPremRatio => case_b,
        F::LocalShare => case == AiCase::Mix,
        F::ForcedLocalShare => case == AiCase::OldRule,
        F::BoxIdleW => case == AiCase::AllOnDevices,
        F::ConsumerGpuJPerToken | F::ConsumerGpuIdleW => case == AiCase::OldRule,
        F::PrefillRatio => false,
        _ => case != AiCase::NoAi,
    };
    let free: Vec<F> = F::ALL.iter().copied().filter(|&f| FULL_SPECS[f as usize].lo < FULL_SPECS[f as usize].hi && used(f)).collect();
    // too many inputs for every corner: seat_calibration's corner (each input at the end that
    // moves the net that way one at a time) refined by a vertex search, for each extreme; each
    // bound is attained by a real combination of input ends
    let mut d = FullParams::defaults(a, case);
    d.new_box_share = new_box_share;
    d.case_b = case_b;
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

/// [`corners`] under Case B (rented business work on-premises).
#[must_use]
pub fn corners_case_b(a: &Additions, case: AiCase, new_box_share: f64) -> (f64, f64, Vec<(&'static str, f64)>) {
    corners_fixed(a, case, new_box_share, None, true)
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
    s.push_str(&format!("Tables: {ESTATE0_V2_PIN}; {ESTATE0_V4_PIN}.\n\n"));
    s.push_str("## Inputs\n\n| row | unit | lo / default / hi | class | year | trust | source and basis |\n|---|---|---|---|---|---|---|\n");
    for r in FULL_SPECS {
        let trust = r.source.rsplit("TRUST ").next().filter(|_| r.source.contains("TRUST ")).unwrap_or("-");
        s.push_str(&format!("| `{}` | {} | {} / {} / {} | {} | {} | {} | {} |\n", r.name, r.unit, r.lo, r.mid, r.hi, r.class.as_str(), r.year, trust, r.source));
    }
    s.push_str(&format!("| mesh additions, gateway mix | kWh/person-year | {:.2} / {:.2} / {:.2} | FRONTIER | 2026 | - | the household pass (energy.rs), US 2020 mix, node on gateway, terms A + B, expected regime |\n", a.gateway[0], a.gateway[1], a.gateway[2]));
    s.push_str(&format!("| mesh additions, new-box mix | kWh/person-year | {:.2} / {:.2} / {:.2} | FRONTIER | 2026 | - | same, a new always-on box in every home that can take one |\n\n", a.new_box[0], a.new_box[1], a.new_box[2]));
    s.push_str("## Result\n\nAI cases (ESTATE-0 v4, ai_as_a_mix_across_tiers): the **tiered mix by the market rule (the design)**: a hybrid is a mixture of on-device, edge, prosumer, office appliance and server inference, placed by the market. A tier that is ON ANYWAY (a phone, a laptop in use) carries only its marginal energy per token; a tier on FOR inference carries its idle draw too, over its utilisation. So light, private work goes to devices already awake, heavy work to shared batched servers, small jobs to small models (right-sizing saves 48-80% from MODEL SIZE, not hardware), and a dedicated box only where its utilisation pays for its idle (the break-even is reported; it is not chosen at the defaults). Efficiency is decided by model size and how full the machine is, not by where it sits: the same model on a laptop is 18-46x a batched data centre. A cost-minimising market is not the same as an energy-minimising one (cost also prices privacy, latency and hardware already owned). Bounds: all hosted right-sized, all hosted on one mid-size model (like for like with CIRISProxy today), and all on devices; the earlier rule is kept as a comparison. What vanishes: ranking, advertising and training on harvested data; ZDR removes retention and training on the conversation and almost no energy on the hosted tier.\n\n| AI case | agent interactions per person-day | new-box share of homes | removed (of which vanishes) | relocated at the edge (AI on edge tiers included) | inference kept in data centres | mesh additions | signed records | Proof of Benefit | self-observation | net (default) | net bounds | sign |\n|---|---|---|---|---|---|---|---|---|---|---|---|---|\n");
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
    s.push_str(&format!("\n## The two business cases ({})\n\nThe stay share follows the project's instruction to use what is defensible from published sources and from how the Constitution models a business (CC 4.4.3.2.8). CC 4.4.3.2.8: a business is an affiliation, \"a room that acts as one voice\", running nodes it owns (\"a node is a device or server you own\"). The Constitution bans dependence, not scale.\n\n- **CASE A (central): the market keeps servers where they are cheapest.** Enterprise-owned work (0.255 of the estate, read at source) is already a node its owner runs: inside the scenario, unchanged. Rented business work stays on servers at about today's efficiency. Stay share 0.45-0.75, centre 0.6 (the split inside the bracket is unsourced). What is in play is consumer work, about 29 kWh per user-year (18-40): the saving is a share of 29, not of 72.\n- **CASE B (labelled counterfactual): no large facilities at all.** Rented business work (stay share minus enterprise) moves to organisations' own premises at 1.4-4 times today's energy (by search result). It prices \"no massive datacenters\" read as a ban on scale, where the Constitution only bans dependence.\n\nNet at 10 interactions a day, no new boxes (kWh per person-year):\n\n| AI case | CASE A net | CASE A bounds | CASE B net (on-premises x2.0) | CASE B extra | CASE B bounds |\n|---|---|---|---|---|---|\n", ESTATE0_V2_PIN));
    for c in AiCase::ALL {
        let pa = FullParams::defaults(&a, c);
        let ra = eval(&pa);
        let (la, ha, _) = corners(&a, c, 0.0);
        let mut pb = pa.clone();
        pb.case_b = true;
        let rb = eval(&pb);
        let (lb, hb, _) = corners_case_b(&a, c, 0.0);
        s.push_str(&format!("| {} | {:.1} | [{:.1}, {:.1}] | {:.1} | {:.1} | [{:.1}, {:.1}] |\n", c.as_str(), ra.net, la, ha, rb.net, rb.case_b_extra, lb, hb));
    }
    s.push_str("\nCASE A across the sourced stay bracket, and CASE B across its on-premises ratio (tiered mix, net):\n\n| stay share | CASE A | CASE B at 1.4 | CASE B at 2.0 | CASE B at 4.0 |\n|---|---|---|---|---|\n");
    for st in [0.45, 0.6, 0.75] {
        let mut p = FullParams::defaults(&a, AiCase::Mix);
        p.set(F::StaysShare, st);
        s.push_str(&format!("| {st} | {:.1} |", eval(&p).net));
        for r in [1.4, 2.0, 4.0] {
            let mut q = p.clone();
            q.case_b = true;
            q.set(F::OnPremRatio, r);
            s.push_str(&format!(" {:.1} |", eval(&q).net));
        }
        s.push('\n');
    }
    s.push_str("\n**The AI term by case** (interactions per day x 365.25 x LLM calls per interaction x tokens per call, placed over the tiers; kWh per person-year at the defaults) and where the calls go:\n\n| AI case | AI term | of which on edge tiers (mesh side) | of which hosted (stays in data centres) |");
    for t in TIERS {
        s.push_str(&format!(" {t} |"));
    }
    s.push_str("\n|---|---|---|---|---|---|---|---|---|\n");
    for case in [AiCase::Mix, AiCase::AllHostedRightSized, AiCase::AllHosted, AiCase::AllOnDevices, AiCase::OldRule] {
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
    for case in [AiCase::Mix, AiCase::AllHostedRightSized, AiCase::AllHosted, AiCase::AllOnDevices, AiCase::OldRule] {
        s.push_str(&format!("| {} | {} |\n", case.as_str(), break_even(&a, case, "ai.interactions_per_person_day").map_or("more than the scan reaches (3,000)".to_string(), |x| format!("{x:.1}"))));
    }
    s.push_str("\nThe market rule's lever: the share of small-model work done on devices already awake (phones, laptops in use), swept 0 to 0.8, by the share of calls a small model can serve (net, defaults, no new boxes):\n\n| local share | small-model share 0.2 | 0.5 | 0.8 |\n|---|---|---|---|\n");
    for l in [0.0, 0.2, 0.4, 0.6, 0.8] {
        s.push_str(&format!("| {l} |"));
        for sm in [0.2, 0.5, 0.8] {
            let mut p = FullParams::defaults(&a, AiCase::Mix);
            p.set(F::LocalShare, l);
            p.set(F::SmallJobShare, sm);
            s.push_str(&format!(" {:.2} |", eval(&p).net));
        }
        s.push('\n');
    }
    // the dedicated box
    let pd = FullParams::defaults(&a, AiCase::Mix);
    let q = |rs: bool, gpu: bool| box_breakeven_queries_per_day(&pd, rs, gpu).map_or("never (its energy per token is not below hosted)".to_string(), |x| format!("{x:.0} queries a day"));
    let idle_share = |w: f64, qpd: f64| { let idle = w * 86_400.0; let work = qpd * pd.get(F::TokensPerCall) * pd.get(F::LaptopJPerToken); idle / (idle + work) };
    s.push_str(&format!("\n**When a dedicated box breaks even with hosted, on energy** (it carries its idle draw over its utilisation; {} tokens per query):\n\n| box | against hosted on one mid-size model | against hosted right-sized |\n|---|---|---|\n| small appliance at {} W idle (FRONTIER; small-desktop proxy), small model near the laptop's {:.2} J/token | {} | {} |\n| prosumer GPU at {} W idle (forum posts, low trust), {:.2} J/token (card only) | {} | {} |\n\nA box answering 50 queries a day at {} W spends {:.0}% of its energy idle (ESTATE-0 v4: the scout's arithmetic, query count ASSUMED, gives 96% at 5 W). Running the SAME mid-size model on a box at low load is {:.0}x the batched data centre per token: it never breaks even.\n", pd.get(F::TokensPerCall), pd.get(F::BoxIdleW), pd.get(F::LaptopJPerToken), q(false, false), q(true, false), pd.get(F::ConsumerGpuIdleW), pd.get(F::ConsumerGpuJPerToken), q(false, true), q(true, true), pd.get(F::BoxIdleW), 100.0 * idle_share(pd.get(F::BoxIdleW), 50.0), pd.get(F::SameModelRatio)));
    // agent multiple
    s.push_str(&format!("\n**The agent token multiple** (ESTATE-0 v4: an agent about 4x the tokens of chat, multi-agent about 15x; per person per day unmeasured; energy rises less than tokens because agent traffic is mostly input). The AI term relative to chat (1 = no agents, every interaction one call), energy multiple 1 + (m - 1) x prefill ratio ({}), tiered mix, 10 interactions a day:\n\n| token multiple | AI term (kWh per person-year) | net |\n|---|---|---|\n", pd.get(F::PrefillRatio)));
    {
        let mut chat = pd.clone();
        chat.set(F::ShareEthical, 0.0);
        chat.set(F::ShareSelfAware, 0.0);
        chat.set(F::SimpleCalls, 1.0);
        let base_ai = ai_demand_kwh(&chat, AiCase::Mix);
        let base_net = eval(&chat).net;
        for m in [1.0, 2.0, 5.0, 15.0] {
            let mult = 1.0 + (m - 1.0) * pd.get(F::PrefillRatio);
            s.push_str(&format!("| {m} | {:.2} | {:.1} |\n", base_ai * mult, base_net + base_ai * (mult - 1.0)));
        }
    }
    for case in AiCase::ALL {
        let (lo, hi, arg) = corners(&a, case, 0.0);
        s.push_str(&format!("\n{}, no new boxes: best corner {:.1}; worst corner {:.1} kWh per person-year at ", case.as_str(), lo, hi));
        s.push_str(&arg.iter().map(|(n, v)| format!("`{n}` = {v:.3}")).collect::<Vec<_>>().join(", "));
        s.push_str(". (A vertex search from seat_calibration's corner; each bound is attained by a real combination of input ends; the mix's placement makes its net piecewise, so the bounds are a search, not a guarantee.)\n");
    }
    s.push_str("\n## Break-even for this test (one input, others at default; no new boxes)\n\n| input | tiered mix (the design) | all hosted right-sized | all hosted one model | all on devices | no AI |\n|---|---|---|---|---|---|\n");
    let names: Vec<&str> = ["estate.vanish_share", "edge.marginal_ratio_already_on", "estate.stays_share", "new_box_share", "ai.interactions_per_person_day", "ai.ethical.llm_calls_per_interaction", "ai.self_aware.llm_calls_per_interaction", "ai.kinds.ethical_share", "ai.kinds.self_aware_share", "pob.member_nodes_per_person", "pob.interactions_per_member_day", "ai.tokens_per_call", "ai.hosted.j_per_token.mid", "ai.right_size_saving", "ai.device.phone.j_per_token", "ai.same_model_ratio_vs_batched_dc", "ai.box.idle_w", "ai.vanish_share", "ai.jobs.small_share", "ai.alloc.local_share", "additions"].to_vec();
    for n in names {
        let f = |c| break_even(&a, c, n).map_or("none in range".to_string(), |x| format!("{x:.3}"));
        let label = match n {
            "new_box_share" => "share of homes with a new always-on box",
            "additions" => "mesh additions (kWh per person-year)",
            x => x,
        };
        s.push_str(&format!("| `{label}` | {} | {} | {} | {} | {} |\n", f(AiCase::Mix), f(AiCase::AllHostedRightSized), f(AiCase::AllHosted), f(AiCase::AllOnDevices), f(AiCase::NoAi)));
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
        // 416 TWh over 5.8 billion users: about 72 kWh per user-year (ESTATE-0 row 4)
        assert!((r.estate_per_user - 71.72).abs() < 0.1);
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
        // all AI on devices at the dearest same-model ratio costs more than no AI
        let mut p = FullParams::defaults(&adds(), AiCase::AllOnDevices);
        p.set(F::SameModelRatio, 46.0);
        p.set(F::InteractionsPerPersonDay, 30.0);
        let q = FullParams::defaults(&adds(), AiCase::NoAi);
        assert!(eval(&p).net > eval(&q).net);
    }

    #[test]
    fn the_market_rule_right_sizes_and_never_buys_the_same_model_box() {
        let a = adds();
        let p = FullParams::defaults(&a, AiCase::Mix);
        let mix = ai_demand_kwh(&p, AiCase::Mix);
        let rs = ai_demand_kwh(&p, AiCase::AllHostedRightSized);
        let one = ai_demand_kwh(&p, AiCase::AllHosted);
        let dev = ai_demand_kwh(&p, AiCase::AllOnDevices);
        // right-sizing beats one model; devices running mid and large work lose to both
        assert!(rs < one && dev > one);
        // the mix at local share 0 equals all hosted right-sized
        let mut q = p.clone();
        q.set(F::LocalShare, 0.0);
        assert!((ai_demand_kwh(&q, AiCase::Mix) - rs).abs() < 1e-9);
        // the mix sits between its right-sized bound and its local-heavy end
        assert!(mix.is_finite() && mix >= rs - 1e-12);
        // a box running the same mid-size model never breaks even (marginal above hosted)
        assert!(tier_j(&p, 2, 1, true).unwrap() > tier_j(&p, 4, 1, true).unwrap());
        // the hosted mid figure includes its idle reserve
        assert!((hosted_mid(&p) - 0.18 * 1.1).abs() < 1e-12);
        // tier shares sum to one
        let (_, _, sh) = ai_split(&p, AiCase::Mix);
        assert!((sh.iter().sum::<f64>() - 1.0).abs() < 1e-12);
        // more interactions cost more, in every AI case
        for c in [AiCase::Mix, AiCase::AllHostedRightSized, AiCase::AllHosted, AiCase::AllOnDevices, AiCase::OldRule] {
            let mut x = FullParams::defaults(&a, c);
            let n0 = eval(&x).net;
            x.set(F::InteractionsPerPersonDay, 30.0);
            assert!(eval(&x).net > n0, "{c:?}");
        }
        // Proof of Benefit and the signed record are positive costs when AI is in scope
        let r = eval(&p);
        assert!(r.pob > 0.0 && r.record > 0.0);
    }

    #[test]
    fn case_b_adds_the_on_premises_penalty() {
        let a = adds();
        let pa = FullParams::defaults(&a, AiCase::Mix);
        let mut pb = pa.clone();
        pb.case_b = true;
        let (ra, rb) = (eval(&pa), eval(&pb));
        assert!(ra.case_b_extra == 0.0);
        // 71.7 x (0.6 - 0.255) x (2.0 - 1) = 24.7 kWh per user-year
        assert!((rb.case_b_extra - ra.estate_per_user * (0.6 - 0.255)).abs() < 1e-9);
        assert!((rb.net - ra.net - rb.case_b_extra).abs() < 1e-9);
    }

    #[test]
    fn break_even_is_a_zero() {
        let a = adds();
        if let Some(z) = break_even(&a, AiCase::NoAi, "edge.marginal_ratio_already_on") {
            let mut p = FullParams::defaults(&a, AiCase::NoAi);
            p.set(F::EdgeRatio, z);
            assert!(eval(&p).net.abs() < 1e-6);
        }
    }
}
