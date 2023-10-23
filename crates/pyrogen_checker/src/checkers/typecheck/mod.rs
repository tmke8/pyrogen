use std::path::Path;

use pyrogen_python_ast::PySourceType;
use pyrogen_python_index::Indexer;
use pyrogen_source_file::Locator;
use rustpython_ast::{text_size::TextRange, Constant, Expr, Stmt, StmtAnnAssign, Suite};

use crate::{
    registry::{Diagnostic, DiagnosticKind, ErrorCode},
    settings::{flags, CheckerSettings},
    type_ignore::NoqaMapping,
};

fn type_mismatch(var_type: String, value_type: String) -> DiagnosticKind {
    DiagnosticKind {
        body: format!(
            "Type mismatch: variable is of type {}, but value is of type {}",
            var_type, value_type
        ),
        error_code: ErrorCode::GeneralTypeError,
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
                                            type_mismatch("int".into(), "str".into()),
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
