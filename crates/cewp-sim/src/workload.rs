//! Compound-Poisson Workload, 4 Baselines (A1–A4), and Parameter Regimes (FSD §4).
//!
//! Models:
//! - Corrected dimensional rate: λ(t) = μ_diurnal(t_local) * (D / (E[B] * 86400)) [events / s]
//! - Discrete timestep arrival probability: P(K >= 1) = 1 - e^(-λ * Δt)
//! - Poisson event count generation: K ~ Poisson(λ * Δt)
//! - IMIX payload sampling across 6 categories (RFC 6985 & Sandvine/Ericsson mixtures)
//! - 4 Credible Baseline Architectures:
//!   - A1_HYPERSCALE_CDN: Centralized Origin + Edge CDN + ISP Cache
//!   - A2_MULTIREGION_SFU: Multi-Region Active-Active Cloud + SFU Mesh
//!   - A3_HYBRID_P2P: Hybrid P2P with Central Bootstrap / Relays
//!   - A4_CEWP_RC17: CEWP Constitutional Substrate (CC §4.5.13, §5.4.3, §6.1.6, §6.1.2)
//! - 4 Parameter Regimes: Optimistic, Expected, Pessimistic, Adversarial

/// Seconds in one standard astronomical day.
pub const SECONDS_PER_DAY: f64 = 86_400.0;

/// Daily low of [`diurnal_multiplier`], times the daily mean (MODEL stand-in, no reference).
pub const DIURNAL_TROUGH: f64 = 0.50;
/// Daily high of [`diurnal_multiplier`], times the daily mean (MODEL stand-in, no reference).
pub const DIURNAL_PEAK: f64 = 1.60;
/// Local hour of the daily high (MODEL stand-in, no reference).
pub const DIURNAL_PEAK_HOUR: f64 = 19.5;
// The two harmonics fitted (once, offline, Nelder-Mead) so the curve's low, high and the high's hour
// are the three constants above and the 09:00-17:00 stretch is as flat as two harmonics allow; the
// test `diurnal_multiplier_hits_its_stated_trough_peak_and_hour` holds them to the constants.
const DIURNAL_A1: f64 = 0.3929;
const DIURNAL_H1: f64 = 18.0014;
const DIURNAL_A2: f64 = 0.2488;
const DIURNAL_H2: f64 = 8.0926;

/// Normalized diurnal activity multiplier at local time `t_local_secs` (0 .. 86,400): a MODEL
/// stand-in with no reference (registered on CIRISHolon's side as generator `model.diurnal.v1`; the
/// previous curve, peak 2.3x at 20:30 and 0.2x before dawn, was `model.diurnal.v0`).
///
/// Two harmonics, integrating to exactly 1.0 over 24 h: a low of [`DIURNAL_TROUGH`] (0.50x)
/// around 03:10, a daytime stretch of 0.89x to 1.21x from 09:00 to 16:00 (rising to 1.37x by 17:00),
/// and an evening high of
/// [`DIURNAL_PEAK`] (1.60x) at [`DIURNAL_PEAK_HOUR`] (19:30). Peak over trough is 3.2, not the 11.5
/// of v0: people's AI agents, scheduled activity and other always-on use give the mesh a floor that
/// human-only traffic does not have, and messaging is a waking-hours plateau rather than an evening
/// spike (Eric, 2026-11-11). Two harmonics cannot hold the plateau at a full 1.0 while also hitting
/// the trough, the peak and its hour exactly; the morning of the plateau sits at 0.89x to 0.95x.
#[must_use]
pub fn diurnal_multiplier(t_local_secs: f64) -> f64 {
    let hour = t_local_secs.rem_euclid(SECONDS_PER_DAY) / 3600.0;
    let w = std::f64::consts::PI / 12.0;
    1.0 + DIURNAL_A1 * (w * (hour - DIURNAL_H1)).cos() + DIURNAL_A2 * (2.0 * w * (hour - DIURNAL_H2)).cos()
}

/// Compute dimensional event arrival rate:
/// λ(t) = μ_diurnal(t_local) * D / (E[B] * 86,400) [events / second].
///
/// - `daily_bytes`: Total produced or consumed bytes per day (D)
/// - `mean_payload_bytes`: Expected payload size in bytes per event (E[B])
/// - `t_local_secs`: Local time of day in seconds
#[must_use]
pub fn compute_arrival_rate(
    daily_bytes: f64,
    mean_payload_bytes: f64,
    t_local_secs: f64,
) -> f64 {
    if daily_bytes <= 0.0 || mean_payload_bytes <= 0.0 {
        return 0.0;
    }
    let mu = diurnal_multiplier(t_local_secs);
    mu * (daily_bytes / (mean_payload_bytes * SECONDS_PER_DAY))
}

/// Probability of at least one arrival during discrete timestep Δt:
/// P(K >= 1) = 1 - e^(-λ * Δt).
#[must_use]
pub fn timestep_arrival_probability(lambda: f64, delta_t_secs: f64) -> f64 {
    if lambda <= 0.0 || delta_t_secs <= 0.0 {
        return 0.0;
    }
    1.0 - (-lambda * delta_t_secs).exp()
}

/// Knuth's algorithm for drawing a Poisson random variable K ~ Poisson(λ * Δt)
/// using a deterministic pseudo-random linear congruential generator seed.
#[must_use]
pub fn sample_poisson(lambda_delta_t: f64, seed: &mut u64) -> u64 {
    if lambda_delta_t <= 0.0 {
        return 0;
    }
    // L = e^(-λΔt)
    let l = (-lambda_delta_t).exp();
    let mut k = 0u64;
    let mut p = 1.0f64;

    // LCG random generator step
    while p > l {
        k += 1;
        *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let u = (*seed >> 11) as f64 / ((1u64 << 53) as f64);
        p *= u.max(1e-15);
        if k > 10_000 {
            break; // safety bound
        }
    }
    k.saturating_sub(1)
}

/// The 6 IMIX payload categories (FSD §4.1, RFC 6985).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImixCategory {
    /// Compact signed CEG rows (256 B .. 1,536 B, mean ~512 B).
    CompactCegRow = 0,
    /// H3ERE agent traces (~14 KiB = 14,336 B).
    AgentTraceH3ere = 1,
    /// Small attachments, icons, avatars (<= 64 KiB, mean ~32 KiB).
    SmallAttachment = 2,
    /// Inline short-form media / images (<= 16 MiB, mean ~1 MiB).
    InlineMedia = 3,
    /// Bulk files, video, audio (> 64 KiB, mean ~10 MiB).
    BulkFile = 4,
    /// External VoD / bulk pointer rows (~400 B).
    ExternalPointer = 5,
}

impl ImixCategory {
    /// Nominal mean payload bytes for this category.
    #[must_use]
    pub const fn mean_bytes(self) -> u64 {
        match self {
            Self::CompactCegRow => 512,
            Self::AgentTraceH3ere => 14_336,
            Self::SmallAttachment => 32_768,
            Self::InlineMedia => 1_048_576,
            Self::BulkFile => 10_485_760,
            Self::ExternalPointer => 400,
        }
    }

    /// Whether this category is constitutionally allowed over shaped links (<= 64 KiB).
    #[must_use]
    pub const fn allowed_on_shaped_link(self) -> bool {
        match self {
            Self::CompactCegRow | Self::AgentTraceH3ere | Self::SmallAttachment | Self::ExternalPointer => true,
            Self::InlineMedia | Self::BulkFile => false,
        }
    }
}

/// Distribution profile across the 6 IMIX categories.
#[derive(Debug, Clone, PartialEq)]
pub struct ImixDistribution {
    /// Probabilities summing to 1.0 for each of the 6 categories.
    pub weights: [f64; 6],
}

impl Default for ImixDistribution {
    fn default() -> Self {
        // Standard social/mesh profile:
        // 60% compact CEG rows, 15% agent traces, 15% small attachments,
        // 6% inline media, 1% bulk files, 3% external pointers.
        Self {
            weights: [0.60, 0.15, 0.15, 0.06, 0.01, 0.03],
        }
    }
}

impl ImixDistribution {
    /// Compute the overall expected payload size E[B] in bytes.
    #[must_use]
    pub fn mean_payload_bytes(&self) -> f64 {
        let cats = [
            ImixCategory::CompactCegRow,
            ImixCategory::AgentTraceH3ere,
            ImixCategory::SmallAttachment,
            ImixCategory::InlineMedia,
            ImixCategory::BulkFile,
            ImixCategory::ExternalPointer,
        ];
        let mut total = 0.0;
        for (i, cat) in cats.iter().enumerate() {
            total += self.weights[i] * (cat.mean_bytes() as f64);
        }
        total
    }

    /// Sample an IMIX category given a pseudo-random value u in [0, 1).
    #[must_use]
    pub fn sample_category(&self, u: f64) -> ImixCategory {
        let cats = [
            ImixCategory::CompactCegRow,
            ImixCategory::AgentTraceH3ere,
            ImixCategory::SmallAttachment,
            ImixCategory::InlineMedia,
            ImixCategory::BulkFile,
            ImixCategory::ExternalPointer,
        ];
        let mut accum = 0.0;
        for (i, cat) in cats.iter().enumerate() {
            accum += self.weights[i];
            if u < accum {
                return *cat;
            }
        }
        ImixCategory::CompactCegRow
    }
}

/// The four credible baseline architectures evaluated under identical workloads (FSD §4.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(non_camel_case_types)]
pub enum BaselineArchitecture {
    /// A1: Centralized Origin + Edge CDN + ISP Cache.
    A1_HYPERSCALE_CDN,
    /// A2: Multi-Region Active-Active Cloud + SFU Mesh.
    A2_MULTIREGION_SFU,
    /// A3: Hybrid P2P with Central Bootstrap / Relays.
    A3_HYBRID_P2P,
    /// A4: CEWP Constitutional Substrate (rc17 / Edge v44).
    A4_CEWP_RC17,
}

impl BaselineArchitecture {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::A1_HYPERSCALE_CDN => "A1_HYPERSCALE_CDN",
            Self::A2_MULTIREGION_SFU => "A2_MULTIREGION_SFU",
            Self::A3_HYBRID_P2P => "A3_HYBRID_P2P",
            Self::A4_CEWP_RC17 => "A4_CEWP_RC17",
        }
    }

    /// Whether this architecture uses centralized SFU / server relays for media.
    #[must_use]
    pub const fn has_sfu_role(self) -> bool {
        match self {
            Self::A2_MULTIREGION_SFU => true,
            Self::A1_HYPERSCALE_CDN | Self::A3_HYBRID_P2P | Self::A4_CEWP_RC17 => false,
        }
    }

    /// Whether this architecture enforces constitutional link shaping (L <= 8).
    #[must_use]
    pub const fn enforces_link_shaping(self) -> bool {
        match self {
            Self::A4_CEWP_RC17 => true,
            _ => false,
        }
    }
}

/// The four standard parameter regimes (FSD §4.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParameterRegime {
    Optimistic,
    Expected,
    Pessimistic,
    Adversarial,
}

/// Parameters for a simulation scenario regime.
#[derive(Debug, Clone, PartialEq)]
pub struct RegimeParameters {
    pub regime: ParameterRegime,
    /// Share of traffic staying within self, family, and local community.
    pub locality_share: f64,
    /// Share of home nodes directly reachable (IPv6 + PCP/UPnP).
    pub reach_home_share: f64,
    /// Share of population in dedicated server tier.
    pub server_tier_share: f64,
    /// Share of nodes opting into relay offering.
    pub relay_offering_share: f64,
    /// Probability of mobile node being offline / sleeping.
    pub mobile_offline_churn: f64,
}

impl RegimeParameters {
    /// Return the standardized parameters for the given regime (FSD §4.4 table).
    #[must_use]
    pub fn for_regime(regime: ParameterRegime) -> Self {
        match regime {
            ParameterRegime::Optimistic => Self {
                regime,
                locality_share: 0.85,
                reach_home_share: 0.65,
                server_tier_share: 0.12,
                relay_offering_share: 0.95,
                mobile_offline_churn: 0.05,
            },
            ParameterRegime::Expected => Self {
                regime,
                locality_share: 0.65,
                reach_home_share: 0.40,
                server_tier_share: 0.08,
                relay_offering_share: 0.90,
                mobile_offline_churn: 0.15,
            },
            ParameterRegime::Pessimistic => Self {
                regime,
                locality_share: 0.40,
                reach_home_share: 0.15,
                server_tier_share: 0.04,
                relay_offering_share: 0.50,
                mobile_offline_churn: 0.30,
            },
            ParameterRegime::Adversarial => Self {
                regime,
                locality_share: 0.20,
                reach_home_share: 0.00,
                server_tier_share: 0.02,
                relay_offering_share: 0.00,
                mobile_offline_churn: 0.50,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimensional_rate_correctness() {
        // D = 10,000,000 B/day (10 MB/day)
        // E[B] = 1,000 B/event
        // E[B] * 86,400 = 86,400,000 B*s / event
        // D / (E[B] * 86,400) = 10,000,000 / 86,400,000 = 0.1157407 events / s
        let daily_bytes = 10_000_000.0;
        let mean_payload = 1_000.0;
        let t_secs = 43_200.0; // noon

        let lambda = compute_arrival_rate(daily_bytes, mean_payload, t_secs);
        assert!(lambda > 0.0);

        // Arrival probability over 1 second: P(K >= 1) = 1 - e^(-lambda * 1)
        let prob_1s = timestep_arrival_probability(lambda, 1.0);
        assert!(prob_1s > 0.0 && prob_1s < 1.0);

        // Probability over 0 seconds is 0.0
        assert_eq!(timestep_arrival_probability(lambda, 0.0), 0.0);
    }

    #[test]
    fn diurnal_multiplier_normalization() {
        // Numerical integration of diurnal_multiplier over 24 hours (86,400 seconds)
        let steps = 864; // 100-second steps
        let dt = SECONDS_PER_DAY / (steps as f64);
        let mut sum = 0.0;
        for i in 0..steps {
            let t = (i as f64 + 0.5) * dt;
            sum += diurnal_multiplier(t) * dt;
        }
        let integral_mean = sum / SECONDS_PER_DAY;
        assert!(
            (integral_mean - 1.0).abs() < 1e-9,
            "diurnal multiplier must integrate to approximately 1.0 over 24 hours (got {integral_mean})"
        );

        // the evening is the high, the small hours the low (model.diurnal.v1)
        assert!(diurnal_multiplier(19.5 * 3600.0) > 1.55);
        assert!(diurnal_multiplier(4.0 * 3600.0) < 0.55);
    }

    #[test]
    fn diurnal_multiplier_hits_its_stated_trough_peak_and_hour() {
        let (mut lo, mut hi, mut at) = (f64::MAX, f64::MIN, 0.0);
        for m in 0..(24 * 60) {
            let v = diurnal_multiplier(f64::from(m) * 60.0);
            lo = lo.min(v);
            if v > hi {
                hi = v;
                at = f64::from(m) / 60.0;
            }
        }
        assert!((lo - DIURNAL_TROUGH).abs() < 2e-3, "trough {lo}");
        assert!((hi - DIURNAL_PEAK).abs() < 2e-3, "peak {hi}");
        assert!((at - DIURNAL_PEAK_HOUR).abs() <= 0.25, "peak hour {at}");
        // the waking-hours stretch: no spike, no hole
        for h in 9..=16 {
            let v = diurnal_multiplier(f64::from(h) * 3600.0);
            assert!((0.85..=1.25).contains(&v), "{h}:00 {v}");
        }
    }

    #[test]
    fn imix_distribution_and_sampling() {
        let imix = ImixDistribution::default();
        let mean = imix.mean_payload_bytes();
        assert!(mean > 10_000.0 && mean < 500_000.0);

        // Sample across [0, 1) range
        let cat_first = imix.sample_category(0.1);
        assert_eq!(cat_first, ImixCategory::CompactCegRow);

        let cat_bulk = imix.sample_category(0.965);
        assert_eq!(cat_bulk, ImixCategory::BulkFile);
        assert!(!cat_bulk.allowed_on_shaped_link());

        let cat_agent = ImixCategory::AgentTraceH3ere;
        assert!(cat_agent.allowed_on_shaped_link());
    }

    #[test]
    fn four_baseline_architectures() {
        let a1 = BaselineArchitecture::A1_HYPERSCALE_CDN;
        let a2 = BaselineArchitecture::A2_MULTIREGION_SFU;
        let a3 = BaselineArchitecture::A3_HYBRID_P2P;
        let a4 = BaselineArchitecture::A4_CEWP_RC17;

        assert_eq!(a1.as_str(), "A1_HYPERSCALE_CDN");
        assert!(!a1.has_sfu_role());
        assert!(!a1.enforces_link_shaping());

        assert_eq!(a2.as_str(), "A2_MULTIREGION_SFU");
        assert!(a2.has_sfu_role());

        assert_eq!(a3.as_str(), "A3_HYBRID_P2P");
        assert!(!a3.has_sfu_role());

        assert_eq!(a4.as_str(), "A4_CEWP_RC17");
        assert!(!a4.has_sfu_role()); // CC §5.3.3.2 no SFU role
        assert!(a4.enforces_link_shaping()); // CC §5.4.3 shaped links
    }

    #[test]
    fn four_parameter_regimes() {
        let opt = RegimeParameters::for_regime(ParameterRegime::Optimistic);
        assert_eq!(opt.locality_share, 0.85);
        assert_eq!(opt.reach_home_share, 0.65);

        let exp = RegimeParameters::for_regime(ParameterRegime::Expected);
        assert_eq!(exp.locality_share, 0.65);
        assert_eq!(exp.reach_home_share, 0.40);

        let pess = RegimeParameters::for_regime(ParameterRegime::Pessimistic);
        assert_eq!(pess.locality_share, 0.40);
        assert_eq!(pess.reach_home_share, 0.15);

        let adv = RegimeParameters::for_regime(ParameterRegime::Adversarial);
        assert_eq!(adv.locality_share, 0.20);
        assert_eq!(adv.reach_home_share, 0.00); // 100% hard CGNAT
    }
}
