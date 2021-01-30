// Return the absolute difference between two values.
// If both values are nan or same-sign infinite, consider the difference to be 0.
pub fn calc_diff_abs(x: f64, y: f64) -> (f64, bool) {
    let diff_abs = if x.is_nan() && y.is_nan() {
        0f64
    } else if x.is_infinite() && y.is_infinite() {
        if x.is_sign_negative() == y.is_sign_negative() { 0f64 } else { f64::INFINITY }
    } else {
        (x - y).abs()
    };
    // For the sign change check use is_sign_negative rather than "< 0.0",
    // to allow (NAN vs NAN), but not (0.0 vs -0.0) or (NAN vs -NAN).
    let sign_change = x.is_sign_negative() != y.is_sign_negative();
    (diff_abs, sign_change)
}

// Return the relative difference between two values.
// If both values are nan or same-sign infinite, consider the difference to be 0.
pub fn calc_diff_rel(x: f64, y: f64) -> (f64, bool) {
    let (mut diff, sign_change) = calc_diff_abs(x, y);
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
pub fn calc_diff_lesser(x: f64, y: f64) -> (f64, bool) {
    let (mut diff, sign_change) = calc_diff_abs(x, y);
    if diff != 0.0 && !diff.is_infinite() { // and implicitly not nan
        let sum_abs = x.abs() + y.abs();
        if sum_abs > 2.0 {
            // use relative difference
            diff *= 2.0 / sum_abs;
        }
    }
    (diff, sign_change)
}

// Return the absolute difference between two values using a cyclic range,
// for example angles using a preferred range of [0, 360].
// Any range enforcement adjustments are reported as a sign change.
// For example (0, 1) is not reported as a sign change for the range [0, 360],
// but all of the following are: (1, -1) (359, 361) (0, 361) (720, 721)
pub fn calc_diff_cyclic(x: f64, y: f64, range_min: f64, range_max: f64) -> (f64, bool) {
    assert!(range_min < range_max, "range_min must be less than range_max");
    assert!(range_min <= 0.0 && 0.0 <= range_max, "0.0 must fall within [range_min, range_max]");
    let xmod = cyclic_range(x, range_min, range_max);
    let ymod = cyclic_range(y, range_min, range_max);
    let diff1 = if (xmod.is_nan() && !x.is_nan()) || (ymod.is_nan() && !y.is_nan()) {
        // This can happen if x or y is infinite, and possibly other degenerate cases.
        (f64::NAN, true)
    } else {
        calc_diff_abs(xmod, ymod)
    };
    let diff2 = if xmod < ymod {
        calc_diff_abs(xmod + range_max - range_min, ymod)
    } else if xmod > ymod {
        calc_diff_abs(xmod, ymod + range_max - range_min)
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

// Return true if diff a is "worse" than diff b.
// NAN is worse than INFINITY is worse than anything finite.
// All diffs are required to be positive
// (including positive zero and positive nan).
pub fn is_diff_worse(a: f64, b: f64) -> bool {
    assert!(a.is_sign_positive() && b.is_sign_positive());
    (a.is_nan() && !b.is_nan()) || a > b
}

// Round a value for use in LogHistogram display.
// Never round to 0 or 100. Only accept those values naturally.
pub fn to_percent(num_part: usize, num_all: usize) -> usize {
    let percent = 100f64 * num_part as f64 / num_all as f64;
    let rounded = if percent < 1.0 && num_part != 0 {
        1
    } else if percent > 99.0 && num_part != num_all {
        99
    } else {
        percent.round() as usize
    };
    rounded
}

// When displaying f64, stable Rust declines to display the sign for -0 or -nan,
// as of 2021/01/10. It looks like a fix for this is on the way:
//     https://github.com/rust-lang/rust/issues/20596
// Here's a work-around in the meantime. Its mechanics aren't great, but it's temporary.
pub fn help_sign(x: f64) -> String {
    if (x == 0.0 || x.is_nan()) && x.is_sign_negative() {
        "-".to_string()
    } else {
        "".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{calc_diff_abs, calc_diff_rel, calc_diff_lesser, calc_diff_cyclic};

    #[test]
    fn test_abs() {
        // Values chosen to be cleanly representable as exact f64
        assert_eq!(calc_diff_abs(0.0, 0.5), (0.5, false));
        assert_eq!(calc_diff_abs(10.0, 10.5), (0.5, false));
        assert_eq!(calc_diff_abs(-0.25, 0.25), (0.5, true));
        assert_eq!(calc_diff_abs(0.0, 0.0), (0.0, false));
        assert_eq!(calc_diff_abs(-0.0, 0.0), (0.0, true));
        assert_eq!(calc_diff_abs(f64::NAN, f64::NAN), (0.0, false));
        assert_eq!(calc_diff_abs(f64::NAN, -f64::NAN), (0.0, true));
        let diff = calc_diff_abs(f64::INFINITY, f64::NAN);
        assert!(diff.0.is_nan() && !diff.1);
        assert_eq!(calc_diff_abs(f64::INFINITY, f64::INFINITY), (0.0, false));
        assert_eq!(calc_diff_abs(f64::INFINITY, f64::NEG_INFINITY), (f64::INFINITY, true));
    }

    #[test]
    fn test_rel() {
        // Values chosen to be cleanly representable as exact f64
        assert_eq!(calc_diff_rel(0.0, 0.5), (2.0, false));
        assert_eq!(calc_diff_rel(10.0, 10.5), (1.0 / 20.5, false));
        assert_eq!(calc_diff_rel(-0.25, 0.25), (2.0, true));
        assert_eq!(calc_diff_rel(0.0, 0.0), (0.0, false));
        assert_eq!(calc_diff_rel(-0.0, 0.0), (0.0, true));
        assert_eq!(calc_diff_rel(f64::NAN, f64::NAN), (0.0, false));
        assert_eq!(calc_diff_rel(f64::NAN, -f64::NAN), (0.0, true));
        let diff = calc_diff_rel(f64::INFINITY, f64::NAN);
        assert!(diff.0.is_nan() && !diff.1);
        assert_eq!(calc_diff_rel(f64::INFINITY, f64::INFINITY), (0.0, false));
        let diff = calc_diff_rel(f64::INFINITY, f64::NEG_INFINITY);
        assert!(diff.0.is_nan() && diff.1);
    }

    #[test]
    fn test_lesser() {
        // Values chosen to be cleanly representable as exact f64
        assert_eq!(calc_diff_lesser(0.0, 0.5), (0.5, false));
        assert_eq!(calc_diff_lesser(10.0, 10.5), (1.0 / 20.5, false));
        assert_eq!(calc_diff_lesser(-0.25, 0.25), (0.5, true));
        assert_eq!(calc_diff_lesser(0.0, 0.0), (0.0, false));
        assert_eq!(calc_diff_lesser(-0.0, 0.0), (0.0, true));
        assert_eq!(calc_diff_lesser(f64::NAN, f64::NAN), (0.0, false));
        assert_eq!(calc_diff_lesser(f64::NAN, -f64::NAN), (0.0, true));
        let diff = calc_diff_lesser(f64::INFINITY, f64::NAN);
        assert!(diff.0.is_nan() && !diff.1);
        assert_eq!(calc_diff_lesser(f64::INFINITY, f64::INFINITY), (0.0, false));
        assert_eq!(calc_diff_lesser(f64::INFINITY, f64::NEG_INFINITY), (f64::INFINITY, true));
    }

    #[test]
    fn test_cyclic() {
        // Values chosen to be cleanly representable as exact f64
        assert_eq!(calc_diff_cyclic(0.0, 0.5, -180.0, 180.0), (0.5, false));
        assert_eq!(calc_diff_cyclic(10.0, 10.5, -180.0, 180.0), (0.5, false));
        assert_eq!(calc_diff_cyclic(-0.25, 0.25, -180.0, 180.0), (0.5, true));
        assert_eq!(calc_diff_cyclic(0.0, 0.0, -180.0, 180.0), (0.0, false));
        assert_eq!(calc_diff_cyclic(-0.0, 0.0, -180.0, 180.0), (0.0, true));
        assert_eq!(calc_diff_cyclic(f64::NAN, f64::NAN, -180.0, 180.0), (0.0, true));
        assert_eq!(calc_diff_cyclic(f64::NAN, -f64::NAN, -180.0, 180.0), (0.0, true));
        let diff = calc_diff_cyclic(f64::INFINITY, f64::NAN, -180.0, 180.0);
        assert!(diff.0.is_nan() && diff.1);
        let diff = calc_diff_cyclic(f64::INFINITY, f64::INFINITY, -180.0, 180.0);
        assert!(diff.0.is_nan() && diff.1);
        let diff = calc_diff_cyclic(f64::INFINITY, f64::NEG_INFINITY, -180.0, 180.0);
        assert!(diff.0.is_nan() && diff.1);
        assert_eq!(calc_diff_cyclic(-180.0, 180.0, -180.0, 180.0), (0.0, true));
        assert_eq!(calc_diff_cyclic(-179.0, 179.0, -180.0, 180.0), (2.0, true));
        assert_eq!(calc_diff_cyclic(-179.0, -179.0, -180.0, 180.0), (0.0, false));
        assert_eq!(calc_diff_cyclic(181.0, 181.0, -180.0, 180.0), (0.0, true));
        assert_eq!(calc_diff_cyclic(0.0, 721.0, -180.0, 180.0), (1.0, true));
    }

}