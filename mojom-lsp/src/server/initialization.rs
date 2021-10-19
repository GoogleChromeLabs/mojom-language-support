// Copyright 2020 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::io::{BufRead, Write};

use anyhow::anyhow;

use super::protocol::{read_message, write_success_result, Message};

fn create_server_capabilities() -> lsp_types::ServerCapabilities {
    let options = lsp_types::TextDocumentSyncOptions {
        open_close: Some(true),
        change: Some(lsp_types::TextDocumentSyncKind::FULL),
        will_save: None,
        will_save_wait_until: None,
        save: None,
    };

    let text_document_sync = lsp_types::TextDocumentSyncCapability::Options(options);

    // TODO: Understand each field and avoid using None if applicable.
    lsp_types::ServerCapabilities {
        text_document_sync: Some(text_document_sync),
        selection_range_provider: None,
        hover_provider: None,
        completion_provider: None,
        signature_help_provider: None,
        definition_provider: Some(lsp_types::OneOf::Left(true)),
        type_definition_provider: None,
        implementation_provider: None,
        references_provider: None,
        document_highlight_provider: None,
        document_symbol_provider: None,
        workspace_symbol_provider: None,
        code_action_provider: None,
        code_lens_provider: None,
        document_formatting_provider: None,
        document_range_formatting_provider: None,
        document_on_type_formatting_provider: None,
        rename_provider: None,
        document_link_provider: None,
        color_provider: None,
        folding_range_provider: None,
        declaration_provider: Some(lsp_types::DeclarationCapability::Simple(true)),
        execute_command_provider: None,
        workspace: None,
        experimental: None,
        call_hierarchy_provider: None,
        semantic_tokens_provider: None,
        moniker_provider: None,
        linked_editing_range_provider: None,
    }
}

pub(crate) fn initialize(
    reader: &mut impl BufRead,
    writer: &mut impl Write,
) -> anyhow::Result<lsp_types::InitializeParams> {
    use lsp_types::notification::Notification;
    use lsp_types::request::Request;

    let message = read_message(reader)?;
    let (id, params) = match message {
        Message::Request(req) => {
            if req.method != lsp_types::request::Initialize::METHOD {
                let error_message = anyhow!("Expected initialize message but got {:?}", req.method);
                return Err(error_message);
            }
            let params = serde_json::from_value::<lsp_types::InitializeParams>(req.params)?;
            (req.id, params)
        }
        _ => {
            let error_message = anyhow!("Expected initialize message but got {:?}", message);
            return Err(error_message);
        }
    };

    let capabilities = create_server_capabilities();
    let res = lsp_types::InitializeResult {
        capabilities: capabilities,
        server_info: Some(lsp_types::ServerInfo {
            name: "mojom-lsp".to_string(),
            version: Some("0.1.0".to_string()),
        }),
    };
    write_success_result(writer, id, res)?;

    let message = read_message(reader)?;
    match message {
        Message::Notofication(notif) => {
            if notif.method != lsp_types::notification::Initialized::METHOD {
                let error_message =
                    anyhow!("Expected initialized message but got {:?}", notif.method);
                return Err(error_message);
            }
        }
        _ => {
            let error_message = anyhow!("Expected initialized message but got {:?}", message);
            return Err(error_message);
        }
    };

    Ok(params)
}
