import { ExtensionContext } from "vscode";

import {
  LanguageClient,
  LanguageClientOptions,
  Executable
} from "vscode-languageclient";

let client: LanguageClient;

export function activate(context: ExtensionContext) {
  let serverOptions: Executable = {
    command: "mojom-lsp-server"
  };

  let clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "mojom" }]
  };

  client = new LanguageClient(
    "mojomLanguageServer",
    "Mojom Language Server",
    serverOptions,
    clientOptions
  );

  client.start();
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}
