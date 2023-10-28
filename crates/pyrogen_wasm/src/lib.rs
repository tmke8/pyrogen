use std::path::Path;

use js_sys::Error;
use pyrogen_checker::settings::code_table::MessageKind;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use rustpython_ast::{Mod, Ranged};
use rustpython_parser::lexer::LexResult;
use rustpython_parser::{parse_tokens, Mode};

use pyrogen_checker::checker::{check_path, CheckerResult};
use pyrogen_checker::directives;
use pyrogen_checker::settings::types::PythonVersion;
use pyrogen_checker::settings::{flags, DEFAULT_ERRORS, DEFAULT_WARNINGS};
use pyrogen_checker::source_kind::SourceKind;
use pyrogen_python_ast::{AsMode, PySourceType};
use pyrogen_python_index::{CommentRangesBuilder, Indexer};
use pyrogen_python_trivia::CommentRanges;
use pyrogen_source_file::{Locator, SourceLocation};
use pyrogen_workspace::configuration::Configuration;
use pyrogen_workspace::options::Options;
use pyrogen_workspace::Settings;

#[wasm_bindgen(typescript_custom_section)]
const TYPES: &'static str = r#"
export interface Diagnostic {
    code: string;
    message: string;
    location: {
        row: number;
        column: number;
    };
    end_location: {
        row: number;
        column: number;
    };
    kind: "error" | "warning";
};
"#;

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
pub struct ExpandedMessage {
    pub code: String,
    pub message: String,
    pub location: SourceLocation,
    pub end_location: SourceLocation,
    pub kind: MessageKind,
}

#[wasm_bindgen(start)]
pub fn run() {
    use log::Level;

    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function at least once during initialization, and then
    // we will get better error messages if our code ever panics.
    //
    // For more details see
    // https://github.com/rustwasm/console_error_panic_hook#readme
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();

    console_log::init_with_level(Level::Debug).expect("Initializing logger went wrong.");
}

#[wasm_bindgen]
pub struct Workspace {
    settings: Settings,
}

#[wasm_bindgen]
impl Workspace {
    pub fn version() -> String {
        pyrogen_checker::VERSION.to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(options: JsValue) -> Result<Workspace, Error> {
        let options: Options = serde_wasm_bindgen::from_value(options).map_err(into_error)?;
        let configuration =
            Configuration::from_options(options, Path::new(".")).map_err(into_error)?;
        let settings = configuration
            .into_settings(Path::new("."))
            .map_err(into_error)?;

        Ok(Workspace { settings })
    }

    #[wasm_bindgen(js_name = defaultSettings)]
    pub fn default_settings() -> Result<JsValue, Error> {
        serde_wasm_bindgen::to_value(&Options {
            // Propagate defaults.
            ignore: Some(Vec::default()),
            error: Some(DEFAULT_ERRORS.to_vec()),
            warning: Some(DEFAULT_WARNINGS.to_vec()),
            target_version: Some(PythonVersion::default()),
            // Ignore a bunch of options that don't make sense in a single-file editor.
            cache_dir: None,
            exclude: None,
            extend_error: None,
            extend_warning: None,
            force_exclude: None,
            output_format: None,
            include: None,
            namespace_packages: None,
            per_file_ignores: None,
            respect_gitignore: None,
            src: None,
            ..Options::default()
        })
        .map_err(into_error)
    }

    pub fn check(&self, contents: &str) -> Result<JsValue, Error> {
        let source_type = PySourceType::default();

        // TODO(dhruvmanila): Support Jupyter Notebooks
        let source_kind = SourceKind::new(contents.to_string());

        // Tokenize once.
        let tokens: Vec<LexResult> =
            rustpython_parser::lexer::lex(contents, source_type.as_mode()).collect::<Vec<_>>();

        // Map row and column locations to byte slices (lazily).
        let locator = Locator::new(contents);

        // Extra indices from the code.
        let indexer = Indexer::from_tokens(&tokens, &locator);

        // Extract the `# noqa` and `# isort: skip` directives from the source.
        let directives = directives::extract_noqa_line_for(&tokens, &locator, &indexer);

        // Generate checks.
        let CheckerResult {
            data: (diagnostics, _imports),
            ..
        } = check_path(
            Path::new("<filename>"),
            None,
            tokens,
            &locator,
            // &stylist,
            &indexer,
            &directives,
            &self.settings.checker,
            flags::TypeIgnore::Enabled,
            &source_kind,
            source_type,
        );

        let source_code = locator.to_source_code();

        let messages: Vec<ExpandedMessage> = diagnostics
            .into_iter()
            .map(|message| {
                let start_location = source_code.source_location(message.start());
                let end_location = source_code.source_location(message.end());
                let code = message.kind.error_code;

                ExpandedMessage {
                    code: code.to_string(),
                    message: message.kind.body,
                    location: start_location,
                    end_location,
                    kind: if self.settings.checker.table.is_warning(code) {
                        MessageKind::Warning
                    } else {
                        MessageKind::Error
                    },
                }
            })
            .collect();

        serde_wasm_bindgen::to_value(&messages).map_err(into_error)
    }

    /// Parses the content and returns its AST
    pub fn parse(&self, contents: &str) -> Result<String, Error> {
        let parsed = rustpython_parser::parse(contents, Mode::Module, ".").map_err(into_error)?;

        Ok(format!("{parsed:#?}"))
    }

    pub fn tokens(&self, contents: &str) -> Result<String, Error> {
        let tokens: Vec<_> = rustpython_parser::lexer::lex(contents, Mode::Module).collect();

        Ok(format!("{tokens:#?}"))
    }
}

pub(crate) fn into_error<E: std::fmt::Display>(err: E) -> Error {
    Error::new(&err.to_string())
}

struct ParsedModule<'a> {
    source_code: &'a str,
    module: Mod,
    comment_ranges: CommentRanges,
}

impl<'a> ParsedModule<'a> {
    fn from_source(source: &'a str) -> Result<Self, Error> {
        let tokens: Vec<_> = rustpython_parser::lexer::lex(source, Mode::Module).collect();
        let mut comment_ranges = CommentRangesBuilder::default();

        for (token, range) in tokens.iter().flatten() {
            comment_ranges.visit_token(token, *range);
        }
        let comment_ranges = comment_ranges.finish();
        let module = parse_tokens(tokens, Mode::Module, ".").map_err(into_error)?;

        Ok(Self {
            source_code: source,
            comment_ranges,
            module,
        })
    }
}
