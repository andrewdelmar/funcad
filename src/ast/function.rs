use std::{collections::HashMap, fmt::Display};

use pest::Span;

use super::*;

/// A single argument in a function definition, and maybe an expression for its 
/// default value.
#[derive(Clone, Debug)]
pub struct ArgDef<'src> {
    pub name: Identifier<'src>,
    pub default: Option<Expr<'src>>,
    pub span: Span<'src>,
}

impl<'src> TryFrom<Pair<'src, Rule>> for ArgDef<'src> {
    type Error = ParseError<'src>;

    fn try_from(value: Pair<'src, Rule>) -> Result<Self, Self::Error> {
        let span = value.as_span();
        let mut inner = value.into_inner();

        let name = Identifier::try_from(inner.try_next()?)?;

        let default = if let Some(pair) = inner.next() {
            Some(Expr::try_from(pair)?)
        } else {
            None
        };

        Ok(Self {
            name,
            default,
            span,
        })
    }
}

/// A collection of all the arguments in a function definition.
#[derive(Clone, Debug)]
pub struct ArgDefs<'src> {
    pub args: Vec<ArgDef<'src>>,
    pub span: Span<'src>,
}

impl<'src> TryFrom<Pair<'src, Rule>> for ArgDefs<'src> {
    type Error = ParseError<'src>;

    fn try_from(value: Pair<'src, Rule>) -> Result<Self, Self::Error> {
        let span = value.as_span();
        let args: Result<Vec<_>, _> = value.into_inner().map(ArgDef::try_from).collect();
        Ok(Self { args: args?, span })
    }
}

/// A collection of all of the expressions passed as arguments in a single
/// function call.
#[derive(Clone, Default, Debug)]
pub enum CallArgs<'src> {
    #[default]
    None,
    Positional(Vec<Box<Expr<'src>>>),
    Named(HashMap<&'src str, NamedCallArg<'src>>),
}

impl<'src> TryFrom<Pair<'src, Rule>> for CallArgs<'src> {
    type Error = ParseError<'src>;

    fn try_from(value: Pair<'src, Rule>) -> Result<Self, Self::Error> {
        match value.as_rule() {
            Rule::empty_call_args => Ok(CallArgs::None),
            Rule::pos_call_args => {
                let args: Result<Vec<_>, ParseError> = value
                    .into_inner()
                    .map(|pair| Expr::try_from(pair).map(Box::new))
                    .collect();
                Ok(CallArgs::Positional(args?))
            }
            Rule::named_call_args => {
                let named_args: Result<Vec<_>, ParseError> =
                    value.into_inner().map(NamedCallArg::try_from).collect();

                let mut arg_map = HashMap::new();

                for new in named_args? {
                    if let Some(old) = arg_map.insert(new.name.text, new.clone()) {
                        return Err(ParseError::DuplicateNamedArgument(old, new));
                    }
                }

                Ok(CallArgs::Named(arg_map))
            }
            _ => return Err(ParseError::UnexpectedFieldType),
        }
    }
}

/// A single named argument in a function call. Like `foo = 1`.
#[derive(Clone, Debug)]
pub struct NamedCallArg<'src> {
    pub name: Identifier<'src>,
    pub expr: Box<Expr<'src>>,
    pub span: Span<'src>,
}

impl<'src> Display for NamedCallArg<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (line, col) = self.span.start_pos().line_col();
        write!(f, "\"{0}\" on line {1}, col {2}", self.name.text, line, col)
    }
}

impl<'src> TryFrom<Pair<'src, Rule>> for NamedCallArg<'src> {
    type Error = ParseError<'src>;

    fn try_from(value: Pair<'src, Rule>) -> Result<Self, Self::Error> {
        let span = value.as_span();
        // named_call_arg  = { identifier ~ "=" ~ expr }
        let mut inner = value.into_inner();
        let name = Identifier::try_from(inner.try_next()?)?;
        let expr = Box::new(Expr::try_from(inner.try_next()?)?);
        Ok(Self { name, expr, span })
    }
}

/// A complete function definition, including its arguments and body.
#[derive(Clone, Debug)]
pub struct FuncDef<'src> {
    pub name: Identifier<'src>,
    pub args: Option<ArgDefs<'src>>,
    pub body: Expr<'src>,
    pub span: Span<'src>,
}

impl<'src> Display for FuncDef<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let line = self.span.start_pos().line_col().0;
        write!(f, "\"{0}\" on line {1}", self.name.text, line)
    }
}

impl<'src> TryFrom<Pair<'src, Rule>> for FuncDef<'src> {
    type Error = ParseError<'src>;

    fn try_from(value: Pair<'src, Rule>) -> Result<Self, Self::Error> {
        let span = value.as_span();
        let mut inner = value.into_inner();

        // func_def = { identifier ~ arg_defs? ~ "=" ~ expr }
        let name = Identifier::try_from(inner.try_next()?)?;

        let args_or_body = inner.try_next()?;
        let (args, body) = match args_or_body.as_rule() {
            Rule::arg_defs => (
                Some(ArgDefs::try_from(args_or_body)?),
                Expr::try_from(inner.try_next()?)?,
            ),
            Rule::expr => (None, Expr::try_from(args_or_body)?),
            _ => return Err(ParseError::UnexpectedFieldType),
        };

        Ok(FuncDef {
            name,
            args,
            body,
            span,
        })
    }
}
