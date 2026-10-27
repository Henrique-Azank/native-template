use crate::TemplateApp;
use crate::components::{Badge, Button, ButtonProps, ButtonVariant, Card};
use crate::router::Route;
use crate::theme::Tokens;
use egui::{RichText, Ui};
use serde_json::json;
use template_core::EntityId;

pub fn page(ui: &mut Ui, app: &mut TemplateApp, id: EntityId) {
    let t = Tokens::get(ui.ctx());

    ui.add_space(16.0);

    // Back navigation
    if Button(
        ui,
        ButtonProps::new("← Back to Dashboard").variant(ButtonVariant::Secondary),
    )
    .clicked()
    {
        app.router.navigate(Route::Home);
        return;
    }

    ui.add_space(16.0);

    let Some(item) = app.session.doc.get_item(id).cloned() else {
        Card(ui, |ui| {
            ui.label(RichText::new(format!("Item {id} not found.")).color(t.danger));
        });
        return;
    };

    Card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.heading(RichText::new(&item.title).size(20.0));
            if item.completed {
                Badge(ui, "Completed", t.success);
            } else {
                Badge(ui, "Active", t.accent);
            }
        });

        ui.add_space(8.0);
        ui.label(
            RichText::new(format!("ID: {id}"))
                .monospace()
                .color(t.text_secondary),
        );

        ui.add_space(16.0);
        ui.label(RichText::new("Description:").strong());
        ui.label(if item.description.is_empty() {
            "No description provided."
        } else {
            &item.description
        });

        ui.add_space(20.0);
        ui.horizontal(|ui| {
            let toggle_label = if item.completed {
                "Mark Incomplete"
            } else {
                "Mark Complete"
            };
            if Button(
                ui,
                ButtonProps::new(toggle_label).variant(ButtonVariant::Primary),
            )
            .clicked()
            {
                app.run("item.toggle", json!({ "id": id.0 }));
            }

            if Button(
                ui,
                ButtonProps::new("Delete Item").variant(ButtonVariant::Danger),
            )
            .clicked()
            {
                app.run("item.delete", json!({ "id": id.0 }));
                app.router.navigate(Route::Home);
            }
        });
    });
}
