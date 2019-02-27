use std::fs::File;
use std::io::Read;
use std::path::Path;

use lsp_types::{Location, Range, Url};

use crate::definition::create_lsp_range;
use crate::mojomast::MojomAst;

#[derive(Debug)]
struct ImportDefinition {
    pub ident: String,
    pub range: Range,
}

#[derive(Debug)]
struct Import {
    uri: Url,
    module_name: Option<String>,
    defs: Vec<ImportDefinition>,
}

#[derive(Debug)]
enum ImportError {
    IoError(std::io::Error),
    SyntaxError(String),
}

impl From<std::io::Error> for ImportError {
    fn from(err: std::io::Error) -> Self {
        ImportError::IoError(err)
    }
}

type ImportResult = std::result::Result<Import, ImportError>;

#[derive(Debug)]
pub struct ImportedFiles {
    imports: Vec<ImportResult>,
}

impl ImportedFiles {
    pub fn find_definition(&self, ident: &str) -> Option<Location> {
        for imported in &self.imports {
            if let Ok(ref imported) = imported {
                for definition in &imported.defs {
                    if definition.ident == ident {
                        let loc = Location::new(imported.uri.clone(), definition.range.clone());
                        return Some(loc);
                    }

                    if let Some(module_name) = &imported.module_name {
                        let canonocal_name = format!("{}.{}", module_name, definition.ident);
                        if canonocal_name == ident {
                            let loc = Location::new(imported.uri.clone(), definition.range.clone());
                            return Some(loc);
                        }
                    }
                }
            }
        }
        None
    }
}

pub fn check_imports<P: AsRef<Path>>(root_path: P, ast: &MojomAst) -> ImportedFiles {
    let root_path = root_path.as_ref();
    let mut imports = Vec::new();
    for stmt in &ast.mojom.stmts {
        match stmt {
            mojom_parser::Statement::Import(stmt) => {
                let path = ast.text(&stmt.path);
                let path = root_path.join(&path[1..path.len() - 1]);
                let imported = parse_imported(&path);
                imports.push(imported);
            }
            _ => (),
        }
    }

    ImportedFiles { imports: imports }
}

struct ImportVisitor<'a> {
    ast: &'a MojomAst,
    path: Vec<&'a str>,
    // Output of this visitor.
    defs: Vec<ImportDefinition>,
}

impl<'a> ImportVisitor<'a> {
    fn add(&mut self, field: &mojom_parser::Range) {
        let name = self.ast.text(field);
        self.path.push(name);
        let ident = self.path.join(".");
        self.path.pop();
        let range = create_lsp_range(&self.ast, field);
        self.defs.push(ImportDefinition {
            ident: ident,
            range: range,
        })
    }
}

impl<'a> mojom_parser::Visitor for ImportVisitor<'a> {
    fn visit_interface(&mut self, elem: &mojom_parser::Interface) {
        self.add(&elem.name);
        let name = self.ast.text(&elem.name);
        self.path.push(name);
    }

    fn leave_interface(&mut self, _: &mojom_parser::Interface) {
        self.path.pop();
    }

    fn visit_struct(&mut self, elem: &mojom_parser::Struct) {
        self.add(&elem.name);
        let name = self.ast.text(&elem.name);
        self.path.push(name);
    }

    fn leave_struct(&mut self, _: &mojom_parser::Struct) {
        self.path.pop();
    }

    fn visit_union(&mut self, elem: &mojom_parser::Union) {
        self.add(&elem.name);
    }

    fn visit_enum(&mut self, elem: &mojom_parser::Enum) {
        self.add(&elem.name);
    }

    fn visit_const(&mut self, elem: &mojom_parser::Const) {
        self.add(&elem.name);
    }
}

fn parse_imported<P: AsRef<Path>>(path: P) -> ImportResult {
    let mut text = String::new();
    File::open(path.as_ref())?.read_to_string(&mut text)?;

    let mojom =
        mojom_parser::parse(&text).map_err(|err| ImportError::SyntaxError(err.to_string()))?;

    // Unwrap shoud be safe because we opened file already.
    let path = path.as_ref().canonicalize().unwrap();
    let uri = Url::from_file_path(&path).unwrap();

    let ast = MojomAst::from_mojom(uri, text, mojom);

    let mut v = ImportVisitor {
        ast: &ast,
        path: Vec::new(),
        defs: Vec::new(),
    };
    use mojom_parser::Element;
    ast.mojom.accept(&mut v);

    let module_name = ast.module_name().map(|name| name.to_owned());

    Ok(Import {
        uri: ast.uri.clone(),
        module_name: module_name,
        defs: v.defs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_uri<P: AsRef<Path>>(path: P) -> Url {
        let path = path.as_ref().canonicalize().unwrap();
        Url::from_file_path(path).unwrap()
    }

    #[test]
    fn test_parse_imported() {
        let res = parse_imported("../testdata/my_interface.mojom");
        assert!(res.is_ok());
    }

    #[test]
    fn test_check_imports() {
        let root_path = "../testdata";
        let file_path = "../testdata/my_service.mojom";
        let mut text = String::new();
        File::open(&file_path)
            .unwrap()
            .read_to_string(&mut text)
            .unwrap();
        let uri = create_uri(&file_path);
        let ast = MojomAst::new(uri, text).unwrap();

        let imports = check_imports(&root_path, &ast);

        let res = imports.find_definition("BarStruct.BarEnum");
        assert!(res.is_some());
    }
}
