//! Stock TW 程式進入點
//! 採用純 Windows 原生 Win32 GDI 精修介面（無 OpenGL、無內嵌字型檔），
//! 兼顧極低記憶體佔用（~6–15 MB）與細膩深色介面質感，並建立具名 Mutex 配合安裝升級。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod dialogs;
mod i18n;
mod market_hours;
mod quote;
mod storage;
mod ui_win32;
mod updater;
mod win32_util;
mod worker;

/// 建立本機具名 Mutex，供 Inno Setup 安裝程式透過 Restart Manager 偵測升級
fn create_app_mutex() -> *mut core::ffi::c_void {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateMutexW(
            lp_mutex_attributes: *mut core::ffi::c_void,
            b_initial_owner: i32,
            lp_name: *const u16,
        ) -> *mut core::ffi::c_void;
    }
    let name = win32_util::to_wide("Local\\simple_stock_app_mutex");
    unsafe { CreateMutexW(std::ptr::null_mut(), 0, name.as_ptr()) }
}

fn main() {
    let _app_mutex = create_app_mutex();
    ui_win32::run_win32_app();
}
