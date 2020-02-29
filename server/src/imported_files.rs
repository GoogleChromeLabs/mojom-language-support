use std::fs::File;
use std::io::Read;
use std::path::Path;

use lsp_types::{Location, Range, Url};

use mojom_syntax::{preorder, Traversal};

use super::definition::create_lsp_range;
use super::mojomast::MojomAst;
use super::semantic;

#[derive(Debug)]
struct ImportDefinition {
    pub ident: String,
    pub range: Range,
}

#[derive(Debug)]
struct Import {
    uri: Url,
    module_name: Option<String>,
    definitions: Vec<ImportDefinition>,
}

#[derive(Debug)]
enum ImportError {
    IoError(std::io::Error),
    NotFound(String /* path */),
    SyntaxError(String),
}

impl From<std::io::Error> for ImportError {
    fn from(err: std::io::Error) -> Self {
        if err.kind() == std::io::ErrorKind::NotFound {
            return ImportError::NotFound(err.to_string());
        }
        ImportError::IoError(err)
    }
}

type ImportResult = std::result::Result<Import, ImportError>;

#[derive(Debug)]
pub(crate) struct ImportedFiles {
    parsed_imports: Vec<ImportResult>,
}

impl ImportedFiles {
    pub(crate) fn find_definition(&self, ident: &str) -> Option<Location> {
        let valid_imports = self.parsed_imports.iter().filter_map(|i| i.as_ref().ok());
        for imported in valid_imports {
            for definition in &imported.definitions {
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
        None
    }
}

pub(crate) fn check_imports<P: AsRef<Path>>(root_path: P, ast: &MojomAst) -> ImportedFiles {
    let root_path = root_path.as_ref();
    let mut parsed_imports = Vec::new();
    for stmt in &ast.mojom.stmts {
        match stmt {
            mojom_syntax::Statement::Import(stmt) => {
                let path = ast.text(&stmt.path);
                let path = root_path.join(&path[1..path.len() - 1]);
                let imported = parse_imported(&path);
                parsed_imports.push(imported);
            }
            _ => (),
        }
    }

    ImportedFiles {
        parsed_imports: parsed_imports,
    }
}

fn add_definition<'a, 'b, 'c>(
    field: &'a mojom_syntax::Range,
    ast: &'b MojomAst,
    path: &'c mut Vec<&'b str>,
    definitions: &'c mut Vec<ImportDefinition>,
) {
    let name = ast.text(field);
    path.push(name);
    let ident = path.join(".");
    path.pop();
    let range = create_lsp_range(&ast, field);
    definitions.push(ImportDefinition {
        ident: ident,
        range: range,
    });
}

fn parse_imported<P: AsRef<Path>>(path: P) -> ImportResult {
    let mut text = String::new();
    File::open(path.as_ref())?.read_to_string(&mut text)?;

    let mojom =
        mojom_syntax::parse(&text).map_err(|err| ImportError::SyntaxError(err.to_string()))?;

    // Unwrap shoud be safe because we opened file already.
    let path = path.as_ref().canonicalize().unwrap();
    let uri = Url::from_file_path(&path).unwrap();

    // TODO: Maybe store semantics errors.
    let analysis = semantic::check_semantics(&text, &mojom);
    let ast = MojomAst::from_mojom(uri, text, mojom, analysis.module);

    let mut path = Vec::new();
    let mut definitions: Vec<ImportDefinition> = Vec::new();
    for traversal in preorder(&ast.mojom) {
        match traversal {
            Traversal::EnterInterface(node) => {
                add_definition(&node.name, &ast, &mut path, &mut definitions);
                let name = ast.text(&node.name);
                path.push(name);
            }
            Traversal::LeaveInterface(_) => {
                path.pop();
            }
            Traversal::EnterStruct(node) => {
                add_definition(&node.name, &ast, &mut path, &mut definitions);
                let name = ast.text(&node.name);
                path.push(name);
            }
            Traversal::LeaveStruct(_) => {
                path.pop();
            }
            Traversal::Union(node) => add_definition(&node.name, &ast, &mut path, &mut definitions),
            Traversal::Enum(node) => add_definition(&node.name, &ast, &mut path, &mut definitions),
            Traversal::Const(node) => add_definition(&node.name, &ast, &mut path, &mut definitions),
            _ => (),
        }
    }

    let module_name = ast.module_name().map(|name| name.to_owned());

    Ok(Import {
        uri: ast.uri.clone(),
        module_name: module_name,
        definitions: definitions,
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
        let mojom = mojom_syntax::parse(&text).unwrap();
        let analytics = semantic::check_semantics(&text, &mojom);
        let ast = MojomAst::from_mojom(uri, text, mojom, analytics.module);

        let imports = check_imports(&root_path, &ast);

        let res = imports.find_definition("FooStruct.FooEnum");
        assert!(res.is_some());
    }
}
