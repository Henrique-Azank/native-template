use crate::TemplateApp;
use crate::components::{Badge, Button, ButtonProps, ButtonVariant, Card, TextInput};
use crate::router::Route;
use crate::theme::Tokens;
use egui::{Id, RichText, ScrollArea, Ui};
use serde_json::json;

pub fn page(ui: &mut Ui, app: &mut TemplateApp) {
    let t = Tokens::get(ui.ctx());

    ScrollArea::vertical().show(ui, |ui| {
        ui.add_space(16.0);

        // Header section
        ui.horizontal(|ui| {
            ui.heading(RichText::new("Dashboard").size(22.0));
            ui.add_space(8.0);
            Badge(ui, &format!("{} items", app.session.doc.len()), t.accent);

            // Undo / Redo toolbar
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let can_redo = !app.session.redo_stack.is_empty();
                if Button(
                    ui,
                    ButtonProps::new("Redo ↷")
                        .variant(ButtonVariant::Secondary)
                        .disabled(!can_redo),
                )
                .clicked()
                    && can_redo
                {
                    app.run("edit.redo", json!({}));
                }

                let can_undo = !app.session.undo_stack.is_empty();
                if Button(
                    ui,
                    ButtonProps::new("↶ Undo")
                        .variant(ButtonVariant::Secondary)
                        .disabled(!can_undo),
                )
                .clicked()
                    && can_undo
                {
                    app.run("edit.undo", json!({}));
                }
            });
        });

        ui.add_space(16.0);

        // Create new item section
        Card(ui, |ui| {
            ui.label(RichText::new("Add New Item").strong());
            ui.add_space(8.0);

            let mut title_to_add = None;
            ui.horizontal(|ui| {
                if let Some(text) = TextInput(
                    ui,
                    Id::new("new_item_title"),
                    "Item title (press Enter)...",
                    "",
                ) {
                    title_to_add = Some(text);
                }
                if Button(
                    ui,
                    ButtonProps::new("Add Item").variant(ButtonVariant::Primary),
                )
                .clicked()
                {
                    // Also read draft if Add button was clicked
                    let draft = ui
                        .data_mut(|d| d.get_temp::<String>(Id::new("new_item_title").with("draft")))
                        .unwrap_or_default();
                    if !draft.trim().is_empty() {
                        title_to_add = Some(draft);
                    }
                }
            });

            if let Some(title) = title_to_add {
                app.run(
                    "item.create",
                    json!({ "title": title, "description": "Created from dashboard" }),
                );
                ui.data_mut(|d| {
                    d.remove::<String>(Id::new("new_item_title").with("draft"));
                });
            }
        });

        ui.add_space(16.0);

        // List of items
        if app.session.doc.is_empty() {
            Card(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(20.0);
                    ui.label(
                        RichText::new(
                            "No items yet. Type a title above and press Enter to create one.",
                        )
                        .color(t.text_secondary),
                    );
                    ui.add_space(20.0);
                });
            });
        } else {
            let items: Vec<_> = app.session.doc.items().cloned().collect();

            for item in items {
                Card(ui, |ui| {
                    ui.horizontal(|ui| {
                        // Checkbox
                        let mut checked = item.completed;
                        if ui.checkbox(&mut checked, "").changed() {
                            app.run("item.toggle", json!({ "id": item.id.0 }));
                        }

                        // Title with link to details
                        let label = if item.completed {
                            RichText::new(&item.title)
                                .strikethrough()
                                .color(t.text_secondary)
                        } else {
                            RichText::new(&item.title).strong()
                        };

                        if ui.link(label).clicked() {
                            app.router.navigate(Route::Details(item.id));
                        }

                        // Actions
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if Button(
                                ui,
                                ButtonProps::new("Delete").variant(ButtonVariant::Danger),
                            )
                            .clicked()
                            {
                                app.run("item.delete", json!({ "id": item.id.0 }));
                            }

                            if item.completed {
                                Badge(ui, "Completed", t.success);
                            } else {
                                Badge(ui, "Pending", t.accent);
                            }
                        });
                    });
                });
                ui.add_space(8.0);
            }
        }
    });
}
