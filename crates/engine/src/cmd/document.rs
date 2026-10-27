use crate::cmd::CommandSpec;
use serde_json::{Value, json};
use std::sync::Arc;
use template_core::EntityId;
use template_model::Item;

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec {
            id: "item.create",
            label: "Create Item",
            params: &["title", "description"],
            undoable: true,
            journal: true,
            run: Arc::new(|session, params| {
                let title = params
                    .get("title")
                    .and_then(Value::as_str)
                    .unwrap_or("New Item");
                let desc = params
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or("");

                let doc = session.doc_mut();
                let id = doc.alloc_id();
                let item = Item::new(id, title, desc);
                doc.insert_item(item);

                Ok(json!({ "id": id.0 }))
            }),
        },
        CommandSpec {
            id: "item.toggle",
            label: "Toggle Item Completion",
            params: &["id"],
            undoable: true,
            journal: true,
            run: Arc::new(|session, params| {
                let id_u64 = params
                    .get("id")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| "missing item id".to_string())?;
                let id = EntityId(id_u64);

                let doc = session.doc_mut();
                let (mut updated, was_completed) = {
                    let existing = doc
                        .get_item(id)
                        .ok_or_else(|| format!("item {id} not found"))?;
                    ((**existing).clone(), existing.completed)
                };
                updated.completed = !was_completed;
                doc.insert_item(updated);

                Ok(json!({ "id": id.0, "completed": !was_completed }))
            }),
        },
        CommandSpec {
            id: "item.delete",
            label: "Delete Item",
            params: &["id"],
            undoable: true,
            journal: true,
            run: Arc::new(|session, params| {
                let id_u64 = params
                    .get("id")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| "missing item id".to_string())?;
                let id = EntityId(id_u64);

                let doc = session.doc_mut();
                doc.remove_item(id)
                    .ok_or_else(|| format!("item {id} not found"))?;

                Ok(json!({ "deleted": id.0 }))
            }),
        },
        CommandSpec {
            id: "doc.rename",
            label: "Rename Document",
            params: &["name"],
            undoable: true,
            journal: true,
            run: Arc::new(|session, params| {
                let name = params
                    .get("name")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing document name".to_string())?;

                session.doc_mut().name = name.to_string();
                Ok(json!({ "name": name }))
            }),
        },
        CommandSpec {
            id: "edit.undo",
            label: "Undo",
            params: &[],
            undoable: false,
            journal: false,
            run: Arc::new(|session, _| {
                session.undo()?;
                Ok(json!({ "undone": true }))
            }),
        },
        CommandSpec {
            id: "edit.redo",
            label: "Redo",
            params: &[],
            undoable: false,
            journal: false,
            run: Arc::new(|session, _| {
                session.redo()?;
                Ok(json!({ "redone": true }))
            }),
        },
    ]
}
