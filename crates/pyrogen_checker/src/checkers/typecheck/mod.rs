use std::path::Path;

use pyrogen_python_ast::PySourceType;
use pyrogen_python_index::Indexer;
use pyrogen_source_file::Locator;
use rustpython_ast::{text_size::TextRange, Constant, Expr, Stmt, StmtAnnAssign, Suite};

use crate::{
    registry::{Diagnostic, DiagnosticKind, ErrorCode},
    settings::{flags, CheckerSettings},
    type_ignore::TypeIgnoreMapping,
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
    noqa_line_for: &TypeIgnoreMapping,
    settings: &CheckerSettings,
    noqa: flags::TypeIgnore,
    path: &Path,
    package: Option<&Path>,
    source_type: PySourceType,
) -> Vec<Diagnostic> {
    let mut diagnostics: Vec<Diagnostic> = vec![];
    if !settings.table.enabled(ErrorCode::GeneralTypeError) {
        return diagnostics;
    }
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
                                    let value_type: Option<&str> = match constant.value {
                                        Constant::Bool(_) => Some("bool"),
                                        Constant::Float(_) => Some("float"),
                                        Constant::Str(_) => Some("str"),
                                        Constant::Complex { .. } => Some("complex"),
                                        Constant::None => Some("None"),
                                        Constant::Tuple(_) => Some("tuple"),
                                        _ => None,
                                    };
                                    if let Some(value_type) = value_type {
                                        diagnostics.push(Diagnostic::new(
                                            type_mismatch("int".into(), value_type.into()),
                                            *range,
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
