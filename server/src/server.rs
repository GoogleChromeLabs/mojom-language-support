use std::io::{self, BufReader, BufWriter};
use std::path::PathBuf;

use serde_json::Value;

use crate::protocol::{
    self, read_message, ErrorCodes, Message, NotificationMessage, RequestMessage, ResponseError,
};

use crate::diagnostic::{start_diagnostics_thread, DiagnosticsThread};
use crate::messagesender::{start_message_sender_thread, MessageSender};

#[derive(Debug)]
pub enum Error {
    IoError(std::io::Error),
    ProtocolError(String),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::IoError(err)
    }
}

impl From<protocol::Error> for Error {
    fn from(err: protocol::Error) -> Error {
        match err {
            protocol::Error::ProtocolError(msg) => Error::ProtocolError(msg),
        }
    }
}

#[derive(PartialEq)]
enum State {
    Initialized,
    ShuttingDown,
}

struct ServerContext {
    state: State,
    // A handler to the diagnostics thread.
    diag: DiagnosticsThread,
    // Set when `exit` notification is received.
    exit_code: Option<i32>,
}

impl ServerContext {
    fn new(diag: DiagnosticsThread) -> ServerContext {
        ServerContext {
            state: State::Initialized,
            diag: diag,
            exit_code: None,
        }
    }
}

// Requests

fn get_request_params<P: serde::de::DeserializeOwned>(
    params: Value,
) -> std::result::Result<P, ResponseError> {
    serde_json::from_value::<P>(params)
        .map_err(|err| ResponseError::new(ErrorCodes::InvalidRequest, err.to_string()))
}

fn handle_request(
    ctx: &mut ServerContext,
    msg_sender: MessageSender,
    msg: RequestMessage,
) -> std::result::Result<(), Error> {
    let id = msg.id;
    let method = msg.method.as_str();

    // Workaround for Eglot. It sends "exit" as a request, not as a notification.
    if method == "exit" {
        return exit_notification(ctx);
    }

    use lsp_types::request::*;
    let res = match method {
        Initialize::METHOD => initialize_request(),
        Shutdown::METHOD => shutdown_request(ctx),
        GotoDefinition::METHOD => get_request_params(msg.params)
            .and_then(|params| goto_definition_request(&mut ctx.diag, params)),
        _ => unimplemented_request(id, method),
    };
    match res {
        Ok(res) => {
            msg_sender.send_success_response(id, res);
        }
        Err(err) => msg_sender.send_error_response(id, err),
    };
    Ok(())
}

fn unimplemented_request(id: u64, method_name: &str) -> std::result::Result<Value, ResponseError> {
    let msg = format!(
        "Unimplemented request: id = {} method = {}",
        id, method_name
    );
    let err = ResponseError::new(ErrorCodes::InternalError, msg);
    Err(err)
}

type RequestResult = std::result::Result<Value, ResponseError>;

fn initialize_request() -> RequestResult {
    // The server has been initialized already.
    let error_message = "Unexpected initialize message".to_owned();
    Err(ResponseError::new(
        ErrorCodes::ServerNotInitialized,
        error_message,
    ))
}

fn shutdown_request(ctx: &mut ServerContext) -> RequestResult {
    ctx.state = State::ShuttingDown;
    Ok(Value::Null)
}

fn goto_definition_request(
    diag: &mut DiagnosticsThread,
    params: lsp_types::TextDocumentPositionParams,
) -> RequestResult {
    if let Some(loc) = diag.goto_definition(params.text_document.uri, params.position) {
        let res = serde_json::to_value(loc).unwrap();
        return Ok(res);
    }
    return Ok(Value::Null);
}

// Notifications

fn get_params<P: serde::de::DeserializeOwned>(params: Value) -> std::result::Result<P, Error> {
    serde_json::from_value::<P>(params).map_err(|err| Error::ProtocolError(err.to_string()))
}

fn do_nothing(msg: &NotificationMessage) -> std::result::Result<(), Error> {
    eprintln!("Received `{}` but do nothing.", msg.method.as_str());
    Ok(())
}

fn handle_notification(
    ctx: &mut ServerContext,
    msg: NotificationMessage,
) -> std::result::Result<(), Error> {
    let method = msg.method.as_str();
    eprintln!("Got notification: {}", method);

    use lsp_types::notification::*;
    match msg.method.as_str() {
        Exit::METHOD => exit_notification(ctx),
        DidOpenTextDocument::METHOD => {
            get_params(msg.params).and_then(|params| did_open_text_document(ctx, params))
        }
        DidChangeTextDocument::METHOD => {
            get_params(msg.params).and_then(|params| did_change_text_document(ctx, params))
        }
        // Accept following notifications but do nothing.
        DidChangeConfiguration::METHOD => do_nothing(&msg),
        WillSaveTextDocument::METHOD => do_nothing(&msg),
        DidSaveTextDocument::METHOD => do_nothing(&msg),
        _ => {
            eprintln!("Received unimplemented notification: {:#?}", msg);
            Ok(())
        }
    }
}

fn exit_notification(ctx: &mut ServerContext) -> std::result::Result<(), Error> {
    // https://microsoft.github.io/language-server-protocol/specification#exit
    if ctx.state == State::ShuttingDown {
        ctx.exit_code = Some(0);
    } else {
        ctx.exit_code = Some(1);
    }
    Ok(())
}

fn did_open_text_document(
    ctx: &mut ServerContext,
    params: lsp_types::DidOpenTextDocumentParams,
) -> std::result::Result<(), Error> {
    ctx.diag
        .check(params.text_document.uri, params.text_document.text);
    Ok(())
}

fn did_change_text_document(
    ctx: &mut ServerContext,
    params: lsp_types::DidChangeTextDocumentParams,
) -> std::result::Result<(), Error> {
    let uri = params.text_document.uri.clone();
    let content = params
        .content_changes
        .iter()
        .map(|i| i.text.to_owned())
        .collect::<Vec<_>>();
    let text = content.join("");
    ctx.diag.check(uri, text);
    Ok(())
}

fn get_root_path(params: &lsp_types::InitializeParams) -> PathBuf {
    if let Some(ref uri) = params.root_uri.as_ref() {
        if uri.scheme() == "file" {
            if let Ok(path) = uri.to_file_path() {
                return path;
            }
        }
    }
    if let Some(ref path) = params.root_path.as_ref() {
        return PathBuf::from(path);
    }
    PathBuf::new()
}

// Returns exit code.
pub fn start() -> std::result::Result<i32, Error> {
    let mut reader = BufReader::new(io::stdin());
    let mut writer = BufWriter::new(io::stdout());

    let params = crate::initialization::initialize(&mut reader, &mut writer)?;

    let root_path = get_root_path(&params);
    eprintln!("root_path: {:?}", root_path);

    let msg_sender_thread = start_message_sender_thread(writer);
    let diag = start_diagnostics_thread(root_path, msg_sender_thread.get_sender());

    let mut ctx = ServerContext::new(diag);
    loop {
        eprintln!("Reading message...");
        let message = read_message(&mut reader)?;
        let msg_sender = msg_sender_thread.get_sender();
        match message {
            Message::Request(request) => handle_request(&mut ctx, msg_sender, request)?,
            Message::Notofication(notification) => handle_notification(&mut ctx, notification)?,
            // TODO: Send a protocol error?
            _ => unimplemented!(),
        };

        if let Some(exit_code) = ctx.exit_code {
            return Ok(exit_code);
        }
    }
}
