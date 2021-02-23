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

// When displaying f64, we want to make sure to display the "-" for values like
// -0.0, -f64::NAN, and f64::NEG_INFINITY. We also want to display concise
// values, which calls for using scientific notation in cases like 5e-200
// (we don't care as much about representation of more moderate values).
// As of Rust 1.50, I do not see any combination of format specifiers that 
// yields this combination of qualities, since the debug specifier seems to be
// required to get a reliably - sign, but the debug specifier doesn't seem to
// be compatible with the exponent specifiers (e, E).
// Here are some related Rust issues that might be worth watching:
//   https://github.com/rust-lang/rfcs/issues/1074
//   https://github.com/rust-lang/rfcs/issues/1075
//   https://github.com/rust-lang/rust/issues/20596
//   https://github.com/rust-lang/rust/issues/24556
//   https://github.com/rust-lang/rust/issues/24623
//   https://github.com/rust-lang/rust/issues/24624
// For now, here's a lame work-around.
pub fn help_sign(x: f64) -> String {
    if (x == 0.0 || x.is_nan()) && x.is_sign_negative() {
        "-".to_string()
    } else {
        "".to_string()
    }
}
