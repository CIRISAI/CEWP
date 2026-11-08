//! Explicit Coverage Qualification Reporter.
//!
//! Enforces the normative requirement from `FSD/SIMULATION_ENGINE.md` §2.2 / §2.3:
//! Every simulation coverage report must compute and format:
//! `(atoms_exercised / 110 promised / 156 vocabulary / 242 constitutional_state_table)`
//! and list the 6 `file.durable.*` atoms awaiting `T20` (`CIRISServer#809`).

use std::collections::BTreeSet;
use std::fmt;
use crate::catalogue::{
    TopologyId, AWAITING_T20_DURABLE_ATOMS, TOTAL_CONSTITUTIONAL_STATE_TABLE,
    TOTAL_PROMISED_ATOMS, TOTAL_VOCABULARY_ATOMS,
};

/// Simulation coverage report over CSD/4 scenarios.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageReport {
    /// Distinct state atoms exercised in the scenario run.
    pub exercised_atoms: BTreeSet<String>,
    /// Number of distinct atoms promised across T0–T19 (110).
    pub promised_count: usize,
    /// Total vocabulary atoms defined in CIRISConstitution (156).
    pub vocabulary_count: usize,
    /// Total states in the constitutional state table (242).
    pub state_table_count: usize,
    /// Durability atoms awaiting T20 (CIRISServer#809).
    pub awaiting_t20_atoms: &'static [&'static str],
}

impl Default for CoverageReport {
    fn default() -> Self {
        Self {
            exercised_atoms: BTreeSet::new(),
            promised_count: TOTAL_PROMISED_ATOMS,
            vocabulary_count: TOTAL_VOCABULARY_ATOMS,
            state_table_count: TOTAL_CONSTITUTIONAL_STATE_TABLE,
            awaiting_t20_atoms: &AWAITING_T20_DURABLE_ATOMS,
        }
    }
}

impl CoverageReport {
    /// Creates a coverage report from a list of executed topologies.
    pub fn from_topologies(topos: &[TopologyId]) -> Self {
        let mut exercised = BTreeSet::new();
        for &id in topos {
            let decl = id.get();
            for ts in decl.trace {
                for &atom in ts.atoms {
                    exercised.insert(atom.to_string());
                }
            }
        }
        Self {
            exercised_atoms: exercised,
            promised_count: TOTAL_PROMISED_ATOMS,
            vocabulary_count: TOTAL_VOCABULARY_ATOMS,
            state_table_count: TOTAL_CONSTITUTIONAL_STATE_TABLE,
            awaiting_t20_atoms: &AWAITING_T20_DURABLE_ATOMS,
        }
    }

    /// Creates a coverage report directly from an exercised atom set.
    pub fn from_atoms(atoms: BTreeSet<String>) -> Self {
        Self {
            exercised_atoms: atoms,
            promised_count: TOTAL_PROMISED_ATOMS,
            vocabulary_count: TOTAL_VOCABULARY_ATOMS,
            state_table_count: TOTAL_CONSTITUTIONAL_STATE_TABLE,
            awaiting_t20_atoms: &AWAITING_T20_DURABLE_ATOMS,
        }
    }

    /// Returns the exact required qualification string:
    /// `(atoms_exercised / 110 promised / 156 vocabulary / 242 constitutional_state_table)`
    pub fn format_qualification(&self) -> String {
        format!(
            "({} / {} promised / {} vocabulary / {} constitutional_state_table)",
            self.exercised_atoms.len(),
            self.promised_count,
            self.vocabulary_count,
            self.state_table_count
        )
    }

    /// Returns the full formatted coverage summary report including awaiting T20 atoms.
    pub fn format_full_report(&self) -> String {
        let mut out = String::new();
        out.push_str("=== CSD/4 Simulation Coverage Qualification Report ===\n");
        out.push_str(&format!("Qualification: {}\n", self.format_qualification()));
        out.push_str(&format!("Exercised Atoms Count: {}\n", self.exercised_atoms.len()));
        out.push_str(&format!("Promised in CSD/4 (T0–T19): {}\n", self.promised_count));
        out.push_str(&format!("Vocabulary in CIRISConstitution rc17: {}\n", self.vocabulary_count));
        out.push_str(&format!("States in Constitutional State Table: {}\n", self.state_table_count));
        out.push_str("\nAtoms Awaiting T20 (CIRISServer#809):\n");
        for atom in self.awaiting_t20_atoms {
            out.push_str(&format!("  - {}\n", atom));
        }
        out
    }
}

impl fmt::Display for CoverageReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format_qualification())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coverage_qualification_formatting() {
        let report = CoverageReport::from_topologies(&[TopologyId::T5, TopologyId::T14, TopologyId::T15]);
        let qual = report.format_qualification();
        assert!(qual.contains("/ 110 promised / 156 vocabulary / 242 constitutional_state_table)"));
        assert_eq!(report.promised_count, 110);
        assert_eq!(report.vocabulary_count, 156);
        assert_eq!(report.state_table_count, 242);

        // Verify awaiting T20 durability atoms
        assert_eq!(report.awaiting_t20_atoms.len(), 6);
        assert!(report.awaiting_t20_atoms.contains(&"file.durable"));
        assert!(report.awaiting_t20_atoms.contains(&"file.durable.at_target"));
        assert!(report.awaiting_t20_atoms.contains(&"file.durable.below_floor"));
        assert!(report.awaiting_t20_atoms.contains(&"file.durable.descending"));
        assert!(report.awaiting_t20_atoms.contains(&"file.durable.recovered"));
        assert!(report.awaiting_t20_atoms.contains(&"file.durable.unmarked"));
    }

    #[test]
    fn test_full_report_text() {
        let mut atoms = BTreeSet::new();
        atoms.insert("node.unclaimed".to_string());
        atoms.insert("node.claimed".to_string());
        let report = CoverageReport::from_atoms(atoms);
        let text = report.format_full_report();
        assert!(text.contains("(2 / 110 promised / 156 vocabulary / 242 constitutional_state_table)"));
        assert!(text.contains("Atoms Awaiting T20 (CIRISServer#809):"));
        assert!(text.contains("file.durable.recovered"));
    }
}
