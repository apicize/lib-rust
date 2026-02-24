# Change Log

# 0.33.0

* Performance optimization

# 0.32.1

* Performance optimization (parent index for indexed_entities)

# 0.32.0

* Add group setup scripts
* Optimize V8 setup

# 0.31.0

* Add disabled flag to requests/groups

# 0.30.9

* Performance optimizations in timeout handling and workspace generation

# 0.30.8

* Fix assignment of run exec ctr
* Validate default parameters upon load and set warnings

# 0.30.7

* Move ExecutionResultBuilder back to lib

# 0.30.6

* Fix PATCH calls
* Standardize data set source error handling
* Remove warnings from stored requests

# 0.30.5

* Re-implement validation (warnings and errors)
* Refactor persistent parameter file access
* Fix embedding of quotation marks for JSON bodies

# 0.30.4

* Set children to None when serializing/deserializing empty result children lists
* Serialize raw body as Base64

# 0.30.3

* Performance improvements

# 0.30.2

* Limit setting tcp_user_timeout to Linux

# 0.30.1

* Fix certificate/proxy selection for authorizations
* Improve max timeout handling
* Index execution results by exec_ctr

# 0.30.0

* Standardize validation state storage (for later refinement)
* Store rolled-up execution state

# 0.29.0

* Clean-up of error enum
* Move overriden defaults for parameters to Workspace::Open

# 0.28.1

* Support setting request and group runs to zero

# 0.28.0

* Return Cancellation error on all cancelled executions

# 0.27.0

* Remove Zephyr test report format (BREAKING)
* Some memory optimization, mostly preallocations 

# 0.26.1

* Refactor multi-run reporting to "flatten" results

# 0.26.0

* Support Key property for requests and groups, add to reporting
* Separate execution result assembly into ExecutionResultBuilder

# 0.25.6

* Fix Zephyr report to say "Passed" instead of "Success"

# 0.25.5

* Throw error if invalid property specified in tag handlebar

# 0.25.4

* Allow handlebars in tag names to pull in values (data, scenario, output)

# 0.25.3

* Update Zephyr output to only include tagged entries

# 0.25.2

* Include method and URL to CSV reporting

# 0.25.1

* Include method in execution results

# 0.25.0

* Add "tag" test function to store tag with executed tests
* Include URL and tag in execution results

# 0.24.5

* Update jp function to always return array (even if single result)

# 0.24.4

* Test framework: Add body type and refine $ values

# 0.24.3

* Reorg cache methods, add get cached token method for PKCE flow

# 0.24.2

* Restore serialization for default runs and redirects

# 0.24.1

* Restore serialization for concurrency default values

# 0.24.0

* Support keep alive, allow invalid certs and number of redirect options

# 0.23.4

* Add substitution to test text

# 0.23.3

* Fix populating form data

# 0.23.2

* Fix mult-run reporting

# 0.23.1

* Add multi-run reporting functionality

# 0.23.0

* Add CSV reporting output

# 0.22.0

* Add validation errors and restore warning (parameter selection) functionality

# 0.21.2

* Deprecate "variables" in place of unambigous "scenario" variable (retain "variables" for backward compatibility)

# 0.21.1

* Changed "entries" to "results" in groups

# 0.20.0

* Significant refactor to support data assignment at request/group level
* Reorganize test context variables, data and output, with (hopefully) better consistency in how values are passed between requests and children

# 0.19.10

* Update Request default to include a status 200 test

# 0.19.9

* whoops - typo promoted 0.19.0 to 0.19.9...

# 0.19.0

* Return test results as a hierarchy of scenarios/behaviors (BREAKING CHANGE)

# 0.18.3

* Rename input_variables to variables in execution result

# 0.18.2

* Standardize request/response body info so that `text` is the data that is sent and `data` is a parsed value
* Improve XML support

# 0.18.1

* Change order of variable preference to scenario variables (primary), then data, and then last call variables
* Set up "editable" feature to isolate functionality only used for interactive editing

# 0.18.0

* Use Rust to store workspace status

# 0.17.2

* Fix timeout handling
* Update JSON body storage to store string for non-JSON serializable content (ex. when using handlebar scenario vars for insertion)

# 0.17.1

* Add Audience and Send Credentials in Body to OAuth2

# 0.17.0

* BREAKING CHANGE - add support for seed data (i.e. data rows)
* Update result structure to accomodate rows with multiple runs
* Refactor execution to accomodate multiple rows and runs

# 0.16.2 / 0.16.3

* Store JSON data files in "pretty" format

# 0.16.1

* Add default implementation to WorkbookDefaultParameters

# 0.16.0

* Add support for reqwest trace logging

# 0.15.2

* Update Workspace representation of Parameters to be hierarchical by persistence (public, private, global)

# 0.15.1

* Add PKCE Port to settings

# 0.15.0

* Add support for PKCE

# 0.14.1

* Improve diagnostic info for OAuth2 token requests

# 0.14.0

* Added Workspace::new to load globals when creating new workspace

# 0.13.3

* Fix warnings on NO_SELECTION_ID 

# 0.13.2

* Added ApicizeExecutionRequestRun::input_variables

# 0.13.1

* Added Clone to ApicizeSettings

# 0.13.0

* Added recent workbook file names to settings

# 0.12.0

* Re-introduced optional number of runs override argument for run function

# 0.11.3

* Move test-framework.js build back out of Cargo build, move prebuilt file using Cargo build

# 0.11.2

* Switch to yarn for JavaScript build

# 0.11.1

* Improved child error rendering

# 0.11.0

* Rework errors to be easier to serialize and format

# 0.10.1

* Implement Serialize for ExecutionError

# 0.10.0

* Add JSONPath and XPath libraries for Javascript testing
* Trigger Webpack from Cargo build

# 0.9.3

* Bug Fixes

# 0.9.2

* Move run back under TestRunner

# 0.9.1

* Move run to Workspace, restore orig oauth2 client tokens

# 0.9.0

* Added initial unt testing

# 0.8.4

* Migrated from Apicize monorepo
