use std::collections::HashMap;

use super::*;

/// A single argument in a function definition, and maybe an expression for its
/// default value.
#[derive(Clone, Debug)]
pub struct ArgDef<'src> {
    pub name: SpannedIdentifier<'src>,
    pub default: Option<SpannedExpr<'src>>,
}

/// [`ArgDef`] but [`Spanned`].
pub type SpannedArgDef<'src> = Spanned<'src, ArgDef<'src>>;

impl<'src> TryFrom<Pair<'src, Rule>> for ArgDef<'src> {
    type Error = ParseError<'src>;

    fn try_from(value: Pair<'src, Rule>) -> Result<Self, Self::Error> {
        let mut inner = value.into_inner();

        let name = SpannedIdentifier::try_from(inner.try_next()?)?;

        let default = if let Some(pair) = inner.next() {
            Some(SpannedExpr::try_from(pair)?)
        } else {
            None
        };

        Ok(Self { name, default })
    }
}

/// A collection of all the arguments in a function definition.
#[derive(Clone, Debug)]
pub struct ArgDefs<'src> {
    pub args: Vec<SpannedArgDef<'src>>,
}

/// [`ArgDefs`] but [`Spanned`].
pub type SpannedArgDefs<'src> = Spanned<'src, ArgDefs<'src>>;

impl<'src> TryFrom<Pair<'src, Rule>> for ArgDefs<'src> {
    type Error = ParseError<'src>;

    fn try_from(value: Pair<'src, Rule>) -> Result<Self, Self::Error> {
        let args: Result<Vec<_>, _> = value.into_inner().map(SpannedArgDef::try_from).collect();
        Ok(Self { args: args? })
    }
}

/// A collection of all of the expressions passed as arguments in a single
/// function call.
#[derive(Clone, Default, Debug)]
pub enum CallArgs<'src> {
    #[default]
    None,
    Positional(Vec<Box<SpannedExpr<'src>>>),
    Named(HashMap<&'src str, SpannedNamedCallArg<'src>>),
}

/// [`CallArgs`] but [`Spanned`].
pub type SpannedCallArgs<'src> = Spanned<'src, CallArgs<'src>>;

impl<'src> TryFrom<Pair<'src, Rule>> for CallArgs<'src> {
    type Error = ParseError<'src>;

    fn try_from(value: Pair<'src, Rule>) -> Result<Self, Self::Error> {
        match value.as_rule() {
            Rule::empty_call_args => Ok(CallArgs::None),
            Rule::pos_call_args => {
                let args: Result<Vec<_>, ParseError> = value
                    .into_inner()
                    .map(|pair| SpannedExpr::try_from(pair).map(Box::new))
                    .collect();
                Ok(CallArgs::Positional(args?))
            }
            Rule::named_call_args => {
                let named_args: Result<Vec<_>, ParseError> = value
                    .into_inner()
                    .map(SpannedNamedCallArg::try_from)
                    .collect();

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
    pub name: SpannedIdentifier<'src>,
    pub expr: Box<SpannedExpr<'src>>,
}

/// [`NamedCallArg`] but [`Spanned`].
pub type SpannedNamedCallArg<'src> = Spanned<'src, NamedCallArg<'src>>;

impl<'src> TryFrom<Pair<'src, Rule>> for NamedCallArg<'src> {
    type Error = ParseError<'src>;

    fn try_from(value: Pair<'src, Rule>) -> Result<Self, Self::Error> {
        // named_call_arg  = { identifier ~ "=" ~ expr }
        let mut inner = value.into_inner();
        let name = SpannedIdentifier::try_from(inner.try_next()?)?;
        let expr = Box::new(SpannedExpr::try_from(inner.try_next()?)?);
        Ok(Self { name, expr })
    }
}

/// A complete function definition, including its arguments and body.
#[derive(Clone, Debug)]
pub struct FuncDef<'src> {
    pub name: SpannedIdentifier<'src>,
    pub args: Option<SpannedArgDefs<'src>>,
    pub body: SpannedExpr<'src>,
}

/// [`FuncDef`] but [`Spanned`].
pub type SpannedFuncDef<'src> = Spanned<'src, FuncDef<'src>>;

impl<'src> TryFrom<Pair<'src, Rule>> for FuncDef<'src> {
    type Error = ParseError<'src>;

    fn try_from(value: Pair<'src, Rule>) -> Result<Self, Self::Error> {
        let mut inner = value.into_inner();

        // func_def = { identifier ~ arg_defs? ~ "=" ~ expr }
        let name = SpannedIdentifier::try_from(inner.try_next()?)?;

        let args_or_body = inner.try_next()?;
        let (args, body) = match args_or_body.as_rule() {
            Rule::arg_defs => (
                Some(SpannedArgDefs::try_from(args_or_body)?),
                SpannedExpr::try_from(inner.try_next()?)?,
            ),
            Rule::expr => (None, SpannedExpr::try_from(args_or_body)?),
            _ => return Err(ParseError::UnexpectedFieldType),
        };

        Ok(FuncDef { name, args, body })
    }
}
