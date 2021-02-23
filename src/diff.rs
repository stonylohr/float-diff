extern crate float_cmp;

use float_cmp::Ulps;

// Return true if diff a is "worse" than diff b.
// NAN is worse than INFINITY is worse than anything finite.
// All diffs are required to be positive
// (including positive zero and positive nan).
pub fn is_diff_worse(a: f64, b: f64) -> bool {
    assert!(a.is_sign_positive() && b.is_sign_positive());
    (a.is_nan() && !b.is_nan()) || a > b
}

// Return the absolute difference between two values.
// If both values are nan or same-sign infinite, consider the difference to be 0.
pub fn diff_abs(x: f64, y: f64) -> (f64, bool) {
    let diff = if x.is_nan() && y.is_nan() {
        0f64
    } else if x.is_infinite() && y.is_infinite() {
        if x.is_sign_negative() == y.is_sign_negative() { 0f64 } else { f64::INFINITY }
    } else {
        (x - y).abs()
    };
    // For the sign change check use is_sign_negative rather than "< 0.0",
    // to allow (NAN vs NAN), but not (0.0 vs -0.0) or (NAN vs -NAN).
    let sign_change = x.is_sign_negative() != y.is_sign_negative();
    (diff, sign_change)
}

// Return the relative difference between two values.
// If both values are nan or same-sign infinite, consider the difference to be 0.
pub fn diff_rel(x: f64, y: f64) -> (f64, bool) {
    let (mut diff, sign_change) = diff_abs(x, y);
    if diff != 0.0 { // and implicitly not nan
        diff *= 2.0 / (x.abs() + y.abs());
    }
    (diff, sign_change)
}

// Return the lesser of the absolute and relative difference between two values.
// If both values are nan or same-sign infinite, consider the difference to be 0.
// Can be helpful in cases where there is a wide range of expected values,
// such that it's difficult to have a low absolute difference for large expected
// values and a low relative difference for near-zero expected values.
pub fn diff_lesser(x: f64, y: f64) -> (f64, bool) {
    let (mut diff, sign_change) = diff_abs(x, y);
    if diff != 0.0 && !diff.is_infinite() { // and implicitly not nan
        let sum_abs = x.abs() + y.abs();
        if sum_abs > 2.0 {
            // use relative difference
            diff *= 2.0 / sum_abs;
        }
    }
    (diff, sign_change)
}

// Calculate difference in ULPs (units in the last place or unit of least precision),
// with special handling for a few cases.
// Note that this handling may not be appropriate for all cases where ULPs are desired.
// While one would normally expect an ULPs-based comparison to return an integer value,
// this uses floating point, to match its sibling function signatures.
pub fn diff_ulps(x: f64, y: f64) -> (f64, bool) {
    let ulps = if x.is_nan() != y.is_nan() {
        f64::NAN
    } else if x.is_nan() {
        // For -NAN vs NAN, indicate a sign change, but otherwise treat as equal.
        0.0
    } else if x.is_finite() != y.is_finite() {
        // For -INFINITY vs INFINITY, go ahead and return a huge ulps difference.
        f64::INFINITY
    } else {
        // Cast to f64 before abs to avoid risk of overflow in extreme cases.
        (x.ulps(&y) as f64).abs()
    };
    (ulps, x.is_sign_negative() != y.is_sign_negative())
}

// Return the absolute difference between two values using a cyclic range,
// for example angles using a preferred range of [0, 360].
// Any range enforcement adjustments are reported as a sign change.
// For example (0, 1) is not reported as a sign change for the range [0, 360],
// but all of the following are: (1, -1) (359, 361) (0, 361) (720, 721)
pub fn diff_cyclic(x: f64, y: f64, range_min: f64, range_max: f64) -> (f64, bool) {
    assert!(range_min < range_max, "range_min must be less than range_max");
    assert!(range_min <= 0.0 && 0.0 <= range_max, "0.0 must fall within [range_min, range_max]");
    let xmod = cyclic_range(x, range_min, range_max);
    let ymod = cyclic_range(y, range_min, range_max);
    let diff1 = if (xmod.is_nan() && !x.is_nan()) || (ymod.is_nan() && !y.is_nan()) {
        // This can happen if x or y is infinite, and possibly other degenerate cases.
        (f64::NAN, true)
    } else {
        diff_abs(xmod, ymod)
    };
    let diff2 = if xmod < ymod {
        diff_abs(xmod + range_max - range_min, ymod)
    } else if xmod > ymod {
        diff_abs(xmod, ymod + range_max - range_min)
    } else {
        diff1
    };
    if diff2.0 < diff1.0 {
        (diff2.0, true)
    } else {
        (diff1.0, x != xmod || y != ymod || diff1.1)
    }
}

// Adjust a value to fall within a specified cyclic range.
fn cyclic_range(x: f64, range_min: f64, range_max: f64) -> f64 {
    let span = range_max - range_min;
    let xmod = x % span;
    if xmod < range_min {
        xmod + span
    } else if xmod > range_max {
        xmod - span
    } else {
        xmod
    }
}

#[cfg(test)]
mod tests {
    use super::{diff_abs, diff_cyclic, diff_lesser, diff_rel, diff_ulps};

    #[test]
    fn test_abs() {
        // Values chosen to be cleanly representable as exact f64
        assert_eq!(diff_abs(0.0, 0.5), (0.5, false));
        assert_eq!(diff_abs(10.0, 10.5), (0.5, false));
        assert_eq!(diff_abs(-0.25, 0.25), (0.5, true));
        assert_eq!(diff_abs(0.0, 0.0), (0.0, false));
        assert_eq!(diff_abs(-0.0, 0.0), (0.0, true));
        assert_eq!(diff_abs(f64::NAN, f64::NAN), (0.0, false));
        assert_eq!(diff_abs(f64::NAN, -f64::NAN), (0.0, true));
        let diff = diff_abs(f64::INFINITY, f64::NAN);
        assert!(diff.0.is_nan() && !diff.1);
        assert_eq!(diff_abs(f64::INFINITY, f64::INFINITY), (0.0, false));
        assert_eq!(diff_abs(f64::INFINITY, f64::NEG_INFINITY), (f64::INFINITY, true));
    }

    #[test]
    fn test_cyclic() {
        // Values chosen to be cleanly representable as exact f64
        assert_eq!(diff_cyclic(0.0, 0.5, -180.0, 180.0), (0.5, false));
        assert_eq!(diff_cyclic(10.0, 10.5, -180.0, 180.0), (0.5, false));
        assert_eq!(diff_cyclic(-0.25, 0.25, -180.0, 180.0), (0.5, true));
        assert_eq!(diff_cyclic(0.0, 0.0, -180.0, 180.0), (0.0, false));
        assert_eq!(diff_cyclic(-0.0, 0.0, -180.0, 180.0), (0.0, true));
        assert_eq!(diff_cyclic(f64::NAN, f64::NAN, -180.0, 180.0), (0.0, true));
        assert_eq!(diff_cyclic(f64::NAN, -f64::NAN, -180.0, 180.0), (0.0, true));
        let diff = diff_cyclic(f64::INFINITY, f64::NAN, -180.0, 180.0);
        assert!(diff.0.is_nan() && diff.1);
        let diff = diff_cyclic(f64::INFINITY, f64::INFINITY, -180.0, 180.0);
        assert!(diff.0.is_nan() && diff.1);
        let diff = diff_cyclic(f64::INFINITY, f64::NEG_INFINITY, -180.0, 180.0);
        assert!(diff.0.is_nan() && diff.1);
        assert_eq!(diff_cyclic(-180.0, 180.0, -180.0, 180.0), (0.0, true));
        assert_eq!(diff_cyclic(-179.0, 179.0, -180.0, 180.0), (2.0, true));
        assert_eq!(diff_cyclic(-179.0, -179.0, -180.0, 180.0), (0.0, false));
        assert_eq!(diff_cyclic(181.0, 181.0, -180.0, 180.0), (0.0, true));
        assert_eq!(diff_cyclic(0.0, 721.0, -180.0, 180.0), (1.0, true));
    }

    #[test]
    fn test_lesser() {
        // Values chosen to be cleanly representable as exact f64
        assert_eq!(diff_lesser(0.0, 0.5), (0.5, false));
        assert_eq!(diff_lesser(10.0, 10.5), (1.0 / 20.5, false));
        assert_eq!(diff_lesser(-0.25, 0.25), (0.5, true));
        assert_eq!(diff_lesser(0.0, 0.0), (0.0, false));
        assert_eq!(diff_lesser(-0.0, 0.0), (0.0, true));
        assert_eq!(diff_lesser(f64::NAN, f64::NAN), (0.0, false));
        assert_eq!(diff_lesser(f64::NAN, -f64::NAN), (0.0, true));
        let diff = diff_lesser(f64::INFINITY, f64::NAN);
        assert!(diff.0.is_nan() && !diff.1);
        assert_eq!(diff_lesser(f64::INFINITY, f64::INFINITY), (0.0, false));
        assert_eq!(diff_lesser(f64::INFINITY, f64::NEG_INFINITY), (f64::INFINITY, true));
    }

    #[test]
    fn test_rel() {
        // Values chosen to be cleanly representable as exact f64
        assert_eq!(diff_rel(0.0, 0.5), (2.0, false));
        assert_eq!(diff_rel(10.0, 10.5), (1.0 / 20.5, false));
        assert_eq!(diff_rel(-0.25, 0.25), (2.0, true));
        assert_eq!(diff_rel(0.0, 0.0), (0.0, false));
        assert_eq!(diff_rel(-0.0, 0.0), (0.0, true));
        assert_eq!(diff_rel(f64::NAN, f64::NAN), (0.0, false));
        assert_eq!(diff_rel(f64::NAN, -f64::NAN), (0.0, true));
        let diff = diff_rel(f64::INFINITY, f64::NAN);
        assert!(diff.0.is_nan() && !diff.1);
        assert_eq!(diff_rel(f64::INFINITY, f64::INFINITY), (0.0, false));
        let diff = diff_rel(f64::INFINITY, f64::NEG_INFINITY);
        assert!(diff.0.is_nan() && diff.1);
    }

    #[test]
    fn test_ulps() {
        assert_eq!(diff_ulps(0.0, 0.0), (0.0, false));
        assert_eq!(diff_ulps(1.0, 1.0 + f64::EPSILON), (1.0, false));
        assert!(f64::is_nan(diff_ulps(1.0, f64::NAN).0));
        assert!(f64::is_infinite(diff_ulps(f64::MAX, f64::INFINITY).0));
    }

}