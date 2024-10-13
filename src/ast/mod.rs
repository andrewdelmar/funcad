use std::{
    fmt::{Debug, Display},
    ops::Deref,
};

use pest::{
    iterators::{Pair, Pairs},
    Span,
};

use crate::{error::ParseResult, ParseError, Rule};

mod document;
pub use document::Document;

mod identifier;
pub use identifier::{FuncName, Identifier, SpannedFuncName, SpannedIdentifier};

mod import;
pub use import::{Import, SpannedImport};

mod expr;
pub use expr::{
    BinaryExpr, BinaryOp, Expr, FuncCallExpr, Number, SpannedBinaryExpr, SpannedExpr,
    SpannedFuncCallExpr, SpannedNumber, SpannedUnaryExpr, UnaryExpr, UnaryOp,
};

mod function;
pub use function::{
    ArgDef, ArgDefs, CallArgs, FuncDef, NamedCallArg, SpannedArgDef, SpannedArgDefs,
    SpannedCallArgs, SpannedFuncDef, SpannedNamedCallArg,
};

// This is a conveniece trait to return an error if case the code doesn't match
// the grammar and we unwrap somewhere we shouldn't.
pub(crate) trait TryNext<'src, I> {
    fn try_next(&mut self) -> ParseResult<'src, I>;
}

impl<'src> TryNext<'src, Pair<'src, Rule>> for Pairs<'src, Rule> {
    fn try_next(&mut self) -> ParseResult<'src, Pair<'src, Rule>> {
        self.next().ok_or(ParseError::ExpectedUnwrap)
    }
}

#[derive(Clone, Debug)]
pub struct Spanned<'src, T>
where
    T: Clone + Debug,
{
    pub inner: T,
    pub span: Span<'src>,
}

impl<'src, T> Copy for Spanned<'src, T> where T: Clone + Debug + Copy {}

impl<'src, T> Display for Spanned<'src, T>
where
    T: Clone + Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (line, col) = self.span.start_pos().line_col();
        write!(
            f,
            "\"{0}\" on line {1} col {2}",
            self.span.as_str(),
            line,
            col
        )
    }
}

impl<'src, T> TryFrom<Pair<'src, Rule>> for Spanned<'src, T>
where
    T: Clone + Debug + TryFrom<Pair<'src, Rule>>,
{
    type Error = T::Error;

    fn try_from(value: Pair<'src, Rule>) -> Result<Self, Self::Error> {
        let span = value.as_span();
        Ok(Self {
            inner: T::try_from(value)?,
            span,
        })
    }
}

pub(crate) trait ToSpanned: Clone + Debug {
    fn spanned<'src>(&self, span: &Span<'src>) -> Spanned<'src, Self> {
        Spanned {
            inner: self.clone(),
            span: *span,
        }
    }
}

impl<'src, T> ToSpanned for T where T: Clone + Debug {}

impl<'src, T> Deref for Spanned<'src, T>
where
    T: Clone + Debug,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
