//! Layer 5: Application Engine and Facade.
//! Centralizes state management (Session), command dispatch with panic isolation,
//! undo/redo Copy-on-Write history, and persistent preferences.

pub mod cmd;
pub mod prefs;
pub mod session_state;
pub mod storage;

use cmd::CommandSpec;
use prefs::AppPrefs;
use serde_json::Value;
use session_state::UserSession;
use std::collections::HashMap;
use std::sync::Arc;
use template_model::Document;

/// Snapshot for undo/redo history.
#[derive(Clone, Debug)]
pub struct HistoryEntry {
    pub doc: Arc<Document>,
}

pub struct Session {
    pub doc: Arc<Document>,
    saved_doc: Arc<Document>,
    pub undo_stack: Vec<HistoryEntry>,
    pub redo_stack: Vec<HistoryEntry>,
    pub prefs: AppPrefs,
    pub user_session: UserSession,
    pub revision: u64,
    commands: HashMap<&'static str, CommandSpec>,
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

impl Session {
    pub fn new() -> Self {
        let storage = storage::Storage::default();
        let doc = Arc::new(Document::default());
        let saved_doc = doc.clone();
        let prefs = storage.load_prefs();
        let user_session = storage.load_session();

        let mut commands = HashMap::new();
        for spec in cmd::all_specs() {
            commands.insert(spec.id, spec);
        }

        Self {
            doc,
            saved_doc,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            prefs,
            user_session,
            revision: 0,
            commands,
        }
    }

    /// Provides mutable access to the active document via Copy-on-Write branching.
    pub fn doc_mut(&mut self) -> &mut Document {
        Arc::make_mut(&mut self.doc)
    }

    /// Returns true if the document has uncommitted changes since the last disk save.
    pub fn is_dirty(&self) -> bool {
        !Arc::ptr_eq(&self.doc, &self.saved_doc)
    }

    /// Mark current state as saved to disk.
    pub fn mark_saved(&mut self) {
        self.saved_doc = self.doc.clone();
    }

    /// Executes a command inside a catch_unwind panic guard.
    /// Manages undo/redo snapshots automatically.
    pub fn execute(&mut self, id: &str, params: &Value) -> Result<Value, String> {
        let (run_fn, undoable) = {
            let spec = self
                .commands
                .get(id)
                .ok_or_else(|| format!("unknown command `{id}`"))?;
            (spec.run.clone(), spec.undoable)
        };
        let before_doc = self.doc.clone();

        // Panic isolation wrapper
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| (run_fn)(self, params)));

        match res {
            Ok(Ok(result)) => {
                if undoable && !Arc::ptr_eq(&self.doc, &before_doc) {
                    self.undo_stack.push(HistoryEntry { doc: before_doc });
                    self.redo_stack.clear();
                    self.revision = self.revision.saturating_add(1);
                }
                Ok(result)
            }
            Ok(Err(err)) => {
                // Rollback on logical error
                self.doc = before_doc;
                Err(err)
            }
            Err(panic_payload) => {
                // Rollback on panic
                self.doc = before_doc;
                let msg = panic_payload
                    .downcast_ref::<&str>()
                    .map(|s| s.to_string())
                    .or_else(|| panic_payload.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "panic in command execution".into());
                log::error!("command `{id}` panicked: {msg}");
                Err(format!("internal error in `{id}`: {msg}"))
            }
        }
    }

    pub fn undo(&mut self) -> Result<(), String> {
        let entry = self
            .undo_stack
            .pop()
            .ok_or_else(|| "nothing to undo".to_string())?;
        self.redo_stack.push(HistoryEntry {
            doc: self.doc.clone(),
        });
        self.doc = entry.doc;
        self.revision = self.revision.saturating_add(1);
        Ok(())
    }

    pub fn redo(&mut self) -> Result<(), String> {
        let entry = self
            .redo_stack
            .pop()
            .ok_or_else(|| "nothing to redo".to_string())?;
        self.undo_stack.push(HistoryEntry {
            doc: self.doc.clone(),
        });
        self.doc = entry.doc;
        self.revision = self.revision.saturating_add(1);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_command_undo_redo() {
        let mut session = Session::new();
        assert_eq!(session.doc.len(), 0);

        // Execute create item
        let res = session.execute(
            "item.create",
            &json!({ "title": "Buy Milk", "description": "2% organic" }),
        );
        assert!(res.is_ok());
        assert_eq!(session.doc.len(), 1);

        // Undo
        assert!(session.undo().is_ok());
        assert_eq!(session.doc.len(), 0);

        // Redo
        assert!(session.redo().is_ok());
        assert_eq!(session.doc.len(), 1);
    }
}
