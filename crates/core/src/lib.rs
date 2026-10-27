//! Layer 0: Core primitives, entity identifiers, and foundational error types.
//! Completely headless with zero framework or OS dependencies.

use serde::{Deserialize, Serialize};

/// Stable identifier for entities, items, or nodes within the application.
/// Supports client-partitioned ID spaces (client << 32 | counter)
/// to guarantee zero collision in multi-window or collaborative contexts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EntityId(pub u64);

impl EntityId {
    pub const NONE: Self = Self(0);

    pub fn new(counter: u32, client: u32) -> Self {
        Self(((client as u64) << 32) | (counter as u64))
    }

    pub fn counter(&self) -> u32 {
        self.0 as u32
    }

    pub fn client(&self) -> u32 {
        (self.0 >> 32) as u32
    }
}

impl std::fmt::Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for EntityId {
    type Err = std::num::ParseIntError;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        s.trim().parse::<u64>().map(EntityId)
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone)]
pub enum AppError {
    #[error("entity {0} not found")]
    NotFound(EntityId),
    #[error("invalid parameter: {0}")]
    InvalidInput(String),
    #[error("internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_id_partition() {
        let id = EntityId::new(42, 7);
        assert_eq!(id.counter(), 42);
        assert_eq!(id.client(), 7);
    }
}
