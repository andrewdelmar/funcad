use std::collections::BTreeMap;

use crate::{
    error::{EvalErrorType, EvalResult},
    FQPath,
};

use super::{EvalCache, EvalContext, Value};

/// A Scope is an identifier of a single cacheable unit of evaluation.
/// i.e. A call to a specific function call with a specific set of arguments
/// or, an expression defining a default value.
/// If Scopes are equal they should always evaluate to the same value.
#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub(crate) enum Scope {
    FuncCall {
        name: String,
        args: BTreeMap<String, Value>,
        doc_path: FQPath,
    },
    ArgDefault {
        doc_path: FQPath,
        func: String,
        arg: String,
    },
    BuiltIn {
        name: String,
        args: BTreeMap<String, Value>,
    },
}

impl Scope {
    pub(super) fn doc(&self) -> Option<&FQPath> {
        match self {
            Scope::FuncCall { doc_path, .. } | Scope::ArgDefault { doc_path, .. } => Some(doc_path),
            Scope::BuiltIn { .. } => None,
        }
    }

    const EMPTY_ARGS: &'static BTreeMap<String, Value> = &BTreeMap::new();
    pub(super) fn args(&self) -> &BTreeMap<String, Value> {
        match self {
            Scope::FuncCall { args, .. } | Scope::BuiltIn { args, .. } => args,
            Scope::ArgDefault { .. } => Self::EMPTY_ARGS,
        }
    }
}

impl<'set, 'src> EvalCache<'set, 'src> {
    pub(crate) fn eval_scope(
        &mut self,
        scope: &Scope,
        context: &EvalContext,
    ) -> EvalResult<'src, Value> {
        if self.evaluating.contains(scope) {
            return context.eval_err(EvalErrorType::InfiniteRecursion);
        }
        self.evaluating.insert(scope.clone());

        let res = if let Some(cached) = self.cache.get(scope) {
            Ok(cached.clone())
        } else {
            self.eval_scope_unchecked(scope, context)
        };

        self.evaluating.remove(scope);

        if let Ok(val) = &res {
            self.cache.insert(scope.clone(), val.clone());
        }

        res
    }

    fn eval_scope_unchecked(
        &mut self,
        scope: &Scope,
        context: &EvalContext,
    ) -> EvalResult<'src, Value> {
        match scope {
            Scope::FuncCall { name, doc_path, .. } => {
                let Some(doc) = self.docs.get(doc_path) else {
                    return context.eval_err(EvalErrorType::DocNotFound {
                        path: doc_path.clone(),
                    });
                };
                let Some(func) = doc.funcs.get(name.as_str()) else {
                    return context.eval_err(EvalErrorType::FuncNotFound { name: name.clone() });
                };

                let context = context.push_func_def(func, doc_path);

                self.eval_expr(&func.body, scope, &context)
            }
            Scope::ArgDefault {
                doc_path,
                func,
                arg,
            } => {
                let Some(doc) = self.docs.get(doc_path) else {
                    return context.eval_err(EvalErrorType::DocNotFound {
                        path: doc_path.clone(),
                    });
                };
                let Some(func) = doc.funcs.get(func.as_str()) else {
                    return context.eval_err(EvalErrorType::FuncNotFound { name: func.clone() });
                };
                let Some(ref args) = func.args else {
                    return context.eval_err(EvalErrorType::ArgNotFound { name: arg.clone() });
                };
                let Some(def) = args.with_name(arg.as_str()) else {
                    return context.eval_err(EvalErrorType::ArgNotFound { name: arg.clone() });
                };
                let Some(ref expr) = def.default else {
                    return context
                        .eval_err(EvalErrorType::NoSuppliedOrDefaultArg { name: arg.clone() });
                };

                let context = context.push_arg_default(def, func, doc_path);

                self.eval_expr(expr, scope, &context)
            }
            Scope::BuiltIn { name, .. } => {
                let Some(built_in) = Self::get_built_in_func(name) else {
                    return context.eval_err(EvalErrorType::BuiltInNotFound { name: name.clone() });
                };

                let context = context.push_built_in(name);

                built_in.eval(&mut self.solids, scope, &context)
            }
        }
    }
}
