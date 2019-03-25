use mojom_syntax::{self, parse, Module, MojomFile};

type Error = crate::semantic::Error;

#[derive(Debug)]
pub(crate) struct MojomAst {
    pub(crate) uri: lsp_types::Url,
    pub(crate) text: String,
    pub(crate) mojom: MojomFile,

    module: Option<Module>,
}

impl MojomAst {
    pub(crate) fn new<S: Into<String>>(
        uri: lsp_types::Url,
        text: S,
    ) -> std::result::Result<MojomAst, Error> {
        let text = text.into();
        let mojom = parse(&text)?;
        let analysis = crate::semantic::do_semantics_analysis(&mojom)?;

        Ok(MojomAst {
            uri: uri,
            text: text,
            mojom: mojom,
            module: analysis.module,
        })
    }

    pub(crate) fn from_mojom(uri: lsp_types::Url, text: String, mojom: MojomFile) -> MojomAst {
        let mut module = None;
        match crate::semantic::do_semantics_analysis(&mojom) {
            Ok(analysis) => {
                module = analysis.module;
            }
            Err(_) => (),
        };

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

    pub(crate) fn line_col(&self, offset: usize) -> (usize, usize) {
        // Can panic.
        mojom_syntax::line_col(&self.text, offset).unwrap()
    }

    pub(crate) fn module_name(&self) -> Option<&str> {
        self.module
            .as_ref()
            .map(|ref module| self.text(&module.name))
    }
}
