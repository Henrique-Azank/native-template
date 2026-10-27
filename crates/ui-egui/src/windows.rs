//! Multi-window desktop manager using eframe deferred viewports.
//! All open windows share the exact same underlying Session state through thread-local storage.

use crate::TemplateApp;
use crate::router::Route;
use egui::{Context, ViewportBuilder, ViewportId};
use std::cell::RefCell;

#[derive(Clone, Debug)]
pub struct Window {
    pub id: u64,
    pub title: String,
    pub route: Route,
    pub size: egui::Vec2,
}

#[derive(Default)]
pub struct Windows {
    pub list: Vec<Window>,
    pub current: u64,
    last_id: u64,
}

thread_local! {
    static APP: RefCell<Option<TemplateApp>> = const { RefCell::new(None) };
}

pub fn install(app: TemplateApp) {
    APP.with(|a| *a.borrow_mut() = Some(app));
}

pub fn with_app<R>(f: impl FnOnce(&mut TemplateApp) -> R) -> Option<R> {
    APP.with(|a| a.try_borrow_mut().ok()?.as_mut().map(f))
}

pub fn viewport(id: u64) -> ViewportId {
    if id == 0 {
        ViewportId::ROOT
    } else {
        ViewportId::from_hash_of(("template_window", id))
    }
}

pub fn open_window(app: &mut TemplateApp, title: &str, route: Route) {
    app.windows.last_id += 1;
    let id = app.windows.last_id;
    app.windows.list.push(Window {
        id,
        title: title.to_string(),
        route,
        size: egui::vec2(900.0, 600.0),
    });
}

/// Request repainting of all open windows when shared state changes.
pub fn repaint_others(ctx: &Context) {
    with_app(|app| {
        for id in std::iter::once(0).chain(app.windows.list.iter().map(|w| w.id)) {
            ctx.request_repaint_of(viewport(id));
        }
    });
}

/// Render deferred viewports for all secondary windows.
pub fn show(app: &TemplateApp, ctx: &Context) {
    for w in &app.windows.list {
        let id = w.id;
        let builder = ViewportBuilder::default()
            .with_title(&w.title)
            .with_inner_size(w.size)
            .with_min_inner_size([400.0, 300.0]);

        ctx.show_viewport_deferred(viewport(id), builder, move |ui, _| {
            let _ = with_app(|app| {
                app.windows.current = id;
                draw_window(app, id, ui);
                app.windows.current = 0;
            });
        });
    }
}

fn draw_window(app: &mut TemplateApp, id: u64, ui: &mut egui::Ui) {
    let route = app
        .windows
        .list
        .iter()
        .find(|w| w.id == id)
        .map(|w| w.route.clone())
        .unwrap_or(Route::Home);

    egui::CentralPanel::default().show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.heading(format!("Window #{id} (Shared State)"));
        });
        ui.separator();

        match route {
            Route::Home => crate::pages::home::page(ui, app),
            Route::Details(entity_id) => crate::pages::details::page(ui, app, entity_id),
            Route::Settings => crate::pages::settings::page(ui, app),
            Route::NetworkDemo => crate::pages::network_demo::page(ui, app),
        }
    });
}
