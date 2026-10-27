use crate::theme::Tokens;
use egui::{Color32, Response, Stroke, Ui, vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Danger,
    Outline,
}

pub struct ButtonProps<'a> {
    pub label: &'a str,
    pub variant: ButtonVariant,
    pub disabled: bool,
}

impl<'a> ButtonProps<'a> {
    pub fn new(label: &'a str) -> Self {
        Self {
            label,
            variant: ButtonVariant::Primary,
            disabled: false,
        }
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// Custom styled button component.
#[allow(non_snake_case)]
pub fn Button(ui: &mut Ui, props: ButtonProps<'_>) -> Response {
    let t = Tokens::get(ui.ctx());
    let (bg, text_color, border) = match props.variant {
        ButtonVariant::Primary => (t.accent, Color32::WHITE, Stroke::NONE),
        ButtonVariant::Secondary => (t.bg_card, t.text_primary, Stroke::new(1.0, t.border)),
        ButtonVariant::Danger => (t.danger, Color32::WHITE, Stroke::NONE),
        ButtonVariant::Outline => (Color32::TRANSPARENT, t.accent, Stroke::new(1.0, t.accent)),
    };

    let font = egui::FontId::proportional(13.0);
    let galley = ui
        .painter()
        .layout_no_wrap(props.label.to_string(), font, text_color);
    let size = vec2(galley.size().x + 20.0, 32.0);

    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());

    if props.disabled {
        ui.painter().rect_filled(rect, 6.0, bg.gamma_multiply(0.5));
    } else if resp.is_pointer_button_down_on() {
        ui.painter().rect_filled(rect, 6.0, bg.gamma_multiply(0.8));
    } else if resp.hovered() {
        ui.painter().rect_filled(rect, 6.0, bg.gamma_multiply(0.9));
    } else {
        ui.painter().rect_filled(rect, 6.0, bg);
    }

    if border != Stroke::NONE {
        ui.painter()
            .rect_stroke(rect, 6.0, border, egui::StrokeKind::Inside);
    }

    let text_pos = egui::pos2(
        rect.center().x - galley.size().x / 2.0,
        rect.center().y - galley.size().y / 2.0,
    );
    ui.painter().galley(text_pos, galley, text_color);

    resp
}
