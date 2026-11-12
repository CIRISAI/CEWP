//! Which country a point is in: docs/geo/countries.bin (Natural Earth 1:50m admin-0, public domain,
//! simplified; tools/geo/build_countries.py), compiled in and decoded once. Used only to choose a
//! statistical reference's envelope (CIRISHolon tables name a country); never drawn as a border.
//! The same integer test as docs/geo/countries.js, so the page and native agree.

use std::sync::OnceLock;

struct Ring {
    iso: u16,
    p: Vec<(i64, i64)>,
    box_: (i64, i64, i64, i64),
}

static BIN: &[u8] = include_bytes!("../../../docs/geo/countries.bin");
static RINGS: OnceLock<Vec<Ring>> = OnceLock::new();

fn rings() -> &'static [Ring] {
    RINGS.get_or_init(|| {
        let b = BIN;
        let u32_at = |i: usize| u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
        let i32_at = |i: usize| i32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
        let i16_at = |i: usize| i16::from_le_bytes([b[i], b[i + 1]]);
        assert_eq!(&b[0..3], b"CC1");
        let n = u32_at(4) as usize;
        let mut i = 8;
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            let iso = u16::from_le_bytes([b[i], b[i + 1]]);
            let np = u32_at(i + 2) as usize;
            let (mut la, mut lo) = (i64::from(i32_at(i + 6)), i64::from(i32_at(i + 10)));
            i += 14;
            let mut p = Vec::with_capacity(np);
            p.push((la, lo));
            let mut bx = (la, lo, la, lo);
            for _ in 1..np {
                let s = b[i] as i8;
                if s == -128 {
                    la += i64::from(i16_at(i + 1));
                    lo += i64::from(i16_at(i + 3));
                    i += 5;
                } else {
                    la += i64::from(s);
                    lo += i64::from(b[i + 1] as i8);
                    i += 2;
                }
                p.push((la, lo));
                bx = (bx.0.min(la), bx.1.min(lo), bx.2.max(la), bx.3.max(lo));
            }
            out.push(Ring { iso, p, box_: bx });
        }
        out
    })
}

fn q(lat: f64, lon: f64) -> (i64, i64) {
    ((lat * 100.0).round() as i64, ((((lon + 540.0) % 360.0) - 180.0) * 100.0).round() as i64)
}

/// ISO 3166-1 numeric of the country holding (lat, lon); 0 for an entity with no ISO code; None at sea.
#[must_use]
pub fn country_at(lat: f64, lon: f64) -> Option<u16> {
    let (y, x) = q(lat, lon);
    let mut odd: Vec<(u16, bool)> = Vec::new();
    for r in rings() {
        let b = r.box_;
        if y < b.0 || y > b.2 || x < b.1 || x > b.3 {
            continue;
        }
        let n = r.p.len();
        let mut inside = false;
        let mut c = n - 1;
        for a in 0..n {
            let ((ya, xa), (yc, xc)) = (r.p[a], r.p[c]);
            // x < (xc - xa) * (y - ya) / (yc - ya) + xa, in exact integers (rational compare)
            if (ya > y) != (yc > y) {
                let num = (xc - xa) * (y - ya);
                let den = yc - ya;
                let lhs = (x - xa) * den;
                if if den > 0 { lhs < num } else { lhs > num } {
                    inside = !inside;
                }
            }
            c = a;
        }
        if inside {
            match odd.iter_mut().find(|e| e.0 == r.iso) {
                Some(e) => e.1 = !e.1,
                None => odd.push((r.iso, true)),
            }
        }
    }
    odd.into_iter().find(|e| e.1).map(|e| e.0)
}

/// [`country_at`], else the country whose edge is nearest within `max_deg` (the mask is simplified at
/// 0.05 degree, so a coastal city's centre can fall just offshore); open sea stays None.
#[must_use]
pub fn country_near(lat: f64, lon: f64, max_deg: f64) -> Option<u16> {
    if let Some(c) = country_at(lat, lon) {
        return Some(c);
    }
    let (y, x) = q(lat, lon);
    let lim = (max_deg * 100.0).round();
    let cl = crate::detmath::cos_deg(lat).max(0.05);
    let (yf, xf) = (y as f64, x as f64 * cl);
    let mut best = (lim * lim, None);
    for r in rings() {
        let b = r.box_;
        if (y as f64) < b.0 as f64 - lim || (y as f64) > b.2 as f64 + lim || (x as f64) < b.1 as f64 - lim / cl || (x as f64) > b.3 as f64 + lim / cl {
            continue;
        }
        let n = r.p.len();
        let mut c = n - 1;
        for a in 0..n {
            let (ay, ax) = (r.p[a].0 as f64, r.p[a].1 as f64 * cl);
            let (cy, cx) = (r.p[c].0 as f64, r.p[c].1 as f64 * cl);
            let (dx, dy) = (cx - ax, cy - ay);
            let l = dx * dx + dy * dy;
            let t = if l > 0.0 { (((xf - ax) * dx + (yf - ay) * dy) / l).clamp(0.0, 1.0) } else { 0.0 };
            let (ex, ey) = (ax + t * dx - xf, ay + t * dy - yf);
            let d = ex * ex + ey * ey;
            if d < best.0 {
                best = (d, Some(r.iso));
            }
            c = a;
        }
    }
    best.1
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn the_same_cases_as_the_page() {
        for (name, la, lo, iso) in [
            ("Basel", 47.5596, 7.5886, 756), ("Loerrach", 47.6156, 7.6617, 276), ("El Paso airport", 31.8072, -106.3776, 840),
            ("Ciudad Juarez", 31.6904, -106.4245, 484), ("Kaliningrad", 54.7104, 20.4522, 643), ("Reykjavik", 64.1466, -21.9426, 352),
            ("Maseru", -29.3151, 27.4869, 426), ("Berlin", 52.52, 13.405, 276), ("New York", 40.7128, -74.006, 840), ("Honolulu", 21.3069, -157.8583, 840),
        ] {
            assert_eq!(country_near(la, lo, 0.15), Some(iso), "{name}");
        }
        assert_eq!(country_near(0.0, -30.0, 0.15), None);
        assert_eq!(country_at(47.5596, 7.5886), Some(756));
    }
}
