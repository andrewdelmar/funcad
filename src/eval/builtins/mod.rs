mod shapes;

mod math;

use std::collections::BTreeMap;

use crate::{ast::*, error::EvalResult, EvalErrorType, SolidSet, SpannedFuncCallExpr, Value};

use super::{EvalCache, EvalContext, Scope};

pub(crate) trait BuiltIn {
    fn arg_defs(&self) -> &'static [BuiltInArgDef];

    fn eval<'src>(
        &self,
        solids: &mut SolidSet,
        scope: &Scope,
        context: &EvalContext,
    ) -> EvalResult<'src, Value>;

    fn add_default_args<'src>(
        &self,
        args: &mut BTreeMap<String, Value>,
        context: &EvalContext,
    ) -> EvalResult<'src, ()> {
        for def in self.arg_defs() {
            if !args.contains_key(def.name) {
                let Some(ref val) = def.default else {
                    return context.eval_err(EvalErrorType::NoSuppliedOrDefaultArg {
                        name: def.name.into(),
                    });
                };
                args.insert(def.name.into(), val.clone());
            }
        }

        Ok(())
    }
}

pub(crate) struct BuiltInArgDef {
    name: &'static str,
    default: Option<Value>,
}

trait BuiltInStatic {
    const ARGS: &[BuiltInArgDef];

    fn eval_static<'src>(
        solids: &mut SolidSet,
        args: &BTreeMap<String, Value>,
        context: &EvalContext,
    ) -> EvalResult<'src, Value>;

    fn num_arg<'src>(
        name: &str,
        args: &BTreeMap<String, Value>,
        context: &EvalContext,
    ) -> EvalResult<'src, f64> {
        let Some(val) = args.get(name) else {
            return context.eval_err(EvalErrorType::ArgNotFound { name: name.into() });
        };

        let Value::Number(num) = val else {
            return context.eval_err(EvalErrorType::ArgWrongType {
                name: name.into(),
                expected: Value::NUMBER_TYPE_NAME,
                got: val.type_name(),
            });
        };

        Ok(*num)
    }
}

impl<T: BuiltInStatic> BuiltIn for T {
    fn arg_defs(&self) -> &'static [BuiltInArgDef] {
        T::ARGS
    }

    fn eval<'src>(&self, 
        solids: &mut SolidSet,
        scope: &Scope, context: &EvalContext) -> EvalResult<'src, Value> {
        let args = scope.args();

        Self::eval_static(solids, args, context)
    }
}

impl<'set, 'src> EvalCache<'set, 'src> {
    pub(super) fn eval_built_in_call_args(
        &mut self,
        call_expr: &SpannedFuncCallExpr<'src>,
        built_in: &dyn BuiltIn,
        scope: &Scope,
        context: &EvalContext,
    ) -> EvalResult<'src, BTreeMap<String, Value>> {
        let mut args =
            self.eval_supplied_built_in_call_args(call_expr, built_in, scope, context)?;
        built_in.add_default_args(&mut args, context)?;
        Ok(args)
    }

    /// Evaluates supplied arguments and returns an error if there are too many.
    fn eval_supplied_built_in_call_args(
        &mut self,
        func_call: &SpannedFuncCallExpr<'src>,
        built_in: &dyn BuiltIn,
        scope: &Scope,
        context: &EvalContext,
    ) -> EvalResult<'src, BTreeMap<String, Value>> {
        let arg_defs = built_in.arg_defs();

        match &func_call.args {
            CallArgs::None => Ok(BTreeMap::new()),
            CallArgs::Positional(args) => {
                let mut arg_vals = BTreeMap::new();

                for (arg_index, arg_expr) in args.iter().enumerate() {
                    let Some(arg_def) = arg_defs.get(arg_index) else {
                        return context.eval_err(EvalErrorType::TooManyArgs);
                    };

                    let val = self.eval_expr(&arg_expr, scope, context)?;
                    arg_vals.insert(arg_def.name.into(), val);
                }

                Ok(arg_vals)
            }
            CallArgs::Named(args) => {
                let mut arg_vals = BTreeMap::new();

                for (name, arg) in args {
                    if arg_defs.iter().all(|f| &f.name != name) {
                        return context.eval_err(EvalErrorType::InvalidNamedArg {
                            name: (*name).into(),
                        });
                    }

                    let val = self.eval_expr(&arg.expr, scope, context)?;
                    arg_vals.insert((*name).into(), val);
                }

                Ok(arg_vals)
            }
        }
    }

    pub(crate) fn get_built_in_func(name: &str) -> Option<&'static dyn BuiltIn> {
        match name {
            "Cube" => Some(&shapes::Cube() as &dyn BuiltIn),

            "Sin" => Some(&math::Sin() as &dyn BuiltIn),
            "Cos" => Some(&math::Cos() as &dyn BuiltIn),
            "Tan" => Some(&math::Tan() as &dyn BuiltIn),
            _ => None,
        }
    }
}
