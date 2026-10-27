use crate::prefs::AppPrefs;
use crate::session_state::UserSession;
use std::fs;
use std::path::PathBuf;

pub struct Storage {
    config_dir: Option<PathBuf>,
}

impl Default for Storage {
    fn default() -> Self {
        Self::new("TemplateApp")
    }
}

pub fn default_config_dir(app_name: &str) -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    let base =
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Application Support"));
    #[cfg(target_os = "windows")]
    let base = std::env::var_os("APPDATA").map(PathBuf::from);
    #[cfg(all(unix, not(target_os = "macos")))]
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")));
    base.map(|b| b.join(app_name))
}

impl Storage {
    pub fn new(app_name: &str) -> Self {
        let config_dir = default_config_dir(app_name);
        if let Some(dir) = &config_dir {
            let _ = fs::create_dir_all(dir);
        }
        Self { config_dir }
    }

    pub fn load_prefs(&self) -> AppPrefs {
        let Some(dir) = &self.config_dir else {
            return AppPrefs::default();
        };
        let file = dir.join("prefs.json");
        fs::read_to_string(file)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save_prefs(&self, prefs: &AppPrefs) {
        let Some(dir) = &self.config_dir else { return };
        let file = dir.join("prefs.json");
        if let Ok(data) = serde_json::to_string_pretty(prefs) {
            let _ = fs::write(file, data);
        }
    }

    pub fn load_session(&self) -> UserSession {
        let Some(dir) = &self.config_dir else {
            return UserSession::default();
        };
        let file = dir.join("session.json");
        fs::read_to_string(file)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save_session(&self, session: &UserSession) {
        let Some(dir) = &self.config_dir else { return };
        let file = dir.join("session.json");
        if let Ok(data) = serde_json::to_string_pretty(session) {
            let _ = fs::write(file, data);
        }
    }
}
