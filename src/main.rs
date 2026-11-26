#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod config;
mod fonts;
mod market_hours;
mod quote;
mod worker;

use crate::app::StockApp;
use crate::config::{AppConfig, ViewMode};

fn main() -> eframe::Result<()> {
    let initial_cfg = AppConfig::load();
    let [w, h] = initial_cfg.view_mode.default_size();
    let decorations = initial_cfg.view_mode != ViewMode::FloatingWidget;

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("simple_stock")
            .with_inner_size([w, h])
            .with_min_inner_size([220.0, 200.0])
            .with_decorations(decorations)
            .with_transparent(true),
        ..Default::default()
    };

    eframe::run_native(
        "simple_stock",
        options,
        Box::new(|cc| Ok(Box::new(StockApp::new(cc)))),
    )
}
