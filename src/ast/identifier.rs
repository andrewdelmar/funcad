use pest::iterators::Pair;

use crate::{ParseError, Rule};

use super::Spanned;

/// A single instance of an identifier such as a function or parameter name in a
/// function call or definition.
#[derive(Clone, Copy, Debug)]
pub struct Identifier<'src> {
    pub text: &'src str,
}

/// [`Identifier`], but [`Spanned`].
pub type SpannedIdentifier<'src> = Spanned<'src, Identifier<'src>>;

impl<'src> TryFrom<Pair<'src, Rule>> for Identifier<'src> {
    type Error = ParseError<'src>;

    fn try_from(value: Pair<'src, Rule>) -> Result<Self, Self::Error> {
        Ok(Identifier {
            text: value.as_str(),
        })
    }
}

/// A potentially qualified function name in a function call.
#[derive(Clone, Debug)]
pub struct FuncName<'src> {
    pub import_part: Option<SpannedIdentifier<'src>>,
    pub name_part: SpannedIdentifier<'src>,
}

/// [`FuncName`], but [`Spanned`].
pub type SpannedFuncName<'src> = Spanned<'src, FuncName<'src>>;

impl<'src> TryFrom<Pair<'src, Rule>> for FuncName<'src> {
    type Error = ParseError<'src>;

    fn try_from(value: Pair<'src, Rule>) -> Result<Self, Self::Error> {
        // func_name = ${ identifier ~ ("." ~ identifier)? }
        let mut inner = value.into_inner();
        match (inner.next(), inner.next()) {
            (Some(name), None) => Ok(Self {
                import_part: None,
                name_part: name.try_into()?,
            }),
            (Some(import), Some(name)) => Ok(Self {
                import_part: Some(import.try_into()?),
                name_part: name.try_into()?,
            }),
            _ => Err(ParseError::ExpectedUnwrap),
        }
    }
}
