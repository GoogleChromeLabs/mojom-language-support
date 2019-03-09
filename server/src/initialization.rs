use std::io::{BufRead, Write};

use crate::protocol::{read_message, write_success_result, Message};

use crate::server::Error;

fn create_server_capabilities() -> lsp_types::ServerCapabilities {
    let options = lsp_types::TextDocumentSyncOptions {
        open_close: Some(true),
        change: Some(lsp_types::TextDocumentSyncKind::Full),
        will_save: None,
        will_save_wait_until: None,
        save: None,
    };

    let text_document_sync = lsp_types::TextDocumentSyncCapability::Options(options);

    lsp_types::ServerCapabilities {
        text_document_sync: Some(text_document_sync),
        hover_provider: None,
        completion_provider: None,
        signature_help_provider: None,
        definition_provider: Some(true),
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
        color_provider: None,
        folding_range_provider: None,
        execute_command_provider: None,
        workspace: None,
    }
}

pub(crate) fn initialize(
    reader: &mut impl BufRead,
    writer: &mut impl Write,
) -> std::result::Result<lsp_types::InitializeParams, Error> {
    use lsp_types::notification::Notification;
    use lsp_types::request::Request;

    let message = read_message(reader)?;
    let (id, params) = match message {
        Message::Request(req) => {
            if req.method != lsp_types::request::Initialize::METHOD {
                let error_message = format!("Expected initialize message but got {:?}", req.method);
                return Err(Error::ProtocolError(error_message));
            }
            let params = serde_json::from_value::<lsp_types::InitializeParams>(req.params)
                .map_err(|err| Error::ProtocolError(err.to_string()))?;
            (req.id, params)
        }
        _ => {
            let error_message = format!("Expected initialize message but got {:?}", message);
            return Err(Error::ProtocolError(error_message));
        }
    };

    let capabilities = create_server_capabilities();
    let res = lsp_types::InitializeResult {
        capabilities: capabilities,
    };
    write_success_result(writer, id, res)?;

    let message = read_message(reader)?;
    match message {
        Message::Notofication(notif) => {
            if notif.method != lsp_types::notification::Initialized::METHOD {
                let error_message =
                    format!("Expected initialized message but got {:?}", notif.method);
                return Err(Error::ProtocolError(error_message));
            }
        }
        _ => {
            let error_message = format!("Expected initialized message but got {:?}", message);
            return Err(Error::ProtocolError(error_message));
        }
    };

    Ok(params)
}
