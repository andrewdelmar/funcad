use std::fmt::Display;

use pest::{iterators::Pair, Span};

use crate::{ParseError, Rule};

use super::TryNext;

/// An import directive.
#[derive(Clone, Copy, Debug)]
pub struct Import<'src> {
    pub alias: &'src str,
    pub file: &'src str,
    pub span: Span<'src>,
}

impl<'src> Display for Import<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let line = self.span.start_pos().line_col().0;
        write!(f, "\"{0}\" on line {1}", self.span.as_str(), line)
    }
}

impl<'src> TryFrom<Pair<'src, Rule>> for Import<'src> {
    type Error = ParseError<'src>;

    fn try_from(value: Pair<'src, Rule>) -> Result<Self, Self::Error> {
        let span = value.as_span();
        let id = value.into_inner().try_next()?;
        match id.as_rule() {
            Rule::file_name => {
                let file = id.as_str();
                let alias = id
                    .into_inner()
                    .last()
                    .ok_or(ParseError::ExpectedUnwrap)?
                    .as_str();
                Ok(Self { alias, file, span })
            }
            _ => Err(ParseError::UnexpectedFieldType),
        }
    }
}
