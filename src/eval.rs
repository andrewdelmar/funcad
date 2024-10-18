use std::{
    collections::{BTreeMap, HashMap, HashSet},
    hash::Hash,
};

use pest::Span;

use crate::{
    ast::*,
    error::{EvalError, EvalResult},
    DocSet, FQPath, SolidId, SolidSet,
};

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Number(f64),
    Solid(SolidId),
}

// This is dangerous since float NaNs are never equal.
// We throw errors the moment NaNs are detected so this *shouldn't* be an issue.
impl Eq for Value {}

impl Hash for Value {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            // Just comparing the bits of floats ignores the fact that NaNs
            // should never be equal, but we should error on NaN anyway.
            Value::Number(val) => val.to_bits().hash(state),
            Value::Solid(id) => id.hash(state),
        }
    }
}

#[derive(Clone, Hash, PartialEq, Eq)]
struct Scope {
    func_name: String,
    args: BTreeMap<String, Value>,
    doc: FQPath,
}

pub(crate) struct EvalCache<'set, 'src> {
    docs: &'set DocSet<'src>,
    evaluating: HashSet<Scope>,

    cache: HashMap<Scope, Value>,
    solids: SolidSet,
}

impl<'set, 'src> EvalCache<'set, 'src> {
    const GLOBAL_FUNCNAME: &'static str = "GLOBAL";

    pub(crate) fn new(docs: &'set DocSet<'src>) -> Self {
        Self {
            docs,
            evaluating: HashSet::new(),
            cache: HashMap::new(),
            solids: SolidSet::default(),
        }
    }

    pub(crate) fn eval_func_by_name(
        &mut self,
        doc_path: &FQPath,
        func_name: &str,
    ) -> EvalResult<'src, Value> {
        let Some(doc) = self.docs.get(doc_path) else {
            return Err(EvalError::EvalFuncDocNotFound(doc_path.clone()));
        };

        let Some(func) = doc.funcs.get(func_name) else {
            return Err(EvalError::EvalFuncFuncNotFound(func_name.to_string()));
        };

        let mut scope = Scope {
            func_name: func_name.to_string(),
            args: BTreeMap::new(),
            doc: doc_path.clone(),
        };

        if let Some(arg_defs) = &func.args {
            for arg in &arg_defs.args {
                if let Some(default) = &arg.default {
                    let val = self.eval_default_value(default, doc_path)?;
                    scope.args.insert(arg.name.to_string(), val);
                } else {
                    return Err(EvalError::EvalFuncHasArgs(func.clone()));
                }
            }
        }

        self.eval_expr(&func.body, &scope)
    }

    fn eval_expr(&mut self, expr: &SpannedExpr<'src>, scope: &Scope) -> EvalResult<'src, Value> {
        match &expr.inner {
            Expr::Number(Number { val }) => Ok(Value::Number(*val)),
            Expr::Unary(unary) => self.eval_unary_expr(unary, &expr.span, scope),
            Expr::Binary(binary) => self.eval_binary_expr(binary, &expr.span, scope),
            Expr::FuncCall(call) => self.eval_func_call_expr(call, &expr.span, scope),
        }
    }

    fn eval_unary_expr(
        &mut self,
        expr: &UnaryExpr<'src>,
        _span: &Span<'src>,
        scope: &Scope,
    ) -> EvalResult<'src, Value> {
        match expr.op {
            UnaryOp::Neg => match self.eval_expr(&expr.unit, scope)? {
                Value::Number(number) => Ok(Value::Number(-number)),
                Value::Solid(ref solid) => Ok(Value::Solid(self.solids.negate(solid)?)),
            },
        }
    }

    fn eval_binary_expr(
        &mut self,
        expr: &BinaryExpr<'src>,
        span: &Span<'src>,
        scope: &Scope,
    ) -> EvalResult<'src, Value> {
        let lhsv = self.eval_expr(&expr.lhs, scope)?;
        let rhsv = self.eval_expr(&expr.rhs, scope)?;

        use {BinaryOp::*, Value::*};
        let val = match (lhsv, expr.op, rhsv) {
            (Number(lhs), Add, Number(rhs)) => Number(lhs + rhs),
            (Number(lhs), Sub, Number(rhs)) => Number(lhs - rhs),
            (Number(lhs), Mul, Number(rhs)) => Number(lhs * rhs),
            (Number(lhs), Div, Number(rhs)) => Number(lhs / rhs),
            (Solid(ref lhs), Add, Solid(ref rhs)) => Solid(self.solids.union(lhs, rhs)?),
            (Solid(ref lhs), Sub, Solid(ref rhs)) => Solid(self.solids.difference(lhs, rhs)?),
            (Solid(ref lhs), Mul, Solid(ref rhs)) => Solid(self.solids.intersection(lhs, rhs)?),
            _ => todo!(),
        };

        if let Number(fval) = val {
            if !fval.is_finite() {
                return Err(EvalError::BinaryExprNotFinite(expr.spanned(span)));
            }
        }

        Ok(val)
    }

    fn eval_func_call_expr(
        &mut self,
        expr: &FuncCallExpr<'src>,
        span: &Span<'src>,
        scope: &Scope,
    ) -> EvalResult<'src, Value> {
        let this_doc = &self.docs[&scope.doc];

        if let Some(import_part) = expr.name.import_part {
            let Some(import) = this_doc.imports.get(import_part.text) else {
                return Err(EvalError::FuncCallImportNotInDoc(
                    expr.spanned(span),
                    import_part,
                ));
            };

            let import_path = scope.doc.import_path(import)?;

            let Some(import_doc) = self.docs.get(&import_path) else {
                return Err(EvalError::FuncCallImportDocNotFound(expr.spanned(span)));
            };
            let Some(func) = import_doc.funcs.get(expr.name.name_part.text) else {
                return Err(EvalError::FuncCallFuncNotFound(expr.spanned(span)));
            };

            let call_scope = self.build_call_scope(expr, span, scope, func, &import_path)?;
            self.eval_func(func, &call_scope)
        } else {
            if let Some(arg) = scope.args.get(expr.name.name_part.text) {
                Ok(arg.clone())
            } else {
                let Some(func) = this_doc.funcs.get(expr.name.name_part.text) else {
                    return Err(EvalError::FuncCallFuncNotFound(expr.spanned(span)));
                };

                let call_scope = self.build_call_scope(expr, span, scope, func, &scope.doc)?;
                self.eval_func(func, &call_scope)
            }
        }
    }

    fn eval_func(&mut self, func: &SpannedFuncDef<'src>, scope: &Scope) -> EvalResult<'src, Value> {
        if self.evaluating.contains(scope) {
            return Err(EvalError::FuncCallInfiniteRecursion(func.clone()));
        }
        self.evaluating.insert(scope.clone());

        let res = if let Some(cached) = self.cache.get(scope) {
            Ok(cached.clone())
        } else {
            let res = self.eval_expr(&func.body, scope);
            if let Ok(val) = &res {
                self.cache.insert(scope.clone(), val.clone());
            }
            res
        };

        self.evaluating.remove(&scope);

        res
    }

    fn build_call_scope(
        &mut self,
        expr: &FuncCallExpr<'src>,
        span: &Span<'src>,
        scope: &Scope,
        func_def: &SpannedFuncDef<'src>,
        def_doc_path: &FQPath,
    ) -> EvalResult<'src, Scope> {
        let mut new = Scope {
            func_name: func_def.name.text.into(),
            args: BTreeMap::default(),
            doc: def_doc_path.clone(),
        };

        match &expr.args {
            CallArgs::None => {}
            CallArgs::Positional(args) => {
                for (i, call_arg) in args.iter().enumerate() {
                    let Some(def_args) = &func_def.args else {
                        return Err(EvalError::FuncCallTooManyArgs(
                            expr.spanned(span),
                            func_def.clone(),
                        ));
                    };

                    if let Some(SpannedArgDef {
                        inner: ArgDef { name, .. },
                        ..
                    }) = def_args.args.get(i)
                    {
                        let val = self.eval_expr(&call_arg, scope)?;
                        new.args.insert(name.text.into(), val);
                    } else {
                        return Err(EvalError::FuncCallTooManyArgs(
                            expr.spanned(span),
                            func_def.clone(),
                        ));
                    }
                }
            }
            CallArgs::Named(args) => {
                let def_names: HashSet<_> = if let Some(def_args) = &func_def.args {
                    def_args.args.iter().map(|a| a.name.text).collect()
                } else {
                    HashSet::new()
                };

                let mut extra_args = Vec::new();
                for (name, arg) in args {
                    if !def_names.contains(name) {
                        extra_args.push(arg.clone());
                    }
                }

                if !extra_args.is_empty() {
                    return Err(EvalError::FuncCallExtraNamedArgs(
                        expr.spanned(span),
                        extra_args,
                        func_def.clone(),
                    ));
                }
            }
        }

        if let Some(def_args) = &func_def.args {
            let mut missing_args = Vec::new();

            for def_arg in &def_args.args {
                if !new.args.contains_key(&def_arg.name.text.to_string()) {
                    if let Some(default) = &def_arg.default {
                        let val = self.eval_default_value(default, def_doc_path)?;
                        new.args.insert(def_arg.name.text.into(), val);
                    } else {
                        missing_args.push(def_arg.clone());
                    }
                }
            }

            if !missing_args.is_empty() {
                return Err(EvalError::FuncCallMissingArguments(
                    missing_args,
                    expr.spanned(span),
                ));
            }
        }

        Ok(new)
    }

    fn eval_default_value(
        &mut self,
        expr: &SpannedExpr<'src>,
        doc: &FQPath,
    ) -> EvalResult<'src, Value> {
        let default_scope = Scope {
            func_name: Self::GLOBAL_FUNCNAME.into(),
            args: BTreeMap::default(),
            doc: doc.clone(),
        };

        self.eval_expr(expr, &default_scope)
    }
}
