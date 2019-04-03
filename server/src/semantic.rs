use mojom_syntax::Error as SyntaxError;
use mojom_syntax::{Module, MojomFile};

#[derive(Debug)]
pub(crate) enum Error {
    SyntaxError(String),
}

impl<'a> From<SyntaxError<'a>> for Error {
    fn from(err: SyntaxError<'a>) -> Error {
        let msg = format!("{:?}", err);
        Error::SyntaxError(msg)
    }
}

pub(crate) struct Analysis {
    pub(crate) module: Option<Module>,
    pub(crate) diagnostics: Vec<lsp_types::Diagnostic>,
}

fn partial_text<'a>(text: &'a str, range: &mojom_syntax::Range) -> &'a str {
    &text[range.start..range.end]
}

fn create_diagnostic(
    text: &str,
    range: &mojom_syntax::Range,
    message: String,
) -> lsp_types::Diagnostic {
    let start = mojom_syntax::line_col(text, range.start).unwrap();
    let end = mojom_syntax::line_col(text, range.end).unwrap();
    let range = crate::diagnostic::into_lsp_range(&start, &end);
    lsp_types::Diagnostic {
        range: range,
        severity: Some(lsp_types::DiagnosticSeverity::Error),
        code: Some(lsp_types::NumberOrString::String("mojom".to_owned())),
        source: Some("mojom-lsp".to_owned()),
        message: message,
        related_information: None,
    }
}

fn find_module(
    text: &str,
    mojom: &MojomFile,
    diagnostics: &mut Vec<lsp_types::Diagnostic>,
) -> Option<Module> {
    let mut module: Option<Module> = None;
    for stmt in &mojom.stmts {
        match stmt {
            mojom_syntax::Statement::Module(stmt) => {
                if let Some(ref module) = module {
                    let message = format!(
                        "Found more than one module statement: {} and {}",
                        partial_text(&text, &module.name),
                        partial_text(&text, &stmt.name)
                    );
                    let diagnostic = create_diagnostic(&text, &stmt.name, message);
                    diagnostics.push(diagnostic);
                } else {
                    module = Some(stmt.clone());
                }
            }
            _ => (),
        }
    }
    module
}

pub(crate) fn check_semantics(text: &str, mojom: &MojomFile) -> Analysis {
    let mut diagnostics = Vec::new();
    let module = find_module(text, mojom, &mut diagnostics);
    Analysis {
        module: module,
        diagnostics: diagnostics,
    }
}
