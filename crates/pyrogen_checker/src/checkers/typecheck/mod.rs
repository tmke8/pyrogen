use std::path::Path;

use pyrogen_diagnostics::{Diagnostic, DiagnosticKind, Violation};
use pyrogen_python_ast::PySourceType;
use pyrogen_python_index::Indexer;
use pyrogen_source_file::Locator;
use rustpython_ast::{text_size::TextRange, Constant, Expr, Stmt, StmtAnnAssign, Suite};

use crate::{
    settings::{flags, CheckerSettings},
    type_ignore::NoqaMapping,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeMismatch {
    var_type: String,
    value_type: String,
}

impl Violation for TypeMismatch {
    fn message(&self) -> String {
        format!(
            "Type mismatch: variable is of type {}, but value is of type {}",
            self.var_type, self.value_type
        )
    }
}

// The following implementation is done by a macro in ruff.
// TODO: It seems to me you can achieve the same thing with just a function which
// composes the message and which also sets the "rule", so it can return a fully
// formed DiagnosticKind.
impl From<TypeMismatch> for DiagnosticKind {
    fn from(value: TypeMismatch) -> Self {
        Self {
            body: Violation::message(&value),
            name: "general".to_string(), // TODO: instead of using the rule name, reference the rule
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn check_ast(
    python_ast: &Suite<TextRange>,
    locator: &Locator,
    indexer: &Indexer,
    noqa_line_for: &NoqaMapping,
    settings: &CheckerSettings,
    noqa: flags::Noqa,
    path: &Path,
    package: Option<&Path>,
    source_type: PySourceType,
) -> Vec<Diagnostic> {
    let mut diagnostics: Vec<Diagnostic> = vec![];
    for stmt in python_ast {
        match stmt {
            Stmt::AnnAssign(StmtAnnAssign {
                range,
                target,
                annotation,
                value,
                simple,
            }) => match **annotation {
                Expr::Name(ref name) => {
                    if name.id.as_str() == "int" {
                        match value {
                            Some(value) => match **value {
                                Expr::Constant(ref constant) => {
                                    if let Constant::Int(_) = constant.value {
                                    } else {
                                        diagnostics.push(Diagnostic::new(
                                            TypeMismatch {
                                                var_type: "int".into(),
                                                value_type: "str".into(),
                                            },
                                            range.clone(),
                                        ))
                                    }
                                }
                                _ => {}
                            },
                            None => {}
                        }
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
    diagnostics
}
