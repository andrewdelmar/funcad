use std::{fmt::Display, io::Error as IoError, num::ParseFloatError};

use pest::{error::Error as PestError, Span};
use thiserror::Error;

use crate::{ast::*, Rule};

#[derive(Error, Debug)]
pub enum ParseError<'src> {
    #[error("Parsing error:\n{0}")]
    Parse(#[from] PestError<Rule>),
    #[error("Duplicate imports:\n\t{0}\nthen\n\t{1}")]
    DumplicateImport(Import<'src>, Import<'src>),
    #[error("Import path is above entry point: \n\t{0}\n")]
    ImportNotInDir(Import<'src>),
    #[error("Duplicate function definition {0} then {1}")]
    DuplicateFuncDef(FuncDef<'src>, FuncDef<'src>),
    #[error("Float parsing error:\n{0}")]
    Float(ParseFloatError, Span<'src>),
    #[error("Duplicate named argument {0} then {1}")]
    DuplicateNamedArgument(NamedCallArg<'src>, NamedCallArg<'src>),
    #[error("IO Error \"{0}\"")]
    IO(#[from] IoError),
    #[error("Entry point is listed as filesystem root")]
    MainRoot,
    // These errors shouldn't occur.
    #[error("An expected field was missing from the parse tree")]
    ExpectedUnwrap,
    #[error("An unexpected field type was encountered in the parse tree")]
    UnexpectedFieldType,
}

#[derive(Debug)]
pub struct ErrSpan<'src>(Span<'src>);

impl<'src> From<Span<'src>> for ErrSpan<'src> {
    fn from(value: Span<'src>) -> Self {
        Self(value)
    }
}

impl<'src> Display for ErrSpan<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (line, col) = self.0.start_pos().line_col();
        write!(f, "\"{}\" at line {} col {}", self.0.as_str(), line, col)
    }
}
