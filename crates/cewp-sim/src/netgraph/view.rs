//! Flat-buffer view of a [`NetGraph`] for the globe (`HOOKS.md` §2; `twin-globe` wraps these
//! in `kernel-wasm`, this crate never touches it).
//!
//! The scenario is the MODEL stand-in [`generate`]`(seed, default parameters)`: NOT real
//! Internet data. Its metros are the page's 181 population-grid localities (real places at
//! city grain, DERIVED); every AS, IXP, relationship and cable is invented. No coordinate here
//! is finer than a city, and none is an address or a production identifier.
//!
//! # Points: `Vec<f32>`, stride [`POINT_STRIDE`] = 5
//!
//! `[lat_deg, lon_deg, role, size, region]` per point, in this order of rows:
//!
//! 1. one row per metro (`metros.len()` rows; point index = metro id),
//! 2. one row per AS (graph order; point index = `metros.len() + as index`),
//! 3. one row per IXP (point index = `metros.len() + ases.len() + ixp id`).
//!
//! `role` ([`ViewRole`]): 0 fixed access AS, 1 IXP, 2 regional transit AS, 3 content / data
//! centre AS (where seats, hosted servers and CI runners sit), 4 metro (backbone node, where
//! cables land), 5 mobile access AS, 6 tier-1 AS.
//!
//! `size`: an AS is `log10` of its customer cone (itself plus every AS reachable by
//! provider-to-customer links; a stub is 0); an IXP is `log10` of its member count; a metro is
//! `log10` of its population weight (the grid's own units). `size` is display-only and uses
//! the platform's `log10`, so native and wasm may differ in its last bit; positions do not.
//!
//! `region`: index into [`super::REGION_NAMES`].
//!
//! Position: a metro sits at its locality. ASes and IXPs sit at their home metro, spread on a
//! sunflower spiral (`DISPLAY_RING_DEG` × √k degrees, golden-angle turns, IXPs first) so they
//! do not draw on top of each other. That offset is display only (MODEL, no meaning): the
//! model places an AS at a metro, not at a point.
//!
//! # Links: `Vec<u32>`, stride [`LINK_STRIDE`] = 3
//!
//! `[a, b, kind]`, `a` and `b` point indices. `kind` ([`ViewLink`]): 0 provider -> customer
//! (`a` provider), 1 settlement-free peering (`a` lower ASN), 2 IXP membership (`a` the AS,
//! `b` the IXP), 3 terrestrial long-haul (metro to metro), 4 submarine long-haul (metro to
//! metro). Order: AS links, then memberships, then cables.
//!
//! # Path: `Vec<f32>`
//!
//! `[as_hops, long_haul_km, lat0, lon0, lat1, lon1, ...]`: the first two values are the path's
//! [`PathCounters`] (`as_hops`, `long_haul_km`), then one waypoint per stride-2 pair: the
//! source AS's point, every metro the route visits in order (centres), the destination AS's
//! point. Both ends must be AS points; each is routed from its home metro. Empty when an end is
//! not an AS point or no valley-free path exists.
//!
//! # Classes
//!
//! [`view_classes`] gives one FSD §8.2 class per field (0 MEASURED, 1 DERIVED, 2 MODEL,
//! 3 PROJECTED, 4 FRONTIER) for points, links, path header and path waypoint, in that order.
//! A field's class is its weakest row's: metro rows are DERIVED, but the AS and IXP rows that
//! share the field are MODEL, so the field is MODEL.

use super::{generate, sin_d, AsRole, CableKind, NetGraph, NetParams, PathCounters, Rel};
use std::collections::BTreeMap;

/// f32 values per point row.
pub const POINT_STRIDE: usize = 5;
/// u32 values per link row.
pub const LINK_STRIDE: usize = 3;
/// Spiral step of the display offset, degrees (MODEL, display only).
pub const DISPLAY_RING_DEG: f64 = 0.12;

/// Point roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ViewRole {
    /// Fixed access AS.
    AccessFixed = 0,
    /// IXP.
    Ixp = 1,
    /// Regional transit AS.
    Transit = 2,
    /// Content / data-centre AS.
    Content = 3,
    /// Metro (backbone node).
    Metro = 4,
    /// Mobile access AS.
    AccessMobile = 5,
    /// Tier-1 AS.
    Tier1 = 6,
}

/// Link kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ViewLink {
    /// Provider to customer.
    ProviderCustomer = 0,
    /// Peering.
    Peer = 1,
    /// IXP membership.
    IxpMember = 2,
    /// Terrestrial cable.
    Terrestrial = 3,
    /// Submarine cable.
    Submarine = 4,
}

const fn view_role(r: AsRole) -> ViewRole {
    match r {
        AsRole::Tier1 => ViewRole::Tier1,
        AsRole::RegionalTransit => ViewRole::Transit,
        AsRole::AccessFixed => ViewRole::AccessFixed,
        AsRole::AccessMobile => ViewRole::AccessMobile,
        AsRole::Content => ViewRole::Content,
    }
}

fn log10(x: u64) -> f32 {
    // display-only magnitude; x >= 1
    (x.max(1) as f64).log10() as f32
}

/// One graph's view, built once; the free functions below build one per call.
#[derive(Debug, Clone)]
pub struct TopologyView {
    /// The graph drawn.
    pub graph: NetGraph,
    /// Point rows ([`POINT_STRIDE`]).
    pub points: Vec<f32>,
    /// Link rows ([`LINK_STRIDE`]).
    pub links: Vec<u32>,
}

impl TopologyView {
    /// View of the stand-in for `seed` with default parameters.
    #[must_use]
    pub fn new(seed: u32) -> Self {
        Self::of(generate(u64::from(seed), &NetParams::default()))
    }

    /// View of any graph (stand-in or loaded).
    #[must_use]
    pub fn of(graph: NetGraph) -> Self {
        let g = &graph;
        let (nm, na) = (g.metros.len(), g.ases.len());
        let as_idx: BTreeMap<u32, usize> = g.ases.iter().enumerate().map(|(i, a)| (a.asn, i)).collect();
        // customer cones: provider -> customers, then a walk per AS
        let mut down: Vec<Vec<usize>> = vec![Vec::new(); na];
        for l in &g.links {
            if l.rel == Rel::ProviderCustomer {
                if let (Some(&p), Some(&c)) = (as_idx.get(&l.a), as_idx.get(&l.b)) {
                    down[p].push(c);
                }
            }
        }
        let mut mark = vec![usize::MAX; na];
        let cone: Vec<u64> = (0..na)
            .map(|s| {
                let mut stack = vec![s];
                mark[s] = s;
                let mut n = 0u64;
                while let Some(u) = stack.pop() {
                    n += 1;
                    for &v in &down[u] {
                        if mark[v] != s {
                            mark[v] = s;
                            stack.push(v);
                        }
                    }
                }
                n
            })
            .collect();
        let mut slot = vec![0u32; nm];
        let mut spot = |m: usize| -> (f32, f32) {
            slot[m] += 1;
            let k = f64::from(slot[m]);
            let golden = std::f64::consts::PI * (3.0 - 5f64.sqrt());
            let (lat, lon) = (f64::from(g.metros[m].lat_cdeg) / 100.0, f64::from(g.metros[m].lon_cdeg) / 100.0);
            let r = DISPLAY_RING_DEG * k.sqrt();
            let t = golden * k;
            let half_pi = std::f64::consts::FRAC_PI_2;
            let coslat = sin_d(lat.to_radians() + half_pi).abs().max(0.2);
            let dlat = r * sin_d(t);
            let dlon = r * sin_d(t + half_pi) / coslat;
            let mut lo = lon + dlon;
            if lo > 180.0 {
                lo -= 360.0;
            } else if lo < -180.0 {
                lo += 360.0;
            }
            ((lat + dlat).clamp(-90.0, 90.0) as f32, lo as f32)
        };
        let mut points = Vec::with_capacity((nm + na + g.ixps.len()) * POINT_STRIDE);
        for m in &g.metros {
            points.extend_from_slice(&[
                m.lat_cdeg as f32 / 100.0,
                m.lon_cdeg as f32 / 100.0,
                ViewRole::Metro as u8 as f32,
                log10(u64::from(m.weight)),
                f32::from(m.region),
            ]);
        }
        // IXPs take the inner spiral slots, so place them first and append after the ASes
        let ixp_pos: Vec<(f32, f32)> = g.ixps.iter().map(|x| spot(x.metro as usize)).collect();
        for (i, a) in g.ases.iter().enumerate() {
            let (la, lo) = spot(a.home_metro as usize);
            points.extend_from_slice(&[la, lo, view_role(a.role) as u8 as f32, log10(cone[i]), f32::from(a.region)]);
        }
        for (x, &(la, lo)) in g.ixps.iter().zip(&ixp_pos) {
            let region = g.metros[x.metro as usize].region;
            points.extend_from_slice(&[la, lo, ViewRole::Ixp as u8 as f32, log10(x.members.len() as u64), f32::from(region)]);
        }
        let as_pt = |asn: u32| as_idx.get(&asn).map(|&i| (nm + i) as u32);
        let mut links = Vec::new();
        for l in &g.links {
            if let (Some(a), Some(b)) = (as_pt(l.a), as_pt(l.b)) {
                let kind = match l.rel {
                    Rel::ProviderCustomer => ViewLink::ProviderCustomer,
                    Rel::Peer => ViewLink::Peer,
                };
                links.extend_from_slice(&[a, b, kind as u32]);
            }
        }
        for (xi, x) in g.ixps.iter().enumerate() {
            for &asn in &x.members {
                if let Some(a) = as_pt(asn) {
                    links.extend_from_slice(&[a, (nm + na + xi) as u32, ViewLink::IxpMember as u32]);
                }
            }
        }
        for c in &g.cables {
            let kind = match c.kind {
                CableKind::Terrestrial => ViewLink::Terrestrial,
                CableKind::Submarine => ViewLink::Submarine,
            };
            links.extend_from_slice(&[u32::from(c.a), u32::from(c.b), kind as u32]);
        }
        Self { graph, points, links }
    }

    /// Number of point rows.
    #[must_use]
    pub fn point_count(&self) -> usize {
        self.points.len() / POINT_STRIDE
    }

    /// The route between two AS points (layout in the module docs; empty if none).
    #[must_use]
    pub fn path(&self, src_point: u32, dst_point: u32) -> Vec<f32> {
        let g = &self.graph;
        let nm = g.metros.len();
        let as_of = |p: u32| {
            let p = p as usize;
            (p >= nm && p < nm + g.ases.len()).then(|| &g.ases[p - nm])
        };
        let (Some(a), Some(b)) = (as_of(src_point), as_of(dst_point)) else {
            return Vec::new();
        };
        let (Some(la), Some(lb)) = (g.label(a.asn, a.home_metro), g.label(b.asn, b.home_metro)) else {
            return Vec::new();
        };
        let Some(p) = g.path(la, lb) else {
            return Vec::new();
        };
        let PathCounters { as_hops, long_haul_km, .. } = p.counters;
        let at = |pt: u32| {
            let o = pt as usize * POINT_STRIDE;
            [self.points[o], self.points[o + 1]]
        };
        let mut out = vec![as_hops as f32, long_haul_km as f32];
        out.extend_from_slice(&at(src_point));
        for &m in &p.metro_trail {
            out.extend_from_slice(&at(u32::from(m)));
        }
        out.extend_from_slice(&at(dst_point));
        out
    }
}

/// Point rows for the stand-in `seed` (layout in the module docs).
#[must_use]
pub fn topology_view_points(seed: u32) -> Vec<f32> {
    TopologyView::new(seed).points
}

/// Link rows for the stand-in `seed` (layout in the module docs).
#[must_use]
pub fn topology_view_links(seed: u32) -> Vec<u32> {
    TopologyView::new(seed).links
}

/// One route for the stand-in `seed` between two AS points (layout in the module docs).
#[must_use]
pub fn topology_view_path(seed: u32, src_point: u32, dst_point: u32) -> Vec<f32> {
    TopologyView::new(seed).path(src_point, dst_point)
}

/// Class per field: 5 point fields, 3 link fields, 2 path-header fields, 2 waypoint fields.
#[must_use]
pub fn view_classes() -> Vec<u8> {
    // points: lat, lon, role, size, region; links: a, b, kind; path: as_hops, km; lat, lon
    vec![2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::netgraph::REGION_NAMES;

    #[test]
    fn view_buffers_are_well_formed_and_deterministic() {
        let v = TopologyView::new(7);
        let g = &v.graph;
        let n = v.point_count();
        assert_eq!(v.points.len() % POINT_STRIDE, 0);
        assert_eq!(n, g.metros.len() + g.ases.len() + g.ixps.len());
        assert_eq!(v.links.len() % LINK_STRIDE, 0);
        for r in v.points.chunks(POINT_STRIDE) {
            assert!((-90.0..=90.0).contains(&r[0]) && (-180.0..=180.0).contains(&r[1]), "{r:?}");
            assert!(r[2] >= 0.0 && r[2] <= 6.0 && r[3] >= 0.0 && (r[4] as usize) < REGION_NAMES.len());
        }
        for r in v.links.chunks(LINK_STRIDE) {
            assert!((r[0] as usize) < n && (r[1] as usize) < n && r[2] <= 4 && r[0] != r[1], "{r:?}");
        }
        // every metro row sits exactly on its locality
        for (i, m) in g.metros.iter().enumerate() {
            assert_eq!(v.points[i * POINT_STRIDE], m.lat_cdeg as f32 / 100.0);
        }
        let again = TopologyView::new(7);
        assert_eq!((&v.points, &v.links), (&again.points, &again.links));
        assert_eq!(view_classes().len(), POINT_STRIDE + LINK_STRIDE + 2 + 2);
        let kinds: Vec<usize> = (0..5).map(|k| v.links.chunks(LINK_STRIDE).filter(|r| r[2] == k).count()).collect();
        println!("view: points {n} (metros {} ases {} ixps {}) | links {} by kind {kinds:?}", g.metros.len(), g.ases.len(), g.ixps.len(), v.links.len() / LINK_STRIDE);
    }

    #[test]
    fn view_path_starts_and_ends_on_its_as_points() {
        let v = TopologyView::new(7);
        let g = &v.graph;
        let nm = g.metros.len() as u32;
        let first_fixed = g.ases.iter().position(|a| a.role == AsRole::AccessFixed).unwrap() as u32 + nm;
        let last_fixed = g.ases.iter().rposition(|a| a.role == AsRole::AccessFixed).unwrap() as u32 + nm;
        let p = v.path(first_fixed, last_fixed);
        assert!(p.len() >= 6 && p.len() % 2 == 0, "{}", p.len());
        let pt = |i: u32| &v.points[i as usize * POINT_STRIDE..i as usize * POINT_STRIDE + 2];
        assert_eq!(&p[2..4], pt(first_fixed));
        assert_eq!(&p[p.len() - 2..], pt(last_fixed));
        // the header matches the routed path's counters
        let (a, b) = (&g.ases[(first_fixed - nm) as usize], &g.ases[(last_fixed - nm) as usize]);
        let r = g.path(g.label(a.asn, a.home_metro).unwrap(), g.label(b.asn, b.home_metro).unwrap()).unwrap();
        assert_eq!((p[0], p[1]), (r.counters.as_hops as f32, r.counters.long_haul_km as f32));
        println!("view path: {} AS hops, {} km, {} waypoints", p[0], p[1], (p.len() - 2) / 2);
        // a metro or IXP end gives nothing
        assert!(v.path(0, last_fixed).is_empty());
        assert!(!g.ixps.is_empty() && v.path(first_fixed, v.point_count() as u32 - 1).is_empty());
    }
}
