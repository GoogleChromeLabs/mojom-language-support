use mojom_syntax::Error as SyntaxError;
use mojom_syntax::{Module, MojomFile, Statement};

#[derive(Debug)]
pub(crate) enum Error {
    SyntaxError(SyntaxError),
    MultipleModuleError(String),
}

impl From<SyntaxError> for Error {
    fn from(err: SyntaxError) -> Error {
        Error::SyntaxError(err)
    }
}

pub(crate) struct Analysis {
    pub(crate) module: Option<Module>,
}

pub(crate) fn do_semantics_analysis(mojom: &MojomFile) -> std::result::Result<Analysis, Error> {
    let mut module = None;
    for stmt in &mojom.stmts {
        match stmt {
            Statement::Module(stmt) => {
                if module.is_some() {
                    let msg = format!(
                        "Found more than one module stmt: {:?} and {:?}",
                        module, stmt
                    );
                    return Err(Error::MultipleModuleError(msg));
                }
                module = Some(stmt.clone());
            }
            _ => continue,
        }
    }
    Ok(Analysis { module: module })
}
