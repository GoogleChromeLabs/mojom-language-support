# mojom-lsp

A language server for Chromium mojom files. It supports:
- Syntax check
- Goto definition

This server implements version 3.x of the [language server protocol](https://microsoft.github.io/language-server-protocol/specification).

mojom-lsp is tested on Emacs with [eglot](https://github.com/joaotavora/eglot) and vscode with [mojom-lsp-vscode](./mojom-lsp-vscode) extension.

## Setup

mojom-lsp is at an early stage of development and you need to build it from source. mojom-lsp requires stable Rust to build.

```sh
$ cargo build --release -p mojom-lsp-server
```

The server will be generated as `./target/release/mojom-lsp-server`. Copy the binary into your `$PATH`, or add `./target/release` to your `$PATH`.

### Editor settings

mojom-lsp assumes that your LSP client sends `rootUri` (or `rootPath`) in the `initialize` request. `rootUri` should specifie the `src` directory of your Chromium working directory.

### Syntax highlighting

mojom-lsp doesn't provide syntax highlighting for now. This means that you need to have editor specific configuration to get syntax highlighting.

#### Vim

Use [mojom.vim](https://chromium.googlesource.com/chromium/src.git/+/refs/heads/master/tools/vim/mojom/syntax/mojom.vim) in the chromium repository.

#### Emacs

An easy way to get syntax highliting is to use `define-generic-mode` like below:

```lisp
;; Mojom
(require 'generic)
(define-generic-mode mojom-mode
  ;; comments
  '("//" ("/*" . "*/"))
  ;; keywords
  '("module" "import" "struct" "union" "enum" "interface")
  ;; font-locks
  nil
  ;; auto-mode
  nil
  ;; hooks
  nil
  "Major mode for mojom")
(add-to-list 'auto-mode-alist '("\\.mojom$" . mojom-mode))
```

#### VSCode

Use [mojom-lsp-vscode](./mojom-lsp-vscode) extension.

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
