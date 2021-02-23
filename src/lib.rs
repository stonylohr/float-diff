mod diff_part_summary;
mod diff_summary_f64;
mod log_histogram;
mod util;

pub mod diff;
pub use crate::diff_summary_f64::DiffSummary as DiffSummary64;

// PLEASE NOTE that this macro is more likely than
// average to experience breaking changes or
// to be dropped in future releases.
// Log a single comparison, using logic similar to
// DiffSummary's handling of sets of comparisons.
// A call to this function can can be thought of as a
// more elaborate variation on the approx crate's:
// assert_approx_eq!(x, y, allow_diff)
#[macro_export]
macro_rules! log_assert_approx_eq {
    ($name: expr, $x: expr, $y: expr, $allow_diff: expr, $allow_sign_change: expr, $calc_diff: expr) => {
        let (diff, sign_change) = (*($calc_diff))($x, $y);
        println!(
            "{}: {}{:e} vs {}{:e} diff {:e}, sign diff {}",
            $name,
            util::help_sign($x),
            $x,
            util::help_sign($y),
            $y,
            diff,
            sign_change
        );
        assert!(
            diff <= $allow_diff,
            "assert failed {}: {}{:e} vs {}{:e} diff abs {:e} outside inclusive {:e}",
            $name,
            util::help_sign($x),
            $x,
            util::help_sign($y),
            $y,
            diff,
            $allow_diff
        );
        assert!($allow_sign_change || !sign_change,
            "assert failed {}: {}{:e} vs {}{:e} sign difference disallowed.",
            $name,
            util::help_sign($x),
            $x,
            util::help_sign($y),
            $y,
    );
    }
}
