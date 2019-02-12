use std::io::{self, BufReader, BufWriter, Write};

use serde_json::Value;

use crate::protocol::{
    read_message, write_error_response, write_success_response, ErrorCodes, Message,
    NotificationMessage, RequestMessage, ResponseError,
};

use crate::Result;

#[derive(PartialEq)]
enum State {
    Initializing,
    Initialized,
    ShuttingDown,
}

struct ServerContext {
    state: State,
    // Set when `exit` notification is received.
    exit_code: Option<i32>,
}

impl ServerContext {
    fn new() -> ServerContext {
        ServerContext {
            state: State::Initializing,
            exit_code: None,
        }
    }
}

type MessageResult<T> = std::result::Result<T, ResponseError>;

// Request handlers

// https://microsoft.github.io/language-server-protocol/specification#initialize
fn initialize(ctx: &mut ServerContext, msg: RequestMessage) -> MessageResult<Value> {
    if ctx.state != State::Initializing {
        let error_message = "The server has already initialized".to_owned();
        return Err(ResponseError::new(
            ErrorCodes::InvalidRequest,
            error_message,
        ));
    }

    let params = serde_json::from_value::<lsp_types::InitializeParams>(msg.params);
    let params = params.map_err(|err| {
        let error_message = err.to_string();
        ResponseError::new(ErrorCodes::ParseError, error_message)
    })?;

    eprintln!("{:?}", params.process_id);

    let res = lsp_types::InitializeResult {
        capabilities: create_server_capabilities(),
    };

    let res = serde_json::to_value(&res).map_err(|err| {
        let error_message = err.to_string();
        ResponseError::new(ErrorCodes::InternalError, error_message)
    })?;

    ctx.state = State::Initialized;

    Ok(res)
}

// https://microsoft.github.io/language-server-protocol/specification#shutdown
fn shutdown(ctx: &mut ServerContext) -> MessageResult<Value> {
    ctx.state = State::ShuttingDown;
    Ok(Value::Null)
}

// Notification handlers

fn initialized(_ctx: &mut ServerContext) -> MessageResult<()> {
    Ok(())
}

// ---

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

fn handle_request(
    writer: &mut impl Write,
    ctx: &mut ServerContext,
    msg: RequestMessage,
) -> Result<()> {
    let id = msg.id;
    let method = msg.method.as_str();

    // Send an error when not initialized as the spec says.
    if ctx.state == State::Initializing && method != "initialize" {
        let error_message = "Server not initialized".to_owned();
        let error = ResponseError::new(ErrorCodes::ServerNotInitialized, error_message);
        return write_error_response(writer, id, error);
    }

    let response = match method {
        "initialize" => initialize(ctx, msg),
        "shutdown" => shutdown(ctx),
        _ => unimplemented!(),
    };

    match response {
        Ok(result) => write_success_response(writer, id, result)?,
        Err(error) => write_error_response(writer, id, error)?,
    }
    Ok(())
}

fn handle_notification(
    _write: &mut impl Write,
    ctx: &mut ServerContext,
    msg: NotificationMessage,
) -> Result<()> {
    let method = msg.method.as_str();

    eprintln!("Got notification: {}", method);

    if method == "exit" {
        // https://microsoft.github.io/language-server-protocol/specification#exit
        if ctx.state == State::ShuttingDown {
            ctx.exit_code = Some(0);
        } else {
            ctx.exit_code = Some(1);
        }
        return Ok(());
    }

    // Drop notifications when not initialized as the spec says.
    if ctx.state == State::Initializing {
        return Ok(());
    }

    let result = match method {
        "initialized" => initialized(ctx),
        _ => unimplemented!(),
    };

    if result.is_err() {
        // Ignore errors as notifications must not send a response back.
        // TODO: Log errors and/or update server state.
    }

    Ok(())
}

// Returns exit code.
pub fn start() -> Result<i32> {
    let mut reader = BufReader::new(io::stdin());
    let mut writer = BufWriter::new(io::stdout());
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
