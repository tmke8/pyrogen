pub use code_selector::ErrorCodeSelector;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod checker;
mod checkers;
pub mod code_selector;
pub mod directives;
pub mod fs;
pub mod line_width;
pub mod logging;
pub mod message;
pub mod packaging;
pub mod pyproject_toml;
pub mod registry;
pub mod settings;
pub mod source_kind;
mod type_ignore;

#[cfg(any(test, fuzzing))]
pub mod test;
