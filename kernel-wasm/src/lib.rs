//! The kernel's door at scale.
//!
//! One tick = `acts` random signed rows over a world of nodes and rooms. For each row the door
//! runs at the signer's own node and at every node of the row's audience: object state (a new
//! object is in state `none`, so `attest` is enabled; a withdrawal needs the author), standing
//! (the signer must be in the room's roster), and a stranger is refused at the first door and
//! travels nowhere. What is counted is what the constitution's locality dividend claims: how many
//! deliveries never leave the signer's metro, and how many bytes move, against a broadcast
//! network and a datacenter in the middle.
//!
//! No allocation inside a tick; the world lives in buffers the page owns. Same model as the
//! JavaScript engine in docs/globe.html, so the two can be timed against each other.

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

/// out (f64 × 16): 0 acts, 1 deliveries, 2 door checks, 3 admitted, 4 refused (strangers),
/// 5 deliveries within the metro, 6 cross-metro within the region, 7 cross-region, 8 km summed,
/// 9 bytes moved, 10 bytes a broadcast network would move, 11 bytes with a datacenter in the middle,
/// 12 self/family acts, 13 room acts, 14 consent-free acts, 15 public-room acts,
/// 16 acts whose every delivery stayed in the signer's metro, 17 acts that had an audience.
#[no_mangle]
pub extern "C" fn tick(
    lat: *const f32, lon: *const f32, metro: *const u16, region_of_metro: *const u8, n: u32,
    room_off: *const u32, room_mem: *const u32,
    node_off: *const u32, node_rooms: *const u32,
    seats_off: *const u32, seats: *const u32, region_room: *const u32, global_room: u32,
    acts: u32, seed_lo: u32, seed_hi: u32,
    mix_self: u32, mix_room: u32, mix_fed: u32, stranger_pm: u32,
    activity: *mut u8, arcs: *mut u32, arc_cap: u32, out: *mut f64,
) -> u32 {
    const ROW_BYTES: f64 = 4096.0; // one COSE row with a composite ML-DSA-65 + Ed25519 signature
    let mut s = ((seed_hi as u64) << 32) | seed_lo as u64 | 1;
    let mut o = [0f64; 20];
    let mut arcs_n = 0u32;
    unsafe {
        let lat = std::slice::from_raw_parts(lat, n as usize);
        let lon = std::slice::from_raw_parts(lon, n as usize);
        let metro = std::slice::from_raw_parts(metro, n as usize);
        let node_off = std::slice::from_raw_parts(node_off, n as usize + 1);
        let activity = std::slice::from_raw_parts_mut(activity, n as usize);
        for _ in 0..acts {
            o[0] += 1.0;
            let signer = below(&mut s, n) as usize;
            let roll = below(&mut s, 1000);
            // the signer's rooms: node_rooms[node_off[signer]..node_off[signer+1]]; the first is the family
            let r0 = *node_off.get_unchecked(signer) as usize;
            let r1 = *node_off.get_unchecked(signer + 1) as usize;
            if r1 == r0 { continue; }
            let (room, kind) = if roll < mix_self { (*node_rooms.add(r0), 0u8) }
                else if roll < mix_self + mix_room { let k = (r1 - r0 - 1) as u32; (if k == 0 { *node_rooms.add(r0) } else { *node_rooms.add(r0 + 1 + below(&mut s, k) as usize) }, 1u8) }
                else if roll < mix_self + mix_room + mix_fed { (u32::MAX, 2u8) }
                else { (if below(&mut s, 10) == 0 { global_room } else { *region_room.add(*region_of_metro.add(metro[signer] as usize) as usize) }, 3u8) };
            o[12 + kind as usize] += 1.0;
            let stranger = below(&mut s, 1000) < stranger_pm;
            // the signer's own door first
            o[2] += 1.0;
            if stranger { o[4] += 1.0; continue; } // no standing: refused here, travels nowhere
            o[3] += 1.0;
            let (slat, slon, smetro) = (lat[signer], lon[signer], metro[signer] as usize);
            let sregion = *region_of_metro.add(smetro);
            // the audience
            let (members, count): (*const u32, usize) = if kind == 2 {
                // a consent-free row: the metro's seats carry it, and sixteen seats elsewhere index it
                let a = *seats_off.add(smetro) as usize; let b = *seats_off.add(smetro + 1) as usize;
                (seats.add(a), b - a)
            } else {
                let a = *room_off.add(room as usize) as usize; let b = *room_off.add(room as usize + 1) as usize;
                (room_mem.add(a), b - a)
            };
            let bytes_broadcast = n as f64 * ROW_BYTES;
            let mut delivered = 0u32;
            let mut crossed = 0u32;
            for i in 0..count {
                let m = *members.add(i) as usize;
                if m == signer { continue; }
                // the door at m: object state none → attest enabled; standing: the signer is on the roster the
                // audience was drawn from, so it holds; a seat holds the row sealed and checks only the set
                o[2] += 1.0; o[3] += 1.0;
                delivered += 1;
                let mm = metro[m] as usize;
                if mm == smetro { o[5] += 1.0; }
                else {
                    crossed += 1;
                    if *region_of_metro.add(mm) == sregion { o[6] += 1.0; } else { o[7] += 1.0; }
                    o[8] += km(slat, slon, lat[m], lon[m]);
                    if arcs_n < arc_cap && below(&mut s, 64) == 0 { *arcs.add((arcs_n * 2) as usize) = signer as u32; *arcs.add((arcs_n * 2 + 1) as usize) = m as u32; arcs_n += 1; }
                }
                let a = activity.get_unchecked_mut(m); *a = a.saturating_add(90);
            }
            if kind == 2 {
                // sixteen seats elsewhere, cross-region by construction
                for _ in 0..16 { let m = below(&mut s, n) as usize; o[2] += 1.0; o[3] += 1.0; delivered += 1; crossed += 1; o[7] += 1.0; o[8] += km(slat, slon, lat[m], lon[m]); if arcs_n < arc_cap && below(&mut s, 8) == 0 { *arcs.add((arcs_n * 2) as usize) = signer as u32; *arcs.add((arcs_n * 2 + 1) as usize) = m as u32; arcs_n += 1; } }
            }
            let a = activity.get_unchecked_mut(signer); *a = a.saturating_add(160);
            if delivered > 0 { o[17] += 1.0; if crossed == 0 { o[16] += 1.0; } }
            o[1] += delivered as f64;
            o[9] += delivered as f64 * ROW_BYTES;
            o[10] += bytes_broadcast;
            o[11] += ROW_BYTES + delivered as f64 * ROW_BYTES; // up to the datacenter, then down to each
        }
        for i in 0..20 { *out.add(i) = o[i]; }
    }
    arcs_n
}
