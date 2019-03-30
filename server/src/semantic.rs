use mojom_syntax::Error as SyntaxError;
use mojom_syntax::{Module, MojomFile};

#[derive(Debug)]
pub(crate) enum Error {
    SyntaxError(SyntaxError),
}

impl From<SyntaxError> for Error {
    fn from(err: SyntaxError) -> Error {
        Error::SyntaxError(err)
    }
}

pub(crate) struct Analysis {
    pub(crate) module: Option<Module>,
    pub(crate) diagnostics: Vec<lsp_types::Diagnostic>,
}

fn partial_text<'a>(text: &'a str, range: &mojom_syntax::Range) -> &'a str {
    &text[range.start..range.end]
}

pub(crate) fn check_semantics(text: &str, mojom: &MojomFile) -> Analysis {
    let mut module: Option<Module> = None;
    let mut diagnostics = Vec::new();

    for traverse in mojom_syntax::preorder(mojom) {
        match traverse {
            mojom_syntax::Traversal::Module(stmt) => {
                if let Some(ref module) = module {
                    let message = format!(
                        "Found more than one module statement: {} and {}",
                        partial_text(&text, &module.name),
                        partial_text(&text, &stmt.name)
                    );
                    let start = mojom_syntax::line_col(text, stmt.name.start).unwrap();
                    let end = mojom_syntax::line_col(text, stmt.name.end).unwrap();
                    let range = crate::diagnostic::into_lsp_range(&start, &end);
                    let diagnostic = lsp_types::Diagnostic {
                        range: range,
                        severity: Some(lsp_types::DiagnosticSeverity::Error),
                        code: Some(lsp_types::NumberOrString::String("mojom".to_owned())),
                        source: Some("mojom-lsp".to_owned()),
                        message: message,
                        related_information: None,
                    };
                    diagnostics.push(diagnostic);
                } else {
                    module = Some(stmt.clone());
                }
            }
            _ => (),
        }
    }

    Analysis {
        module: module,
        diagnostics: diagnostics,
    }
}
