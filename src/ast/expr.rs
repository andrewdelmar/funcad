use pest::pratt_parser::{Assoc, Op, PrattParser};

use super::*;

/// An expression, or part of one such as `sin(1.2 * pi) * 0.5`.
#[derive(Clone, Debug)]
pub enum Expr<'src> {
    Number(Number),
    Unary(UnaryExpr<'src>),
    Binary(BinaryExpr<'src>),
    FuncCall(FuncCallExpr<'src>),
}

/// [`Expr`] but [`Spanned`].
pub type SpannedExpr<'src> = Spanned<'src, Expr<'src>>;

impl<'src> TryFrom<Pair<'src, Rule>> for SpannedExpr<'src> {
    type Error = ParseError<'src>;

    fn try_from(value: Pair<'src, Rule>) -> ParseResult<Self> {
        Self::pratt()
            .map_primary(Self::primary)
            .map_infix(Self::infix)
            .map_prefix(Self::prefix)
            .parse(value.into_inner())
    }
}

impl<'src> SpannedExpr<'src> {
    fn pratt() -> PrattParser<Rule> {
        PrattParser::new()
            .op(Op::infix(Rule::add, Assoc::Left) | Op::infix(Rule::sub, Assoc::Left))
            .op(Op::infix(Rule::mul, Assoc::Left) | Op::infix(Rule::div, Assoc::Left))
            .op(Op::prefix(Rule::neg))
    }

    fn primary(primary: Pair<'src, Rule>) -> ParseResult<'src, Self> {
        let span = primary.as_span();

        match primary.as_rule() {
            Rule::number => Ok(Expr::Number(Number::try_from(primary)?).spanned(&span)),
            Rule::func_call => Ok(Expr::FuncCall(primary.try_into()?).spanned(&span)),
            Rule::paren_expr => primary.into_inner().try_next()?.try_into(),
            _ => Err(ParseError::UnexpectedFieldType),
        }
    }

    fn infix(
        lhs: ParseResult<'src, Self>,
        op: Pair<'src, Rule>,
        rhs: ParseResult<'src, Self>,
    ) -> ParseResult<'src, Self> {
        let (lhs, rhs) = (lhs?, rhs?);

        let (lspan, rspan) = (lhs.span, rhs.span);
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
        })
        .spanned(&span))
    }

    fn prefix(op: Pair<'src, Rule>, unit: ParseResult<'src, Self>) -> ParseResult<'src, Self> {
        let unit = unit?;

        let (ospan, uspan) = (op.as_span(), unit.span);
        let span = Span::new(ospan.get_input(), ospan.start(), uspan.end())
            .ok_or(ParseError::ExpectedUnwrap)?;

        let op = match op.as_rule() {
            Rule::neg => UnaryOp::Neg,
            _ => return Err(ParseError::UnexpectedFieldType),
        };
        Ok(Expr::Unary(UnaryExpr {
            op,
            unit: Box::new(unit),
        })
        .spanned(&span))
    }
}

/// A single scalar value literal.
#[derive(Clone, Copy, Debug)]
pub struct Number {
    pub val: f64,
}

impl<'src> TryFrom<Pair<'src, Rule>> for Number {
    type Error = ParseError<'src>;

    fn try_from(value: Pair<'src, Rule>) -> ParseResult<'src, Self> {
        Ok(Self {
            val: value
                .as_str()
                .parse()
                .map_err(|e| ParseError::Float(e, value.as_span()))?,
        })
    }
}

/// [`Number`] but [`Spanned`].
pub type SpannedNumber<'src> = Spanned<'src, Number>;

/// A unary operator such as `-` (negation).
#[derive(Clone, Copy, Debug)]
pub enum UnaryOp {
    Neg,
}

/// A unary expression like `-a`.
#[derive(Clone, Debug)]
pub struct UnaryExpr<'src> {
    pub op: UnaryOp,
    pub unit: Box<SpannedExpr<'src>>,
}

/// [`UnaryExpr`] but [`Spanned`].
pub type SpannedUnaryExpr<'src> = Spanned<'src, UnaryExpr<'src>>;

/// A binary operator such as `+` or `-`.
#[derive(Clone, Copy, Debug)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
}

impl BinaryOp {
    pub(crate) fn op_name(&self) -> &'static str {
        match self {
            BinaryOp::Add => "Addition",
            BinaryOp::Sub => "Subtraction",
            BinaryOp::Mul => "Multiplication",
            BinaryOp::Div => "Division",
        }
    }
}

/// A binary expression like `a + b`.
#[derive(Clone, Debug)]
pub struct BinaryExpr<'src> {
    pub lhs: Box<SpannedExpr<'src>>,
    pub op: BinaryOp,
    pub rhs: Box<SpannedExpr<'src>>,
}

/// [`BinaryExpr`] but [`Spanned`].
pub type SpannedBinaryExpr<'src> = Spanned<'src, BinaryExpr<'src>>;

/// A function call like `foo` or `bar(1, 2)`.
#[derive(Clone, Debug)]
pub struct FuncCallExpr<'src> {
    pub name: FuncName<'src>,
    pub args: CallArgs<'src>,
}

/// [`FuncCallExpr`] but [`Spanned`].
pub type SpannedFuncCallExpr<'src> = Spanned<'src, FuncCallExpr<'src>>;

impl<'src> TryFrom<Pair<'src, Rule>> for FuncCallExpr<'src> {
    type Error = ParseError<'src>;

    fn try_from(value: Pair<'src, Rule>) -> ParseResult<'src, Self> {
        let mut inner = value.into_inner();
        let name = FuncName::try_from(inner.try_next()?)?;
        let args = if let Some(pair) = inner.next() {
            CallArgs::try_from(pair)?
        } else {
            CallArgs::default()
        };

        Ok(FuncCallExpr { name, args })
    }
}
