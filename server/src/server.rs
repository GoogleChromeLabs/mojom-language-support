use std::io::{self, BufRead, BufReader, BufWriter, Write};

use serde_json::Value;

use mojom_parser::{self, parse, MojomFile, ParseError};

use crate::protocol::{
    read_message, write_error_response, write_notification, write_success_response,
    write_success_result, ErrorCodes, Message, NotificationMessage, RequestMessage, ResponseError,
};

use crate::definition;
use crate::{Error, Result};

#[derive(Debug)]
pub struct MojomAst {
    pub uri: lsp_types::Url,
    pub text: String,
    pub mojom: MojomFile,
}

impl MojomAst {
    pub fn new<S: Into<String>>(
        uri: lsp_types::Url,
        text: S,
    ) -> std::result::Result<MojomAst, ParseError> {
        let text = text.into();
        let mojom = parse(&text)?;
        Ok(MojomAst {
            uri: uri,
            text: text,
            mojom: mojom,
        })
    }

    pub fn text(&self, field: &mojom_parser::Range) -> &str {
        // Can panic.
        &self.text[field.start..field.end]
    }

    pub fn line_col(&self, offset: usize) -> (usize, usize) {
        // Can panic.
        mojom_parser::line_col(&self.text, offset).unwrap()
    }
}

#[derive(PartialEq)]
enum State {
    Initialized,
    ShuttingDown,
}

struct ServerContext {
    state: State,
    // True when the previous text document has errors.
    has_error: bool,
    // Contains the current document text and ast. None when the text is an
    // invalid mojom.
    ast: Option<MojomAst>,
    // Set when `exit` notification is received.
    exit_code: Option<i32>,
}

impl ServerContext {
    fn new() -> ServerContext {
        ServerContext {
            state: State::Initialized,
            has_error: false,
            ast: None,
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
    writer: &mut impl Write,
    ctx: &mut ServerContext,
    msg: RequestMessage,
) -> Result<()> {
    let id = msg.id;
    let method = msg.method.as_str();

    use lsp_types::request::*;
    let res = match method {
        Initialize::METHOD => initialize_request(),
        Shutdown::METHOD => shutdown_request(ctx),
        GotoDefinition::METHOD => get_request_params(msg.params)
            .and_then(|params| goto_definition_request(writer, ctx, params)),
        _ => unimplemented!(),
    };
    match res {
        Ok(res) => write_success_response(writer, id, res)?,
        Err(error) => write_error_response(writer, id, error)?,
    }
    Ok(())
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

fn goto_definition_request(
    _writer: &mut Write,
    ctx: &mut ServerContext,
    params: lsp_types::TextDocumentPositionParams,
) -> RequestResult {
    match &ctx.ast {
        Some(ref ast) => {
            let ident = get_identifier(&ast.text, params.position);
            let loc = definition::find_definition(ident, &ast);
            match loc {
                Some(loc) => {
                    let loc = serde_json::to_value(loc).unwrap();
                    Ok(loc)
                }
                None => Ok(Value::Null),
            }
        }
        None => Ok(Value::Null),
    }
}

// Notifications

fn get_params<P: serde::de::DeserializeOwned>(params: Value) -> Result<P> {
    serde_json::from_value::<P>(params).map_err(|err| Error::ProtocolError(err.to_string()))
}

fn do_nothing() -> Result<()> {
    Ok(())
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
        // Accept following notifications but do nothing.
        DidChangeConfiguration::METHOD => do_nothing(),
        WillSaveTextDocument::METHOD => do_nothing(),
        DidSaveTextDocument::METHOD => do_nothing(),
        // Intentionally crash for unsupported notifications.
        _ => {
            panic!(format!(
                "Notification `{}` isn't supported yet",
                msg.method.as_str()
            ));
        }
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

fn convert_error_position(line_col: &mojom_parser::LineColLocation) -> lsp_types::Range {
    let (start, end) = match line_col {
        mojom_parser::LineColLocation::Pos((line, col)) => {
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
        mojom_parser::LineColLocation::Span(start, end) => {
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
    writer: &mut impl Write,
    ctx: &mut ServerContext,
    text: String,
    uri: lsp_types::Url,
) -> Result<()> {
    match MojomAst::new(uri.clone(), text) {
        Ok(ast) => {
            ctx.ast = Some(ast);
            let params = lsp_types::PublishDiagnosticsParams {
                uri: uri,
                diagnostics: vec![],
            };
            publish_diagnotics(writer, params)?;
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
            publish_diagnotics(writer, params)?;
            ctx.ast = None;
            ctx.has_error = true;
        }
    }
    Ok(())
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
    _check_syntax(writer, ctx, params.text_document.text, uri)
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
    let text = content.join("");
    _check_syntax(writer, ctx, text, uri)
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
