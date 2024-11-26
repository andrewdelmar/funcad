use std::collections::BTreeMap;

use truck_modeling::{builder, Point3, Vector3};

use crate::{SolidSet, Value};

use super::{BuiltInArgDef, BuiltInStatic, EvalContext, EvalResult};

pub(super) struct Cube();

impl BuiltInStatic for Cube {
    const ARGS: &[BuiltInArgDef] = &[BuiltInArgDef {
        name: "size",
        default: Some(Value::Number(1.)),
    }];

    fn eval_static<'src>(
        solids: &mut SolidSet,
        args: &BTreeMap<String, Value>,
        context: &EvalContext,
    ) -> EvalResult<'src, Value> {
        let size = Self::num_arg("size", args, context)?;

        let coord = -0.5 * size;
        let vert = builder::vertex(Point3::new(coord, coord, coord));
        let edge = builder::tsweep(&vert, Vector3::unit_x() * size);
        let face = builder::tsweep(&edge, Vector3::unit_y() * size);
        let cube = builder::tsweep(&face, Vector3::unit_z() * size);
        let id = solids.push(cube);

        Ok(Value::Solid(id))
    }
}
