use pest::{
    pratt_parser::{Assoc, Op, PrattParser},
    Span,
};

use super::*;

/// An expression, or part of one such as `sin(1.2 * pi) * 0.5`.
#[derive(Clone, Debug)]
pub enum Expr<'src> {
    Number(Number<'src>),
    Unary {
        op: UnaryOp,
        unit: Box<Self>,
    },
    Binary {
        lhs: Box<Self>,
        op: BinaryOp,
        rhs: Box<Self>,
    },
    FuncCall {
        name: Identifier<'src>,
        args: CallArgs<'src>,
    },
}

impl<'src> TryFrom<Pair<'src, Rule>> for Expr<'src> {
    type Error = ParseError<'src>;

    fn try_from(value: Pair<'src, Rule>) -> ParseResult<Self> {
        Self::pratt()
            .map_primary(Self::primary)
            .map_infix(Self::infix)
            .map_prefix(Self::prefix)
            .parse(value.into_inner())
    }
}

impl<'src> Expr<'src> {
    fn pratt() -> PrattParser<Rule> {
        PrattParser::new()
            .op(Op::infix(Rule::add, Assoc::Left) | Op::infix(Rule::sub, Assoc::Left))
            .op(Op::infix(Rule::mul, Assoc::Left) | Op::infix(Rule::div, Assoc::Left))
            .op(Op::prefix(Rule::neg))
    }

    fn primary(primary: Pair<'src, Rule>) -> ParseResult<'src, Self> {
        match primary.as_rule() {
            Rule::number => Ok(Expr::Number(Number::try_from(primary)?)),
            Rule::func_call => {
                let mut inner = primary.into_inner();
                let name = Identifier::try_from(inner.try_next()?)?;
                let args = if let Some(pair) = inner.next() {
                    CallArgs::try_from(pair)?
                } else {
                    CallArgs::default()
                };

                Ok(Expr::FuncCall { name, args })
            }
            Rule::paren_expr => Expr::try_from(primary.into_inner().try_next()?),
            _ => Err(ParseError::UnexpectedFieldType),
        }
    }

    fn infix(
        lhs: ParseResult<'src, Self>,
        op: Pair<'src, Rule>,
        rhs: ParseResult<'src, Self>,
    ) -> ParseResult<'src, Self> {
        let op = match op.as_rule() {
            Rule::add => BinaryOp::Add,
            Rule::sub => BinaryOp::Sub,
            Rule::mul => BinaryOp::Mul,
            Rule::div => BinaryOp::Div,
            _ => return Err(ParseError::UnexpectedFieldType),
        };
        Ok(Expr::Binary {
            lhs: Box::new(lhs?),
            op,
            rhs: Box::new(rhs?),
        })
    }

    fn prefix(op: Pair<'src, Rule>, unit: ParseResult<'src, Self>) -> ParseResult<'src, Self> {
        let op = match op.as_rule() {
            Rule::neg => UnaryOp::Neg,
            _ => return Err(ParseError::UnexpectedFieldType),
        };
        Ok(Expr::Unary {
            op,
            unit: Box::new(unit?),
        })
    }
}

/// A single scalar value literal.
#[derive(Clone, Copy, Debug)]
pub struct Number<'src> {
    pub val: f64,
    pub span: Span<'src>,
}

impl<'src> TryFrom<Pair<'src, Rule>> for Number<'src> {
    type Error = ParseError<'src>;

    fn try_from(value: Pair<'src, Rule>) -> ParseResult<Self> {
        Ok(Self {
            val: value
                .as_str()
                .parse()
                .map_err(|e| ParseError::Float(e, value.as_span()))?,
            span: value.as_span(),
        })
    }
}

/// A unary operator such as `-` (negation).
#[derive(Clone, Copy, Debug)]
pub enum UnaryOp {
    Neg,
}

/// A binary operator such as `+` or `-`.
#[derive(Clone, Copy, Debug)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
}
