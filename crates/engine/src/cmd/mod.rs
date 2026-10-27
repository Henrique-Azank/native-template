pub mod document;

use crate::Session;
use serde_json::Value;
use std::sync::Arc;

pub type CommandResult = Result<Value, String>;
pub type CommandFn = Arc<dyn Fn(&mut Session, &Value) -> CommandResult + Send + Sync>;

#[derive(Clone)]
pub struct CommandSpec {
    pub id: &'static str,
    pub label: &'static str,
    pub params: &'static [&'static str],
    pub undoable: bool,
    pub journal: bool,
    pub run: CommandFn,
}

pub fn all_specs() -> Vec<CommandSpec> {
    let mut specs = Vec::new();
    specs.extend(document::specs());
    specs
}
