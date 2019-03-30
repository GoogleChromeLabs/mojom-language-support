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

fn is_keyword(ident: &str) -> bool {
    ident == "array"
        || ident == "true"
        || ident == "false"
        || ident == "default"
        || ident == "bool"
        || ident == "int8"
        || ident == "uint8"
        || ident == "int16"
        || ident == "uint8"
        || ident == "int32"
        || ident == "uint32"
        || ident == "int64"
        || ident == "uint64"
        || ident == "float"
        || ident == "double"
        || ident == "associated"
        || ident == "const"
        || ident == "enum"
        || ident == "handle"
        || ident == "import"
        || ident == "interface"
        || ident == "map"
        || ident == "module"
        || ident == "struct"
        || ident == "union"
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

fn check_invalid_keywords(
    text: &str,
    mojom: &MojomFile,
    diagnostics: &mut Vec<lsp_types::Diagnostic>,
) {
    let idents = mojom_syntax::preorder(mojom).filter_map(|traverse| match traverse {
        mojom_syntax::Traversal::EnterInterface(stmt) => Some(&stmt.name),
        mojom_syntax::Traversal::EnterStruct(stmt) => Some(&stmt.name),
        mojom_syntax::Traversal::Module(stmt) => Some(&stmt.name),
        mojom_syntax::Traversal::Method(stmt) => Some(&stmt.name),
        mojom_syntax::Traversal::Union(stmt) => Some(&stmt.name),
        mojom_syntax::Traversal::Enum(stmt) => Some(&stmt.name),
        mojom_syntax::Traversal::Const(stmt) => Some(&stmt.name),
        mojom_syntax::Traversal::StructField(stmt) => Some(&stmt.name),
        _ => None,
    });
    for range in idents {
        let name = partial_text(&text, range);
        if is_keyword(name) {
            let message = format!("Unexpected keyword: {}", name);
            let diagnostic = create_diagnostic(&text, range, message);
            diagnostics.push(diagnostic);
        }
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

    check_invalid_keywords(text, mojom, &mut diagnostics);

    Analysis {
        module: module,
        diagnostics: diagnostics,
    }
}
