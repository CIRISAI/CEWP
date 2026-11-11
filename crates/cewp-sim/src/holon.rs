//! CIRISHolon Multiscale Coarse-Graining Certification (FSD §4.2, CIRISHolon).
//!
//! Models:
//! - `ClosureCertificate`: Commuting square closure defect D_v^2 <= β
//! - `WitnessPair`: Concrete detection of non-factoring states when ¬Closed
//! - `RemovabilityGate`: Coarse state variable certification with `PriceCertificate` vs `MaintenanceRent`
//! - `TierStack`: Sound tier composition (`TierStack::compose`, `closed_comp`)
//! - Non-expansive Lipschitz gate: K <= 1.05
//! - `horizon_budget(eps, k, n)`: Linear (n * ε) vs geometric error bounds
//! - `SecondSquareFence`: Refusing uncertified claim transport; `use_certificate` extends it to
//!   distributional certificates and their validity envelopes (`holon_dist`, FSD/twin/holon.md)

use crate::holon_dist::{DistributionalCertificate, EnvelopeBound, OperatingPoint};

/// The Non-Expansive Lipschitz threshold (K <= 1.05) per CIRISHolon.Budget.
pub const LIPSCHITZ_GATE_CEILING: f64 = 1.05;

/// Outcome of evaluating the commuting square closure defect between fine and coarse tiers.
#[derive(Debug, Clone, PartialEq)]
pub struct ClosureCertificate {
    /// Reference fine tier ID (e.g. "R5" or "R4").
    pub reference_tier: String,
    /// Target coarse tier ID (e.g. "R4" or "R3").
    pub coarse_tier: String,
    /// Squared closure defect D_v^2 = ||v(T(x)) - h(v(x))||^2 / ||v(T(x)) - v(x)||^2.
    pub closure_defect_d2: f64,
    /// Declared closure defect budget β.
    pub budget_beta: f64,
    /// Lipschitz amplification constant K.
    pub lipschitz_k: f64,
    /// True if D_v^2 <= β and K <= LIPSCHITZ_GATE_CEILING.
    pub certified: bool,
}

impl ClosureCertificate {
    /// Compute a closure certificate from measured residuals.
    #[must_use]
    pub fn compute(
        reference_tier: &str,
        coarse_tier: &str,
        norm_diff_sq: f64,
        norm_motion_sq: f64,
        budget_beta: f64,
        lipschitz_k: f64,
    ) -> Self {
        let d2 = if norm_motion_sq <= 1e-15 {
            if norm_diff_sq <= 1e-15 {
                0.0
            } else {
                f64::INFINITY
            }
        } else {
            norm_diff_sq / norm_motion_sq
        };

        let certified = d2 <= budget_beta && lipschitz_k <= LIPSCHITZ_GATE_CEILING;

        Self {
            reference_tier: reference_tier.to_string(),
            coarse_tier: coarse_tier.to_string(),
            closure_defect_d2: d2,
            budget_beta,
            lipschitz_k,
            certified,
        }
    }
}

/// A concrete witness pair (x, y) proving that a view `v` is NOT Closed under motion `T`:
/// `v(x) == v(y)` but `v(T(x)) != v(T(y))` (nonfactoring_iff_not_closed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitnessPair<X> {
    pub state_x: X,
    pub state_y: X,
}

impl<X> WitnessPair<X> {
    /// Verify whether two candidate states form a valid non-closure witness pair.
    pub fn verify<C, FView, FMotion>(
        state_x: &X,
        state_y: &X,
        view: FView,
        fine_motion: FMotion,
    ) -> bool
    where
        C: PartialEq,
        FView: Fn(&X) -> C,
        FMotion: Fn(&X) -> X,
    {
        // Must agree on the coarse view
        let vx = view(state_x);
        let vy = view(state_y);
        if vx != vy {
            return false;
        }

        // Must diverge after fine motion
        let tx = fine_motion(state_x);
        let ty = fine_motion(state_y);
        view(&tx) != view(&ty)
    }
}

/// Price certificate charging for dropped remainder weight when a coarse variable is pruned.
#[derive(Debug, Clone, PartialEq)]
pub struct PriceCertificate {
    pub variable_name: String,
    pub dropped_remainder_weight: f64,
    pub observable_prediction_error: f64,
}

/// Maintenance rent under decay λ and refresh dose q (Object.lean / GiniRent.lean):
/// G_inf = q / ((1 - λ) + q * λ).
#[derive(Debug, Clone, PartialEq)]
pub struct MaintenanceRent {
    pub decay_lambda: f64,
    pub dose_q: f64,
    pub equilibrium_retention_ginf: f64,
}

impl MaintenanceRent {
    /// Compute equilibrium rent G_inf from decay λ in [0, 1) and refresh dose q in (0, 1].
    #[must_use]
    pub fn compute(decay_lambda: f64, dose_q: f64) -> Self {
        let denom = (1.0 - decay_lambda) + dose_q * decay_lambda;
        let ginf = if denom <= 1e-15 {
            0.0
        } else {
            dose_q / denom
        };
        Self {
            decay_lambda,
            dose_q,
            equilibrium_retention_ginf: ginf,
        }
    }
}

/// Removability Gate: tests whether a retained coarse state variable earns its keep.
#[derive(Debug, Clone, PartialEq)]
pub enum RemovabilityVerdict {
    /// Removing the variable keeps prediction error <= β: variable is removable.
    Removable { price: PriceCertificate },
    /// Removing the variable causes error > β: variable is load-bearing and must be kept.
    RetainedLoadBearing {
        error: f64,
        threshold: f64,
        rent: MaintenanceRent,
    },
}

pub fn evaluate_removability_gate(
    variable_name: &str,
    dropped_weight: f64,
    error_without_variable: f64,
    budget_beta: f64,
    decay_lambda: f64,
    dose_q: f64,
) -> RemovabilityVerdict {
    if error_without_variable <= budget_beta {
        RemovabilityVerdict::Removable {
            price: PriceCertificate {
                variable_name: variable_name.to_string(),
                dropped_remainder_weight: dropped_weight,
                observable_prediction_error: error_without_variable,
            },
        }
    } else {
        RemovabilityVerdict::RetainedLoadBearing {
            error: error_without_variable,
            threshold: budget_beta,
            rent: MaintenanceRent::compute(decay_lambda, dose_q),
        }
    }
}

/// A certified tier boundary bundling a projection view, fine step, and coarse update.
#[derive(Debug, Clone)]
pub struct Tier<X, C> {
    pub name: String,
    pub view_fn: fn(&X) -> C,
    pub fine_step: fn(&X) -> X,
    pub coarse_update: fn(&C) -> C,
}

/// Stacking of tiers (CIRISHolon.Tier.stack / closed_comp).
pub struct TierStack;

impl TierStack {
    /// Compose two certified tier boundaries: fine tier (X -> C) and coarse tier (C -> D).
    /// Composite tier is (X -> D) with view `u ∘ v`, step `T`, and rate `g`.
    pub fn compose<X: 'static, C: 'static, D: 'static>(
        fine_name: &str,
        coarse_name: &str,
        v: fn(&X) -> C,
        t: fn(&X) -> X,
        h: fn(&C) -> C,
        u: fn(&C) -> D,
        g: fn(&D) -> D,
    ) -> (String, Box<dyn Fn(&X) -> D>, fn(&X) -> X, fn(&D) -> D) {
        let name = format!("{fine_name}->{coarse_name}");
        let composed_view = Box::new(move |x: &X| u(&v(x)));
        let _ = h;
        (name, composed_view, t, g)
    }
}

/// Multi-step approximation error budget:
/// For K <= 1.0: linear bound n * ε
/// For K > 1.0: geometric sum ε * (K^n - 1) / (K - 1).
#[must_use]
pub fn horizon_budget(eps: f64, k: f64, n: usize) -> f64 {
    if n == 0 || eps <= 0.0 {
        return 0.0;
    }
    if (k - 1.0).abs() <= 1e-9 || k < 1.0 {
        (n as f64) * eps
    } else {
        let kn = k.powi(n as i32);
        eps * (kn - 1.0) / (k - 1.0)
    }
}

/// Check the non-expansive Lipschitz gate K <= 1.05.
pub fn check_lipschitz_gate(k: f64) -> Result<(), &'static str> {
    if k <= LIPSCHITZ_GATE_CEILING {
        Ok(())
    } else {
        Err("Lipschitz constant K exceeds 1.05 ceiling")
    }
}

/// Second-Square Fence (CC §5.4.3 / CIRISHolon.Transport):
/// Refuses uncertified claim transport. A parameter correspondence between two scenarios
/// never transfers a validity certificate without re-verifying target certificate predicates.
#[derive(Debug, Clone, PartialEq)]
pub struct SecondSquareFence;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportRefusal {
    TargetPredicateFailed { reason: String },
    UncertifiedClaimTransport,
    /// The certificate does not certify this observable: it is not listed, or its measured
    /// W1 / quantile defects broke their budgets (`holon_dist`). Every breach is named.
    NotCertified { observable: String, breaches: Vec<EnvelopeBound> },
    /// The operating point lies outside the certificate's validity envelope: "I cannot predict
    /// this: the coarse model has left its certified envelope". Every broken bound is named.
    OutsideEnvelope { observable: String, broken: Vec<EnvelopeBound> },
}

impl SecondSquareFence {
    /// Transport a claim from source scenario to target scenario with target re-certification.
    pub fn transport_claim<P>(
        source_claim: &str,
        target_scenario_id: &str,
        target_predicate: P,
    ) -> Result<String, TransportRefusal>
    where
        P: FnOnce() -> Result<(), String>,
    {
        match target_predicate() {
            Ok(()) => Ok(format!(
                "CertifiedTransport[claim='{source_claim}',target='{target_scenario_id}']"
            )),
            Err(reason) => Err(TransportRefusal::TargetPredicateFailed { reason }),
        }
    }

    /// Use a distributional certificate for one observable at one operating point. The same
    /// fence, with the target predicate made explicit: the certificate is re-checked against
    /// the target (observable listed, its defects within budget, the point inside the validity
    /// envelope) and never carried on the strength of the source alone. A refusal names every
    /// bound that broke. Not Lean-checked: `claim_transport_does_not_grant_certificate` proves
    /// only that a claim map does not carry a certificate, which is why this re-checks.
    pub fn use_certificate(
        cert: &DistributionalCertificate,
        observable: &str,
        at: &OperatingPoint,
    ) -> Result<String, TransportRefusal> {
        let Some(o) = cert.covers(observable) else {
            return Err(TransportRefusal::NotCertified {
                observable: observable.to_string(),
                breaches: vec![EnvelopeBound::ObservableNotCertified { observable: observable.to_string() }],
            });
        };
        let breaches = o.breaches();
        if !breaches.is_empty() {
            return Err(TransportRefusal::NotCertified { observable: observable.to_string(), breaches });
        }
        let broken = cert.envelope.check(at);
        if !broken.is_empty() {
            return Err(TransportRefusal::OutsideEnvelope { observable: observable.to_string(), broken });
        }
        Ok(format!(
            "CertifiedUse[observable='{observable}',tiers='{}->{}']",
            cert.reference_tier, cert.coarse_tier
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closure_certificate_commuting_square() {
        // Certified case: D_v^2 = 0.014 <= 0.030, K = 1.018 <= 1.05
        let cert = ClosureCertificate::compute("R4", "R3", 0.014, 1.0, 0.030, 1.018);
        assert!(cert.certified);
        assert!((cert.closure_defect_d2 - 0.014).abs() < 1e-6);

        // Failing defect case: D_v^2 = 0.045 > 0.030
        let uncert_defect = ClosureCertificate::compute("R4", "R3", 0.045, 1.0, 0.030, 1.018);
        assert!(!uncert_defect.certified);

        // Failing Lipschitz case: K = 1.08 > 1.05
        let uncert_k = ClosureCertificate::compute("R4", "R3", 0.014, 1.0, 0.030, 1.08);
        assert!(!uncert_k.certified);
    }

    #[test]
    fn witness_pair_detection_when_not_closed() {
        // Fine state: (x1, x2). Coarse view: x1.
        // Fine motion: (x1 + x2, x2).
        // Let x = (10, 1), y = (10, 2).
        // view(x) = 10, view(y) = 10 (view agrees!)
        // fine_motion(x) = (11, 1) -> view is 11.
        // fine_motion(y) = (12, 2) -> view is 12 (view diverges!).
        // This is a concrete non-factoring witness pair!
        let state_x = (10, 1);
        let state_y = (10, 2);

        let view = |s: &(i32, i32)| s.0;
        let fine_motion = |s: &(i32, i32)| (s.0 + s.1, s.1);

        assert!(WitnessPair::verify(&state_x, &state_y, view, fine_motion));

        // When view does not diverge, it is NOT a witness pair
        let fine_motion_closed = |s: &(i32, i32)| (s.0 * 2, s.1);
        assert!(!WitnessPair::verify(
            &state_x,
            &state_y,
            view,
            fine_motion_closed
        ));
    }

    #[test]
    fn removability_gate_and_maintenance_rent() {
        // Rent formula G_inf = q / ((1 - λ) + q * λ)
        // With λ = 0.1 (decay), q = 0.2 (refresh dose):
        // denom = (1 - 0.1) + 0.2 * 0.1 = 0.9 + 0.02 = 0.92
        // G_inf = 0.2 / 0.92 ≈ 0.21739
        let rent = MaintenanceRent::compute(0.1, 0.2);
        assert!((rent.equilibrium_retention_ginf - (0.2 / 0.92)).abs() < 1e-5);

        // Variable is removable when error <= beta
        let verdict_removable =
            evaluate_removability_gate("stale_cache_count", 0.05, 0.01, 0.03, 0.1, 0.2);
        assert!(matches!(
            verdict_removable,
            RemovabilityVerdict::Removable { .. }
        ));

        // Variable is load-bearing when error > beta
        let verdict_retained =
            evaluate_removability_gate("leaf_load_metric", 0.50, 0.08, 0.03, 0.1, 0.2);
        assert!(matches!(
            verdict_retained,
            RemovabilityVerdict::RetainedLoadBearing { .. }
        ));
    }

    #[test]
    fn tier_stack_composition() {
        fn fine_view(x: &i32) -> i32 {
            x / 10
        }
        fn fine_step(x: &i32) -> i32 {
            x + 10
        }
        fn coarse_update_mid(c: &i32) -> i32 {
            c + 1
        }
        fn coarse_view(c: &i32) -> i32 {
            c / 2
        }
        fn coarse_update_top(d: &i32) -> i32 {
            d + 1
        }

        let (name, composed_view, t, g) = TierStack::compose(
            "Fine",
            "Coarse",
            fine_view,
            fine_step,
            coarse_update_mid,
            coarse_view,
            coarse_update_top,
        );

        assert_eq!(name, "Fine->Coarse");
        assert_eq!(composed_view(&200), 10);
        assert_eq!(t(&200), 210);
        assert_eq!(g(&10), 11);
    }

    #[test]
    fn lipschitz_gate_and_horizon_budget() {
        assert_eq!(check_lipschitz_gate(1.02), Ok(()));
        assert_eq!(check_lipschitz_gate(1.05), Ok(()));
        assert!(check_lipschitz_gate(1.06).is_err());

        // Linear scaling when K <= 1.0: n * eps
        let budget_linear = horizon_budget(0.01, 1.0, 10);
        assert!((budget_linear - 0.10).abs() < 1e-6);

        // Controlled geometric growth when K = 1.02 <= 1.05
        let budget_geom = horizon_budget(0.01, 1.02, 10);
        // eps * (1.02^10 - 1) / 0.02 ≈ 0.01 * (1.21899 - 1) / 0.02 ≈ 0.1095
        assert!(budget_geom > 0.10 && budget_geom < 0.12);
    }

    #[test]
    fn second_square_fence_refuses_uncertified_transport() {
        // Success case when target predicate re-certifies
        let cert_res = SecondSquareFence::transport_claim(
            "k2_leaf_load_le_0_6",
            "scenario_t5_flash_crowd",
            || Ok(()),
        );
        assert!(cert_res.is_ok());

        // Refusal case when target predicate fails
        let fail_res = SecondSquareFence::transport_claim(
            "k2_leaf_load_le_0_6",
            "scenario_adversarial_cgnat",
            || Err("target violates reach_home_share constraint".to_string()),
        );
        assert!(matches!(
            fail_res,
            Err(TransportRefusal::TargetPredicateFailed { .. })
        ));
    }
}
