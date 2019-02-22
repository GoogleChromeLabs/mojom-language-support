use std::collections::HashMap;

use lsp_types::{Location, Position, Range, Url};

use mojom_parser::{MojomFile, Statement};

#[derive(Debug)]
pub struct Definitions {
    pub uri: Url,
    defs: HashMap<String, Range>,
}

impl Definitions {
    pub fn new(uri: Url, mojom: &MojomFile) -> Definitions {
        let defs = get_definitions(mojom);
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

fn get_definitions(mojom: &MojomFile) -> HashMap<String, Range> {
    let mut res = HashMap::new();
    for stmt in &mojom.stmts {
        match stmt {
            Statement::Interface(ref intr) => {
                let name = intr.name.as_str().to_owned();
                let start_pos = intr.name.start_pos();
                let (line, col) = start_pos.line_col();
                let start = Position::new((line - 1) as u64, (col - 1) as u64);
                let end_pos = intr.name.end_pos();
                let (line, col) = end_pos.line_col();
                let end = Position::new((line - 1) as u64, (col - 1) as u64);
                let range = Range::new(start, end);
                res.insert(name, range);
            }
            _ => (),
        }
    }
    res
}
