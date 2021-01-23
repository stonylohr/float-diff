use std::fmt::Display;
use std::collections::{BTreeMap, HashMap};
use crate::util;

// A struct for taking a set of values values, splitting into special case
// and log10 buckets, and displaying the current distribution using a
// specified maximum number of log10 buckets.
// Primarily intended for getting a quick overview of expected vs calculated
// values for a potentially large dataset.
// Current implementation assumes that all incoming values are non-negative.
// Note that formatting for display may be relatively expensive.
pub struct LogHistogram {
    // The number of nans added
    pub(crate) num_nan: usize,
    // The number of infinite values added
    pub(crate) num_inf: usize,
    // The number of exactly-zero values added
    pub(crate) num_zero: usize,

    // max_display_buckets is the maximum number of log buckets to display, not
    // counting the special case buckets for NAN, INF, and 0. The bucket count
    // is enforced by reporting sparse buckets with neighboring buckets.
    // max_display_buckets must be at least 3, to avoid some special cases that
    // would come up for lower caps.
    pub(crate) max_display_buckets: usize,

    // The standard buckets based on log10 of the incoming value
    pub(crate) log10_buckets: HashMap<isize, usize>,
}

impl LogHistogram {
    pub fn new(max_display_buckets: usize) -> Self {
        assert!(max_display_buckets > 2);
        LogHistogram {
            num_nan: 0,
            num_inf: 0,
            num_zero: 0,
            max_display_buckets: max_display_buckets,
            log10_buckets: HashMap::new(),
        }
    }

    // Add a new item to the dataset being tracked.
    pub fn add(&mut self, diff: f64) {
        assert!(diff.is_sign_positive());
        if diff.is_nan() {
            self.num_nan += 1;
        } else if diff.is_infinite() {
            self.num_inf += 1;
        } else if diff == 0.0 {
            self.num_zero += 1;
        } else {
            let exp = diff.log10() as isize;
            let current: usize = match self.log10_buckets.get(&exp) {
                Some(val) => *val,
                _ => 0,
            };
            self.log10_buckets.insert(exp, current + 1);
        }
    }
}

impl Clone for LogHistogram {
    fn clone(&self) -> Self {
        LogHistogram {
            num_nan: self.num_nan,
            num_inf: self.num_inf,
            num_zero: self.num_zero,
            max_display_buckets: self.max_display_buckets,
            log10_buckets: self.log10_buckets.clone(),
        }
    }
}

impl Display for LogHistogram {
    // Display a summary, reduced down to a manageable number of buckets.
    // Note that this bucket reduction may be relatively expensive.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        assert!(self.max_display_buckets > 2);
        let mut keys_asc: Vec<isize> = Vec::new();
        // histo_reduced map's keys are the original exponent.
        // Its values are (reduced_exponent_min, reduced_exponent_max, count).
        let mut histo_reduced: BTreeMap<isize, (isize, isize, usize)> = BTreeMap::new();
        let mut num_total = self.num_inf + self.num_nan + self.num_zero;
        self.log10_buckets.iter().for_each(|(&key, &val)| {
            keys_asc.push(key);
            histo_reduced.insert(key, (key, key, val));
            num_total += val;
        });
        keys_asc.sort();
        while histo_reduced.len() > self.max_display_buckets {
            // Collapse the smallest bucket into its less-populated neighbor.
            // Favor the less-populated neighbor, to improve odds that ending
            // buckets are at least somewhat evenly distributed in population.
            let mut collapse_from = isize::MIN;
            let mut val_smallest = (collapse_from, collapse_from, usize::MAX);
            histo_reduced.iter().for_each(|(&key, &(_exp_min, _exp_max, count))| {
                if count < val_smallest.2 {
                    collapse_from = key;
                    val_smallest = (key, key, count);
                }
            });

            let index_smallest = keys_asc.iter().position(|&val| val == collapse_from).unwrap();
            // Note that our restriction on max_display_buckets lets us
            // trust we stop looping before we reach the case of 2 or fewer
            // buckets, which would require additional special case logic.
            let (collapse_to, val_to) = if index_smallest == 0 {
                let key_next = keys_asc[index_smallest + 1];
                let val_next = histo_reduced.get(&key_next).unwrap();
                (key_next, val_next)
            } else if index_smallest >= histo_reduced.len() - 1 {
                let key_prev = keys_asc[index_smallest - 1];
                let val_prev = histo_reduced.get(&key_prev).unwrap();
                (key_prev, val_prev)
            } else {
                // Favor collapsing into the smaller bucket, to reduce lopsided bucket sizes
                let key_prev = keys_asc[index_smallest - 1];
                let key_next = keys_asc[index_smallest + 1];
                let val_prev = histo_reduced.get(&key_prev).unwrap();
                let val_next = histo_reduced.get(&key_next).unwrap();
                if val_next.2 < val_prev.2 {
                    (key_next, val_next)
                } else {
                    (key_prev, val_prev)
                }
            };

            let val_sum = (isize::min(val_to.0, val_smallest.0), isize::max(val_to.1, val_smallest.1), val_to.2 + val_smallest.2);
            histo_reduced.insert(collapse_to, val_sum);
            histo_reduced.remove(&collapse_from);
            keys_asc.remove(index_smallest);
            assert_eq!(keys_asc.len(), histo_reduced.len(), "Size mismatch between key list and map");
        }

        let mut first = true;
        let mut pad_maybe = || {
            if first {
                first = false;
                ""
            } else {
                ", "
            }
        };

        // write!(f, "{}count {}", pad_maybe(), num_total)?;

        if self.num_zero > 0 {
            let percent_zero = util::to_percent(self.num_zero, num_total); 
            write!(f, "{}zero {}%", pad_maybe(), percent_zero)?;
        }

        // Convert counts to percentages
        histo_reduced.iter_mut().for_each(|(_key, (_exp_min, _exp_max, count))| {
            assert!(*count != 0, "Internal error: Bucket contains no items");
            *count = util::to_percent(*count, num_total);
        });
        for (key, (exp_min, exp_max, count)) in &histo_reduced {
            if exp_min == exp_max {
                write!(f, "{}e{} {}%", pad_maybe(), key, count)?;
            } else {
                write!(f, "{}e{} to e{} {}%", pad_maybe(), exp_min, exp_max, count)?;
            }
        }
        if self.num_inf > 0 {
            let percent_inf = util::to_percent(self.num_inf, num_total);
            write!(f, "{}inf {}%", pad_maybe(), percent_inf)?;
        }
        if self.num_nan > 0 {
            let percent_nan = util::to_percent(self.num_nan, num_total);
            write!(f, "{}nan {}%", pad_maybe(), percent_nan)?;
        }
        Ok(())
    }
}
