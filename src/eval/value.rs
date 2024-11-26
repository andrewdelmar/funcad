use crate::SolidId;

use std::hash::Hash;

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

impl Value {
    pub(crate) const NUMBER_TYPE_NAME: &str = "number";
    pub(crate) const SOLID_TYPE_NAME: &str = "number";

    pub(crate) fn type_name(&self) -> &'static str {
        match self {
            Value::Number(_) => Self::NUMBER_TYPE_NAME,
            Value::Solid(_) => Self::SOLID_TYPE_NAME,
        }
    }
}
