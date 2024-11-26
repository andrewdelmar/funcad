use std::fmt::Display;

use pest::Span;

use crate::{error::EvalErrorType, EvalError, FQPath};

use super::{EvalResult, SpannedArgDef, SpannedFuncCallExpr, SpannedFuncDef};

#[derive(Clone, Debug)]
enum ContextEntryType {
    FuncCall { text: String },
    FuncDef { name: String },
    ArgDefault { func: String, arg: String },
    BuiltIn { name: String },
}

impl Display for ContextEntryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContextEntryType::FuncCall { text } => write!(f, "function call \"{text}\""),
            ContextEntryType::FuncDef { name } => write!(f, "body of function \"{name}\""),
            ContextEntryType::ArgDefault { func, arg } => write!(
                f,
                "evaluation of argument \"{arg}\" default of function \"{func}\""
            ),
            ContextEntryType::BuiltIn { name } => write!(f, "built-in function \"{name}\""),
        }
    }
}

#[derive(Clone, Debug)]
struct ContextPos {
    line: usize,
    col: usize,
    doc: FQPath,
}

impl ContextPos {
    fn new(span: Span, doc: &FQPath) -> Self {
        let (line, col) = span.start_pos().line_col();
        ContextPos {
            line,
            col,
            doc: doc.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ContextEntry {
    entry_type: ContextEntryType,
    pos: Option<ContextPos>,
}

impl Display for ContextEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\t in {}", self.entry_type)?;
        if let Some(pos) = &self.pos {
            write!(
                f,
                " on line {}, col {} of \"{}\"",
                pos.line, pos.col, pos.doc
            )?;
        }
        write!(f, "\n")
    }
}

#[derive(Clone, Default)]
pub(crate) enum EvalContext<'c> {
    #[default]
    None,
    Node {
        entry: ContextEntry,
        outer: &'c EvalContext<'c>,
    },
}

impl<'c> EvalContext<'c> {
    const MAX_TEXT_LEN: usize = 20;

    pub(crate) fn push_func_call(&'c self, expr: &SpannedFuncCallExpr, doc: &FQPath) -> Self {
        let mut text = expr.span.as_str().trim();
        if text.len() > Self::MAX_TEXT_LEN {
            text = &text[..Self::MAX_TEXT_LEN];
        }

        let entry = ContextEntry {
            entry_type: ContextEntryType::FuncCall { text: text.into() },
            pos: Some(ContextPos::new(expr.span, doc)),
        };

        Self::Node { entry, outer: self }
    }

    pub(crate) fn push_func_def(&'c self, expr: &SpannedFuncDef, doc: &FQPath) -> Self {
        let entry = ContextEntry {
            entry_type: ContextEntryType::FuncDef {
                name: expr.name.text.into(),
            },
            pos: Some(ContextPos::new(expr.span, doc)),
        };

        Self::Node { entry, outer: self }
    }

    pub(crate) fn push_arg_default(
        &'c self,
        arg: &SpannedArgDef,
        func: &SpannedFuncDef,
        doc: &FQPath,
    ) -> Self {
        let entry = ContextEntry {
            entry_type: ContextEntryType::ArgDefault {
                func: func.name.text.into(),
                arg: arg.name.text.into(),
            },
            pos: Some(ContextPos::new(arg.span, doc)),
        };

        Self::Node {
            entry: entry,
            outer: self,
        }
    }

    pub(crate) fn push_built_in(&'c self, name: &str) -> Self {
        let entry = ContextEntry {
            entry_type: ContextEntryType::BuiltIn { name: name.into() },
            pos: None,
        };

        Self::Node { entry, outer: self }
    }

    pub(crate) fn eval_err<'src, T>(&self, error_type: EvalErrorType<'src>) -> EvalResult<'src, T> {
        let mut context_entries = self.to_vec_rev();
        context_entries.reverse();

        Err(EvalError {
            error_type,
            context_entries,
        })
    }

    fn to_vec_rev(&self) -> Vec<ContextEntry> {
        match self {
            EvalContext::None => Vec::default(),
            EvalContext::Node { entry, outer } => {
                let mut entries = outer.to_vec_rev();
                entries.push(entry.clone());
                entries
            }
        }
    }
}
