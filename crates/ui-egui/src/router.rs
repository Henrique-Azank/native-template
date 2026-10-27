use crate::TemplateApp;
use crate::pages;
use template_core::EntityId;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Route {
    Home,
    Details(EntityId),
    Settings,
    NetworkDemo,
}

#[derive(Clone, Debug)]
pub struct Router {
    pub current: Route,
    pub history: Vec<Route>,
}

impl Default for Router {
    fn default() -> Self {
        Self {
            current: Route::Home,
            history: Vec::new(),
        }
    }
}

impl Router {
    pub fn navigate(&mut self, next: Route) {
        if self.current != next {
            self.history.push(self.current.clone());
            self.current = next;
        }
    }

    pub fn back(&mut self) -> bool {
        if let Some(prev) = self.history.pop() {
            self.current = prev;
            true
        } else {
            false
        }
    }
}

/// Dispatches rendering to the active page component matching the current route.
pub fn render(app: &mut TemplateApp, ui: &mut egui::Ui) {
    match app.router.current.clone() {
        Route::Home => pages::home::page(ui, app),
        Route::Details(id) => pages::details::page(ui, app, id),
        Route::Settings => pages::settings::page(ui, app),
        Route::NetworkDemo => pages::network_demo::page(ui, app),
    }
}
