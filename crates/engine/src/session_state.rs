use serde::{Deserialize, Serialize};
use template_core::EntityId;

/// User session state persisted between launches.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserSession {
    pub auth_token: Option<String>,
    pub username: Option<String>,
    pub last_route: String,
    pub active_item_id: Option<EntityId>,
}

impl Default for UserSession {
    fn default() -> Self {
        Self {
            auth_token: None,
            username: None,
            last_route: "home".into(),
            active_item_id: None,
        }
    }
}
