use crate::hooks::use_state;
use egui::{Id, TextEdit, Ui};

/// Controlled text input holding uncommitted draft text in local temporary state.
/// Returns `Some(text)` when the user presses Enter or changes text.
#[allow(non_snake_case)]
pub fn TextInput(ui: &mut Ui, id: Id, placeholder: &str, current_value: &str) -> Option<String> {
    let ctx = ui.ctx().clone();
    let (mut draft, set_draft) = use_state(&ctx, id.with("draft"), || current_value.to_string());

    let edit = TextEdit::singleline(&mut draft)
        .hint_text(placeholder)
        .desired_width(f32::INFINITY);

    let resp = ui.add(edit);

    if resp.changed() {
        set_draft.set(&ctx, draft.clone());
    }

    if resp.lost_focus()
        && ui.input(|i| i.key_pressed(egui::Key::Enter))
        && !draft.trim().is_empty()
    {
        return Some(draft);
    }

    None
}
