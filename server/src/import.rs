use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use mojom_parser::MojomFile;

#[derive(Debug)]
pub struct ImportDefinition {
    pub ident: String,
    pub line_col: (usize, usize),
}

#[derive(Debug)]
pub struct Import {
    path: PathBuf,
    defs: Vec<ImportDefinition>,
}

#[derive(Debug)]
pub enum ImportError {
    IoError(std::io::Error),
    SyntaxError(String),
}

impl From<std::io::Error> for ImportError {
    fn from(err: std::io::Error) -> Self {
        ImportError::IoError(err)
    }
}

struct Ast {
    text: String,
    mojom: MojomFile,
}

// TODO: Merge this into MojomAst
impl Ast {
    pub fn text(&self, field: &mojom_parser::Range) -> &str {
        // Can panic.
        &self.text[field.start..field.end]
    }

    pub fn line_col(&self, offset: usize) -> (usize, usize) {
        // Can panic.
        mojom_parser::line_col(&self.text, offset).unwrap()
    }
}

struct ImportVisitor<'a> {
    ast: &'a Ast,
    path: Vec<&'a str>,
    // Output of this visitor.
    defs: Vec<ImportDefinition>,
}

impl<'a> mojom_parser::Visitor for ImportVisitor<'a> {
    fn visit_interface(&mut self, elem: &mojom_parser::Interface) {
        let name = self.ast.text(&elem.name);
        self.path.push(name);

        let line_col = self.ast.line_col(elem.name.start);
        self.defs.push(ImportDefinition {
            ident: self.path.join("."),
            line_col: line_col,
        })
    }

    fn leave_interface(&mut self, _: &mojom_parser::Interface) {
        self.path.pop();
    }
}

pub fn parse_imported<P: AsRef<Path>>(path: P) -> Result<Import, ImportError> {
    let mut text = String::new();
    File::open(path.as_ref())?.read_to_string(&mut text)?;

    let mojom =
        mojom_parser::parse(&text).map_err(|err| ImportError::SyntaxError(err.to_string()))?;

    let ast = Ast {
        text: text,
        mojom: mojom,
    };

    let mut v = ImportVisitor {
        ast: &ast,
        path: Vec::new(),
        defs: Vec::new(),
    };
    use mojom_parser::Element;
    ast.mojom.accept(&mut v);

    Ok(Import {
        path: path.as_ref().to_owned(),
        defs: v.defs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_imported() {
        let res = parse_imported("../testdata/my_interface.mojom").unwrap();
        println!("{:?}", res);
    }
}
