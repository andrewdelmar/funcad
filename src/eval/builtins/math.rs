use std::collections::BTreeMap;

use truck_modeling::cgmath::AbsDiffEq;

use crate::{EvalErrorType, SolidSet, Value};

use super::{BuiltInArgDef, BuiltInStatic, EvalContext, EvalResult};

pub(super) struct Sin();

impl BuiltInStatic for Sin {
    const ARGS: &[BuiltInArgDef] = &[BuiltInArgDef {
        name: "angle",
        default: None,
    }];

    fn eval_static<'src>(
        _solids: &mut SolidSet,
        args: &BTreeMap<String, Value>,
        context: &EvalContext,
    ) -> EvalResult<'src, Value> {
        let angle = Self::num_arg("angle", args, context)?;

        Ok(Value::Number(f64::sin(angle.to_radians())))
    }
}

pub(super) struct Cos();

impl BuiltInStatic for Cos {
    const ARGS: &[BuiltInArgDef] = &[BuiltInArgDef {
        name: "angle",
        default: None,
    }];

    fn eval_static<'src>(
        _solids: &mut SolidSet,
        args: &BTreeMap<String, Value>,
        context: &EvalContext,
    ) -> EvalResult<'src, Value> {
        let angle = Self::num_arg("angle", args, context)?;

        Ok(Value::Number(f64::cos(angle.to_radians())))
    }
}

pub(super) struct Tan();

impl BuiltInStatic for Tan {
    const ARGS: &[BuiltInArgDef] = &[BuiltInArgDef {
        name: "angle",
        default: None,
    }];

    fn eval_static<'src>(
        solids: &mut SolidSet,
        args: &BTreeMap<String, Value>,
        context: &EvalContext,
    ) -> EvalResult<'src, Value> {
        let angle = Self::num_arg("angle", args, context)?;

        // f64::tan doesn't actually ever return infinity since it operates in
        // radians and PI/2 is irrational.
        // When dealing with degrees, Tan(90) should be undefined though.
        if angle.rem_euclid(90.).abs_diff_eq(&0., solids.tolerance) {
            return context.eval_err(EvalErrorType::NumExprNotFinite);
        }

        Ok(Value::Number(f64::tan(angle.to_radians())))
    }
}
