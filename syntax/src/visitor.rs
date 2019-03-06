use crate::syntax::*;

pub trait Visitor {
    fn is_done(&self) -> bool {
        false
    }

    fn visit_mojom(&mut self, _: &MojomFile) {}
    fn leave_mojom(&mut self, _: &MojomFile) {}
    fn visit_module(&mut self, _: &Module) {}
    fn visit_import(&mut self, _: &Import) {}
    fn visit_interface(&mut self, _: &Interface) {}
    fn leave_interface(&mut self, _: &Interface) {}
    fn visit_method(&mut self, _: &Method) {}
    fn visit_struct(&mut self, _: &Struct) {}
    fn leave_struct(&mut self, _: &Struct) {}
    fn visit_struct_field(&mut self, _: &StructField) {}
    fn visit_union(&mut self, _: &Union) {}
    fn visit_enum(&mut self, _: &Enum) {}
    fn visit_const(&mut self, _: &Const) {}
}

pub trait Element {
    fn accept<V: Visitor>(&self, visitor: &mut V);
}

impl Element for MojomFile {
    fn accept<V: Visitor>(&self, visitor: &mut V) {
        visitor.visit_mojom(&self);
        for stmt in &self.stmts {
            if visitor.is_done() {
                break;
            }
            match stmt {
                Statement::Module(ref stmt) => stmt.accept(visitor),
                Statement::Import(ref stmt) => stmt.accept(visitor),
                Statement::Interface(ref stmt) => stmt.accept(visitor),
                Statement::Struct(ref stmt) => stmt.accept(visitor),
                Statement::Union(ref stmt) => stmt.accept(visitor),
                Statement::Enum(ref stmt) => stmt.accept(visitor),
                Statement::Const(ref stmt) => stmt.accept(visitor),
            }
        }
        visitor.leave_mojom(&self);
    }
}

impl Element for Module {
    fn accept<V: Visitor>(&self, visitor: &mut V) {
        visitor.visit_module(&self);
    }
}

impl Element for Import {
    fn accept<V: Visitor>(&self, visitor: &mut V) {
        visitor.visit_import(&self);
    }
}

impl Element for Union {
    fn accept<V: Visitor>(&self, visitor: &mut V) {
        visitor.visit_union(&self);
    }
}

impl Element for Enum {
    fn accept<V: Visitor>(&self, visitor: &mut V) {
        visitor.visit_enum(&self);
    }
}

impl Element for Const {
    fn accept<V: Visitor>(&self, visitor: &mut V) {
        visitor.visit_const(&self);
    }
}

impl Element for Interface {
    fn accept<V: Visitor>(&self, visitor: &mut V) {
        visitor.visit_interface(&self);
        for member in &self.members {
            if visitor.is_done() {
                break;
            }
            match member {
                InterfaceMember::Const(member) => visitor.visit_const(member),
                InterfaceMember::Enum(member) => visitor.visit_enum(member),
                InterfaceMember::Method(member) => visitor.visit_method(member),
            }
        }
        visitor.leave_interface(&self);
    }
}

impl Element for Struct {
    fn accept<V: Visitor>(&self, visitor: &mut V) {
        visitor.visit_struct(&self);
        for member in &self.members {
            if visitor.is_done() {
                break;
            }
            match member {
                StructBody::Const(member) => visitor.visit_const(member),
                StructBody::Enum(member) => visitor.visit_enum(member),
                StructBody::Field(member) => visitor.visit_struct_field(member),
            }
        }
        visitor.leave_struct(&self);
    }
}
