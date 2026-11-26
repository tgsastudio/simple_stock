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

/// 建立本機具名 Mutex，供單一執行個體檢查與 Inno Setup 安裝程式透過 Restart Manager 偵測升級。
/// 若已有執行中的實例，會喚醒原本的主視窗並回傳 `None` 結束本次重複啟動。
fn acquire_single_instance() -> Option<*mut core::ffi::c_void> {
    use windows_sys::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        FindWindowW, IsIconic, SW_RESTORE, SW_SHOW, SetForegroundWindow, ShowWindow,
    };

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateMutexW(
            lp_mutex_attributes: *mut core::ffi::c_void,
            b_initial_owner: i32,
            lp_name: *const u16,
        ) -> *mut core::ffi::c_void;
    }

    let name = win32_util::to_wide("Local\\simple_stock_app_mutex");
    unsafe {
        let handle = CreateMutexW(std::ptr::null_mut(), 0, name.as_ptr());
        if !handle.is_null() && GetLastError() == ERROR_ALREADY_EXISTS {
            let cls = win32_util::to_wide("StockTW_MainWindow_v2");
            let existing = FindWindowW(cls.as_ptr(), std::ptr::null());
            if !existing.is_null() {
                if IsIconic(existing) != 0 {
                    ShowWindow(existing, SW_RESTORE);
                } else {
                    ShowWindow(existing, SW_SHOW);
                }
                SetForegroundWindow(existing);
            }
            CloseHandle(handle);
            return None;
        }
        Some(handle)
    }
}

fn main() {
    let Some(_app_mutex) = acquire_single_instance() else {
        return;
    };
    ui_win32::run_win32_app();
}
