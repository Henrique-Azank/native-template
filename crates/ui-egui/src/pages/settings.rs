use crate::TemplateApp;
use crate::components::{Button, ButtonProps, ButtonVariant, Card};
use crate::theme::Tokens;
use egui::{RichText, Slider, Ui};

pub fn page(ui: &mut Ui, app: &mut TemplateApp) {
    let t = Tokens::get(ui.ctx());

    ui.add_space(16.0);
    ui.heading(RichText::new("Settings & Preferences").size(22.0));
    ui.add_space(16.0);

    Card(ui, |ui| {
        ui.label(RichText::new("Appearance & Theme").strong());
        ui.add_space(8.0);

        let mut current_theme = app.session.prefs.theme.clone();
        ui.horizontal(|ui| {
            ui.radio_value(&mut current_theme, "system".into(), "System Default");
            ui.radio_value(&mut current_theme, "light".into(), "Light");
            ui.radio_value(&mut current_theme, "dark".into(), "Dark");
        });

        if current_theme != app.session.prefs.theme {
            app.session.prefs.theme = current_theme;
            app.save_prefs();
            crate::theme::apply(ui.ctx(), app.session.prefs.theme == "dark");
        }

        ui.add_space(16.0);
        ui.label(RichText::new("Interface Scale").strong());
        ui.add_space(4.0);

        let mut scale = app.session.prefs.ui_scale;
        if ui
            .add(Slider::new(&mut scale, 0.75..=2.0).text("Zoom"))
            .changed()
        {
            app.session.prefs.ui_scale = scale;
            ui.ctx().set_zoom_factor(scale);
            app.save_prefs();
        }
    });

    ui.add_space(16.0);

    Card(ui, |ui| {
        ui.label(RichText::new("Application & Session State").strong());
        ui.add_space(8.0);

        ui.label(format!("Open Items: {}", app.session.doc.len()));
        ui.label(format!("Revision Counter: {}", app.session.revision));
        ui.label(format!(
            "Dirty State: {}",
            if app.session.is_dirty() {
                "Unsaved changes"
            } else {
                "Clean (Saved to disk)"
            }
        ));

        ui.add_space(12.0);
        if Button(
            ui,
            ButtonProps::new("Save Changes to Disk").variant(ButtonVariant::Primary),
        )
        .clicked()
        {
            app.session.mark_saved();
            app.save_prefs();
            app.toast("Settings and session state saved.");
        }
    });

    ui.label(
        RichText::new("Persistent data is saved automatically to ~/.config/template-app/")
            .color(t.text_secondary)
            .small(),
    );
}
