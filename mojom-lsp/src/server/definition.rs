// Copyright 2020 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use lsp_types::{Location, Position, Range};

use crate::syntax::{self, preorder, Traversal};

use super::mojomast::MojomAst;

pub(crate) fn create_lsp_range(ast: &MojomAst, field: &syntax::Range) -> Range {
    let pos = ast.line_col(field.start);
    let start = Position::new(pos.line as u64, pos.col as u64);
    let pos = ast.line_col(field.end);
    let end = Position::new(pos.line as u64, pos.col as u64);
    Range::new(start, end)
}

fn match_field<'a, 'b, 'c>(
    target: &'a str,
    field: &'b syntax::Range,
    ast: &'a MojomAst,
    path: &'c mut Vec<&'a str>,
) -> Option<Location> {
    let name = ast.text(field);
    path.push(name);
    let ident = path.join(".");
    path.pop();
    if ident == target {
        let range = create_lsp_range(ast, field);
        return Some(Location::new(ast.uri.clone(), range));
    }
    None
}

pub(crate) fn find_definition_preorder(ident: &str, ast: &MojomAst) -> Option<Location> {
    let mut path = Vec::new();
    for traversal in preorder(&ast.mojom) {
        let loc = match traversal {
            Traversal::EnterInterface(node) => {
                let loc = match_field(ident, &node.name, ast, &mut path);
                let name = ast.text(&node.name);
                path.push(name);
                loc
            }
            Traversal::LeaveInterface(_) => {
                path.pop();
                None
            }
            Traversal::EnterStruct(node) => {
                let loc = match_field(ident, &node.name, ast, &mut path);
                let name = ast.text(&node.name);
                path.push(name);
                loc
            }
            Traversal::LeaveStruct(_) => {
                path.pop();
                None
            }
            Traversal::Union(node) => match_field(ident, &node.name, ast, &mut path),
            Traversal::Enum(node) => match_field(ident, &node.name, ast, &mut path),
            Traversal::Const(node) => match_field(ident, &node.name, ast, &mut path),
            _ => None,
        };
        if loc.is_some() {
            return loc;
        }
    }
    None
}
