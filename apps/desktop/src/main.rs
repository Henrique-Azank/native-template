//! Template native desktop application entry point.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

use template_ui_egui::{TemplateApp, windows};

struct DesktopApp;

impl eframe::App for DesktopApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let _ = windows::with_app(|app| app.logic(ctx));
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let _ = windows::with_app(|app| app.ui(ui));
    }
}

fn main() -> eframe::Result {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let viewport = egui::ViewportBuilder::default()
        .with_title("TemplateApp")
        .with_inner_size([1280.0, 800.0])
        .with_min_inner_size([600.0, 400.0])
        .with_drag_and_drop(true);

    let options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    let app = TemplateApp::new();
    windows::install(app);

    eframe::run_native(
        "TemplateApp",
        options,
        Box::new(|_cc| Ok(Box::new(DesktopApp))),
    )
}
