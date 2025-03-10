use super::{
    ApicizeExecution, ApicizeExecutionType, ApicizeGroup, ApicizeGroupChildren, ApicizeGroupItem,
    ApicizeGroupRun, ApicizeList, ApicizeRequest, ApicizeResult, ApicizeRow, ApicizeRowRuns,
    ApicizeRowSummary,
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

impl Tally for ApicizeGroup {
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

impl Tally for ApicizeList<ApicizeGroupItem> {
    fn get_tallies(&self) -> Tallies {
        self.items.get_tallies()
    }
}

impl Tally for Vec<ApicizeGroupItem> {
    fn get_tallies(&self) -> Tallies {
        let mut tallies = Tallies::default();
        for item in self {
            let item_tallies = match item {
                ApicizeGroupItem::Group(group) => group.get_tallies(),
                ApicizeGroupItem::Request(request) => request.get_tallies(),
            };
            tallies.add(&item_tallies);
        }
        tallies
    }
}

impl Tally for Vec<ApicizeRow> {
    fn get_tallies(&self) -> Tallies {
        let mut tallies = Tallies::default();
        for summary in self {
            tallies.add(&summary.items.get_tallies());
        }
        tallies
    }
}

impl Tally for ApicizeGroupChildren {
    fn get_tallies(&self) -> Tallies {
        match self {
            ApicizeGroupChildren::Items(children) => children.get_tallies(),
            ApicizeGroupChildren::Runs(runs) => runs.get_tallies(),
        }
    }
}

impl Tally for ApicizeGroupRun {
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

impl Tally for ApicizeList<ApicizeGroupRun> {
    fn get_tallies(&self) -> Tallies {
        self.items.get_tallies()
    }
}

impl Tally for ApicizeRowSummary {
    fn get_tallies(&self) -> Tallies {
        let mut tallies = Tallies::default();
        for row in &self.rows {
            tallies.add(&row.get_tallies());
        }
        tallies
    }
}

impl Tally for ApicizeRow {
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

impl Tally for ApicizeResult {
    fn get_tallies(&self) -> Tallies {
        match self {
            ApicizeResult::Items(items) => items.get_tallies(),
            ApicizeResult::Rows(summary) => summary.get_tallies(),
        }
    }
}

impl Tally for Vec<ApicizeGroupRun> {
    fn get_tallies(&self) -> Tallies {
        let mut tallies = Tallies::default();
        for run in self {
            tallies.add(&run.get_tallies());
        }
        tallies
    }
}

impl Tally for ApicizeGroupItem {
    fn get_tallies(&self) -> Tallies {
        match self {
            ApicizeGroupItem::Group(group) => Tallies {
                success: group.success,
                request_success_count: group.request_success_count,
                request_failure_count: group.request_failure_count,
                request_error_count: group.request_error_count,
                test_pass_count: group.test_pass_count,
                test_fail_count: group.test_fail_count,
            },
            ApicizeGroupItem::Request(request) => Tallies {
                success: request.success,
                request_success_count: request.request_success_count,
                request_failure_count: request.request_failure_count,
                request_error_count: request.request_error_count,
                test_pass_count: request.test_pass_count,
                test_fail_count: request.test_fail_count,
            },
        }
    }
}

impl Tally for ApicizeRequest {
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

impl Tally for ApicizeExecution {
    fn get_tallies(&self) -> Tallies {
        Tallies {
            success: self.success,
            request_success_count: if self.success { 1 } else { 0 },
            request_failure_count: if self.success {
                0
            } else if self.error.is_none() {
                1
            } else {
                0
            },
            request_error_count: if self.success {
                0
            } else if self.error.is_some() {
                1
            } else {
                0
            },
            test_pass_count: self.test_pass_count,
            test_fail_count: self.test_fail_count,
        }
    }
}

impl Tally for ApicizeExecutionType {
    fn get_tallies(&self) -> Tallies {
        match self {
            ApicizeExecutionType::None => Tallies::default(),
            ApicizeExecutionType::Single(execution) => execution.get_tallies(),
            ApicizeExecutionType::Runs(execution) => execution.get_tallies(),
            // ApicizeExecutionType::Rows(execution) => execution.get_tallies(),
            // ApicizeExecutionType::MultiRunRows(rows) => rows.get_tallies(),
        }
    }
}

impl Tally for ApicizeList<ApicizeExecution> {
    fn get_tallies(&self) -> Tallies {
        self.items.get_tallies()
    }
}

impl Tally for Vec<ApicizeExecution> {
    fn get_tallies(&self) -> Tallies {
        let mut tallies = Tallies::default();
        for execution in self {
            tallies.add(&execution.get_tallies());
        }
        tallies
    }
}

impl Tally for ApicizeRowRuns {
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

impl Tally for ApicizeList<ApicizeRowRuns> {
    fn get_tallies(&self) -> Tallies {
        let mut tallies = Tallies::default();
        for execution in &self.items {
            tallies.add(&execution.get_tallies());
        }
        tallies
    }
}
