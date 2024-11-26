use std::{fmt::Display, io::Error as IoError, num::ParseFloatError};

use pest::{error::Error as PestError, Span};
use thiserror::Error;

use crate::{ast::*, eval::ContextEntry, FQPath, Rule};

/// An error in parsing a document.
#[derive(Error, Debug)]
pub enum ParseError<'src> {
    #[error("Parsing error:\n{0}")]
    Parse(#[from] PestError<Rule>),
    #[error("Duplicate imports:\n\t{0}\nthen\n\t{1}")]
    DumplicateImport(SpannedImport<'src>, SpannedImport<'src>),
    #[error("Import path is above entry point: \n\t{0}\n")]
    ImportNotInDir(SpannedImport<'src>),
    #[error("Duplicate function definition:\n\t{0}\n\t\tthen\n\t{1}")]
    DuplicateFuncDef(SpannedFuncDef<'src>, SpannedFuncDef<'src>),
    #[error("Float parsing error:\n\t{0}")]
    Float(ParseFloatError, Span<'src>),
    #[error("Duplicate argument definition:\n\t{0}\nthen\n\t{1}")]
    DuplicateArgDef(SpannedArgDef<'src>, SpannedArgDef<'src>),
    #[error("Duplicate named argument {0} then {1}")]
    DuplicateNamedArgument(SpannedNamedCallArg<'src>, SpannedNamedCallArg<'src>),
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
#[derive(Debug)]
pub struct EvalError<'src> {
    pub error_type: EvalErrorType<'src>,
    pub(crate) context_entries: Vec<ContextEntry>,
}

impl<'src> Display for EvalError<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.error_type)?;
        for entry in &self.context_entries {
            write!(f, "\n\t{}", entry)?;
        }
        Ok(())
    }
}

impl<'src> From<ParseError<'src>> for EvalError<'src> {
    fn from(value: ParseError<'src>) -> Self {
        Self {
            error_type: EvalErrorType::Parse(value),
            context_entries: Vec::default(),
        }
    }
}

pub(crate) type EvalResult<'src, T> = Result<T, EvalError<'src>>;

// The type of an EvalError.
#[derive(Error, Debug)]
pub enum EvalErrorType<'src> {
    #[error("Parsing error:\n{0}")]
    Parse(ParseError<'src>),

    #[error("Numeric expression was not finite")]
    NumExprNotFinite,

    #[error("The import \"{name}\" was not found")]
    ImportNotFound { name: String },
    #[error("The document \"{path}\" was not found")]
    DocNotFound { path: FQPath },
    #[error("The function \"{name}\" was not found")]
    FuncNotFound { name: String },
    #[error("The argument \"{name}\" was not found")]
    ArgNotFound { name: String },
    #[error("The built-in function \"{name}\" was not found")]
    BuiltInNotFound { name: String },

    #[error("Too many args in function call")]
    TooManyArgs,
    #[error("No argument named \"{name}\" in function definition")]
    InvalidNamedArg { name: String },
    #[error("No supplied or default value of argument \"{name}\"")]
    NoSuppliedOrDefaultArg { name: String },
    #[error("The supplied argument \"{name}\" is the wrong type: expected a \"{expected}\"; got a \"{got}\"")]
    ArgWrongType {
        name: String,
        expected: &'static str,
        got: &'static str,
    },

    #[error("Cannot perform {op} between a {lhs_type} and a {rhs_type}")]
    BinaryOpWrongTypes {
        op: &'static str,
        lhs_type: &'static str,
        rhs_type: &'static str,
    },

    #[error("Infinite recursion")]
    InfiniteRecursion,

    #[error("Invalid Solid ID")]
    InvalidSolidId,
}
