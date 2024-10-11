use std::fmt::Display;

use pest::{
    pratt_parser::{Assoc, Op, PrattParser},
    Span,
};

use super::*;

/// An expression, or part of one such as `sin(1.2 * pi) * 0.5`.
#[derive(Clone, Debug)]
pub enum Expr<'src> {
    Number(Number<'src>),
    Unary(UnaryExpr<'src>),
    Binary(BinaryExpr<'src>),
    FuncCall(FuncCallExpr<'src>),
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
            Rule::func_call => Ok(Expr::FuncCall(primary.try_into()?)),
            Rule::paren_expr => Expr::try_from(primary.into_inner().try_next()?),
            _ => Err(ParseError::UnexpectedFieldType),
        }
    }

    fn infix(
        lhs: ParseResult<'src, Self>,
        op: Pair<'src, Rule>,
        rhs: ParseResult<'src, Self>,
    ) -> ParseResult<'src, Self> {
        let (lhs, rhs) = (lhs?, rhs?);

        let (lspan, rspan) = (lhs.span(), rhs.span());
        let span = Span::new(lspan.get_input(), lspan.start(), rspan.end())
            .ok_or(ParseError::ExpectedUnwrap)?;

        let op = match op.as_rule() {
            Rule::add => BinaryOp::Add,
            Rule::sub => BinaryOp::Sub,
            Rule::mul => BinaryOp::Mul,
            Rule::div => BinaryOp::Div,
            _ => return Err(ParseError::UnexpectedFieldType),
        };
        Ok(Expr::Binary(BinaryExpr {
            lhs: Box::new(lhs),
            op,
            rhs: Box::new(rhs),
            span,
        }))
    }

    fn prefix(op: Pair<'src, Rule>, unit: ParseResult<'src, Self>) -> ParseResult<'src, Self> {
        let unit = unit?;

        let (ospan, uspan) = (op.as_span(), unit.span());
        let span = Span::new(ospan.get_input(), ospan.start(), uspan.end())
            .ok_or(ParseError::ExpectedUnwrap)?;

        let op = match op.as_rule() {
            Rule::neg => UnaryOp::Neg,
            _ => return Err(ParseError::UnexpectedFieldType),
        };
        Ok(Expr::Unary(UnaryExpr {
            op,
            unit: Box::new(unit),
            span,
        }))
    }

    fn span(&self) -> Span<'src> {
        match self {
            Expr::Number(number) => number.span,
            Expr::Unary(unary_expr) => unary_expr.span,
            Expr::Binary(binary_expr) => binary_expr.span,
            Expr::FuncCall(func_call_expr) => func_call_expr.span,
        }
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

    fn try_from(value: Pair<'src, Rule>) -> ParseResult<'src, Self> {
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

/// A unary expression like `-a`.
#[derive(Clone, Debug)]
pub struct UnaryExpr<'src> {
    pub op: UnaryOp,
    pub unit: Box<Expr<'src>>,
    pub span: Span<'src>,
}

/// A binary operator such as `+` or `-`.
#[derive(Clone, Copy, Debug)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
}

/// A binary expression like `a + b`.
#[derive(Clone, Debug)]
pub struct BinaryExpr<'src> {
    pub lhs: Box<Expr<'src>>,
    pub op: BinaryOp,
    pub rhs: Box<Expr<'src>>,
    pub span: Span<'src>,
}

/// A function call like `foo` or `bar(1, 2)`.
#[derive(Clone, Debug)]
pub struct FuncCallExpr<'src> {
    pub name: FuncName<'src>,
    pub args: CallArgs<'src>,
    pub span: Span<'src>,
}

impl<'src> Display for FuncCallExpr<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (line, col) = self.span.start_pos().line_col();
        write!(
            f,
            "\"{0}\" on line {1}, col {2}",
            self.span.as_str(),
            line,
            col
        )
    }
}

impl<'src> TryFrom<Pair<'src, Rule>> for FuncCallExpr<'src> {
    type Error = ParseError<'src>;

    fn try_from(value: Pair<'src, Rule>) -> ParseResult<'src, Self> {
        let span = value.as_span();
        let mut inner = value.into_inner();
        let name = FuncName::try_from(inner.try_next()?)?;
        let args = if let Some(pair) = inner.next() {
            CallArgs::try_from(pair)?
        } else {
            CallArgs::default()
        };

        Ok(FuncCallExpr { name, args, span })
    }
}
