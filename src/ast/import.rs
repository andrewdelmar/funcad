use pest::{iterators::Pair, Span};

use crate::{ParseError, Rule};

use super::{Spanned, TryNext};

/// An import directive.
#[derive(Clone, Copy, Debug)]
pub struct Import<'src> {
    pub alias: &'src str,
    pub file: &'src str,
    pub span: Span<'src>,
}

/// [`Import`], but [`Spanned`].
pub type SpannedImport<'src> = Spanned<'src, Import<'src>>;

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
