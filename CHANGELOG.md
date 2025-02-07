# Change Log

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
