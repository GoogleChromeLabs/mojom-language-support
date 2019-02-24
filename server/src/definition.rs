use std::collections::HashMap;

use lsp_types::{Location, Position, Range, Url};

use mojom_parser::{MojomAst, Statement};

#[derive(Debug)]
pub struct Definitions {
    pub uri: Url,
    defs: HashMap<String, Range>,
}

impl Definitions {
    pub fn new(uri: Url, ast: &MojomAst) -> Definitions {
        let defs = get_definitions(ast);
        Definitions {
            uri: uri,
            defs: defs,
        }
    }

    pub fn find(&self, name: &str) -> Option<Location> {
        self.defs
            .get(name)
            .map(|range| Location::new(self.uri.clone(), range.clone()))
    }
}

fn get_definitions(ast: &MojomAst) -> HashMap<String, Range> {
    let mut res = HashMap::new();
    for stmt in &ast.mojom.stmts {
        match stmt {
            Statement::Interface(ref intr) => {
                let name = ast.text(&intr.name).to_owned();
                let (line, col) = ast.line_col(intr.name.start);
                let start = Position::new((line - 1) as u64, (col - 1) as u64);
                let (line, col) = ast.line_col(intr.name.end);
                let end = Position::new((line - 1) as u64, (col - 1) as u64);
                let range = Range::new(start, end);
                res.insert(name, range);
            }
            _ => (),
        }
    }
    res
}
