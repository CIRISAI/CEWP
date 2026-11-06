//! The kernel's rulebook at scale — CC 5.4.3 budget classes, 1-hop shaped links & CC 6.1.6 `deterministic-alm` edition (CC RC17 / Edge v44).
//!
//! One tick = `acts` random signed rows over a world of nodes and rooms placed on a land-bounded
//! population density grid. For each row the rulebook runs at the signer's own node and at every node
//! of the row's audience: object state (a new object is in state `none`, so `attest` is enabled;
//! a withdrawal needs the author), standing (the signer must be in the room's roster), and a
//! stranger is refused at the first door and travels nowhere.
//!
//! Transport follows CC 5.4.3, CC 5.4.6, CC 6.1.6, and Edge v44:
//!   1. Device-class shaped links (CC 5.4.3 budget classes: `server` 7,000 B/s, `home` 2,800 B/s,
//!      `phone-wifi` 1,400 B/s, `phone-cellular` 350 B/s, L <= 8 links, rho = 0.5, W = 600 s or
//!      3,600 s on cellular; frame = channel payload MDU, ~400 B at Reticulum base MTU or ~16 KB
//!      over TCP). Per CC 5.4.3 ("A shaped link is one hop"), a shaped link is one direct hop
//!      between its two ends, never forwarded through a third node; a peer with no direct path
//!      gets no shaped link. Carries identity, consent, chat, and small attachments (<= 64 KiB),
//!      consent-free stranger roads, and 15,296 B X-Wing `CONP` onion packets replacing cover
//!      frames at +0 extra wire bytes.
//!   2. Serve-seat shaped links (CC 5.4.3 `seat` class, 33,000 B/s at rho = 0.5, W = 600 s,
//!      outside the 8-link cap): direct 1-hop links between canonical serve seats.
//!   3. Ordinary & unshaped links + CC 6.1.6 `deterministic-alm` member relay tree: A/V and bulk
//!      (>64 KiB) are unshaped only; a node not opted into shaping or without a direct path carries
//!      identity, consent, chat, and small attachments on ordinary links; large-N rooms fan out
//!      over the CC 6.1.6 `deterministic-alm` member relay tree over sealed chunks (distinct from
//!      CC 5.3.3.2's realtime profile, which has no selective-forwarding relay role — no seat or
//!      third party ever decodes or selects).

use std::alloc::{alloc as sys_alloc, Layout};

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

/// out (f64 × 24):
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
/// 22 onion-routed acts (0 extra wire bytes over rho=0.5 cover), 23 direct IPv6/punched-path hops.
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
    const ROW_BYTES: f64 = 4096.0; // one COSE row with a composite ML-DSA-65 + Ed25519 signature
    let mut s = ((seed_hi as u64) << 32) | seed_lo as u64 | 1;
    let mut o = [0f64; 24];
    let mut arcs_n = 0u32;
    unsafe {
        let lat = std::slice::from_raw_parts(lat, n as usize);
        let lon = std::slice::from_raw_parts(lon, n as usize);
        let metro = std::slice::from_raw_parts(metro, n as usize);
        let node_off = std::slice::from_raw_parts(node_off, n as usize + 1);
        let activity = std::slice::from_raw_parts_mut(activity, n as usize);
        for act_i in 0..acts {
            o[0] += 1.0;
            let signer = below(&mut s, n) as usize;
            let roll = below(&mut s, 1000);
            let r0 = *node_off.get_unchecked(signer) as usize;
            let r1 = *node_off.get_unchecked(signer + 1) as usize;
            if r1 == r0 { continue; }
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
            // the signer's own door first
            o[2] += 1.0;
            if stranger { o[4] += 1.0; continue; } // no standing: refused here, travels nowhere
            o[3] += 1.0;

            let (slat, slon, smetro) = (lat[signer], lon[signer], metro[signer] as usize);
            let sregion = *region_of_metro.add(smetro);
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
            let bytes_broadcast = n as f64 * ROW_BYTES;
            let mut delivered = 0u32;
            let mut crossed = 0u32;

            // Check if this room/act rides the Anonymous Onion Tier (driven by population opt-in & cover capacity)
            let is_onion = (kind == 1 || kind == 2) && below(&mut s, 1000) < onion_pm;
            if is_onion {
                o[22] += 1.0;
            }
            // Check if this community room act is a live A/V or >64 KiB bulk stream (~12% of community acts, or displaced when unshaped)
            let is_live_alm = kind == 1 && !is_onion && (act_i & 7) == 0;

            if kind == 3 {
                // Public room / CC 6.1.6 `deterministic-alm` member relay tree over sealed chunks:
                // Every member computes the same deterministic relay tree over end-to-end sealed chunks
                // (distinct from CC 5.3.3.2's realtime transport profile, which has no selective-forwarding
                // relay role — no seat or third party ever decodes or selects).
                let mut touched_metros = [0xFFFFu16; 32];
                let mut touched_n = 0usize;
                let mut prev_hub = local_seat;

                for i in 0..count {
                    let m = *members.add(i) as usize;
                    if m == signer { continue; }
                    // The kernel's rulebook still runs at every receiving member m
                    o[2] += 1.0;
                    o[3] += 1.0;
                    delivered += 1;
                    let mm = metro[m] as usize;
                    if mm == smetro {
                        o[5] += 1.0;
                    } else {
                        crossed += 1;
                        if *region_of_metro.add(mm) == sregion { o[6] += 1.0; } else { o[7] += 1.0; }
                        // Deduplicate cross-locality fan-out via the CC 6.1.6 `deterministic-alm` member relay tree
                        let mm16 = mm as u16;
                        let mut seen = false;
                        for t_i in 0..touched_n {
                            if touched_metros[t_i] == mm16 { seen = true; break; }
                        }
                        if !seen && touched_n < 32 {
                            touched_metros[touched_n] = mm16;
                            touched_n += 1;
                            let ma0 = *seats_off.add(mm) as usize;
                            let ma1 = *seats_off.add(mm + 1) as usize;
                            let target_seat = if ma1 > ma0 { *seats.add(ma0) as usize } else { m };
                            o[8] += km(lat[prev_hub], lon[prev_hub], lat[target_seat], lon[target_seat]);
                            // CC 6.1.6 `deterministic-alm` member relay tree hops over sealed chunks
                            o[20] += 1.0;
                            let tier = 2;
                            if below(&mut s, 3) == 0 {
                                push_arc(arcs, &mut arcs_n, arc_cap, prev_hub as u32, target_seat as u32, tier);
                            }
                            let a_seat = activity.get_unchecked_mut(target_seat);
                            *a_seat = a_seat.saturating_add(70);
                            if (touched_n & 3) == 0 {
                                prev_hub = target_seat; // log-depth deterministic-alm member relay tree chaining
                            }
                        }
                    }
                    // Staggered leaf activity so a 5,000-member room never pulses the globe in lockstep
                    if ((i + act_i as usize) & 31) == 0 {
                        let a = activity.get_unchecked_mut(m);
                        *a = a.saturating_add(36);
                    }
                }
                // Wire cost: CC 6.1.6 `deterministic-alm` member relay tree branches + local intra-cell fan-out (amortized 1/12 per leaf)
                let tree_hops = 1 + (touched_n as u32) * 2 + (delivered / 12).max(1);
                let saved = delivered.saturating_sub(tree_hops);
                o[21] += saved as f64;
                o[9] += (tree_hops as f64) * ROW_BYTES;
            } else {
                // kind 0 (family), kind 1 (community room), kind 2 (consent-free)
                let mut wire_hops = 0u32;
                for i in 0..count {
                    let m = *members.add(i) as usize;
                    if m == signer { continue; }
                    o[2] += 1.0;
                    o[3] += 1.0;
                    delivered += 1;
                    // Direct IPv6 / punched UDP path vs no direct path (leviculum#100 / leviculum#101)
                    let direct_nat = below(&mut s, 1000) < direct_nat_pm;
                    if direct_nat { o[23] += 1.0; }
                    // CC 5.4.3 ("A shaped link is one hop"): a shaped link is one direct hop between its
                    // two ends, never forwarded through a third node; a peer with no direct path gets no
                    // shaped link; a node not opted in (or with no direct path) carries the same classes
                    // on ordinary links.
                    let shaped_hop = direct_nat && (is_onion || (!is_live_alm && below(&mut s, 1000) < shaping_pm));
                    let mm = metro[m] as usize;
                    if mm == smetro {
                        o[5] += 1.0;
                        if shaped_hop {
                            o[18] += 1.0;
                            wire_hops += 1; // 1 direct shaped hop
                        } else {
                            o[20] += 1.0;
                            wire_hops += if direct_nat { 1 } else { 2 };
                        }
                    } else {
                        crossed += 1;
                        if *region_of_metro.add(mm) == sregion { o[6] += 1.0; } else { o[7] += 1.0; }
                        o[8] += km(slat, slon, lat[m], lon[m]);
                        if shaped_hop {
                            // 1 direct hop on a CC 5.4.3 device budget class (server/home/phone-wifi/phone-cellular)
                            o[18] += 1.0;
                            wire_hops += 1;
                        } else {
                            // Ordinary unshaped link (or live A/V & bulk)
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
                    o[9] += (wire_hops.min(delivered) as f64) * ROW_BYTES;
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
            o[11] += ROW_BYTES + delivered as f64 * ROW_BYTES; // up to the datacenter, then down to each
        }
        for i in 0..24 { *out.add(i) = o[i]; }
    }
    arcs_n
}
