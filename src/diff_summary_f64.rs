use std::fmt::Display;
use crate::diff_part_summary::DiffPartSummary;
use crate::log_histogram::LogHistogram;
use crate::util;

// An object for tracking a series of test results for a the same measurement type,
// recording how they compare to the expected value for the test case, and 
// reporting out those findings.
pub struct DiffSummary<'a>
{
    // The name of this summary.
    pub name: &'a str,

    // The maximum difference found so far in data passed to this summary.
    diff: f64,

    // The maximum allowable difference for this summary to consider an item successful.
    allow_diff: f64,

    // Indicates whether the summary should allow sign changes when deciding whether an item is successful.
    allow_sign: bool,

    // The total number of items added to this summary.
    num_total: usize,

    // The number of items that have failed based on difference (ignoring sign change).
    num_diff_fail: usize,

    // Count of items with non-zero diffs, and information about the item with the worst diff.
    summary_diff: DiffPartSummary,

    // Count of items with sign changes, and information about the first such item.
    summary_sign: DiffPartSummary,

    // A partially logarithmic breakdown of differences.
    histo: LogHistogram,

    // The function to use when calculating the difference and sign change status of a value pair.
    calc_diff: &'a dyn Fn(f64, f64) -> (f64, bool),
}

impl<'a> DiffSummary<'a> {
    pub fn new(name: &'a str, allow_diff: f64, allow_sign: bool, bucket_count: usize, calc_diff: &'a dyn Fn(f64, f64) -> (f64, bool)) -> Self {
        DiffSummary {
            name: name,
            allow_diff: allow_diff,
            allow_sign: allow_sign,
            diff: 0.0,
            num_total: 0,
            num_diff_fail: 0,
            summary_diff: DiffPartSummary::new(),
            summary_sign: DiffPartSummary::new(),
            histo: LogHistogram::new(bucket_count),
            calc_diff: calc_diff,
        }
    }

    // Create a vector of DiffSummary based on a slice of tuples with the form:
    // (name, allow_diff, allow_sign, calc_diff)
    pub fn new_vec(bucket_count: usize, infos: &'a [(&str, f64, bool, &'a dyn Fn(f64, f64) -> (f64, bool))]) -> Vec<Self> {
        infos.iter().map(|(name, allow_diff, allow_sign, calc_diff)| {
            DiffSummary {
                name: name,
                allow_diff: *allow_diff,
                allow_sign: *allow_sign,
                diff: 0.0,
                num_total: 0,
                num_diff_fail: 0,
                summary_diff: DiffPartSummary::new(),
                summary_sign: DiffPartSummary::new(),
                histo: LogHistogram::new(bucket_count),
                calc_diff: calc_diff,
            }
        }).collect()
    }

    // Given x and y, calculate their difference and sign change status,
    // then check whether any of those values is the worst seen so far
    // for comparable operations. If it is, record the iteration
    // information and the new worst difference.
    // For purposes of deciding "worst", infinity is worse than any
    // finite number, and nan is worse than infinity.
    pub fn add(&mut self, x: f64, y: f64, index: usize) {
        self.num_total += 1;
        let (diff, sign_change) = (*self.calc_diff)(x, y);
        let is_diff_worst = util::is_diff_worse(diff, self.diff);
        // Funky negation on next line is intentional, to get desired nan behavior.
        if !(diff == 0.0) {
            self.summary_diff.add(x, y, index, is_diff_worst);
            if is_diff_worst {
                self.diff = diff;
            }
            // Funky negation on next line is intentional, to get desired nan behavior.
            if !(diff <= self.allow_diff) {
                self.num_diff_fail += 1;
            }
        }
        // For the sign change check, allow (NAN vs NAN), but not (0.0 vs -0.0) or (NAN vs -NAN).
        if sign_change {
            self.summary_sign.add(x, y, index, false);
        }
        self.histo.add(diff);
    }

    // Indicate whether data currently satisfies allowed tolerance and sign change acceptance.
    pub fn is_ok(&self) -> bool {
        self.diff <= self.allow_diff && (self.allow_sign || self.summary_sign.count == 0)
    }

    // Assert that worst diff is within tolerance,
    // then assert that sign change status is allowed.
    pub fn assert(&self) {
        assert!(
            self.diff <= self.allow_diff,
            "assert failed item {}, {}: {}{:e} vs {}{:e} diff abs {:e} outside inclusive {:e}",
            self.summary_diff.sample_index,
            self.name,
            util::help_sign(self.summary_diff.sample_x),
            self.summary_diff.sample_x,
            util::help_sign(self.summary_diff.sample_y),
            self.summary_diff.sample_y,
            self.diff,
            self.allow_diff
        );
        assert!(
            self.allow_sign || self.summary_sign.count == 0,
            "assert failed item {}, {}: {}{:e} vs {}{:e} sign difference disallowed.",
            self.summary_sign.sample_index,
            self.name,
            util::help_sign(self.summary_sign.sample_x),
            self.summary_sign.sample_x,
            util::help_sign(self.summary_sign.sample_y),
            self.summary_sign.sample_y
        );
    }
}

impl Clone for DiffSummary<'_> {
        fn clone(&self) -> Self {
            DiffSummary {
                name: self.name,
                diff: self.diff,
                allow_diff: self.allow_diff,
                allow_sign: self.allow_sign,
                num_total: self.num_total,
                num_diff_fail: self.num_diff_fail,
                summary_diff: self.summary_diff.clone(),
                summary_sign: self.summary_sign.clone(),
                histo: self.histo.clone(),
                calc_diff: self.calc_diff,
            }
        }
}

impl Display for DiffSummary<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        assert!(self.num_diff_fail <= self.num_total);
        write!(
            f,
            "{}{}count {}",
            self.name,
            if self.name.len() > 0 { ": " } else { "" },
            self.num_total
        )?;
        if self.summary_diff.count > 0 {
            write!(
                f,
                ", worst index {} {}{:e} vs {}{:e} diff {:e}, {}% failed tolerance {:e}, {}",
                self.summary_diff.sample_index,
                util::help_sign(self.summary_diff.sample_x),
                self.summary_diff.sample_x,
                util::help_sign(self.summary_diff.sample_y),
                self.summary_diff.sample_y,
                self.diff,
                util::to_percent(self.num_diff_fail, self.num_total),
                self.allow_diff,
                self.histo,
            )?;
        } else if self.num_total > 0 {
            write!(f, ", zero 100%, 0% failed tolerance {:e}", self.allow_diff)?;
        }
        if self.num_total > 0 {
            write!(
                f,
                ", sign diffs {}%",
                util::to_percent(self.summary_sign.count, self.num_total),
            )?;
            if self.summary_sign.count > 0 {
                write!(f,
                    " first index {} {}{:e} vs {}{:e}",
                    self.summary_sign.sample_index,
                    util::help_sign(self.summary_sign.sample_x),
                    self.summary_sign.sample_x,
                    util::help_sign(self.summary_sign.sample_y),
                    self.summary_sign.sample_y,
                )?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{DiffSummary};
    use crate::diff;
    use std::f64;

    #[test]
    fn test1() {
        let data = &[
            (0.0, 1.0),
            (2.0, 1.0),
            (1.0, 10.0),
            (0.1, -0.1),
            (f64::NAN, f64::NAN),
        ];
        let mut summary = DiffSummary::new("simple", 1.0, false, 4, &diff::diff_abs);
        for (i, item) in data.iter().enumerate() {
            summary.add(item.0, item.1, i);
        }
        println!();
        println!("{}", summary);
        assert!(!summary.is_ok());
    }

    #[test]
    fn test2() {
        let data = &[
            (0.0, 0.0, 1.0, 1.0),
            (0.0, -0.0, 2.1, 2.1),
            (-0.0, 0.0, -5.3, -5.3),
            (-0.0, -0.0, 504.0, 504.0),
            (f64::NAN, f64::NAN, 1.2, 1.21),
            (f64::NAN, -f64::NAN, 1.2, 1.201),
            (-f64::NAN, f64::NAN, 1.2, 1.2001),
            (-f64::NAN, -f64::NAN, 1.2, 1.20001),
            (f64::INFINITY, f64::INFINITY, 0.0, 1.1e-7),
            (f64::INFINITY, f64::NEG_INFINITY, 0.0, 2e-8),
            (f64::NEG_INFINITY, f64::INFINITY, 0.0, -6e-9),
            (f64::NEG_INFINITY, f64::NEG_INFINITY, 0.0, 7e-10),
            (f64::NAN, f64::INFINITY, 0.0, -4e-11),
            (f64::NEG_INFINITY, -f64::NAN, 0.0, 1e-6),
            (f64::INFINITY, -f64::NAN, 0.0, 1e-12),
            (17.0, f64::NAN, 0.0, 1e-13),
            (f64::INFINITY, 23.0, 0.0, 1e-14),
            (0.0, 3e-8, 0.0, 2e-15),
            (-6.7e-19, 1.2e-32, 0.0, 1e-15),
            (-1.1e-2, -0.0, 0.0, 1e-16),
            (f64::MIN_POSITIVE, 0.0, 0.0, 1e-17),
            (5e200, 5.001e200, 0.0, -1e-17),
            (f64::MAX, f64::MIN, 0.0, 1e-18),
        ];

        let mut summaries = DiffSummary::new_vec(4, &[
            ("data0", 2e-8, false, &diff::diff_abs),
            ("data1", 1e-6, true, &diff::diff_abs),
            ("data2", 1e-9, false, &diff::diff_abs),
            ("data3", 1e-9, false, &diff::diff_abs),
        ]);
        for (i, item) in data.iter().enumerate() {
            summaries[0].add(item.0, item.1, i);
            summaries[1].add(item.2, item.3, i);
            summaries[2].add(item.0, item.0, i);
        }

        println!();
        for summary in &summaries {
            println!("{}", summary);
        }
        assert_eq!(summaries[0].num_total, data.len());
        assert_eq!(summaries[1].num_total, data.len());
        assert!(summaries[0].num_diff_fail > summaries[1].num_diff_fail);
        assert_eq!(summaries[2].num_total, data.len());
        assert_eq!(summaries[2].summary_diff.count, 0);
        assert_eq!(summaries[2].summary_sign.count, 0);
        assert_eq!(summaries[3].num_total, 0);
        assert!(!summaries[0].is_ok());
        assert!(!summaries[1].is_ok());
        assert!(summaries[2].is_ok());
        assert!(summaries[3].is_ok());
    }
}