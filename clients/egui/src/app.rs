use eframe::egui;

#[derive(Default)]
pub struct OpenCodeApp {
    // Multi-session tabs (server-backed sessions in later milestones)
    tabs: Vec<Tab>,
    active: usize,
}

#[derive(Default, Clone)]
struct Tab {
    title: String,
}

impl OpenCodeApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::default();
        app.tabs.push(Tab { title: "Session 1".to_string() });
        app.active = 0;
        app
    }
}

impl eframe::App for OpenCodeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Top: Tabs
        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            ui.horizontal(|ui| {
                for (i, tab) in self.tabs.iter().enumerate() {
                    let selected = self.active == i;
                    if ui.selectable_label(selected, &tab.title).clicked() {
                        self.active = i;
                    }
                }
                if ui.button("+").clicked() {
                    let idx = self.tabs.len() + 1;
                    self.tabs.push(Tab { title: format!("Session {idx}") });
                    self.active = self.tabs.len() - 1;
                }
            });
        });

        // Center: Placeholder
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("OpenCode EGUI (M0)");
            ui.label("Thin client scaffold is running.");
            ui.label("Next: server discovery and session wiring.");
        });
    }
}
