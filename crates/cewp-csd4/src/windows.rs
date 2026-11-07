//! Step-window model and transient state atom calculation.
//!
//! Provides step-level state calculation [start, end) intervals during which
//! each state atom holds, and verification that test atoms co-hold simultaneously.

use std::collections::BTreeSet;
use crate::catalogue::{StepDecl, TraceStepDecl, WindowDecl, TopologyDecl};

/// Compute transient windows `[start, end)` for all atoms appearing in the trace.
/// An atom is transient if it holds at some step, but ceases to hold at or before the final step.
pub fn compute_transient_windows(trace: &[TraceStepDecl]) -> Vec<WindowDecl> {
    if trace.is_empty() {
        return Vec::new();
    }

    let mut atom_steps: std::collections::BTreeMap<&'static str, Vec<usize>> = std::collections::BTreeMap::new();
    for (idx, step) in trace.iter().enumerate() {
        for &atom in step.atoms {
            atom_steps.entry(atom).or_default().push(idx);
        }
    }

    let total_steps = trace.len();
    let mut out = Vec::new();

    for (atom, steps) in atom_steps {
        if !steps.is_empty() {
            let first = steps[0];
            let last = *steps.last().unwrap();
            // Transient if it ceases to hold before the final step
            if last < total_steps - 1 {
                out.push(WindowDecl::new(atom, first, last + 1));
            }
        }
    }

    out
}

/// Returns cumulative atoms after executing through `step_idx` of a list of steps.
pub fn simulate_steps(steps: &[StepDecl], initial_root_none: bool) -> Vec<BTreeSet<&'static str>> {
    let mut state: BTreeSet<&'static str> = BTreeSet::new();
    if initial_root_none {
        state.insert("root.none");
    }

    let mut trace = Vec::with_capacity(steps.len());
    for s in steps {
        for rem in s.removes {
            state.remove(rem);
        }
        for add in s.adds {
            state.insert(add);
        }
        trace.push(state.clone());
    }
    trace
}

/// Checks whether a set of required atoms co-holds simultaneously at any step of the topology.
pub fn atoms_co_hold_in_topology(topology: &TopologyDecl, required_atoms: &[&str]) -> bool {
    if required_atoms.is_empty() {
        return true;
    }
    topology.earliest_step_for(required_atoms).is_some()
}

/// Returns all step indices in the topology where the required atoms co-hold.
pub fn co_holding_step_indices(topology: &TopologyDecl, required_atoms: &[&str]) -> Vec<usize> {
    if required_atoms.is_empty() {
        return (0..topology.trace.len()).collect();
    }
    let mut indices = Vec::new();
    for (idx, ts) in topology.trace.iter().enumerate() {
        let all_present = required_atoms.iter().all(|req| ts.atoms.iter().any(|a| a == req));
        if all_present {
            indices.push(idx);
        }
    }
    indices
}

/// Verifies whether a step falls inside a given window `[start, end)`.
pub fn step_in_window(window: &WindowDecl, step: usize) -> bool {
    window.contains_step(step)
}

/// Returns all active atoms according to the transient and persistent windows at a given step.
pub fn active_atoms_at_step<'a>(topology: &'a TopologyDecl, step_idx: usize) -> Option<&'a [&'static str]> {
    topology.state_at(step_idx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalogue::TopologyId;

    #[test]
    fn test_transient_window_t0() {
        let t0 = TopologyId::T0.get();
        // Prompt requirement: T0 has node.unclaimed at [0, 1)
        let unclaimed_win = t0.model_window("node.unclaimed").expect("node.unclaimed window in T0");
        assert_eq!(unclaimed_win.start, 0);
        assert_eq!(unclaimed_win.end, 1);
        assert!(unclaimed_win.contains_step(0));
        assert!(!unclaimed_win.contains_step(1));

        // Semantic window in T0: node.claimed_sole at [1, end)
        let claimed_win = t0.semantic_window("node.claimed_sole").expect("node.claimed_sole semantic window");
        assert_eq!(claimed_win.start, 1);
        assert!(claimed_win.contains_step(1));
    }

    #[test]
    fn test_transient_window_t3() {
        let t3 = TopologyId::T3.get();
        // Prompt requirement: T3 transient windows
        let req_win = t3.semantic_window("device.join_requested").expect("device.join_requested window");
        assert_eq!(req_win.start, 3);
        assert_eq!(req_win.end, 4);

        let app_win = t3.semantic_window("device.join_approved").expect("device.join_approved window");
        assert_eq!(app_win.start, 4);
    }

    #[test]
    fn test_transient_window_t8() {
        let t8 = TopologyId::T8.get();
        // Prompt requirement: T8 root.halted at [1, 2), root.halt_confirmed at [2, 3), root.resumed at [3, end)
        let halted_win = t8.semantic_window("root.halted").expect("root.halted window");
        assert_eq!(halted_win.start, 1);
        assert_eq!(halted_win.end, 2);

        let conf_win = t8.semantic_window("root.halt_confirmed").expect("root.halt_confirmed window");
        assert_eq!(conf_win.start, 2);
        assert_eq!(conf_win.end, 3);

        let res_win = t8.semantic_window("root.resumed").expect("root.resumed window");
        assert_eq!(res_win.start, 3);
    }

    #[test]
    fn test_co_holding_t0_atoms() {
        let t0 = TopologyId::T0.get();
        // root.none and node.unclaimed co-hold at step 0
        assert!(atoms_co_hold_in_topology(t0, &["root.none", "node.unclaimed"]));
        assert_eq!(t0.earliest_step_for(&["root.none", "node.unclaimed"]), Some(0));

        // root.none and node.claimed co-hold at step 1
        assert!(atoms_co_hold_in_topology(t0, &["root.none", "node.claimed"]));
        assert_eq!(t0.earliest_step_for(&["root.none", "node.claimed"]), Some(1));

        // node.unclaimed and node.claimed NEVER co-hold in T0
        assert!(!atoms_co_hold_in_topology(t0, &["node.unclaimed", "node.claimed"]));
        assert_eq!(t0.earliest_step_for(&["node.unclaimed", "node.claimed"]), None);
    }
}
