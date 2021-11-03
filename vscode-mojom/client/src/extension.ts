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
  State,
} from "vscode-languageclient/node";

const SERVER_COMMAND = "mojom-lsp";

let client: LanguageClient | null = null;
let lspStatusBarItem: vscode.StatusBarItem;

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

  client.onDidChangeState((event) => {
    switch (event.newState) {
      case State.Starting:
        lspStatusBarItem.tooltip = "Starting";
        break;
      case State.Running:
        lspStatusBarItem.tooltip = "Running";
        break;
      case State.Stopped:
        lspStatusBarItem.hide();
        if (client.initializeResult === undefined) {
          // Failed to start the server, update the configuration to disable the server.
          const configuration = vscode.workspace.getConfiguration("mojom");
          configuration.update("enableLanguageServer", "Disabled");
        }
        break;
    }
  });
  client.start();

  if (IsMojomTextEditor(vscode.window.activeTextEditor)) {
    lspStatusBarItem.show();
  }
}

async function stopClient(): Promise<void> {
  if (!client) {
    return;
  }

  const result = client.stop();
  client = null;
  return result;
}

async function hasCommand(command: string): Promise<boolean> {
  return new Promise<boolean>((resolve) => {
    const checkCommand = process.platform === "win32" ? "where" : "command -v";
    const proc = child_process.exec(`${checkCommand} ${command}`);
    proc.on("exit", (code) => {
      resolve(code === 0);
    });
  });
}

async function isLanguageServerIsAvailable(): Promise<boolean> {
  return hasCommand(SERVER_COMMAND);
}

async function isCargoAvaiable(): Promise<boolean> {
  return hasCommand("cargo");
}

async function installServerBinary(): Promise<boolean> {
  const task = new vscode.Task(
    { type: "cargo", task: "install" },
    vscode.workspace.workspaceFolders![0],
    "Installing mojom-lsp",
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
  const hasCargo = await isCargoAvaiable();
  if (!hasCargo) {
    configuration.update("enableLanguageServer", "Disabled");
    return;
  }

  const message = "Install Mojom Language Server? (Rust toolchain required)";
  const selected = await vscode.window.showInformationMessage(
    message,
    "Yes",
    "No",
    "Never"
  );
  if (selected === "Yes") {
    const installed = await installServerBinary();
    if (installed) {
      startClient();
    } else {
      configuration.update("enableLanguageServer", "Disabled");
    }
  } else if (selected === "Never") {
    configuration.update("enableLanguageServer", "Never");
  }
}

async function applyConfigurations() {
  const configuration = vscode.workspace.getConfiguration("mojom");
  const enableLanguageServer = configuration.get<string>("enableLanguageServer");
  const shouldStartClient = (enableLanguageServer === "Enabled") && (await isLanguageServerIsAvailable());
  if (shouldStartClient) {
    startClient();
  } else if (enableLanguageServer === "Enabled") {
    tryToInstallLanguageServer(configuration);
  } else if (enableLanguageServer !== "Enabled") {
    stopClient();
  }
}

function IsMojomTextEditor(editor: vscode.TextEditor | undefined): boolean {
  return editor && editor.document.languageId === "mojom";
}

export async function activate(context: vscode.ExtensionContext) {
  // Set up a status bar item for mojom-lsp.
  lspStatusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left);
  lspStatusBarItem.text = "mojom-lsp";
  lspStatusBarItem.hide();

  // Register Listeners
  const subscriptions = context.subscriptions;
  subscriptions.push(vscode.workspace.onDidChangeConfiguration((event) => {
    if (event.affectsConfiguration("mojom")) {
      applyConfigurations();
    }
  }));
  subscriptions.push(vscode.window.onDidChangeActiveTextEditor((editor) => {
    const shouldShowStatusBarItem = IsMojomTextEditor(editor) && client;
    if (shouldShowStatusBarItem) {
      lspStatusBarItem.show();
    } else {
      lspStatusBarItem.hide();
    }
  }));

  applyConfigurations();
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}
