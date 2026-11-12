//! The twin's couplings registry for CIRISHolon (`FSD/twin/couplings.md`, `docs/geo/couplings.json`;
//! CIRISHolon `engine/FSD_WORLD_MODEL.md` §10).
//!
//! Every quantity the twin reads from, or would want from, a generated scene is a registry NAME of
//! one of four types (population, relationship, attribute, rhythm) with a status:
//!
//! * `read_today` (part A): a twin model reads it from the generated scene today (the holon tree,
//!   the leaf layout and the focal world built on it, or the cohort grid of the scale engine). The
//!   consumer names the file and function. Where nothing reads a scene quantity it is listed with
//!   the consumer `decoration`, so a certificate is not issued for a picture.
//! * `invented_today` (part B): a package models it on invented (MODEL or FRONTIER) inputs that a
//!   certified scene should supply instead. Rows name the parameters they stand on.
//! * `wanted` (part C): nothing models it yet; the consumer is `future: ...`.
//!
//! Part B is checked against the code: every MODEL or FRONTIER parameter of every spec array that
//! describes the world appears in exactly one row's `params` (part A rows also carry the parameters
//! that generate them), and every other one is in [`NOT_WORLD`] with the reason it describes the
//! protocol, the deployment, the display or the evaluation rather than the world. A parameter added
//! to a spec array without a row fails `every_world_parameter_has_exactly_one_row`.

use std::fmt::Write as _;

/// A row's status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Part A: read by a twin model from the generated scene today.
    ReadToday,
    /// Part B: modelled today on invented inputs.
    InventedToday,
    /// Part C: wanted, not modelled.
    Wanted,
}

impl Status {
    /// As written in the registry.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ReadToday => "read_today",
            Self::InventedToday => "invented_today",
            Self::Wanted => "wanted",
        }
    }
    /// The part letter.
    #[must_use]
    pub const fn part(self) -> &'static str {
        match self {
            Self::ReadToday => "A",
            Self::InventedToday => "B",
            Self::Wanted => "C",
        }
    }
}

/// What a rhythm is indexed over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Base {
    /// A population per bin.
    Population,
    /// A relationship per bin.
    Relationship,
}

/// A row's type and its type-specific fields.
#[derive(Debug, Clone, Copy)]
pub enum Ty {
    /// A count of a kind inside a holon; children sum to the parent. `parents` lists every parent
    /// kind, so a multi-parent row is explicit.
    Population { parents: &'static [&'static str] },
    /// A count on a pair of objects, joining kind `from` to kind `to`.
    Relationship { from: &'static str, to: &'static str, directed: bool },
    /// A value on one object of kind `of`; `with` lists the rows a model reads it jointly with.
    Attribute { of: &'static str, with: &'static [&'static str] },
    /// A population or relationship per time bin; `by` names the cells of each bin.
    Rhythm { base: Base, bins: u16, by: &'static str },
}

impl Ty {
    /// The type name.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Population { .. } => "population",
            Self::Relationship { .. } => "relationship",
            Self::Attribute { .. } => "attribute",
            Self::Rhythm { .. } => "rhythm",
        }
    }
}

/// One registry row.
#[derive(Debug, Clone, Copy)]
pub struct Row {
    /// Registry name.
    pub name: &'static str,
    /// Status (and so its part).
    pub status: Status,
    /// Type with its fields.
    pub ty: Ty,
    /// Tier: cell, street tile, leaf, building, household, person, device, locality, region,
    /// network, access link, room, act.
    pub tier: &'static str,
    /// Unit of the count or value.
    pub unit: &'static str,
    /// The model that reads it (`file:function`), `decoration`, or `future: ...`.
    pub consumer: &'static str,
    /// The class of today's value (FSD §8.2), or `-` for a wanted row.
    pub class: &'static str,
    /// A first guess at a reference from the open list, or `none known`.
    pub reference: &'static str,
    /// Spec parameters standing behind today's value, as `<array>.<name>` (see [`spec_names`]).
    pub params: &'static [&'static str],
    /// One line of context.
    pub note: &'static str,
}

#[allow(clippy::too_many_arguments)]
const fn row(
    name: &'static str, status: Status, ty: Ty, tier: &'static str, unit: &'static str, consumer: &'static str, class: &'static str,
    reference: &'static str, params: &'static [&'static str], note: &'static str,
) -> Row {
    Row { name, status, ty, tier, unit, consumer, class, reference, params, note }
}
const fn pop(parents: &'static [&'static str]) -> Ty {
    Ty::Population { parents }
}
const fn rel(from: &'static str, to: &'static str, directed: bool) -> Ty {
    Ty::Relationship { from, to, directed }
}
const fn attr(of: &'static str, with: &'static [&'static str]) -> Ty {
    Ty::Attribute { of, with }
}
const fn rhythm(base: Base, bins: u16, by: &'static str) -> Ty {
    Ty::Rhythm { base, bins, by }
}

use Base::{Population as BP, Relationship as BR};
use Status::{InventedToday as B, ReadToday as A, Wanted as C};

/// Hour-of-week bins (CIRISHolon's convention, stated once): 168 bins, bin 0 = Monday 00:00-01:00
/// local zone time of the holon; where the twin knows only longitude, the zone is round(lon / 15)
/// hours; the working week is a class coordinate `week` = mon-fri | sun-thu | sat-wed keyed by
/// region.
pub const WEEK_BINS: u16 = 168;

const NONE: &str = "none known";
const GHS: &str = "GHS-POP R2023A (shipped: docs/geo/gd/pop0, pop1)";
const NAT_GAP: &str = "none known (NAT and IPv6 behaviour: no stated licence on either public source); fleet gauges in FSD/twin/nat.md section 5";

/// Every row: part A, then B, then C.
pub const ROWS: &[Row] = &[
    // ------------------------------------------------------------------ A: read today
    row("cell.people", A, pop(&["region holon (1.5 degree root)", "parent holon"]), "cell (every holon, root to street tile)", "persons",
        "crates/cewp-sim/src/popchain.rs:World::new, split (kernel-wasm pop_load/pop_split; docs/tiling.js tlRoot/tlSplit) -> tiling.rs LeafContext.people -> micro.rs:MicroWorld::populate",
        "DERIVED", GHS, &[], "the street tier apportions the cohort tier's GHS total (7,830,639,988) over the 355,434 floored 1/8-degree cells; exact at every split (docs/geo/popcond.test.js); inside a cell, people follow CIRISHolon's UNEVEN-0 through the in-cell seam (popchain::incell, unresolved)"),
    row("cell.households", A, pop(&["parent holon"]), "cell (every holon, root to street tile)", "households",
        "crates/cewp-sim/src/popchain.rs:World::new, split -> tiling.rs:HolonTree::budget -> tiling.rs leaf layout (dwelling units) -> micro.rs:MicroWorld::from_leaf, populate",
        "MODEL", NONE, &["micro.persons_per_household"], "round(people / 2.5) at the root, apportioned by the children's people below it; persons per household is a named gap"),
    row("cell.density", A, attr("street tile", &["cell.settlement_class"]), "street tile", "persons per km2",
        "crates/cewp-sim/src/micro.rs:DensityClass::of; tiling.rs:place_dense_fabric (storeys)", "DERIVED", GHS,
        &["micro.density_sigma"], "GHS-POP 1/8-degree mean; the focal world without a leaf (MicroWorld::build) spreads it lognormally by density_sigma"),
    row("cell.settlement_class", A, attr("street tile", &["cell.density"]), "street tile", "class (village | suburban | terraced | apartment blocks | dense fabric)",
        "crates/cewp-sim/src/micro.rs:DensityClass::of -> tiling.rs leaf layout (street grid, building kind, units per building)", "MODEL",
        "GHSL settlement classes (R2023A)", &["micro.suburban_density", "micro.urban_density", "micro.urban_core_density"],
        "thresholds in docs/geo/classes.json; the class decides dwellings per building, which the focal world turns into households"),
    row("cell.land_cover", A, attr("cell", &[]), "cell", "class (LC100 majority)", "decoration", "DERIVED", "Copernicus LC100 (shipped: majority class per 1/8 degree)", &[],
        "ground tint only (docs/tiling.js mosaic); no model reads it. In-cell weighting needs a fractional built-up layer (blocked: the shipped asset has one class per cell)"),
    row("cell.relief", A, attr("cell", &[]), "cell", "m", "decoration", "DERIVED", "ETOPO 2022, Natural Earth shaded relief (shipped)", &[], "shading only; no model reads elevation or line of sight"),
    row("locality.people", A, pop(&["region"]), "locality (cohort cell, 1.5 degree, at least 20,000 persons)", "persons",
        "docs/globe.js:buildWorld -> kernel-wasm/src/lib.rs:tick (nodes per locality, lat, lon, metro); crates/cewp-sim/src/social.rs:Geography::from_globe_js; netgraph.rs metros",
        "DERIVED", GHS, &[], "4,559 cells holding 7,830,639,988 persons (POP_GRID_B64); the scale engine samples nodes by these weights"),
    row("locality.region", A, rel("locality", "region", true), "locality", "membership (one region per locality)",
        "kernel-wasm/src/lib.rs:tick (region_of_metro: cross-region deliveries, region rooms); social.rs placement rules", "MODEL",
        "Natural Earth admin-0 (approved set; not used for this today)", &[], "the page's region rule; the twin has no country layer"),
    row("locality.local_hour", A, attr("locality", &["act.volume"]), "locality", "hours from UTC (solar, lon / 15)",
        "kernel-wasm/src/lib.rs:tick (globe::local_sec, diurnal_accept); micro.rs:MicroWorld::local_hour (start_local_hour from the page)", "DERIVED",
        "Natural Earth time zones (approved set; solar time from longitude today)", &["micro.start_local_hour"], "day and night by longitude is what spreads acts over the planet"),
    row("node_pair.distance_km", A, rel("node", "node", false), "locality", "km (great circle)",
        "kernel-wasm/src/lib.rs:tick (out slot 8 km summed; locality and region crossings)", "DERIVED", "derived from positions; no reference needed", &[], "between node positions drawn inside their locality"),
    row("node.room", A, rel("node", "room", true), "locality", "membership",
        "docs/globe.js:buildWorld -> kernel-wasm/src/lib.rs:tick (node_rooms: family, community, region and global rooms)", "MODEL", NONE,
        &["micro.family_share", "micro.community_share"], "the scale page draws every family in the signer's locality and community rooms 90% local; act mix 65/30/4/1"),
    row("leaf.streets", A, rel("building", "street", true), "street tile", "frontage (building on street)",
        "decoration", "MODEL", "STREETREF-1 via CIRISHolon (OpenStreetMap statistics)", &[],
        "a graph since item 6 f: tiling.rs:LeafLayout::street_graph (street id = leaf + road index, junctions cross | tee | corner | exit | dead end), Building.street; the tiled street drawing takes its end caps from it (docs/tiling.js). No model reads it. A small leaf whose ancestors' lines all pass outside it has no street (Cairo and Paris probe leaves)"),
    row("household.street", A, rel("household", "street", true), "household", "membership (the building's frontage street)",
        "decoration", "MODEL", "STREETREF-1 via CIRISHolon", &[],
        "micro.rs:MicroWorld::household_street, households_by_street; summed by street they are the leaf's households (popcond.test.js). No model reads it"),
    row("leaf.arterial_crossings", A, rel("street tile", "street tile", false), "street tile", "arterials crossing a side",
        "decoration", "MODEL", "STREETREF-1 via CIRISHolon", &[], "tiling.rs LeafContext.edges and l2_nb place arterials so neighbouring tiles join; no model reads them. Counted per side of every split from L2 down (tiling::arterial_crossings, kernel pop_crossings) and written to the sums file with side and peer: the fine crossings under a side sum to the coarse one and both sides agree"),
    row("building.dwellings", A, pop(&["building", "street tile"]), "building", "dwelling units (one household each)",
        "crates/cewp-sim/src/micro.rs:MicroWorld::populate (households in building order)", "MODEL", "GHSL built-up surface and volume; building footprints (Microsoft, CDLA)",
        &[], "units per building follow the class and the leaf's households (tiling.rs). Household.building is stored, not read by any model"),
    row("building.storeys", A, attr("building", &["building.dwellings"]), "building", "storeys", "decoration", "MODEL", "GHSL building height",
        &[], "glyph height and shadow length; tiling.rs:place_dense_fabric sets storeys from units; nothing reads them"),
    row("household.position", A, attr("household", &[]), "household", "m from the tile centre", "decoration", "MODEL", "building footprints (Microsoft, CDLA)",
        &[], "drawing; micro.rs:story_actors picks the Story's four households by distance (presentation only). No model reads neighbour distance or same-street"),
    row("household.persons", A, pop(&["household"]), "household", "persons",
        "crates/cewp-sim/src/micro.rs:MicroWorld::populate (phones and laptops per person; hosted-server chance 2.25 x persons x server_tier_share)", "MODEL",
        "none known (the UN household-size table forbids derivatives)", &["social.household_size_sd"],
        "by default CIRISHolon's HOUSEGEN-1 per L4 tile (popchain::households: sizes summing to the tile's people, whole households to the leaves; every call here unresolved until the twin can test countries); the legacy rule (drawn round(2.5 + 1.1 N) clamped 1..6, people apportioned over them) is the labelled fallback"),
    row("household.adopting", A, pop(&["street tile"]), "household", "households running CIRIS", "crates/cewp-sim/src/micro.rs:MicroWorld::populate", "MODEL",
        "none known (fleet count by locality would be the measure)", &["micro.adoption"], "adoption is a threshold on the household's own stream, so raising it adds households"),
    row("household.devices", A, pop(&["household", "person"]), "household", "devices by kind (phone, laptop, home node, hosted server)",
        "crates/cewp-sim/src/micro.rs:MicroWorld::populate (device_population, household_devices; R5_MAX_NODES samples it for display)", "MODEL",
        "World Bank indicators mirroring ITU (phones per 100, country grain)",
        &["micro.phones_per_person", "micro.laptops_per_person", "micro.home_node_household_share", "micro.server_tier_share"],
        "the leaf's device total is recorded as its device population; the 1,000-node cap is a display sample of it"),
    row("household.router_class", A, attr("household", &["household.access_network", "access_link.technology", "phone.attach"]), "household", "class (public | punchable | hard)",
        "crates/cewp-sim/src/micro.rs:MicroWorld::unpunched_direct, punch_possible, relay_offering (Node.public, Node.punchable)", "FRONTIER", NAT_GAP,
        &["micro.reach_home_share", "micro.public_listener_share"], "a phone off Wi-Fi takes the carrier's class instead (go_out_with)"),
    row("household.community", A, rel("household", "street community room", true), "household", "membership",
        "crates/cewp-sim/src/micro.rs:MicroWorld::community, pick_destination, room_peers (direct-path proposals go to room peers)", "MODEL", NONE,
        &["micro.community_join_share"], "half the street's adopting households in one room"),
    row("household.lan", A, rel("device", "device", false), "household", "same home LAN (pair)",
        "crates/cewp-sim/src/micro.rs:MicroWorld::unpunched_direct (on_home_lan: same household, phone on Wi-Fi)", "MODEL", NONE,
        &[], "the only neighbour relation any model reads; there is no cross-household radio or LAN relation"),
    row("phone.attach", A, rhythm(BR, WEEK_BINS, "phone x network kind (home Wi-Fi | cellular)"), "device", "phones on each network kind",
        "crates/cewp-sim/src/micro.rs:MicroWorld::churn, change_network, on_home_lan", "MODEL", "none known (ATUS time at home is the nearest)",
        &["micro.phone_wifi_share", "micro.wifi_dwell_mean_s"], "flat today: a stationary share and a mean dwell, no hour of week; becomes attach.phone.network"),
    row("device.awake", A, rhythm(BP, WEEK_BINS, "device kind"), "device", "devices awake",
        "crates/cewp-sim/src/micro.rs:Node::online, MicroWorld::churn, suspend, resume", "MODEL", NONE,
        &["micro.mobile_offline_share", "micro.phone_asleep_mean_s", "micro.laptop_awake_share", "micro.laptop_dwell_mean_s"], "flat today: stationary shares and mean dwells"),
    row("phone.charging", A, rhythm(BP, WEEK_BINS, "phone"), "device", "phones on external power",
        "crates/cewp-sim/src/micro.rs:Node::device_relays, relay_offering (a phone relays only charging and unmetered)", "MODEL", NONE,
        &["micro.phone_charging_share", "micro.charge_dwell_mean_s"], "flat today"),
    row("device.relay_preference", A, attr("device", &["household.router_class", "phone.charging"]), "device", "share offering relay",
        "crates/cewp-sim/src/micro.rs:Node::relay_offering", "MODEL", "none known (fleet gauge: announced relays per device class)", &["micro.relay_offering_share"], "owners who leave the relay default on"),
    row("device.shaping_opt_in", A, attr("device", &["household.devices"]), "device", "share opted in", "crates/cewp-sim/src/micro.rs:Node::budget_class, MicroWorld::negotiate",
        "MODEL", "none known (fleet gauge)", &["micro.home_shaping_opt_in", "micro.phone_shaping_opt_in"], "shaping is off by default on home nodes and phones"),
    row("device.claimed", A, attr("device", &["household.devices"]), "device", "share with an owner", "crates/cewp-sim/src/micro.rs:MicroWorld::assign_claims, seat_may_send", "MODEL",
        "none known (fleet gauge)", &["micro.claimed_share"], ""),
    row("device.old_version", A, attr("device", &["household.devices"]), "device", "share on an older edge", "crates/cewp-sim/src/micro.rs:MicroWorld::punch_possible", "MODEL",
        "none known (fleet version census)", &["micro.server_old_version_share"], "long-soak servers kept old on purpose"),
    row("act.volume", A, rhythm(BP, WEEK_BINS, "device kind"), "device", "acts per awake device per bin",
        "crates/cewp-sim/src/micro.rs:MicroWorld::generate (msg_mean_interval_s / diurnal_multiplier); kernel-wasm/src/lib.rs:tick (diurnal_accept); workload.rs:diurnal_multiplier",
        "MODEL", "none known: sending by hour has no reference in anything read", &["micro.msg_mean_interval_s", "micro.send_by_curve", "activity.acts_per_person_day", "activity.activity_sigma"],
        "two assumptions on the street where people have generated days: each awake device at a flat gap (default), or the day curve's total poured onto the awake; generator model.diurnal.v1 elsewhere; becomes activity.<kind>.events"),
    row("act.scope", A, rel("act", "room kind", true), "device", "acts per scope (family | community | consent-free | public)",
        "crates/cewp-sim/src/micro.rs:MicroWorld::pick_destination; kernel-wasm/src/lib.rs:tick (mix_self, mix_room, mix_fed)", "MODEL", NONE,
        &["activity.mix_family", "activity.mix_community", "activity.mix_consent_free", "activity.mix_public"], "the page's act mix 65/30/4/1"),
    row("seat.distance_km", A, attr("serve seat", &[]), "street tile", "km from the tile", "decoration", "MODEL", "none needed (a deployment choice)",
        &["micro.seat_km_min", "micro.seat_km_max"], "drawn on the chip; no micro model reads a latency from it (netgraph latency is metro to metro)"),
    row("delivery.late_bound", A, attr("held row", &["phone.attach"]), "row", "seconds a held row may wait before it counts as late",
        "crates/cewp-sim/src/micro.rs:MicroWorld::hold_readout", "MODEL", "none known (the longest wait the reference population reports a share for is 12 h)",
        &["micro.late_hold_s"], "a readout bound, no delivery decision reads it; the shares under 1, 4, 8 and 24 h are shown beside it"),
    // ------------------------------------------------------------------ B: invented today
    row("household.access_network", B, rel("household", "access network", true), "household", "membership (one fixed AS per household)",
        "crates/cewp-sim/src/netgraph.rs:attach_access -> micro.rs Household.access, households_by_access (attached in kernel-wasm micro_build and micro_build_leaf; summed both ways in tests, read by no model yet); failure.rs:DomainWorld::build", "MODEL",
        "none known (ISP market shares per metro)", &["netgraph.incumbent_weight", "netgraph.local_weight_top", "failure.world.fixed_isps_per_country", "failure.world.isp_zipf"],
        "the household's second parent: households summed by street tile and by access network agree for every seed (netgraph test, popcond.test.js). Micro's router class is still its own draw; reading the network's NAT behaviour is FSD/twin/nat.md section 8"),
    row("phone.carrier", B, rel("phone", "carrier", true), "device", "membership", "crates/cewp-sim/src/netgraph.rs:assign_micro; failure.rs:DomainWorld::build", "MODEL",
        "World Bank indicators mirroring ITU (subscriptions, country grain); no market shares", &["netgraph.mobile_weight_top", "failure.world.cell_carriers_per_country"],
        "a phone's third parent"),
    row("household.carrier_shared", B, rel("phone", "phone", false), "household", "pairs on one carrier", "crates/cewp-sim/src/reach.rs:World::generate", "MODEL", NONE,
        &["reach.family_same_carrier_share"], "family plans"),
    row("access_link.technology", B, attr("access network", &["household.access_network", "region.ipv4_scarcity"]), "access link", "class (cable | fibre | DSL | 4G | 5G)",
        "crates/cewp-sim/src/reach.rs:ReachModel::draw_tech, network; reach/topo.rs", "MODEL", "World Bank indicators mirroring ITU (fixed broadband per 100); M-Lab NDT for rates",
        &["reach.fixed_cable_share", "reach.fixed_fibre_share", "reach.cell_5g_share"], "drawn per ASN, so one network is one technology"),
    row("access_link.rate", B, attr("access link", &["access_link.technology", "household.devices", "access_link.cross_traffic"]), "access link", "bit/s up and down, buffer ms",
        "crates/cewp-sim/src/media.rs:AccessProfile (FIBER_HOME, CABLE_HOME_WIFI, DSL_HOME, PHONE_HOME_WIFI, PHONE_LTE) -> media_call2.rs scenarios", "MODEL",
        "M-Lab NDT (measured access rates; the open source needs a cloud account)", &[], "uplink read jointly with the household's device count and its other traffic"),
    row("access_link.loss", B, attr("access link", &["access_link.technology"]), "access link", "Gilbert-Elliott ppm (good/bad transitions and loss)",
        "crates/cewp-sim/src/media.rs:GE_WIRED, GE_WIFI, GE_LTE, CELL_LTE -> link.rs:Link::step, cell_step", "MODEL", "M-Lab NDT (loss, RTT)", &[], "Wi-Fi collisions, LTE handover bursts, LTE capacity steps"),
    row("access_link.cross_traffic", B, attr("household", &["access_link.rate", "household.devices"]), "household", "transfers per minute and bytes",
        "crates/cewp-sim/src/media.rs:BG_UP_HOME, BG_DOWN_HOME, BG_UP_PHONE, BG_DOWN_PHONE -> link.rs household transfer (AIMD)", "MODEL", NONE, &[],
        "household traffic competing for the uplink"),
    row("access_link.path", B, attr("access link", &["access_link.technology"]), "access link", "bytes (path MTU, IP and UDP header)", "crates/cewp-sim/src/link.rs:ip_fragments; media.rs", "MODEL",
        "none known", &["media.path_mtu", "media.ip_udp_header"], "IPv6 would raise the header to 48"),
    row("region.ipv4_scarcity", B, attr("region", &["access_link.technology"]), "region", "class (rich | scarce)", "crates/cewp-sim/src/reach.rs:ReachModel::draw_region, network", "MODEL",
        "none known (registry allocation history)", &["reach.region_v4_scarce_share"], "drawn once per page region"),
    row("access_network.cgn", B, attr("access network", &["access_link.technology", "region.ipv4_scarcity"]), "access network", "share behind CGN; mapping and filtering behaviour",
        "crates/cewp-sim/src/reach.rs:ReachModel::network, direct_path", "MODEL", NAT_GAP,
        &["reach.fixed_cgnat_rich", "reach.fixed_cgnat_scarce", "reach.cable_cgnat_mult", "reach.fibre_cgnat_mult", "reach.cgn_eim_share", "reach.cgn_filter_eif", "reach.cgn_filter_adf", "reach.cgn_hairpin_share"], ""),
    row("household.router_behaviour", B, attr("household router", &["access_network.cgn", "access_link.technology"]), "household", "filtering, hairpin, UPnP/PCP, own router, double NAT",
        "crates/cewp-sim/src/reach.rs:ReachModel::home, direct_path", "MODEL", NAT_GAP,
        &["reach.router_filter_eif", "reach.router_filter_adf", "reach.router_hairpin_share", "reach.router_upnp_share", "reach.own_router_share", "reach.double_nat_share"],
        "the joint reach model; micro's router class (part A) is its calibrated marginal"),
    row("household.reach_marginals", B, attr("household router", &["household.router_behaviour"]), "household", "shares (public, punchable, punch success)",
        "crates/cewp-sim/src/reach.rs:ReachModel calibration; failure.rs:DomainWorld::build", "FRONTIER", NAT_GAP,
        &["reach.reach_home_share", "reach.public_listener_share", "reach.punch_success_share", "micro.punch_success_share", "failure.world.reach_home_share", "failure.world.public_listener_share"],
        "the reach and failure packages restate micro's marginals; micro's are on household.router_class"),
    row("access_network.ipv6", B, attr("access network", &["access_link.technology", "household.router_behaviour"]), "access network", "share with IPv6; firewall open | stateful | blocking",
        "crates/cewp-sim/src/reach.rs:ReachModel::network, home, direct_path", "MODEL", NAT_GAP,
        &["reach.fixed_v6_cable", "reach.fixed_v6_fibre", "reach.fixed_v6_dsl", "reach.home_v6_enabled_share", "reach.fixed_v6_fw_open", "reach.fixed_v6_fw_blocked", "reach.cell_v6_4g", "reach.cell_v6_5g", "reach.cell_v6_fw_blocked"], ""),
    row("carrier.nat", B, attr("carrier", &["access_link.technology"]), "access network", "CGN share, filtering, hairpin, punchable share",
        "crates/cewp-sim/src/reach.rs:ReachModel::phone_on_carrier; micro.rs:MicroWorld::go_out_with", "FRONTIER", NAT_GAP,
        &["micro.cellular_punchable_share", "reach.cellular_punchable_share", "reach.cell_cgnat_share", "reach.cell_filter_eif", "reach.cell_filter_adf", "reach.cell_hairpin_share"], ""),
    row("phone.attach_marginal", B, attr("phone", &["phone.attach"]), "device", "share on Wi-Fi at an instant", "crates/cewp-sim/src/reach.rs:World::generate; failure.rs:DomainWorld::build", "MODEL",
        "none known", &["reach.phone_wifi_share", "failure.world.phone_wifi_share"], "restated by reach and failure; micro's own is on phone.attach"),
    row("as.population", B, pop(&["metro", "region"]), "network", "ASes by role (tier-1, content, transit, incumbent, mobile, local fixed)",
        "crates/cewp-sim/src/netgraph.rs:generate", "MODEL", "RouteViews (AS graph; relationships inferred, not imported)",
        &["netgraph.tier1_count", "netgraph.tier1_region_presence", "netgraph.tier1_pops_per_region", "netgraph.content_count", "netgraph.content_region_presence", "netgraph.content_pops_per_region",
          "netgraph.transit_per_region", "netgraph.transit_pop_share", "netgraph.incumbent_reach_km", "netgraph.mobile_per_market", "netgraph.local_fixed_base", "netgraph.local_fixed_weight_step", "netgraph.local_fixed_max"],
        "stand-in ASNs from the private-use range"),
    row("as.upstream", B, rel("AS", "AS", true), "network", "customer to provider", "crates/cewp-sim/src/netgraph.rs:generate, route", "MODEL", "RouteViews",
        &["netgraph.content_upstreams", "netgraph.transit_upstreams_max", "netgraph.incumbent_tier1", "netgraph.access_multihome", "netgraph.mobile_central_gateway"], ""),
    row("as.peering", B, rel("AS", "AS", false), "network", "peer links", "crates/cewp-sim/src/netgraph.rs:generate, route", "MODEL", "RouteViews",
        &["netgraph.transit_peer", "netgraph.ixp_peer", "netgraph.content_peer", "netgraph.restrictive_peering"], ""),
    row("ixp.population", B, pop(&["metro"]), "network", "IXPs", "crates/cewp-sim/src/netgraph.rs:generate", "MODEL", "none known (IXP lists are restricted or unlicensed)",
        &["netgraph.ixp_min_weight", "netgraph.ixp_second_weight"], ""),
    row("ixp.membership", B, rel("AS", "IXP", true), "network", "membership", "crates/cewp-sim/src/netgraph.rs:generate", "MODEL", "none known",
        &["netgraph.ixp_join_content", "netgraph.ixp_join_transit", "netgraph.ixp_join_access", "netgraph.ixp_join_incumbent", "netgraph.ixp_join_tier1"], ""),
    row("cable.links", B, rel("metro", "metro", false), "network", "cables and backbone links (km)", "crates/cewp-sim/src/netgraph.rs:generate; failure.rs:DomainWorld::build", "MODEL",
        "none known (cable maps are restricted or paid)",
        &["netgraph.terrestrial_extra_links", "netgraph.cables_per_region_pair", "netgraph.gateway_candidates", "netgraph.terrestrial_inflation", "netgraph.submarine_inflation",
          "failure.world.cables_per_country_max", "failure.world.cable_shared_share"], ""),
    row("path.latency", B, attr("network path", &["access_link.technology", "as.upstream", "cable.links"]), "network", "us one way per element",
        "crates/cewp-sim/src/netgraph.rs:route; media.rs core delays -> media_call2.rs", "MODEL", "none known (RIPE Atlas needs approval)",
        &["netgraph.as_hop_us", "netgraph.ixp_crossing_us", "netgraph.access_fixed_us", "netgraph.access_mobile_us", "netgraph.access_content_us",
          "media.core_same_region", "media.core_cross_region", "media.server_core"], ""),
    row("failure.geography", B, pop(&["country"]), "region", "countries, metros, grid regions, cloud regions",
        "crates/cewp-sim/src/failure.rs:DomainWorld::build", "MODEL", "Gridfinder (power regions); Natural Earth admin-0 (countries, approved set)",
        &["failure.world.countries", "failure.world.metros_per_country_max", "failure.world.metro_zipf", "failure.world.power_region_metros_max",
          "failure.world.cloud_providers", "failure.world.cloud_regions_per_provider"], "a generated stand-in; no real place"),
    row("device.power_region", B, rel("device", "power region", true), "device", "membership", "crates/cewp-sim/src/failure.rs:DomainWorld::build, evaluate", "MODEL",
        "Gridfinder", &["failure.world.ups_share"], "a home node on a UPS outlasts the evaluation hour"),
    row("failure.domain_world_devices", B, pop(&["household", "person"]), "household", "devices by kind in the failure world",
        "crates/cewp-sim/src/failure.rs:DomainWorld::build", "MODEL", "World Bank indicators mirroring ITU",
        &["failure.world.persons_per_household_max", "failure.world.home_node_share", "failure.world.laptop_share", "failure.world.hosted_share",
          "failure.world.relay_pref", "failure.world.mobile_offline_share", "failure.world.laptop_awake_share"], "the failure package's own household and device draw (its own shares, not micro's)"),
    row("failure.room_membership", B, rel("person", "room", true), "room", "rooms per person, size, locality", "crates/cewp-sim/src/failure.rs:DomainWorld::build, room_losses", "MODEL", NONE,
        &["failure.world.rooms_per_person", "failure.world.room_size_max", "failure.world.room_locality"], ""),
    row("failure.outage", B, attr("failure domain", &["device.power_region", "household.access_network", "cable.links"]), "region", "lognormal duration (h) and reach per kind",
        "crates/cewp-sim/src/failure.rs:sample_event, Event::restore_h", "MODEL", "anchored on post-incident reports (recalled, not re-read)",
        &["failure.power.median_h", "failure.power.sigma", "failure.power.ramp_share", "failure.power.dc_fail", "failure.isp.median_h", "failure.isp.sigma",
          "failure.cable.cuts", "failure.cable.median_h", "failure.cable.sigma", "failure.cloud.median_h", "failure.cloud.sigma", "failure.cloud.ramp_share",
          "failure.dns.median_h", "failure.dns.sigma", "failure.bgp.acceptor_share", "failure.bgp.median_h", "failure.bgp.sigma", "failure.cell.median_h", "failure.cell.sigma",
          "failure.shutdown.median_h", "failure.shutdown.sigma", "failure.shutdown.mobile_also"], "power and network failure domains"),
    row("tower.backup_hours", B, attr("cell tower", &["device.power_region"]), "access network", "h of battery during a grid outage", "crates/cewp-sim/src/failure.rs:evaluate", "FRONTIER",
        "none known (tower data is share-alike)", &["failure.power.tower_backup_h"], ""),
    row("household.kin", B, rel("household", "household", false), "household", "kin room membership (households per room, diaspora)",
        "crates/cewp-sim/src/social.rs:SocialWorld::build -> locality.rs", "MODEL", "none known (UN DESA migrant stock is the kind of source)",
        &["social.persons_per_household", "social.kin_households_mean", "social.diaspora_share", "social.diaspora_abroad_share"], "social.persons_per_household restates micro's"),
    row("person.work_team", B, rel("person", "workplace room", true), "person", "membership", "crates/cewp-sim/src/social.rs:SocialWorld::build -> locality.rs", "MODEL",
        "none known (census commuting flows are the kind of source)",
        &["social.worker_share", "social.work_team_median", "social.work_team_sigma", "social.work_team_max", "social.work_cross_locality_share", "social.work_cross_region_share"], ""),
    row("person.school_class", B, rel("person", "school room", true), "person", "membership", "crates/cewp-sim/src/social.rs:SocialWorld::build -> locality.rs", "MODEL",
        "none known (enrolment and class-size tables)", &["social.student_share", "social.class_size"], ""),
    row("household.neighbourhood_room", B, rel("household", "neighbourhood room", true), "household", "membership", "crates/cewp-sim/src/social.rs:SocialWorld::build", "MODEL", NONE,
        &["social.community_join_share", "social.neighbourhood_room_households"], "the same share as micro's street room"),
    row("person.interest_group", B, rel("person", "interest room", true), "person", "membership", "crates/cewp-sim/src/social.rs:SocialWorld::build", "FRONTIER", NONE,
        &["social.interest_groups_per_person", "social.interest_group_mean_size", "social.interest_local_share", "social.interest_regional_share", "social.interest_pareto_alpha"], ""),
    row("person.public_follow", B, rel("person", "public room", true), "person", "follows", "crates/cewp-sim/src/social.rs:SocialWorld::build", "FRONTIER", NONE,
        &["social.public_follows_per_person", "social.public_room_mean_size", "social.public_local_share", "social.public_regional_share", "social.public_pareto_alpha"], ""),
    row("person.tie", B, rel("person", "person", false), "person", "ties inside a room", "crates/cewp-sim/src/social.rs:SocialWorld::build -> activity.rs", "MODEL", NONE,
        &["social.clique_cap", "social.large_group_ties"], ""),
    row("act.recipient", B, rel("act", "room", true), "act", "share of a person's acts per room kind", "crates/cewp-sim/src/activity.rs:Activity::choose", "FRONTIER", NONE,
        &["activity.family_household_share", "activity.community_weight_work", "activity.community_weight_school", "activity.community_weight_neighbourhood", "activity.community_weight_interest",
          "activity.tie_weight_household", "activity.tie_weight_kin", "activity.tie_weight_work", "activity.tie_weight_school", "activity.tie_weight_neighbourhood",
          "activity.tie_weight_interest", "activity.tie_weight_public", "activity.tie_weight_consent_free"], "the 90/9/1 locality mix is measured on the result (locality.rs), not set"),
    row("presence.work_hours", B, rhythm(BP, WEEK_BINS, "room kind (work, school) active"), "person", "share active", "crates/cewp-sim/src/activity.rs:rhythm, Activity::choose", "MODEL",
        "American Time Use Survey (one country)", &["activity.work_hours_start", "activity.work_hours_end", "activity.off_hours_work_affinity"], "becomes presence.people"),
    row("act.weekly_shape", B, rhythm(BP, WEEK_BINS, "day class"), "person", "multiplier", "crates/cewp-sim/src/activity.rs:rhythm", "MODEL", "American Time Use Survey (one country)",
        &["activity.weekly_weekday", "activity.weekly_saturday", "activity.weekly_sunday", "activity.weekend_shift_h"], "with model.diurnal.v1 inside each day"),
    row("act.cascade", B, attr("act", &["act.recipient"]), "act", "replies per act; reply delay (s)", "crates/cewp-sim/src/activity.rs:Activity::run", "MODEL", NONE,
        &["activity.branching", "activity.reply_scale_s", "activity.reply_alpha", "activity.reply_max_s"], "Hawkes branching"),
    row("event.synchronized", B, rhythm(BP, WEEK_BINS, "event reach (locality | region | world)"), "region", "events and participants",
        "crates/cewp-sim/src/activity.rs:Activity::events, run", "FRONTIER", NONE,
        &["activity.event_share", "activity.event_share_locality", "activity.event_share_region", "activity.event_share_world", "activity.event_participation_locality",
          "activity.event_participation_region", "activity.event_participation_world", "activity.event_acts_per_participant", "activity.event_decay_s",
          "activity.event_mix_family", "activity.event_mix_community", "activity.event_mix_consent_free", "activity.event_mix_public"], "matches, news, a viral post"),
    row("device.capability", B, attr("device", &["household.devices"]), "device", "us per sign / verify; capture and render delay",
        "crates/cewp-sim/src/media.rs:AccessProfile verify_us, sign_us; media_call2.rs", "FRONTIER", "none known (a measured number is requested)",
        &["media.verify_phone", "media.verify_desktop", "media.sign_phone", "media.sign_desktop", "media.capture_encode", "media.decode_render"], "the device-capability class"),
    row("device.crash", B, attr("phone", &[]), "device", "ppm of departures with no signed end", "crates/cewp-sim/src/media.rs (call legs)", "MODEL", NONE, &["media.phone_crash_share"], ""),
    row("device.daily_events", B, rhythm(BP, WEEK_BINS, "device"), "device", "starts, writes and new devices per device per day",
        "crates/cewp-sim/src/capacity.rs:CostParams (seat load)", "MODEL", "none known (fleet gauges)",
        &["cost.runtime_starts_per_device_day", "cost.pull_plane_writes_per_device_day", "cost.new_rows_per_device_day", "cost.device_starts_per_day",
          "cost.new_device_share_day", "cost.key_record_writes_per_device_day", "cost.phone_awake_share", "cost.claimed_share"], "flat per day today"),
    // ------------------------------------------------------------------ C: wanted
    row("presence.people", C, rhythm(BP, WEEK_BINS, "place_kind x person_kind; place_kind home.<dwelling class incl. dense fabric> | work | school | venue | transit | elsewhere; person_kind working_age | student_child | retired | shift"),
        "cell", "persons per bin", "future: micro.rs churn and the scale engine's signer draw (who is home when)", "-", "American Time Use Survey (one country)", &[],
        "in every bin people across place kinds sum to the cell's people; emitted per bin with the flows between place kinds, declared source 0"),
    row("activity.message.direct.events", C, rhythm(BR, WEEK_BINS, "place_kind x person_kind"), "cell", "events per bin per 1,000,000 persons of the class", "future: per-kind act generator (seam owed)", "-",
        "American Time Use Survey (one country)", &[], "generator model.diurnal.v1 attached until a reference stands"),
    row("activity.message.room.events", C, rhythm(BR, WEEK_BINS, "place_kind x person_kind"), "cell", "events per bin per 1,000,000 persons of the class", "future: per-kind act generator (seam owed)", "-", NONE, &[], "model.diurnal.v1"),
    row("activity.call.voice.events", C, rhythm(BR, WEEK_BINS, "place_kind x person_kind"), "cell", "events per bin per 1,000,000 persons of the class", "future: per-kind act generator; media call scenarios", "-", NONE, &[], "model.diurnal.v1"),
    row("activity.call.video.events", C, rhythm(BR, WEEK_BINS, "place_kind x person_kind"), "cell", "events per bin per 1,000,000 persons of the class", "future: per-kind act generator; media call scenarios", "-", NONE, &[], "model.diurnal.v1"),
    row("activity.media.share.events", C, rhythm(BR, WEEK_BINS, "place_kind x person_kind"), "cell", "events per bin per 1,000,000 persons of the class", "future: per-kind act generator; chunk custody", "-", NONE, &[], "model.diurnal.v1"),
    row("activity.agent.follow.events", C, rhythm(BP, WEEK_BINS, "place_kind x person_kind"), "cell", "events per bin per 1,000,000 persons of the class", "future: per-kind act generator", "-", NONE, &[], "model.diurnal.v1; one-ended"),
    row("activity.agent.scheduled.events", C, rhythm(BP, WEEK_BINS, "place_kind x person_kind"), "cell", "events per bin per 1,000,000 persons of the class", "future: per-kind act generator", "-", NONE, &[], "model.diurnal.v1; one-ended"),
    row("activity.presence.change.events", C, rhythm(BP, WEEK_BINS, "place_kind x person_kind"), "cell", "events per bin per 1,000,000 persons of the class", "future: per-kind act generator", "-", NONE, &[], "model.diurnal.v1; one-ended"),
    row("activity.message.direct.weekly", C, pop(&["person"]), "person", "events per person per week", "future: per-kind act generator (volume, separate from the shape)", "-", NONE, &[], ""),
    row("activity.message.room.weekly", C, pop(&["person"]), "person", "events per person per week", "future: per-kind act generator", "-", NONE, &[], ""),
    row("activity.call.voice.weekly", C, pop(&["person"]), "person", "events per person per week", "future: per-kind act generator", "-", NONE, &[], ""),
    row("activity.call.video.weekly", C, pop(&["person"]), "person", "events per person per week", "future: per-kind act generator", "-", NONE, &[], ""),
    row("activity.media.share.weekly", C, pop(&["person"]), "person", "events per person per week", "future: per-kind act generator", "-", NONE, &[], ""),
    row("activity.agent.follow.weekly", C, pop(&["person"]), "person", "events per person per week", "future: per-kind act generator", "-", NONE, &[], ""),
    row("activity.agent.scheduled.weekly", C, pop(&["person"]), "person", "events per person per week", "future: per-kind act generator", "-", NONE, &[], ""),
    row("activity.presence.change.weekly", C, pop(&["person"]), "person", "events per person per week", "future: per-kind act generator", "-", NONE, &[], ""),
    row("attach.phone.network", C, rhythm(BR, WEEK_BINS, "place_kind; network kind home_wifi | work_wifi | cellular"), "device", "phones per network kind",
        "future: micro.rs churn and change_network (replaces phone.attach)", "-", NONE, &[], "in every bin a phone is on exactly one; emitted with flows between network kinds, source 0"),
    row("joint.presence_attach_activity", C, rhythm(BP, WEEK_BINS, "place_kind x person_kind x network kind x activity kind"), "cell", "persons",
        "future: micro.rs focal world (a phone's network, place and acts drawn together)", "-", NONE, &[],
        "joint table whose marginals are presence.people, attach.phone.network and activity.<kind>.events"),
    row("joint.activity_devices", C, pop(&["household", "person"]), "household", "persons by activity kind x device kind", "future: micro.rs populate and generate", "-", NONE, &[],
        "joint table whose marginals are household.devices and activity.<kind>.weekly"),
    row("building.radio_neighbourhood", C, rel("device", "device", false), "building", "pairs within Wi-Fi / BLE / LoRa range",
        "future: a local-link model for edge's packet-radio and LAN transports", "-", "building footprints (Microsoft, CDLA); STREETREF-1 via CIRISHolon", &[], "walls and storeys between them"),
    row("building.line_of_sight", C, rel("building", "building", false), "building", "visible pairs (packet radio)", "future: packet-radio link model", "-", "ETOPO 2022 (shipped) with GHSL building height", &[], ""),
    row("building.material_class", C, attr("building", &["building.radio_neighbourhood"]), "building", "class", "future: indoor radio attenuation", "-", NONE, &[], "indoor versus outdoor"),
    row("building.units_per_storey", C, pop(&["building"]), "building", "dwellings per storey", "future: vertical neighbours for local links", "-", "GHSL building height and volume", &[], ""),
    row("household.nearest_exchange", C, rel("household", "exchange", true), "household", "membership and km", "future: access latency and shared fate", "-", "none known (exchanges are restricted or paid)", &[], ""),
    row("household.nearest_tower", C, rel("household", "cell tower", true), "household", "membership and km", "future: cellular capacity and shared fate", "-", "none known (towers are share-alike)", &[], ""),
    row("tower.backhaul", C, attr("cell tower", &["tower.backup_hours"]), "access network", "class (fibre | microwave | satellite) and resilience", "future: failure.rs tower domain", "-", NONE, &[], ""),
    row("metro.nearest_ixp", C, rel("metro", "IXP", true), "network", "membership and km", "future: netgraph.rs route", "-", NONE, &[], ""),
    row("person.commute", C, rhythm(BR, WEEK_BINS, "home cell x work cell"), "cell", "persons moving", "future: micro.rs mobility (phones changing networks and neighbours through the day)", "-",
        "none known (census commuting flows)", &[], "row sums are the origin's residents"),
    row("gathering.population", C, pop(&["cell"]), "cell", "gathering places by kind (school, workplace, market, venue, transit)", "future: presence.people place kinds; local links where devices meet", "-",
        "STREETREF-1 via CIRISHolon (OpenStreetMap statistics)", &[], ""),
    row("person.language", C, attr("person", &["locality.local_hour"]), "person", "language", "future: call scheduling, room membership", "-", NONE, &[], ""),
    row("locality.time_zone", C, attr("locality", &[]), "locality", "zone offset and DST", "future: every rhythm's bin 0 (today round(lon / 15))", "-", "Natural Earth time zones (approved set)", &[], ""),
    row("region.week_class", C, attr("region", &[]), "region", "mon-fri | sun-thu | sat-wed", "future: every rhythm row's week coordinate", "-", NONE, &[], ""),
    row("household.capability_class", C, attr("household", &["household.devices", "access_link.rate"]), "household", "class (income / device capability)", "future: codec and uplink choice in media", "-", NONE, &[], ""),
    row("device.energy", C, attr("device", &["household.devices", "phone.charging"]), "device", "battery behaviour and energy cost per class", "future: reach::hooks energy counters", "-", NONE, &[], ""),
    row("region.seasonal_load", C, rhythm(BP, 53, "ISO week"), "region", "multiplier", "future: workload volume", "-", NONE, &[], "seasonal and event rhythms"),
    row("region.disaster_exposure", C, attr("region", &["failure.outage"]), "region", "hazard class", "future: failure.rs event library", "-", NONE, &[], ""),
    row("region.censorship_exposure", C, attr("region", &["failure.outage"]), "region", "class", "future: failure.rs shutdown scenario", "-", NONE, &[], ""),
    row("settlement.thin_uplink", C, attr("street tile", &["cell.settlement_class", "access_link.rate"]), "street tile", "villages behind one thin uplink", "future: the 45.1 village case (one shared uplink)", "-",
        "GHSL settlement classes; M-Lab NDT", &[], "rural settlement pattern"),
    row("household.power_region", C, rel("building", "power region", true), "building", "membership", "future: failure.rs, the building's second parent", "-", "Gridfinder", &[], "the building's second parent"),
];

/// Parameters that describe the protocol, the deployment, the display or the evaluation, not the
/// world, with the reason. Every MODEL / FRONTIER parameter is either here or in a row.
pub const NOT_WORLD: &[(&str, &str)] = &[
    ("micro.homing_k", "protocol: top-k homing"),
    ("micro.bundle_bootstrap", "protocol: edge #881 switch"),
    ("micro.ci_arrivals_per_hour", "deployment: the project's own CI fleet"),
    ("micro.ci_job_mean_s", "deployment: the project's own CI fleet"),
    ("micro.ci_old_version_share", "deployment: the project's own CI fleet"),
    ("micro.ci_msg_interval_s", "deployment: the project's own CI fleet"),
    ("micro.lan_autointerface", "host configuration: LAN discovery on"),
    ("micro.target_households", "display: the patch is sized for the R5 visual budget"),
    ("failure.world.nodes", "evaluation: world size"),
    ("failure.world.seats", "deployment: serve seats"),
    ("failure.eval.hour", "evaluation"),
    ("failure.eval.rejoin_share", "evaluation"),
    ("failure.eval.peer_cache_share", "evaluation"),
    ("failure.withhold.relay_share", "adversary model, not a scene"),
    ("failure.withhold.median_h", "adversary model, not a scene"),
    ("failure.withhold.sigma", "adversary model, not a scene"),
    ("media.setup_rtts", "protocol"),
    ("media.audio_bitrate", "host codec configuration"),
    ("media.audio_frame", "host codec configuration"),
    ("media.video_bitrate", "host codec configuration"),
    ("media.video_fps", "host codec configuration"),
    ("media.temporal_layers", "host codec configuration"),
    ("media.gop", "host codec configuration"),
    ("media.keyframe_ratio", "host codec configuration"),
    ("media.t1_ratio", "host codec configuration"),
    ("media.frame_size_jitter", "host codec configuration"),
];

/// Every spec array's parameters as `(qualified name, class)`, qualified `<array>.<name>` with the
/// array named micro, social, activity, netgraph, reach, failure, media or cost.
#[must_use]
pub fn spec_names() -> Vec<(String, &'static str)> {
    use crate::{activity, capacity, failure, media, micro, netgraph, reach, social};
    let mut v: Vec<(String, &'static str)> = Vec::new();
    v.extend(micro::PARAM_SPECS.iter().map(|s| (format!("micro.{}", s.name), s.tag.as_str())));
    v.extend(social::SOCIAL_PARAM_SPECS.iter().map(|s| (format!("social.{}", s.name), s.tag.as_str())));
    v.extend(activity::ACTIVITY_PARAM_SPECS.iter().map(|s| (format!("activity.{}", s.name), s.tag.as_str())));
    v.extend(netgraph::NET_PARAM_SPECS.iter().map(|s| (format!("netgraph.{}", s.name), s.tag.as_str())));
    v.extend(reach::REACH_PARAM_SPECS.iter().map(|s| (format!("reach.{}", s.name), s.class.as_str())));
    v.extend(failure::FAILURE_SPECS.iter().map(|s| (format!("failure.{}", s.name), s.class.as_str())));
    v.extend(media::MEDIA_PARAM_SPECS.iter().map(|s| (format!("media.{}", s.name), s.tag.as_str())));
    v.extend(capacity::COST_SPECS.iter().map(|s| (format!("cost.{}", s.name), s.tag.as_str())));
    v
}

/// The cost table describes the seat's software; only the device-behaviour rows named in
/// `device.daily_events` are world parameters, so the completeness check skips the rest of it.
#[cfg(test)]
fn checked(name: &str) -> bool {
    !name.starts_with("cost.")
}

fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn list(v: &[&str]) -> String {
    format!("[{}]", v.iter().map(|s| esc(s)).collect::<Vec<_>>().join(", "))
}

/// The registry as JSON (one row per line, stable order), with the bin convention and notes.
#[must_use]
pub fn to_json() -> String {
    let classes: std::collections::BTreeMap<String, &str> = spec_names().into_iter().collect();
    let mut s = String::new();
    s.push_str("{\n  \"registry\": \"CEWP twin couplings (CIRISHolon engine/FSD_WORLD_MODEL.md section 10)\",\n");
    s.push_str("  \"generated_by\": \"cargo run -p cewp-sim --example couplings -- --write (crates/cewp-sim/src/couplings.rs)\",\n");
    let _ = writeln!(
        s,
        "  \"bins\": {{\"hour_of_week\": {WEEK_BINS}, \"bin0\": \"Monday 00:00-01:00 local zone time of the holon\", \"zone_from_longitude\": \"round(lon / 15) hours where only longitude is known\", \"week\": [\"mon-fri\", \"sun-thu\", \"sat-wed\"]}},"
    );
    s.push_str("  \"generators\": [{\"name\": \"model.diurnal.v1\", \"was\": \"model.diurnal.v0\", \"class\": \"MODEL\", \"verdict\": \"unresolved: no reference\", \"code\": \"crates/cewp-sim/src/workload.rs:diurnal_multiplier\", \"trough\": 0.50, \"peak\": 1.60, \"peak_hour\": 19.5, \"attached_to\": \"activity.<kind>.events\"}],\n");
    s.push_str("  \"rows\": [\n");
    for (i, r) in ROWS.iter().enumerate() {
        let mut o = format!(
            "    {{\"name\": {}, \"part\": \"{}\", \"status\": \"{}\", \"type\": \"{}\", \"tier\": {}, \"unit\": {}, ",
            esc(r.name), r.status.part(), r.status.as_str(), r.ty.as_str(), esc(r.tier), esc(r.unit)
        );
        match r.ty {
            Ty::Population { parents } => o.push_str(&format!("\"parents\": {}, ", list(parents))),
            Ty::Relationship { from, to, directed } => o.push_str(&format!("\"joins\": [{}, {}], \"directed\": {directed}, ", esc(from), esc(to))),
            Ty::Attribute { of, with } => o.push_str(&format!("\"of\": {}, \"read_with\": {}, ", esc(of), list(with))),
            Ty::Rhythm { base, bins, by } => o.push_str(&format!(
                "\"base\": \"{}\", \"bins\": {bins}, \"by\": {}, ",
                if base == Base::Population { "population" } else { "relationship" },
                esc(by)
            )),
        }
        let params: Vec<String> = r.params.iter().map(|p| format!("{{\"name\": {}, \"class\": \"{}\"}}", esc(p), classes.get(*p).copied().unwrap_or("?"))).collect();
        o.push_str(&format!(
            "\"consumer\": {}, \"class\": {}, \"reference\": {}, \"params\": [{}], \"note\": {}}}",
            esc(r.consumer), esc(r.class), esc(r.reference), params.join(", "), esc(r.note)
        ));
        s.push_str(&o);
        s.push_str(if i + 1 < ROWS.len() { ",\n" } else { "\n" });
    }
    s.push_str("  ],\n  \"not_world\": [\n");
    for (i, (n, why)) in NOT_WORLD.iter().enumerate() {
        let _ = write!(s, "    {{\"name\": {}, \"why\": {}}}", esc(n), esc(why));
        s.push_str(if i + 1 < NOT_WORLD.len() { ",\n" } else { "\n" });
    }
    s.push_str("  ]\n}\n");
    s
}

fn md_cell(s: &str) -> String {
    s.replace('|', "\\|")
}

/// The generated tables for `FSD/twin/couplings.md`, one per part.
#[must_use]
pub fn to_markdown_tables() -> String {
    let mut s = String::new();
    for (status, title) in [
        (Status::ReadToday, "Part A: read today"),
        (Status::InventedToday, "Part B: modelled today on invented inputs"),
        (Status::Wanted, "Part C: wanted, not modelled"),
    ] {
        let rows: Vec<&Row> = ROWS.iter().filter(|r| r.status == status).collect();
        let _ = writeln!(s, "### {title} ({} rows)\n", rows.len());
        s.push_str("| name | type | tier | unit | joins / parents / read with / bins | consumer | class | reference | params |\n|---|---|---|---|---|---|---|---|---|\n");
        for r in rows {
            let shape = match r.ty {
                Ty::Population { parents } => format!("parents: {}", parents.join(", ")),
                Ty::Relationship { from, to, directed } => format!("{from} {} {to}", if directed { "->" } else { "--" }),
                Ty::Attribute { of, with } => {
                    if with.is_empty() {
                        format!("of {of}")
                    } else {
                        format!("of {of}; with {}", with.join(", "))
                    }
                }
                Ty::Rhythm { base, bins, by } => format!("{} x {bins} bins by {by}", if base == Base::Population { "population" } else { "relationship" }),
            };
            let note = if r.note.is_empty() { String::new() } else { format!(" ({})", r.note) };
            let _ = writeln!(
                s,
                "| `{}` | {} | {} | {} | {} | {}{} | {} | {} | {} |",
                r.name,
                r.ty.as_str(),
                md_cell(r.tier),
                md_cell(r.unit),
                md_cell(&shape),
                md_cell(r.consumer),
                md_cell(&note),
                r.class,
                md_cell(r.reference),
                r.params.iter().map(|p| format!("`{p}`")).collect::<Vec<_>>().join(" ")
            );
        }
        s.push('\n');
    }
    s.push_str("### Parameters that do not describe the world\n\n| parameter | why |\n|---|---|\n");
    for (n, why) in NOT_WORLD {
        let _ = writeln!(s, "| `{n}` | {why} |");
    }
    s
}

/// Markers around the generated tables in `FSD/twin/couplings.md`.
pub const BEGIN: &str = "<!-- couplings:begin (generated by crates/cewp-sim/examples/couplings.rs; do not edit) -->";
/// See [`BEGIN`].
pub const END: &str = "<!-- couplings:end -->";

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};

    #[test]
    fn every_world_parameter_has_exactly_one_row() {
        let specs = spec_names();
        let known: BTreeSet<&str> = specs.iter().map(|(n, _)| n.as_str()).collect();
        let mut seen: BTreeMap<&str, usize> = BTreeMap::new();
        for r in ROWS {
            for p in r.params {
                assert!(known.contains(p), "{}: no spec parameter {p}", r.name);
                *seen.entry(p).or_default() += 1;
            }
        }
        for (n, _) in NOT_WORLD {
            assert!(known.contains(n), "NOT_WORLD names no spec parameter {n}");
            *seen.entry(n).or_default() += 1;
        }
        for (n, k) in &seen {
            assert_eq!(*k, 1, "{n} is listed {k} times");
        }
        let missing: Vec<&str> = specs
            .iter()
            .filter(|(n, c)| checked(n) && matches!(*c, "MODEL" | "FRONTIER") && !seen.contains_key(n.as_str()))
            .map(|(n, _)| n.as_str())
            .collect();
        assert!(missing.is_empty(), "MODEL/FRONTIER parameters with no row: {missing:?}");
    }

    #[test]
    fn names_are_unique_and_rows_are_well_formed() {
        let mut names = BTreeSet::new();
        for r in ROWS {
            assert!(names.insert(r.name), "duplicate name {}", r.name);
            assert!(!r.consumer.is_empty() && !r.unit.is_empty() && !r.tier.is_empty(), "{}", r.name);
            match r.status {
                Status::Wanted => assert!(r.consumer.starts_with("future: ") && r.class == "-" && r.params.is_empty(), "{}", r.name),
                _ => assert!(!r.consumer.starts_with("future: ") && r.class != "-", "{}", r.name),
            }
            if let Ty::Rhythm { bins, .. } = r.ty {
                assert!(bins == WEEK_BINS || bins == 53, "{}", r.name);
            }
        }
    }

    #[test]
    fn read_with_names_rows() {
        let names: BTreeSet<&str> = ROWS.iter().map(|r| r.name).collect();
        for r in ROWS {
            if let Ty::Attribute { with, .. } = r.ty {
                for w in with {
                    assert!(names.contains(w), "{} reads with {w}, which is no row", r.name);
                }
            }
        }
    }

    #[test]
    fn holons_registered_rhythm_names_are_present_exactly_as_spelled() {
        let kinds = ["message.direct", "message.room", "call.voice", "call.video", "media.share", "agent.follow", "agent.scheduled", "presence.change"];
        let mut want: Vec<String> = vec!["presence.people".into(), "attach.phone.network".into(), "joint.presence_attach_activity".into(), "joint.activity_devices".into()];
        for k in kinds {
            want.push(format!("activity.{k}.events"));
            want.push(format!("activity.{k}.weekly"));
        }
        for w in &want {
            let r = ROWS.iter().find(|r| r.name == w).unwrap_or_else(|| panic!("missing {w}"));
            assert_eq!(r.status, Status::Wanted, "{w}");
        }
        for k in ["agent.follow", "agent.scheduled", "presence.change"] {
            let r = ROWS.iter().find(|r| r.name == format!("activity.{k}.events")).unwrap();
            assert!(matches!(r.ty, Ty::Rhythm { base: Base::Population, bins: WEEK_BINS, .. }), "{k} is one-ended");
        }
        for k in ["message.direct", "message.room", "call.voice", "call.video", "media.share"] {
            let r = ROWS.iter().find(|r| r.name == format!("activity.{k}.events")).unwrap();
            assert!(matches!(r.ty, Ty::Rhythm { base: Base::Relationship, bins: WEEK_BINS, .. }), "{k} is two-ended");
        }
    }

    #[test]
    fn the_committed_files_are_current() {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
        let json = std::fs::read_to_string(format!("{root}/docs/geo/couplings.json")).expect("docs/geo/couplings.json");
        assert!(json == to_json(), "docs/geo/couplings.json is stale: cargo run -p cewp-sim --example couplings -- --write");
        let md = std::fs::read_to_string(format!("{root}/FSD/twin/couplings.md")).expect("FSD/twin/couplings.md");
        let (a, b) = (md.find(BEGIN).expect("begin marker"), md.find(END).expect("end marker"));
        assert!(md[a + BEGIN.len()..b] == format!("\n{}", to_markdown_tables()), "FSD/twin/couplings.md tables are stale: cargo run -p cewp-sim --example couplings -- --write");
    }
}
