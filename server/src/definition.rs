use lsp_types::{Location, Position, Range};

use mojom_syntax::{Element, Visitor};

use crate::mojomast::MojomAst;

pub(crate) fn create_lsp_range(ast: &MojomAst, field: &mojom_syntax::Range) -> Range {
    let (line, col) = ast.line_col(field.start);
    let start = Position::new((line - 1) as u64, (col - 1) as u64);
    let (line, col) = ast.line_col(field.end);
    let end = Position::new((line - 1) as u64, (col - 1) as u64);
    Range::new(start, end)
}

struct DefinitionVisitor<'a> {
    ast: &'a MojomAst,
    ident: &'a str,
    path: Vec<&'a str>,
    found: Option<Location>,
}

impl<'a> DefinitionVisitor<'a> {
    fn match_field(&mut self, field: &mojom_syntax::Range) {
        assert!(self.found.is_none());
        let name = self.ast.text(field);
        self.path.push(name);
        let ident = self.path.join(".");
        self.path.pop();
        if ident == self.ident {
            let range = create_lsp_range(self.ast, field);
            self.found = Some(Location::new(self.ast.uri.clone(), range));
        }
    }
}

impl<'a> Visitor for DefinitionVisitor<'a> {
    fn is_done(&self) -> bool {
        self.found.is_some()
    }

    fn visit_interface(&mut self, elem: &mojom_syntax::Interface) {
        self.match_field(&elem.name);
        let name = self.ast.text(&elem.name);
        self.path.push(name);
    }

    fn leave_interface(&mut self, _: &mojom_syntax::Interface) {
        self.path.pop();
    }

    fn visit_struct(&mut self, elem: &mojom_syntax::Struct) {
        self.match_field(&elem.name);
        let name = self.ast.text(&elem.name);
        self.path.push(name);
    }

    fn leave_struct(&mut self, _: &mojom_syntax::Struct) {
        self.path.pop();
    }

    fn visit_union(&mut self, elem: &mojom_syntax::Union) {
        self.match_field(&elem.name);
    }

    fn visit_enum(&mut self, elem: &mojom_syntax::Enum) {
        self.match_field(&elem.name);
    }

    fn visit_const(&mut self, elem: &mojom_syntax::Const) {
        self.match_field(&elem.name);
    }
}

pub(crate) fn find_definition(ident: &str, ast: &MojomAst) -> Option<Location> {
    let mut v = DefinitionVisitor {
        ast: ast,
        ident: ident,
        path: Vec::new(),
        found: None,
    };
    ast.mojom.accept(&mut v);
    v.found
}
