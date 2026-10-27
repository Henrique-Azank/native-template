use egui::{Color32, Response, Ui, pos2, vec2};

/// Informational or status badge component.
#[allow(non_snake_case)]
pub fn Badge(ui: &mut Ui, label: &str, color: Color32) -> Response {
    let font = egui::FontId::proportional(11.0);
    let galley = ui.painter().layout_no_wrap(label.to_string(), font, color);
    let size = vec2(galley.size().x + 14.0, 20.0);

    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::hover());

    ui.painter()
        .rect_filled(rect, 10.0, color.gamma_multiply(0.15));
    let text_pos = pos2(
        rect.center().x - galley.size().x / 2.0,
        rect.center().y - galley.size().y / 2.0,
    );
    ui.painter().galley(text_pos, galley, color);

    resp
}
