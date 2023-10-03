use std::fmt::Debug;

use pyrogen_python_trivia::CommentRanges;
use rustpython_parser::lexer::{lex, LexicalError};
use rustpython_parser::text_size::TextRange;
use rustpython_parser::{Mode, Tok};

#[derive(Debug, Clone, Default)]
pub struct CommentRangesBuilder {
    ranges: Vec<TextRange>,
}

impl CommentRangesBuilder {
    pub fn visit_token(&mut self, token: &Tok, range: TextRange) {
        if let Tok::Comment(..) = token {
            self.ranges.push(range);
        }
    }

    pub fn finish(self) -> CommentRanges {
        CommentRanges::new(self.ranges)
    }
}

/// Helper method to lex and extract comment ranges
pub fn tokens_and_ranges(
    source: &str,
) -> Result<(Vec<(Tok, TextRange)>, CommentRanges), LexicalError> {
    let mut tokens = Vec::new();
    let mut comment_ranges = CommentRangesBuilder::default();

    for result in lex(source, Mode::Module) {
        let (token, range) = result?;

        comment_ranges.visit_token(&token, range);
        tokens.push((token, range));
    }

    let comment_ranges = comment_ranges.finish();
    Ok((tokens, comment_ranges))
}
