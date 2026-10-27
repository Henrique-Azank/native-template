use egui::Id;
use std::marker::PhantomData;

#[derive(Clone, Copy)]
pub struct StateSetter<T> {
    id: Id,
    _marker: PhantomData<T>,
}

impl<T: Clone + Send + Sync + 'static> StateSetter<T> {
    pub fn set(&self, ctx: &egui::Context, next: T) {
        ctx.data_mut(|d| d.insert_temp(self.id, next));
    }
}

/// React-like `useState` hook for immediate-mode widgets.
/// Retrieves uncommitted draft/temporary state from egui's data store,
/// and returns `(current_value, setter)`.
pub fn use_state<T: Clone + Send + Sync + 'static>(
    ctx: &egui::Context,
    id: Id,
    initial: impl FnOnce() -> T,
) -> (T, StateSetter<T>) {
    let current = ctx
        .data_mut(|d| d.get_temp::<T>(id))
        .unwrap_or_else(initial);
    (
        current,
        StateSetter {
            id,
            _marker: PhantomData,
        },
    )
}

/// Reset state held at `id`.
pub fn use_reset<T: Send + Sync + Default + 'static>(ctx: &egui::Context, id: Id) {
    ctx.data_mut(|d| {
        let _ = d.remove_temp::<T>(id);
    });
}
