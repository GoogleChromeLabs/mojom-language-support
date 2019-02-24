use lsp_types::{Location, Position, Range, Url};

use mojom_parser::Statement;

use crate::server::MojomAst;

fn create_lsp_range(ast: &MojomAst, field: &mojom_parser::Range) -> Range {
    let (line, col) = ast.line_col(field.start);
    let start = Position::new((line - 1) as u64, (col - 1) as u64);
    let (line, col) = ast.line_col(field.end);
    let end = Position::new((line - 1) as u64, (col - 1) as u64);
    Range::new(start, end)
}

pub fn find_definition(name: &str, uri: Url, ast: &MojomAst) -> Option<Location> {
    macro_rules! match_field {
        ($field:expr) => {{
            let field_name = ast.text(&$field);
            if field_name != name {
                continue;
            }
            let range = create_lsp_range(ast, &$field);
            return Some(Location::new(uri, range));
        }};
    }

    // Only toplevel definitions for now.
    for stmt in &ast.mojom.stmts {
        match stmt {
            Statement::Interface(ref stmt) => {
                match_field!(stmt.name);
            }
            Statement::Struct(ref stmt) => {
                match_field!(stmt.name);
            }
            Statement::Union(ref stmt) => {
                match_field!(stmt.name);
            }
            Statement::Enum(ref stmt) => {
                match_field!(stmt.name);
            }
            Statement::Const(ref stmt) => {
                match_field!(stmt.name);
            }
            _ => (),
        }
    }
    None
}
