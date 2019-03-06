use mojom_syntax::{self, parse, Module, MojomFile, ParseError, Statement};

#[derive(Debug)]
pub struct MojomAst {
    pub uri: lsp_types::Url,
    pub text: String,
    pub mojom: MojomFile,

    module: Option<Module>,
}

impl MojomAst {
    pub fn new<S: Into<String>>(
        uri: lsp_types::Url,
        text: S,
    ) -> std::result::Result<MojomAst, ParseError> {
        let text = text.into();
        let mojom = parse(&text)?;
        let module = find_module_stmt(&mojom);
        Ok(MojomAst {
            uri: uri,
            text: text,
            mojom: mojom,
            module: module,
        })
    }

    pub fn from_mojom(uri: lsp_types::Url, text: String, mojom: MojomFile) -> MojomAst {
        let module = find_module_stmt(&mojom);
        MojomAst {
            uri: uri,
            text: text,
            mojom: mojom,
            module: module,
        }
    }

    pub fn text(&self, field: &mojom_syntax::Range) -> &str {
        // Can panic.
        &self.text[field.start..field.end]
    }

    pub fn line_col(&self, offset: usize) -> (usize, usize) {
        // Can panic.
        mojom_syntax::line_col(&self.text, offset).unwrap()
    }

    pub fn module_name(&self) -> Option<&str> {
        self.module
            .as_ref()
            .map(|ref module| self.text(&module.name))
    }
}

fn find_module_stmt(mojom: &MojomFile) -> Option<Module> {
    // This function assumes that `mojom` has only one Module, which should be
    // checked in semantics analysis.
    for stmt in &mojom.stmts {
        match stmt {
            Statement::Module(module) => {
                return Some(module.clone());
            }
            _ => continue,
        }
    }
    None
}
