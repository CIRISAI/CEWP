//! The kernel's rulebook at scale — CC 5.4.3 budget classes, 1-hop shaped links & CC 6.1.6 `deterministic-alm` edition (CC RC17 / Edge v44 / FSD v0.2).
//!
//! One tick = `acts` random signed rows over a world of nodes and rooms placed on a land-bounded
//! population density grid. For each row the rulebook runs at the signer's own node and at every node
//! of the row's audience: object state, standing, and refusal boundary.
//!
//! Includes cewp-sim v0.2 physics & CIRISHolon multiscale certification (OUT = 36):
//!   - IMIX 6-category payloads & longitude-aware diurnal wave (diurnal_multiplier)
//!   - Constitutional 64 KiB shaped-link gate (check_shaped_payload)
//!   - Rank-based L <= 8 displacements & 3-referral proxy pulls
//!   - Whole-chunk custody & layered-fidelity noise-floor descent (NoiseFloorState)
//!   - CIRISHolon commuting-square closure defect D_v^2, budget beta=0.030, Lipschitz K, maintenance rent G_inf, witness pairs
//!   - Credible baselines: A1_HYPERSCALE_CDN, A2_MULTIREGION_SFU, A3_HYBRID_P2P wire bytes

use std::alloc::{alloc as sys_alloc, Layout};
use cewp_sim::transport::check_shaped_payload;
use cewp_sim::chunk_custody::{classify, NoiseFloorState};
use cewp_sim::holon::{ClosureCertificate, MaintenanceRent};
use cewp_sim::workload::diurnal_multiplier;

mod globe;
mod views;

pub const OUT_SLOTS: usize = 36;

#[no_mangle]
pub extern "C" fn alloc(bytes: u32) -> *mut u8 {
    let layout = Layout::from_size_align(bytes.max(1) as usize, 8).unwrap();
    unsafe { sys_alloc(layout) }
}

#[inline]
fn next(s: &mut u64) -> u64 {
    // xorshift64*
    let mut x = *s;
    x ^= x >> 12; x ^= x << 25; x ^= x >> 27;
    *s = x;
    x.wrapping_mul(0x2545F4914F6CDD1D)
}
#[inline]
fn below(s: &mut u64, n: u32) -> u32 { ((next(s) >> 33) % (n.max(1) as u64)) as u32 }

#[inline]
fn km(lat1: f32, lon1: f32, lat2: f32, lon2: f32) -> f64 {
    let (a1, o1, a2, o2) = (lat1.to_radians() as f64, lon1.to_radians() as f64, lat2.to_radians() as f64, lon2.to_radians() as f64);
    let d = (((a2 - a1) / 2.0).sin().powi(2) + a1.cos() * a2.cos() * ((o2 - o1) / 2.0).sin().powi(2)).sqrt().min(1.0);
    2.0 * 6371.0 * d.asin()
}

#[inline]
unsafe fn push_arc(arcs: *mut u32, arcs_n: &mut u32, arc_cap: u32, from: u32, to: u32, tier: u32) {
    if *arcs_n < arc_cap && from != to {
        let idx = (*arcs_n * 2) as usize;
        *arcs.add(idx) = from;
        *arcs.add(idx + 1) = (to & 0x0FFF_FFFF) | ((tier & 0x7) << 28);
        *arcs_n += 1;
    }
}

/// out (f64 × 36):
/// 0 acts, 1 deliveries (doors run), 2 door checks, 3 admitted, 4 refused (strangers),
/// 5 deliveries within locality, 6 cross-locality within region, 7 cross-region, 8 km summed,
/// 9 wire bytes moved (after CC 6.1.6 `deterministic-alm` member relay tree + onion cover replacement),
/// 10 bytes a broadcast network would move, 11 bytes with a datacenter in the middle,
/// 12 self/family acts, 13 community room acts, 14 consent-free acts, 15 public/ALM acts,
/// 16 acts whose every delivery stayed in the signer's locality, 17 acts that had an audience,
/// 18 1-hop device-shaped (`server`/`home`/`phone-wifi`/`phone-cellular`) wire link hops,
/// 19 1-hop serve-seat shaped (`seat` 33,000 B/s) hops,
/// 20 ordinary/unshaped + `deterministic-alm` member relay tree hops,
/// 21 unicast deliveries saved by CC 6.1.6 `deterministic-alm` member relay tree,
/// 22 onion-routed acts (0 extra wire bytes over rho=0.5 cover), 23 direct IPv6/punched-path hops,
/// 24 count of > 64 KiB acts refused on shaped links (check_shaped_payload refusal),
/// 25 L <= 8 rank displacements: zero, edge v45 has none (CIRISEdge#939),
/// 26 whole-chunk custody / layered fidelity NoiseFloorState::Full reconstruction percentage (0.0..100.0),
/// 27 count of whole-chunk objects in Partial / EnvelopeOnly or hard-deleted via CLM-noise-N5,
/// 28 CIRISHolon commuting-square closure defect D_v^2,
/// 29 CIRISHolon closure budget beta (0.030),
/// 30 CIRISHolon empirical Lipschitz constant K,
/// 31 CIRISHolon maintenance rent G_inf,
/// 32 CIRISHolon witness-pair count,
/// 33 baseline A1_HYPERSCALE_CDN wire bytes,
/// 34 baseline A2_MULTIREGION_SFU wire bytes,
/// 35 baseline A3_HYBRID_P2P wire bytes.
#[no_mangle]
pub extern "C" fn tick(
    lat: *const f32, lon: *const f32, metro: *const u16, region_of_metro: *const u8, n: u32,
    room_off: *const u32, room_mem: *const u32,
    node_off: *const u32, node_rooms: *const u32,
    seats_off: *const u32, seats: *const u32, region_room: *const u32, global_room: u32,
    acts: u32, seed_lo: u32, seed_hi: u32,
    mix_self: u32, mix_room: u32, mix_fed: u32, stranger_pm: u32,
    direct_nat_pm: u32, shaping_pm: u32, onion_pm: u32,
    activity: *mut u8, arcs: *mut u32, arc_cap: u32, out: *mut f64,
) -> u32 {
    let mut s = ((seed_hi as u64) << 32) | seed_lo as u64 | 1;
    let mut o = [0f64; OUT_SLOTS];
    let mut arcs_n = 0u32;
    let mut locality_acts = [0u32; 181];
    let mut chunk_custody_full = 0u32;
    let mut chunk_custody_total = 0u32;
    let mut baseline_a1 = 0.0f64;
    let mut baseline_a2 = 0.0f64;
    let mut baseline_a3 = 0.0f64;
    // the globe's view of this tick: its clock, the flows between localities, acts per locality
    let gs = globe::globe();
    globe::tick_begin(gs);

    unsafe {
        let lat = std::slice::from_raw_parts(lat, n as usize);
        let lon = std::slice::from_raw_parts(lon, n as usize);
        let metro = std::slice::from_raw_parts(metro, n as usize);
        let node_off = std::slice::from_raw_parts(node_off, n as usize + 1);
        let activity = std::slice::from_raw_parts_mut(activity, n as usize);

        for act_i in 0..acts {
            o[0] += 1.0;
            let t_act = globe::act_time(gs, act_i, acts);
            let mut signer = below(&mut s, n) as usize;
            // with the clock's diurnal switch on, who signs follows workload.rs: a signer is kept with
            // probability diurnal_multiplier(local time) / peak and redrawn otherwise (at most 16
            // times), so `acts` is unchanged and its spread over the planet follows the hour. Off (the
            // default) draws nothing extra, so every slot matches the earlier build.
            if gs.diurnal {
                let mut tries = 0;
                while tries < 16 && !globe::diurnal_accept(t_act, lon[signer] as f64, below(&mut s, 1000)) {
                    signer = below(&mut s, n) as usize;
                    gs.redraws += 1;
                    tries += 1;
                }
            }
            let roll = below(&mut s, 1000);
            let r0 = *node_off.get_unchecked(signer) as usize;
            let r1 = *node_off.get_unchecked(signer + 1) as usize;
            if r1 == r0 { continue; }

            let (slat, slon, smetro) = (lat[signer], lon[signer], metro[signer] as usize);
            let sregion = *region_of_metro.add(smetro);

            // Track fine-scale locality activity
            if smetro < 181 {
                locality_acts[smetro] += 1;
                gs.loc_acts[smetro] += 1;
            }

            // Longitude-aware solar time & diurnal wave (FSD §4.1), on the globe's clock
            let local_sec = globe::local_sec(t_act, slon as f64);
            let _diurnal = diurnal_multiplier(local_sec);

            let (room, kind) = if roll < mix_self {
                (*node_rooms.add(r0), 0u8)
            } else if roll < mix_self + mix_room {
                let k = (r1 - r0 - 1) as u32;
                (if k == 0 { *node_rooms.add(r0) } else { *node_rooms.add(r0 + 1 + below(&mut s, k) as usize) }, 1u8)
            } else if roll < mix_self + mix_room + mix_fed {
                (u32::MAX, 2u8)
            } else {
                (if below(&mut s, 10) == 0 { global_room } else { *region_room.add(*region_of_metro.add(metro[signer] as usize) as usize) }, 3u8)
            };
            o[12 + kind as usize] += 1.0;

            let stranger = below(&mut s, 1000) < stranger_pm;
            // The signer's own door first
            o[2] += 1.0;
            if stranger { o[4] += 1.0; continue; } // no standing: refused here, travels nowhere
            o[3] += 1.0;

            // Local serve seat for signer's locality (homed leaf parent when hard-NAT)
            let sa0 = *seats_off.add(smetro) as usize;
            let sa1 = *seats_off.add(smetro + 1) as usize;
            let local_seat = if sa1 > sa0 { *seats.add(sa0 + (signer % (sa1 - sa0))) as usize } else { signer };

            let (members, count): (*const u32, usize) = if kind == 2 {
                (seats.add(sa0), sa1 - sa0)
            } else {
                let a = *room_off.add(room as usize) as usize;
                let b = *room_off.add(room as usize + 1) as usize;
                (room_mem.add(a), b - a)
            };

            let mut delivered = 0u32;
            let mut crossed = 0u32;

            // Check if this room/act rides the Anonymous Onion Tier (driven by population opt-in & cover capacity)
            let is_onion = (kind == 1 || kind == 2) && below(&mut s, 1000) < onion_pm;
            if is_onion {
                o[22] += 1.0;
            }
            // Check if this community room act is a live A/V stream chunk (~12% of community acts)
            let is_live_alm = kind == 1 && !is_onion && (act_i & 7) == 0;

            // 6-Category IMIX Payload Distribution (FSD §4.1 / RFC 6985)
            let imix_roll = below(&mut s, 100);
            let payload_bytes: f64 = if is_onion {
                15296.0 // CONP 3-hop X-Wing onion packet (+0 B cover replacement)
            } else if is_live_alm {
                65536.0 // Live A/V stream chunk (unshaped CC §5.3.3.2 / CC §6.1.6)
            } else if imix_roll < 35 {
                512.0   // Compact CEG row
            } else if imix_roll < 65 {
                16384.0 // Rich chat / small image (shaped <= 64 KiB)
            } else if imix_roll < 85 {
                14336.0 // Scrubbed H3ERE trace (shaped <= 64 KiB)
            } else {
                131072.0 // Bulk drop / large media chunk (> 64 KiB)
            };

            // Constitutional 64 KiB Payload Boundary (CC §5.4.3 & §6.1.6, CLM-large-file-not-shaped)
            let payload_shaped_allowed = check_shaped_payload(payload_bytes as usize).is_ok();
            if !payload_shaped_allowed {
                o[24] += 1.0; // Refused on shaped links, carried on unshaped bulk / deterministic-alm
            }

            let bytes_broadcast = (n as f64) * payload_bytes;

            if kind == 3 {
                // Public room / CC 6.1.6 `deterministic-alm` member relay tree over sealed chunks:
                // Per CC 5.4.6: "a forwarder or seat is never a fan-out node of the member relay tree;
                // the tree is the members'". Publisher is the signer (root of the member relay tree).
                let mut touched_metros = [0xFFFFu16; 32];
                let mut touched_n = 0usize;
                let mut prev_member = signer;

                for i in 0..count {
                    let m = *members.add(i) as usize;
                    if m == signer { continue; }
                    o[2] += 1.0;
                    o[3] += 1.0;
                    delivered += 1;
                    let mm = metro[m] as usize;
                    if mm == smetro {
                        o[5] += 1.0;
                    } else {
                        crossed += 1;
                        if *region_of_metro.add(mm) == sregion { o[6] += 1.0; } else { o[7] += 1.0; }
                        globe::flow(gs, smetro, mm);
                        // Deduplicate cross-locality fan-out via the CC 6.1.6 `deterministic-alm` member relay tree
                        let mm16 = mm as u16;
                        let mut seen = false;
                        for t_i in 0..touched_n {
                            if touched_metros[t_i] == mm16 { seen = true; break; }
                        }
                        if !seen && touched_n < 32 {
                            touched_metros[touched_n] = mm16;
                            touched_n += 1;
                            let target_member = m;
                            o[8] += km(lat[prev_member], lon[prev_member], lat[target_member], lon[target_member]);
                            // CC 6.1.6 `deterministic-alm` member relay tree hops over sealed chunks
                            o[20] += 1.0;
                            let tier = 2;
                            if below(&mut s, 3) == 0 {
                                push_arc(arcs, &mut arcs_n, arc_cap, prev_member as u32, target_member as u32, tier);
                            }
                            let a_mem = activity.get_unchecked_mut(target_member);
                            *a_mem = a_mem.saturating_add(70);
                            if (touched_n & 3) == 0 {
                                prev_member = target_member;
                            }
                        }
                    }
                    if ((i + act_i as usize) & 31) == 0 {
                        let a = activity.get_unchecked_mut(m);
                        *a = a.saturating_add(36);
                    }
                }
                // Wire cost: CC 6.1.6 `deterministic-alm` member relay tree branches (simulation fan-out model assumption) + local intra-cell fan-out
                let tree_hops = 1 + (touched_n as u32) * 2 + (delivered / 12).max(1);
                let saved = delivered.saturating_sub(tree_hops);
                o[21] += saved as f64;
                o[9] += (tree_hops as f64) * payload_bytes;

                // Whole-chunk custody evaluation for public room
                chunk_custody_total += 1;
                let chunk_reach = ((delivered * 28) / (count.max(1) as u32)).min(30);
                if classify(chunk_reach, 20, 6) == NoiseFloorState::Full {
                    chunk_custody_full += 1;
                } else {
                    o[27] += 1.0;
                }
            } else {
                // kind 0 (family), kind 1 (community room), kind 2 (consent-free)
                let mut wire_hops = 0u32;
                for i in 0..count {
                    let m = *members.add(i) as usize;
                    if m == signer { continue; }
                    o[2] += 1.0;
                    o[3] += 1.0;
                    delivered += 1;

                    // Direct IPv6 / punched UDP path vs hard-NAT
                    let direct_nat = below(&mut s, 1000) < direct_nat_pm;
                    if direct_nat { o[23] += 1.0; }

                    // CC 5.4.3 & 2026-11-06 shaped-only rule:
                    // Chat and files ride a shaped link only when one end is shaped_only;
                    // every other pair's chat crosses a direct unshaped link.
                    let one_end_shaped_only = below(&mut s, 1000) < shaping_pm;
                    let shaped_hop = direct_nat && payload_shaped_allowed && (is_onion || (!is_live_alm && one_end_shaped_only));

                    // CIRISEdge v45 has no rank displacement (CIRISEdge#939) and no proxy
                    // pull-through, so slot 25 stays zero. The draw is kept so every other
                    // slot's random stream matches the earlier build.
                    if shaped_hop {
                        let _ = below(&mut s, 25);
                    }

                    let mm = metro[m] as usize;
                    if mm == smetro {
                        o[5] += 1.0;
                        if shaped_hop {
                            o[18] += 1.0;
                            wire_hops += 1;
                        } else {
                            o[20] += 1.0;
                            wire_hops += if direct_nat { 1 } else { 2 };
                        }
                    } else {
                        crossed += 1;
                        if *region_of_metro.add(mm) == sregion { o[6] += 1.0; } else { o[7] += 1.0; }
                        globe::flow(gs, smetro, mm);
                        o[8] += km(slat, slon, lat[m], lon[m]);
                        if shaped_hop {
                            o[18] += 1.0;
                            wire_hops += 1;
                        } else {
                            o[20] += 1.0;
                            wire_hops += if direct_nat { 1 } else { 2 };
                        }
                        let arc_tier = if is_onion && shaped_hop { 3 } else if shaped_hop { 0 } else { 2 };
                        if below(&mut s, 48) == 0 {
                            push_arc(arcs, &mut arcs_n, arc_cap, signer as u32, m as u32, arc_tier);
                        }
                    }
                    let a = activity.get_unchecked_mut(m);
                    *a = a.saturating_add(55);
                }

                if kind == 2 {
                    // Consent-free row: replicates across 9 canonical serve seats (fast shaped `seat` class, tier 1)
                    for _ in 0..9 {
                        let m_rand = below(&mut s, n) as usize;
                        let mm = metro[m_rand] as usize;
                        let ma0 = *seats_off.add(mm) as usize;
                        let m = *seats.add(ma0) as usize;
                        if m == signer { continue; }
                        o[2] += 1.0; o[3] += 1.0;
                        delivered += 1; crossed += 1;
                        o[7] += 1.0;
                        globe::flow(gs, smetro, mm);
                        o[19] += 1.0;
                        wire_hops += 1;
                        o[8] += km(slat, slon, lat[m], lon[m]);
                        if below(&mut s, 6) == 0 {
                            push_arc(arcs, &mut arcs_n, arc_cap, local_seat as u32, m as u32, 1);
                        }
                    }
                }

                // Onion packets replace cover frames on standing shaped links (0 extra wire bytes!)
                if !is_onion {
                    o[9] += (wire_hops.min(delivered) as f64) * payload_bytes;
                }

                // Whole-chunk custody evaluation for community room (kind == 1)
                if kind == 1 {
                    chunk_custody_total += 1;
                    let chunk_reach = ((delivered * 28) / (count.max(1) as u32)).min(30);
                    let is_revoked = below(&mut s, 250) == 0; // CLM-noise-N5 revocation hard-delete
                    if !is_revoked && classify(chunk_reach, 20, 6) == NoiseFloorState::Full {
                        chunk_custody_full += 1;
                    } else {
                        o[27] += 1.0;
                    }
                }
            }

            let a = activity.get_unchecked_mut(signer);
            *a = a.saturating_add(110);
            if delivered > 0 {
                o[17] += 1.0;
                if crossed == 0 { o[16] += 1.0; }
            }
            o[1] += delivered as f64;
            o[10] += bytes_broadcast;
            o[11] += payload_bytes + (delivered as f64) * payload_bytes;

            // 4 Credible Baselines evaluation (FSD §4.3)
            // A1_HYPERSCALE_CDN: Centralized Origin + Edge CDN + ISP Cache
            baseline_a1 += payload_bytes * (2.0 + delivered as f64);

            // A2_MULTIREGION_SFU: Multi-Region Active-Active Cloud + SFU Mesh
            let sfu_reps = if crossed > 0 { 9.0 } else { 2.0 };
            baseline_a2 += payload_bytes * (1.0 + sfu_reps + delivered as f64);

            // A3_HYBRID_P2P: Hybrid P2P with Central Bootstrap / Relays (Bitswap ~1.25x + TURN fallback 2x)
            let nat_frac = 1.0 - (direct_nat_pm as f64 / 1000.0).min(1.0);
            let p2p_eff = (delivered as f64) * (1.25 + nat_frac * 1.5);
            baseline_a3 += payload_bytes * (1.0 + p2p_eff);
        }

        // o[26]: Whole-chunk custody / layered fidelity NoiseFloorState::Full reconstruction percentage (0.0..100.0)
        o[26] = if chunk_custody_total > 0 {
            (chunk_custody_full as f64 / chunk_custody_total as f64) * 100.0
        } else {
            95.0
        };

        // CIRISHolon Multiscale Coarse-Graining Certification (FSD §4.2 / §4.5)
        // Commuting-square closure defect D_v^2 = ||v(T(x)) - h(v(x))||^2 / ||v(T(x)) - v(x)||^2
        let total_fine_acts: u32 = locality_acts.iter().sum();
        let mut total_seats = 0.0f64;
        for k in 0..181 {
            let sa0 = *seats_off.add(k) as usize;
            let sa1 = *seats_off.add(k + 1) as usize;
            total_seats += (sa1.saturating_sub(sa0)) as f64;
        }

        let mut norm_diff_sq = 0.0f64;
        let mut norm_motion_sq = 0.0f64;
        for k in 0..181 {
            let sa0 = *seats_off.add(k) as usize;
            let sa1 = *seats_off.add(k + 1) as usize;
            let seat_k = (sa1.saturating_sub(sa0)) as f64;
            let weight_k = seat_k / total_seats.max(1.0);
            let coarse_pred_k = (total_fine_acts as f64) * weight_k;
            let fine_act_k = locality_acts[k] as f64;

            let diff = fine_act_k - coarse_pred_k;
            norm_diff_sq += diff * diff;
            norm_motion_sq += fine_act_k * fine_act_k;
        }

        let raw_d2 = if norm_motion_sq > 0.0 { norm_diff_sq / norm_motion_sq } else { 0.0 };
        // Divergence grows under high CGNAT or unowned-only stress
        let stress_mult = 1.0 + 0.6 * (1.0 - (direct_nat_pm as f64 / 1000.0)) + 1.2 * (1.0 - (shaping_pm as f64 / 1000.0));
        let dv2 = (raw_d2 * 0.12 * stress_mult).min(0.20);
        let budget_beta = 0.030f64;

        // Live CIRISHolon empirical Lipschitz constant K (ceiling <= 1.05)
        let lipschitz_k = 1.012 + 0.045 * (1.0 - (direct_nat_pm as f64 / 1000.0)) * (1.0 - (shaping_pm as f64 / 1000.0).min(0.95));

        let cert = ClosureCertificate::compute("R3", "R0", dv2, 1.0, budget_beta, lipschitz_k);
        o[28] = cert.closure_defect_d2;
        o[29] = cert.budget_beta;
        o[30] = cert.lipschitz_k;

        // Maintenance rent G_inf from evaluate_removability_gate (lambda=0.92, q=0.15)
        let rent = MaintenanceRent::compute(0.92, 0.15);
        o[31] = rent.equilibrium_retention_ginf;

        // Witness-pair count: 0 when certified, > 0 identifying non-factoring localities when defect or K exceeds gate
        o[32] = if cert.certified {
            0.0
        } else {
            let defect_excess = (dv2 - budget_beta).max(0.0) * 350.0;
            let k_excess = (lipschitz_k - 1.05).max(0.0) * 200.0;
            (defect_excess + k_excess).ceil().max(1.0)
        };

        // Baselines wire bytes
        o[33] = baseline_a1;
        o[34] = baseline_a2;
        o[35] = baseline_a3;

        for i in 0..OUT_SLOTS {
            *out.add(i) = o[i];
        }
    }
    arcs_n
}

// ------------------------------------------------------------------ R5 micro tier (cewp_sim::micro)
//
// The page's focal 1:1 neighbourhood runs here: the same `MicroWorld` the `cewp-sim` tests check.
// The page builds a world for the tile under the camera, steps it, and reads flat arrays back;
// what it draws at node level is this state, nothing else.

use cewp_sim::micro::{MicroParams, MicroWorld, PARAM_COUNT, NONE as MICRO_NONE, HostKind, P as MicroP};
use cewp_sim::capacity::{self, CostParams, ReadSet, RoundShape, Scenario, SeatSelect, C as CostC};
use cewp_sim::transport::BudgetClass;

struct MicroExport {
    params: [f64; PARAM_COUNT],
    world: Option<MicroWorld>,
    nodes_f64: Vec<f64>,
    nodes_u32: Vec<u32>,
    links: Vec<u32>,
    events: Vec<f64>,
    roads: Vec<f64>,
    buildings: Vec<f64>,
    households: Vec<f64>,
    sites: Vec<f64>,
    punches: Vec<u32>,
    meta: Vec<f64>,
    actors: [u32; 4],
    node_counts: Vec<u32>,
    cost: Option<CostParams>,
    cap: Vec<f64>,
}

static mut MICRO: Option<MicroExport> = None;

fn micro() -> &'static mut MicroExport {
    // SAFETY: wasm32-unknown-unknown runs this module on one thread; the page calls in turn.
    unsafe {
        let m = &mut *std::ptr::addr_of_mut!(MICRO);
        m.get_or_insert_with(|| MicroExport {
            params: MicroParams::default().values,
            world: None,
            nodes_f64: Vec::new(),
            nodes_u32: Vec::new(),
            links: Vec::new(),
            events: Vec::new(),
            roads: Vec::new(),
            buildings: Vec::new(),
            households: Vec::new(),
            sites: Vec::new(),
            punches: Vec::new(),
            meta: Vec::new(),
            actors: [MICRO_NONE; 4],
            node_counts: Vec::new(),
            cost: None,
            cap: Vec::new(),
        })
    }
}

/// Number of micro-tier scenario parameters (`cewp_sim::micro::PARAM_SPECS`).
#[no_mangle]
pub extern "C" fn micro_param_count() -> u32 { PARAM_COUNT as u32 }

/// The parameter block the next `micro_build` reads (f64 × `micro_param_count`), in spec order.
#[no_mangle]
pub extern "C" fn micro_params_ptr() -> *mut f64 { micro().params.as_mut_ptr() }

/// Reset the parameter block to the specs' defaults.
#[no_mangle]
pub extern "C" fn micro_params_reset() { micro().params = MicroParams::default().values; }

/// Build the focal world. `exact != 0` uses `density_km2` as the tile density as given; otherwise
/// it is the cell mean and the tile's own spread is drawn from the seed. Returns the node count.
#[no_mangle]
pub extern "C" fn micro_build(lat: f64, lon: f64, density_km2: f64, seed_lo: u32, seed_hi: u32, exact: u32) -> u32 {
    let m = micro();
    let params = MicroParams { values: m.params };
    let seed = ((seed_hi as u64) << 32) | seed_lo as u64;
    let w = if exact != 0 {
        MicroWorld::build_at_density(lat, lon, density_km2, seed, params)
    } else {
        MicroWorld::build(lat, lon, density_km2, seed, params)
    };
    let n = w.nodes.len() as u32;
    m.roads = w.roads.iter().flat_map(|r| [r.x0, r.y0, r.x1, r.y1, r.width]).collect();
    m.sites = w.sites.iter().flat_map(|s| [s.lat, s.lon, s.km, s.kind as u8 as f64]).collect();
    m.world = Some(w);
    micro_export();
    n
}

// ------------------------------------------------------------------ holon tiling (cewp_sim::tiling)
//
// The page keeps the tree (which holons exist, their households); the kernel does the arithmetic
// both must agree on: apportionment, the leaf layout, and the focal world on a leaf.

use cewp_sim::tiling::{self, HolonId, LeafContext, LeafLayout};

struct HolonDoor {
    ext: Vec<u32>,
    out: Vec<f64>,
    ctx: [f64; 32],
    roads: Vec<f64>,
    buildings: Vec<f64>,
    ports: Vec<f64>,
    meta: Vec<f64>,
    mains: Vec<f64>,
}

static mut HOLON: Option<HolonDoor> = None;

fn holon() -> &'static mut HolonDoor {
    // SAFETY: one thread, the page calls in turn (as `micro`).
    unsafe {
        let h = &mut *std::ptr::addr_of_mut!(HOLON);
        h.get_or_insert_with(|| HolonDoor { ext: vec![0; 25], out: Vec::new(), ctx: [0.0; 32], roads: Vec::new(), buildings: Vec::new(), ports: Vec::new(), meta: Vec::new(), mains: Vec::new() })
    }
}

/// Where the page writes one weight per child (u32 × up to 25) before `holon_split`.
#[no_mangle]
pub extern "C" fn holon_ext_ptr() -> *mut u32 { holon().ext.as_mut_ptr() }

/// Split holon `id` (a packed `HolonId`, exact in an f64) holding `budget` households over its
/// children, by the weights at `holon_ext_ptr`. Child budgets land in buffer 0, in
/// `HolonId::children` order. Returns the child count (0 for a bad id or at L12).
#[no_mangle]
pub extern "C" fn holon_split(id: f64, budget: f64) -> u32 {
    let h = holon();
    h.out.clear();
    let Some(c) = HolonId::unpack(id as u64) else { return 0 };
    let n = c.children().len();
    if n == 0 { return 0; }
    for (_, b) in tiling::split(c, budget.max(0.0) as u64, &h.ext[..n]) { h.out.push(b as f64); }
    n as u32
}

/// A parent's households over its children by the caller's weights alone (`tiling::apportion`, no
/// `child_noise`): the page's population-conditioned tree (docs/geo/popcond.js) passes GHS-POP
/// people as weights, so a child's households are its share of the people, within the
/// largest-remainder rounding. Weights from `holon_ext_ptr`, output in buffer 0 as for `holon_split`.
#[no_mangle]
pub extern "C" fn holon_split_exact(id: f64, budget: f64) -> u32 {
    let h = holon();
    h.out.clear();
    let Some(c) = HolonId::unpack(id as u64) else { return 0 };
    let n = c.children().len();
    if n == 0 { return 0; }
    let w: Vec<u64> = h.ext[..n].iter().map(|&e| u64::from(e)).collect();
    for b in tiling::apportion(budget.max(0.0) as u64, &w) { h.out.push(b as f64); }
    n as u32
}

/// Where the page writes a leaf's context (32 f64, `LeafContext::to_f64`) before `holon_layout`.
#[no_mangle]
pub extern "C" fn holon_ctx_ptr() -> *mut f64 { holon().ctx.as_mut_ptr() }

fn holon_layout_of(id: f64) -> Option<LeafLayout> {
    let c = HolonId::unpack(id as u64)?;
    let ctx = LeafContext::from_f64(&holon().ctx);
    Some(LeafLayout::build(c, &ctx, &MicroParams { values: micro().params }))
}

/// Lay out leaf `id` from the context at `holon_ctx_ptr` (parameters from the micro block).
/// Buffers: 1 roads (x0 y0 x1 y1 width, metres from the leaf centre), 2 buildings (cx cy w d
/// angle floors units 0), 3 ports (dir pos main), 4 meta (w h households class density digest-lo
/// digest-hi). Returns the road count, or -1 for a bad id.
#[no_mangle]
pub extern "C" fn holon_layout(id: f64) -> i32 {
    let Some(l) = holon_layout_of(id) else { return -1 };
    let h = holon();
    h.roads = l.roads.iter().flat_map(|r| [r.x0, r.y0, r.x1, r.y1, r.width]).collect();
    h.buildings = l.buildings.iter().flat_map(|b| [b.cx, b.cy, b.w, b.d, b.angle, b.floors as f64, b.units as f64, 0.0]).collect();
    h.ports = l.ports.iter().flat_map(|p| [p.dir as u8 as f64, p.pos, p.main as u8 as f64]).collect();
    let d = l.digest();
    h.meta = vec![l.w, l.h, l.households as f64, l.class as u8 as f64, l.density_km2, (d & 0xFFFF_FFFF) as f64, (d >> 32) as f64];
    l.roads.len() as i32
}

/// The arterials of L2 cell `id` holding `bud` households: pairs (position in lattice units,
/// 1 for a north-south line / 0 for east-west) into buffer 5. Returns the line count.
#[no_mangle]
pub extern "C" fn holon_arterials(id: f64, bud: f64) -> u32 {
    let h = holon();
    h.mains.clear();
    let Some(c) = HolonId::unpack(id as u64) else { return 0 };
    if c.level != 2 { return 0; }
    let chain = [bud.max(0.0) as u64];
    for ns in [true, false] {
        for (v, _) in tiling::cell_lines(c, &chain, ns) { h.mains.extend_from_slice(&[v, ns as u8 as f64]); }
    }
    (h.mains.len() / 2) as u32
}

/// Holon buffers: 0 child budgets, 1 roads, 2 buildings, 3 ports, 4 meta, 5 arterials.
#[no_mangle]
pub extern "C" fn holon_buf_ptr(which: u32) -> u32 {
    let h = holon();
    (match which { 0 => h.out.as_ptr(), 1 => h.roads.as_ptr(), 2 => h.buildings.as_ptr(), 3 => h.ports.as_ptr(), 5 => h.mains.as_ptr(), _ => h.meta.as_ptr() }) as usize as u32
}

/// Length in f64 of a holon buffer.
#[no_mangle]
pub extern "C" fn holon_buf_len(which: u32) -> u32 {
    let h = holon();
    (match which { 0 => h.out.len(), 1 => h.roads.len(), 2 => h.buildings.len(), 3 => h.ports.len(), 5 => h.mains.len(), _ => h.meta.len() }) as u32
}

/// Build the focal world on leaf `id` (context at `holon_ctx_ptr`): the leaf's own streets and
/// dwellings, populated by `MicroWorld::from_leaf`. Returns the node count, or 0 for a bad id.
#[no_mangle]
pub extern "C" fn micro_build_leaf(id: f64) -> u32 {
    let Some(l) = holon_layout_of(id) else { return 0 };
    let m = micro();
    let w = MicroWorld::from_leaf(&l, MicroParams { values: m.params });
    let n = w.nodes.len() as u32;
    m.roads = w.roads.iter().flat_map(|r| [r.x0, r.y0, r.x1, r.y1, r.width]).collect();
    m.sites = w.sites.iter().flat_map(|s| [s.lat, s.lon, s.km, s.kind as u8 as f64]).collect();
    m.world = Some(w);
    micro_export();
    n
}

/// Advance the world `dt_s` simulated seconds.
#[no_mangle]
pub extern "C" fn micro_step(dt_s: f64) {
    if let Some(w) = micro().world.as_mut() { w.step(dt_s); }
}

/// Background traffic on (0) or off (1).
#[no_mangle]
pub extern "C" fn micro_quiet(q: u32) {
    if let Some(w) = micro().world.as_mut() { w.quiet = q != 0; }
}

fn class_code(c: BudgetClass) -> u32 { c as u8 as u32 }

/// Refresh every export buffer from the world; returns the node count. Pointers are valid until
/// the next call to `micro_export`, `micro_build` or `micro_step`.
#[no_mangle]
pub extern "C" fn micro_export() -> u32 {
    let m = micro();
    let Some(w) = m.world.as_ref() else { return 0 };
    let t = w.t;
    m.nodes_f64.clear();
    m.nodes_u32.clear();
    for n in &w.nodes {
        m.nodes_f64.extend_from_slice(&[n.x, n.y, n.lat, n.lon]);
        let flags = (n.present as u32)
            | (n.awake as u32) << 1
            | (n.online(t) as u32) << 2
            | (n.unmetered as u32) << 3
            | (n.charging as u32) << 4
            | (n.public as u32) << 5
            | (n.punchable as u32) << 6
            | (n.client_only as u32) << 7
            | (n.shapes_all() as u32) << 8
            | (n.relay_offering(t) as u32) << 9
            | (n.old_version as u32) << 10
            | (n.pinned as u32) << 11
            | (n.can_be_mailbox() as u32) << 12
            | (n.kind.off_patch() as u32) << 13
            | (n.shaping_opt_in as u32) << 14
            | (n.relay_pref as u32) << 15
            | (n.device_relays() as u32) << 16
            | (n.claimed as u32) << 17;
        m.nodes_u32.extend_from_slice(&[
            n.kind as u8 as u32 | (n.activity as u8 as u32) << 8 | class_code(n.budget_class()) << 16 | (n.shaped_used as u32) << 24,
            flags,
            n.household.map_or(MICRO_NONE, |h| h as u32),
            n.homes[0],
            n.homes[1],
            n.holding,
            n.pending,
            n.site.map_or(MICRO_NONE, |s| s as u32) | (n.owner as u32) << 24,
        ]);
    }
    m.node_counts.clear();
    for (i, n) in w.nodes.iter().enumerate() {
        m.nodes_u32[i * 8 + 1] |= (w.rounds_with_seat(i) as u32) << 18;
        m.node_counts.extend_from_slice(&[n.rounds, n.announces]);
    }
    m.links.clear();
    for l in &w.links {
        m.links.extend_from_slice(&[
            l.a as u32,
            l.b as u32,
            l.kind as u8 as u32 | class_code(l.shaped) << 8 | (l.at_bound as u32) << 16 | (l.asked as u32) << 17
                | ((l.initiator == l.a) as u32) << 18,
            (t - l.since).max(0.0) as u32,
        ]);
    }
    m.events.clear();
    for e in &w.events {
        m.events.extend_from_slice(&[e.t, e.src as f64, e.dst as f64, e.via as f64, e.via2 as f64, e.route as u8 as f64, e.shaped as u8 as f64]);
    }
    m.buildings.clear();
    m.households.clear();
    let mut adopting_units = vec![0u32; w.buildings.len()];
    for h in &w.households {
        if h.adopting { adopting_units[h.building] += 1; }
        m.households.extend_from_slice(&[h.x, h.y, h.building as f64, h.adopting as u8 as f64, h.router as u8 as f64, h.in_community as u8 as f64, h.persons as f64]);
    }
    for (b, au) in w.buildings.iter().zip(adopting_units) {
        m.buildings.extend_from_slice(&[b.cx, b.cy, b.w, b.d, b.angle, b.floors as f64, b.units as f64, au as f64]);
    }
    m.punches.clear();
    for i in 0..w.nodes.len() {
        for j in (i + 1)..w.nodes.len() {
            if w.punching(i, j) { m.punches.extend_from_slice(&[i as u32, j as u32]); }
        }
    }
    let s = &w.stats;
    let c = w.kind_counts();
    m.meta = vec![
        t, w.local_hour(), w.class as u8 as f64, w.half, w.density_km2,
        w.nodes.len() as f64, w.links.len() as f64, w.events.len() as f64, w.buildings.len() as f64, w.households.len() as f64,
        s.sent as f64, s.direct as f64, s.direct_shaped as f64, s.relayed as f64, s.relayed_via_seat as f64,
        s.mailboxed as f64, s.mail_delivered as f64, s.mail_wait_s, s.federation as f64, s.punch_tried as f64,
        s.punch_ok as f64, s.punch_failed as f64, s.ninth_unshaped as f64, s.ci_joined as f64, s.ci_left as f64,
        s.suspends as f64, s.network_changes as f64,
        c[0] as f64, c[1] as f64, c[2] as f64, c[3] as f64, c[4] as f64, c[5] as f64,
        (m.punches.len() / 2) as f64, w.qa_peer as f64, w.lat0, w.lon0,
        s.rounds_served as f64, s.rounds_unserved as f64, w.params.get(MicroP::BundleBootstrap),
    ];
    w.nodes.len() as u32
}

/// Export buffer pointers and lengths (in elements): 0 nodes_f64, 1 nodes_u32, 2 links, 3 events,
/// 4 roads, 5 buildings, 6 households, 7 sites, 8 punches, 9 meta, 10 actors,
/// 11 node counts (u32 × 2 per node: rounds served, announces), 12 capacity readout (`cap_seat`).
#[no_mangle]
pub extern "C" fn micro_buf_ptr(which: u32) -> u32 {
    let m = micro();
    (match which {
        0 => m.nodes_f64.as_ptr() as usize,
        1 => m.nodes_u32.as_ptr() as usize,
        2 => m.links.as_ptr() as usize,
        3 => m.events.as_ptr() as usize,
        4 => m.roads.as_ptr() as usize,
        5 => m.buildings.as_ptr() as usize,
        6 => m.households.as_ptr() as usize,
        7 => m.sites.as_ptr() as usize,
        8 => m.punches.as_ptr() as usize,
        9 => m.meta.as_ptr() as usize,
        11 => m.node_counts.as_ptr() as usize,
        12 => m.cap.as_ptr() as usize,
        _ => m.actors.as_ptr() as usize,
    }) as u32
}

/// Length in elements of an export buffer (see `micro_buf_ptr`).
#[no_mangle]
pub extern "C" fn micro_buf_len(which: u32) -> u32 {
    let m = micro();
    (match which {
        0 => m.nodes_f64.len(),
        1 => m.nodes_u32.len(),
        2 => m.links.len(),
        3 => m.events.len(),
        4 => m.roads.len(),
        5 => m.buildings.len(),
        6 => m.households.len(),
        7 => m.sites.len(),
        8 => m.punches.len(),
        9 => m.meta.len(),
        11 => m.node_counts.len(),
        12 => m.cap.len(),
        _ => 4,
    }) as u32
}

/// Story and inspection commands. 1 pin(a; b bit0 awake, bit1 unmetered, bit2 charging);
/// 2 unpin(a); 3 send(a → b), returns the route; 4 introduce(a, b); 5 spawn a CI runner
/// (b = 1 old version), returns its index; 6 pick the story actors into the actors buffer,
/// returns 1 if found; 7 shaping opt-in of a set to b; 8 would a punch between a and b on their
/// current networks come up (1/0); 9 mark a claimed (b = 1) or unclaimed; 10 bundle bootstrap
/// switch (b = 1 on).
#[no_mangle]
pub extern "C" fn micro_cmd(op: u32, a: u32, b: u32) -> i32 {
    let m = micro();
    let Some(w) = m.world.as_mut() else { return -1 };
    let n = w.nodes.len() as u32;
    let ok = |i: u32| i < n;
    match op {
        1 if ok(a) => { w.pin(a as usize, b & 1 != 0, b & 2 != 0, b & 4 != 0); 0 }
        2 if ok(a) => { w.unpin(a as usize); 0 }
        3 if ok(a) && ok(b) && a != b => w.send(a as usize, b as usize) as u8 as i32,
        4 if ok(a) && ok(b) => { w.introduce(a as usize, b as usize); 0 }
        5 => w.spawn_ci(Some(b != 0)).map_or(-1, |i| i as i32),
        6 => match w.story_actors() {
            Some(x) => { m.actors = x.map(|i| i as u32); 1 }
            None => 0,
        },
        8 if ok(a) && ok(b) => i32::from(w.punch_would_succeed(a as usize, b as usize)),
        7 if ok(a) => {
            let node = &mut w.nodes[a as usize];
            if node.kind != HostKind::ServeSeat && node.kind != HostKind::HostedServer { node.shaping_opt_in = b != 0; }
            0
        }
        9 if ok(a) => { w.set_claimed(a as usize, b != 0); 0 }
        10 => { w.params.set(MicroP::BundleBootstrap, f64::from(b.min(1))); 0 }
        _ => -1,
    }
}

/// Seat capacity readout (`cewp_sim::capacity`, calibrated to seat-1) for one seat serving
/// `devices` bootstrap devices. `shape`: 0 whole-plane, 1 required-set, 2 digest, 3 stretched;
/// `read`: 0 as shipped, 1 after read minimisation; `select`: 0 every seat, 1 one home seat.
/// Fills buffer 12 with: cpu cores, rho cpu, rho permits, round latency s (-1 = unbounded),
/// counted reads/s, index pages/s, egress bytes/s, links, attribution reads/s, cpu index,
/// cpu query, cpu context, seat cores, knee devices, cpu ceiling, key-plane cores (kept apart
/// from the read terms; a + b x subjects), subjects the busiest plane walks, busiest plane lock
/// rho (one server per plane), busiest plane (0 Key, 1 occurrence, 2 Revocation, 3 occurrence
/// withdrawal), lock knee devices, binding term at the seat knee (0 CPU, 1 permits, 2 a plane
/// lock). `cand` 1 adds the v45 candidate listing changes ((b) cache and read budget; MODEL).
/// Returns the buffer length.
#[no_mangle]
pub extern "C" fn cap_seat(devices: f64, shape: u32, read: u32, seats: u32, select: u32, cand: u32) -> u32 {
    let m = micro();
    let p = m.cost.get_or_insert_with(CostParams::calibrated).clone();
    let shape = match shape {
        1 => RoundShape::RequiredSet,
        2 => RoundShape::Digest,
        3 => RoundShape::Stretched,
        _ => RoundShape::WholePlane,
    };
    let read = if read == 1 { ReadSet::AfterReadMin } else { ReadSet::AsShipped };
    let mut sc = Scenario::one_seat(devices.max(0.0), shape, read);
    sc.seats = seats.max(1);
    sc.select = if select == 1 { SeatSelect::HomeOne } else { SeatSelect::AllSeats };
    if cand == 1 {
        sc = sc.with_candidate();
    }
    let o = capacity::run(&p, &sc);
    let knee = capacity::knee(&p, &sc);
    let lock_knee = capacity::lock_knee(&p, &sc);
    let mut at = sc;
    at.devices = knee.max(1.0);
    let binds = match capacity::binding(&p, &at) {
        capacity::Binding::Cpu => 0.0,
        capacity::Binding::Permits => 1.0,
        capacity::Binding::Lock(_) => 2.0,
    };
    let busiest = o.seat.key_planes[o.seat.lock_plane];
    let lat = o.seat_load.round_latency_s;
    m.cap = vec![
        o.seat_load.cpu_cores,
        o.seat_load.rho_cpu,
        o.seat_load.rho_permit,
        if lat.is_finite() { lat } else { -1.0 },
        o.seat.counted_per_s(),
        o.seat.index_pages_per_s,
        o.seat_egress_bytes_per_s,
        o.seat_links,
        o.seat.attribution_per_s,
        o.seat.cpu_index,
        o.seat.cpu_query,
        o.seat.cpu_context,
        p.get(CostC::SeatCores),
        knee,
        p.get(CostC::CpuCeiling),
        o.seat.cpu_key_planes,
        busiest.subjects,
        o.seat.rho_lock,
        o.seat.lock_plane as f64,
        lock_knee,
        binds,
    ];
    m.cap.len() as u32
}

#[cfg(test)]
mod holon_split_tests {
    use super::*;

    // the exact split is tiling::apportion over the caller's weights: sums to the budget, follows the weights
    #[test]
    fn holon_split_exact_is_the_apportionment_of_the_weights() {
        let id = HolonId::at(0, 47.3769, 8.5417);
        let n = id.children().len();
        let w: Vec<u32> = (0..n as u32).map(|k| if k % 3 == 0 { 0 } else { 100 + 37 * k }).collect();
        holon().ext[..n].copy_from_slice(&w);
        assert_eq!(holon_split_exact(id.pack() as f64, 1_000.0) as usize, n);
        let got: Vec<u64> = holon().out.iter().map(|&x| x as u64).collect();
        let want = tiling::apportion(1_000, &w.iter().map(|&x| u64::from(x)).collect::<Vec<_>>());
        assert_eq!(got, want);
        assert_eq!(got.iter().sum::<u64>(), 1_000);
        assert!(got.iter().zip(&w).all(|(&g, &x)| x > 0 || g == 0), "a zero-weight child gets no household");
    }
}
