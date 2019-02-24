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
    for stmt in &ast.mojom.stmts {
        match stmt {
            Statement::Interface(ref intr) => {
                let intr_name = ast.text(&intr.name);
                if intr_name != name {
                    continue;
                }
                let range = create_lsp_range(ast, &intr.name);
                return Some(Location::new(uri, range));
            }
            _ => (),
        }
    }
    None
}
