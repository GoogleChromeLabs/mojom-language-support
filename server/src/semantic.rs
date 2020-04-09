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

use mojom_syntax::{Module, MojomFile};

use super::diagnostic;

pub(crate) struct Analysis {
    pub(crate) module: Option<Module>,
    pub(crate) diagnostics: Vec<lsp_types::Diagnostic>,
}

fn partial_text<'a>(text: &'a str, range: &mojom_syntax::Range) -> &'a str {
    &text[range.start..range.end]
}

fn find_module(
    text: &str,
    mojom: &MojomFile,
    diagnostics: &mut Vec<lsp_types::Diagnostic>,
) -> Option<Module> {
    let mut module: Option<Module> = None;
    for stmt in &mojom.stmts {
        match stmt {
            mojom_syntax::Statement::Module(stmt) => {
                if let Some(ref module) = module {
                    let message = format!(
                        "Found more than one module statement: {} and {}",
                        partial_text(&text, &module.name),
                        partial_text(&text, &stmt.name)
                    );
                    let start = mojom_syntax::line_col(text, stmt.name.start).unwrap();
                    let end = mojom_syntax::line_col(text, stmt.name.end).unwrap();
                    let range = diagnostic::into_lsp_range(&start, &end);
                    let diagnostic = diagnostic::create_diagnostic(range, message);
                    diagnostics.push(diagnostic);
                } else {
                    module = Some(stmt.clone());
                }
            }
            _ => (),
        }
    }
    module
}

pub(crate) fn check_semantics(text: &str, mojom: &MojomFile) -> Analysis {
    let mut diagnostics = Vec::new();
    let module = find_module(text, mojom, &mut diagnostics);
    Analysis {
        module: module,
        diagnostics: diagnostics,
    }
}
