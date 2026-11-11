//! Correlated failure domains and durability: `cargo run -p cewp-sim --example failure_domains`.
//!
//! Prints the generated world, then for each of the nine scenarios the domain event against
//! the same number of nodes lost at random (p50 / p95 / p99 / mean over seeds), then the
//! review's named durability question across placements, holder counts and granularity, then
//! every parameter with its class. Arguments: `seeds=N`, `world_seed=N`, `review_seeds=N`, and
//! any failure parameter as `name=value` (see `FAILURE_SPECS`). Every output is MODEL: the
//! twin's result under the tagged inputs printed at the end.

use cewp_sim::durability::{review_question, review_sweep, Granularity, ReviewScenario};
use cewp_sim::failure::{compare, DomainWorld, FailureParams, Probe, FAILURE_SPECS, OUTCOMES, SCENARIOS};

fn main() {
    let mut p = FailureParams::default();
    let mut seeds = 200usize;
    let mut review_seeds = 400usize;
    let mut world_seed = 2026u64;
    for arg in std::env::args().skip(1) {
        let Some((k, v)) = arg.split_once('=') else {
            eprintln!("ignored argument {arg}: expected name=value");
            continue;
        };
        let Ok(x) = v.parse::<f64>() else {
            eprintln!("ignored {arg}: value is not a number");
            continue;
        };
        match k {
            "seeds" => seeds = x as usize,
            "review_seeds" => review_seeds = x as usize,
            "world_seed" => world_seed = x as u64,
            _ => {
                if !p.set_by_name(k, x) {
                    eprintln!("ignored {arg}: no parameter named {k}");
                }
            }
        }
    }
    let w = DomainWorld::build(world_seed, &p);
    println!("# Failure domains (all outputs MODEL)\n");
    println!(
        "World seed {world_seed}: {} nodes, {} persons, {} households, {} countries, {} metros, {} power regions, {} access networks, {} cables, {} rooms.",
        w.nodes.len(),
        w.persons.len(),
        w.households,
        w.countries,
        w.metros.len(),
        w.power_regions,
        w.asns.len(),
        w.cables.len(),
        w.rooms.len()
    );
    for (k, n) in w.kind_counts() {
        println!("- {}: {n}", k.as_str());
    }
    let probe = Probe::default();
    println!(
        "\n## Domain event vs the same number of nodes lost at random ({seeds} seeds, hour {} after onset)\n",
        p.get(cewp_sim::failure::F::EvalHour)
    );
    println!(
        "Durability probe: {} holders per chunk, {}, 10 GB object, {} publishers per seed.\n",
        probe.holders,
        probe.placement.label(),
        probe.publishers
    );
    println!("| scenario | outcome | domain p50 | p95 | p99 | mean | random p50 | p95 | p99 | mean |");
    println!("|---|---|---|---|---|---|---|---|---|---|");
    for kind in SCENARIOS {
        let r = compare(&w, &p, kind, seeds, &probe);
        println!(
            "| {} | nodes lost | {:.0} | {:.0} | {:.0} | {:.1} | (same) | | | |",
            kind.as_str(),
            r.lost.p50,
            r.lost.p95,
            r.lost.p99,
            r.lost.mean
        );
        println!(
            "| {} | duration h | {:.1} | {:.1} | {:.1} | {:.1} | | | | |",
            kind.as_str(),
            r.duration_h.p50,
            r.duration_h.p95,
            r.duration_h.p99,
            r.duration_h.mean
        );
        for (k, name) in OUTCOMES.iter().enumerate() {
            let (d, x) = (r.domain[k], r.random[k]);
            println!(
                "| {} | {name} | {:.4e} | {:.4e} | {:.4e} | {:.4e} | {:.4e} | {:.4e} | {:.4e} | {:.4e} |",
                kind.as_str(),
                d.p50,
                d.p95,
                d.p99,
                d.mean,
                x.p50,
                x.p95,
                x.p99,
                x.mean
            );
        }
    }
    for on_lost in [false, true] {
        let sc = ReviewScenario { publisher_on_lost_isp: on_lost, ..ReviewScenario::default() };
        println!(
            "\n## Review question: popular 10 GB object, two of the four largest ISPs down, 30% of home-class nodes gone ({review_seeds} seeds; publisher {})\n",
            if on_lost { "a customer of a lost ISP" } else { "anyone" }
        );
        println!("P(NOT fully recoverable) over seeds; 'during' = ISPs down and 30% gone, 'after' = ISPs back.\n");
        println!("| placement | holders | granularity | during mean | p50 | p95 | p99 | after mean | p50 | p95 | p99 | k_eff p50 | k_eff p05 | axis n_eff p50 |");
        println!("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|");
        for row in review_question(&w, &p, &sc, &review_sweep(), review_seeds) {
            let g = match row.cfg.granularity {
                Granularity::WholeObject => "whole object".to_string(),
                Granularity::PerChunk { chunks } => format!("per chunk ({chunks})"),
            };
            let (d, a) = (row.during, row.after);
            println!(
                "| {} | {} | {} | {:.3e} | {:.3e} | {:.3e} | {:.3e} | {:.3e} | {:.3e} | {:.3e} | {:.3e} | {:.2} | {:.2} | {:.0} |",
                row.cfg.placement.label(),
                row.cfg.holders,
                g,
                d.mean,
                d.p50,
                d.p95,
                d.p99,
                a.mean,
                a.p50,
                a.p95,
                a.p99,
                row.k_eff.p50,
                row.k_eff.p05,
                row.axis_n_eff.p50
            );
        }
    }
    println!("\n## Parameters\n");
    println!("| name | value | class | source |");
    println!("|---|---|---|---|");
    for (i, s) in FAILURE_SPECS.iter().enumerate() {
        println!("| {} | {} | {} | {} |", s.name, p.v[i], s.class.as_str(), s.source);
    }
}
