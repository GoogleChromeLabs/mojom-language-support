use std::io::{BufReader, BufWriter, Read, Write};
use std::path::PathBuf;

use serde_json::Value;

use crate::protocol::{
    self, read_message, ErrorCodes, Message, NotificationMessage, RequestMessage, ResponseError,
};

use crate::diagnostic::{start_diagnostics_thread, DiagnosticsThread};
use crate::messagesender::{start_message_sender_thread, MessageSender};

#[derive(Debug)]
pub enum ServerError {
    ProtocolError(String),
}

impl From<protocol::ProtocolError> for ServerError {
    fn from(err: protocol::ProtocolError) -> ServerError {
        ServerError::ProtocolError(err.0)
    }
}

impl From<serde_json::Error> for ServerError {
    fn from(err: serde_json::Error) -> ServerError {
        let msg = format!("Invalid json message: {}", err);
        ServerError::ProtocolError(msg)
    }
}

#[derive(PartialEq)]
enum State {
    Initialized,
    ShuttingDown,
}

struct ServerContext {
    state: State,
    // A handler to send messages on the main thread.
    msg_sender: MessageSender,
    // A handler to the diagnostics thread.
    diag: DiagnosticsThread,
    // Set when `exit` notification is received.
    exit_code: Option<i32>,
}

impl ServerContext {
    fn new(msg_sender: MessageSender, diag: DiagnosticsThread) -> ServerContext {
        ServerContext {
            state: State::Initialized,
            msg_sender: msg_sender,
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
    msg: RequestMessage,
) -> std::result::Result<(), ServerError> {
    let id = msg.id;
    let method = msg.method.as_str();

    // Workaround for Eglot. It sends "exit" as a request, not as a notification.
    if method == "exit" {
        exit_notification(ctx);
        return Ok(());
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
            ctx.msg_sender.send_success_response(id, res);
        }
        Err(err) => ctx.msg_sender.send_error_response(id, err),
    };
    Ok(())
}

type RequestResult = std::result::Result<Value, ResponseError>;

fn unimplemented_request(id: u64, method_name: &str) -> RequestResult {
    let msg = format!(
        "Unimplemented request: id = {} method = {}",
        id, method_name
    );
    let err = ResponseError::new(ErrorCodes::InternalError, msg);
    Err(err)
}

fn initialize_request() -> RequestResult {
    // The server was already initialized.
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

fn get_params<P: serde::de::DeserializeOwned>(
    params: Value,
) -> std::result::Result<P, ServerError> {
    serde_json::from_value::<P>(params).map_err(|err| ServerError::ProtocolError(err.to_string()))
}

fn handle_notification(
    ctx: &mut ServerContext,
    msg: NotificationMessage,
) -> std::result::Result<(), ServerError> {
    use lsp_types::notification::*;
    match msg.method.as_str() {
        Exit::METHOD => exit_notification(ctx),
        DidOpenTextDocument::METHOD => {
            get_params(msg.params).map(|params| did_open_text_document(ctx, params))?;
        }
        DidChangeTextDocument::METHOD => {
            get_params(msg.params).map(|params| did_change_text_document(ctx, params))?;
        }
        // Accept following notifications but do nothing.
        DidChangeConfiguration::METHOD => do_nothing(&msg),
        WillSaveTextDocument::METHOD => do_nothing(&msg),
        DidSaveTextDocument::METHOD => do_nothing(&msg),
        _ => {
            eprintln!("Received unimplemented notification: {:#?}", msg);
        }
    }
    Ok(())
}

fn do_nothing(msg: &NotificationMessage) {
    eprintln!("Received `{}` but do nothing.", msg.method.as_str());
}

fn exit_notification(ctx: &mut ServerContext) {
    // https://microsoft.github.io/language-server-protocol/specification#exit
    if ctx.state == State::ShuttingDown {
        ctx.exit_code = Some(0);
    } else {
        ctx.exit_code = Some(1);
    }
}

fn did_open_text_document(ctx: &mut ServerContext, params: lsp_types::DidOpenTextDocumentParams) {
    ctx.diag
        .check(params.text_document.uri, params.text_document.text);
}

fn did_change_text_document(
    ctx: &mut ServerContext,
    params: lsp_types::DidChangeTextDocumentParams,
) {
    let uri = params.text_document.uri.clone();
    let content = params
        .content_changes
        .iter()
        .map(|i| i.text.to_owned())
        .collect::<Vec<_>>();
    let text = content.join("");
    ctx.diag.check(uri, text);
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
pub fn start<R, W>(reader: R, writer: W) -> std::result::Result<i32, ServerError>
where
    R: Read,
    W: Write + Send + 'static,
{
    let mut reader = BufReader::new(reader);
    let mut writer = BufWriter::new(writer);

    let params = crate::initialization::initialize(&mut reader, &mut writer)?;

    let root_path = get_root_path(&params);

    let msg_sender_thread = start_message_sender_thread(writer);
    let diag = start_diagnostics_thread(root_path, msg_sender_thread.get_sender());

    let mut ctx = ServerContext::new(msg_sender_thread.get_sender(), diag);
    loop {
        let message = read_message(&mut reader)?;
        match message {
            Message::Request(request) => handle_request(&mut ctx, request)?,
            Message::Notofication(notification) => handle_notification(&mut ctx, notification)?,
            _ => unreachable!(),
        };

        if let Some(exit_code) = ctx.exit_code {
            return Ok(exit_code);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{read_message, write_notification, write_request};

    use lsp_types::notification::*;
    use lsp_types::request::*;
    use pipe::pipe;

    #[test]
    fn test_server_init() {
        let (reader, mut writer) = pipe();

        let capabilities = lsp_types::ClientCapabilities {
            workspace: None,
            text_document: None,
            window: None,
            experimental: None,
        };
        let params = lsp_types::InitializeParams {
            process_id: None,
            root_path: None, // TODO: Stop using this deprecated field.
            root_uri: None,
            initialization_options: None,
            capabilities: capabilities,
            trace: None,
            workspace_folders: None,
            client_info: None,
        };
        let params = serde_json::to_value(&params).unwrap();

        let (r, w) = pipe();
        let handle = std::thread::spawn(move || {
            let status = start(reader, w);
            status
        });

        write_request(
            &mut writer,
            1,
            lsp_types::request::Initialize::METHOD,
            params,
        )
        .unwrap();

        let mut r = BufReader::new(r);
        let msg = read_message(&mut r).unwrap();
        match msg {
            protocol::Message::Response(msg) => {
                assert_eq!(1, msg.id);
            }
            _ => unreachable!(),
        }

        write_notification(
            &mut writer,
            lsp_types::notification::Initialized::METHOD,
            serde_json::Value::Null,
        )
        .unwrap();

        write_request(
            &mut writer,
            2,
            lsp_types::request::Shutdown::METHOD,
            serde_json::Value::Null,
        )
        .unwrap();

        let msg = read_message(&mut r).unwrap();
        match msg {
            protocol::Message::Response(msg) => {
                assert_eq!(2, msg.id);
            }
            _ => unreachable!(),
        }

        write_notification(
            &mut writer,
            lsp_types::notification::Exit::METHOD,
            serde_json::Value::Null,
        )
        .unwrap();

        drop(writer);
        drop(r);

        let status = handle.join().unwrap();
        assert!(status.is_ok());
    }
}
