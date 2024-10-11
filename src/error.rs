use std::{io::Error as IoError, num::ParseFloatError};

use pest::{error::Error as PestError, Span};
use thiserror::Error;

use crate::{ast::*, FQPath, Rule};

/// An error in parsing a document.
#[derive(Error, Debug)]
pub enum ParseError<'src> {
    #[error("Parsing error:\n{0}")]
    Parse(#[from] PestError<Rule>),
    #[error("Duplicate imports:\n\t{0}\nthen\n\t{1}")]
    DumplicateImport(Import<'src>, Import<'src>),
    #[error("Import path is above entry point: \n\t{0}\n")]
    ImportNotInDir(Import<'src>),
    #[error("Duplicate function definition:\n\t{0}\n\t\tthen\n\t{1}")]
    DuplicateFuncDef(FuncDef<'src>, FuncDef<'src>),
    #[error("Float parsing error:\n\t{0}")]
    Float(ParseFloatError, Span<'src>),
    #[error("Duplicate argument definition:\n\t{0}\nthen\n\t{1}")]
    DuplicateArgDef(ArgDef<'src>, ArgDef<'src>),
    #[error("Duplicate named argument {0} then {1}")]
    DuplicateNamedArgument(NamedCallArg<'src>, NamedCallArg<'src>),
    #[error("IO Error \"{0}\"")]
    IO(#[from] IoError),
    #[error("Entry point is not a file")]
    InvalidMain,

    // These errors shouldn't occur.
    #[error("An expected field was missing from the parse tree")]
    ExpectedUnwrap,
    #[error("An unexpected field type was encountered in the parse tree")]
    UnexpectedFieldType,
}

pub(crate) type ParseResult<'src, T> = Result<T, ParseError<'src>>;

/// An error in evaluating a function.
#[derive(Error, Debug)]
pub enum EvalError<'src> {
    #[error("Parsing error:\n{0}")]
    Parse(ParseError<'src>),

    #[error("Function not found:\n\t{0}")]
    FuncCallFuncNotFound(FuncCallExpr<'src>),
    #[error("Import document not found:\n\t{0}")]
    FuncCallImportDocNotFound(FuncCallExpr<'src>),
    #[error("Function call:\n\t{0}\nrefers to import \"{1}\" that is not in document")]
    FuncCallImportNotInDoc(FuncCallExpr<'src>, Identifier<'src>),
    #[error("Missing arguments\n\t\"{}\"\nin call:\n\t{1}", .0.iter().map(ArgDef::to_string).collect::<Vec<_>>().join("\n\t"))]
    FuncCallMissingArguments(Vec<ArgDef<'src>>, FuncCallExpr<'src>),
    #[error("Too many arguments in call\n\t\"{0}\"\nof funtion:\n\t{1}")]
    FuncCallTooManyArgs(FuncCallExpr<'src>, FuncDef<'src>),
    #[error("Extra named args:\n\t{0}\nin call:\n\t{}\nof funtion:\n\t{2}", .1.iter().map(NamedCallArg::to_string).collect::<Vec<_>>().join(","))]
    FuncCallExtraNamedArgs(FuncCallExpr<'src>, Vec<NamedCallArg<'src>>, FuncDef<'src>),
    #[error("Function is infinitely recursive:\n\t{0}")]
    FuncCallInfiniteRecursion(FuncDef<'src>),

    #[error("Document of function not found:\n\t{0}")]
    EvalFuncDocNotFound(FQPath),
    #[error("Function \"{0}\" not found")]
    EvalFuncFuncNotFound(String),
    #[error("Function to evaluate has arguments without default values:\n\t{0}")]
    EvalFuncHasArgs(FuncDef<'src>),

    #[error("An arithmetic operation resulted in a non finite result:\n\t{0}")]
    BinaryExprNotFinite(BinaryExpr<'src>),
}

// The #[from] macro can't handle non static lifetimes.
impl<'src> From<ParseError<'src>> for EvalError<'src> {
    fn from(value: ParseError<'src>) -> Self {
        Self::Parse(value)
    }
}

pub(crate) type EvalResult<'src, T> = Result<T, EvalError<'src>>;
