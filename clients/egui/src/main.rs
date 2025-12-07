#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
pub mod error; // contains api, events, discovery, spawn submodules
pub mod discovery;
pub mod client;

use eframe::egui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 720.0])
            .with_title("OpenCode EGUI"),
        ..Default::default()
    };

    eframe::run_native(
        "OpenCode EGUI",
        options,
        Box::new(|cc| Ok(Box::new(app::OpenCodeApp::new(cc))))
    )
}
