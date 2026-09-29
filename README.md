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
* **btoa** / **atob**:  Browser-style base64 encoding/decoding of binary (Latin-1) strings
* **base64**:  `base64.encode(value)` encodes a string (as UTF-8), array, typed array or `ArrayBuffer`;  `base64.decode(value)` returns a `Uint8Array`;  `base64.decodeText(value)` returns a UTF-8 string

### Setup Scripts

Requests and groups can define an optional `setup` script that runs before execution.  Setup scripts run in a separate sandbox that does not include Chai or test functions (`describe`, `it`, `tag`), since no tests are executed.

For requests, the setup script receives the request definition *before* `{{variable}}` substitution and can update it.  Substitution is applied afterwards, including any values set via `output`.  The setup script itself is not substituted (use `$` to access values).  If the setup script throws an error, the request is not dispatched and a `FailedSetup` error is recorded.

The following variables and functions are available in the setup sandbox:

* **request**:  The request definition (`url`, `method`, `headers`, `queryStringParams`, `body`), which can be modified or replaced (setting it to anything other than an object is an error);  `null` for group setup scripts
  * `headers` and `queryStringParams` are arrays of `{ name, value, disabled }` (a `{ name: value }` object is also accepted)
  * `body` is `{ type, data }` where `type` is one of `BodyType` (`Text`, `JSON`, `XML`, `GraphQL`, `Form`, `Raw`);  JSON `data` can be assigned an object and will be serialized;  Raw `data` is base64, or can be assigned an array, typed array or `ArrayBuffer`
  * Helpers: `request.setHeader(name, value)`, `request.removeHeader(name)` (case-insensitive), `request.setQueryParam(name, value)`, `request.removeQueryParam(name)`
* **scenario**, **data**, **$**:  Same as in tests
* **jsonpath**:  Same as in tests
* **output**:  Call to output a value, making it available for `{{variable}}` substitution in the request, to the request's test and to the next request in a group
* **console**:  Log functions, logs are included with request execution logs
* **btoa** / **atob** / **base64**:  Same as in tests

### Variable Precedence

When the same name is defined in more than one place, values are resolved in the following order (later wins), both for `{{variable}}` substitution and for `$` in scripts:

1. Scenario variables
2. Output values (from `output` calls in previous requests, group setup or the request's setup)
3. Data set row values

For example, when requests in a group run sequentially with a scenario selected, each request receives the scenario's values;  if the first request calls `output` with the name of a scenario variable, the second and subsequent requests receive the output value instead.

Example:

```js
request.setHeader('X-Request-Time', `${Date.now()}`)
const body = JSON.parse(request.body.data)
body.nonce = Math.random().toString(36).slice(2)
request.body.data = body
output('nonce', body.nonce)
```

### Buliding JavaScript Dependencies

The [build.rs](./build.rs) file triggers a copy of the files `script-frameworks/test/dist/framework.min.js` and `script-frameworks/setup/dist/setup.min.js`, which are used in the test runner.

If you change `script-frameworks/test/index.js`, `script-frameworks/setup/index.js` or their dependencies, you will need to rebuild the minified files.  To do, run `yarn build` from the `script-frameworks/test` or `script-frameworks/setup` directory.