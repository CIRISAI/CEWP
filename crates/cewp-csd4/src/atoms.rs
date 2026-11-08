//! Canonical CSD/4 Vocabulary Atoms, State Table Mapping, and Durability Definitions.
//!
//! Synchronized against CIRISConstitution rc17 manifests:
//! - `manifests/topology_vocabulary.json` (Catalogue version 6, 156 atoms)
//! - `formal/STATE_TABLE.tsv` (152 atoms in state table, 242 total constitutional states)
//! - `manifests/csd4_chain.json` (110 distinct promised atoms across T0–T19)
//! - 6 replication durability atoms (`file.durable.*`) awaiting T20 (`CIRISServer#809`)


/// Catalogue version pinned by CIRISConstitution rc17.
pub const CATALOGUE_VERSION: u32 = 6;

/// Total vocabulary atoms in CIRISConstitution rc17 catalogue version 6.
pub const TOTAL_VOCABULARY_ATOMS: usize = 156;

/// Total vocabulary atoms mapped directly to predicates in `formal/STATE_TABLE.tsv`.
pub const TOTAL_STATE_TABLE_ATOMS: usize = 152;

/// Total normalized states in the constitutional state table across 81 variables & 12 planes.
pub const TOTAL_CONSTITUTIONAL_STATE_TABLE: usize = 242;

/// Total distinct promised atoms across the 20 canonical topologies T0..T19.
pub const TOTAL_PROMISED_ATOMS: usize = 110;

/// The 6 `file.durable.*` atoms awaiting T20 (`CIRISServer#809`).
/// Corresponds to the target-holder replication durability tier (CC 6.1.5).
pub static AWAITING_T20_DURABLE_ATOMS: [&str; 6] = [
    "file.durable",
    "file.durable.at_target",
    "file.durable.below_floor",
    "file.durable.descending",
    "file.durable.recovered",
    "file.durable.unmarked",
];

/// The 4 vocabulary atoms not mapped to a state-table variable (surface/boundary assertions).
pub static NON_STATE_TABLE_ATOMS: [&str; 4] = [
    "negatives.held",
    "peer.direct",
    "person.outsider",
    "values.ready",
];

/// Complete, sorted list of all 156 canonical vocabulary atoms in catalogue version 6.
pub static VOCABULARY_ATOMS: [&str; 156] = [
    "affiliation.charter.declared",
    "affiliation.charter.superseded",
    "age.assured",
    "agent.bond.lapsed",
    "agent.bond.unbound",
    "agent.bound",
    "agent.deferral.approved",
    "agent.deferral.pending",
    "agent.deferral.rejected",
    "agent.halt_gate.acting",
    "agent.identity.amended",
    "agent.retirement.custodian_minted",
    "agent.retirement.decided",
    "agent.retirement.frozen",
    "agent.retirement.handed_over",
    "agent.retirement.notified",
    "agent.retirement.retired",
    "agent.retirement.secrets_destroyed",
    "approach.declared",
    "authority.licensing",
    "blob.custody.purged",
    "canonical.peered",
    "child.member",
    "community.formed",
    "community.member",
    "consent.community_trust.bounded",
    "consent.community_trust.granted",
    "consent.decay.complete",
    "consent.decay.identity_severed",
    "consent.decay.patterns_anonymized",
    "consent.replication.none",
    "consent.state.expired",
    "consent.state.granted",
    "consent.state.revoked",
    "contact.added",
    "contact.reachable",
    "contact.via_code",
    "delegation.chain",
    "delegation.link.expired",
    "delegation.link.withdrawn",
    "delegation.refused",
    "delegation.scope.grant",
    "delegation.scope.license",
    "device.join_approved",
    "device.join_requested",
    "device.joined",
    "evidence.admitted",
    "evidence.counted",
    "evidence.offered",
    "evidence.reference_unresolved",
    "evidence.standing.blessed",
    "evidence.standing.unblessed",
    "evidence.superseded",
    "file.custody_received",
    "file.durable",
    "file.durable.at_target",
    "file.durable.below_floor",
    "file.durable.descending",
    "file.durable.recovered",
    "file.durable.unmarked",
    "file.held",
    "file.large",
    "file.withdrawn",
    "goal.declared",
    "goal.proposed",
    "goal.recanted",
    "goal.staged",
    "goal.superseded",
    "goal.withdrawn",
    "grant.axis.content",
    "grant.ended_by_rotation",
    "grant.onward_refused",
    "grant.state.held",
    "group.federating.no",
    "group.moderator.fail_secure",
    "group.moderator.lapsed",
    "group.moderator.merit_promoted",
    "guardian.bond.none",
    "guardian.bound",
    "household.formed",
    "household.member",
    "key.escrowed",
    "key.holder.lost",
    "key.holder.owner",
    "key.holder.thief",
    "key.membership.evicted",
    "key.membership.proposed_add",
    "key.membership.proposed_evict",
    "key.revocation.none",
    "key.revocation.revoked_after",
    "key.revocation.revoked_all",
    "key.rotated",
    "key.successor_committed",
    "licence.honoured",
    "licence.issued",
    "licence.not_trusted",
    "licence.status.revoked",
    "licence.status.suspended",
    "member.stage.accepted_awaiting_group",
    "member.stage.declined",
    "member.stage.proposed",
    "member.stage.removed",
    "member.stage.resigned",
    "message.delivered",
    "method.declared",
    "moderator.named",
    "negatives.held",
    "node.announce.announced",
    "node.announced",
    "node.claimed",
    "node.unannounced",
    "node.unclaimed",
    "note.synced",
    "pair.rooted",
    "party.outside_roots",
    "peer.direct",
    "person.multi_device",
    "person.outsider",
    "progress_measure.declared",
    "quorum_change.applied",
    "quorum_change.held",
    "recognition.inbound",
    "recognition.outbound",
    "room.pair.keyed",
    "room.self.keyed",
    "root.halt_confirmed",
    "root.halted",
    "root.head.stale",
    "root.holder_disposition.censored",
    "root.holder_disposition.coerced",
    "root.holder_disposition.honest",
    "root.holder_disposition.lost",
    "root.infrastructure",
    "root.key",
    "root.liveness.live",
    "root.liveness.stalled",
    "root.none",
    "root.resumed",
    "root.rooted",
    "root.standing.Stalled",
    "root.two",
    "roster.visible",
    "row.scope.community",
    "row.scope.family",
    "row.standing.admitted",
    "row.standing.tombstoned",
    "seat.identity_type.human",
    "seat.identity_type.node_bearing",
    "self.roster_size.one",
    "self.roster_size.three_more",
    "session.unified",
    "steward.bonded",
    "trust.entry.licensor",
    "trust.entry.root_removed",
    "values.ready",
    "wa.present",
];

/// Returns true if the atom is in the 156-atom canonical vocabulary.
pub fn is_vocabulary_atom(atom: &str) -> bool {
    VOCABULARY_ATOMS.binary_search(&atom).is_ok()
}

/// Returns true if the atom is mapped to a state-table variable (152 of 156 atoms).
pub fn is_state_table_atom(atom: &str) -> bool {
    is_vocabulary_atom(atom) && !NON_STATE_TABLE_ATOMS.contains(&atom)
}

/// Returns true if the atom is one of the 6 durability atoms awaiting T20.
pub fn is_awaiting_t20_durable_atom(atom: &str) -> bool {
    AWAITING_T20_DURABLE_ATOMS.contains(&atom)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn test_vocabulary_counts() {
        assert_eq!(VOCABULARY_ATOMS.len(), TOTAL_VOCABULARY_ATOMS);
        assert_eq!(TOTAL_VOCABULARY_ATOMS, 156);
        assert_eq!(TOTAL_STATE_TABLE_ATOMS, 152);
        assert_eq!(TOTAL_CONSTITUTIONAL_STATE_TABLE, 242);
        assert_eq!(TOTAL_PROMISED_ATOMS, 110);
        assert_eq!(CATALOGUE_VERSION, 6);

        let set: BTreeSet<&str> = VOCABULARY_ATOMS.iter().copied().collect();
        assert_eq!(set.len(), 156, "No duplicate atoms in VOCABULARY_ATOMS");

        let non_st: BTreeSet<&str> = NON_STATE_TABLE_ATOMS.iter().copied().collect();
        assert_eq!(non_st.len(), 4);
        for a in &NON_STATE_TABLE_ATOMS {
            assert!(set.contains(a), "Non-state-table atom must be in vocabulary: {}", a);
        }

        let st_count = VOCABULARY_ATOMS.iter().filter(|&&a| is_state_table_atom(a)).count();
        assert_eq!(st_count, 152, "State table atoms must equal 152");
    }

    #[test]
    fn test_durable_atoms_exact_names() {
        assert_eq!(AWAITING_T20_DURABLE_ATOMS.len(), 6);
        let expected = [
            "file.durable",
            "file.durable.at_target",
            "file.durable.below_floor",
            "file.durable.descending",
            "file.durable.recovered",
            "file.durable.unmarked",
        ];
        assert_eq!(AWAITING_T20_DURABLE_ATOMS, expected);
        for atom in expected {
            assert!(is_vocabulary_atom(atom), "Durable atom {} must be in vocabulary", atom);
            assert!(is_state_table_atom(atom), "Durable atom {} must be in state table", atom);
            assert!(is_awaiting_t20_durable_atom(atom));
        }
    }
}
