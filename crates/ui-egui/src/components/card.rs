use crate::theme::Tokens;
use egui::{Frame, Margin, Response, Stroke, Ui};

/// Reusable card component wrapping child content in a styled container.
#[allow(non_snake_case)]
pub fn Card(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui)) -> Response {
    let t = Tokens::get(ui.ctx());
    Frame::NONE
        .fill(t.bg_card)
        .stroke(Stroke::new(1.0, t.border))
        .corner_radius(8.0)
        .inner_margin(Margin::same(16))
        .show(ui, add_contents)
        .response
}
