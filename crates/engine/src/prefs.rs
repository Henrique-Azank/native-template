use serde::{Deserialize, Serialize};

/// Application preferences persisted between sessions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AppPrefs {
    pub theme: String, // "system", "dark", "light"
    pub ui_scale: f32,
    pub language: String,
    pub recent_files: Vec<String>,
    pub window_size: (f32, f32),
}

impl Default for AppPrefs {
    fn default() -> Self {
        Self {
            theme: "system".into(),
            ui_scale: 1.0,
            language: "en".into(),
            recent_files: Vec::new(),
            window_size: (1280.0, 800.0),
        }
    }
}
