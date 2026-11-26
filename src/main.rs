#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod config;
mod fonts;
mod i18n;
mod icon;
mod market_hours;
mod quote;
mod storage;
mod worker;

use crate::app::StockApp;
use crate::config::{AppConfig, ViewMode};

#[cfg(target_os = "windows")]
fn create_app_mutex() -> *mut core::ffi::c_void {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateMutexW(
            lp_mutex_attributes: *mut core::ffi::c_void,
            b_initial_owner: i32,
            lp_name: *const u16,
        ) -> *mut core::ffi::c_void;
    }
    let name: Vec<u16> = "Local\\simple_stock_app_mutex\0".encode_utf16().collect();
    unsafe { CreateMutexW(std::ptr::null_mut(), 0, name.as_ptr()) }
}

fn main() -> eframe::Result<()> {
    // Keep a named Windows mutex alive while running so the installer can detect
    // and cleanly coordinate upgrades via Windows Restart Manager.
    #[cfg(target_os = "windows")]
    let _app_mutex = create_app_mutex();

    let initial_cfg = AppConfig::load();
    let [w, h] = initial_cfg.view_mode.default_size();
    let decorations = initial_cfg.view_mode != ViewMode::FloatingWidget;

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("simple_stock")
            .with_icon(icon::build_window_icon())
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
