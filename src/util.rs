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

// Return the lesser of the absolute and relative difference between two values.
// If both values are nan or same-sign infinite, consider the difference to be 0.
// Can be helpful in cases where there is a wide range of expected values,
// such that it's difficult to have a low absolute difference for large expected
// values and a low relative difference for near-zero expected values.
pub fn calc_diff_lesser(x: f64, y: f64) -> (f64, bool) {
    let (diff_abs, sign_change) = calc_diff_abs(x, y);
    let diff = if diff_abs == 0.0 || diff_abs.is_nan() || diff_abs.is_infinite() {
        diff_abs
    } else {
        let diff_rel = 2.0 * diff_abs / (x.abs() + y.abs());
        f64::min(diff_abs, diff_rel)
    };
    (diff, sign_change)
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
