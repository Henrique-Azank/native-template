use crate::TemplateApp;
use crate::components::{Badge, Button, ButtonProps, ButtonVariant, Card, TextInput};
use crate::hooks::use_state;
use crate::theme::Tokens;
use egui::{Id, RichText, ScrollArea, Ui};
use template_net::RequestPolicy;

pub fn page(ui: &mut Ui, app: &mut TemplateApp) {
    let ctx = ui.ctx().clone();
    let t = Tokens::get(&ctx);

    ui.add_space(16.0);
    ui.heading(RichText::new("Async Network & Dual-Tier Cache Demo").size(22.0));
    ui.label(RichText::new("Demonstrates non-blocking background HTTP requests with in-memory + persistent disk caching.").color(t.text_secondary));

    ui.add_space(16.0);

    Card(ui, |ui| {
        ui.label(RichText::new("Request Configuration").strong());
        ui.add_space(8.0);

        // URL input
        let (url, set_url) = use_state(&ctx, Id::new("net_demo_url"), || {
            "https://httpbin.org/json".to_string()
        });
        ui.label("Target URL:");
        if let Some(new_url) = TextInput(ui, Id::new("net_demo_url_input"), "https://...", &url) {
            set_url.set(&ctx, new_url);
        }

        ui.add_space(12.0);

        // Policy selector
        let (mut policy_idx, set_policy) = use_state(&ctx, Id::new("net_policy"), || 3usize);
        ui.label("Caching Policy:");
        ui.horizontal(|ui| {
            if ui.radio_value(&mut policy_idx, 0, "CacheOnly").changed() {
                set_policy.set(&ctx, 0);
            }
            if ui.radio_value(&mut policy_idx, 1, "NetworkOnly").changed() {
                set_policy.set(&ctx, 1);
            }
            if ui.radio_value(&mut policy_idx, 2, "CacheFirst").changed() {
                set_policy.set(&ctx, 2);
            }
            if ui
                .radio_value(&mut policy_idx, 3, "StaleWhileRevalidate")
                .changed()
            {
                set_policy.set(&ctx, 3);
            }
        });

        let policy = match policy_idx {
            0 => RequestPolicy::CacheOnly,
            1 => RequestPolicy::NetworkOnly,
            2 => RequestPolicy::CacheFirst,
            _ => RequestPolicy::StaleWhileRevalidate,
        };

        ui.add_space(16.0);

        // Action button
        let is_loading = app.network_state.is_loading;
        let btn_label = if is_loading {
            "Fetching in Background..."
        } else {
            "Execute Fetch"
        };
        if Button(
            ui,
            ButtonProps::new(btn_label)
                .variant(ButtonVariant::Primary)
                .disabled(is_loading),
        )
        .clicked()
        {
            app.fetch_network(&url, policy, 60);
        }
    });

    ui.add_space(16.0);

    // Response view
    Card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Response Viewer").strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if app.network_state.is_loading {
                    Badge(ui, "FETCHING", t.accent);
                } else if app.network_state.error.is_some() {
                    Badge(ui, "ERROR", t.danger);
                } else if app.network_state.content.is_some() {
                    Badge(ui, "CACHED / OK", t.success);
                }
            });
        });

        ui.add_space(8.0);

        if let Some(err) = &app.network_state.error {
            ui.label(RichText::new(format!("Error: {err}")).color(t.danger));
        } else if let Some(content) = &app.network_state.content {
            ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut content.as_str())
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY),
                );
            });
        } else {
            ui.label(
                RichText::new("No request performed yet. Click 'Execute Fetch' above.")
                    .color(t.text_secondary),
            );
        }
    });
}
