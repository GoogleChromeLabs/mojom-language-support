# mojom-lsp

A language server for Chromium mojom files. This implements version 3.x of the [language server protocol](https://microsoft.github.io/language-server-protocol/specification).

`mojom-lsp` is at an early stage of development. Currently it supports:
- Syntax check
- Goto definition

## Limitations

- `mojom-lsp` doesn't provide syntax highlighting.
- `mojom-lsp` assumes that the client sends `rootUri` (or `rootPath`) in the `initialize` request and `rootUri` specifies the `src` directory of your Chromium working directory.


## Supported messages

General:

- [ ] `$/cancelRequest`
- [x] `initialize`
- [x] `initialized`
- [x] `shutdown`
- [x] `exit`

Window:

- [ ] `window/showMessage`
- [ ] `window/showMessageRequest`
- [ ] `window/logMessage`

Telemetry:

- [ ] `telemetry/event`

Client:

- [ ] `client/registerCapability`
- [ ] `client/unregisterCapability`

Workspace:

- [ ] `workspace/workspaceFolders`
- [ ] `workspace/didChangeWorkspaceFolders`
- [ ] `workspace/didChangeWorkspaceFolders`
- [ ] `workspace/didChangeConfiguration'`
- [ ] `workspace/configuration`
- [ ] `workspace/didChangeWatchedFiles`
- [ ] `workspace/symbol`
- [ ] `workspace/executeCommand`
- [ ] `workspace/applyEdit`

Text Synchronization:

- [x] `textDocument/didOpen`
- [x] `textDocument/didChange`
- [ ] `textDocument/willSave`
- [ ] `textDocument/willSaveWaitUntil`
- [ ] `textDocument/didSave`
- [ ] `textDocument/didClose`

Diagnostics:

- [x] `textDocument/publishDiagnostics`

Language Features:

- [ ] `textDocument/completion`
- [ ] `completionItem/resolve`
- [ ] `textDocument/hover`
- [ ] `textDocument/signatureHelp`
- [ ] `textDocument/declaration`
- [x] `textDocument/definition`
- [ ] `textDocument/typeDefinition`
- [ ] `textDocument/implementation`
- [ ] `textDocument/references`
- [ ] `textDocument/documentHighlight`
- [ ] `textDocument/documentSymbol`
- [ ] `textDocument/codeAction`
- [ ] `textDocument/codeLens`
- [ ] `codeLens/resolve`
- [ ] `textDocument/documentLink`
- [ ] `documentLink/resolve`
- [ ] `textDocument/documentColor`
- [ ] `textDocument/colorPresentation`
- [ ] `textDocument/formatting`
- [ ] `textDocument/rangeFormatting'`
- [ ] `textDocument/onTypeFormatting`
- [ ] `textDocument/rename`
- [ ] `textDocument/prepareRename`
- [ ] `textDocument/foldingRange`
