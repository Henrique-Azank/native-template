use serde::{Deserialize, Serialize};
use template_core::EntityId;

/// A sample entity illustrating domain modeling within the persistent tree.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    pub id: EntityId,
    pub title: String,
    pub description: String,
    pub completed: bool,
    pub created_at: u64,
}

impl Item {
    pub fn new(id: EntityId, title: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id,
            title: title.into(),
            description: description.into(),
            completed: false,
            created_at: 0,
        }
    }
}
