//! Constitutional Transport Contract (CC §5.4.3 & §5.4.6, Tiers.lean, claims.tsv).
//!
//! Models:
//! - 5 Named Budget Classes + Seat class (CC §5.4.3, Tiers.lean)
//! - Shaped-Link Bound (L ≤ 8, CLM-shaped-link-bound-refuse)
//! - Rank-based Displacement (CLM-shaped-link-displacement)
//! - One direct hop (CLM-shaped-link-one-hop) & separate failure counters (reach ≠ capacity)
//! - Up to 3 sealed referrals (MAX_REFERRALS = 3, CLM-refusal-carries-referrals) + proxy pull
//! - 64 KiB (65,536 B) Shaped-Link Payload Boundary & unshaped bulk/AV (CC §5.3.3.2)
//! - Carriage axis (Carriage::ConsentOnly vs Carriage::Full, CLM-link-carriage-axis) + 16 consent-free envelope kinds
//! - Anonymous Tier (CC §5.4.6) 3-hop CONP (15,296 B, +0 B cover replacement, floor ≥ 3) and 2-member room-mailbox

use std::collections::BTreeMap;

/// Maximum number of device-tier shaped links per node (Tiers.bound = 8, CC §5.4.3).
pub const SHAPED_LINK_BOUND: usize = 8;

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
    /// Rank of the budget class: higher rank displaces lowest rank on full nodes.
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

    /// Pure function resolving two offers into an agreed class.
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
}

/// Link Carriage axis: independent of BudgetClass (CLM-link-carriage-axis).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Carriage {
    /// Stranger / unconsented link: carries ONLY the 16 consent-free envelope kinds.
    ConsentOnly,
    /// Consented / room member link: carries all federation and private rows.
    Full,
}

/// The 16 consent-free envelope kinds permitted over a `ConsentOnly` stranger link
/// (CC §5.4.3, CLM-stranger-carries-consent-free).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EnvelopeKind {
    Cover = 1,
    Kex = 2,
    RouteFetch = 3,
    KeyRecord = 4,
    KeyRevocation = 5,
    Occurrence = 6,
    Allegiance = 7,
    UnownedRoute = 8,
    DeviceJoin = 9,
    ContactAsk = 10,
    ContactAnswer = 11,
    Invite = 12,
    InviteAnswer = 13,
    GroupKeyWrap = 14,
    Halt = 15,
    Tombstone = 16,
    /// Any non-consent-free data row (requires Full carriage).
    PrivateRoomData = 100,
    UserProfile = 101,
    FederationRow = 102,
}

impl EnvelopeKind {
    /// Returns true if this envelope kind is one of the 16 constitutional consent-free kinds.
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
}

/// A sealed referral carried on refusal (CLM-refusal-carries-referrals).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealedReferral {
    pub peer_id: String,
    pub free_shaped_slots: usize,
    pub inbound_dial_count: u32,
}

/// Control message delivered to a peer when it is displaced by a higher-rank arrival.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovedToShapedPeer {
    pub displaced_peer_id: String,
    pub referrals: Vec<SealedReferral>,
}

/// Outcome of attempting to place a shaped link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkPlacement {
    /// Admitted directly into a free slot.
    Admitted,
    /// Higher rank peer displaced an existing lower rank peer.
    Displaced {
        displaced_peer_id: String,
        moved_notice: MovedToShapedPeer,
    },
}

/// Refusal reason when a shaped link cannot be established.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShapingRefusal {
    /// Failure Mode A (reach ≠ capacity): node lacks a direct IPv6 / punched UDP path.
    UnreachableForShaping,
    /// Failure Mode B (reach ≠ capacity): node is directly reachable, but all slots are full
    /// and the candidate rank is equal to or lower than all existing peers.
    CapacityExhausted { referrals: Vec<SealedReferral> },
    /// Offer resolved to BudgetClass::Off.
    ShapingOff,
}

/// Tracks shaped links on a simulated or real node, enforcing the 8-link bound,
/// rank displacement, reachability vs capacity separation, and referrals.
#[derive(Debug, Clone)]
pub struct ShapedLinkManager {
    /// This node's identity.
    pub local_peer_id: String,
    /// Whether this node has a direct path (IPv6 or punched UDP).
    pub has_direct_path: bool,
    /// Whether this node holds a `serve` seat in the signed roster.
    pub holds_serve_seat: bool,
    /// Active device-tier shaped links: peer_id -> budget class. Bound <= 8.
    pub active_device_links: BTreeMap<String, BudgetClass>,
    /// Active seat-tier shaped links: peer_id -> budget class. Outside the 8-link bound.
    pub active_seat_links: BTreeMap<String, BudgetClass>,
    /// Separate failure counter A: direct reachability failures.
    pub unreachable_for_shaping_count: u64,
    /// Separate failure counter B: slot capacity exhausted failures.
    pub capacity_exhausted_count: u64,
}

impl ShapedLinkManager {
    /// Create a new link manager for a node.
    #[must_use]
    pub fn new(local_peer_id: String, has_direct_path: bool, holds_serve_seat: bool) -> Self {
        Self {
            local_peer_id,
            has_direct_path,
            holds_serve_seat,
            active_device_links: BTreeMap::new(),
            active_seat_links: BTreeMap::new(),
            unreachable_for_shaping_count: 0,
            capacity_exhausted_count: 0,
        }
    }

    /// Available free device-tier shaped slots (0 ..= 8).
    #[must_use]
    pub fn free_device_slots(&self) -> usize {
        SHAPED_LINK_BOUND.saturating_sub(self.active_device_links.len())
    }

    /// Rank and filter up to `MAX_REFERRALS` (3) candidate nodes.
    /// Only announced nodes with `relay_offering == true` and `free_shaped_slots > 0`
    /// are eligible, sorted by free slots descending, then inbound dials ascending.
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
            b.free_shaped_slots
                .cmp(&a.free_shaped_slots)
                .then_with(|| a.inbound_dial_count.cmp(&b.inbound_dial_count))
                .then_with(|| a.peer_id.cmp(&b.peer_id))
        });

        eligible.truncate(MAX_REFERRALS);
        eligible
    }

    /// Request establishment of a shaped link with `peer_id`.
    /// Enforces:
    /// 1. Direct reachability check (Failure Mode A vs Failure Mode B)
    /// 2. Seat-class bypass of the 8-link bound (CLM-seat-class)
    /// 3. Rank-based displacement if full (CLM-shaped-link-displacement)
    /// 4. Sealed referrals on refusal
    pub fn request_shaped_link(
        &mut self,
        peer_id: &str,
        peer_class: BudgetClass,
        peer_has_direct_path: bool,
        peer_holds_serve_seat: bool,
        candidate_referrals: &[NodeRelayCandidate],
    ) -> Result<LinkPlacement, ShapingRefusal> {
        // One direct hop check: both ends must have direct reachability.
        if !self.has_direct_path || !peer_has_direct_path {
            self.unreachable_for_shaping_count += 1;
            return Err(ShapingRefusal::UnreachableForShaping);
        }

        let both_seats = self.holds_serve_seat && peer_holds_serve_seat;
        let agreed_class = BudgetClass::agree(peer_class, peer_class, both_seats);
        if agreed_class == BudgetClass::Off {
            return Err(ShapingRefusal::ShapingOff);
        }

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

        // Node is full (8 device links): attempt rank-based displacement.
        // Find lowest-ranked currently held peer.
        let mut lowest_peer: Option<(String, BudgetClass)> = None;
        for (held_peer, held_class) in &self.active_device_links {
            match &lowest_peer {
                None => lowest_peer = Some((held_peer.clone(), *held_class)),
                Some((_, lowest_class)) => {
                    if held_class.rank() < lowest_class.rank() {
                        lowest_peer = Some((held_peer.clone(), *held_class));
                    }
                }
            }
        }

        if let Some((displaced_id, lowest_class)) = lowest_peer {
            // Higher rank displaces lowest rank (e.g. Home displaces PhoneCellular).
            // Phone never displaces Phone; lower rank never displaces equal/higher.
            if agreed_class.rank() > lowest_class.rank() {
                self.active_device_links.remove(&displaced_id);
                self.active_device_links
                    .insert(peer_id.to_string(), agreed_class);

                let referrals = Self::rank_referrals(candidate_referrals);
                let notice = MovedToShapedPeer {
                    displaced_peer_id: displaced_peer_id_clone(&displaced_id),
                    referrals,
                };
                return Ok(LinkPlacement::Displaced {
                    displaced_peer_id: displaced_id,
                    moved_notice: notice,
                });
            }
        }

        // Ninth link of equal or lower rank is refused.
        self.capacity_exhausted_count += 1;
        let referrals = Self::rank_referrals(candidate_referrals);
        Err(ShapingRefusal::CapacityExhausted { referrals })
    }
}

fn displaced_peer_id_clone(s: &str) -> String {
    s.to_string()
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

/// Simulates a proxy pull (CLM-referred-peer-is-proxy).
/// When an asker connects to a referred peer, that referred peer serves what it holds
/// and pulls missing federation rows from canonical seat as its own pull (`proxy`),
/// so the seat attributes the pull to the proxy rather than the asker.
#[derive(Debug, Clone)]
pub struct ProxyPullResult {
    pub asker_peer_id: String,
    pub proxy_peer_id: String,
    pub canonical_seat_id: String,
    pub rows_served_local: usize,
    pub rows_proxied_from_seat: usize,
}

pub fn execute_proxy_pull(
    asker: &str,
    proxy: &str,
    canonical_seat: &str,
    requested_rows: &[String],
    proxy_held_rows: &[String],
) -> ProxyPullResult {
    let mut local = 0;
    let mut proxied = 0;
    for row in requested_rows {
        if proxy_held_rows.contains(row) {
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
    }
}

/// Anonymous tier verification: ensures that at least 3 answering relays exist per layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnonymousTierError {
    BelowAnsweringFloor { answering: usize, floor: usize },
}

pub fn check_onion_tier_availability(
    answering_relays_per_layer: usize,
) -> Result<(), AnonymousTierError> {
    if answering_relays_per_layer >= ONION_TIER_FLOOR_RELAYS {
        Ok(())
    } else {
        Err(AnonymousTierError::BelowAnsweringFloor {
            answering: answering_relays_per_layer,
            floor: ONION_TIER_FLOOR_RELAYS,
        })
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

        // Agreement takes minimum rank
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
        let mut mgr = ShapedLinkManager::new("node-alpha".into(), true, false);

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

        // 9th peer of equal rank (PhoneWifi) is refused
        let res = mgr.request_shaped_link("phone-9", BudgetClass::PhoneWifi, true, false, &[]);
        assert!(matches!(
            res,
            Err(ShapingRefusal::CapacityExhausted { .. })
        ));
        assert_eq!(mgr.capacity_exhausted_count, 1);
    }

    #[test]
    fn home_displaces_a_phone_and_displaced_keeps_shaping() {
        let mut mgr = ShapedLinkManager::new("server-host".into(), true, false);

        // Fill with 8 PhoneCellular peers (rank 1)
        for i in 0..8 {
            mgr.request_shaped_link(
                &format!("cellular-{i}"),
                BudgetClass::PhoneCellular,
                true,
                false,
                &[],
            )
            .unwrap();
        }

        let referrals = vec![NodeRelayCandidate {
            peer_id: "relay-alt".into(),
            relay_offering: true,
            free_shaped_slots: 4,
            inbound_dial_count: 10,
        }];

        // Arriving Home node (rank 3) displaces a PhoneCellular node (rank 1)
        let res = mgr.request_shaped_link(
            "home-arriving",
            BudgetClass::Home,
            true,
            false,
            &referrals,
        );

        match res {
            Ok(LinkPlacement::Displaced {
                displaced_peer_id,
                moved_notice,
            }) => {
                assert!(displaced_peer_id.starts_with("cellular-"));
                assert_eq!(moved_notice.displaced_peer_id, displaced_peer_id);
                assert_eq!(moved_notice.referrals.len(), 1);
                assert_eq!(moved_notice.referrals[0].peer_id, "relay-alt");
            }
            other => panic!("Expected displacement, got {other:?}"),
        }

        // Manager still has exactly 8 device links
        assert_eq!(mgr.active_device_links.len(), 8);
        assert!(mgr.active_device_links.contains_key("home-arriving"));
    }

    #[test]
    fn phone_does_not_displace_a_phone_and_lower_never_displaces() {
        let mut mgr = ShapedLinkManager::new("host".into(), true, false);

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
        let mut mgr = ShapedLinkManager::new("host".into(), true, false);

        // Attempt from peer behind symmetric NAT (no direct path)
        let res1 = mgr.request_shaped_link("nat-peer", BudgetClass::Home, false, false, &[]);
        assert_eq!(res1, Err(ShapingRefusal::UnreachableForShaping));
        assert_eq!(mgr.unreachable_for_shaping_count, 1);
        assert_eq!(mgr.capacity_exhausted_count, 0);

        // Fill slots
        for i in 0..8 {
            mgr.request_shaped_link(&format!("d-{i}"), BudgetClass::Server, true, false, &[])
                .unwrap();
        }

        // Attempt from reachable peer when capacity is full
        let res2 = mgr.request_shaped_link("reachable-peer", BudgetClass::Home, true, false, &[]);
        assert!(matches!(
            res2,
            Err(ShapingRefusal::CapacityExhausted { .. })
        ));
        assert_eq!(mgr.unreachable_for_shaping_count, 1);
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
            },
            NodeRelayCandidate {
                peer_id: "r2".into(),
                relay_offering: false, // not offering
                free_shaped_slots: 8,
                inbound_dial_count: 0,
            },
            NodeRelayCandidate {
                peer_id: "r3".into(),
                relay_offering: true,
                free_shaped_slots: 7,
                inbound_dial_count: 2,
            },
            NodeRelayCandidate {
                peer_id: "r4".into(),
                relay_offering: true,
                free_shaped_slots: 5,
                inbound_dial_count: 1,
            },
            NodeRelayCandidate {
                peer_id: "r5".into(),
                relay_offering: true,
                free_shaped_slots: 4,
                inbound_dial_count: 10,
            },
        ];

        let ranked = ShapedLinkManager::rank_referrals(&candidates);
        assert_eq!(ranked.len(), 3);
        assert_eq!(ranked[0].peer_id, "r3"); // 7 free
        assert_eq!(ranked[1].peer_id, "r4"); // 5 free
        assert_eq!(ranked[2].peer_id, "r5"); // 4 free
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
            Err(AnonymousTierError::BelowAnsweringFloor {
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
    }
}
