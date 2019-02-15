use std::io::{self, BufRead, BufReader, BufWriter, Write};

use serde_json::Value;

use crate::protocol::{
    read_message, write_error_response, write_notification, write_success_response,
    write_success_result, ErrorCodes, Message, NotificationMessage, RequestMessage, ResponseError,
};

use crate::{Error, Result};

#[derive(PartialEq)]
enum State {
    Initialized,
    ShuttingDown,
}

struct ServerContext {
    state: State,
    // True when the previous text document has errors.
    has_error: bool,
    // Set when `exit` notification is received.
    exit_code: Option<i32>,
}

impl ServerContext {
    fn new() -> ServerContext {
        ServerContext {
            state: State::Initialized,
            has_error: false,
            exit_code: None,
        }
    }
}

fn create_server_capabilities() -> lsp_types::ServerCapabilities {
    let options = lsp_types::TextDocumentSyncOptions {
        open_close: Some(true),
        change: Some(lsp_types::TextDocumentSyncKind::Full),
        will_save: Some(true),
        will_save_wait_until: Some(false),
        save: None,
    };

    let text_document_sync = lsp_types::TextDocumentSyncCapability::Options(options);

    lsp_types::ServerCapabilities {
        text_document_sync: Some(text_document_sync),
        hover_provider: None,
        completion_provider: None,
        signature_help_provider: None,
        definition_provider: None,
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

// Requests

fn handle_request(
    writer: &mut impl Write,
    ctx: &mut ServerContext,
    msg: RequestMessage,
) -> Result<()> {
    let id = msg.id;
    let method = msg.method.as_str();

    let res = match method {
        "initialize" => initialize_request(),
        "shutdown" => shutdown_request(ctx),
        _ => unimplemented!(),
    };
    match res {
        Ok(res) => write_success_response(writer, id, res)?,
        Err(error) => write_error_response(writer, id, error)?,
    }
    Ok(())
}

type MessageResult<T> = std::result::Result<T, ResponseError>;

fn initialize_request() -> MessageResult<Value> {
    // The server has been initialized already.
    let error_message = "Unexpected initialize message".to_owned();
    Err(ResponseError::new(
        ErrorCodes::ServerNotInitialized,
        error_message,
    ))
}

fn shutdown_request(ctx: &mut ServerContext) -> MessageResult<Value> {
    ctx.state = State::ShuttingDown;
    Ok(Value::Null)
}

// Notifications

fn get_params<P: serde::de::DeserializeOwned>(params: Value) -> Result<P> {
    serde_json::from_value::<P>(params).map_err(|err| Error::ProtocolError(err.to_string()))
}

fn handle_notification(
    writer: &mut impl Write,
    ctx: &mut ServerContext,
    msg: NotificationMessage,
) -> Result<()> {
    let method = msg.method.as_str();
    eprintln!("Got notification: {}", method);

    use lsp_types::notification::*;
    match msg.method.as_str() {
        Exit::METHOD => exit_notification(ctx),
        DidOpenTextDocument::METHOD => {
            get_params(msg.params).and_then(|params| did_open_text_document(writer, ctx, params))
        }
        DidChangeTextDocument::METHOD => {
            get_params(msg.params).and_then(|params| did_change_text_document(writer, ctx, params))
        }
        _ => unimplemented!(),
    }
}

fn publish_diagnotics(
    writer: &mut impl Write,
    params: lsp_types::PublishDiagnosticsParams,
) -> Result<()> {
    // TODO: Don't use unwrap().
    let params = serde_json::to_value(&params).unwrap();
    write_notification(writer, "textDocument/publishDiagnostics", params)
}

fn _check_syntax(
    writer: &mut impl Write,
    ctx: &mut ServerContext,
    uri: lsp_types::Url,
    input: &str,
) -> Result<()> {
    match mojom_parser::parse(input) {
        Ok(_) => {
            if ctx.has_error {
                ctx.has_error = false;
                let params = lsp_types::PublishDiagnosticsParams {
                    uri: uri,
                    diagnostics: vec![],
                };
                publish_diagnotics(writer, params)?;
            }
            Ok(())
        }
        Err(mojom_parser::Error::SyntaxError(span)) => {
            ctx.has_error = true;
            let start = lsp_types::Position {
                line: span.line as u64 - 1,
                character: span.get_column() as u64,
            };
            let end = lsp_types::Position {
                line: span.line as u64 - 1,
                character: (span.get_column() + span.fragment.len()) as u64,
            };
            let range = lsp_types::Range {
                start: start,
                end: end,
            };
            let diagostic = lsp_types::Diagnostic {
                range: range,
                severity: None,
                code: None,
                source: None,
                message: "Syntax error".to_owned(),
                related_information: None,
            };
            let params = lsp_types::PublishDiagnosticsParams {
                uri: uri,
                diagnostics: vec![diagostic],
            };
            publish_diagnotics(writer, params)
        }
    }
}

fn exit_notification(ctx: &mut ServerContext) -> Result<()> {
    // https://microsoft.github.io/language-server-protocol/specification#exit
    if ctx.state == State::ShuttingDown {
        ctx.exit_code = Some(0);
    } else {
        ctx.exit_code = Some(1);
    }
    Ok(())
}

fn did_open_text_document(
    writer: &mut impl Write,
    ctx: &mut ServerContext,
    params: lsp_types::DidOpenTextDocumentParams,
) -> Result<()> {
    let uri = params.text_document.uri.clone();
    _check_syntax(writer, ctx, uri, &params.text_document.text)
}

fn did_change_text_document(
    writer: &mut impl Write,
    ctx: &mut ServerContext,
    params: lsp_types::DidChangeTextDocumentParams,
) -> Result<()> {
    let uri = params.text_document.uri.clone();
    let content = params
        .content_changes
        .iter()
        .map(|i| i.text.to_owned())
        .collect::<Vec<_>>();
    let res = content.join("");
    _check_syntax(writer, ctx, uri, &res)
}

// Initialization

fn initialize(
    reader: &mut impl BufRead,
    writer: &mut impl Write,
) -> Result<lsp_types::InitializeParams> {
    let message = read_message(reader)?;

    let (id, params) = match message {
        Message::Request(req) => req.cast::<lsp_types::request::Initialize>()?,
        _ => {
            // TODO: Gracefully handle `exit` and `shutdown` messages.
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
        Message::Notofication(notif) => notif.cast::<lsp_types::notification::Initialized>()?,
        _ => {
            let error_message = format!("Expected initialized message but got {:?}", message);
            return Err(Error::ProtocolError(error_message));
        }
    };

    Ok(params)
}

// Returns exit code.
pub fn start() -> Result<i32> {
    let mut reader = BufReader::new(io::stdin());
    let mut writer = BufWriter::new(io::stdout());

    let _params = initialize(&mut reader, &mut writer)?;

    let mut ctx = ServerContext::new();

    loop {
        eprintln!("Reading message...");
        let message = read_message(&mut reader)?;
        match message {
            Message::Request(request) => handle_request(&mut writer, &mut ctx, request)?,
            Message::Notofication(notification) => {
                handle_notification(&mut writer, &mut ctx, notification)?
            }
            _ => unimplemented!(),
        };

        if let Some(exit_code) = ctx.exit_code {
            return Ok(exit_code);
        }
    }
}
