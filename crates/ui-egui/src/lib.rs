//! Layer 6: Presentation Layer built with egui & eframe.
//! Follows the Thin-UI and "everything is a command" architectural principles.
//! UI state mutations occur strictly via `app.run("cmd.id", params)`.

pub mod components;
pub mod hooks;
pub mod pages;
pub mod router;
pub mod theme;
pub mod windows;

use components::Navbar;
use router::Router;
use serde_json::Value;
use std::sync::mpsc::{Receiver, Sender, channel};
use template_engine::Session;
use template_engine::storage::Storage;
use template_net::{NetworkClient, RequestPolicy};
use windows::Windows;

#[derive(Default)]
pub struct NetworkState {
    pub is_loading: bool,
    pub content: Option<String>,
    pub from_cache: Option<bool>,
    pub error: Option<String>,
}

pub struct TemplateApp {
    pub session: Session,
    pub router: Router,
    pub net: NetworkClient,
    pub windows: Windows,
    pub network_state: NetworkState,
    pub toast_msg: Option<String>,
    storage: Storage,
    net_rx: Receiver<(Result<Vec<u8>, String>, bool)>,
    net_tx: Sender<(Result<Vec<u8>, String>, bool)>,
    ctx_handle: Option<egui::Context>,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateApp {
    pub fn new() -> Self {
        let session = Session::new();
        let router = Router::default();
        let net = NetworkClient::new();
        let windows = Windows::default();
        let storage = Storage::default();
        let (net_tx, net_rx) = channel();

        Self {
            session,
            router,
            net,
            windows,
            network_state: NetworkState::default(),
            toast_msg: None,
            storage,
            net_rx,
            net_tx,
            ctx_handle: None,
        }
    }

    /// The Command Boundary: all state mutations pass through the engine command bus.
    pub fn run(&mut self, cmd: &str, params: Value) -> Option<Value> {
        match self.session.execute(cmd, &params) {
            Ok(val) => {
                if let Some(ctx) = &self.ctx_handle {
                    windows::repaint_others(ctx);
                }
                Some(val)
            }
            Err(err) => {
                self.toast(format!("Command error: {err}"));
                None
            }
        }
    }

    pub fn toast(&mut self, msg: impl Into<String>) {
        self.toast_msg = Some(msg.into());
    }

    pub fn save_prefs(&self) {
        self.storage.save_prefs(&self.session.prefs);
    }

    /// Kicks off a non-blocking background HTTP fetch with dual-tier caching.
    pub fn fetch_network(&mut self, url: &str, policy: RequestPolicy, ttl_secs: u64) {
        self.network_state.is_loading = true;
        self.network_state.error = None;
        let tx = self.net_tx.clone();
        let ctx = self.ctx_handle.clone();

        self.net
            .fetch(url, policy, ttl_secs, move |res, from_cache| {
                let _ = tx.send((res, from_cache));
                if let Some(c) = &ctx {
                    c.request_repaint();
                }
            });
    }

    /// Polls completed background network responses.
    fn poll_network(&mut self) {
        while let Ok((res, from_cache)) = self.net_rx.try_recv() {
            self.network_state.is_loading = false;
            self.network_state.from_cache = Some(from_cache);
            match res {
                Ok(bytes) => {
                    let text =
                        String::from_utf8(bytes).unwrap_or_else(|_| "<Binary data>".to_string());
                    self.network_state.content = Some(text);
                    self.network_state.error = None;
                }
                Err(e) => {
                    self.network_state.error = Some(e);
                    self.network_state.content = None;
                }
            }
        }
    }

    pub fn logic(&mut self, ctx: &egui::Context) {
        self.ctx_handle = Some(ctx.clone());
        self.poll_network();
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        self.ctx_handle = Some(ctx.clone());

        // 1. Apply theme
        let is_dark = match self.session.prefs.theme.as_str() {
            "dark" => true,
            "light" => false,
            _ => ctx.global_style().visuals.dark_mode,
        };
        theme::apply(&ctx, is_dark);

        // 2. Render top navbar
        egui::Panel::top("navbar").show(ui, |ui| {
            Navbar(ui, self);
        });

        // 3. Render central page according to current route
        egui::CentralPanel::default().show(ui, |ui| {
            router::render(self, ui);

            // Toast notification banner
            if let Some(msg) = &self.toast_msg {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("ℹ {msg}"))
                            .color(theme::Tokens::get(&ctx).accent),
                    );
                });
            }
        });

        // 4. Render any open secondary windows (deferred viewports)
        windows::show(self, &ctx);
    }
}
