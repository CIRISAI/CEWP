//! Three-Mode Runner Contract, `rows.jsonl` Trace Exporter, and Checkpoint Model Drift Witness.
//!
//! Connecting CSD/4 across three execution planes (FSD/SIMULATION_ENGINE.md §2.2 / §2.4):
//! - `Formal`: Translates scenario into TLC (`formal/cewp_node/CewpNode.tla` / `Trace.tla`) and Lean.
//! - `Sim`: Instantiates 1:1 or tiled cohort motifs in `cewp-sim` with stochastic arrivals and churn.
//! - `Native`: Real-node test harness (`ciris-server` NativeMesh) with HTTP witnesses.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use crate::catalogue::TopologyId;

/// The three execution modes of the unified CSD/4 runner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExecutionMode {
    /// Translates topology into TLC and Lean formal specifications.
    Formal,
    /// Executes discrete-event simulation inside cewp-sim.
    Sim,
    /// Invokes real ciris-server native mesh processes with live HTTP witness checks.
    Native,
}

impl ExecutionMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExecutionMode::Formal => "formal",
            ExecutionMode::Sim => "sim",
            ExecutionMode::Native => "native",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "formal" | "--mode formal" => Some(ExecutionMode::Formal),
            "sim" | "--mode sim" => Some(ExecutionMode::Sim),
            "native" | "--mode native" => Some(ExecutionMode::Native),
            _ => None,
        }
    }
}

impl fmt::Display for ExecutionMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ── rows.jsonl Exporter (Replayable V(rows, t) trace) ─────────────────────

/// One admitted federation row for rows.jsonl export (FSD/TOPOLOGY.md §4.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FederationRow {
    pub rowid: u64,
    pub dimension: String,
    pub cohort_scope: String,
    pub admitting_key_id: String,
    pub attesting_key_id: String,
    pub payload_hash: String,
    pub signature: String,
    pub admitted_step: usize,
}

impl FederationRow {
    pub fn to_json_line(&self) -> String {
        format!(
            "{{\"admitted_step\":{},\"admitting_key_id\":\"{}\",\"attesting_key_id\":\"{}\",\"cohort_scope\":\"{}\",\"dimension\":\"{}\",\"payload_hash\":\"{}\",\"rowid\":{},\"signature\":\"{}\"}}",
            self.admitted_step,
            self.admitting_key_id,
            self.attesting_key_id,
            self.cohort_scope,
            self.dimension,
            self.payload_hash,
            self.rowid,
            self.signature
        )
    }
}

/// Computes SHA-256 over a byte slice without external dependencies.
pub fn sha256_digest(input: &[u8]) -> String {
    // Pure std SHA-256 implementation
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];

    let k: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
        0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
        0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
        0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
        0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];

    let mut data = input.to_vec();
    let bit_len = (input.len() as u64) * 8;
    data.push(0x80);
    while (data.len() + 8) % 64 != 0 {
        data.push(0x00);
    }
    data.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in data.chunks(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([chunk[i * 4], chunk[i * 4 + 1], chunk[i * 4 + 2], chunk[i * 4 + 3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }

        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut h_val = h[7];

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h_val.wrapping_add(s1).wrapping_add(ch).wrapping_add(k[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h_val = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(h_val);
    }

    let mut out = String::with_capacity(64);
    for val in h {
        out.push_str(&format!("{:08x}", val));
    }
    out
}

/// Exports a series of federation rows as JSONL text and computes its SHA-256 digest.
pub fn export_rows_jsonl(rows: &[FederationRow]) -> (String, String) {
    let mut content = String::new();
    for row in rows {
        content.push_str(&row.to_json_line());
        content.push('\n');
    }
    let digest = sha256_digest(content.as_bytes());
    (content, digest)
}

// ── Checkpoint Summary & MODEL_DRIFT Witness ──────────────────────────────

/// Detailed witness mismatch when live node states disagree with model expectations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelDriftWitness {
    pub step_id: String,
    pub atom: String,
    pub model_expected: bool,
    pub live_witness: bool,
}

impl fmt::Display for ModelDriftWitness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MODEL_DRIFT on atom '{}' at step '{}': model expected {}, live witness was {}",
            self.atom, self.step_id, self.model_expected, self.live_witness
        )
    }
}

impl std::error::Error for ModelDriftWitness {}

/// Summary of a materialized checkpoint step during runner execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckpointSummary {
    pub entry_id: TopologyId,
    pub step_id: String,
    pub step_index: usize,
    pub expected_atoms: BTreeSet<String>,
    pub live_witness_readings: BTreeMap<String, bool>,
    pub node_endpoints: BTreeMap<String, String>,
}

impl CheckpointSummary {
    pub fn new(
        entry_id: TopologyId,
        step_id: &str,
        step_index: usize,
        expected: &[&str],
        witnesses: &[(&str, bool)],
    ) -> Self {
        let expected_atoms = expected.iter().map(|s| s.to_string()).collect();
        let live_witness_readings = witnesses.iter().map(|(k, v)| (k.to_string(), *v)).collect();
        Self {
            entry_id,
            step_id: step_id.to_string(),
            step_index,
            expected_atoms,
            live_witness_readings,
            node_endpoints: BTreeMap::new(),
        }
    }

    /// Verifies all live witness readings against model expectations.
    /// Returns Ok(()) if no drift is found; returns Err(ModelDriftWitness) on any divergence.
    pub fn check_model_drift(&self) -> Result<(), ModelDriftWitness> {
        for (atom, live_val) in &self.live_witness_readings {
            let model_has = self.expected_atoms.contains(atom);
            if *live_val != model_has {
                return Err(ModelDriftWitness {
                    step_id: self.step_id.clone(),
                    atom: atom.clone(),
                    model_expected: model_has,
                    live_witness: *live_val,
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_mode_parsing() {
        assert_eq!(ExecutionMode::from_str("formal"), Some(ExecutionMode::Formal));
        assert_eq!(ExecutionMode::from_str("sim"), Some(ExecutionMode::Sim));
        assert_eq!(ExecutionMode::from_str("native"), Some(ExecutionMode::Native));
        assert_eq!(ExecutionMode::Formal.as_str(), "formal");
    }

    #[test]
    fn test_export_rows_jsonl() {
        let rows = vec![
            FederationRow {
                rowid: 1,
                dimension: "note".to_string(),
                cohort_scope: "self".to_string(),
                admitting_key_id: "k1".to_string(),
                attesting_key_id: "k1".to_string(),
                payload_hash: "abcd1234".to_string(),
                signature: "sig1".to_string(),
                admitted_step: 3,
            },
        ];
        let (jsonl, digest) = export_rows_jsonl(&rows);
        assert!(jsonl.contains("\"dimension\":\"note\""));
        assert_eq!(digest.len(), 64);
    }

    #[test]
    fn test_checkpoint_model_drift_detection() {
        let ck = CheckpointSummary::new(
            TopologyId::T0,
            "nodes",
            0,
            &["node.unclaimed", "root.none"],
            &[
                ("node.unclaimed", true),
                ("root.halted", false),
            ],
        );
        assert!(ck.check_model_drift().is_ok());

        // Introduce drift: live node says unclaimed is false while model says true
        let ck_drift = CheckpointSummary::new(
            TopologyId::T0,
            "nodes",
            0,
            &["node.unclaimed", "root.none"],
            &[
                ("node.unclaimed", false), // Divergence!
            ],
        );
        let err = ck_drift.check_model_drift().unwrap_err();
        assert_eq!(err.atom, "node.unclaimed");
        assert_eq!(err.model_expected, true);
        assert_eq!(err.live_witness, false);
    }
}
