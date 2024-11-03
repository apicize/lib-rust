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

## Executing Tests

Tests are executed via the `TestRunner::run` function, which takes a workspace, a list of request IDs to execute, an optional
cancellation token, and the instant that testing was started.