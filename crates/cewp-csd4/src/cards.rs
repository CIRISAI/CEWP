//! CSD/4 Specification, Card Provenance, and Cornerstone Safety Invariants.
//!
//! # CSD/4 Definition
//! **CSD/4** stands for **Card-first Specification Design (4-field card format: `shows` / `reads` / `emits` / `checks`)**.
//! It grounds UX and protocol interaction designs directly in executable constitutional semantics,
//! bounded by the **Cornerstone Invariants `X1–X5`** as the cross-plane formal safety properties.
//!
//! # Card Counts Provenance (101 vs 105)
//! The epistemic UI surface comprises **105 total CSD cards**:
//! - **`CIRISClient`**: **101 CSD cards** across 22 specification files:
//!   - **43 CSD/4 four-field cards** (structured with `shows`, `reads`, `emits`, and `checks` fields).
//!   - **58 earlier bullet cards** (legacy specification format).
//! - **`CEWPOS`**: **4 cards** (`CSD-CEWPOS-001` through `CSD-CEWPOS-004`), providing node operating system
//!   shell primitives for local device and runtime management.
//! - **Total**: `101 + 4 = 105` cards.
//!
//! Note that `CIRISConstitution` rc17 owns no CSD cards directly (`csd4_chain.json` `_meta.not_here`
//! explicitly notes that CSD cards are `CIRISClient`/`CEWPOS` data). The CSD namespace mapping is CEWP's
//! synthesis across client cards and constitutional manifests.

/// Total CSD cards in CIRISClient across 22 files (101).
pub const CIRIS_CLIENT_CARD_COUNT: usize = 101;

/// CSD/4 four-field cards in CIRISClient (43).
pub const CIRIS_CLIENT_CSD4_FOUR_FIELD_CARDS: usize = 43;

/// Earlier bullet cards in CIRISClient (58).
pub const CIRIS_CLIENT_EARLIER_BULLET_CARDS: usize = 58;

/// Node operating system cards in CEWPOS (4).
pub const CEWPOS_CARD_COUNT: usize = 4;

/// Total unified CSD cards modeled in CEWP (105).
pub const TOTAL_CSD_CARDS: usize = 105;

/// Cornerstone Invariant definitions from `formal/cewp_node/README.md:54-58`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum CornerstoneInvariant {
    /// X1: Halt is not a trust leg (accord_halt freezes agent operations without creating a trust edge; CC 3.2 T8(v), 4.2.1.1).
    X1_HaltNotTrustLeg,
    /// X2: Evicted roster key's authority ends (revoked/evicted key cannot exercise authority after eviction; CC 3.3.6.3).
    X2_EvictedKeyAuthorityEnds,
    /// X3: Replication needs peer's consent (no replication without bilateral consent / custody agreement; CC 3.3.7, 3.3.1).
    X3_ReplicationNeedsConsent,
    /// X4: Local-tier row never leaves node (local_only / local-tier rows never cross a federation link; CC 5.3.2.4, 2.6.1.7).
    X4_LocalTierNeverLeavesNode,
    /// X5: Leaving a root is one act (unilateral exit from a root requires no counter-signature; subscription falls and latch lifts together; CC 4.2.1).
    X5_LeavingRootIsOneAct,
}

impl CornerstoneInvariant {
    pub const ALL: [CornerstoneInvariant; 5] = [
        CornerstoneInvariant::X1_HaltNotTrustLeg,
        CornerstoneInvariant::X2_EvictedKeyAuthorityEnds,
        CornerstoneInvariant::X3_ReplicationNeedsConsent,
        CornerstoneInvariant::X4_LocalTierNeverLeavesNode,
        CornerstoneInvariant::X5_LeavingRootIsOneAct,
    ];

    pub fn id(&self) -> &'static str {
        match self {
            Self::X1_HaltNotTrustLeg => "X1",
            Self::X2_EvictedKeyAuthorityEnds => "X2",
            Self::X3_ReplicationNeedsConsent => "X3",
            Self::X4_LocalTierNeverLeavesNode => "X4",
            Self::X5_LeavingRootIsOneAct => "X5",
        }
    }

    pub fn title(&self) -> &'static str {
        match self {
            Self::X1_HaltNotTrustLeg => "Halt is not a trust leg",
            Self::X2_EvictedKeyAuthorityEnds => "Evicted roster key's authority ends",
            Self::X3_ReplicationNeedsConsent => "Replication needs peer's consent",
            Self::X4_LocalTierNeverLeavesNode => "Local-tier row never leaves node",
            Self::X5_LeavingRootIsOneAct => "Leaving a root is one act",
        }
    }

    pub fn cc_citation(&self) -> &'static str {
        match self {
            Self::X1_HaltNotTrustLeg => "3.2 T8 (v), 4.2.1.1",
            Self::X2_EvictedKeyAuthorityEnds => "3.3.6.3",
            Self::X3_ReplicationNeedsConsent => "3.3.7, 3.3.1",
            Self::X4_LocalTierNeverLeavesNode => "5.3.2.4, 2.6.1.7",
            Self::X5_LeavingRootIsOneAct => "4.2.1",
        }
    }

    pub fn property_description(&self) -> &'static str {
        match self {
            Self::X1_HaltNotTrustLeg => {
                "a constitutional halt is not a trust leg: the latch moves no standing, \
                 stops no serving and no replication, and moves exactly the agent's act gate; \
                 a latched node can still serve and replicate (ENABLED)"
            }
            Self::X2_EvictedKeyAuthorityEnds => {
                "the authority of a roster key ends with its eviction; any live roster key acts as the self"
            }
            Self::X3_ReplicationNeedsConsent => {
                "replication of a peer's content needs that peer's consent; a revocation is a boundary"
            }
            Self::X4_LocalTierNeverLeavesNode => {
                "a local-tier row never leaves its node; a served row is a signed federation-tier row"
            }
            Self::X5_LeavingRootIsOneAct => {
                "leaving the root is one act: the subscription falls and the latch lifts together, the record stands"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_card_counts() {
        assert_eq!(CIRIS_CLIENT_CSD4_FOUR_FIELD_CARDS + CIRIS_CLIENT_EARLIER_BULLET_CARDS, CIRIS_CLIENT_CARD_COUNT);
        assert_eq!(CIRIS_CLIENT_CARD_COUNT, 101);
        assert_eq!(CEWPOS_CARD_COUNT, 4);
        assert_eq!(CIRIS_CLIENT_CARD_COUNT + CEWPOS_CARD_COUNT, TOTAL_CSD_CARDS);
        assert_eq!(TOTAL_CSD_CARDS, 105);
    }

    #[test]
    fn test_cornerstone_invariants_count_and_details() {
        assert_eq!(CornerstoneInvariant::ALL.len(), 5);
        for inv in CornerstoneInvariant::ALL {
            assert!(!inv.id().is_empty());
            assert!(!inv.title().is_empty());
            assert!(!inv.cc_citation().is_empty());
            assert!(!inv.property_description().is_empty());
        }
    }
}
