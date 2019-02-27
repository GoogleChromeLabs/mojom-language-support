use mojom_parser::{self, parse, MojomFile, ParseError};

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
