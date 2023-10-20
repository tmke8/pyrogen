use std::path::Path;

use pyrogen_diagnostics::{Diagnostic, DiagnosticKind};
use pyrogen_python_ast::PySourceType;
use pyrogen_python_index::Indexer;
use pyrogen_source_file::Locator;
use rustpython_ast::{text_size::TextRange, Constant, Expr, Stmt, StmtAnnAssign, Suite};

use crate::{
    settings::{flags, CheckerSettings},
    type_ignore::NoqaMapping,
};

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
                                            DiagnosticKind {
                                                name: "general".into(),
                                                body: "Type mismatch in assignment.".into(),
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
