//! Stock TW 程式進入點
//! 負責建立 Windows 具名 Mutex（供安裝升級偵測）、還原上次視窗位置與大小並啟動 egui 主視窗。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod config;
mod fonts;
mod i18n;
mod icon;
mod market_hours;
mod quote;
mod storage;
mod updater;
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
    #[cfg(target_os = "windows")]
    let _app_mutex = create_app_mutex();

    let initial_cfg = AppConfig::load();
    let decorations = initial_cfg.view_mode != ViewMode::FloatingWidget;
    let saved_rect = initial_cfg.window_rect_for(initial_cfg.view_mode);

    let (w, h, pos) = if let Some(r) = saved_rect {
        let valid_pos = if r.x > -2000.0 && r.x < 8000.0 && r.y > -2000.0 && r.y < 8000.0 {
            Some([r.x, r.y])
        } else {
            None
        };
        (r.width.max(220.0), r.height.max(200.0), valid_pos)
    } else {
        let [dw, dh] = initial_cfg.view_mode.default_size();
        (dw, dh, Some([120.0, 120.0]))
    };

    let mut vp = egui::ViewportBuilder::default()
        .with_title("Stock TW")
        .with_icon(icon::build_window_icon())
        .with_inner_size([w, h])
        .with_min_inner_size([220.0, 200.0])
        .with_decorations(decorations)
        .with_visible(true)
        .with_transparent(true);

    if let Some([px, py]) = pos {
        vp = vp.with_position([px, py]);
    }

    let options = eframe::NativeOptions {
        viewport: vp,
        ..Default::default()
    };

    eframe::run_native(
        "Stock TW",
        options,
        Box::new(|cc| Ok(Box::new(StockApp::new(cc)))),
    )
}
