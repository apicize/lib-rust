use super::{
    ApicizeExecution, ApicizeGroupResult, ApicizeGroupResultContent, ApicizeGroupResultRow, ApicizeGroupResultRowContent, ApicizeGroupResultRun, ApicizeRequestResult, ApicizeRequestResultContent, ApicizeRequestResultRow, ApicizeRequestResultRowContent, ApicizeRequestResultRun, ApicizeResult
};

pub trait Tally {
    fn get_tallies(&self) -> Tallies;
}

pub struct Tallies {
    pub success: bool,
    pub request_success_count: usize,
    pub request_failure_count: usize,
    pub request_error_count: usize,
    pub test_pass_count: usize,
    pub test_fail_count: usize,
}

impl Default for Tallies {
    fn default() -> Self {
        Self {
            success: true,
            request_success_count: 0,
            request_failure_count: 0,
            request_error_count: 0,
            test_pass_count: 0,
            test_fail_count: 0,
        }
    }
}

impl Tallies {
    pub fn add(&mut self, other: &Tallies) {
        self.success = self.success && other.success;
        self.request_success_count += other.request_success_count;
        self.request_failure_count += other.request_failure_count;
        self.request_error_count += other.request_error_count;
        self.test_pass_count += other.test_pass_count;
        self.test_fail_count += other.test_fail_count;
    }
}

impl Tally for ApicizeResult {
    fn get_tallies(&self) -> Tallies {
        match self {
            ApicizeResult::Request(request) => request.get_tallies(),
            ApicizeResult::Group(group) => group.get_tallies(),
        }
    }
}

impl Tally for Vec<ApicizeResult> {
    fn get_tallies(&self) -> Tallies {
        let mut tallies = Tallies::default();
        for result in self {
            tallies.add(&result.get_tallies());
        }
        tallies
    }
}
impl Tally for ApicizeRequestResultContent {
    fn get_tallies(&self) -> Tallies {
        match self {
            ApicizeRequestResultContent::Rows { rows, .. } => rows.get_tallies(),
            ApicizeRequestResultContent::Runs { runs, .. } => runs.get_tallies(),
            ApicizeRequestResultContent::Execution { execution } => execution.get_tallies(),
        }
    }
}

impl Tally for ApicizeRequestResult {
    fn get_tallies(&self) -> Tallies {
        Tallies {
            success: self.success,
            request_success_count: self.request_success_count,
            request_failure_count: self.request_failure_count,
            request_error_count: self.request_error_count,
            test_pass_count: self.test_pass_count,
            test_fail_count: self.test_fail_count,
        }
    }
}

impl Tally for ApicizeRequestResultRowContent {
    fn get_tallies(&self) -> Tallies {
        match self {
            ApicizeRequestResultRowContent::Runs(runs) => runs.get_tallies(),
            ApicizeRequestResultRowContent::Execution(execution) => execution.get_tallies(),
        }
    }
}

impl Tally for ApicizeRequestResultRow {
    fn get_tallies(&self) -> Tallies {
        Tallies {
            success: self.success,
            request_success_count: self.request_success_count,
            request_failure_count: self.request_failure_count,
            request_error_count: self.request_error_count,
            test_pass_count: self.test_pass_count,
            test_fail_count: self.test_fail_count,
        }
    }
}

impl Tally for Vec<ApicizeRequestResultRow> {
    fn get_tallies(&self) -> Tallies {
        let mut tallies = Tallies::default();
        for run in self {
            tallies.add(&run.get_tallies());
        }
        tallies
    }
}

impl Tally for ApicizeRequestResultRun {
    fn get_tallies(&self) -> Tallies {
        Tallies {
            success: self.success,
            request_success_count: self.request_success_count,
            request_failure_count: self.request_failure_count,
            request_error_count: self.request_error_count,
            test_pass_count: self.test_pass_count,
            test_fail_count: self.test_fail_count,
        }
    }
}

impl Tally for Vec<ApicizeRequestResultRun> {
    fn get_tallies(&self) -> Tallies {
        let mut tallies = Tallies::default();
        for run in self {
            tallies.add(&run.get_tallies());
        }
        tallies
    }
}

impl Tally for ApicizeGroupResultContent {
    fn get_tallies(&self) -> Tallies {
        match self {
            ApicizeGroupResultContent::Rows { rows } => rows.get_tallies(),
            ApicizeGroupResultContent::Runs { runs } => runs.get_tallies(),
            ApicizeGroupResultContent::Entries { entries} => entries.get_tallies(),
        }
    }
}

impl Tally for ApicizeGroupResult {
    fn get_tallies(&self) -> Tallies {
        Tallies {
            success: self.success,
            request_success_count: self.request_success_count,
            request_failure_count: self.request_failure_count,
            request_error_count: self.request_error_count,
            test_pass_count: self.test_pass_count,
            test_fail_count: self.test_fail_count,
        }
    }
}

impl Tally for ApicizeGroupResultRowContent {
    fn get_tallies(&self) -> Tallies {
        match self {
            ApicizeGroupResultRowContent::Runs { runs } => runs.get_tallies(),
            ApicizeGroupResultRowContent::Entries { entries } => entries.get_tallies(),
        }
    }
}

impl Tally for ApicizeGroupResultRow {
    fn get_tallies(&self) -> Tallies {
        Tallies {
            success: self.success,
            request_success_count: self.request_success_count,
            request_failure_count: self.request_failure_count,
            request_error_count: self.request_error_count,
            test_pass_count: self.test_pass_count,
            test_fail_count: self.test_fail_count,
        }
    }
}

impl Tally for Vec<ApicizeGroupResultRow> {
    fn get_tallies(&self) -> Tallies {
        let mut tallies = Tallies::default();
        for row in self {
            tallies.add(&row.get_tallies());
        }
        tallies
    }
}

impl Tally for ApicizeGroupResultRun {
    fn get_tallies(&self) -> Tallies {
        Tallies {
            success: self.success,
            request_success_count: self.request_success_count,
            request_failure_count: self.request_failure_count,
            request_error_count: self.request_error_count,
            test_pass_count: self.test_pass_count,
            test_fail_count: self.test_fail_count,
        }
    }
}

impl Tally for Vec<ApicizeGroupResultRun> {
    fn get_tallies(&self) -> Tallies {
        let mut tallies = Tallies::default();
        for row in self {
            tallies.add(&row.get_tallies());
        }
        tallies
    }
}

impl Tally for ApicizeExecution {
    fn get_tallies(&self) -> Tallies {
        let has_error = self.error.is_some();
        let has_failures = self.test_fail_count > 0;
        Tallies {
            success: self.success,
            request_success_count: if has_failures || has_error { 0 } else { 1 },
            request_error_count: if has_error { 1 } else { 0 },
            request_failure_count: if has_failures { 1 } else { 0 },
            test_pass_count: self.test_pass_count,
            test_fail_count: self.test_fail_count,
        }
    }
}
