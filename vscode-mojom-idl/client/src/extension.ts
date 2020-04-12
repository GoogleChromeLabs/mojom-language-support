/**
 * Copyright 2020 Google LLC
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

import * as child_process from "child_process";

import * as vscode from "vscode";

import {
  LanguageClient,
  LanguageClientOptions,
  Executable,
} from "vscode-languageclient";

const SERVER_COMMAND = "mojom-lsp-server";

let client: LanguageClient;

function startClient() {
  let serverOptions: Executable = {
    command: SERVER_COMMAND,
  };

  let clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "mojom" }],
  };

  client = new LanguageClient(
    "mojomLanguageServer",
    "Mojom Language Server",
    serverOptions,
    clientOptions
  );

  client.start();
}

async function hasServerBinary(): Promise<boolean> {
  return new Promise<boolean>((resolve) => {
    const checkCommand = process.platform === "win32" ? "where" : "command -v";
    const proc = child_process.exec(`${checkCommand} ${SERVER_COMMAND}`);
    proc.on("exit", (code) => {
      resolve(code === 0);
    });
  });
}

async function installServerBinary(): Promise<boolean> {
  const task = new vscode.Task(
    { type: "cargo", task: "install" },
    vscode.workspace.workspaceFolders![0],
    "Installing lsp server",
    "mojom-lsp",
    new vscode.ShellExecution("cargo install mojom-lsp")
  );
  const promise = new Promise<boolean>((resolve) => {
    vscode.tasks.onDidEndTask((e) => {
      if (e.execution.task === task) {
        e.execution.terminate();
      }
    });
    vscode.tasks.onDidEndTaskProcess((e) => {
      resolve(e.exitCode === 0);
    });
  });
  vscode.tasks.executeTask(task);

  return promise;
}

async function tryToInstallLanguageServer(
  configuration: vscode.WorkspaceConfiguration
) {
  const selected = await vscode.window.showInformationMessage(
    "Install mojom-lsp-server (Rust toolchain required) ?",
    "Install",
    "Never"
  );
  if (selected === "Install") {
    const installed = await installServerBinary();
    if (installed) {
      startClient();
    }
  } else if (selected === "Never") {
    configuration.update("useLanguageServer", false);
  }
}

export async function activate(context: vscode.ExtensionContext) {
  const configuration = vscode.workspace.getConfiguration("mojom");
  const useLanguageServer = configuration.get<boolean>("useLanguageServer");
  const shouldStartClient = useLanguageServer && (await hasServerBinary());
  if (shouldStartClient) {
    startClient();
  } else if (useLanguageServer) {
    tryToInstallLanguageServer(configuration);
  }
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}
