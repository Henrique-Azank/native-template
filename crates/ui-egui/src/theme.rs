use egui::{Color32, Stroke};

#[derive(Clone, Copy, Debug)]
pub struct Tokens {
    pub bg_app: Color32,
    pub bg_surface: Color32,
    pub bg_card: Color32,
    pub border: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub accent: Color32,
    pub accent_hover: Color32,
    pub danger: Color32,
    pub success: Color32,
}

impl Tokens {
    pub fn light() -> Self {
        Self {
            bg_app: Color32::from_rgb(248, 249, 250),
            bg_surface: Color32::from_rgb(255, 255, 255),
            bg_card: Color32::from_rgb(255, 255, 255),
            border: Color32::from_rgb(222, 226, 230),
            text_primary: Color32::from_rgb(33, 37, 41),
            text_secondary: Color32::from_rgb(108, 117, 125),
            accent: Color32::from_rgb(13, 110, 253),
            accent_hover: Color32::from_rgb(11, 94, 215),
            danger: Color32::from_rgb(220, 53, 69),
            success: Color32::from_rgb(25, 135, 84),
        }
    }

    pub fn dark() -> Self {
        Self {
            bg_app: Color32::from_rgb(18, 18, 20),
            bg_surface: Color32::from_rgb(28, 28, 32),
            bg_card: Color32::from_rgb(34, 34, 40),
            border: Color32::from_rgb(50, 50, 58),
            text_primary: Color32::from_rgb(240, 240, 245),
            text_secondary: Color32::from_rgb(160, 160, 175),
            accent: Color32::from_rgb(79, 140, 255),
            accent_hover: Color32::from_rgb(105, 158, 255),
            danger: Color32::from_rgb(248, 81, 73),
            success: Color32::from_rgb(63, 185, 80),
        }
    }

    pub fn get(ctx: &egui::Context) -> Self {
        if ctx.global_style().visuals.dark_mode {
            Self::dark()
        } else {
            Self::light()
        }
    }
}

pub fn apply(ctx: &egui::Context, dark: bool) {
    let t = if dark {
        Tokens::dark()
    } else {
        Tokens::light()
    };
    let mut visuals = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };

    visuals.panel_fill = t.bg_app;
    visuals.window_fill = t.bg_surface;
    visuals.widgets.noninteractive.bg_fill = t.bg_surface;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, t.border);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, t.text_primary);

    visuals.widgets.inactive.bg_fill = t.bg_card;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, t.border);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, t.text_primary);

    visuals.widgets.hovered.bg_fill = t.bg_surface;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, t.accent);

    visuals.widgets.active.bg_fill = t.accent;
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);

    visuals.selection.bg_fill = t.accent.gamma_multiply(0.2);
    visuals.selection.stroke = Stroke::new(1.0, t.accent);

    ctx.set_visuals(visuals);
}
