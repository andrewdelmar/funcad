mod builtins;

mod context;
pub(crate) use context::{ContextEntry, EvalContext};

mod value;
pub use value::Value;

mod scope;
pub(crate) use scope::Scope;

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::{
    ast::*,
    error::{EvalErrorType, EvalResult},
    DocSet, FQPath, SolidSet,
};

pub(crate) struct EvalCache<'set, 'src> {
    docs: &'set DocSet<'src>,
    evaluating: HashSet<Scope>,

    cache: HashMap<Scope, Value>,
    solids: SolidSet,
}

impl<'set, 'src> EvalCache<'set, 'src> {
    pub(crate) fn new(docs: &'set DocSet<'src>) -> Self {
        Self {
            docs,
            evaluating: HashSet::new(),
            cache: HashMap::new(),
            solids: SolidSet::default(),
        }
    }

    fn eval_expr(
        &mut self,
        expr: &SpannedExpr<'src>,
        scope: &Scope,
        context: &EvalContext,
    ) -> EvalResult<'src, Value> {
        match &expr.inner {
            Expr::Number(Number { val }) => Ok(Value::Number(*val)),
            Expr::Unary(unary) => self.eval_unary_expr(&unary.spanned(&expr.span), scope, context),
            Expr::Binary(binary) => {
                self.eval_binary_expr(&binary.spanned(&expr.span), scope, context)
            }
            Expr::FuncCall(call) => {
                self.eval_func_call_expr(&call.spanned(&expr.span), scope, context)
            }
        }
    }

    fn eval_unary_expr(
        &mut self,
        expr: &SpannedUnaryExpr<'src>,
        scope: &Scope,
        context: &EvalContext,
    ) -> EvalResult<'src, Value> {
        match expr.op {
            UnaryOp::Neg => match self.eval_expr(&expr.unit, scope, context)? {
                Value::Number(number) => Ok(Value::Number(-number)),
                Value::Solid(ref solid) => Ok(Value::Solid(self.solids.negate(solid)?)),
            },
        }
    }

    fn eval_binary_expr(
        &mut self,
        expr: &SpannedBinaryExpr<'src>,
        scope: &Scope,
        context: &EvalContext,
    ) -> EvalResult<'src, Value> {
        let lhsv = self.eval_expr(&expr.lhs, scope, context)?;
        let rhsv = self.eval_expr(&expr.rhs, scope, context)?;

        use {BinaryOp::*, Value::*};
        let val = match (lhsv, expr.op, rhsv) {
            (Number(lhs), Add, Number(rhs)) => Number(lhs + rhs),
            (Number(lhs), Sub, Number(rhs)) => Number(lhs - rhs),
            (Number(lhs), Mul, Number(rhs)) => Number(lhs * rhs),
            (Number(lhs), Div, Number(rhs)) => Number(lhs / rhs),

            (Solid(ref lhs), Add, Solid(ref rhs)) => Solid(self.solids.union(lhs, rhs)?),
            (Solid(ref lhs), Sub, Solid(ref rhs)) => Solid(self.solids.difference(lhs, rhs)?),
            (Solid(ref lhs), Mul, Solid(ref rhs)) => Solid(self.solids.intersection(lhs, rhs)?),

            (lhs, op, rhs) => {
                return context.eval_err(EvalErrorType::BinaryOpWrongTypes {
                    op: op.op_name(),
                    lhs_type: lhs.type_name(),
                    rhs_type: rhs.type_name(),
                })
            }
        };

        if let Number(fval) = val {
            if !fval.is_finite() {
                return context.eval_err(EvalErrorType::NumExprNotFinite);
            }
        }

        Ok(val)
    }

    fn eval_func_call_expr(
        &mut self,
        expr: &SpannedFuncCallExpr<'src>,
        scope: &Scope,
        context: &EvalContext,
    ) -> EvalResult<'src, Value> {
        let doc_path = scope
            .doc()
            .expect("Wrong kind of scope for func call evaluation");
        let this_doc = &self.docs[doc_path];
        let context = context.push_func_call(expr, doc_path);

        if let Some(import_part) = expr.name.import_part {
            // Function call with import.
            let Some(import) = this_doc.imports.get(import_part.text) else {
                return context.eval_err(EvalErrorType::ImportNotFound {
                    name: import_part.text.into(),
                });
            };

            let import_path = doc_path.import_path(import)?;
            let Some(import_doc) = self.docs.get(&import_path) else {
                return context.eval_err(EvalErrorType::DocNotFound {
                    path: import_path.clone(),
                });
            };

            let Some(func_def) = import_doc.funcs.get(expr.name.name_part.text) else {
                return context.eval_err(EvalErrorType::FuncNotFound {
                    name: expr.name.name_part.text.into(),
                });
            };

            let args = self.eval_func_call_args(expr, func_def, &import_path, scope, &context)?;
            let scope = Scope::FuncCall {
                name: expr.name.name_part.text.into(),
                args,
                doc_path: import_path.clone(),
            };
            self.eval_scope(&scope, &context)
        } else if let Some(arg) = scope.args().get(expr.name.name_part.text) {
            // Argument.
            Ok(arg.clone())
        } else if let Some(built_in) = Self::get_built_in_func(&expr.name.name_part.text) {
            // Built-in function.
            let args = self.eval_built_in_call_args(expr, built_in, scope, &context)?;
            let scope = Scope::BuiltIn {
                name: expr.name.name_part.text.into(),
                args,
            };
            self.eval_scope(&scope, &context)
        } else if let Some(func) = this_doc.funcs.get(expr.name.name_part.text) {
            // Regular function call.
            let args = self.eval_func_call_args(expr, func, &doc_path, scope, &context)?;
            let scope = Scope::FuncCall {
                name: expr.name.name_part.text.into(),
                args,
                doc_path: doc_path.clone(),
            };
            self.eval_scope(&scope, &context)
        } else {
            // No match.
            return context.eval_err(EvalErrorType::FuncNotFound {
                name: expr.name.name_part.text.into(),
            });
        }
    }

    fn eval_func_call_args(
        &mut self,
        call_expr: &SpannedFuncCallExpr<'src>,
        func_def: &SpannedFuncDef<'src>,
        def_doc_path: &FQPath,
        scope: &Scope,
        context: &EvalContext,
    ) -> EvalResult<'src, BTreeMap<String, Value>> {
        let mut args = self.eval_supplied_func_call_args(call_expr, func_def, scope, context)?;
        self.add_default_func_def_args(&mut args, func_def, def_doc_path, context)?;
        Ok(args)
    }

    /// Evaluates supplied arguments and returns an error if there are too many.
    fn eval_supplied_func_call_args(
        &mut self,
        func_call: &SpannedFuncCallExpr<'src>,
        func_def: &SpannedFuncDef<'src>,
        scope: &Scope,
        context: &EvalContext,
    ) -> EvalResult<'src, BTreeMap<String, Value>> {
        match (&func_call.args, &func_def.args) {
            (CallArgs::None, _) => Ok(BTreeMap::default()),
            (CallArgs::Positional(args), Some(arg_defs)) => {
                let mut arg_vals = BTreeMap::new();

                for (arg_index, arg_expr) in args.iter().enumerate() {
                    let Some(arg_def) = arg_defs.args.get(arg_index) else {
                        return context.eval_err(EvalErrorType::TooManyArgs);
                    };

                    let val = self.eval_expr(&arg_expr, scope, context)?;
                    arg_vals.insert(arg_def.name.text.into(), val);
                }

                Ok(arg_vals)
            }
            (CallArgs::Named(args), Some(arg_defs)) => {
                let mut arg_vals = BTreeMap::new();

                for (name, arg) in args {
                    if arg_defs.args.iter().all(|f| &f.name.text != name) {
                        return context.eval_err(EvalErrorType::InvalidNamedArg {
                            name: (*name).into(),
                        });
                    }

                    let val = self.eval_expr(&arg.expr, scope, context)?;
                    arg_vals.insert((*name).into(), val);
                }

                Ok(arg_vals)
            }
            (_, None) => context.eval_err(EvalErrorType::TooManyArgs),
        }
    }

    /// Evaluates and adds argument defaults not already in args and throws an
    /// error if required arguments are missing from a function call.
    fn add_default_func_def_args(
        &mut self,
        args: &mut BTreeMap<String, Value>,
        func_def: &SpannedFuncDef<'src>,
        def_doc_path: &FQPath,
        context: &EvalContext,
    ) -> EvalResult<'src, ()> {
        if let Some(def_args) = &func_def.args {
            for def_arg in &def_args.args {
                if !args.contains_key(&def_arg.name.text.to_string()) {
                    let scope = Scope::ArgDefault {
                        doc_path: def_doc_path.clone(),
                        func: func_def.name.text.into(),
                        arg: def_arg.name.text.into(),
                    };
                    let val = self.eval_scope(&scope, context)?;
                    args.insert(def_arg.name.text.into(), val);
                }
            }
        }

        Ok(())
    }
}
