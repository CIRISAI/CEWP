//! Subset-Search Cost-Minimizing Planner (`plan_tests`).
//!
//! Given a set of CSD test requests `(card_id, test_name, topology_id, required_atoms)`,
//! finds valid step checkpoints `(T_n, s)` where `required_atoms ⊆ state(T_n, s)`
//! and minimizes `cost = (builds, 10 * nodes + steps)`.
//!
//! Includes folding compatible single-node tests (e.g. from `T0`) into an already-scheduled
//! multi-node topology (e.g. `T2`) with zero extra builds.

use std::collections::{BTreeMap, BTreeSet};
use std::cmp::Ordering;
use crate::catalogue::{TopologyId, VOCABULARY_ATOMS};

/// A request for a CSD test execution at a specific state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsdTestRequest {
    pub card_id: String,
    pub test_name: String,
    pub topology_id: Option<TopologyId>,
    pub required_atoms: Vec<String>,
}

impl CsdTestRequest {
    pub fn new(card_id: &str, test_name: &str, topology: Option<TopologyId>, required: &[&str]) -> Self {
        Self {
            card_id: card_id.to_string(),
            test_name: test_name.to_string(),
            topology_id: topology,
            required_atoms: required.iter().map(|s| s.to_string()).collect(),
        }
    }

    pub fn key(&self) -> String {
        format!("{}/{}", self.card_id, self.test_name)
    }
}

/// The multi-dimensional cost metric for executing a test plan.
/// Strictly ordered by number of builds first, then total node+step weight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlanCost {
    pub builds: usize,
    pub node_step_cost: usize,
}

impl PlanCost {
    pub const fn new(builds: usize, node_step_cost: usize) -> Self {
        Self { builds, node_step_cost }
    }
}

impl PartialOrd for PlanCost {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PlanCost {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.builds.cmp(&other.builds) {
            Ordering::Equal => self.node_step_cost.cmp(&other.node_step_cost),
            ord => ord,
        }
    }
}

/// Options controlling planner search and optimization behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlannerOptions {
    /// Allow reselecting a cheaper topology than the card's declared hint if it reaches the states.
    pub allow_reselection: bool,
    /// Allow folding compatible tests (such as single-node tests) into an already scheduled topology.
    pub allow_folding: bool,
}

impl Default for PlannerOptions {
    fn default() -> Self {
        Self {
            allow_reselection: true,
            allow_folding: true,
        }
    }
}

/// The scheduled execution plan for a suite of CSD tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// Chosen topology entries to build.
    pub entries: Vec<TopologyId>,
    /// Minimum total cost of the plan.
    pub cost: PlanCost,
    /// Schedule: topology -> step_id -> list of test keys scheduled after that step.
    pub schedule: BTreeMap<TopologyId, BTreeMap<String, Vec<String>>>,
    /// Reselected tests: test_key -> "T_orig -> T_chosen".
    pub reselected: BTreeMap<String, String>,
    /// Uncovered tests: test_key -> reason why no entry reaches it.
    pub uncovered: BTreeMap<String, String>,
    /// Tests folded into a higher-tier topology with 0 extra builds.
    pub folded_into: BTreeMap<String, TopologyId>,
}

/// Plans a set of CSD test requests across the closed catalogue T0–T19.
pub fn plan_tests(requests: &[CsdTestRequest], options: &PlannerOptions) -> Plan {
    let vocab_set: BTreeSet<&str> = VOCABULARY_ATOMS.iter().copied().collect();

    // 1. Identify which topologies can satisfy each requirement at which earliest step
    // map: req_key -> (TopologyId -> earliest_step_idx)
    let mut hit_map: BTreeMap<String, BTreeMap<TopologyId, usize>> = BTreeMap::new();
    let mut uncovered: BTreeMap<String, String> = BTreeMap::new();

    for req in requests {
        let key = req.key();
        let req_atoms_str: Vec<&str> = req.required_atoms.iter().map(|s| s.as_str()).collect();

        // Check if all atoms are known
        let unknown: Vec<String> = req.required_atoms.iter()
            .filter(|a| !vocab_set.contains(a.as_str()))
            .cloned()
            .collect();
        if !unknown.is_empty() {
            uncovered.insert(key.clone(), format!("atoms not in catalogue vocabulary: {:?}", unknown));
            continue;
        }

        let mut hits = BTreeMap::new();
        for id in TopologyId::ALL {
            if !options.allow_reselection {
                if let Some(hint) = req.topology_id {
                    if hint != id {
                        continue;
                    }
                }
            }
            let decl = id.get();
            if let Some(step_idx) = decl.earliest_step_for(&req_atoms_str) {
                hits.insert(id, step_idx);
            }
        }

        if hits.is_empty() {
            uncovered.insert(key.clone(), format!("no catalogue entry reaches {:?} at one instant", req.required_atoms));
        } else {
            hit_map.insert(key, hits);
        }
    }

    let coverable_requests: Vec<&CsdTestRequest> = requests.iter()
        .filter(|r| hit_map.contains_key(&r.key()))
        .collect();

    if coverable_requests.is_empty() {
        return Plan {
            entries: Vec::new(),
            cost: PlanCost::new(0, 0),
            schedule: BTreeMap::new(),
            reselected: BTreeMap::new(),
            uncovered,
            folded_into: BTreeMap::new(),
        };
    }

    // 2. Candidate topologies that cover at least one requirement
    let mut candidate_topos: BTreeSet<TopologyId> = BTreeSet::new();
    for req in &coverable_requests {
        if let Some(hits) = hit_map.get(&req.key()) {
            for &topo_id in hits.keys() {
                candidate_topos.insert(topo_id);
            }
        }
    }

    let candidates: Vec<TopologyId> = candidate_topos.into_iter().collect();

    // 3. Exact subset search: find subset of candidates that covers all coverable requirements with minimal cost (builds, node_step_cost)
    let n = candidates.len();
    let mut best_subset: Option<(PlanCost, Vec<TopologyId>)> = None;

    // Search combinations by size: 1, 2, ..., n
    for k in 1..=n {
        let mut found_for_size = false;

        // Iterate through all combinations of size k
        for combo in Combinations::new(&candidates, k) {
            let covers_all = coverable_requests.iter().all(|req| {
                let hits = hit_map.get(&req.key()).unwrap();
                combo.iter().any(|c| hits.contains_key(c))
            });

            if covers_all {
                let total_node_step_cost: usize = combo.iter().map(|c| c.get().cost).sum();
                let cost = PlanCost::new(combo.len(), total_node_step_cost);

                match &best_subset {
                    None => {
                        best_subset = Some((cost, combo));
                        found_for_size = true;
                    }
                    Some((best_cost, _)) => {
                        if cost < *best_cost {
                            best_subset = Some((cost, combo));
                            found_for_size = true;
                        }
                    }
                }
            }
        }

        // Since combinations are tested in increasing size k (builds),
        // once we find valid covers of size k, we don't need size k+1 because builds will strictly increase!
        if found_for_size {
            break;
        }
    }

    let (final_cost, chosen_entries) = best_subset.unwrap_or_else(|| (PlanCost::new(0, 0), Vec::new()));

    // 4. Assign each coverable test to its earliest checkpoint in the chosen entries
    let mut schedule: BTreeMap<TopologyId, BTreeMap<String, Vec<String>>> = BTreeMap::new();
    let mut reselected: BTreeMap<String, String> = BTreeMap::new();
    let mut folded_into: BTreeMap<String, TopologyId> = BTreeMap::new();

    for topo_id in &chosen_entries {
        schedule.insert(*topo_id, BTreeMap::new());
    }

    for req in coverable_requests {
        let key = req.key();
        let hits = hit_map.get(&key).unwrap();

        // If the card's hinted topology is among chosen entries and reaches the states, prefer it unless reselection/folding applies
        let chosen_topo = if let Some(hint) = req.topology_id {
            if chosen_entries.contains(&hint) && hits.contains_key(&hint) {
                hint
            } else {
                // Pick the chosen topology that hits it
                *chosen_entries.iter().find(|c| hits.contains_key(c)).unwrap()
            }
        } else {
            *chosen_entries.iter().find(|c| hits.contains_key(c)).unwrap()
        };

        if let Some(orig) = req.topology_id {
            if orig != chosen_topo {
                reselected.insert(key.clone(), format!("{} → {}", orig, chosen_topo));
                if orig == TopologyId::T0 && chosen_topo != TopologyId::T0 {
                    folded_into.insert(key.clone(), chosen_topo);
                }
            }
        }

        let step_idx = hits[&chosen_topo];
        let topo_decl = chosen_topo.get();
        let step_id = topo_decl.steps[step_idx].id.to_string();

        schedule.get_mut(&chosen_topo)
            .unwrap()
            .entry(step_id)
            .or_default()
            .push(key);
    }

    Plan {
        entries: chosen_entries,
        cost: final_cost,
        schedule,
        reselected,
        uncovered,
        folded_into,
    }
}

// ── Combinations helper ────────────────────────────────────────────────────

struct Combinations<'a, T> {
    slice: &'a [T],
    k: usize,
    indices: Vec<usize>,
    first: bool,
}

impl<'a, T> Combinations<'a, T> {
    fn new(slice: &'a [T], k: usize) -> Self {
        let indices = (0..k).collect();
        Self {
            slice,
            k,
            indices,
            first: true,
        }
    }
}

impl<'a, T: Clone> Iterator for Combinations<'a, T> {
    type Item = Vec<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.k == 0 || self.k > self.slice.len() {
            return None;
        }
        if self.first {
            self.first = false;
            return Some(self.indices.iter().map(|&i| self.slice[i].clone()).collect());
        }

        let mut i = self.k;
        while i > 0 {
            i -= 1;
            if self.indices[i] < self.slice.len() - (self.k - i) {
                self.indices[i] += 1;
                for j in (i + 1)..self.k {
                    self.indices[j] = self.indices[j - 1] + 1;
                }
                return Some(self.indices.iter().map(|&idx| self.slice[idx].clone()).collect());
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_planner_single_request() {
        let reqs = vec![
            CsdTestRequest::new("CSD-118", "first_run", Some(TopologyId::T0), &["node.unclaimed"]),
        ];
        let plan = plan_tests(&reqs, &PlannerOptions::default());
        assert_eq!(plan.entries.len(), 1);
        assert_eq!(plan.cost.builds, 1);
        assert!(plan.schedule.contains_key(&TopologyId::T0));
    }

    #[test]
    fn test_planner_folds_t0_into_t2() {
        // Prompt requirement: Folding compatible T0 single-node tests into an already-scheduled T2 build with zero extra builds!
        // Suppose we have T0 tests:
        //   CSD-118/first_run -> [node.unclaimed]
        //   CSD-118/declared -> [node.claimed]
        // And T2 tests:
        //   CSD-091/conversation -> [contact.reachable, room.pair.keyed]
        //   CSD-091/outsider -> [person.outsider]
        let reqs = vec![
            CsdTestRequest::new("CSD-118", "first_run", Some(TopologyId::T0), &["node.unclaimed"]),
            CsdTestRequest::new("CSD-118", "declared", Some(TopologyId::T0), &["node.claimed"]),
            CsdTestRequest::new("CSD-091", "conversation", Some(TopologyId::T2), &["contact.reachable", "room.pair.keyed"]),
            CsdTestRequest::new("CSD-091", "outsider", Some(TopologyId::T2), &["person.outsider"]),
        ];

        let plan = plan_tests(&reqs, &PlannerOptions::default());

        // The planner should find that building ONLY T2 covers ALL 4 tests, requiring only 1 build!
        assert_eq!(plan.entries.len(), 1, "Expected exactly 1 build due to folding");
        assert_eq!(plan.entries[0], TopologyId::T2, "Expected T2 to be chosen as host");
        assert_eq!(plan.cost.builds, 1);

        // Verify CSD-118 tests were reselected and folded into T2
        assert_eq!(plan.reselected.get("CSD-118/first_run"), Some(&"T0 → T2".to_string()));
        assert_eq!(plan.reselected.get("CSD-118/declared"), Some(&"T0 → T2".to_string()));
        assert_eq!(plan.folded_into.get("CSD-118/first_run"), Some(&TopologyId::T2));
        assert_eq!(plan.folded_into.get("CSD-118/declared"), Some(&TopologyId::T2));

        // Checkpoint verification in T2:
        // first_run runs at step 1 (nodes) where node.unclaimed holds
        // declared runs at step 2 (person:alice) where node.claimed holds
        let t2_sched = plan.schedule.get(&TopologyId::T2).expect("T2 schedule");
        assert!(t2_sched.get("nodes").unwrap().contains(&"CSD-118/first_run".to_string()));
        assert!(t2_sched.get("person:alice").unwrap().contains(&"CSD-118/declared".to_string()));
    }
}
