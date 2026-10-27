use crate::TemplateApp;
use crate::components::button::{Button, ButtonProps, ButtonVariant};
use crate::router::Route;
use crate::theme::Tokens;
use crate::windows;
use egui::{Frame, Margin, RichText, Stroke, Ui};

#[allow(non_snake_case)]
pub fn Navbar(ui: &mut Ui, app: &mut TemplateApp) {
    let t = Tokens::get(ui.ctx());

    Frame::NONE
        .fill(t.bg_surface)
        .stroke(Stroke::new(1.0, t.border))
        .inner_margin(Margin::symmetric(16, 10))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // App Logo / Title
                ui.label(RichText::new("⚡ TemplateApp").strong().size(15.0));

                ui.add_space(16.0);

                // Nav Links
                let nav_link = |ui: &mut Ui, label: &str, active: bool| -> bool {
                    let text = if active {
                        RichText::new(label).strong().color(t.accent)
                    } else {
                        RichText::new(label).color(t.text_secondary)
                    };
                    ui.selectable_label(active, text).clicked()
                };

                let cur = app.router.current.clone();
                if nav_link(ui, "Dashboard", cur == Route::Home) {
                    app.router.navigate(Route::Home);
                }
                if nav_link(ui, "Network Demo", cur == Route::NetworkDemo) {
                    app.router.navigate(Route::NetworkDemo);
                }
                if nav_link(ui, "Settings", cur == Route::Settings) {
                    app.router.navigate(Route::Settings);
                }

                if app.session.is_dirty() {
                    ui.label(RichText::new("• unsaved").color(t.danger).small());
                }

                // Right-aligned controls
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Open New Window button
                    if Button(
                        ui,
                        ButtonProps::new("+ New Window").variant(ButtonVariant::Secondary),
                    )
                    .clicked()
                    {
                        windows::open_window(app, "Secondary Window", Route::Home);
                    }

                    // Theme toggle button
                    let is_dark = app.session.prefs.theme == "dark";
                    let icon = if is_dark { "☀️ Light" } else { "🌙 Dark" };
                    if Button(ui, ButtonProps::new(icon).variant(ButtonVariant::Secondary))
                        .clicked()
                    {
                        let next = if is_dark { "light" } else { "dark" };
                        app.session.prefs.theme = next.to_string();
                        app.save_prefs();
                        crate::theme::apply(ui.ctx(), next == "dark");
                    }
                });
            });
        });
}
