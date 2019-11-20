use crate::syntax::*;

#[derive(Debug, PartialEq)]
pub enum Traversal<'a> {
    EnterMojomFile(&'a MojomFile),
    LeaveMojomFile(&'a MojomFile),
    EnterInterface(&'a Interface),
    LeaveInterface(&'a Interface),
    EnterStruct(&'a Struct),
    LeaveStruct(&'a Struct),
    Module(&'a Module),
    Import(&'a Import),
    Method(&'a Method),
    Union(&'a Union),
    Enum(&'a Enum),
    Const(&'a Const),
    StructField(&'a StructField),
}

enum Node<'a> {
    Leaf(&'a dyn Leaf),
    NonLeaf(&'a dyn NonLeaf),
}

trait Leaf {
    fn visit(&self) -> Traversal;
}

macro_rules! define_leaf {
    ($name:tt) => {
        impl Leaf for $name {
            fn visit(&self) -> Traversal {
                Traversal::$name(self)
            }
        }
    };
}

define_leaf!(Module);
define_leaf!(Import);
define_leaf!(Method);
define_leaf!(Union);
define_leaf!(Enum);
define_leaf!(Const);
define_leaf!(StructField);

trait NonLeaf {
    fn enter(&self) -> Traversal;
    fn leave(&self) -> Traversal;
    fn visit_child(&self, pos: usize) -> Option<Node>;
}

impl NonLeaf for MojomFile {
    fn enter(&self) -> Traversal {
        Traversal::EnterMojomFile(self)
    }

    fn leave(&self) -> Traversal {
        Traversal::LeaveMojomFile(self)
    }

    fn visit_child(&self, pos: usize) -> Option<Node> {
        if pos >= self.stmts.len() {
            return None;
        }
        let node = match &self.stmts[pos] {
            Statement::Module(m) => Node::Leaf(m),
            Statement::Import(i) => Node::Leaf(i),
            Statement::Interface(i) => Node::NonLeaf(i),
            Statement::Struct(s) => Node::NonLeaf(s),
            Statement::Union(u) => Node::Leaf(u),
            Statement::Enum(e) => Node::Leaf(e),
            Statement::Const(c) => Node::Leaf(c),
        };
        Some(node)
    }
}

impl NonLeaf for Interface {
    fn enter(&self) -> Traversal {
        Traversal::EnterInterface(self)
    }

    fn leave(&self) -> Traversal {
        Traversal::LeaveInterface(self)
    }

    fn visit_child(&self, pos: usize) -> Option<Node> {
        if pos >= self.members.len() {
            return None;
        }
        let node = match &self.members[pos] {
            InterfaceMember::Const(c) => Node::Leaf(c),
            InterfaceMember::Enum(e) => Node::Leaf(e),
            InterfaceMember::Method(m) => Node::Leaf(m),
        };
        Some(node)
    }
}

impl NonLeaf for Struct {
    fn enter(&self) -> Traversal {
        Traversal::EnterStruct(self)
    }

    fn leave(&self) -> Traversal {
        Traversal::LeaveStruct(self)
    }

    fn visit_child(&self, pos: usize) -> Option<Node> {
        if pos >= self.members.len() {
            return None;
        }
        let node = match &self.members[pos] {
            StructBody::Const(c) => Node::Leaf(c),
            StructBody::Enum(e) => Node::Leaf(e),
            StructBody::Field(f) => Node::Leaf(f),
        };
        Some(node)
    }
}

enum TraversalState<'a> {
    NonLeaf(&'a dyn NonLeaf),
    Child(&'a dyn NonLeaf, usize),
}

pub struct Preorder<'a> {
    stack: Vec<TraversalState<'a>>,
}

impl<'a> Iterator for Preorder<'a> {
    type Item = Traversal<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let state = match self.stack.pop() {
            Some(state) => state,
            None => return None,
        };
        let res = match state {
            TraversalState::NonLeaf(node) => {
                self.stack.push(TraversalState::Child(node, 0));
                node.enter()
            }
            TraversalState::Child(node, pos) => match node.visit_child(pos) {
                Some(child) => {
                    self.stack.push(TraversalState::Child(node, pos + 1));
                    match child {
                        Node::Leaf(leaf) => leaf.visit(),
                        Node::NonLeaf(node) => {
                            self.stack.push(TraversalState::Child(node, 0));
                            node.enter()
                        }
                    }
                }
                None => node.leave(),
            },
        };
        Some(res)
    }
}

pub fn preorder(mojom: &MojomFile) -> Preorder {
    Preorder {
        stack: vec![TraversalState::NonLeaf(mojom)],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn partial_text<'t>(text: &'t str, range: &Range) -> &'t str {
        &text[range.start..range.end]
    }

    #[test]
    fn test_preorder() {
        let input = r#"
        module test.mod;
        struct MyStruct {
            const string kMyStructString = "const_value";
        };
        interface MyInterface {
            MyMethod() => ();
        };
        "#;
        let mojom = parse(input).unwrap();

        let module = preorder(&mojom)
            .find_map(|t| match t {
                Traversal::Module(m) => Some(m),
                _ => None,
            })
            .unwrap();
        assert_eq!("test.mod", partial_text(&input, &module.name));

        let method = preorder(&mojom)
            .find_map(|t| match t {
                Traversal::Method(m) => Some(m),
                _ => None,
            })
            .unwrap();
        assert_eq!("MyMethod", partial_text(&input, &method.name));
    }
}
