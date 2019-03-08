use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;

use serde_json::Value;

use crate::protocol::{
    self, into_notification_params, into_request_id_params, read_message, write_success_result,
    ErrorCodes, Message, NotificationMessage, RequestMessage, ResponseError,
};

use crate::definition;
use crate::import::{check_imports, ImportedFiles};
use crate::messagesender::{MessageSender, MessageSenderThread};
use crate::mojomast::MojomAst;

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
    // Workspace root path.
    root_path: std::path::PathBuf,
    // True when the previous text document has errors.
    has_error: bool,
    // Contains the current document text and ast. None when the text is an
    // invalid mojom.
    ast: Option<MojomAst>,
    // Parsed mojom files that are imported from the current document.
    imported_files: Option<ImportedFiles>,
    // Set when `exit` notification is received.
    exit_code: Option<i32>,
}

impl ServerContext {
    fn new(root_path: PathBuf) -> ServerContext {
        ServerContext {
            state: State::Initialized,
            root_path: root_path,
            has_error: false,
            ast: None,
            imported_files: None,
            exit_code: None,
        }
    }
}

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

// Requests

fn get_request_params<P: serde::de::DeserializeOwned>(
    params: Value,
) -> std::result::Result<P, ResponseError> {
    serde_json::from_value::<P>(params)
        .map_err(|err| ResponseError::new(ErrorCodes::InvalidRequest, err.to_string()))
}

fn handle_request(
    ctx: &mut ServerContext,
    mut msg_sender: MessageSender,
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
            .and_then(|params| goto_definition_request(ctx, msg_sender.clone(), params)),
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

fn _get_offset_from_position(text: &str, pos: lsp_types::Position) -> usize {
    let pos_line = pos.line as usize;
    let pos_col = pos.character as usize;
    let mut offset = 0;
    for (i, line) in text.lines().enumerate() {
        if i == pos_line {
            break;
        }
        offset += line.len() + 1;
    }
    offset + pos_col
}

#[inline(always)]
fn is_identifier_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '.'
}

fn get_identifier(text: &str, pos: lsp_types::Position) -> &str {
    // TODO: The current implementation isn't accurate.

    let offset = _get_offset_from_position(text, pos);
    let mut s = offset;
    for ch in text[..offset].chars().rev() {
        if !is_identifier_char(ch) {
            break;
        }
        s -= 1;
    }
    let mut e = offset;
    for ch in text[offset..].chars() {
        if !is_identifier_char(ch) {
            break;
        }
        e += 1;
    }
    &text[s..e]
}

fn find_definition_in_doc(ast: &MojomAst, ident: &str) -> Option<lsp_types::Location> {
    definition::find_definition(ident, ast)
}

fn find_definition_in_imports(ctx: &ServerContext, ident: &str) -> Option<lsp_types::Location> {
    ctx.imported_files
        .as_ref()
        .and_then(|ref imported_files| imported_files.find_definition(ident))
}

fn is_same_uri(ctx: &ServerContext, uri: &lsp_types::Url) -> bool {
    if let Some(ref ast) = ctx.ast {
        *uri == ast.uri
    } else {
        false
    }
}

fn invalidate_context(ctx: &mut ServerContext, msg_sender: MessageSender, uri: &lsp_types::Url) {
    // Invalidate the current document.
    use std::io::Read;
    let path = uri.to_file_path().unwrap();
    let mut text = String::new();
    std::fs::File::open(path)
        .unwrap()
        .read_to_string(&mut text)
        .unwrap();

    match _check_syntax(ctx, msg_sender, text, uri.clone()) {
        Ok(_) => (),
        Err(err) => {
            // TODO: Should return an Err that indicates the file specified
            // by `uri` isn't a vailid mojom file.
            eprintln!("{:#?}", err);
            unimplemented!();
        }
    }
}

fn goto_definition_request(
    ctx: &mut ServerContext,
    msg_sender: MessageSender,
    params: lsp_types::TextDocumentPositionParams,
) -> RequestResult {
    if !is_same_uri(ctx, &params.text_document.uri) {
        invalidate_context(ctx, msg_sender, &params.text_document.uri);
    }

    if let Some(ref ast) = &ctx.ast {
        let ident = get_identifier(&ast.text, params.position);
        let loc = find_definition_in_doc(ast, ident).or(find_definition_in_imports(ctx, ident));
        let res = match loc {
            Some(loc) => serde_json::to_value(loc).unwrap(),
            None => Value::Null,
        };
        Ok(res)
    } else {
        Ok(Value::Null)
    }
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
    msg_sender: MessageSender,
    msg: NotificationMessage,
) -> std::result::Result<(), Error> {
    let method = msg.method.as_str();
    eprintln!("Got notification: {}", method);

    use lsp_types::notification::*;
    match msg.method.as_str() {
        Exit::METHOD => exit_notification(ctx),
        DidOpenTextDocument::METHOD => get_params(msg.params)
            .and_then(|params| did_open_text_document(ctx, msg_sender, params)),
        DidChangeTextDocument::METHOD => get_params(msg.params)
            .and_then(|params| did_change_text_document(ctx, msg_sender, params)),
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

fn publish_diagnotics(
    mut msg_sender: MessageSender,
    params: lsp_types::PublishDiagnosticsParams,
) -> std::result::Result<(), Error> {
    // TODO: Don't use unwrap().
    let params = serde_json::to_value(&params).unwrap();
    let msg = NotificationMessage {
        method: "textDocument/publishDiagnostics".to_owned(),
        params: params,
    };
    msg_sender.send_notification(msg);
    Ok(())
}

fn convert_error_position(line_col: &mojom_syntax::LineColLocation) -> lsp_types::Range {
    let (start, end) = match line_col {
        mojom_syntax::LineColLocation::Pos((line, col)) => {
            let start = lsp_types::Position {
                line: *line as u64 - 1,
                character: *col as u64 - 1,
            };
            // ???
            let end = lsp_types::Position {
                line: *line as u64 - 1,
                character: *col as u64 - 1,
            };
            (start, end)
        }
        mojom_syntax::LineColLocation::Span(start, end) => {
            // `start` and `end` are tuples like (line, col).
            let start = lsp_types::Position {
                line: start.0 as u64 - 1,
                character: start.1 as u64 - 1,
            };
            let end = lsp_types::Position {
                line: end.0 as u64 - 1,
                character: end.1 as u64 - 1,
            };
            (start, end)
        }
    };
    lsp_types::Range {
        start: start,
        end: end,
    }
}

fn _check_syntax(
    ctx: &mut ServerContext,
    msg_sender: MessageSender,
    text: String,
    uri: lsp_types::Url,
) -> std::result::Result<(), Error> {
    match MojomAst::new(uri.clone(), text) {
        Ok(ast) => {
            ctx.ast = Some(ast);
            let params = lsp_types::PublishDiagnosticsParams {
                uri: uri,
                diagnostics: vec![],
            };
            publish_diagnotics(msg_sender, params)?;
            ctx.has_error = false;
        }
        Err(err) => {
            let range = convert_error_position(&err.line_col);
            let diagnostic = lsp_types::Diagnostic {
                range: range,
                severity: Some(lsp_types::DiagnosticSeverity::Error),
                code: Some(lsp_types::NumberOrString::String("mojom".to_owned())),
                source: Some("mojom-lsp".to_owned()),
                message: err.to_string(),
                related_information: None,
            };
            let params = lsp_types::PublishDiagnosticsParams {
                uri: uri,
                diagnostics: vec![diagnostic],
            };
            publish_diagnotics(msg_sender, params)?;
            ctx.ast = None;
            ctx.has_error = true;
        }
    }
    // TODO: Tentative
    if let Some(ast) = &ctx.ast {
        let imported_files = check_imports(&ctx.root_path, ast);
        ctx.imported_files = Some(imported_files);
    }
    Ok(())
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
    msg_sender: MessageSender,
    params: lsp_types::DidOpenTextDocumentParams,
) -> std::result::Result<(), Error> {
    let uri = params.text_document.uri.clone();
    _check_syntax(ctx, msg_sender, params.text_document.text, uri)
}

fn did_change_text_document(
    ctx: &mut ServerContext,
    msg_sender: MessageSender,
    params: lsp_types::DidChangeTextDocumentParams,
) -> std::result::Result<(), Error> {
    let uri = params.text_document.uri.clone();
    let content = params
        .content_changes
        .iter()
        .map(|i| i.text.to_owned())
        .collect::<Vec<_>>();
    let text = content.join("");
    _check_syntax(ctx, msg_sender, text, uri)
}

// Initialization

fn initialize(
    reader: &mut impl BufRead,
    writer: &mut impl Write,
) -> std::result::Result<lsp_types::InitializeParams, Error> {
    let message = read_message(reader)?;

    let (id, params) = match message {
        Message::Request(req) => into_request_id_params::<lsp_types::request::Initialize>(req)?,
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
        Message::Notofication(notif) => {
            into_notification_params::<lsp_types::notification::Initialized>(notif)?
        }
        _ => {
            let error_message = format!("Expected initialized message but got {:?}", message);
            return Err(Error::ProtocolError(error_message));
        }
    };

    Ok(params)
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

    let params = initialize(&mut reader, &mut writer)?;

    let root_path = get_root_path(&params);
    eprintln!("root_path: {:?}", root_path);

    let msg_sender_thread = MessageSenderThread::start(writer);

    let mut ctx = ServerContext::new(root_path);

    loop {
        eprintln!("Reading message...");
        let message = read_message(&mut reader)?;
        let msg_sender = msg_sender_thread.get_sender();
        match message {
            Message::Request(request) => handle_request(&mut ctx, msg_sender, request)?,
            Message::Notofication(notification) => {
                handle_notification(&mut ctx, msg_sender, notification)?
            }
            _ => unimplemented!(),
        };

        if let Some(exit_code) = ctx.exit_code {
            return Ok(exit_code);
        }
    }
}
