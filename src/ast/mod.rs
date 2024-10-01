mod document;
pub use document::Document;

mod identifier;
pub use identifier::Identifier;

mod import;
pub use import::Import;

mod expr;
pub use expr::{BinaryOp, Expr, Number, UnaryOp};

mod function;
pub use function::{ArgDef, ArgDefs, CallArgs, FuncDef, NamedCallArg};

use pest::iterators::{Pair, Pairs};

use crate::{ParseError, Rule};

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

pub(crate) type ParseResult<'src, T> = Result<T, ParseError<'src>>;