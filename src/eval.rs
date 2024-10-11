use std::collections::{HashMap, HashSet};

use crate::{
    error::{EvalError, EvalResult},
    ArgDef, BinaryExpr, BinaryOp, CallArgs, DocSet, Expr, FQPath, FuncCallExpr, FuncDef, Number,
    UnaryExpr, UnaryOp,
};

#[derive(Clone, Debug)]
pub enum Value {
    Number(f64),
}

//TODO this should be used as a key for EvalCache.
struct Scope {
    _func_name: String,
    //TODO Eventually args should be lazily evaluated.
    args: HashMap<String, Value>,
    doc: FQPath,
}

pub(crate) struct EvalCache<'set, 'src> {
    pub(crate) docs: &'set DocSet<'src>,
    //TODO actual cache
    //TODO evaluating set for loop detection.
}

impl<'set, 'src> EvalCache<'set, 'src> {
    const GLOBAL_FUNCNAME: &'static str = "GLOBAL";

    pub(crate) fn eval_func_by_name(
        &self,
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
            _func_name: func_name.to_string(),
            args: HashMap::new(),
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

    fn eval_expr(&self, expr: &Expr<'src>, scope: &Scope) -> EvalResult<'src, Value> {
        match expr {
            Expr::Number(Number { val, .. }) => Ok(Value::Number(*val)),
            Expr::Unary(unary) => self.eval_unary(unary, scope),
            Expr::Binary(binary) => self.eval_binary(binary, scope),
            Expr::FuncCall(call) => self.eval_func_call(call, scope),
        }
    }

    fn eval_unary(&self, expr: &UnaryExpr<'src>, scope: &Scope) -> EvalResult<'src, Value> {
        match expr.op {
            UnaryOp::Neg => match self.eval_expr(&expr.unit, scope)? {
                Value::Number(number) => Ok(Value::Number(-number)),
            },
        }
    }

    fn eval_binary(&self, expr: &BinaryExpr<'src>, scope: &Scope) -> EvalResult<'src, Value> {
        let lhsv = self.eval_expr(&expr.lhs, scope)?;
        let rhsv = self.eval_expr(&expr.rhs, scope)?;

        use {BinaryOp::*, Value::*};
        match (lhsv, expr.op, rhsv) {
            (Number(lhs), Add, Number(rhs)) => Ok(Number(lhs + rhs)),
            (Number(lhs), Sub, Number(rhs)) => Ok(Number(lhs - rhs)),
            (Number(lhs), Mul, Number(rhs)) => Ok(Number(lhs * rhs)),
            (Number(lhs), Div, Number(rhs)) => Ok(Number(lhs / rhs)),
        }
    }

    fn eval_func_call(&self, expr: &FuncCallExpr<'src>, scope: &Scope) -> EvalResult<'src, Value> {
        let this_doc = &self.docs[&scope.doc];

        if let Some(import_part) = expr.name.import_part {
            let Some(import) = this_doc.imports.get(import_part.text) else {
                return Err(EvalError::FuncCallImportNotInDoc(expr.clone(), import_part));
            };

            let import_path = scope.doc.import_path(import)?;

            let Some(import_doc) = self.docs.get(&import_path) else {
                return Err(EvalError::FuncCallImportDocNotFound(expr.clone()));
            };
            let Some(func) = import_doc.funcs.get(expr.name.name_part.text) else {
                return Err(EvalError::FuncCallFuncNotFound(expr.clone()));
            };

            let call_scope = self.build_call_scope(scope, func, &import_path, expr)?;
            self.eval_expr(&func.body, &call_scope)
        } else {
            if let Some(arg) = scope.args.get(expr.name.name_part.text) {
                Ok(arg.clone())
            } else {
                let Some(func) = this_doc.funcs.get(expr.name.name_part.text) else {
                    return Err(EvalError::FuncCallFuncNotFound(expr.clone()));
                };

                let call_scope = self.build_call_scope(scope, func, &scope.doc, expr)?;
                self.eval_expr(&func.body, &call_scope)
            }
        }
    }

    fn build_call_scope(
        &self,
        scope: &Scope,
        func_def: &FuncDef<'src>,
        def_doc_path: &FQPath,
        expr: &FuncCallExpr<'src>,
    ) -> EvalResult<'src, Scope> {
        let mut new = Scope {
            _func_name: func_def.name.text.into(),
            args: HashMap::default(),
            doc: def_doc_path.clone(),
        };

        match &expr.args {
            CallArgs::None => {}
            CallArgs::Positional(args) => {
                for (i, call_arg) in args.iter().enumerate() {
                    let Some(def_args) = &func_def.args else {
                        return Err(EvalError::FuncCallTooManyArgs(
                            expr.clone(),
                            func_def.clone(),
                        ));
                    };

                    if let Some(ArgDef { name, .. }) = def_args.args.get(i) {
                        let val = self.eval_expr(&call_arg, scope)?;
                        new.args.insert(name.text.into(), val);
                    } else {
                        return Err(EvalError::FuncCallTooManyArgs(
                            expr.clone(),
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
                        expr.clone(),
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
                    expr.clone(),
                ));
            }
        }

        Ok(new)
    }

    fn eval_default_value(&self, expr: &Expr<'src>, doc: &FQPath) -> EvalResult<'src, Value> {
        let default_scope = Scope {
            _func_name: Self::GLOBAL_FUNCNAME.into(),
            args: HashMap::default(),
            doc: doc.clone(),
        };

        self.eval_expr(expr, &default_scope)
    }
}
