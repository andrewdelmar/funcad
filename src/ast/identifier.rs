use std::fmt::Display;

use pest::{iterators::Pair, Span};

use crate::{ParseError, Rule};

/// A single instance of an identifier such as a function or parameter name in a
/// function call or definition.
#[derive(Clone, Copy, Debug)]
pub struct Identifier<'src> {
    pub text: &'src str,
    pub span: Span<'src>,
}

impl<'src> Display for Identifier<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (line, col) = self.span.start_pos().line_col();
        write!(
            f,
            "\"{0}\" on line {1}, col {2}",
            self.span.as_str(),
            line,
            col
        )
    }
}

impl<'src> TryFrom<Pair<'src, Rule>> for Identifier<'src> {
    type Error = ParseError<'src>;

    fn try_from(value: Pair<'src, Rule>) -> Result<Self, Self::Error> {
        Ok(Identifier {
            text: value.as_str(),
            span: value.as_span(),
        })
    }
}

/// A potentially qualified function name in a function call.
#[derive(Clone, Debug)]
pub struct FuncName<'src> {
    pub import_part: Option<Identifier<'src>>,
    pub name_part: Identifier<'src>,
    pub span: Span<'src>,
}

impl<'src> Display for FuncName<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (line, col) = self.span.start_pos().line_col();
        write!(
            f,
            "\"{0}\" on line {1}, col {2}",
            self.span.as_str(),
            line,
            col
        )
    }
}

impl<'src> TryFrom<Pair<'src, Rule>> for FuncName<'src> {
    type Error = ParseError<'src>;

    fn try_from(value: Pair<'src, Rule>) -> Result<Self, Self::Error> {
        let span = value.as_span();

        // func_name = ${ identifier ~ ("." ~ identifier)? }
        let mut inner = value.into_inner();
        match (inner.next(), inner.next()) {
            (Some(name), None) => Ok(Self {
                import_part: None,
                name_part: name.try_into()?,
                span,
            }),
            (Some(import), Some(name)) => Ok(Self {
                import_part: Some(import.try_into()?),
                name_part: name.try_into()?,
                span,
            }),
            _ => Err(ParseError::ExpectedUnwrap),
        }
    }
}
