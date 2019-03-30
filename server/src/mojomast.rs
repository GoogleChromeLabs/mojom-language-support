use mojom_syntax::{self, Module, MojomFile};

#[derive(Debug)]
pub(crate) struct MojomAst {
    pub(crate) uri: lsp_types::Url,
    pub(crate) text: String,
    pub(crate) mojom: MojomFile,

    module: Option<Module>,
}

impl MojomAst {
    pub(crate) fn from_mojom(
        uri: lsp_types::Url,
        text: String,
        mojom: MojomFile,
        module: Option<Module>,
    ) -> MojomAst {
        MojomAst {
            uri: uri,
            text: text,
            mojom: mojom,
            module: module,
        }
    }

    pub(crate) fn text(&self, field: &mojom_syntax::Range) -> &str {
        // Can panic.
        &self.text[field.start..field.end]
    }

    pub(crate) fn line_col(&self, offset: usize) -> mojom_syntax::LineCol {
        // Can panic.
        mojom_syntax::line_col(&self.text, offset).unwrap()
    }

    pub(crate) fn module_name(&self) -> Option<&str> {
        self.module
            .as_ref()
            .map(|ref module| self.text(&module.name))
    }
}
