//! Layer 1: Document Object Model (DOM) using persistent data structures (imbl).
//! Changes to this tree use Copy-on-Write (COW) branching with structural sharing,
//! enabling zero-cost O(1) snapshots for instant undo/redo.

pub mod item;
pub use item::Item;

use imbl::OrdMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use template_core::EntityId;

/// A whole application document state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Document {
    pub name: String,
    pub client_id: u32,
    next_id: u32,
    items: OrdMap<EntityId, Arc<Item>>,
}

impl Default for Document {
    fn default() -> Self {
        Self::new("Untitled Document")
    }
}

impl Document {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            client_id: 0,
            next_id: 1,
            items: OrdMap::new(),
        }
    }

    /// Allocates a new collision-free EntityId in this document's partitioned space.
    pub fn alloc_id(&mut self) -> EntityId {
        let id = EntityId::new(self.next_id, self.client_id);
        self.next_id = self.next_id.saturating_add(1);
        id
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn get_item(&self, id: EntityId) -> Option<&Arc<Item>> {
        self.items.get(&id)
    }

    pub fn items(&self) -> impl Iterator<Item = &Arc<Item>> {
        self.items.values()
    }

    /// Inserts or replaces an item in O(log N) time with structural sharing.
    pub fn insert_item(&mut self, item: Item) -> EntityId {
        let id = item.id;
        self.items.insert(id, Arc::new(item));
        id
    }

    /// Removes an item in O(log N) time.
    pub fn remove_item(&mut self, id: EntityId) -> Option<Arc<Item>> {
        self.items.remove(&id)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_structural_sharing() {
        let mut doc1 = Document::new("Doc 1");
        let id1 = doc1.alloc_id();
        doc1.insert_item(Item::new(id1, "Item 1", "Desc 1"));

        // Snapshot is an O(1) Arc clone of the imbl tree root
        let doc2 = doc1.clone();

        // Mutating doc1 creates a COW branch for the new item, sharing existing items with doc2
        let id2 = doc1.alloc_id();
        doc1.insert_item(Item::new(id2, "Item 2", "Desc 2"));

        assert_eq!(doc1.len(), 2);
        assert_eq!(doc2.len(), 1);
        assert!(Arc::ptr_eq(
            doc1.get_item(id1).unwrap(),
            doc2.get_item(id1).unwrap()
        ));
    }
}
