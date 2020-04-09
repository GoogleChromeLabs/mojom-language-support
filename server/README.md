# mojom-lsp-server

A [language server](https://microsoft.github.io/language-server-protocol/specification) for Mojom IDL. It supports:

- Syntax check
- Goto definition

mojom-lsp is tested on Visual Studio Code with [vscode-mojom-idl](../vscode-mojom-idl/README.md) extension and Emacs with [eglot](https://github.com/joaotavora/eglot).

## Setup

mojom-lsp is at an early stage of development and you need to build it from source. mojom-lsp requires stable Rust to build.

```sh
$ cargo build --release -p mojom-lsp-server
```

The server will be generated as `target/release/mojom-lsp-server`. Copy the binary into your `$PATH`, or add `target/release` to your `$PATH`.

## Editor settings

mojom-lsp assumes that your LSP client sends `rootUri` in the `initialize` request. `rootUri` should be a path that contains the `src` directory of your Chromium working directory.

## Syntax highlighting

mojom-lsp itself doesn't provide syntax highlighting for now. You need to configure your editor to get syntax highlighting.

### VSCode

Use [vscode-mojom-idl](../vscode-mojom-idl) extension.

### Vim

The Chromium repository provides basic [mojom](https://chromium.googlesource.com/chromium/src.git/+/refs/heads/master/tools/vim/mojom/) support.

### Emacs

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
