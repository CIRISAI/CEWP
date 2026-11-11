//! Thin adapter from the `topology` package's access label (`crate::netgraph::NetLabel`, branch
//! `twin/topology`) to this model's [`AccessNet`].
//!
//! The label carries the access AS (`asn`), its role, metro and page region, but no access
//! technology and no NAT attribute (topology leaves NAT to this package on purpose). So the
//! adapter keys every per-network draw on the ASN: one ASN is one network, the same behaviour for
//! every household and phone on it, which is the correlation [`super::World`] models.
//!
//! - `AccessFixed` -> Cable | Fibre | Dsl, drawn per ASN by the fixed-technology mix (MODEL);
//! - `AccessMobile` -> Cell4g | Cell5g, drawn per ASN by `cell_5g_share` (MODEL);
//! - `Content` -> [`Resolved::Hosted`]: a data-centre host on a public address, outside this
//!   model (micro already dials hosted servers and seats on their listener);
//! - `Tier1` / `RegionalTransit` never carry a node; asn 0 is topology's "unknown": both
//!   [`Resolved::Unknown`], and the caller keeps micro's own rule for that node.
//!
//! The region class (IPv4 rich / scarce) is drawn once per page region (MODEL), so every network
//! of one region shares it, as in [`super::World::generate`].
//!
//! The types below MIRROR topology's until its branch lands (its instruction: "until my branch
//! lands, mirror NetLabel locally"). On merge, replace the mirror block with
//! `pub use crate::netgraph::{AsRole, Asn, MetroId, NetLabel, NodeNet};` and nothing else changes.

use super::{mix, AccessNet, AccessTech, Endpoint, ReachModel};
use crate::micro::Rng;

// ---------------------------------------------------------------- mirror of crate::netgraph

/// Autonomous-system number (topology's stand-ins are RFC 6996 private-use, never real).
pub type Asn = u32;
/// Index into topology's metro list (the page's `POP_GRID` order).
pub type MetroId = u16;

/// An AS's role (mirror of `netgraph::AsRole`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum AsRole {
    /// Tier-1 transit (carries no node).
    Tier1 = 0,
    /// Regional transit (carries no node).
    RegionalTransit = 1,
    /// Fixed broadband access.
    AccessFixed = 2,
    /// Mobile access (a carrier).
    AccessMobile = 3,
    /// Cloud / hosting.
    Content = 4,
}

/// A node's network label (mirror of `netgraph::NetLabel`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetLabel {
    /// Its AS (0 = unknown).
    pub asn: Asn,
    /// The AS's role.
    pub role: AsRole,
    /// Metro.
    pub metro: MetroId,
    /// Page region (index into topology's nine regions).
    pub region: u8,
}

/// A node's networks (mirror of `netgraph::NodeNet`): its household's fixed AS and, for a phone,
/// its carrier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeNet {
    /// Fixed (household) or hosting AS.
    pub home: NetLabel,
    /// Carrier, phones only.
    pub cellular: Option<NetLabel>,
}

impl NodeNet {
    /// The label in use: the household's when `unmetered` (on Wi-Fi), else the carrier's.
    #[must_use]
    pub fn current(&self, unmetered: bool) -> NetLabel {
        if unmetered { self.home } else { self.cellular.unwrap_or(self.home) }
    }
}

// ---------------------------------------------------------------- adapter

/// What a label resolves to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolved {
    /// An access network this model draws behaviour for.
    Access(AccessNet),
    /// A hosted node on a public address (outside this model).
    Hosted,
    /// No usable label: keep micro's rule.
    Unknown,
}

impl ReachModel {
    /// Resolve a topology label to an access network. Pure in (`seed`, label): technology per ASN,
    /// region class per page region.
    #[must_use]
    pub fn resolve(&self, seed: u64, l: &NetLabel) -> Resolved {
        if l.asn == 0 {
            return Resolved::Unknown;
        }
        let cellular = match l.role {
            AsRole::AccessFixed => false,
            AsRole::AccessMobile => true,
            AsRole::Content => return Resolved::Hosted,
            AsRole::Tier1 | AsRole::RegionalTransit => return Resolved::Unknown,
        };
        let tech = self.draw_tech(cellular, &mut Rng::new(mix(seed, 0x5445_4348, u64::from(l.asn))));
        let region = self.draw_region(&mut Rng::new(mix(seed, 0x5245_4749, u64::from(l.region))));
        Resolved::Access(AccessNet { id: l.asn, tech, region })
    }

    /// The endpoint a micro node presents. `household` seeds the home router draw (one per
    /// household); `phone` marks a client-only phone; `unmetered` is the phone on its home Wi-Fi.
    /// `None` for a hosted or unknown label.
    #[must_use]
    pub fn endpoint_of(&self, seed: u64, nn: &NodeNet, household: u64, phone: bool, unmetered: bool) -> Option<Endpoint> {
        let home = |label: &NetLabel| match self.resolve(seed, label) {
            Resolved::Access(net) if !net.tech.is_cellular() => Some(self.home(seed, &self.network(seed, net), household)),
            _ => None,
        };
        if !phone {
            return home(&nn.home);
        }
        if unmetered || nn.cellular.is_none() {
            return home(&nn.home).map(|h| Self::phone_at_home(&h));
        }
        match self.resolve(seed, &nn.current(false)) {
            Resolved::Access(net) if net.tech.is_cellular() => Some(self.phone_on_carrier(&self.network(seed, net))),
            _ => None,
        }
    }
}

/// The technology a fixed or mobile label resolves to, for reports.
#[must_use]
pub fn tech_of(r: Resolved) -> Option<AccessTech> {
    match r {
        Resolved::Access(n) => Some(n.tech),
        Resolved::Hosted | Resolved::Unknown => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn label(asn: Asn, role: AsRole, region: u8) -> NetLabel {
        NetLabel { asn, role, metro: 3, region }
    }

    #[test]
    fn roles_resolve_to_the_right_kind_of_network() {
        let m = ReachModel::default_model();
        let base = 4_200_000_000;
        for i in 0..64 {
            assert!(!tech_of(m.resolve(1, &label(base + i, AsRole::AccessFixed, 2))).unwrap().is_cellular());
            assert!(tech_of(m.resolve(1, &label(base + i, AsRole::AccessMobile, 2))).unwrap().is_cellular());
        }
        assert_eq!(m.resolve(1, &label(base, AsRole::Content, 2)), Resolved::Hosted);
        assert_eq!(m.resolve(1, &label(0, AsRole::AccessFixed, 2)), Resolved::Unknown);
        assert_eq!(m.resolve(1, &label(base, AsRole::Tier1, 2)), Resolved::Unknown);
    }

    #[test]
    fn one_asn_is_one_network_whatever_the_metro() {
        let m = ReachModel::default_model();
        let a = label(4_200_000_123, AsRole::AccessFixed, 5);
        let b = NetLabel { metro: 99, ..a };
        assert_eq!(m.resolve(7, &a), m.resolve(7, &b));
        let Resolved::Access(net) = m.resolve(7, &a) else { panic!("fixed label must resolve") };
        // every household on it sees one ISP behaviour (its CGN and v6 deployment)
        let nb = m.network(7, net);
        let nn = NodeNet { home: a, cellular: None };
        for hh in 0..32 {
            let ep = m.endpoint_of(7, &nn, hh, false, true).unwrap();
            assert_eq!(ep.net, net);
            assert_eq!(ep.cgnat, nb.cgn.is_some());
        }
    }

    #[test]
    fn a_phone_takes_its_carrier_off_wifi_and_its_router_on_it() {
        let m = ReachModel::default_model();
        let nn = NodeNet {
            home: label(4_200_000_010, AsRole::AccessFixed, 1),
            cellular: Some(label(4_200_000_900, AsRole::AccessMobile, 1)),
        };
        let out = m.endpoint_of(3, &nn, 11, true, false).unwrap();
        assert!(out.net.tech.is_cellular() && out.client_only);
        let home = m.endpoint_of(3, &nn, 11, false, true).unwrap();
        let wifi = m.endpoint_of(3, &nn, 11, true, true).unwrap();
        assert_eq!(wifi, ReachModel::phone_at_home(&home));
        // a hosted node is outside the model
        let hosted = NodeNet { home: label(4_200_000_500, AsRole::Content, 1), cellular: None };
        assert_eq!(m.endpoint_of(3, &hosted, 0, false, true), None);
    }
}
