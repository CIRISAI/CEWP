// The package views (HOOKS.md §2) behind one door: each `view_*` call runs a package's own
// `*_view_*` functions from cewp-sim once and leaves the flat buffers here for the page to read.
// Nothing in this file computes a number of its own; it only moves the packages' vectors into
// wasm memory. Every view is a scenario run once per page (not per frame).
//
// Slots (f32 unless noted; read with view_ptr / view_len):
//   0 topology points (stride 5)      1 topology links (u32, stride 3)   2 topology path
//   3 nat nodes (u32, stride 4)       4 nat node positions (stride 2)    5 nat pairs (u32, stride 3)
//   6 nat joint ([3, 7, 21 shares])
//   7 people localities (stride 3)    8 people week ([k * 168 + h])       9 people ties (u32, stride 3)
//  10 people flash (stride 6)
//  11 holon cells (stride 8)         12 holon bound codes (u32)         13 holon certificates u32
//  14 holon certificates f32         15 topology field classes (u32)
//  16 holon bound names: the UTF-8 bytes of BOUND_NAMES.concat(), one per u32 (slot 12 indexes them)
use cewp_sim::holon_demo::demonstration_seeded;
use cewp_sim::holon_view::{certificates_of, cells_of, holon_view_bounds, BOUND_NAMES, VIEW_HORIZON_S};
use cewp_sim::netgraph::view::{view_classes, TopologyView};
use cewp_sim::people_hooks::{people_view_flash, people_view_localities, people_view_ties, people_view_week};
use cewp_sim::reach::hooks::{nat_view_joint, nat_view_node_pos, nat_view_nodes, nat_view_pairs};

pub const SLOTS: usize = 17;
const U32_SLOTS: [bool; SLOTS] = [false, true, false, true, false, true, false, false, false, true, false, false, true, true, false, true, true];

pub struct Views {
    f: Vec<Vec<f32>>,
    u: Vec<Vec<u32>>,
    topo: Option<(u32, TopologyView)>,
}

static mut VIEWS: Option<Views> = None;

pub fn views() -> &'static mut Views {
    // SAFETY: wasm32-unknown-unknown runs this module on one thread; the page calls in turn.
    unsafe {
        let v = &mut *std::ptr::addr_of_mut!(VIEWS);
        v.get_or_insert_with(|| Views { f: vec![Vec::new(); SLOTS], u: vec![Vec::new(); SLOTS], topo: None })
    }
}

/// Topology (twin-topology, netgraph::view): fills slots 0, 1, 15 and returns the point count.
/// The graph is kept so `view_topology_path` does not rebuild it.
#[no_mangle]
pub extern "C" fn view_topology(seed: u32) -> u32 {
    let v = views();
    if v.topo.as_ref().map(|t| t.0) != Some(seed) { v.topo = Some((seed, TopologyView::new(seed))); }
    let t = &v.topo.as_ref().unwrap().1;
    v.f[0] = t.points.clone();
    v.u[1] = t.links.clone();
    v.u[15] = view_classes().into_iter().map(u32::from).collect();
    t.point_count() as u32
}

/// One route between two AS points of the last `view_topology` graph: slot 2, header
/// [AS hops, long-haul km] then lat/lon per waypoint. Returns the slot length (0: no route).
#[no_mangle]
pub extern "C" fn view_topology_path(src: u32, dst: u32) -> u32 {
    let v = views();
    v.f[2] = match &v.topo { Some((_, t)) => t.path(src, dst), None => Vec::new() };
    v.f[2].len() as u32
}

/// NAT reachability (twin-nat, reach::hooks): slots 3-6. Returns the device count.
#[no_mangle]
pub extern "C" fn view_nat(seed: u32) -> u32 {
    let s = u64::from(seed);
    let v = views();
    v.u[3] = nat_view_nodes(s);
    v.f[4] = nat_view_node_pos(s);
    v.u[5] = nat_view_pairs(s);
    v.f[6] = nat_view_joint(s);
    (v.u[3].len() / 4) as u32
}

/// People (twin-people, people_hooks): slots 7-10 from the page's decoded population grid
/// (`grid`, `len` bytes, written with `alloc`). Returns the locality count (0: bad grid).
#[no_mangle]
pub extern "C" fn view_people(grid: *const u8, len: u32, seed: u32) -> u32 {
    // SAFETY: the page wrote `len` bytes at `grid` through `alloc`
    let g = unsafe { std::slice::from_raw_parts(grid, len as usize) };
    let s = u64::from(seed);
    let v = views();
    v.f[7] = people_view_localities(g, s);
    v.f[8] = people_view_week(g, s);
    v.u[9] = people_view_ties(g, s);
    v.f[10] = people_view_flash(g, s);
    (v.f[7].len() / 3) as u32
}

/// Holon certificates (twin-holon, holon_view): runs the seeded demonstration once and fills
/// slots 11-14 and 16 from that one run. Returns the cell record count (record 0 is the patch).
#[no_mangle]
pub extern "C" fn view_holon(seed: u32) -> u32 {
    let r = demonstration_seeded(VIEW_HORIZON_S, u64::from(seed));
    let v = views();
    v.f[11] = cells_of(&r);
    v.u[12] = holon_view_bounds();
    v.u[16] = BOUND_NAMES.concat().bytes().map(u32::from).collect();
    let (cu, cf) = certificates_of(&r);
    v.u[13] = cu;
    v.f[14] = cf;
    (v.f[11].len() / 8) as u32
}

/// Address of a view slot (u32 slots and f32 slots are both 4-byte elements).
#[no_mangle]
pub extern "C" fn view_ptr(slot: u32) -> u32 {
    let v = views();
    let s = (slot as usize).min(SLOTS - 1);
    (if U32_SLOTS[s] { v.u[s].as_ptr() as usize } else { v.f[s].as_ptr() as usize }) as u32
}

/// Length in elements of a view slot.
#[no_mangle]
pub extern "C" fn view_len(slot: u32) -> u32 {
    let v = views();
    let s = (slot as usize).min(SLOTS - 1);
    (if U32_SLOTS[s] { v.u[s].len() } else { v.f[s].len() }) as u32
}

/// 1 when the slot holds u32, 0 for f32 (so the page never guesses a slot's type).
#[no_mangle]
pub extern "C" fn view_is_u32(slot: u32) -> u32 {
    u32::from(U32_SLOTS[(slot as usize).min(SLOTS - 1)])
}

#[cfg(test)]
mod tests {
    use super::*;
    use cewp_sim::netgraph::view::{topology_view_links, topology_view_points};
    use std::sync::Mutex;
    static TURN: Mutex<()> = Mutex::new(());

    // the page reads exactly what the package's own free function returns
    #[test]
    fn views_topology_matches_the_package_and_the_reported_counts() {
        let _t = TURN.lock().unwrap_or_else(|e| e.into_inner());
        let n = view_topology(7);
        assert_eq!(views().f[0], topology_view_points(7));
        assert_eq!(views().u[1], topology_view_links(7));
        // twin-topology's seed-7 numbers: 1,115 points, 3,732 links
        assert_eq!(n, 1115);
        assert_eq!(view_len(1) / 3, 3732);
        assert_eq!(view_is_u32(1), 1);
        assert_eq!(view_is_u32(0), 0);
    }

    #[test]
    fn views_nat_matches_the_package() {
        let _t = TURN.lock().unwrap_or_else(|e| e.into_inner());
        let n = view_nat(1);
        assert_eq!(views().u[3], nat_view_nodes(1));
        assert_eq!(views().u[5], nat_view_pairs(1));
        assert_eq!(views().f[4].len(), 2 * n as usize);
        let j = &views().f[6];
        assert_eq!((j[0], j[1]), (3.0, 7.0));
        let s: f32 = j[2..].iter().sum();
        assert!((s - 1.0).abs() < 1e-4);
    }
}
