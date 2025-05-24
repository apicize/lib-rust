# Apicize Rust Library

This is a Rust library supporting Apicize serilization, request dispatching (via Reqwest) and test running (via V8).

## Serialization

Broadly speaking, [Workbooks](./src/workbook.rs) and associated structures are how Apicize testing information is persisted in JSON format.  
[Workspaces](./src/workspace.rs) contain indexed views of Workbook structures like requests, scenarios, etc. which make it more efficient
to traverse hierarchical and ordered information.  

### Opening a Workspace from a Workbook

The function `Workspace::open_from_file` will populate a workspace from a a workbook file, its private parameters file, and global
parameters file (if specified).  Entities are indexed and warnings are generated if a workbook contains any references to parameters
that are not found in the private or globals file.

### Saving a Workspace to a Workbook

The function `Workspace::save` persists workspace information to workbook, private parameters and global parameters files.  Private parameters
are saved to a file with the same name as the workbook but with an `.apicize-priv` extension.  Global parameters are saved to the 
user's OS configuration directory under `apicize/globals.json`.

## Executing Tests in a Workspace

Tests are executed via the `test_runner::run` function, which accepts an Arc to the workspace being tested, an optional list of request IDs to execute (defaults to all), an optional
cancellation token, and an Arc to instant that testing was started.

## JavaScript Testing

This library leverages [V8](https://v2.dev) to execute tests to validate requests.  This sandboxed envioronment does not include NodeJS or Browser functionality, primarily to prevent arbitrary test code in a Workbook from doing anything harmful.

The following variables and functions are available in the testing sandbox:

* **request**:  A variable containing the submitted HTTP request
* **response**:  A variable containing the HTTP response
* **scenario**:  A variable containing key-value pairs originally sourced from the active Scenario request parameter (legacy value `variables` is also available)
* **assert**:  An exported function of [Chai's Node assertion style](https://www.chaijs.com/api/assert/)
* **expect** / **should**:  Exported functions of [Chai's BDD assertion style](https://www.chaijs.com/api/bdd/)
* **jsonpath**:  An exported function of [JSONPath Plus](https://www.npmjs.com/package/jsonpath-plus); also added as a `jp` function to JavaScript types
* **output**: Call to output a value and make available to the next request in a group (ex. `output('id', 12345)`)

### Buliding JavaScript Dependencies

The [build.rs](./build.rs) file triggers a copy of the file `test-framework/dist/framework.min.js` file, which is used in the test runner.

If you change `test-framework/index.js` or dependencies, you will need to rebuild `framework.min.js`  To do, run `yarn build` from the `test-framework` directory.