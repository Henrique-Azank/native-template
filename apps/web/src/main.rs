//! Template WebAssembly browser client entry point.

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
struct WebApp(template_ui_egui::TemplateApp);

#[cfg(target_arch = "wasm32")]
impl eframe::App for WebApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.0.logic(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.0.ui(ui);
    }
}

#[cfg(target_arch = "wasm32")]
fn main() {
    wasm_bindgen_futures::spawn_local(async {
        let web_options = eframe::WebOptions::default();

        let document = web_sys::window()
            .and_then(|w| w.document())
            .expect("missing window.document");
        let canvas = document
            .get_element_by_id("the_canvas_id")
            .and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok())
            .expect("missing canvas element #the_canvas_id");

        let runner = eframe::WebRunner::new();
        let _ = runner
            .start(
                canvas,
                web_options,
                Box::new(|_cc| Ok(Box::new(WebApp(template_ui_egui::TemplateApp::new())))),
            )
            .await;
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    println!("template-web is designed for wasm32. Build using `trunk serve apps/web/index.html`");
}
