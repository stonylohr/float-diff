// Summary of count of times a condition occurs for DiffSummary,
// and information about a sample occurrence (first for sign
// difference, worst for numeric difference).
pub struct DiffPartSummary {
    pub sample_x: f64,
    pub sample_y: f64,
    pub sample_index: usize,
    pub count: usize,
}

impl Copy for DiffPartSummary {
}

impl Clone for DiffPartSummary {
    fn clone(&self) -> Self {
        DiffPartSummary {
            sample_x: self.sample_x,
            sample_y: self.sample_y,
            sample_index: self.sample_index,
            count: self.count,
        }
    }
}

impl DiffPartSummary {
    pub fn new() -> Self {
        DiffPartSummary {
            sample_x: f64::NAN,
            sample_y: f64::NAN,
            sample_index: 0,
            count: 0,
        }
    }

    // Update the summary based on an iteration.
    // If "worst" is true, update sample_* values even if this isn't the first item added.
    pub fn add(&mut self, x: f64, y: f64, index: usize, worst: bool) {
        if worst || self.count == 0 {
            self.sample_x = x;
            self.sample_y = y;
            self.sample_index = index;
        }
        self.count += 1;
    }
}
