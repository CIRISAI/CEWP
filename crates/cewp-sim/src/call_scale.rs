//! What one live call costs each member and the seat, in closed form, from the costs MEASURED
//! on CIRIS Edge v45.2.1's group calls (`FSD/twin/call_scale.md`).
//!
//! [`crate::media`] runs a call packet by packet over access links, but it carries no control
//! traffic and does not reach a thousand members. This module prices a member's uplink and
//! downlink at any room size N, term by term:
//!
//! * **offers** (C3): each member offers its own stream and every stream it relays, on its leg to
//!   each member, once a second (`OFFER_EVERY_MS`); every member relays every stream it receives,
//!   so a leg carries 1 + (N − 2) offers and a member sends (N − 1)² a second (DERIVED, edge
//!   v45.2.1 `av_call/alm.rs` :2098-2170, :53). The bytes per offer are fitted to the N = 4
//!   measurement; N = 6 is held out.
//! * **proofs**: the receiver of each Channel message proves it upstream, ~110 B (MEASURED).
//! * **announces**: O(N − 1) per member, fitted at N = 4, N = 6 held out.
//! * **media**: every member receives every other member's stream on a primary and a warm
//!   standby (audio + base, 0.69 of a stream, MODEL output of the media engine); in a tree the
//!   mean uplink equals the mean downlink (conservation), whoever carries it.
//!
//! Two code shapes: [`Shape::Today`] (edge v45.2.1 as measured) and [`Shape::Fixed`] (MODEL: the
//! 45.3 fixes, sender withholds video while backlogged and receivers drop by end-to-end age, plus
//! per-stream subscription: a member receives the `watch` streams it chooses and a presence blob
//! for up to 256 others, and offers go only to the legs it holds).
//!
//! What touches the seat is priced by [`crate::capacity`] (the calibrated ledger), not here.

use crate::capacity::{self, CTag, CostParams, Scenario, C};
use crate::seat_calibration;

/// One named number of the call-scale model.
#[derive(Debug, Clone, Copy)]
pub struct CallSpec {
    /// Name.
    pub name: &'static str,
    /// Value (the low end when the source gives a range; `hi` is the other end).
    pub value: f64,
    /// The upper end of a measured range (equal to `value` for a point).
    pub hi: f64,
    /// Unit.
    pub unit: &'static str,
    /// Class.
    pub tag: CTag,
    /// Citation or reasoning.
    pub source: &'static str,
}

const BENCH: &str = "edge v45.2.1 mesh witness with a socket-level FIFO shaper, 2026-11-25";
const MATRIX: &str = "server 4-6-node container matrix on edge v45.2.1, tc netem cap on one member's whole uplink, 2026-11-25";

// ---- MEASURED -------------------------------------------------------------------------------

/// Control (C3 offers) at the publisher, kbit/s, N = 4 (the fit point).
pub const CTRL_KBPS_N4: f64 = 14.0;
/// The same at N = 6 (held out), kbit/s.
pub const CTRL_KBPS_N6: (f64, f64) = (26.0, 38.0);
/// Proofs at the publisher, kbit/s, N = 4.
pub const PROOF_KBPS_N4: f64 = 10.0;
/// Proofs at N = 6, kbit/s.
pub const PROOF_KBPS_N6: (f64, f64) = (18.0, 20.0);
/// Announces at the publisher, kbit/s, N = 4 (the fit point).
pub const ANN_KBPS_N4: f64 = 2.5;
/// Announces at N = 6 (held out), kbit/s.
pub const ANN_KBPS_N6: (f64, f64) = (4.0, 5.0);
/// One proof upstream, bytes.
pub const PROOF_BYTES: f64 = 110.0;
/// C3 messages a member sends per second, uncapped baseline, N = 4 and N = 6.
pub const C3_SENT_BASELINE: [(f64, f64); 2] = [(4.0, 9.2), (6.0, 24.7)];
/// One media copy, kbit/s with IPv4/UDP: the bench's 30 fps stream and the matrix's codec.
pub const COPY_KBPS: (f64, f64) = (127.0, 190.0);
/// Non-media the capped member sends during a call, N = 6, 300 kbit/s cap, kbit/s.
pub const NONMEDIA_CAPPED_N6_KBPS: (f64, f64) = (130.0, 150.0);
/// Under the cap at N = 6/300: C3 sent and received per second (the stall cadence).
pub const C3_CAPPED_N6: (f64, f64) = (10.6, 16.7);
/// Offered load after the shed (one copy + control + proofs) over the cap.
pub const OFFERED_OVER_CAP: (f64, f64) = (0.96, 1.11);
/// Packets queued in the first capped second, and their drain time at the cap, seconds.
/// (The seconds are those packets as 532 B chunks at 250 kbit/s.)
pub const FIRST_SECOND_QUEUE: ((f64, f64), (f64, f64)) = ((260.0, 440.0), (4.4, 7.5));
/// When the congestion verdict fires after the cap, seconds.
pub const VERDICT_S: f64 = 0.9;
/// Room admission ladder (12 seats, two runs): the 12th seated, everyone reads everything,
/// takedown; seconds.
pub const ADMIT_12: [(f64, f64); 3] = [(93.0, 127.0), (81.0, 85.0), (68.0, 71.0)];

/// One capped-uplink run (MEASURED, matrix).
#[derive(Debug, Clone, Copy)]
pub struct CappedRun {
    /// Members.
    pub n: u32,
    /// Uplink cap, kbit/s.
    pub cap_kbps: f64,
    /// Listener delivery latency p50, seconds (range over listeners).
    pub p50_s: (f64, f64),
    /// p95, seconds.
    pub p95_s: (f64, f64),
    /// Share of frames heard.
    pub heard: (f64, f64),
}

/// The measured capped runs.
pub const CAPPED_RUNS: [CappedRun; 3] = [
    CappedRun { n: 4, cap_kbps: 250.0, p50_s: (6.0, 9.9), p95_s: (11.1, 19.9), heard: (0.75, 0.93) },
    CappedRun { n: 4, cap_kbps: 300.0, p50_s: (3.2, 3.9), p95_s: (6.6, 6.9), heard: (0.75, 0.93) },
    CappedRun { n: 6, cap_kbps: 300.0, p50_s: (11.9, 11.9), p95_s: (15.9, 15.9), heard: (0.857, 0.857) },
];

// ---- DERIVED / FITTED -----------------------------------------------------------------------

/// Offers on one leg per second: the member's own stream and every stream it relays that is not
/// the peer's own (v45.2.1 alm.rs :2098-2170: own offer, shares, a relay offer per forwarding
/// admission skipping the peer's ancestry and publisher; `ensure_relay` makes one per received
/// stream, :2015).
#[must_use]
pub fn offers_per_leg_today(n: f64) -> f64 {
    if n < 2.0 { 0.0 } else { 1.0 + (n - 2.0) }
}

/// C3 offers a member sends per second, today: one leg to each other member.
#[must_use]
pub fn offers_per_s_today(n: f64) -> f64 {
    (n - 1.0).max(0.0) * offers_per_leg_today(n)
}

/// Wire bytes per control message, FITTED: N = 4's 14 kbit/s over its measured 9.2 messages a
/// second. Source check: an offer is 64 B + its ancestry plus `OFFER_WIRE_OVERHEAD` 120 B
/// (v45.2.1 alm.rs :71, docs/alm_calm_wire.md :68), 184 B with no ancestry.
pub const CTRL_BYTES_PER_MSG: f64 = CTRL_KBPS_N4 * 1_000.0 / 8.0 / 9.2;

/// Announce bits per second per peer, FITTED at N = 4 (2.5 kbit/s over 3 peers).
pub const ANN_BPS_PER_PEER: f64 = ANN_KBPS_N4 * 1_000.0 / 3.0;

/// Every number this module uses, its class and source.
pub const CALL_SPECS: &[CallSpec] = &[
    CallSpec { name: "chunk_wire", value: 532.0, hi: 532.0, unit: "B", tag: CTag::Measured, source: "504 B RNS + 28 IPv4/UDP for a 246 B frame (bench); media.rs MEASURED_CHUNK_RNS_BYTES, 5 B outside the RNS packet" },
    CallSpec { name: "copy_kbps", value: COPY_KBPS.0, hi: COPY_KBPS.1, unit: "kbit/s", tag: CTag::Measured, source: "one copy: bench 30 fps, 30.3 chunks/s, ~127 kbit/s with IPv4/UDP; matrix ~190 (its codec frames are larger)" },
    CallSpec { name: "ctrl_kbps_n4", value: CTRL_KBPS_N4, hi: CTRL_KBPS_N4, unit: "kbit/s", tag: CTag::Measured, source: "C3 control at the publisher, N = 4 (bench and matrix); the fit point of ctrl_bytes_per_msg" },
    CallSpec { name: "ctrl_kbps_n6", value: CTRL_KBPS_N6.0, hi: CTRL_KBPS_N6.1, unit: "kbit/s", tag: CTag::Measured, source: "the same at N = 6; held out" },
    CallSpec { name: "proof_kbps_n4", value: PROOF_KBPS_N4, hi: PROOF_KBPS_N4, unit: "kbit/s", tag: CTag::Measured, source: "proofs at the publisher, N = 4" },
    CallSpec { name: "proof_kbps_n6", value: PROOF_KBPS_N6.0, hi: PROOF_KBPS_N6.1, unit: "kbit/s", tag: CTag::Measured, source: "proofs at the publisher, N = 6" },
    CallSpec { name: "announce_kbps_n4", value: ANN_KBPS_N4, hi: ANN_KBPS_N4, unit: "kbit/s", tag: CTag::Measured, source: "announces at the publisher, N = 4; the fit point of announce_bps_per_peer" },
    CallSpec { name: "announce_kbps_n6", value: ANN_KBPS_N6.0, hi: ANN_KBPS_N6.1, unit: "kbit/s", tag: CTag::Measured, source: "announces at N = 6; held out" },
    CallSpec { name: "proof_bytes", value: PROOF_BYTES, hi: PROOF_BYTES, unit: "B", tag: CTag::Measured, source: "one proof upstream per received Channel message (bench)" },
    CallSpec { name: "c3_sent_n4", value: 9.2, hi: 9.2, unit: "msg/s", tag: CTag::Measured, source: "matrix baseline C3 sent per second, N = 4 (uncapped)" },
    CallSpec { name: "c3_sent_n6", value: 24.7, hi: 24.7, unit: "msg/s", tag: CTag::Measured, source: "matrix baseline C3 sent per second, N = 6 (uncapped)" },
    CallSpec { name: "offers_per_s", value: 0.0, hi: 0.0, unit: "msg/s", tag: CTag::Derived, source: "(N - 1) legs x (1 + (N - 2)) offers a leg each second: v45.2.1 alm.rs :2098-2170 (own, shares, one relay offer per forwarding admission, the peer's own stream skipped), OFFER_EVERY_MS 1000 (:53), ensure_relay per received stream (:2015); 9 at N = 4, 25 at N = 6" },
    CallSpec { name: "ctrl_bytes_per_msg", value: CTRL_BYTES_PER_MSG, hi: CTRL_BYTES_PER_MSG, unit: "B", tag: CTag::Fitted, source: "14 kbit/s / 9.2 msg/s at N = 4; the source gives 184 B for an offer with no ancestry (64 + OFFER_WIRE_OVERHEAD 120, alm.rs :71)" },
    CallSpec { name: "announce_bps_per_peer", value: ANN_BPS_PER_PEER, hi: ANN_BPS_PER_PEER, unit: "bit/s", tag: CTag::Fitted, source: "2.5 kbit/s over 3 peers at N = 4; O(N - 1)" },
    CallSpec { name: "standby_share", value: 0.69, hi: 0.69, unit: "stream", tag: CTag::Model, source: "a warm standby (audio + base) is 0.69 of a stream at the media engine's codec and GOP (FSD/twin/media.md :146, MODEL output); v45.2.1 alm.rs :31-32 sends a standby child base and audio only" },
    CallSpec { name: "nonmedia_capped_n6", value: NONMEDIA_CAPPED_N6_KBPS.0, hi: NONMEDIA_CAPPED_N6_KBPS.1, unit: "kbit/s", tag: CTag::Measured, source: "the capped member's non-media at N = 6, 300 kbit/s cap (matrix), mostly identity-plane replication, split not measured" },
    CallSpec { name: "c3_capped_n6", value: C3_CAPPED_N6.0, hi: C3_CAPPED_N6.1, unit: "msg/s", tag: CTag::Measured, source: "under the cap at N = 6/300: C3 sent 10.6/s, received 16.7/s (matrix step log)" },
    CallSpec { name: "offered_over_cap", value: OFFERED_OVER_CAP.0, hi: OFFERED_OVER_CAP.1, unit: "ratio", tag: CTag::Measured, source: "after the shed the publisher offers one copy + control + proofs = 96-111 % of the cap: the queue never drains" },
    CallSpec { name: "first_second_queue", value: FIRST_SECOND_QUEUE.0 .0, hi: FIRST_SECOND_QUEUE.0 .1, unit: "packets", tag: CTag::Measured, source: "packets queued in the first capped second (4.4-7.5 s at the cap) before the verdict at +0.9 s" },
    CallSpec { name: "admit_12th", value: ADMIT_12[0].0, hi: ADMIT_12[0].1, unit: "s", tag: CTag::Measured, source: "server 12-person ladder on v45.2.1, two runs: the 12th seat seated at 93 / 127 s; everyone reads everything 81 / 85 s; takedown 68 / 71 s" },
    CallSpec { name: "binding_bytes", value: 3_504.0, hi: 3_504.0, unit: "B", tag: CTag::Derived, source: "a stream key binding is 3486 + len(room) + len(publisher) B, the KAT 3504 (v45.2.1 docs/av_chunk_auth_wire.md :45-46); sent on every leg at every roll (av_call.rs :60-62) and every Add/Remove rolls every live stream (:10-13)" },
    CallSpec { name: "presence_blob", value: 176.0, hi: 176.0, unit: "B", tag: CTag::Model, source: "presence tier, locked design (not shipped): 176 B blob, at least 1 per 5 s, at most 5 per s and 20 per 10 s, 256 visible by default" },
    CallSpec { name: "presence_wire", value: 306.0, hi: 462.0, unit: "B", tag: CTag::Model, source: "a blob on the wire: 176 + E1 seal 25 + RNS token ~53 + header 19 + framing ~5 + IPv4/UDP 28 = 306 if the blob carries its own signature; + chunk header 52, tags 32 and trailer 72 = 462 if it rides as a media chunk" },
    CallSpec { name: "presence_rate", value: 0.2, hi: 2.0, unit: "1/s", tag: CTag::Model, source: "per visible member: the design's floor (1 per 5 s) to its sustained cap (20 per 10 s)" },
    CallSpec { name: "watch", value: 9.0, hi: 9.0, unit: "streams", tag: CTag::Model, source: "streams a member chooses to watch under per-stream subscription (a 3 x 3 grid); the table also shows 25" },
    CallSpec { name: "fixed_legs_extra", value: 4.0, hi: 4.0, unit: "legs", tag: CTag::Model, source: "legs a member holds beyond its watched streams' parents and its children: the presence tier's neighbours (not designed)" },
    CallSpec { name: "age_budget", value: 0.8, hi: 0.8, unit: "s", tag: CTag::Model, source: "45.3 receiver drop by end-to-end age; today's MEDIA_STALE_BUDGET 800 ms (av_call.rs :136) measures only the receiver's own queue" },
];

// ---- the per-member model ---------------------------------------------------------------------

/// Which code the member runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// Edge v45.2.1 as measured: every member subscribes to every stream (primary + standby),
    /// offers every stream it relays on every leg, legs to every member.
    Today,
    /// MODEL: the 45.3 fixes plus per-stream subscription.
    Fixed,
}

/// The inputs a fixed-shape member's cost depends on (every one MODEL).
#[derive(Debug, Clone, Copy)]
pub struct Choice {
    /// Streams watched (full copies).
    pub watch: f64,
    /// Presence blobs per visible member per second.
    pub presence_rate: f64,
    /// Wire bytes per presence blob.
    pub presence_wire: f64,
    /// Standby share of a stream (0: a cold standby, promoted on failure).
    pub standby: f64,
}

impl Choice {
    /// The low end of every MODEL input: floor rate, small blob, no warm standby.
    pub const LOW: Self = Self { watch: 9.0, presence_rate: 0.2, presence_wire: 306.0, standby: 0.0 };
    /// The high end: the sustained cap, a blob as a media chunk, a warm audio + base standby.
    pub const HIGH: Self = Self { watch: 9.0, presence_rate: 2.0, presence_wire: 462.0, standby: 0.69 };
}

/// One member's cost in one live call, kbit/s each direction (mean member; the tree moves
/// relay work between members but not its mean).
#[derive(Debug, Clone, Copy, Default)]
pub struct MemberCost {
    /// Links held.
    pub legs: f64,
    /// Control messages sent per second.
    pub ctrl_msgs: f64,
    /// Control, kbit/s each way.
    pub ctrl_kbps: f64,
    /// Proofs, kbit/s each way.
    pub proof_kbps: f64,
    /// Announces, kbit/s each way.
    pub announce_kbps: f64,
    /// Media, kbit/s each way (mean; conservation).
    pub media_kbps: f64,
    /// Presence blobs, kbit/s each way (mean).
    pub presence_kbps: f64,
}

impl MemberCost {
    /// Everything but media and presence.
    #[must_use]
    pub fn nonmedia_kbps(&self) -> f64 {
        self.ctrl_kbps + self.proof_kbps + self.announce_kbps
    }

    /// The whole of one direction.
    #[must_use]
    pub fn total_kbps(&self) -> f64 {
        self.nonmedia_kbps() + self.media_kbps + self.presence_kbps
    }
}

/// A member's cost at room size `n` with one copy at `copy_kbps`.
#[must_use]
pub fn member_cost(shape: Shape, n: f64, copy_kbps: f64, c: Choice) -> MemberCost {
    let peers = (n - 1.0).max(0.0);
    match shape {
        Shape::Today => {
            let msgs = offers_per_s_today(n);
            MemberCost {
                legs: 2.0 * peers,
                ctrl_msgs: msgs,
                ctrl_kbps: msgs * CTRL_BYTES_PER_MSG * 8.0 / 1_000.0,
                // every received offer is proved; a member receives as many as it sends
                proof_kbps: msgs * PROOF_BYTES * 8.0 / 1_000.0,
                announce_kbps: peers * ANN_BPS_PER_PEER / 1_000.0,
                media_kbps: peers * (1.0 + 0.69) * copy_kbps,
                presence_kbps: 0.0,
            }
        }
        Shape::Fixed => {
            let watch = c.watch.min(peers);
            // a parent per watched stream (plus its standby when warm), as many children on
            // average, and the presence tier's neighbours; one liveness offer a leg a second
            let legs = watch * (2.0 + if c.standby > 0.0 { 1.0 } else { 0.0 }) + 4.0_f64.min(peers);
            let visible = (peers.min(256.0) - watch).max(0.0);
            MemberCost {
                legs,
                ctrl_msgs: legs,
                ctrl_kbps: legs * CTRL_BYTES_PER_MSG * 8.0 / 1_000.0,
                proof_kbps: legs * PROOF_BYTES * 8.0 / 1_000.0,
                announce_kbps: legs * ANN_BPS_PER_PEER / 1_000.0,
                media_kbps: watch * (1.0 + c.standby) * copy_kbps,
                presence_kbps: visible * c.presence_rate * c.presence_wire * 8.0 / 1_000.0,
            }
        }
    }
}

/// The smallest room at which today's control, proofs and announces alone fill an uplink of
/// `cap_kbps` (before any media).
#[must_use]
pub fn control_fills_uplink_at(cap_kbps: f64) -> f64 {
    let mut n = 2.0;
    while member_cost(Shape::Today, n, 0.0, Choice::LOW).nonmedia_kbps() < cap_kbps && n < 1.0e6 {
        n += 1.0;
    }
    n
}

/// The capped member at N = 6, 300 kbit/s, today: offered load over the cap after the shed, with
/// one copy and the MEASURED non-media. The measured 96-111 % must fall inside.
#[must_use]
pub fn capped_offered_n6() -> (f64, f64) {
    ((COPY_KBPS.0 + NONMEDIA_CAPPED_N6_KBPS.0) / 300.0, (COPY_KBPS.1 + NONMEDIA_CAPPED_N6_KBPS.1) / 300.0)
}

/// The identity-plane replication share of the capped member's non-media at N = 6/300, by
/// subtraction (DERIVED from MEASURED): non-media less C3 at the stall cadence (sent x bytes),
/// proofs for C3 received, and announces.
#[must_use]
pub fn replication_by_subtraction_n6() -> (f64, f64) {
    let c3 = C3_CAPPED_N6.0 * CTRL_BYTES_PER_MSG * 8.0 / 1_000.0;
    let proofs = C3_CAPPED_N6.1 * PROOF_BYTES * 8.0 / 1_000.0;
    (NONMEDIA_CAPPED_N6_KBPS.0 - c3 - proofs - ANN_KBPS_N6.1, NONMEDIA_CAPPED_N6_KBPS.1 - c3 - proofs - ANN_KBPS_N6.0)
}

/// Under the 45.3 fixes (MODEL): a capped member shows video only when its uplink carries its
/// non-media and one copy; otherwise the sender withholds video (audio stays). `replication_kbps`
/// is the identity-plane replication it also carries.
#[must_use]
pub fn fixed_capped_sends_video(n: f64, cap_kbps: f64, copy_kbps: f64, replication_kbps: f64) -> bool {
    let m = member_cost(Shape::Fixed, n, copy_kbps, Choice::LOW);
    m.nonmedia_kbps() + replication_kbps + copy_kbps <= cap_kbps
}

/// Per-Add stream roll, today: every member re-sends its binding on every leg (DERIVED), bytes.
#[must_use]
pub fn roll_bytes_per_member_per_add(n: f64) -> f64 {
    3_504.0 * (n - 1.0).max(0.0)
}

/// Seconds one member's uplink of `up_kbps` needs for one Add's roll.
#[must_use]
pub fn roll_seconds(n: f64, up_kbps: f64) -> f64 {
    roll_bytes_per_member_per_add(n) * 8.0 / (up_kbps * 1_000.0)
}

// ---- the seat ---------------------------------------------------------------------------------

/// What a room whose hub is the seat costs it, per member, at a fleet of `fleet` devices.
#[derive(Debug, Clone, Copy)]
pub struct SeatPerMember {
    /// Members.
    pub members: f64,
    /// Seat cores added per member (steady, each rounding every 30 s).
    pub cores_per_member: f64,
    /// Sweep-permit utilisation added per member.
    pub permit_rho_per_member: f64,
    /// Counted store reads per second added per member.
    pub reads_per_member: f64,
    /// Seat cores while joins arrive at one a second into a room this size (rows and deliveries).
    pub join_cores_at_1_per_s: f64,
}

/// Price `members` room members at the seat as the room's hub. Not reachable on v45.2.x (the
/// canonical is never a room member, and room rows are audience-gated to members); MODEL, kept
/// for a server that seats its canonical in a room.
#[must_use]
pub fn seat_per_member(p: &CostParams, fleet: f64, members: f64) -> SeatPerMember {
    let base = Scenario::v45(fleet);
    let o0 = capacity::run(p, &base);
    let mut sc = base;
    sc.room_members = members;
    let o1 = capacity::run(p, &sc);
    let mut j = base;
    j.room_joins_per_min = 60.0;
    j.room_size = members;
    let oj = capacity::run(p, &j);
    SeatPerMember {
        members,
        cores_per_member: (o1.seat_load.cpu_cores - o0.seat_load.cpu_cores) / members,
        permit_rho_per_member: (o1.seat_load.rho_permit - o0.seat_load.rho_permit) / members,
        reads_per_member: (o1.seat.counted_per_s() - o0.seat.counted_per_s()) / members,
        join_cores_at_1_per_s: oj.seat_load.cpu_cores - o0.seat_load.cpu_cores,
    }
}

/// The fleet knee (MODEL, interval over every input interval) with `members` of the fleet in a
/// room hubbed at the seat, at announced fraction `g`.
#[must_use]
pub fn knee_with_room(p: &CostParams, g: f64, members: f64) -> seat_calibration::KneeInterval {
    let q = seat_calibration::with_value(p, C::AnnouncedFraction, g);
    let mut sc = Scenario::v45(1.0);
    sc.room_members = members;
    seat_calibration::knee_interval(&q, &sc)
}

/// The largest room the seat can hub at a fleet of `fleet` devices (the room's members counted
/// in the fleet, so the fleet is at least the room) with `term` alone under the ceiling.
#[must_use]
pub fn hub_room_term(p: &CostParams, fleet: f64, term: seat_calibration::Term) -> f64 {
    let ceiling = p.get(C::CpuCeiling);
    let ok = |m: f64| {
        let mut sc = Scenario::v45(fleet.max(m));
        sc.room_members = m;
        let l = capacity::run(p, &sc).seat_load;
        let rho = match term {
            seat_calibration::Term::Cpu => l.rho_cpu,
            seat_calibration::Term::Permits => l.rho_permit,
            seat_calibration::Term::Lock => l.rho_lock,
        };
        rho <= ceiling
    };
    if !ok(1.0) {
        return 0.0;
    }
    let (mut lo, mut hi) = (1.0_f64, 1.0e7_f64);
    if ok(hi) {
        return hi;
    }
    for _ in 0..80 {
        let mid = (lo * hi).sqrt();
        if ok(mid) {
            lo = mid;
        } else {
            hi = mid;
        }
        if hi / lo < 1.001 {
            break;
        }
    }
    lo.floor()
}

/// The largest room the seat can hub at `fleet`, at growth `g`: point (calibrated inputs) and the
/// outer interval over every input interval, each term propagated alone (as
/// [`seat_calibration::knee_interval`]), with the term that binds at the point.
#[must_use]
pub fn hub_room(p: &CostParams, g: f64, fleet: f64) -> (f64, seat_calibration::Interval, &'static str) {
    use seat_calibration::Term;
    let q = seat_calibration::with_value(p, C::AnnouncedFraction, g);
    let inputs = seat_calibration::input_intervals(&q);
    let terms = [(Term::Cpu, "CPU"), (Term::Permits, "sweep permits"), (Term::Lock, "a key-plane lock")];
    let mut point = f64::INFINITY;
    let mut bind = "";
    let (mut lo, mut hi) = (f64::INFINITY, f64::INFINITY);
    for (t, name) in terms {
        let x = hub_room_term(&q, fleet, t);
        if x < point {
            point = x;
            bind = name;
        }
        let pr = seat_calibration::propagate(&q, &inputs, |r| hub_room_term(r, fleet, t));
        lo = lo.min(pr.interval.lo);
        hi = hi.min(pr.interval.hi);
    }
    (point, seat_calibration::Interval::new(lo, hi), bind)
}

// ---- report -----------------------------------------------------------------------------------

fn k(x: f64) -> String {
    if x >= 100_000.0 {
        format!("{:.0}", x)
    } else if x >= 100.0 {
        format!("{:.0}", x)
    } else if x >= 10.0 {
        format!("{:.1}", x)
    } else {
        format!("{:.2}", x)
    }
}

/// The room sizes the report prices.
pub const SIZES: [f64; 5] = [4.0, 12.0, 50.0, 100.0, 1_000.0];

/// The per-member table, markdown: kbit/s each way, today and fixed, low and high ends.
#[must_use]
pub fn member_table() -> String {
    let mut s = String::from(
        "| N | today: legs | today: offers sent/s (DERIVED) | today: control + proofs + announces, kbit/s each way | today: media each way, kbit/s (copy 127-190) | today: total each way, kbit/s | fixed, watch 9: non-media | fixed: media (9 copies, cold to warm standby) | fixed: presence | fixed: total each way, kbit/s | fixed, watch 25: total |\n|---|---|---|---|---|---|---|---|---|---|---|\n",
    );
    for &n in &SIZES {
        let t0 = member_cost(Shape::Today, n, COPY_KBPS.0, Choice::LOW);
        let t1 = member_cost(Shape::Today, n, COPY_KBPS.1, Choice::LOW);
        let f0 = member_cost(Shape::Fixed, n, COPY_KBPS.0, Choice::LOW);
        let f1 = member_cost(Shape::Fixed, n, COPY_KBPS.1, Choice::HIGH);
        let w0 = member_cost(Shape::Fixed, n, COPY_KBPS.0, Choice { watch: 25.0, ..Choice::LOW });
        let w1 = member_cost(Shape::Fixed, n, COPY_KBPS.1, Choice { watch: 25.0, ..Choice::HIGH });
        s.push_str(&format!(
            "| {} | {} | {} | {} | {} to {} | {} to {} | {} to {} | {} to {} | {} to {} | {} to {} | {} to {} |\n",
            n,
            k(t0.legs),
            k(t0.ctrl_msgs),
            k(t0.nonmedia_kbps()),
            k(t0.media_kbps),
            k(t1.media_kbps),
            k(t0.total_kbps()),
            k(t1.total_kbps()),
            k(f0.nonmedia_kbps()),
            k(f1.nonmedia_kbps()),
            k(f0.media_kbps),
            k(f1.media_kbps),
            k(f0.presence_kbps),
            k(f1.presence_kbps),
            k(f0.total_kbps()),
            k(f1.total_kbps()),
            k(w0.total_kbps()),
            k(w1.total_kbps()),
        ));
    }
    s
}

/// The checks of the fitted terms against the held-out N = 6 measurement, markdown.
#[must_use]
pub fn check_table() -> String {
    let m4 = member_cost(Shape::Today, 4.0, 0.0, Choice::LOW);
    let m6 = member_cost(Shape::Today, 6.0, 0.0, Choice::LOW);
    let mut s = String::from("| term | N = 4 model | N = 4 MEASURED | N = 6 model | N = 6 MEASURED (held out) |\n|---|---|---|---|---|\n");
    s.push_str(&format!("| offers sent/s (DERIVED count) | {} | 9.2 | {} | 24.7 |\n", k(offers_per_s_today(4.0)), k(offers_per_s_today(6.0))));
    s.push_str(&format!("| control kbit/s (bytes FITTED at N = 4) | {} | {} | {} | {} to {} |\n", k(m4.ctrl_kbps), CTRL_KBPS_N4, k(m6.ctrl_kbps), CTRL_KBPS_N6.0, CTRL_KBPS_N6.1));
    s.push_str(&format!("| proofs kbit/s (one 110 B proof per offer received; not fitted) | {} | {} | {} | {} to {} |\n", k(m4.proof_kbps), PROOF_KBPS_N4, k(m6.proof_kbps), PROOF_KBPS_N6.0, PROOF_KBPS_N6.1));
    s.push_str(&format!("| announces kbit/s (FITTED at N = 4) | {} | {} | {} | {} to {} |\n", k(m4.announce_kbps), ANN_KBPS_N4, k(m6.announce_kbps), ANN_KBPS_N6.0, ANN_KBPS_N6.1));
    let (lo, hi) = capped_offered_n6();
    s.push_str(&format!("| capped member offered / cap, N = 6/300 (one copy + MEASURED non-media) | | | {:.2} to {:.2} | {} to {} |\n", lo, hi, OFFERED_OVER_CAP.0, OFFERED_OVER_CAP.1));
    s
}

/// The seat table at today's fleet, markdown.
#[must_use]
pub fn seat_table(p: &CostParams) -> String {
    let mut s = String::from(
        "| room members hubbed at the seat | seat cores per member (steady) | permit utilisation per member | counted reads/s per member | seat cores while joins arrive at 1/s |\n|---|---|---|---|---|\n",
    );
    for &n in &[12.0, 100.0, 1_000.0] {
        let r = seat_per_member(p, 1_349.0, n);
        s.push_str(&format!(
            "| {} | {:.2e} | {:.2e} | {:.1} | {:.3} |\n",
            n, r.cores_per_member, r.permit_rho_per_member, r.reads_per_member, r.join_cores_at_1_per_s
        ));
    }
    s
}

/// The knee with and without a 1000-member room hubbed at the seat, markdown.
#[must_use]
pub fn knee_table(p: &CostParams) -> String {
    let mut s = String::from(
        "| announced fraction f (MODEL) | no room at the seat: knee, devices (MODEL, interval) | binds low / high | a 1000-member room hubbed at the seat: knee (MODEL, interval) | binds low / high |\n|---|---|---|---|---|\n",
    );
    for &g in &[0.0, 1.0] {
        let a = knee_with_room(p, g, 0.0);
        let b = knee_with_room(p, g, 1_000.0);
        s.push_str(&format!(
            "| {} | {} | {} / {} | {} | {} / {} |\n",
            g,
            a.devices.devices(),
            a.binds_low.label(),
            a.binds_high.label(),
            b.devices.devices(),
            b.binds_low.label(),
            b.binds_high.label()
        ));
    }
    s
}

/// The whole report, markdown.
#[must_use]
pub fn report(p: &CostParams) -> String {
    let mut s = String::from("## Call scale (edge v45.2.1 measured, closed form)\n\n");
    s.push_str(&format!("Sources: {BENCH}; {MATRIX}. Every input in CALL_SPECS with its class.\n\n### Fitted terms against the held-out N = 6\n\n"));
    s.push_str(&check_table());
    s.push_str(&format!(
        "\nIdentity-plane replication on the capped member at N = 6/300, by subtraction (DERIVED from MEASURED): {:.0} to {:.0} kbit/s.\n",
        replication_by_subtraction_n6().0,
        replication_by_subtraction_n6().1
    ));
    s.push_str(&format!(
        "Today's control, proofs and announces alone fill a 300 kbit/s uplink at N = {} and a 1 Mbit/s uplink at N = {} (DERIVED + FITTED).\n\n",
        control_fills_uplink_at(300.0),
        control_fills_uplink_at(1_000.0)
    ));
    s.push_str("### Per member, one live call, kbit/s each way (mean member)\n\n");
    s.push_str(&member_table());
    s.push_str("\n### Per-Add stream roll today (DERIVED): bytes each member sends, and seconds on a 1 Mbit/s uplink\n\n| N | bytes | seconds at 1 Mbit/s |\n|---|---|---|\n");
    for &n in &SIZES {
        s.push_str(&format!("| {} | {:.0} | {:.2} |\n", n, roll_bytes_per_member_per_add(n), roll_seconds(n, 1_000.0)));
    }
    s.push_str("\n### The seat (v45.2.x: a call costs it nothing per member joined)\n\nThe canonical is never a call-room member and is refused any call relay; a room's rows are served to a peer only past persist's may_receive audience gate (v45.2.1 bridge.rs audience_withholds :6445-6468), so a canonical outside the room is never sent them and holds none to relay. The rows below price a room hubbed at the seat, a case v45.2.x does not reach; kept for a server that makes its canonical a room member.\n\n");
    s.push_str(&seat_table(p));
    s.push_str("\n### The largest room the seat can hub, fleet 1,349 (MODEL; interval over every input interval)\n\n| announced fraction f | members (point) | members (interval) | binds at the point |\n|---|---|---|---|\n");
    for &g in &[0.0, 1.0] {
        let (pt, iv, b) = hub_room(p, g, 1_349.0);
        s.push_str(&format!("| {} | {:.0} | {} | {} |\n", g, pt, iv.devices(), b));
    }
    s.push_str("\n### The fleet knee with a 1000-member room hubbed at the seat (the room counted in the fleet)\n\n");
    s.push_str(&knee_table(p));
    s.push_str("\n### Parameters\n\n| name | value | unit | class | source |\n|---|---|---|---|---|\n");
    for c in CALL_SPECS {
        let v = if (c.hi - c.value).abs() > 0.0 { format!("{} to {}", c.value, c.hi) } else { format!("{:.4}", c.value).trim_end_matches('0').trim_end_matches('.').to_owned() };
        s.push_str(&format!("| `{}` | {} | {} | {} | {} |\n", c.name, v, c.unit, c.tag.as_str(), c.source));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rel(x: f64, t: f64) -> f64 {
        (x / t - 1.0).abs()
    }

    #[test]
    fn every_input_has_a_class_and_a_source() {
        for c in CALL_SPECS {
            assert!(!c.source.is_empty(), "{}", c.name);
            assert!(c.hi >= c.value, "{}", c.name);
        }
        assert!(CALL_SPECS.iter().any(|c| c.tag == CTag::Measured));
        assert!(CALL_SPECS.iter().filter(|c| c.tag == CTag::Fitted).count() == 2, "two fitted terms, both at N = 4");
    }

    /// The source's offer count reproduces the measured C3 rate at both sizes, and the 3-5
    /// offers a leg the bench saw.
    #[test]
    fn the_offer_count_is_the_measured_c3_rate() {
        assert!((offers_per_leg_today(4.0) - 3.0).abs() < 1e-12 && (offers_per_leg_today(6.0) - 5.0).abs() < 1e-12);
        for (n, measured) in C3_SENT_BASELINE {
            assert!(rel(offers_per_s_today(n), measured) < 0.03, "N = {n}: {} vs {measured}", offers_per_s_today(n));
        }
        // the fitted bytes per message agree with the source's offer size (184 B) within 5 %
        assert!(rel(CTRL_BYTES_PER_MSG, 184.0) < 0.05, "{CTRL_BYTES_PER_MSG}");
    }

    /// Fitted at N = 4, the held-out N = 6 control and announces land in the measured range (the
    /// control at its upper end); proofs are not fitted and miss by about a fifth either way.
    #[test]
    fn the_n4_fit_predicts_n6() {
        let m4 = member_cost(Shape::Today, 4.0, 0.0, Choice::LOW);
        let m6 = member_cost(Shape::Today, 6.0, 0.0, Choice::LOW);
        assert!(rel(m4.ctrl_kbps, CTRL_KBPS_N4) < 0.03);
        assert!(m6.ctrl_kbps >= CTRL_KBPS_N6.0 && m6.ctrl_kbps <= CTRL_KBPS_N6.1 * 1.01, "{}", m6.ctrl_kbps);
        assert!(m6.announce_kbps >= ANN_KBPS_N6.0 && m6.announce_kbps <= ANN_KBPS_N6.1, "{}", m6.announce_kbps);
        assert!(rel(m4.proof_kbps, PROOF_KBPS_N4) < 0.25 && m4.proof_kbps < PROOF_KBPS_N4, "{}", m4.proof_kbps);
        assert!(m6.proof_kbps > PROOF_KBPS_N6.1 && rel(m6.proof_kbps, PROOF_KBPS_N6.1) < 0.15, "{}", m6.proof_kbps);
    }

    /// One copy plus the measured non-media brackets the measured 96-111 % of the cap after the
    /// shed: the queue never drains, as the matrix saw.
    #[test]
    fn the_capped_member_is_overloaded_as_measured() {
        let (lo, hi) = capped_offered_n6();
        assert!(lo <= OFFERED_OVER_CAP.0 && hi >= OFFERED_OVER_CAP.1, "{lo} {hi}");
        let (rlo, rhi) = replication_by_subtraction_n6();
        assert!(rlo > 60.0 && rhi < 130.0, "{rlo} {rhi}");
        // the first capped second's queue is full media chunks (532 B) drained at 250 kbit/s
        let ((p0, p1), (s0, s1)) = FIRST_SECOND_QUEUE;
        let drain = |pkts: f64| pkts * 532.0 * 8.0 / 250_000.0;
        assert!(rel(drain(p0), s0) < 0.02 && rel(drain(p1), s1) < 0.02, "{} {}", drain(p0), drain(p1));
    }

    /// Today's control is O(N²) per member and fills a 300 kbit/s uplink near N = 12, before
    /// media; a 1000-person call needs gigabits per member.
    #[test]
    fn today_control_is_quadratic_per_member() {
        let n = control_fills_uplink_at(300.0);
        assert!((11.0..=13.0).contains(&n), "{n}");
        let a = member_cost(Shape::Today, 100.0, 127.0, Choice::LOW);
        let b = member_cost(Shape::Today, 1_000.0, 127.0, Choice::LOW);
        assert!(b.ctrl_kbps / a.ctrl_kbps > 95.0, "quadratic: {}", b.ctrl_kbps / a.ctrl_kbps);
        assert!(b.total_kbps() > 1.0e6, "over a gigabit: {}", b.total_kbps());
    }

    /// Per-stream subscription makes a member's cost flat past the presence window: the same at
    /// 1000 as at 300, set by what it watches and 256 presence blobs.
    #[test]
    fn subscription_caps_a_members_cost() {
        let at = |n: f64, c: Choice| member_cost(Shape::Fixed, n, 190.0, c).total_kbps();
        assert!((at(1_000.0, Choice::HIGH) - at(300.0, Choice::HIGH)).abs() < 1e-9);
        assert!(at(1_000.0, Choice::LOW) < 2_000.0 && at(1_000.0, Choice::LOW) > 1_000.0, "{}", at(1_000.0, Choice::LOW));
        let f = member_cost(Shape::Fixed, 1_000.0, 127.0, Choice::LOW);
        // 22 legs of one liveness offer, its proof and announces: ~71 kbit/s, flat in N
        assert!(f.nonmedia_kbps() > 60.0 && f.nonmedia_kbps() < 80.0, "{}", f.nonmedia_kbps());
        // a 300 kbit/s member under the fixes: non-media small, so one copy fits only when the
        // identity-plane replication fits beside it
        assert!(fixed_capped_sends_video(1_000.0, 300.0, 127.0, 0.0));
        assert!(!fixed_capped_sends_video(1_000.0, 300.0, 190.0, 100.0));
    }

    /// The roll on each Add is O(N) per member: at 1000 members one Add costs each member about
    /// 3.5 MB, 28 s of a 1 Mbit/s uplink, longer than the measured time to seat one member.
    #[test]
    fn each_add_rolls_every_stream() {
        assert!((roll_bytes_per_member_per_add(1_000.0) - 3_500_496.0).abs() < 1.0);
        assert!(roll_seconds(1_000.0, 1_000.0) > ADMIT_12[0].1 / 12.0);
        assert!(roll_seconds(12.0, 1_000.0) < ADMIT_12[0].0 / 12.0);
    }

    /// The seat pays for a room only when it is the room's hub; then per member, steady.
    #[test]
    fn the_seat_pays_per_member_only_as_the_hub() {
        let p = CostParams::calibrated();
        let r = seat_per_member(&p, 1_349.0, 100.0);
        assert!(r.cores_per_member > 0.0 && r.permit_rho_per_member > 0.0 && r.reads_per_member > 0.0);
        let big = seat_per_member(&p, 1_349.0, 1_000.0);
        assert!(big.join_cores_at_1_per_s > r.join_cores_at_1_per_s, "deliveries grow with the room");
        // a 1000-member room alone takes the permits past the ceiling at the calibrated point
        let (pt, iv, bind) = hub_room(&p, 0.0, 1_349.0);
        assert!(pt < 1_000.0 && pt > 100.0 && bind == "sweep permits", "{pt} {bind}");
        assert!(iv.lo <= pt && iv.hi >= pt, "{} {} {pt}", iv.lo, iv.hi);
        let r = report(&p);
        assert!(r.contains("Call scale") && !r.contains("NaN"));
    }
}
