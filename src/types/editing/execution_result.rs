use crate::{editing::execution_result_detail::{ExecutionResultDetailGroup, ExecutionResultDetailRequest}, ApicizeGroup, ApicizeGroupChildren, ApicizeGroupItem, ApicizeGroupRun, ApicizeList, ApicizeRequest, ApicizeResult, ApicizeRow, ApicizeRowSummary};

use super::{execution_result_detail::ExecutionResultDetail, execution_result_summary::ExecutionResultSummary, execution_result_success::ExecutionResultSuccess};

pub type ExecutionResult = (ExecutionResultSummary, ExecutionResultDetail);

impl ApicizeList<ApicizeGroupItem> {
    #[allow(clippy::too_many_arguments)]
    fn append_to_list(
        self,
        list: &mut Vec<ExecutionResult>,
        level: usize,
        parent_index: Option<usize>,
        row_number: Option<usize>,
        row_count: Option<usize>,
        run_number: Option<usize>,
        run_count: Option<usize>,
    ) -> Vec<usize> {
        self.items
            .into_iter()
            .map(|item| {
                item.append_to_list(
                    list,
                    level,
                    parent_index,
                    row_number,
                    row_count,
                    run_number,
                    run_count,
                )
            })
            .collect::<Vec<Vec<usize>>>()
            .concat()
    }
}

impl ApicizeGroupItem {
    #[allow(clippy::too_many_arguments)]
    fn append_to_list(
        self,
        list: &mut Vec<ExecutionResult>,
        level: usize,
        parent_index: Option<usize>,
        row_number: Option<usize>,
        row_count: Option<usize>,
        run_number: Option<usize>,
        run_count: Option<usize>,
    ) -> Vec<usize> {
        match self {
            ApicizeGroupItem::Group(group) => group.append_to_list(
                list,
                level,
                parent_index,
                row_number,
                row_count,
                run_number,
                run_count,
            ),
            ApicizeGroupItem::Request(request) => {
                request.append_to_list(list, level, parent_index, row_number, row_count)
            }
        }
    }
}

impl ApicizeGroup {
    #[allow(clippy::too_many_arguments)]
    fn append_to_list(
        self,
        list: &mut Vec<ExecutionResult>,
        level: usize,
        parent_index: Option<usize>,
        row_number: Option<usize>,
        row_count: Option<usize>,
        run_number: Option<usize>,
        run_count: Option<usize>,
    ) -> Vec<usize> {
        let this_index = list.len();

        let summary = ExecutionResultSummary {
            request_or_group_id: self.id.clone(),
            index: this_index,
            parent_index,
            child_indexes: None,
            level,
            name: self.name.to_string(),
            executed_at: self.executed_at,
            duration: self.duration,
            status: None,
            status_text: None,
            has_response_headers: false,
            response_body_length: None,
            test_results: None,
            success: if self.request_error_count > 0 {
                ExecutionResultSuccess::Error
            } else if self.request_failure_count > 0 {
                ExecutionResultSuccess::Failure
            } else {
                ExecutionResultSuccess::Success
            },
            error: None,
            run_number,
            run_count,
            row_number,
            row_count,
        };

        let detail = ExecutionResultDetail::Grouped(
            Box::new(ExecutionResultDetailGroup {
                id: self.id.clone(),
                name: self.name.to_string(),
                run_number,
                row_number,
                executed_at: self.executed_at,
                duration: self.duration,
                output_variables: self.output_variables,
                success: if self.request_error_count > 0 {
                    ExecutionResultSuccess::Error
                } else if self.request_failure_count > 0 {
                    ExecutionResultSuccess::Failure
                } else {
                    ExecutionResultSuccess::Success
                },
                request_success_count: self.request_success_count,
                request_failure_count: self.request_failure_count,
                request_error_count: self.request_error_count,
                test_pass_count: self.test_pass_count,
                test_fail_count: self.test_fail_count,
            }
        ));

        list.push((summary, detail));

        if let Some(children) = self.children {
            let child_indexes: Vec<usize> = match children {
                ApicizeGroupChildren::Items(items) => items.append_to_list(
                    list,
                    level + 1,
                    Some(this_index),
                    row_number,
                    row_count,
                    run_number,
                    run_count,
                ),
                ApicizeGroupChildren::Runs(runs) => runs.append_to_list(
                    list,
                    level + 1,
                    Some(this_index),
                    &self.id,
                    row_number,
                    row_count,
                ),
            };
            if !child_indexes.is_empty() {
                list.get_mut(this_index).unwrap().0.child_indexes = Some(child_indexes);
            }
        }

        vec![this_index]
    }
}

impl ApicizeRequest {
    fn append_to_list(
        self,
        list: &mut Vec<ExecutionResult>,
        level: usize,
        parent_index: Option<usize>,
        row_number: Option<usize>,
        row_count: Option<usize>,
    ) -> Vec<usize> {
        let this_index = list.len();

        match self.execution {
            crate::ApicizeExecutionType::None => {
                // NO-OP
                vec![]
            }
            crate::ApicizeExecutionType::Single(execution) => {
                let (status, status_text) = match &execution.response {
                    Some(r) => (Some(r.status), Some(r.status_text.clone())),
                    None => (None, None),
                };

                let summary = ExecutionResultSummary {
                    request_or_group_id: self.id.clone(),
                    index: this_index,
                    parent_index,
                    child_indexes: None,
                    level,
                    name: self.name.to_string(),
                    executed_at: self.executed_at,
                    duration: self.duration,
                    status,
                    status_text,
                    has_response_headers: execution.response.as_ref().is_some_and(|r| r.headers.is_some()),
                    response_body_length: execution.response.as_ref()
                        .map(|r| r.body.as_ref())
                        .unwrap_or(None)
                        .map(|b| 
                            match b {
                                crate::ApicizeBody::Text { data } => data.len(),
                                crate::ApicizeBody::JSON { text, .. } => text.len(),
                                crate::ApicizeBody::Binary { data } => data.len(),
                            }
                        ),
                    test_results: execution.tests.clone(),
                    success: if execution.success {
                        ExecutionResultSuccess::Success
                    } else if execution.error.is_none() {
                        ExecutionResultSuccess::Failure
                    } else {
                        ExecutionResultSuccess::Error
                    },    
                    error: execution.error.clone(),
                    run_number: None,
                    run_count: None,
                    row_number,
                    row_count,
                };

                let detail = ExecutionResultDetail::Request(
                    ExecutionResultDetailRequest {
                        id: self.id.clone(),
                        name: self.name.to_string(),
                        row_number,
                        run_number: None,
                        executed_at: self.executed_at,
                        duration: self.duration,
                        input_variables: self.input_variables,
                        data: execution.data,
                        output_variables: execution.output_variables,
                        request: execution.request,
                        response: execution.response,
                        tests: execution.tests,
                        error: execution.error,
                        success: if self.request_error_count > 0 {
                            ExecutionResultSuccess::Error
                        } else if self.request_failure_count > 0 {
                            ExecutionResultSuccess::Failure
                        } else {
                            ExecutionResultSuccess::Success
                        },
                        request_success_count: self.request_success_count,
                        request_failure_count: self.request_failure_count,
                        request_error_count: self.request_error_count,
                        test_pass_count: self.test_pass_count,
                        test_fail_count: self.test_fail_count,
                    }
                );

                list.push((summary, detail));
                vec![this_index]
            }
            crate::ApicizeExecutionType::Runs(executions) => {
                let mut run_number = 0;
                let run_count = executions.items.len();
                executions
                    .items
                    .into_iter()
                    .map(|execution| {
                        let run_index = list.len();
                        run_number += 1;

                        let (status, status_text) = match &execution.response {
                            Some(r) => (Some(r.status), Some(r.status_text.clone())),
                            None => (None, None),
                        };

                        let summary = ExecutionResultSummary {
                            request_or_group_id: self.id.clone(),
                            index: list.len(),
                            parent_index,
                            child_indexes: None,
                            level,
                            name: self.name.to_string(),
                            executed_at: self.executed_at,
                            duration: self.executed_at,
                            status,
                            status_text,
                            has_response_headers: execution.response.as_ref().is_some_and(|r| r.headers.is_some()),
                            response_body_length: execution.response.as_ref().and_then(|r| r.body.as_ref().map(|b| match b {
                                crate::ApicizeBody::Text { data } => data.len(),
                                crate::ApicizeBody::JSON { text, .. } => text.len(),
                                crate::ApicizeBody::Binary { data } => data.len(),
                            })),        
                            test_results: execution.tests.clone(),
                            success: if execution.success {
                                ExecutionResultSuccess::Success
                            } else if execution.error.is_none() {
                                ExecutionResultSuccess::Failure
                            } else {
                                ExecutionResultSuccess::Error
                            },
                            error: execution.error.clone(),
                            run_number: Some(run_number),
                            run_count: Some(run_count),
                            row_number,
                            row_count,
                        };

                        let detail = ExecutionResultDetail::Request(
                            ExecutionResultDetailRequest {
                                id: self.id.clone(),
                                name: self.name.to_string(),
                                row_number,
                                run_number: Some(run_number),
                                executed_at: self.executed_at,
                                duration: self.duration,
                                input_variables: execution.input_variables,
                                data: execution.data,
                                output_variables: execution.output_variables,
                                request: execution.request,
                                response: execution.response,
                                tests: execution.tests,
                                error: execution.error,
                                success: if self.request_error_count > 0 {
                                    ExecutionResultSuccess::Error
                                } else if self.request_failure_count > 0 {
                                    ExecutionResultSuccess::Failure
                                } else {
                                    ExecutionResultSuccess::Success
                                },        
                                request_success_count: self.request_success_count,
                                request_failure_count: self.request_failure_count,
                                request_error_count: self.request_error_count,
                                test_pass_count: self.test_pass_count,
                                test_fail_count: self.test_fail_count,
                            }
                        );

                        list.push((summary, detail));
                        run_index
                    })
                    .collect()
            }
        }
    }
}

impl ApicizeList<ApicizeGroupRun> {
    fn append_to_list(
        self,
        list: &mut Vec<ExecutionResult>,
        level: usize,
        parent_index: Option<usize>,
        request_or_group_id: &str,
        row_number: Option<usize>,
        row_count: Option<usize>,
    ) -> Vec<usize> {
        let run_count = self.items.len();
        let mut run_number = 0;

        self.items
            .into_iter()
            .map(|run| {
                let this_index = list.len();
                run_number += 1;

                let summary = ExecutionResultSummary {
                    request_or_group_id: request_or_group_id.to_string(),
                    index: this_index,
                    parent_index,
                    child_indexes: None,
                    level,
                    name: String::default(),
                    executed_at: run.executed_at,
                    duration: run.duration,
                    status: None,
                    status_text: None,
                    has_response_headers: false,
                    response_body_length: None,
                    test_results: None,
                    success: if run.success {
                        ExecutionResultSuccess::Success
                    } else if run.request_error_count == 0 {
                        ExecutionResultSuccess::Failure
                    } else {
                        ExecutionResultSuccess::Error
                    },
                    error: None,
                    run_number: Some(run_number),
                    run_count: Some(run_count),
                    row_number,
                    row_count,
                };


                let detail = ExecutionResultDetail::Grouped(
                    Box::new(ExecutionResultDetailGroup {
                        id: request_or_group_id.to_string(),
                        name: String::default(),
                        row_number,
                        run_number: Some(run_number),
                        executed_at: run.executed_at,
                        duration: run.duration,
                        output_variables: run.output_variables,
                        success: if run.request_error_count > 0 {
                            ExecutionResultSuccess::Error
                        } else if run.request_failure_count > 0 {
                            ExecutionResultSuccess::Failure
                        } else {
                            ExecutionResultSuccess::Success
                        },
                        request_success_count: run.request_success_count,
                        request_failure_count: run.request_failure_count,
                        request_error_count: run.request_error_count,
                        test_pass_count: run.test_pass_count,
                        test_fail_count: run.test_fail_count,
                    })
                );                
                list.push((summary, detail));

                list.get_mut(this_index).unwrap().0.child_indexes = Some(
                    run.children
                        .into_iter()
                        .map(|child| {
                            child.append_to_list(
                                list,
                                level,
                                Some(this_index),
                                row_number,
                                row_count,
                                Some(run_number),
                                Some(run_count),
                            )
                        })
                        .collect::<Vec<Vec<usize>>>()
                        .concat(),
                );

                this_index
            })
            .collect::<Vec<usize>>()
    }
}

impl ApicizeRow {
    pub fn append_to_list(
        self,
        list: &mut Vec<ExecutionResult>,
        level: usize,
        parent_index: Option<usize>,
        request_or_group_id: &str,
        row_number: usize,
        row_count: usize,
    ) -> usize {
        let this_index = list.len();

        let result = ExecutionResultSummary {
            request_or_group_id: request_or_group_id.to_string(),
            parent_index,
            child_indexes: None,
            index: this_index,
            level,
            name: String::default(),
            executed_at: self.executed_at,
            duration: self.duration,
            status: None,
            status_text: None,
            has_response_headers: false,
            response_body_length: None,
            test_results: None,
            success: if self.success {
                ExecutionResultSuccess::Success
            } else if self.request_error_count == 0 {
                ExecutionResultSuccess::Failure
            } else {
                ExecutionResultSuccess::Error
            },
            error: None,
            run_number: None,
            run_count: None,
            row_number: Some(row_number),
            row_count: Some(row_count),
        };

        let detail = ExecutionResultDetail::Grouped(
            Box::new(ExecutionResultDetailGroup {
                id: request_or_group_id.to_string(),
                name: String::default(),
                run_number: None,
                row_number: None,
                executed_at: self.executed_at,
                duration: self.executed_at,
                output_variables: None,
                success: if self.request_error_count > 0 {
                    ExecutionResultSuccess::Error
                } else if self.request_failure_count > 0 {
                    ExecutionResultSuccess::Failure
                } else {
                    ExecutionResultSuccess::Success
                },
                request_success_count: self.request_success_count,
                request_failure_count: self.request_failure_count,
                request_error_count: self.request_error_count,
                test_pass_count: self.test_pass_count,
                test_fail_count: self.test_fail_count,
            })
        );

        list.push((result, detail));

        list.get_mut(this_index).unwrap().0.child_indexes = Some(
            self.items
                .into_iter()
                .map(|item| {
                    item.append_to_list(
                        list,
                        level,
                        Some(this_index),
                        Some(row_number),
                        Some(row_count),
                        None,
                        None,
                    )
                })
                .collect::<Vec<Vec<usize>>>()
                .concat(),
        );

        this_index
    }
}

impl ApicizeRowSummary {
    pub fn append_to_list(
        self,
        list: &mut Vec<ExecutionResult>,
        level: usize,
        parent_index: Option<usize>,
        request_or_group_id: &str,
    ) -> Vec<usize> {
        let row_count = self.rows.len();

        let this_index = list.len();

        let summary = ExecutionResultSummary {
            request_or_group_id: request_or_group_id.to_string(),
            index: this_index,
            parent_index,
            child_indexes: None,
            level,
            name: "All Rows".to_string(),
            executed_at: self.executed_at,
            duration: self.duration,
            status: None,
            status_text: None,
            has_response_headers: false,
            response_body_length: None,
            success: if self.request_error_count > 0 {
                ExecutionResultSuccess::Error
            } else if self.request_failure_count > 0 {
                ExecutionResultSuccess::Failure
            } else {
                ExecutionResultSuccess::Success
            },
            error: None,
            test_results: None,
            run_number: None,
            run_count: None,
            row_number: None,
            row_count: None,
        };

        let detail = ExecutionResultDetail::Grouped(
            Box::new(ExecutionResultDetailGroup {
                id: request_or_group_id.to_string(),
                name: "All Rows".to_string(),
                run_number: None,
                row_number: None,
                executed_at: self.executed_at,
                duration: self.executed_at,
                output_variables: None,
                success: if self.request_error_count > 0 {
                    ExecutionResultSuccess::Error
                } else if self.request_failure_count > 0 {
                    ExecutionResultSuccess::Failure
                } else {
                    ExecutionResultSuccess::Success
                },
                request_success_count: self.request_success_count,
                request_failure_count: self.request_failure_count,
                request_error_count: self.request_error_count,
                test_pass_count: self.test_pass_count,
                test_fail_count: self.test_fail_count,
            })
        );
        
        list.push((summary, detail));

        self.rows
            .into_iter()
            .map(|row| {
                list.get_mut(this_index).unwrap().0.child_indexes = Some(
                    row.items
                        .into_iter()
                        .map(|item| {
                            item.append_to_list(
                                list,
                                level,
                                Some(this_index),
                                Some(row.row_number),
                                Some(row_count),
                                None,
                                None,
                            )
                        })
                        .collect::<Vec<Vec<usize>>>()
                        .concat(),
                );

                this_index
            })
            .collect()
    }
}

impl ApicizeResult {
    pub fn assemble_results(self, request_or_group_id: &str) -> (Vec<ExecutionResultSummary>, Vec<ExecutionResultDetail>) {
        let mut list = Vec::<ExecutionResult>::new();
        match self {
            ApicizeResult::Rows(summary) => {
                summary.append_to_list(&mut list, 0, None, request_or_group_id);
            }
            ApicizeResult::Items(group_items) => {
                group_items.append_to_list(&mut list, 0, None, None, None, None, None);
            }
        }
        list.into_iter().unzip()
    }
}

