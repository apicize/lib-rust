# Change Log

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
