//! Constitutional Transport Contract (CC §5.4.3 & §5.4.6, Tiers.lean, claims.tsv).
//!
//! Models:
//! - 5 Named Budget Classes + Seat class (CC §5.4.3, Tiers.lean)
//! - Shaped-Link Bound (L ≤ 8, CLM-shaped-link-bound-refuse)
//! - No rank displacement: CIRISEdge v45 has none until CIRISEdge#939 (`src/transport/link_shaping.rs:356`);
//!   a full node runs the next shaping peer unshaped, with referrals, or refuses it if shaped-only
//! - One direct hop (CLM-shaped-link-one-hop): a NAT'd node dialling a reachable node is one hop;
//!   only a link a forwarder carries is unshaped (`src/transport/reticulum.rs:534` `link_is_forwarded`).
//!   Separate failure counters (reach ≠ capacity)
//! - Up to 3 sealed referrals (MAX_REFERRALS = 3, CLM-refusal-carries-referrals) ranked by inbound dials descending
//! - 64 KiB (65,536 B) Shaped-Link Payload Boundary & unshaped bulk/AV (CC §5.3.3.2)
//! - Carriage axis (Carriage::ConsentOnly vs Carriage::Full, CLM-link-carriage-axis) + 16 consent-free envelope kinds
//! - Anonymous Tier (CC §5.4.6) 3-hop CONP (15,296 B, +0 B cover replacement, floor ≥ 3) and 2-member room-mailbox
//! - Sticky guards (CC §5.4.6 item 4): kept as long as they answer (`guard_reachable: bool`), never rotated on a 60-day schedule;
//!   if below floor, returns `OnionTierUnavailableFloor`
//! - `proxy_pull`: checks replication tier (CC §5.4.6) — non-federation tier is refused (`RefusedNonFederationTier`)
//! - `custody:ack:v1` vs retired `holds_bytes`: `holds_bytes` is refused (CC §3.1.3.3)
//! - `shaped_only` node state (2026-11-06 rule): ninth peer refused only on `shaped_only` or Anonymous Tier, runs unshaped otherwise

use std::collections::{BTreeMap, BTreeSet};

/// Maximum number of device-tier shaped links per node (Tiers.bound = 8, CC §5.4.3).
pub const SHAPED_LINK_BOUND: usize = 8;

/// Idle timeout in seconds for an asked shaped link on an off node (10 minutes = 600 s, CC §5.4.3 / §5.4.6).
pub const SHAPED_ASK_IDLE_TIMEOUT_SECS: u64 = 600;

/// Maximum number of sealed referrals returned on a refusal or displacement (CC §5.4.3).
pub const MAX_REFERRALS: usize = 3;

/// Payload boundary: objects <= 65,536 B may traverse shaped links;
/// objects > 65,536 B never cross a shaped link whole or in chunks (CC §5.3.3.2).
pub const SHAPED_PAYLOAD_LIMIT_BYTES: usize = 65_536;

/// Exact constitutional refusal string when an object exceeds 64 KiB on a shaped link.
pub const REFUSAL_PAYLOAD_TOO_LARGE: &str =
    "This file is too large to access over shaped links";

/// Channel payload MDU over base Reticulum transport (CC §5.4.3).
pub const MDU_RETICULUM: usize = 400;

/// Channel payload MDU over TCP transport (CC §5.4.3).
pub const MDU_TCP: usize = 16_272;

/// Anonymous tier 3-hop CONP (X-Wing) onion frame size in bytes (CC §5.4.6).
pub const CONP_FRAME_SIZE: usize = 15_296;

/// Additional wire bytes introduced by CONP frame over scheduled cover: exactly 0 B.
pub const CONP_EXTRA_BYTES: usize = 0;

/// Anonymous tier minimum answering relays required per layer to construct onion paths.
pub const ONION_TIER_FLOOR_RELAYS: usize = 3;

/// Required number of member-owned always-on nodes holding a private room mailbox.
pub const ROOM_MAILBOX_MEMBERS: usize = 2;

/// Warning threshold in seconds when both room endpoints are awake simultaneously.
pub const ROOM_MAILBOX_AWAKE_WARNING_SECS: u64 = 7_200; // 2 hours

/// Canonical holding claim token prefix (CC §3.1.3.3).
pub const CUSTODY_ACK_V1_PREFIX: &str = "custody:ack:v1";

/// Retired holding claim token prefix (CC §3.1.3.3).
pub const RETIRED_HOLDS_BYTES_PREFIX: &str = "holds_bytes";

/// Budget classes naming per-link byte rates and ranks (CC §5.4.3, Tiers.lean).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BudgetClass {
    /// No shaping offered (0 B/s, rank 0).
    Off = 0,
    /// Mobile cellular metered data (350 B/s, ρ=0.5, W=3600s, real share 175 B/s, rank 1).
    PhoneCellular = 1,
    /// Mobile Wi-Fi unmetered data (1,400 B/s, ρ=0.5, W=600s, real share 700 B/s, rank 2).
    PhoneWifi = 2,
    /// Home broadband connection (2,800 B/s, ρ=0.5, W=600s, real share 1,400 B/s, rank 3).
    Home = 3,
    /// Dedicated server connection (7,000 B/s, ρ=0.5, W=600s, real share 3,500 B/s, rank 4).
    Server = 4,
    /// Peer-to-peer seat-class link (33,000 B/s, ρ=0.5, W=600s, real share 16,500 B/s, rank 5).
    /// Offered ONLY when both endpoints hold a `serve` seat proved from the signed roster.
    /// Sits outside the 8-link device bound (CLM-seat-class).
    Seat = 5,
}

impl BudgetClass {
    /// Rank of the budget class (the order `agree` takes the lower of). CIRISEdge v45 has no
    /// displacement by rank (CIRISEdge#939).
    #[must_use]
    pub const fn rank(self) -> u8 {
        self as u8
    }

    /// Nominal per-link byte rate of the class.
    #[must_use]
    pub const fn default_bytes_per_s(self) -> u32 {
        match self {
            Self::Off => 0,
            Self::PhoneCellular => 350,
            Self::PhoneWifi => 1_400,
            Self::Home => 2_800,
            Self::Server => 7_000,
            Self::Seat => 33_000,
        }
    }

    /// ρ (rho): fraction of the shaped stream admitted as real load per window.
    #[must_use]
    pub const fn rho(self) -> f64 {
        match self {
            Self::Off => 0.0,
            _ => 0.5,
        }
    }

    /// Budget window duration in seconds (W).
    #[must_use]
    pub const fn window_secs(self) -> u32 {
        match self {
            Self::Off => 0,
            Self::PhoneCellular => 3_600,
            Self::PhoneWifi | Self::Home | Self::Server | Self::Seat => 600,
        }
    }

    /// Real payload throughput share admitted per second: `bytes_per_s * rho`.
    #[must_use]
    pub const fn real_share_bytes_per_s(self) -> u32 {
        match self {
            Self::Off => 0,
            Self::PhoneCellular => 175,
            Self::PhoneWifi => 700,
            Self::Home => 1_400,
            Self::Server => 3_500,
            Self::Seat => 16_500,
        }
    }

    /// Standard string token for logs, configuration, and telemetry.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::PhoneCellular => "phone-cellular",
            Self::PhoneWifi => "phone-wifi",
            Self::Home => "home",
            Self::Server => "server",
            Self::Seat => "seat",
        }
    }

    /// Parse a configuration or telemetry string token.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "off" | "none" => Some(Self::Off),
            "phone-cellular" | "cellular" => Some(Self::PhoneCellular),
            "phone-wifi" | "wifi" | "phone" => Some(Self::PhoneWifi),
            "home" => Some(Self::Home),
            "server" => Some(Self::Server),
            "seat" => Some(Self::Seat),
            _ => None,
        }
    }

    /// Pure function resolving two offers into an agreed class (CC §5.4.3 item 1).
    /// The link runs at the lower of the two offers (`local_class.min(peer_class)`).
    /// Either side offering Off leaves the link unshaped.
    /// Seat is valid ONLY if `both_hold_serve_seat` is true; otherwise degrades
    /// to the min of the non-seat class or Server.
    #[must_use]
    pub fn agree(a: Self, b: Self, both_hold_serve_seat: bool) -> Self {
        if a == Self::Off || b == Self::Off {
            return Self::Off;
        }
        let a_eff = if a == Self::Seat && !both_hold_serve_seat {
            Self::Server
        } else {
            a
        };
        let b_eff = if b == Self::Seat && !both_hold_serve_seat {
            Self::Server
        } else {
            b
        };

        if a_eff.rank() < b_eff.rank() {
            a_eff
        } else {
            b_eff
        }
    }

    /// Whether shaping is offered by default for this class (CC §5.4.3 / §5.4.6).
    /// Offered by default on Server (true); off by default elsewhere (Home, PhoneWifi, PhoneCellular).
    #[must_use]
    pub const fn shaping_offered_by_default(self) -> bool {
        shaping_offered_by_default(self)
    }

    /// Relay is offered by default on every owned node, except a phone only offers relay
    /// while charging AND on an unmetered link (is_charging && is_unmetered_wifi) (CC §5.4.6).
    #[must_use]
    pub fn relay_offered_by_default(self, is_charging: bool, is_unmetered_wifi: bool) -> bool {
        relay_offered_by_default(self, is_charging, is_unmetered_wifi)
    }
}

/// Pure function determining whether shaping is offered by default for a budget class (CC §5.4.3 / §5.4.6).
/// True for Server (and Seat); false (off by default) elsewhere (Home, PhoneWifi, PhoneCellular).
#[must_use]
pub const fn shaping_offered_by_default(class: BudgetClass) -> bool {
    matches!(class, BudgetClass::Server | BudgetClass::Seat)
}

/// Relay is offered by default on every owned node, except a phone only offers relay
/// while charging AND on an unmetered link (is_charging && is_unmetered_wifi) (CC §5.4.6).
#[must_use]
pub fn relay_offered_by_default(
    device_class: BudgetClass,
    is_charging: bool,
    is_unmetered_wifi: bool,
) -> bool {
    match device_class {
        BudgetClass::PhoneCellular | BudgetClass::PhoneWifi => is_charging && is_unmetered_wifi,
        BudgetClass::Home | BudgetClass::Server | BudgetClass::Seat => true,
        BudgetClass::Off => false,
    }
}

/// Relay offer gate for a device given phone flag, charging state, and unmetered Wi-Fi.
#[must_use]
pub fn relay_offered_for_device(
    is_phone: bool,
    is_charging: bool,
    is_unmetered_wifi: bool,
) -> bool {
    if is_phone {
        is_charging && is_unmetered_wifi
    } else {
        true
    }
}

/// Carriage mode on a shaped link (CC §5.4.3 carriage axis).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Carriage {
    /// Full carriage: all constitutional envelope kinds admitted.
    Full,
    /// Consent-only: admits ONLY the 16 consent-free control envelope kinds.
    ConsentOnly,
}

/// The 19 constitutional envelope kinds classified under the carriage axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EnvelopeKind {
    Cover,
    Kex,
    RouteFetch,
    KeyRecord,
    KeyRevocation,
    Occurrence,
    Allegiance,
    UnownedRoute,
    DeviceJoin,
    ContactAsk,
    ContactAnswer,
    Invite,
    InviteAnswer,
    GroupKeyWrap,
    Halt,
    Tombstone,
    PrivateRoomData,
    UserProfile,
    FederationRow,
}

impl EnvelopeKind {
    /// Whether this envelope kind is in the 16 consent-free kinds permitted on ConsentOnly carriage.
    #[must_use]
    pub const fn is_consent_free(self) -> bool {
        match self {
            Self::Cover
            | Self::Kex
            | Self::RouteFetch
            | Self::KeyRecord
            | Self::KeyRevocation
            | Self::Occurrence
            | Self::Allegiance
            | Self::UnownedRoute
            | Self::DeviceJoin
            | Self::ContactAsk
            | Self::ContactAnswer
            | Self::Invite
            | Self::InviteAnswer
            | Self::GroupKeyWrap
            | Self::Halt
            | Self::Tombstone => true,
            Self::PrivateRoomData | Self::UserProfile | Self::FederationRow => false,
        }
    }
}

impl Carriage {
    /// Check whether this carriage mode admits the given envelope kind.
    #[must_use]
    pub const fn allows(self, kind: EnvelopeKind) -> bool {
        match self {
            Self::Full => true,
            Self::ConsentOnly => kind.is_consent_free(),
        }
    }
}

/// An announced node candidate for referral when a shaped link is refused or displaced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeRelayCandidate {
    pub peer_id: String,
    pub relay_offering: bool,
    pub free_shaped_slots: usize,
    pub inbound_dial_count: u32,
    pub budget_class: BudgetClass,
}

impl NodeRelayCandidate {
    #[must_use]
    pub fn new(
        peer_id: String,
        relay_offering: bool,
        free_shaped_slots: usize,
        inbound_dial_count: u32,
        budget_class: BudgetClass,
    ) -> Self {
        Self {
            peer_id,
            relay_offering,
            free_shaped_slots,
            inbound_dial_count,
            budget_class,
        }
    }

    /// Verify whether this candidate is a shaping home or server with an available slot (CC §5.4.3).
    #[must_use]
    pub fn is_shaping_home_or_server(&self) -> bool {
        (self.budget_class == BudgetClass::Home || self.budget_class == BudgetClass::Server)
            && self.free_shaped_slots > 0
            && self.relay_offering
    }
}

/// A sealed referral carried on refusal (CLM-refusal-carries-referrals).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealedReferral {
    pub peer_id: String,
    pub free_shaped_slots: usize,
    pub inbound_dial_count: u32,
}

/// Outcome of attempting to place a shaped link.
///
/// Follows CIRISEdge v45 (92012e5), not the CC's displacement clause: there is no
/// displacement until CIRISEdge#939 lands (`src/transport/link_shaping.rs:356`, "until it
/// lands a higher-class newcomer at a full node is refused, not given a slot").
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkPlacement {
    /// Admitted directly into a free slot.
    Admitted,
    /// Past the bound on a node that is neither shaped-only nor anonymous-tier: the link
    /// is made and runs unshaped at both ends, and the referrals go out with the `off`
    /// answer as a hint (`src/transport/link_shaping.rs:3347` `past_bound_answer`;
    /// `src/transport/reticulum.rs:17215` `no_slot`, "the referrals go out either way").
    AdmittedUnshaped { referrals: Vec<SealedReferral> },
}

/// Refusal reason when a shaped link cannot be established.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShapingRefusal {
    /// Failure Mode A (reach ≠ capacity): node lacks a direct IPv6 / punched UDP path.
    UnreachableForShaping,
    /// Failure Mode B (reach ≠ capacity): node is directly reachable, but all slots are full
    /// and the candidate rank is equal to or lower than all existing peers on shaped_only node.
    CapacityExhausted { referrals: Vec<SealedReferral> },
    /// Offer resolved to BudgetClass::Off.
    ShapingOff,
}

/// Tracks shaped links on a simulated or real node, enforcing the 8-link bound (all links
/// but seat-to-seat ones count, `src/transport/link_shaping.rs:3172` `counted_links`),
/// the one-hop rule, reachability vs capacity separation, and referrals. Line citations
/// are CIRISEdge at the v45 candidate 92012e5.
#[derive(Debug, Clone)]
pub struct ShapedLinkManager {
    /// This node's identity.
    pub local_peer_id: String,
    /// Local offered budget class (CC §5.4.3).
    pub local_class: BudgetClass,
    /// Whether this node can be dialled directly (public address, IPv6, or a punched UDP
    /// path). A link is one hop when either end can be dialled by the other
    /// ([`link_is_one_hop`]); a NAT'd phone dialling a public seat is one hop.
    pub has_direct_path: bool,
    /// Whether this node holds a `serve` seat in the signed roster.
    pub holds_serve_seat: bool,
    /// Whether this node is set to shaped_only (2026-11-06 rule, required by Anonymous Tier).
    pub shaped_only: bool,
    /// Whether this node is operating under Anonymous Tier.
    pub is_anonymous_tier: bool,
    /// Active device-tier shaped links: peer_id -> budget class. Bound <= 8.
    pub active_device_links: BTreeMap<String, BudgetClass>,
    /// Active seat-tier shaped links: peer_id -> budget class. Outside the 8-link bound.
    pub active_seat_links: BTreeMap<String, BudgetClass>,
    /// Active unshaped links (9th peer overflow or bulk links).
    pub active_unshaped_links: BTreeSet<String>,
    /// Separate failure counter A: direct reachability failures.
    pub unreachable_for_shaping_count: u64,
    /// Separate failure counter B: slot capacity exhausted failures.
    pub capacity_exhausted_count: u64,
    /// Device hardware tier class (e.g. Server, Home, PhoneWifi, PhoneCellular).
    pub device_class: BudgetClass,
    /// Whether shaping offer is enabled by default or overridden (CC §5.4.3 / §5.4.6).
    pub shaping_offer_enabled: bool,
    /// Tracks asked shaped links on off nodes: peer_id -> last_real_frame_unix_secs.
    pub asked_links: BTreeMap<String, u64>,
}

impl ShapedLinkManager {
    /// Create a new link manager for a node.
    #[must_use]
    pub fn new(local_peer_id: String, has_direct_path: bool, holds_serve_seat: bool) -> Self {
        let device_class = if holds_serve_seat {
            BudgetClass::Seat
        } else {
            BudgetClass::Server
        };
        let shaping_offer_enabled = shaping_offered_by_default(device_class);
        let local_class = if shaping_offer_enabled {
            device_class
        } else {
            BudgetClass::Off
        };
        Self {
            local_peer_id,
            local_class,
            has_direct_path,
            holds_serve_seat,
            shaped_only: false,
            is_anonymous_tier: false,
            active_device_links: BTreeMap::new(),
            active_seat_links: BTreeMap::new(),
            active_unshaped_links: BTreeSet::new(),
            unreachable_for_shaping_count: 0,
            capacity_exhausted_count: 0,
            device_class,
            shaping_offer_enabled,
            asked_links: BTreeMap::new(),
        }
    }

    /// Builder setter for local class.
    #[must_use]
    pub fn with_local_class(mut self, local_class: BudgetClass) -> Self {
        self.local_class = local_class;
        self.device_class = local_class;
        self.shaping_offer_enabled = local_class != BudgetClass::Off;
        self
    }

    /// Builder setter for device hardware class.
    #[must_use]
    pub fn with_device_class(mut self, device_class: BudgetClass) -> Self {
        self.device_class = device_class;
        self.shaping_offer_enabled = shaping_offered_by_default(device_class);
        self.local_class = if self.shaping_offer_enabled {
            device_class
        } else {
            BudgetClass::Off
        };
        self
    }

    /// Builder setter for shaping offer enabled state.
    #[must_use]
    pub fn with_shaping_offer_enabled(mut self, enabled: bool) -> Self {
        self.shaping_offer_enabled = enabled;
        self.local_class = if enabled {
            self.device_class
        } else {
            BudgetClass::Off
        };
        self
    }

    /// Builder setter for shaped_only state.
    #[must_use]
    pub fn with_shaped_only(mut self, shaped_only: bool) -> Self {
        self.shaped_only = shaped_only;
        self
    }

    /// Available free device-tier shaped slots (0 ..= 8).
    #[must_use]
    pub fn free_device_slots(&self) -> usize {
        SHAPED_LINK_BOUND.saturating_sub(self.active_device_links.len())
    }

    /// Rank and filter up to `MAX_REFERRALS` (3) candidate nodes (CC §5.4.3).
    /// Candidates are filtered by `relay_offering && free_shaped_slots > 0` and sorted by
    /// `inbound_dial_count` descending (more is better — evidence of reachability, not load),
    /// tie-breaking by `peer_id`.
    #[must_use]
    pub fn rank_referrals(candidates: &[NodeRelayCandidate]) -> Vec<SealedReferral> {
        let mut eligible: Vec<SealedReferral> = candidates
            .iter()
            .filter(|c| c.relay_offering && c.free_shaped_slots > 0)
            .map(|c| SealedReferral {
                peer_id: c.peer_id.clone(),
                free_shaped_slots: c.free_shaped_slots,
                inbound_dial_count: c.inbound_dial_count,
            })
            .collect();

        eligible.sort_by(|a, b| {
            b.inbound_dial_count
                .cmp(&a.inbound_dial_count)
                .then_with(|| a.peer_id.cmp(&b.peer_id))
        });

        eligible.truncate(MAX_REFERRALS);
        eligible
    }

    fn place_shaped_link_internal(
        &mut self,
        peer_id: &str,
        agreed_class: BudgetClass,
        both_seats: bool,
        candidate_referrals: &[NodeRelayCandidate],
    ) -> Result<LinkPlacement, ShapingRefusal> {
        // Seat class bypasses 8-link device bound (CLM-seat-class).
        if agreed_class == BudgetClass::Seat {
            if both_seats {
                self.active_seat_links
                    .insert(peer_id.to_string(), agreed_class);
                return Ok(LinkPlacement::Admitted);
            }
            // If seat was requested but endpoints lack standing, refused.
            self.capacity_exhausted_count += 1;
            return Err(ShapingRefusal::CapacityExhausted {
                referrals: Self::rank_referrals(candidate_referrals),
            });
        }

        // Already holds a link to this peer: update class.
        if self.active_device_links.contains_key(peer_id) {
            self.active_device_links
                .insert(peer_id.to_string(), agreed_class);
            return Ok(LinkPlacement::Admitted);
        }

        // Room available under the 8-link bound.
        if self.active_device_links.len() < SHAPED_LINK_BOUND {
            self.active_device_links
                .insert(peer_id.to_string(), agreed_class);
            return Ok(LinkPlacement::Admitted);
        }

        // Full: no displacement in v45 (CIRISEdge#939, `src/transport/link_shaping.rs:356`).
        // Whoever holds a slot keeps it; nothing is ever taken from a running link
        // (`src/transport/link_shaping.rs:365`).

        // Ninth peer rule (CC §5.4.3 / 2026-11-06 rule):
        // A 9th shaping peer is refused ONLY on a node set to shaped_only or the Anonymous Tier;
        // on any other node the link is made and runs unshaped.
        if self.shaped_only || self.is_anonymous_tier {
            self.capacity_exhausted_count += 1;
            let referrals = Self::rank_referrals(candidate_referrals);
            Err(ShapingRefusal::CapacityExhausted { referrals })
        } else {
            self.active_unshaped_links.insert(peer_id.to_string());
            Ok(LinkPlacement::AdmittedUnshaped {
                referrals: Self::rank_referrals(candidate_referrals),
            })
        }
    }

    /// Request establishment of a shaped link with `peer_id`.
    /// Enforces:
    /// 1. One hop (Failure Mode A vs Failure Mode B): either end directly dialable
    ///    ([`link_is_one_hop`])
    /// 2. Lower of offers agreement (`agree(self.local_class, peer_class)`)
    /// 3. Seat-class bypass of the 8-link bound, seat to seat only (CLM-seat-class;
    ///    `src/transport/link_shaping.rs:2867` `is_seat_link`)
    /// 4. No displacement (CIRISEdge#939): first come keeps the slot
    /// 5. Ninth peer rule: refused only on `shaped_only` or Anonymous Tier; unshaped with
    ///    referrals otherwise
    pub fn request_shaped_link(
        &mut self,
        peer_id: &str,
        peer_class: BudgetClass,
        peer_has_direct_path: bool,
        peer_holds_serve_seat: bool,
        candidate_referrals: &[NodeRelayCandidate],
    ) -> Result<LinkPlacement, ShapingRefusal> {
        // One hop: a link is shaped unless a forwarder carries it
        // (`src/transport/reticulum.rs:534` `link_is_forwarded`, :17182 responder side).
        if !link_is_one_hop(self.has_direct_path, peer_has_direct_path) {
            self.unreachable_for_shaping_count += 1;
            return Err(ShapingRefusal::UnreachableForShaping);
        }

        let both_seats = self.holds_serve_seat && peer_holds_serve_seat;
        // Agreement takes the lower of the two offers (CC §5.4.3 item 1).
        let agreed_class = BudgetClass::agree(self.local_class, peer_class, both_seats);
        if agreed_class == BudgetClass::Off {
            return Err(ShapingRefusal::ShapingOff);
        }

        self.place_shaped_link_internal(peer_id, agreed_class, both_seats, candidate_referrals)
    }

    /// Request establishment of a shaped link via the ask path (CC §5.4.3 / §5.4.6).
    ///
    /// A node with the shaping offer off still gives a shaped link to any peer that asks,
    /// at its device class, and drops the shaped link 10 minutes (600 seconds) after the
    /// last real frame (`SHAPED_ASK_IDLE_TIMEOUT_SECS = 600`).
    pub fn request_shaped_link_asked(
        &mut self,
        peer_id: &str,
        peer_class: BudgetClass,
        peer_has_direct_path: bool,
        peer_holds_serve_seat: bool,
        candidate_referrals: &[NodeRelayCandidate],
        current_unix_secs: u64,
    ) -> Result<LinkPlacement, ShapingRefusal> {
        if !link_is_one_hop(self.has_direct_path, peer_has_direct_path) {
            self.unreachable_for_shaping_count += 1;
            return Err(ShapingRefusal::UnreachableForShaping);
        }

        // When shaping offer is off, the node gives a shaped link at its device class
        let effective_local_class = if self.local_class == BudgetClass::Off || !self.shaping_offer_enabled {
            self.device_class
        } else {
            self.local_class
        };

        let both_seats = self.holds_serve_seat && peer_holds_serve_seat;
        let agreed_class = BudgetClass::agree(effective_local_class, peer_class, both_seats);
        if agreed_class == BudgetClass::Off {
            return Err(ShapingRefusal::ShapingOff);
        }

        let placement = self.place_shaped_link_internal(peer_id, agreed_class, both_seats, candidate_referrals)?;
        if self.local_class == BudgetClass::Off || !self.shaping_offer_enabled {
            self.asked_links.insert(peer_id.to_string(), current_unix_secs);
        }
        Ok(placement)
    }

    /// Record receipt or departure of a real payload frame on an asked shaped link,
    /// resetting its 10-minute idle countdown (CC §5.4.3 / §5.4.6).
    pub fn record_real_frame(&mut self, peer_id: &str, current_unix_secs: u64) {
        if self.asked_links.contains_key(peer_id) {
            self.asked_links.insert(peer_id.to_string(), current_unix_secs);
        }
    }

    /// Check and drop all asked shaped links that have been idle for >= 10 minutes (600 seconds)
    /// since their last real frame (CC §5.4.3 / §5.4.6, `SHAPED_ASK_IDLE_TIMEOUT_SECS = 600`).
    /// Returns the list of peer IDs whose shaped links were dropped due to idle timeout.
    pub fn check_idle_drops(&mut self, current_unix_secs: u64) -> Vec<String> {
        let mut dropped = Vec::new();
        for (peer_id, last_secs) in &self.asked_links {
            if current_unix_secs.saturating_sub(*last_secs) >= SHAPED_ASK_IDLE_TIMEOUT_SECS {
                dropped.push(peer_id.clone());
            }
        }

        for peer_id in &dropped {
            self.active_device_links.remove(peer_id);
            self.asked_links.remove(peer_id);
        }

        dropped
    }

    /// Alias for `check_idle_drops`.
    pub fn check_idle_timeout(&mut self, current_unix_secs: u64) -> Vec<String> {
        self.check_idle_drops(current_unix_secs)
    }
}

/// CC 5.4.3 `CLM-shaped-link-one-hop` as CIRISEdge v45 reads it: a link is forwarded only
/// when it has no direct interface AND the path is more than one hop
/// (`src/transport/reticulum.rs:534` `link_is_forwarded`). A node that cannot be dialled
/// (NAT'd, no punch) reaches a dialable one directly, so the link is one hop when EITHER
/// end can be dialled. Two NAT'd ends with no punch go through a forwarder: unshaped.
#[must_use]
pub const fn link_is_one_hop(a_dialable: bool, b_dialable: bool) -> bool {
    a_dialable || b_dialable
}

/// Predicate checking if an asked link is expired after 10 minutes (600s) of inactivity (CC §5.4.3 / §5.4.6).
#[must_use]
pub fn is_asked_link_expired(last_frame_unix_secs: u64, current_unix_secs: u64) -> bool {
    current_unix_secs.saturating_sub(last_frame_unix_secs) >= SHAPED_ASK_IDLE_TIMEOUT_SECS
}

/// Model the 2026-11-06 rule for traffic routing over shaped links:
/// Chat and files ride a shaped link only when one end is shaped_only;
/// every other pair's chat crosses a direct unshaped link.
#[must_use]
pub fn routes_on_shaped_link(local_shaped_only: bool, peer_shaped_only: bool) -> bool {
    local_shaped_only || peer_shaped_only
}

/// Constitutional check for payload size on shaped links (CC §5.3.3.2).
///
/// Any payload <= 65,536 B (64 KiB) may cross a shaped link.
/// Any payload > 65,536 B is refused whole, never chunked over shaped frames,
/// returning the exact constitutional refusal string.
pub fn check_shaped_payload(size_bytes: usize) -> Result<(), &'static str> {
    if size_bytes <= SHAPED_PAYLOAD_LIMIT_BYTES {
        Ok(())
    } else {
        Err(REFUSAL_PAYLOAD_TOO_LARGE)
    }
}

/// Validate holding claim token: `custody:ack:v1` is accepted; retired `holds_bytes` is refused (CC §3.1.3.3).
pub fn validate_holding_claim_token(token: &str) -> Result<&'static str, &'static str> {
    if token.starts_with(RETIRED_HOLDS_BYTES_PREFIX) {
        Err("RefusedRetiredHoldsBytes")
    } else if token.starts_with(CUSTODY_ACK_V1_PREFIX) {
        Ok(CUSTODY_ACK_V1_PREFIX)
    } else {
        Err("UnknownHoldingClaimToken")
    }
}

/// Replication tier of a row in the federation storage hierarchy (CC §5.4.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReplicationTier {
    /// Row replicates across the federation and is eligible for proxy pull.
    FederationTier,
    /// Row is local to this node/host only and is NEVER proxied.
    LocalTier,
}

/// Proxy pull refusal reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProxyPullRowRefusal {
    RefusedNonFederationTier,
}

/// Request for a row through a proxy pull.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyRowRequest {
    pub row_id: String,
    pub tier: ReplicationTier,
}

/// Simulates a proxy pull (CLM-referred-peer-is-proxy, CC §5.4.6).
/// When an asker connects to a referred peer, that referred peer serves what it holds
/// and pulls missing federation rows from canonical seat as its own pull (`proxy`),
/// so the seat attributes the pull to the proxy rather than the asker.
/// A row that is not federation-tier is NEVER proxied (`RefusedNonFederationTier`).
#[derive(Debug, Clone)]
pub struct ProxyPullResult {
    pub asker_peer_id: String,
    pub proxy_peer_id: String,
    pub canonical_seat_id: String,
    pub rows_served_local: usize,
    pub rows_proxied_from_seat: usize,
    pub rows_refused_non_federation: usize,
}

pub fn execute_proxy_pull(
    asker: &str,
    proxy: &str,
    canonical_seat: &str,
    requested_rows: &[ProxyRowRequest],
    proxy_held_rows: &[String],
) -> ProxyPullResult {
    let mut local = 0;
    let mut proxied = 0;
    let mut refused = 0;
    for row in requested_rows {
        // CC §5.4.6: a row that is not federation-tier is never proxied
        if row.tier != ReplicationTier::FederationTier {
            refused += 1;
        } else if proxy_held_rows.contains(&row.row_id) {
            local += 1;
        } else {
            proxied += 1;
        }
    }
    ProxyPullResult {
        asker_peer_id: asker.to_string(),
        proxy_peer_id: proxy.to_string(),
        canonical_seat_id: canonical_seat.to_string(),
        rows_served_local: local,
        rows_proxied_from_seat: proxied,
        rows_refused_non_federation: refused,
    }
}

pub fn execute_proxy_pull_simple(
    asker: &str,
    proxy: &str,
    canonical_seat: &str,
    requested_rows: &[String],
    proxy_held_rows: &[String],
) -> ProxyPullResult {
    let requests: Vec<ProxyRowRequest> = requested_rows
        .iter()
        .map(|r| ProxyRowRequest {
            row_id: r.clone(),
            tier: ReplicationTier::FederationTier,
        })
        .collect();
    execute_proxy_pull(asker, proxy, canonical_seat, &requests, proxy_held_rows)
}

/// Anonymous tier verification: ensures that at least 3 answering relays exist per layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnonymousTierError {
    BelowAnsweringFloor { answering: usize, floor: usize },
    OnionTierUnavailableFloor { answering: usize, floor: usize },
}

pub fn check_onion_tier_availability(
    answering_relays_per_layer: usize,
) -> Result<(), AnonymousTierError> {
    if answering_relays_per_layer >= ONION_TIER_FLOOR_RELAYS {
        Ok(())
    } else {
        Err(AnonymousTierError::OnionTierUnavailableFloor {
            answering: answering_relays_per_layer,
            floor: ONION_TIER_FLOOR_RELAYS,
        })
    }
}

/// Fallback guard of last resort when the sender has no consented contact offering relay (CC §5.4.6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuardFallback {
    ServeSeat(String),
}

/// A guard slot tracking one contact or fallback serving as hop-one guard (CC §5.4.6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardSlot {
    pub peer_id: String,
    pub reachable: bool,
    pub is_consented_contact: bool,
    pub offers_relay: bool,
}

/// Candidate contact for sticky guard selection (CC §5.4.6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsentedContactGuardCandidate {
    pub peer_id: String,
    pub is_consented: bool,
    pub offers_relay: bool,
    pub reachable: bool,
}

impl ConsentedContactGuardCandidate {
    #[must_use]
    pub fn new(peer_id: impl Into<String>, is_consented: bool, offers_relay: bool, reachable: bool) -> Self {
        Self {
            peer_id: peer_id.into(),
            is_consented,
            offers_relay,
            reachable,
        }
    }
}

/// Sticky guard (CC §5.4.6 item 4): kept as long as it answers, never rotated on a schedule.
///
/// Per CC §5.4.6: hop one is a sticky guard chosen from the sender's own consented contacts
/// that offer relay, two of them for reach (`primary_guard_peer_id` and `secondary_guard_peer_id`);
/// a sender with no such contact uses a serve seat as its guard of last resort (`GuardFallback::ServeSeat`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StickyGuard {
    /// Active / legacy primary guard peer ID.
    pub guard_peer_id: String,
    /// Active / legacy primary guard reachability.
    pub guard_reachable: bool,
    /// Primary consented-contact guard peer ID for reach (CC §5.4.6).
    pub primary_guard_peer_id: Option<String>,
    /// Secondary consented-contact guard peer ID for reach (CC §5.4.6).
    pub secondary_guard_peer_id: Option<String>,
    /// Guard fallback of last resort if sender has no consented contact offering relay (CC §5.4.6).
    pub fallback: Option<GuardFallback>,
    /// Guard slots (up to 2 consented-contact guards for reach, or serve-seat fallback).
    pub guard_slots: Vec<GuardSlot>,
}

impl StickyGuard {
    /// Backward-compatible constructor with single guard peer.
    #[must_use]
    pub fn new(guard_peer_id: String, guard_reachable: bool) -> Self {
        Self {
            guard_peer_id: guard_peer_id.clone(),
            guard_reachable,
            primary_guard_peer_id: Some(guard_peer_id.clone()),
            secondary_guard_peer_id: None,
            fallback: None,
            guard_slots: vec![GuardSlot {
                peer_id: guard_peer_id,
                reachable: guard_reachable,
                is_consented_contact: true,
                offers_relay: true,
            }],
        }
    }

    /// Construct with two consented-contact guards for reach (CC §5.4.6).
    #[must_use]
    pub fn two_guards(
        primary_peer_id: String,
        secondary_peer_id: String,
        primary_reachable: bool,
        secondary_reachable: bool,
    ) -> Self {
        let any_reachable = primary_reachable || secondary_reachable;
        let active = if primary_reachable {
            primary_peer_id.clone()
        } else if secondary_reachable {
            secondary_peer_id.clone()
        } else {
            primary_peer_id.clone()
        };
        Self {
            guard_peer_id: active,
            guard_reachable: any_reachable,
            primary_guard_peer_id: Some(primary_peer_id.clone()),
            secondary_guard_peer_id: Some(secondary_peer_id.clone()),
            fallback: None,
            guard_slots: vec![
                GuardSlot {
                    peer_id: primary_peer_id,
                    reachable: primary_reachable,
                    is_consented_contact: true,
                    offers_relay: true,
                },
                GuardSlot {
                    peer_id: secondary_peer_id,
                    reachable: secondary_reachable,
                    is_consented_contact: true,
                    offers_relay: true,
                },
            ],
        }
    }

    /// Construct with serve seat as guard of last resort (CC §5.4.6).
    #[must_use]
    pub fn with_serve_seat_fallback(seat_peer_id: String, seat_reachable: bool) -> Self {
        Self {
            guard_peer_id: seat_peer_id.clone(),
            guard_reachable: seat_reachable,
            primary_guard_peer_id: None,
            secondary_guard_peer_id: None,
            fallback: Some(GuardFallback::ServeSeat(seat_peer_id.clone())),
            guard_slots: vec![GuardSlot {
                peer_id: seat_peer_id,
                reachable: seat_reachable,
                is_consented_contact: false,
                offers_relay: true,
            }],
        }
    }

    /// Select hop-one sticky guards per CC §5.4.6:
    /// Hop one is chosen from the sender's own consented contacts that offer relay,
    /// two of them for reach; a sender with no such contact uses a serve seat as its guard of last resort.
    #[must_use]
    pub fn select_from_contacts(
        contacts: &[ConsentedContactGuardCandidate],
        fallback_serve_seat: Option<String>,
    ) -> Self {
        let eligible: Vec<&ConsentedContactGuardCandidate> = contacts
            .iter()
            .filter(|c| c.is_consented && c.offers_relay)
            .collect();

        if eligible.is_empty() {
            if let Some(seat) = fallback_serve_seat {
                return Self::with_serve_seat_fallback(seat, true);
            }
            return Self {
                guard_peer_id: String::new(),
                guard_reachable: false,
                primary_guard_peer_id: None,
                secondary_guard_peer_id: None,
                fallback: None,
                guard_slots: Vec::new(),
            };
        }

        if eligible.len() == 1 {
            let mut g = Self::new(eligible[0].peer_id.clone(), eligible[0].reachable);
            if let Some(seat) = fallback_serve_seat {
                g.fallback = Some(GuardFallback::ServeSeat(seat));
            }
            return g;
        }

        let mut g = Self::two_guards(
            eligible[0].peer_id.clone(),
            eligible[1].peer_id.clone(),
            eligible[0].reachable,
            eligible[1].reachable,
        );
        if let Some(seat) = fallback_serve_seat {
            g.fallback = Some(GuardFallback::ServeSeat(seat));
        }
        g
    }

    /// CC §5.4.6 item 4: a guard is kept as long as it answers, never rotated on a schedule.
    /// If two guards are configured for reach, retention holds while at least one answers.
    #[must_use]
    pub fn should_retain(&self) -> bool {
        if !self.guard_reachable {
            return false;
        }
        if self.guard_slots.is_empty() {
            return self.guard_reachable;
        }
        self.guard_slots.iter().any(|s| s.reachable)
    }

    /// Whether this sticky guard is currently using the serve-seat fallback of last resort.
    #[must_use]
    pub fn is_using_serve_seat_fallback(&self) -> bool {
        self.primary_guard_peer_id.is_none() && matches!(self.fallback, Some(GuardFallback::ServeSeat(_)))
    }

    /// Returns the currently active reachable guard peer ID for routing.
    #[must_use]
    pub fn active_guard_peer_id(&self) -> Option<&str> {
        for slot in &self.guard_slots {
            if slot.reachable {
                return Some(&slot.peer_id);
            }
        }
        if self.guard_reachable && !self.guard_peer_id.is_empty() {
            Some(&self.guard_peer_id)
        } else {
            None
        }
    }
}

/// Private room mailbox checker: verifies 2 member-owned nodes or warns on seat fallback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoomMailboxStatus {
    DualMemberHeld,
    ServeSeatFallback { warning_secs: u64 },
    InsufficientMembers,
}

pub fn evaluate_room_mailbox(
    member_nodes_available: usize,
    using_serve_seat: bool,
    both_ends_awake_secs: u64,
) -> RoomMailboxStatus {
    if member_nodes_available >= ROOM_MAILBOX_MEMBERS {
        RoomMailboxStatus::DualMemberHeld
    } else if using_serve_seat {
        RoomMailboxStatus::ServeSeatFallback {
            warning_secs: both_ends_awake_secs,
        }
    } else {
        RoomMailboxStatus::InsufficientMembers
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_class_properties_and_agreement() {
        assert_eq!(BudgetClass::Off.rank(), 0);
        assert_eq!(BudgetClass::PhoneCellular.rank(), 1);
        assert_eq!(BudgetClass::PhoneWifi.rank(), 2);
        assert_eq!(BudgetClass::Home.rank(), 3);
        assert_eq!(BudgetClass::Server.rank(), 4);
        assert_eq!(BudgetClass::Seat.rank(), 5);

        assert_eq!(BudgetClass::PhoneCellular.default_bytes_per_s(), 350);
        assert_eq!(BudgetClass::PhoneCellular.real_share_bytes_per_s(), 175);
        assert_eq!(BudgetClass::PhoneCellular.window_secs(), 3600);

        assert_eq!(BudgetClass::Home.default_bytes_per_s(), 2800);
        assert_eq!(BudgetClass::Home.real_share_bytes_per_s(), 1400);
        assert_eq!(BudgetClass::Home.window_secs(), 600);

        // Agreement takes minimum rank (lower of offers)
        assert_eq!(
            BudgetClass::agree(BudgetClass::Home, BudgetClass::PhoneWifi, false),
            BudgetClass::PhoneWifi
        );
        // Off overrides everything
        assert_eq!(
            BudgetClass::agree(BudgetClass::Server, BudgetClass::Off, true),
            BudgetClass::Off
        );
        // Seat valid only when both endpoints hold serve seat
        assert_eq!(
            BudgetClass::agree(BudgetClass::Seat, BudgetClass::Seat, true),
            BudgetClass::Seat
        );
        assert_eq!(
            BudgetClass::agree(BudgetClass::Seat, BudgetClass::Seat, false),
            BudgetClass::Server
        );
    }

    #[test]
    fn shaped_link_bound_and_ninth_refused() {
        // Node with shaped_only = true refuses 9th peer of equal or lower rank
        let mut mgr = ShapedLinkManager::new("node-alpha".into(), true, false)
            .with_shaped_only(true);

        // Add 8 PhoneWifi links
        for i in 0..8 {
            let res = mgr.request_shaped_link(
                &format!("phone-{i}"),
                BudgetClass::PhoneWifi,
                true,
                false,
                &[],
            );
            assert_eq!(res, Ok(LinkPlacement::Admitted));
        }
        assert_eq!(mgr.free_device_slots(), 0);

        // 9th peer of equal rank (PhoneWifi) is refused on shaped_only node
        let res = mgr.request_shaped_link("phone-9", BudgetClass::PhoneWifi, true, false, &[]);
        assert!(matches!(
            res,
            Err(ShapingRefusal::CapacityExhausted { .. })
        ));
        assert_eq!(mgr.capacity_exhausted_count, 1);

        // Node with shaped_only = false accepts 9th peer as unshaped link
        let mut unshaped_mgr = ShapedLinkManager::new("node-beta".into(), true, false);
        for i in 0..8 {
            unshaped_mgr
                .request_shaped_link(
                    &format!("phone-{i}"),
                    BudgetClass::PhoneWifi,
                    true,
                    false,
                    &[],
                )
                .unwrap();
        }
        let res_unshaped = unshaped_mgr.request_shaped_link("phone-9", BudgetClass::PhoneWifi, true, false, &[]);
        assert_eq!(res_unshaped, Ok(LinkPlacement::AdmittedUnshaped { referrals: vec![] }));
        assert!(unshaped_mgr.active_unshaped_links.contains("phone-9"));
    }

    /// CIRISEdge 92012e5 `src/transport/link_shaping.rs:356`: "Displacement
    /// (`CLM-shaped-link-displacement`) is CIRISEdge#939: until it lands a higher-class
    /// newcomer at a full node is refused, not given a slot." On a node that is not
    /// shaped-only, "refused" is the `off` answer: the link runs unshaped, the referrals go
    /// out with it (`src/transport/reticulum.rs:17215`), and every held link stays shaped.
    #[test]
    fn a_higher_class_newcomer_at_a_full_node_gets_no_slot() {
        let mut mgr = ShapedLinkManager::new("server-host".into(), true, false);
        for i in 0..8 {
            mgr.request_shaped_link(&format!("cellular-{i}"), BudgetClass::PhoneCellular, true, false, &[])
                .unwrap();
        }
        let referrals = vec![NodeRelayCandidate {
            peer_id: "relay-alt".into(),
            relay_offering: true,
            free_shaped_slots: 4,
            inbound_dial_count: 10,
            budget_class: BudgetClass::Home,
        }];
        let res = mgr.request_shaped_link("home-arriving", BudgetClass::Home, true, false, &referrals);
        match res {
            Ok(LinkPlacement::AdmittedUnshaped { referrals }) => {
                assert_eq!(referrals.len(), 1);
                assert_eq!(referrals[0].peer_id, "relay-alt");
            }
            other => panic!("expected an unshaped ninth link with referrals, got {other:?}"),
        }
        assert_eq!(mgr.active_device_links.len(), 8);
        assert!(!mgr.active_device_links.contains_key("home-arriving"));
        assert!((0..8).all(|i| mgr.active_device_links.contains_key(&format!("cellular-{i}"))));
        assert!(mgr.active_unshaped_links.contains("home-arriving"));
    }

    /// CIRISEdge 92012e5 `src/transport/reticulum.rs:534` `link_is_forwarded`: a link is
    /// forwarded only with no direct interface and more than one hop. A phone behind NAT
    /// dials the seat's public interface: one hop, shaped. Two NAT'd ends with no punch go
    /// through a forwarder and run sealed and unshaped (`src/transport/reticulum.rs:13968`
    /// `offer_withheld`, :17182 on the responder).
    #[test]
    fn a_natd_device_dialling_a_public_seat_is_one_hop_and_shaped() {
        let mut seat = ShapedLinkManager::new("seat-1".into(), true, true);
        let res = seat.request_shaped_link("phone-natd", BudgetClass::PhoneWifi, false, false, &[]);
        assert_eq!(res, Ok(LinkPlacement::Admitted));
        assert_eq!(seat.active_device_links.get("phone-natd"), Some(&BudgetClass::PhoneWifi));
        assert_eq!(seat.unreachable_for_shaping_count, 0);

        let mut natd_home = ShapedLinkManager::new("home-natd".into(), false, false)
            .with_local_class(BudgetClass::Home);
        let res = natd_home.request_shaped_link("phone-natd", BudgetClass::PhoneWifi, false, false, &[]);
        assert_eq!(res, Err(ShapingRefusal::UnreachableForShaping));
        assert!(link_is_one_hop(true, false) && link_is_one_hop(false, true));
        assert!(!link_is_one_hop(false, false));
    }

    /// CIRISEdge 92012e5 `src/transport/link_shaping.rs:2867` `is_seat_link` (own seat AND
    /// peer seat) and :3172 `counted_links` ("all but `seat` links"): a seat's links to
    /// devices count within its eight; only seat-to-seat links sit outside. The ninth
    /// device that offers rides unshaped; a second seat still gets its seat link.
    #[test]
    fn a_seat_counts_its_device_links_in_the_eight() {
        let mut seat = ShapedLinkManager::new("seat-1".into(), true, true);
        for i in 0..8 {
            let r = seat.request_shaped_link(&format!("dev-{i}"), BudgetClass::PhoneWifi, false, false, &[]);
            assert_eq!(r, Ok(LinkPlacement::Admitted));
        }
        assert_eq!(seat.free_device_slots(), 0);
        let ninth = seat.request_shaped_link("dev-8", BudgetClass::Server, true, false, &[]);
        assert!(matches!(ninth, Ok(LinkPlacement::AdmittedUnshaped { .. })));
        let peer_seat = seat.request_shaped_link("seat-2", BudgetClass::Seat, true, true, &[]);
        assert_eq!(peer_seat, Ok(LinkPlacement::Admitted));
        assert_eq!(seat.active_seat_links.len(), 1);
        assert_eq!(seat.active_device_links.len(), 8);
    }

    /// CIRISEdge 92012e5 `src/transport/reticulum.rs:14038`: the dialler sends no offer
    /// when its policy does not shape toward the peer, and a responder only ever answers
    /// an offer (:17182-17213). A phone or home at its fleet default (shaping opt-in, off;
    /// `src/transport/link_shaping.rs:429` `FLEET_DEFAULTS`) takes no slot at the seat.
    #[test]
    fn a_device_that_offers_nothing_takes_no_slot() {
        let mut seat = ShapedLinkManager::new("seat-1".into(), true, true);
        let r = seat.request_shaped_link("phone-default", BudgetClass::Off, false, false, &[]);
        assert_eq!(r, Err(ShapingRefusal::ShapingOff));
        assert_eq!(seat.free_device_slots(), SHAPED_LINK_BOUND);
    }

    #[test]
    fn phone_does_not_displace_a_phone_and_lower_never_displaces() {
        let mut mgr = ShapedLinkManager::new("host".into(), true, false)
            .with_shaped_only(true);

        // Fill with 8 PhoneWifi peers (rank 2)
        for i in 0..8 {
            mgr.request_shaped_link(&format!("p-{i}"), BudgetClass::PhoneWifi, true, false, &[])
                .unwrap();
        }

        // Arriving PhoneCellular (rank 1 < rank 2) cannot displace
        let res_lower = mgr.request_shaped_link(
            "p-cellular-new",
            BudgetClass::PhoneCellular,
            true,
            false,
            &[],
        );
        assert!(matches!(
            res_lower,
            Err(ShapingRefusal::CapacityExhausted { .. })
        ));

        // Arriving PhoneWifi (rank 2 == rank 2) cannot displace
        let res_equal =
            mgr.request_shaped_link("p-wifi-new", BudgetClass::PhoneWifi, true, false, &[]);
        assert!(matches!(
            res_equal,
            Err(ShapingRefusal::CapacityExhausted { .. })
        ));
    }

    #[test]
    fn seat_bypasses_eight_link_bound() {
        let mut mgr = ShapedLinkManager::new("seat-host".into(), true, true);

        // Fill 8 device links
        for i in 0..8 {
            mgr.request_shaped_link(&format!("device-{i}"), BudgetClass::Home, true, false, &[])
                .unwrap();
        }
        assert_eq!(mgr.active_device_links.len(), 8);

        // Connect a seat link when both hold serve seat: succeeds without displacing device links!
        let seat_res =
            mgr.request_shaped_link("seat-peer-1", BudgetClass::Seat, true, true, &[]);
        assert_eq!(seat_res, Ok(LinkPlacement::Admitted));
        assert_eq!(mgr.active_device_links.len(), 8);
        assert_eq!(mgr.active_seat_links.len(), 1);
    }

    #[test]
    fn reach_not_equal_capacity_separate_counters() {
        let mut mgr = ShapedLinkManager::new("host".into(), true, false)
            .with_shaped_only(true);

        // Two NAT'd ends with no punch: the link goes through a forwarder.
        let mut natd = ShapedLinkManager::new("natd-host".into(), false, false).with_shaped_only(true);
        let res1 = natd.request_shaped_link("nat-peer", BudgetClass::Home, false, false, &[]);
        assert_eq!(res1, Err(ShapingRefusal::UnreachableForShaping));
        assert_eq!(natd.unreachable_for_shaping_count, 1);
        // A NAT'd peer dialling this reachable host is one hop and takes a slot.
        let res1 = mgr.request_shaped_link("nat-peer", BudgetClass::Home, false, false, &[]);
        assert_eq!(res1, Ok(LinkPlacement::Admitted));
        assert_eq!(mgr.unreachable_for_shaping_count, 0);
        assert_eq!(mgr.capacity_exhausted_count, 0);

        // Fill the other seven slots
        for i in 0..7 {
            mgr.request_shaped_link(&format!("d-{i}"), BudgetClass::Server, true, false, &[])
                .unwrap();
        }

        // Attempt from reachable peer when capacity is full
        let res2 = mgr.request_shaped_link("reachable-peer", BudgetClass::Home, true, false, &[]);
        assert!(matches!(
            res2,
            Err(ShapingRefusal::CapacityExhausted { .. })
        ));
        assert_eq!(mgr.unreachable_for_shaping_count, 0);
        assert_eq!(natd.capacity_exhausted_count, 0);
        assert_eq!(mgr.capacity_exhausted_count, 1);
    }

    #[test]
    fn refusal_carries_up_to_three_sealed_referrals() {
        let candidates = vec![
            NodeRelayCandidate {
                peer_id: "r1".into(),
                relay_offering: true,
                free_shaped_slots: 2,
                inbound_dial_count: 5,
                budget_class: BudgetClass::Home,
            },
            NodeRelayCandidate {
                peer_id: "r2".into(),
                relay_offering: false, // not offering
                free_shaped_slots: 8,
                inbound_dial_count: 0,
                budget_class: BudgetClass::Server,
            },
            NodeRelayCandidate {
                peer_id: "r3".into(),
                relay_offering: true,
                free_shaped_slots: 7,
                inbound_dial_count: 2,
                budget_class: BudgetClass::Home,
            },
            NodeRelayCandidate {
                peer_id: "r4".into(),
                relay_offering: true,
                free_shaped_slots: 5,
                inbound_dial_count: 1,
                budget_class: BudgetClass::Server,
            },
            NodeRelayCandidate {
                peer_id: "r5".into(),
                relay_offering: true,
                free_shaped_slots: 4,
                inbound_dial_count: 10,
                budget_class: BudgetClass::Home,
            },
        ];

        // Sorts by inbound_dial_count descending (more is better reachability), tie-break by peer_id
        let ranked = ShapedLinkManager::rank_referrals(&candidates);
        assert_eq!(ranked.len(), 3);
        assert_eq!(ranked[0].peer_id, "r5"); // 10 dials
        assert_eq!(ranked[1].peer_id, "r1"); // 5 dials
        assert_eq!(ranked[2].peer_id, "r3"); // 2 dials
    }

    #[test]
    fn shaped_link_payload_boundary_sixty_four_kib() {
        assert_eq!(check_shaped_payload(100), Ok(()));
        assert_eq!(check_shaped_payload(65_536), Ok(()));

        let over = check_shaped_payload(65_537);
        assert_eq!(over, Err(REFUSAL_PAYLOAD_TOO_LARGE));
        assert_eq!(
            REFUSAL_PAYLOAD_TOO_LARGE,
            "This file is too large to access over shaped links"
        );
    }

    #[test]
    fn carriage_axis_and_sixteen_consent_free_kinds() {
        let consent_only = Carriage::ConsentOnly;
        let full = Carriage::Full;

        let consent_free_kinds = [
            EnvelopeKind::Cover,
            EnvelopeKind::Kex,
            EnvelopeKind::RouteFetch,
            EnvelopeKind::KeyRecord,
            EnvelopeKind::KeyRevocation,
            EnvelopeKind::Occurrence,
            EnvelopeKind::Allegiance,
            EnvelopeKind::UnownedRoute,
            EnvelopeKind::DeviceJoin,
            EnvelopeKind::ContactAsk,
            EnvelopeKind::ContactAnswer,
            EnvelopeKind::Invite,
            EnvelopeKind::InviteAnswer,
            EnvelopeKind::GroupKeyWrap,
            EnvelopeKind::Halt,
            EnvelopeKind::Tombstone,
        ];

        assert_eq!(consent_free_kinds.len(), 16);
        for kind in &consent_free_kinds {
            assert!(kind.is_consent_free());
            assert!(consent_only.allows(*kind));
            assert!(full.allows(*kind));
        }

        let non_consent_free = [
            EnvelopeKind::PrivateRoomData,
            EnvelopeKind::UserProfile,
            EnvelopeKind::FederationRow,
        ];
        for kind in &non_consent_free {
            assert!(!kind.is_consent_free());
            assert!(!consent_only.allows(*kind));
            assert!(full.allows(*kind));
        }
    }

    #[test]
    fn anonymous_tier_floor_and_room_mailbox() {
        assert_eq!(check_onion_tier_availability(3), Ok(()));
        assert_eq!(check_onion_tier_availability(5), Ok(()));
        assert_eq!(
            check_onion_tier_availability(2),
            Err(AnonymousTierError::OnionTierUnavailableFloor {
                answering: 2,
                floor: 3,
            })
        );

        assert_eq!(CONP_FRAME_SIZE, 15_296);
        assert_eq!(CONP_EXTRA_BYTES, 0);

        assert_eq!(
            evaluate_room_mailbox(2, false, 0),
            RoomMailboxStatus::DualMemberHeld
        );
        assert_eq!(
            evaluate_room_mailbox(1, true, 7200),
            RoomMailboxStatus::ServeSeatFallback {
                warning_secs: ROOM_MAILBOX_AWAKE_WARNING_SECS,
            }
        );

        // Sticky guards: kept as long as they answer, never rotated on a schedule
        let guard_active = StickyGuard::new("guard-1".into(), true);
        assert!(guard_active.should_retain());
        let guard_dead = StickyGuard::new("guard-2".into(), false);
        assert!(!guard_dead.should_retain());
    }

    #[test]
    fn proxy_pull_federation_tier_check() {
        let requests = vec![
            ProxyRowRequest {
                row_id: "fed-row-1".into(),
                tier: ReplicationTier::FederationTier,
            },
            ProxyRowRequest {
                row_id: "local-row-2".into(),
                tier: ReplicationTier::LocalTier,
            },
            ProxyRowRequest {
                row_id: "fed-row-3".into(),
                tier: ReplicationTier::FederationTier,
            },
        ];
        let held_rows = vec!["fed-row-1".to_string()];
        let res = execute_proxy_pull("asker", "proxy", "seat", &requests, &held_rows);
        assert_eq!(res.rows_served_local, 1);
        assert_eq!(res.rows_proxied_from_seat, 1);
        assert_eq!(res.rows_refused_non_federation, 1);
    }

    #[test]
    fn two_sticky_guards_for_reach_and_serve_seat_fallback() {
        // Per CC §5.4.6: hop one is chosen from sender's consented contacts offering relay,
        // two of them for reach; sender with no such contact uses serve seat as guard of last resort.
        let contacts = vec![
            ConsentedContactGuardCandidate::new("friend-1", true, true, true),
            ConsentedContactGuardCandidate::new("friend-2", true, true, true),
            ConsentedContactGuardCandidate::new("stranger-3", false, true, true), // not consented
            ConsentedContactGuardCandidate::new("friend-4", true, false, true),   // no relay
        ];

        // 1. Two consented-contact guards chosen for reach
        let mut guard = StickyGuard::select_from_contacts(&contacts, Some("seat-last-resort".into()));
        assert_eq!(guard.primary_guard_peer_id.as_deref(), Some("friend-1"));
        assert_eq!(guard.secondary_guard_peer_id.as_deref(), Some("friend-2"));
        assert_eq!(guard.guard_slots.len(), 2);
        assert!(!guard.is_using_serve_seat_fallback());
        assert!(guard.should_retain());
        assert_eq!(guard.active_guard_peer_id(), Some("friend-1"));

        // If primary guard fails, secondary guard maintains reach
        guard.guard_slots[0].reachable = false;
        assert!(guard.should_retain(), "reach maintained by secondary guard");
        assert_eq!(guard.active_guard_peer_id(), Some("friend-2"));

        // When both fail, retention ends
        guard.guard_slots[1].reachable = false;
        guard.guard_reachable = false;
        assert!(!guard.should_retain());
        assert_eq!(guard.active_guard_peer_id(), None);

        // 2. Sender with NO consented contact offering relay falls back to serve seat of last resort
        let empty_contacts: Vec<ConsentedContactGuardCandidate> = Vec::new();
        let fallback_guard = StickyGuard::select_from_contacts(&empty_contacts, Some("seat-alpha".into()));
        assert!(fallback_guard.is_using_serve_seat_fallback());
        assert_eq!(fallback_guard.primary_guard_peer_id, None);
        assert_eq!(fallback_guard.secondary_guard_peer_id, None);
        assert_eq!(
            fallback_guard.fallback,
            Some(GuardFallback::ServeSeat("seat-alpha".into()))
        );
        assert_eq!(fallback_guard.active_guard_peer_id(), Some("seat-alpha"));
        assert!(fallback_guard.should_retain());
    }

    #[test]
    fn shaping_offered_by_default_and_ten_minute_idle_drop() {
        // 1. Shaping default: true for Server, false elsewhere (Home, PhoneWifi, PhoneCellular, Off)
        assert!(shaping_offered_by_default(BudgetClass::Server));
        assert!(BudgetClass::Server.shaping_offered_by_default());
        assert!(!shaping_offered_by_default(BudgetClass::Home));
        assert!(!BudgetClass::Home.shaping_offered_by_default());
        assert!(!shaping_offered_by_default(BudgetClass::PhoneWifi));
        assert!(!BudgetClass::PhoneWifi.shaping_offered_by_default());
        assert!(!shaping_offered_by_default(BudgetClass::PhoneCellular));
        assert!(!BudgetClass::PhoneCellular.shaping_offered_by_default());
        assert!(!shaping_offered_by_default(BudgetClass::Off));

        // 2. Off node ask path: a node with shaping offer off still gives a shaped link
        // to any peer that asks, at its device class, and drops it 10 minutes (600s) after the last real frame.
        let mut home_node = ShapedLinkManager::new("home-node".into(), true, false)
            .with_device_class(BudgetClass::Home);
        assert_eq!(home_node.local_class, BudgetClass::Off);
        assert!(!home_node.shaping_offer_enabled);

        // Normal offer results in ShapingOff because offer is off by default
        let unasked = home_node.request_shaped_link("peer-1", BudgetClass::Server, true, false, &[]);
        assert_eq!(unasked, Err(ShapingRefusal::ShapingOff));

        // Ask path at t = 1_000: gives shaped link at device class (Home, 2,800 B/s)
        let placement = home_node.request_shaped_link_asked(
            "peer-1",
            BudgetClass::Server,
            true,
            false,
            &[],
            1_000,
        );
        assert_eq!(placement, Ok(LinkPlacement::Admitted));
        assert_eq!(home_node.active_device_links.get("peer-1"), Some(&BudgetClass::Home));
        assert_eq!(home_node.asked_links.get("peer-1"), Some(&1_000));

        // At t = 1_599 (599 seconds later, < 600s), idle check does NOT drop
        let dropped = home_node.check_idle_drops(1_599);
        assert!(dropped.is_empty());
        assert!(home_node.active_device_links.contains_key("peer-1"));

        // Receive real frame at t = 1_500, refreshing countdown
        home_node.record_real_frame("peer-1", 1_500);
        assert_eq!(home_node.asked_links.get("peer-1"), Some(&1_500));

        // At t = 2_000 (500s after last real frame), still active
        let dropped = home_node.check_idle_drops(2_000);
        assert!(dropped.is_empty());

        // At t = 2_100 (600s after last real frame, >= SHAPED_ASK_IDLE_TIMEOUT_SECS = 600),
        // idle timeout expires and shaped link is dropped
        assert!(is_asked_link_expired(1_500, 2_100));
        let dropped = home_node.check_idle_drops(2_100);
        assert_eq!(dropped, vec!["peer-1".to_string()]);
        assert!(!home_node.active_device_links.contains_key("peer-1"));
        assert!(!home_node.asked_links.contains_key("peer-1"));
    }

    #[test]
    fn phone_relay_offer_charging_and_unmetered_gate() {
        // Relay offered by default on every owned node (Server, Home, Seat)
        assert!(relay_offered_by_default(BudgetClass::Server, false, false));
        assert!(relay_offered_by_default(BudgetClass::Home, false, false));
        assert!(relay_offered_by_default(BudgetClass::Seat, false, false));
        assert!(relay_offered_for_device(false, false, false));

        // Phone only offers relay while charging AND on an unmetered link
        assert!(!relay_offered_by_default(BudgetClass::PhoneCellular, false, false));
        assert!(!relay_offered_by_default(BudgetClass::PhoneCellular, true, false)); // charging on cellular (metered)
        assert!(!relay_offered_by_default(BudgetClass::PhoneCellular, false, true)); // unplugged
        assert!(relay_offered_by_default(BudgetClass::PhoneCellular, true, true));  // charging AND unmetered

        assert!(!relay_offered_by_default(BudgetClass::PhoneWifi, false, false));
        assert!(!relay_offered_by_default(BudgetClass::PhoneWifi, false, true));  // unmetered Wi-Fi but unplugged
        assert!(!relay_offered_by_default(BudgetClass::PhoneWifi, true, false));  // charging but metered
        assert!(relay_offered_by_default(BudgetClass::PhoneWifi, true, true));   // charging AND unmetered Wi-Fi

        assert!(!relay_offered_for_device(true, false, true));
        assert!(!relay_offered_for_device(true, true, false));
        assert!(relay_offered_for_device(true, true, true));
    }
}
