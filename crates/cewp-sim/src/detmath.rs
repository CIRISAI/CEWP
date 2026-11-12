//! Libm-free functions for anything that decides a layout: built from IEEE-754 correctly rounded
//! operations (+ - * / sqrt, fmod) and exact bit manipulation only, so native and wasm32 builds give
//! the same bits. The platform's `cos` and `powf` differ by an ulp between glibc and the libm port
//! wasm32 uses, and an ulp in a leaf's width flipped building placement on 515 of 16,426 real leaves
//! (twin/hhsize crosscheck). Accuracy is about 1e-15 relative; agreement is exact.

use std::f64::consts::{FRAC_PI_2, LN_2, PI, TAU};

fn sin_series(x: f64) -> f64 {
    // x in [-pi/2, pi/2]; Taylor to x^19 (error below 1e-15 there)
    let x2 = x * x;
    let (mut term, mut sum, mut k) = (x, x, 1.0);
    for _ in 0..9 {
        term = -term * x2 / ((k + 1.0) * (k + 2.0));
        sum += term;
        k += 2.0;
    }
    sum
}

/// sin(x), x in radians.
#[must_use]
pub fn sin(x: f64) -> f64 {
    let mut y = x % TAU;
    if y > PI {
        y -= TAU;
    } else if y < -PI {
        y += TAU;
    }
    if y > FRAC_PI_2 {
        y = PI - y;
    } else if y < -FRAC_PI_2 {
        y = -PI - y;
    }
    sin_series(y)
}

/// cos(x), x in radians.
#[must_use]
pub fn cos(x: f64) -> f64 {
    sin(x + FRAC_PI_2)
}

/// cos of an angle in degrees.
#[must_use]
pub fn cos_deg(deg: f64) -> f64 {
    cos(deg * (PI / 180.0))
}

/// ln(x) for x > 0 (0 and below give -inf).
#[must_use]
pub fn ln(x: f64) -> f64 {
    if x <= 0.0 || x.is_nan() {
        return f64::NEG_INFINITY;
    }
    // x = m * 2^e, m in [1, 2), exactly from the bits (x normal here; subnormals scaled first)
    let (x, extra) = if x < f64::MIN_POSITIVE { (x * 2f64.powi(64), -64) } else { (x, 0) };
    let bits = x.to_bits();
    let e = ((bits >> 52) & 0x7FF) as i64 - 1023 + extra;
    let mut m = f64::from_bits((bits & 0x000F_FFFF_FFFF_FFFF) | 0x3FF0_0000_0000_0000);
    let mut e = e as f64;
    if m > std::f64::consts::SQRT_2 {
        m /= 2.0;
        e += 1.0;
    }
    // ln m = 2 atanh(t), t = (m - 1) / (m + 1), |t| <= 0.172
    let t = (m - 1.0) / (m + 1.0);
    let t2 = t * t;
    let (mut p, mut s) = (t, t);
    for k in 1..20 {
        p *= t2;
        s += p / (2 * k + 1) as f64;
    }
    2.0 * s + e * LN_2
}

/// e^x.
#[must_use]
pub fn exp(x: f64) -> f64 {
    if x.is_nan() {
        return x;
    }
    if x > 709.0 {
        return f64::INFINITY;
    }
    if x < -745.0 {
        return 0.0;
    }
    let k = (x / LN_2).round();
    let r = x - k * LN_2;
    let (mut term, mut sum) = (1.0, 1.0);
    for n in 1..24 {
        term *= r / n as f64;
        sum += term;
    }
    // multiply by 2^k exactly, in two steps so neither factor overflows
    let k = k as i64;
    let half = k / 2;
    let p2 = |j: i64| f64::from_bits(((j + 1023) as u64) << 52);
    sum * p2(half) * p2(k - half)
}

/// x^y for x > 0.
#[must_use]
pub fn pow(x: f64, y: f64) -> f64 {
    exp(y * ln(x))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn close_to_the_platform_everywhere_the_layout_reads() {
        let mut worst = 0.0_f64;
        for i in -9000..=9000 {
            let d = f64::from(i) / 100.0;
            worst = worst.max((cos_deg(d) - d.to_radians().cos()).abs());
        }
        println!("cos_deg worst absolute error over -90..90 degrees: {worst:e}");
        assert!(worst < 5e-16, "cos_deg worst {worst:e}");
        for i in 1..=4000 {
            let x = f64::from(i) * 7.3;
            assert!((pow(x, 0.4) / x.powf(0.4) - 1.0).abs() < 4e-15, "pow {x}");
            assert!((ln(x) - x.ln()).abs() < 1e-14 * x.ln().abs().max(1.0), "ln {x}");
        }
        assert_eq!(exp(0.0), 1.0);
        assert!((exp(1.0) - std::f64::consts::E).abs() < 1e-15);
    }
}
