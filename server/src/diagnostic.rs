use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use std::sync::mpsc::{channel, Sender};
use std::thread::{self, JoinHandle};

use lsp_types::Url as Uri;

use super::imported_files::{check_imports, ImportedFiles};
use super::messagesender::MessageSender;
use super::mojomast::MojomAst;
use super::protocol::NotificationMessage;

pub(crate) fn create_diagnostic(range: lsp_types::Range, message: String) -> lsp_types::Diagnostic {
    lsp_types::Diagnostic {
        range: range,
        severity: Some(lsp_types::DiagnosticSeverity::Error),
        code: Some(lsp_types::NumberOrString::String("mojom".to_owned())),
        source: Some("mojom-lsp".to_owned()),
        message: message,
        related_information: None,
        tags: None,
    }
}

enum DiagnosticMessage {
    CheckSyntax((Uri, String)),
    GotoDefinition(
        (
            Uri,
            lsp_types::Position,
            Sender<Option<lsp_types::Location>>,
        ),
    ),
}

pub(crate) struct DiagnosticsThread {
    handle: JoinHandle<()>,
    sender: Sender<DiagnosticMessage>,
}

impl DiagnosticsThread {
    #[allow(unused)]
    pub(crate) fn join(self) {
        self.handle.join().unwrap();
    }

    pub(crate) fn check(&self, uri: Uri, text: String) {
        self.sender
            .send(DiagnosticMessage::CheckSyntax((uri, text)))
            .unwrap();
    }

    pub(crate) fn goto_definition(
        &self,
        uri: Uri,
        pos: lsp_types::Position,
    ) -> Option<lsp_types::Location> {
        let (loc_sender, loc_receiver) = channel::<Option<lsp_types::Location>>();
        self.sender
            .send(DiagnosticMessage::GotoDefinition((uri, pos, loc_sender)))
            .unwrap();
        let loc = loc_receiver.recv().unwrap();
        loc
    }
}

pub(crate) fn start_diagnostics_thread(
    root_path: PathBuf,
    msg_sender: MessageSender,
) -> DiagnosticsThread {
    let mut diag = Diagnostic::new(root_path, msg_sender);
    let (sender, receiver) = channel::<DiagnosticMessage>();
    let handle = thread::spawn(move || loop {
        let msg = match receiver.recv() {
            Ok(msg) => msg,
            Err(_) => break,
        };

        match msg {
            DiagnosticMessage::CheckSyntax((uri, text)) => {
                diag.check(uri, text);
            }
            DiagnosticMessage::GotoDefinition((uri, pos, loc_sender)) => {
                let loc = diag.find_definition(uri, pos);
                loc_sender.send(loc).unwrap();
            }
        }
    });

    DiagnosticsThread {
        handle: handle,
        sender: sender,
    }
}

struct Diagnostic {
    // Workspace root path.
    root_path: PathBuf,
    // A message sender. It is used in the diagnostics thread to send
    // notifications.
    msg_sender: MessageSender,
    // Current parsed syntax tree with the original text.
    ast: Option<MojomAst>,
    // Parsed mojom files that are imported from the current document.
    imported_files: Option<ImportedFiles>,
}

impl Diagnostic {
    fn new(root_path: PathBuf, msg_sender: MessageSender) -> Self {
        Diagnostic {
            root_path: root_path,
            msg_sender: msg_sender,
            ast: None,
            imported_files: None,
        }
    }

    fn check(&mut self, uri: Uri, text: String) {
        self.check_syntax(uri.clone(), text);
        self.check_imported_files();
    }

    fn find_definition(
        &mut self,
        uri: Uri,
        pos: lsp_types::Position,
    ) -> Option<lsp_types::Location> {
        if !self.is_same_uri(&uri) {
            // TODO: Don't use unwrap().
            self.open(uri).unwrap();
        }

        if let Some(ast) = &self.ast {
            let ident = get_identifier(&ast.text, &pos);
            let loc = find_definition_in_doc(ast, &ident).or(find_definition_in_imported_files(
                &self.imported_files,
                &ident,
            ));
            loc
        } else {
            None
        }
    }

    fn is_same_uri(&self, uri: &Uri) -> bool {
        if let Some(ast) = &self.ast {
            *uri == ast.uri
        } else {
            false
        }
    }

    fn open(&mut self, uri: Uri) -> std::io::Result<()> {
        let path = uri.to_file_path().unwrap();
        let mut text = String::new();
        File::open(path).and_then(|mut f| f.read_to_string(&mut text))?;
        self.check(uri, text);
        Ok(())
    }

    fn check_syntax(&mut self, uri: Uri, text: String) {
        let mojom = mojom_syntax::parse(&text);
        let diagnostics = match mojom {
            Ok(mojom) => {
                let analytics = crate::semantic::check_semantics(&text, &mojom);
                // TODO: Don't store ast when semantics check fails?
                self.ast = Some(MojomAst::from_mojom(
                    uri.clone(),
                    text,
                    mojom,
                    analytics.module,
                ));
                analytics.diagnostics
            }
            Err(err) => {
                self.ast = None;
                let (start, end) = err.range();
                let range = into_lsp_range(&start, &end);
                let diagnostic = create_diagnostic(range, err.to_string());
                vec![diagnostic]
            }
        };

        let params = lsp_types::PublishDiagnosticsParams {
            uri: uri,
            diagnostics: diagnostics,
            // TODO: Support version
            version: None,
        };
        publish_diagnostics(&self.msg_sender, params);
    }

    fn check_imported_files(&mut self) {
        if let Some(ast) = &self.ast {
            let imported_files = check_imports(&self.root_path, ast);
            self.imported_files = Some(imported_files);
        }
    }
}

pub(crate) fn into_lsp_range(
    start: &mojom_syntax::LineCol,
    end: &mojom_syntax::LineCol,
) -> lsp_types::Range {
    lsp_types::Range {
        start: lsp_types::Position::new(start.line as u64, start.col as u64),
        end: lsp_types::Position::new(end.line as u64, end.col as u64),
    }
}

fn publish_diagnostics(msg_sender: &MessageSender, params: lsp_types::PublishDiagnosticsParams) {
    let params = serde_json::to_value(&params).unwrap();
    let msg = NotificationMessage {
        method: "textDocument/publishDiagnostics".to_owned(),
        params: params,
    };
    msg_sender.send_notification(msg);
}

fn get_offset_from_position(text: &str, pos: &lsp_types::Position) -> usize {
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

fn get_identifier<'a>(text: &'a str, pos: &lsp_types::Position) -> &'a str {
    // TODO: The current implementation isn't accurate.

    let offset = get_offset_from_position(text, pos);
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
    crate::definition::find_definition_preorder(ident, ast)
}

fn find_definition_in_imported_files(
    imported_files: &Option<ImportedFiles>,
    ident: &str,
) -> Option<lsp_types::Location> {
    imported_files
        .as_ref()
        .and_then(|ref imported_files| imported_files.find_definition(ident))
}
