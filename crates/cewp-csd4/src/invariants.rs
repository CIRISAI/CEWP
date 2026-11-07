//! Cross-Plane Spine Invariants (X1–X5) for CSD/4 scenarios.
//!
//! Normative invariants from `CIRISConstitution` `formal/cewp_node/CewpNode.tla` and `rc17`:
//! - `X1` (`Inv_X1_LatchDoesNotBlockServing`): CC 3.2 T8 (v), 4.2.1.1. Constitutional halt is not a trust leg;
//!   the halt latch moves no standing, stops no serving and no replication, and gates only the agent's act gate.
//! - `X2` (`Inv_X2_EvictedKeyAuthorityEnds`): CC 3.3.6.3. Authority of a self-roster key ends with its eviction;
//!   any live roster key acts as the self.
//! - `X3` (`Inv_X3_ReplicationNeedsConsentAndRevocationIsBoundary`): CC 3.3.7, 3.3.1. Replication of a peer's
//!   content requires that peer's consent; a revocation is a forward-only boundary.
//! - `X4` (`Inv_X4_LocalTierNeverLeavesNode`): CC 5.3.2.4, 2.6.1.7. A local-tier row never leaves its node;
//!   a served row is a signed federation-tier row.
//! - `X5` (`Inv_X5_LeaveRootAtomic`): CC 4.2.1. Leaving a root is one atomic act: subscription falls and
//!   latch lifts together while the historical record stands.

use std::collections::BTreeSet;
use std::fmt;

/// Invariant violation error detailing the exact cross-plane rule that failed.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum SpineViolation {
    X1_LatchBlocksServing { step: usize, node: String },
    X1_LatchBlocksReplication { step: usize, node: String },
    X1_LatchMovedStanding { step: usize, node: String, row_id: u64 },
    X1_AgentActNotGatedDuringHalt { step: usize, agent: String },

    X2_EvictedKeyExercisedAuthority { step: usize, key_id: String },
    X2_EvictedKeyInLiveRoster { step: usize, key_id: String },
    X2_LiveKeyDeniedAuthority { step: usize, key_id: String },

    X3_ReplicationLacksConsent { step: usize, peer: String, row_id: u64 },
    X3_ReplicationCrossedRevocationBoundary { step: usize, peer: String, row_id: u64, rev_step: usize },
    X3_HistoricalRowInvalidatedOnRevocation { step: usize, row_id: u64, rev_step: usize },

    X4_LocalTierRowLeaked { step: usize, node: String, row_id: u64 },
    X4_FederationTierRowUnsigned { step: usize, node: String, row_id: u64 },

    X5_LatchNotLiftedWithSubscriptionFall { step: usize, root_id: String },
    X5_SubscriptionNotFallenWithLatchLift { step: usize, root_id: String },
    X5_HistoricalRecordClearedOnLeave { step: usize, root_id: String },
}

impl fmt::Display for SpineViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpineViolation::X1_LatchBlocksServing { step, node } => {
                write!(f, "Inv_X1 violation at step {}: constitutional halt latch blocked serving on node {}", step, node)
            }
            SpineViolation::X1_LatchBlocksReplication { step, node } => {
                write!(f, "Inv_X1 violation at step {}: constitutional halt latch blocked replication on node {}", step, node)
            }
            SpineViolation::X1_LatchMovedStanding { step, node, row_id } => {
                write!(f, "Inv_X1 violation at step {}: constitutional halt latch moved standing of row {} on node {}", step, row_id, node)
            }
            SpineViolation::X1_AgentActNotGatedDuringHalt { step, agent } => {
                write!(f, "Inv_X1 violation at step {}: agent {} acted while constitutional halt latch was engaged", step, agent)
            }

            SpineViolation::X2_EvictedKeyExercisedAuthority { step, key_id } => {
                write!(f, "Inv_X2 violation at step {}: evicted key {} exercised authority after eviction", step, key_id)
            }
            SpineViolation::X2_EvictedKeyInLiveRoster { step, key_id } => {
                write!(f, "Inv_X2 violation at step {}: evicted key {} still present in live roster", step, key_id)
            }
            SpineViolation::X2_LiveKeyDeniedAuthority { step, key_id } => {
                write!(f, "Inv_X2 violation at step {}: live roster key {} denied authority to act as self", step, key_id)
            }

            SpineViolation::X3_ReplicationLacksConsent { step, peer, row_id } => {
                write!(f, "Inv_X3 violation at step {}: replication of peer {} content (row {}) without active consent", step, peer, row_id)
            }
            SpineViolation::X3_ReplicationCrossedRevocationBoundary { step, peer, row_id, rev_step } => {
                write!(f, "Inv_X3 violation at step {}: post-revocation replication of peer {} (row {} created after revocation step {})", step, peer, row_id, rev_step)
            }
            SpineViolation::X3_HistoricalRowInvalidatedOnRevocation { step, row_id, rev_step } => {
                write!(f, "Inv_X3 violation at step {}: historical row {} invalidated purely by forward revocation at step {}", step, row_id, rev_step)
            }

            SpineViolation::X4_LocalTierRowLeaked { step, node, row_id } => {
                write!(f, "Inv_X4 violation at step {}: local tier row {} transmitted over network link from node {}", step, row_id, node)
            }
            SpineViolation::X4_FederationTierRowUnsigned { step, node, row_id } => {
                write!(f, "Inv_X4 violation at step {}: served federation tier row {} is unsigned from node {}", step, row_id, node)
            }

            SpineViolation::X5_LatchNotLiftedWithSubscriptionFall { step, root_id } => {
                write!(f, "Inv_X5 violation at step {}: leaving root {} dropped subscription but latch was not lifted atomically", step, root_id)
            }
            SpineViolation::X5_SubscriptionNotFallenWithLatchLift { step, root_id } => {
                write!(f, "Inv_X5 violation at step {}: leaving root {} lifted latch but subscription did not drop atomically", step, root_id)
            }
            SpineViolation::X5_HistoricalRecordClearedOnLeave { step, root_id } => {
                write!(f, "Inv_X5 violation at step {}: leaving root {} wiped historical attestation rows", step, root_id)
            }
        }
    }
}

impl std::error::Error for SpineViolation {}

// ── X1: Latch Does Not Block Serving ──────────────────────────────────────

/// Node state for evaluating Inv_X1.
#[derive(Debug, Clone)]
pub struct HaltServingState {
    pub step: usize,
    pub node_id: String,
    pub halt_latch: bool,
    pub serving_enabled: bool,
    pub replication_enabled: bool,
    pub standing_changed: bool,
    pub agent_acting: bool,
    pub latch_blocks_serve_breaker: bool, // Breaker test flag
}

/// Inv_X1 checker: A constitutional halt does not block serving or replication,
/// moves no standing, and only gates the agent's act gate.
pub fn check_inv_x1_latch_does_not_block_serving(state: &HaltServingState) -> Result<(), SpineViolation> {
    if state.halt_latch {
        if state.latch_blocks_serve_breaker || !state.serving_enabled {
            return Err(SpineViolation::X1_LatchBlocksServing {
                step: state.step,
                node: state.node_id.clone(),
            });
        }
        if !state.replication_enabled {
            return Err(SpineViolation::X1_LatchBlocksReplication {
                step: state.step,
                node: state.node_id.clone(),
            });
        }
        if state.standing_changed {
            return Err(SpineViolation::X1_LatchMovedStanding {
                step: state.step,
                node: state.node_id.clone(),
                row_id: 1,
            });
        }
        if state.agent_acting {
            return Err(SpineViolation::X1_AgentActNotGatedDuringHalt {
                step: state.step,
                agent: format!("{}:agent", state.node_id),
            });
        }
    }
    Ok(())
}

// ── X2: Evicted Key Authority Ends ────────────────────────────────────────

/// Roster state for evaluating Inv_X2.
#[derive(Debug, Clone)]
pub struct KeyRosterState {
    pub step: usize,
    pub live_keys: BTreeSet<String>,
    pub evicted_keys: BTreeSet<String>,
    pub acting_key: Option<String>,
}

/// Inv_X2 checker: Authority of a self-roster key ends with its eviction;
/// any live roster key acts as the self.
pub fn check_inv_x2_evicted_key_authority_ends(state: &KeyRosterState) -> Result<(), SpineViolation> {
    for evicted in &state.evicted_keys {
        if state.live_keys.contains(evicted) {
            return Err(SpineViolation::X2_EvictedKeyInLiveRoster {
                step: state.step,
                key_id: evicted.clone(),
            });
        }
    }

    if let Some(ref actor) = state.acting_key {
        if state.evicted_keys.contains(actor) {
            return Err(SpineViolation::X2_EvictedKeyExercisedAuthority {
                step: state.step,
                key_id: actor.clone(),
            });
        }
        if !state.live_keys.contains(actor) {
            return Err(SpineViolation::X2_LiveKeyDeniedAuthority {
                step: state.step,
                key_id: actor.clone(),
            });
        }
    }

    Ok(())
}

// ── X3: Replication Needs Consent & Revocation Is Boundary ────────────────

/// Consent and replication state for evaluating Inv_X3.
#[derive(Debug, Clone)]
pub struct ConsentReplicationState {
    pub step: usize,
    pub peer_id: String,
    pub has_active_consent: bool,
    pub revocation_step: Option<usize>,
    pub row_id: u64,
    pub row_created_step: usize,
    pub row_standing_admitted: bool,
    pub is_replicated: bool,
}

/// Inv_X3 checker: Replication of peer content requires consent; revocation is a forward boundary.
pub fn check_inv_x3_replication_needs_consent_and_revocation_is_boundary(
    state: &ConsentReplicationState,
) -> Result<(), SpineViolation> {
    if state.is_replicated {
        if !state.has_active_consent {
            return Err(SpineViolation::X3_ReplicationLacksConsent {
                step: state.step,
                peer: state.peer_id.clone(),
                row_id: state.row_id,
            });
        }

        if let Some(rev_step) = state.revocation_step {
            if state.row_created_step >= rev_step && state.step >= rev_step {
                return Err(SpineViolation::X3_ReplicationCrossedRevocationBoundary {
                    step: state.step,
                    peer: state.peer_id.clone(),
                    row_id: state.row_id,
                    rev_step,
                });
            }
        }
    }

    // Historical invariant: rows created before revocation remain admitted
    if let Some(rev_step) = state.revocation_step {
        if state.row_created_step < rev_step && !state.row_standing_admitted {
            return Err(SpineViolation::X3_HistoricalRowInvalidatedOnRevocation {
                step: state.step,
                row_id: state.row_id,
                rev_step,
            });
        }
    }

    Ok(())
}

// ── X4: Local Tier Never Leaves Node ──────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowTier {
    Local,
    Federation,
}

/// Row transmission state for evaluating Inv_X4.
#[derive(Debug, Clone)]
pub struct RowTransmissionState {
    pub step: usize,
    pub node_id: String,
    pub row_id: u64,
    pub tier: RowTier,
    pub is_transmitted_over_network: bool,
    pub has_valid_signature: bool,
}

/// Inv_X4 checker: A local-tier row never leaves its node; a served row is a signed federation-tier row.
pub fn check_inv_x4_local_tier_never_leaves_node(state: &RowTransmissionState) -> Result<(), SpineViolation> {
    if state.is_transmitted_over_network {
        if state.tier == RowTier::Local {
            return Err(SpineViolation::X4_LocalTierRowLeaked {
                step: state.step,
                node: state.node_id.clone(),
                row_id: state.row_id,
            });
        }
        if state.tier == RowTier::Federation && !state.has_valid_signature {
            return Err(SpineViolation::X4_FederationTierRowUnsigned {
                step: state.step,
                node: state.node_id.clone(),
                row_id: state.row_id,
            });
        }
    }
    Ok(())
}

// ── X5: Leave Root Atomic ─────────────────────────────────────────────────

/// Leave-root transition state for evaluating Inv_X5.
#[derive(Debug, Clone)]
pub struct LeaveRootState {
    pub step: usize,
    pub root_id: String,
    pub prior_subscribed: bool,
    pub post_subscribed: bool,
    pub prior_latch: bool,
    pub post_latch: bool,
    pub historical_rows_retained: bool,
}

/// Inv_X5 checker: Leaving a root is one atomic act: subscription falls and latch lifts together
/// while the historical record stands.
pub fn check_inv_x5_leave_root_atomic(state: &LeaveRootState) -> Result<(), SpineViolation> {
    let leaving = state.prior_subscribed && !state.post_subscribed;
    if leaving {
        if state.post_latch {
            return Err(SpineViolation::X5_LatchNotLiftedWithSubscriptionFall {
                step: state.step,
                root_id: state.root_id.clone(),
            });
        }
        if !state.historical_rows_retained {
            return Err(SpineViolation::X5_HistoricalRecordClearedOnLeave {
                step: state.step,
                root_id: state.root_id.clone(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inv_x1_passes_valid_halt() {
        let valid_halt = HaltServingState {
            step: 5,
            node_id: "N1".to_string(),
            halt_latch: true,
            serving_enabled: true,
            replication_enabled: true,
            standing_changed: false,
            agent_acting: false,
            latch_blocks_serve_breaker: false,
        };
        assert!(check_inv_x1_latch_does_not_block_serving(&valid_halt).is_ok());
    }

    #[test]
    fn test_inv_x1_detects_breaker_failure() {
        let broken_halt = HaltServingState {
            step: 5,
            node_id: "N1".to_string(),
            halt_latch: true,
            serving_enabled: false, // Breaker flipped
            replication_enabled: true,
            standing_changed: false,
            agent_acting: false,
            latch_blocks_serve_breaker: true,
        };
        let err = check_inv_x1_latch_does_not_block_serving(&broken_halt).unwrap_err();
        match err {
            SpineViolation::X1_LatchBlocksServing { .. } => {}
            _ => panic!("Expected X1_LatchBlocksServing error"),
        }
    }

    #[test]
    fn test_inv_x2_evicted_key_authority() {
        let mut live = BTreeSet::new();
        live.insert("k1".to_string());
        live.insert("k2".to_string());

        let mut evicted = BTreeSet::new();
        evicted.insert("k3_thief".to_string());

        // Live key k1 acting -> OK
        let state_ok = KeyRosterState {
            step: 10,
            live_keys: live.clone(),
            evicted_keys: evicted.clone(),
            acting_key: Some("k1".to_string()),
        };
        assert!(check_inv_x2_evicted_key_authority_ends(&state_ok).is_ok());

        // Evicted key acting -> VIOLATION
        let state_evicted_acting = KeyRosterState {
            step: 11,
            live_keys: live.clone(),
            evicted_keys: evicted.clone(),
            acting_key: Some("k3_thief".to_string()),
        };
        let err = check_inv_x2_evicted_key_authority_ends(&state_evicted_acting).unwrap_err();
        match err {
            SpineViolation::X2_EvictedKeyExercisedAuthority { key_id, .. } => {
                assert_eq!(key_id, "k3_thief");
            }
            _ => panic!("Expected X2_EvictedKeyExercisedAuthority"),
        }
    }

    #[test]
    fn test_inv_x3_replication_and_revocation() {
        let valid_historical = ConsentReplicationState {
            step: 20,
            peer_id: "bob".to_string(),
            has_active_consent: false, // Revoked
            revocation_step: Some(15),
            row_id: 101,
            row_created_step: 10, // Created before revocation -> preserved
            row_standing_admitted: true,
            is_replicated: false,
        };
        assert!(check_inv_x3_replication_needs_consent_and_revocation_is_boundary(&valid_historical).is_ok());

        // Replicating new content after revocation -> VIOLATION
        let invalid_post_rev = ConsentReplicationState {
            step: 20,
            peer_id: "bob".to_string(),
            has_active_consent: true, // Inconsistent
            revocation_step: Some(15),
            row_id: 202,
            row_created_step: 18, // Created after revocation
            row_standing_admitted: true,
            is_replicated: true,
        };
        assert!(check_inv_x3_replication_needs_consent_and_revocation_is_boundary(&invalid_post_rev).is_err());
    }

    #[test]
    fn test_inv_x4_local_tier_leak() {
        let leak = RowTransmissionState {
            step: 4,
            node_id: "N1".to_string(),
            row_id: 42,
            tier: RowTier::Local,
            is_transmitted_over_network: true,
            has_valid_signature: false,
        };
        assert!(matches!(
            check_inv_x4_local_tier_never_leaves_node(&leak),
            Err(SpineViolation::X4_LocalTierRowLeaked { .. })
        ));

        let valid_fed = RowTransmissionState {
            step: 4,
            node_id: "N1".to_string(),
            row_id: 43,
            tier: RowTier::Federation,
            is_transmitted_over_network: true,
            has_valid_signature: true,
        };
        assert!(check_inv_x4_local_tier_never_leaves_node(&valid_fed).is_ok());
    }

    #[test]
    fn test_inv_x5_atomic_leave_root() {
        let valid_leave = LeaveRootState {
            step: 12,
            root_id: "HA".to_string(),
            prior_subscribed: true,
            post_subscribed: false,
            prior_latch: true,
            post_latch: false, // Lifted simultaneously
            historical_rows_retained: true,
        };
        assert!(check_inv_x5_leave_root_atomic(&valid_leave).is_ok());

        let unlifted_latch = LeaveRootState {
            step: 12,
            root_id: "HA".to_string(),
            prior_subscribed: true,
            post_subscribed: false,
            prior_latch: true,
            post_latch: true, // Latch failed to lift
            historical_rows_retained: true,
        };
        assert!(matches!(
            check_inv_x5_leave_root_atomic(&unlifted_latch),
            Err(SpineViolation::X5_LatchNotLiftedWithSubscriptionFall { .. })
        ));
    }
}
