use super::{ApicizeExecution, ApicizeExecutionDetail, ApicizeExecutionSummary, ApicizeItem, ApicizeRequestWithExecution, ApicizeSummary};

pub struct Tallies {
    pub success: bool,
    pub requests_with_passed_tests_count: usize,
    pub requests_with_failed_tests_count: usize,
    pub requests_with_errors: usize,
    pub passed_test_count: usize,
    pub failed_test_count: usize,
}

impl Default for Tallies {
    fn default() -> Self {
        Self {
            success: false,
            requests_with_passed_tests_count: 0,
            requests_with_failed_tests_count: 0,
            requests_with_errors: 0,
            passed_test_count: 0,
            failed_test_count: 0,
        }
    }
}

impl Tallies {
    pub fn add_summary(&mut self, summary: &ApicizeSummary) {
        self.success = self.success && summary.success;
        self.passed_test_count += summary.passed_test_count;
        self.failed_test_count += summary.failed_test_count;

        self.requests_with_passed_tests_count += summary.requests_with_passed_tests_count;
        self.requests_with_failed_tests_count += summary.requests_with_failed_tests_count;
        self.requests_with_errors += summary.requests_with_errors;
    }

    pub fn add_executions(&mut self, executions: &Vec<ApicizeExecution>) {
        for execution in executions {
            self.add_execution(execution);
        }
    }

    pub fn add_execution(&mut self, execution: &ApicizeExecution) {
        match execution {
            ApicizeExecution::Details(items) => self.add_items(items),
            ApicizeExecution::Rows(items) => self.add_execution_summaries(items),
            ApicizeExecution::Runs(items) => self.add_execution_details(items),
        }
    }
    pub fn add_execution_summaries(&mut self, executions: &Vec<ApicizeExecutionSummary>) {
        let mut requests_with_passed_tests_count = self.requests_with_passed_tests_count;
        let mut requests_with_failed_tests_count = self.requests_with_failed_tests_count;
        let mut requests_with_errors = self.requests_with_errors;

        for execution in executions {
            self.success = self.success && execution.success;
            self.passed_test_count += execution.passed_test_count;
            self.failed_test_count += execution.failed_test_count;

            if execution.requests_with_passed_tests_count > 0 {
                requests_with_passed_tests_count = 1;
            }
            if execution.requests_with_failed_tests_count > 0 {
                requests_with_failed_tests_count = 1;
            }
            if execution.requests_with_errors > 0 {
                requests_with_errors = 1;
            }

            if let Some(child_execution) = &execution.children {
                self.add_execution(child_execution);
            }
        }
        self.requests_with_passed_tests_count = requests_with_passed_tests_count;
        self.requests_with_failed_tests_count = requests_with_failed_tests_count;
        self.requests_with_errors = requests_with_errors;
    }

    pub fn add_execution_details(&mut self, executions: &Vec<ApicizeExecutionDetail>) {
        let mut requests_with_passed_tests_count = self.requests_with_passed_tests_count;
        let mut requests_with_failed_tests_count = self.requests_with_failed_tests_count;
        let mut requests_with_errors = self.requests_with_errors;

        for execution in executions {
            self.success = self.success && execution.success;
            self.passed_test_count += execution.passed_test_count;
            self.failed_test_count += execution.failed_test_count;

            if execution.passed_test_count > 0 {
                requests_with_passed_tests_count = 1;
            }
            if execution.failed_test_count > 0 {
                requests_with_failed_tests_count = 1;
            }
            if execution.error.is_some() {
                requests_with_errors = 1;
            }
        }
        self.requests_with_passed_tests_count = requests_with_passed_tests_count;
        self.requests_with_failed_tests_count = requests_with_failed_tests_count;
        self.requests_with_errors = requests_with_errors;    
    }

    pub fn add_executed_request(&mut self, request: &ApicizeRequestWithExecution) {
        self.success = self.success && request.success;
        self.passed_test_count += request.passed_test_count;
        self.failed_test_count += request.failed_test_count;

        if request.passed_test_count > 0 {
            self.requests_with_passed_tests_count = 1;
        }
        if request.failed_test_count > 0 {
            self.requests_with_failed_tests_count = 1;
        }
        if request.error.is_some() {
            self.requests_with_errors = 1;
        }
    }

    pub fn add_items(&mut self, items: &Vec<ApicizeItem>) {
        for item in items {
            match item {
                ApicizeItem::Group(summary) => self.add_summary(summary),
                ApicizeItem::Request(summary) => self.add_summary(summary),
                ApicizeItem::ExecutedRequest(request) => self.add_executed_request(request),
                ApicizeItem::Execution(execution) => self.add_execution(execution),
                ApicizeItem::ExecutionSummaries(summaries) => self.add_execution_summaries(summaries),
                ApicizeItem::Items(items) => self.add_items(items),
            }
        }
    }
}
