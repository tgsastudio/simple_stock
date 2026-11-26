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

#[cfg(target_os = "windows")]
fn virtual_screen_bounds() -> (f32, f32, f32, f32) {
    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetSystemMetrics(n_index: i32) -> i32;
    }
    unsafe {
        let x = GetSystemMetrics(76) as f32; // SM_XVIRTUALSCREEN
        let y = GetSystemMetrics(77) as f32; // SM_YVIRTUALSCREEN
        let w = GetSystemMetrics(78).max(800) as f32; // SM_CXVIRTUALSCREEN
        let h = GetSystemMetrics(79).max(600) as f32; // SM_CYVIRTUALSCREEN
        (x, y, w, h)
    }
}

fn main() -> eframe::Result<()> {
    #[cfg(target_os = "windows")]
    let _app_mutex = create_app_mutex();

    let initial_cfg = AppConfig::load();
    let decorations = initial_cfg.view_mode != ViewMode::FloatingWidget;
    let saved_rect = initial_cfg.window_rect_for(initial_cfg.view_mode);

    #[cfg(target_os = "windows")]
    let (vx, vy, vw, vh) = virtual_screen_bounds();
    #[cfg(not(target_os = "windows"))]
    let (vx, vy, vw, vh) = (0.0f32, 0.0f32, 1920.0f32, 1080.0f32);

    let (w, h, pos) = if let Some(r) = saved_rect {
        let clamped_w = r.width.clamp(220.0, (vw - 40.0).max(320.0));
        let clamped_h = r.height.clamp(200.0, (vh - 40.0).max(260.0));
        // 確保視窗左上角一定落在目前可視螢幕範圍內，避免跑到螢幕外看起來像一直縮在工作列
        let in_bounds = r.x >= vx
            && r.x <= vx + vw - 120.0
            && r.y >= vy
            && r.y <= vy + vh - 120.0;
        let safe_pos = if in_bounds {
            [r.x, r.y]
        } else {
            [120.0, 120.0]
        };
        (clamped_w, clamped_h, safe_pos)
    } else {
        let [dw, dh] = initial_cfg.view_mode.default_size();
        (dw, dh, [120.0, 120.0])
    };

    let vp = egui::ViewportBuilder::default()
        .with_title("Stock TW")
        .with_icon(icon::build_window_icon())
        .with_inner_size([w, h])
        .with_min_inner_size([220.0, 200.0])
        .with_position(pos)
        .with_decorations(decorations)
        .with_visible(true)
        .with_transparent(true);

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
