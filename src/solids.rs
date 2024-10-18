use std::fmt::Display;

use truck_modeling::Solid;

use crate::{error::EvalResult, EvalError};

/// A reference to a solid in SolidSet.
#[derive(Clone, Copy, Debug, PartialEq, Hash)]
pub enum SolidId {
    Regular(usize),
    Empty,
    Universal,
}

impl Display for SolidId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SolidId::Regular(id) => write!(f, "ID = {id}"),
            SolidId::Empty => write!(f, "Empty"),
            SolidId::Universal => write!(f, "Universal"),
        }
    }
}

/// A collection of [`Solid`]s.
pub struct SolidSet {
    solids: Vec<Solid>,
    tolerance: f64,
}

impl Default for SolidSet {
    fn default() -> Self {
        Self {
            solids: Default::default(),
            tolerance: Self::DEFAULT_TOLERANCE,
        }
    }
}

impl SolidSet {
    const DEFAULT_TOLERANCE: f64 = 0.0001;

    pub fn try_get<'src>(&self, id: &SolidId) -> EvalResult<'src, &Solid> {
        match id {
            SolidId::Regular(index) => self
                .solids
                .get(*index)
                .ok_or(EvalError::InvalidSolidId(*id)),
            SolidId::Empty | SolidId::Universal => Err(EvalError::InvalidSolidId(*id)),
        }
    }

    pub(crate) fn push(&mut self, new: Solid) -> SolidId {
        self.solids.push(new);
        SolidId::Regular(self.solids.len() - 1)
    }

    pub(crate) fn push_or_empty(&mut self, new: Option<Solid>) -> SolidId {
        match new {
            Some(new) => self.push(new),
            None => SolidId::Empty,
        }
    }

    pub(crate) fn negate<'src>(&mut self, solid: &SolidId) -> EvalResult<'src, SolidId> {
        match solid {
            SolidId::Regular(_) => {
                let mut new = self.try_get(solid)?.clone();
                new.not();
                Ok(self.push(new))
            }
            SolidId::Empty => Ok(SolidId::Universal),
            SolidId::Universal => Ok(SolidId::Empty),
        }
    }

    pub(crate) fn union<'src>(
        &mut self,
        lhs: &SolidId,
        rhs: &SolidId,
    ) -> EvalResult<'src, SolidId> {
        match (lhs, rhs) {
            (SolidId::Regular(_), SolidId::Regular(_)) => {
                let new =
                    truck_shapeops::or(self.try_get(lhs)?, self.try_get(rhs)?, self.tolerance);
                Ok(self.push_or_empty(new))
            }

            (SolidId::Empty, other) | (other, SolidId::Empty) => Ok(*other),
            (SolidId::Universal, _) | (_, SolidId::Universal) => Ok(SolidId::Universal),
        }
    }

    pub(crate) fn intersection<'src>(
        &mut self,
        lhs: &SolidId,
        rhs: &SolidId,
    ) -> EvalResult<'src, SolidId> {
        match (lhs, rhs) {
            (SolidId::Regular(_), SolidId::Regular(_)) => {
                let new =
                    truck_shapeops::and(self.try_get(lhs)?, self.try_get(rhs)?, self.tolerance);
                Ok(self.push_or_empty(new))
            }

            (SolidId::Empty, _) | (_, SolidId::Empty) => Ok(SolidId::Empty),
            (SolidId::Universal, other) | (other, SolidId::Universal) => Ok(*other),
        }
    }

    pub(crate) fn difference<'src>(
        &mut self,
        lhs: &SolidId,
        rhs: &SolidId,
    ) -> EvalResult<'src, SolidId> {
        match (lhs, rhs) {
            (SolidId::Regular(_), SolidId::Regular(_)) => {
                let mut rhs = self.try_get(rhs)?.clone();
                rhs.not();
                let new = truck_shapeops::and(self.try_get(lhs)?, &rhs, self.tolerance);
                Ok(self.push_or_empty(new))
            }

            (SolidId::Empty, _) | (_, SolidId::Universal) => Ok(SolidId::Empty),

            (lhs, SolidId::Empty) => Ok(*lhs),

            (SolidId::Universal, rhs) => self.negate(rhs),
        }
    }
}
