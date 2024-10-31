//! Apicize test routine persistence and execution.
//!
//! This library supports the opening, saving and dispatching Apicize functional web tests

#[macro_use]
extern crate lazy_static;

pub mod apicize;
pub mod errors;
pub mod oauth2_client_tokens;
pub mod settings;
pub mod parameters;
pub mod shared;
pub mod test_runner;
pub mod utility;
pub mod workbook;
pub mod workspace;
pub use errors::*;
pub use oauth2_client_tokens::*;
pub use parameters::*;
pub use settings::*;
pub use shared::*;
pub use test_runner::*;
pub use utility::*;
pub use workbook::*;
pub use workspace::*;


// #[cfg(test)]
// mod lib_tests {

//     use super::models::{WorkbookRequest, WorkbookRequestMethod};
//     use crate::{ExecutionError, WorkbookRequestEntry};

//     #[tokio::test]
//     async fn test_dispatch_success() -> Result<(), ExecutionError> {
//         let mut server = mockito::Server::new();

//         // Use one of these addresses to configure your client
//         let url = server.url();

//         // Create a mock
//         let mock = server
//             .mock("GET", "/")
//             .with_status(200)
//             .with_header("content-type", "text/plain")
//             .with_header("x-api-key", "1234")
//             .with_body("ok")
//             .create();

//         let request = WorkbookRequest {
//             id: String::from(""),
//             name: String::from("test"),
//             url,
//             method: Some(WorkbookRequestMethod::Get),
//             timeout: None,
//             keep_alive: None,
//             runs: 1,
//             headers: None,
//             query_string_params: None,
//             body: None,
//             test: None,
//             selected_scenario: None,
//             selected_authorization: None,
//             selected_certificate: None,
//             selected_proxy: None,
//         };

//         let result = WorkbookRequestEntry::dispatch(&request, &None, &None, &None).await;
//         mock.assert();

//         match result {
//             Ok((_, response)) => {
//                 assert_eq!(response.status, 200);
//                 assert_eq!(response.body.unwrap().text.unwrap(), String::from("ok"));
//                 Ok(())
//             }
//             Err(err) => Err(err),
//         }
//     }

//     // #[test]
//     // fn test_perform_test_success() {
//     //     let request = WorkbookRequest {
//     //         id: String::from(""),
//     //         name: String::from("Test #1"),
//     //         url: String::from("https://foo"),
//     //         method: Some(WorkbookRequestMethod::Get),
//     //         timeout: None,
//     //         body: None,
//     //         headers: None,
//     //         query_string_params: None,
//     //         keep_alive: None,
//     //         test: Some(String::from("describe(\"Status\", () => it(\"equals 200\", () => expect(response.status).to.equal(200)))"))
//     //     };
//     //     let response = ApicizeResponse {
//     //         status: 200,
//     //         status_text: String::from("Ok"),
//     //         headers: None,
//     //         body: None,
//     //         auth_token_cached: None,
//     //     };

//     //     let result = request.execute(&response).unwrap();

//     //     assert_eq!(
//     //         result,
//     //         vec!(ApicizeTestResult {
//     //             test_name: vec![String::from("Status"), String::from("equals 200")],
//     //             success: true,
//     //             error: None,
//     //             logs: None
//     //         })
//     //     );
//     // }

//     // #[test]
//     // fn test_perform_test_fail() {
//     //     let request = WorkbookRequest {
//     //         id: String::from(""),
//     //         name: String::from("Test #1"),
//     //         url: String::from("https://foo"),
//     //         method: Some(WorkbookRequestMethod::Get),
//     //         timeout: None,
//     //         body: None,
//     //         headers: None,
//     //         query_string_params: None,
//     //         keep_alive: None,
//     //         test: Some(String::from("describe(\"Status\", () => it(\"equals 200\", () => expect(response.status).to.equal(200)))"))
//     //     };

//     //     let response = ApicizeResponse {
//     //         status: 404,
//     //         status_text: String::from("Not Found"),
//     //         headers: None,
//     //         body: None,
//     //         auth_token_cached: None,
//     //     };

//     //     let result = request.execute(&response).unwrap();

//     //     assert_eq!(
//     //         result,
//     //         vec!(ApicizeTestResult {
//     //             test_name: vec![String::from("Status"), String::from("equals 200")],
//     //             success: false,
//     //             error: Some(String::from("expected 404 to equal 200")),
//     //             logs: None
//     //         })
//     //     );
//     // }
// }
