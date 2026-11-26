//! Win32 工具函式與數值格式化輔助模組
//! 提供 UTF-16 字串轉換、GDI 色彩轉換、螢幕邊界查詢、Windows 11 深色標題列、瀏覽器開啟與千分位格式化。

use std::ptr;
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::Graphics::Gdi::{
    CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, CreateFontW, DEFAULT_CHARSET, DEFAULT_PITCH,
    FF_DONTCARE, FW_BOLD, FW_NORMAL, FW_SEMIBOLD, HFONT, OUT_DEFAULT_PRECIS,
};
use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SW_SHOWNORMAL};

/// 將 Rust `&str` 轉換為以 `\0` 結尾的 Windows UTF-16 寬字串
pub fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// 組合 Win32 GDI `COLORREF`（格式為 `0x00BBGGRR`）
#[inline]
pub const fn rgb(r: u8, g: u8, b: u8) -> u32 {
    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16)
}

/// 查詢目前 Windows 虛擬螢幕的可視邊界 `(x, y, width, height)`（支援多螢幕）
pub fn virtual_screen_bounds() -> (i32, i32, i32, i32) {
    unsafe {
        let x = GetSystemMetrics(76); // SM_XVIRTUALSCREEN
        let y = GetSystemMetrics(77); // SM_YVIRTUALSCREEN
        let w = GetSystemMetrics(78).max(800); // SM_CXVIRTUALSCREEN
        let h = GetSystemMetrics(79).max(600); // SM_CYVIRTUALSCREEN
        (x, y, w, h)
    }
}

/// 建立微軟正黑體 UI 字型（直接使用 Windows 內建 ClearType 字型引擎，零額外字型檔記憶體）
pub fn create_ui_font(height_px: i32, weight: u32) -> HFONT {
    let face = to_wide("Microsoft JhengHei UI");
    unsafe {
        CreateFontW(
            -height_px,
            0,
            0,
            0,
            weight as i32,
            0,
            0,
            0,
            DEFAULT_CHARSET as u32,
            OUT_DEFAULT_PRECIS as u32,
            CLIP_DEFAULT_PRECIS as u32,
            CLEARTYPE_QUALITY as u32,
            (DEFAULT_PITCH | FF_DONTCARE) as u32,
            face.as_ptr(),
        )
    }
}

pub fn font_regular(height_px: i32) -> HFONT {
    create_ui_font(height_px, FW_NORMAL)
}

pub fn font_semibold(height_px: i32) -> HFONT {
    create_ui_font(height_px, FW_SEMIBOLD)
}

pub fn font_bold(height_px: i32) -> HFONT {
    create_ui_font(height_px, FW_BOLD)
}

/// 透過 Windows DWM API 設定深色或淺色視窗標題列（Windows 10/11 支援）
pub fn apply_dark_title_bar(hwnd: HWND, dark: bool) {
    #[link(name = "dwmapi")]
    unsafe extern "system" {
        fn DwmSetWindowAttribute(
            hwnd: HWND,
            dw_attribute: u32,
            pv_attribute: *const core::ffi::c_void,
            cb_attribute: u32,
        ) -> i32;
    }
    const DWMWA_USE_IMMUTABLE_DARK_MODE: u32 = 20;
    let val: i32 = if dark { 1 } else { 0 };
    unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMUTABLE_DARK_MODE,
            &val as *const i32 as *const core::ffi::c_void,
            std::mem::size_of::<i32>() as u32,
        );
    }
}

/// 使用預設瀏覽器開啟指定網址（用於下載 GitHub Release 更新）
pub fn open_url_in_browser(url: &str) {
    let op = to_wide("open");
    let target = to_wide(url);
    unsafe {
        ShellExecuteW(
            ptr::null_mut(),
            op.as_ptr(),
            target.as_ptr(),
            ptr::null(),
            ptr::null(),
            SW_SHOWNORMAL,
        );
    }
}

/// Windows 開機自動啟動（HKCU\...\Run）登錄檔存取，查詢與寫入共用同一組宣告
mod autostart_reg {
    use super::to_wide;
    use std::ptr;

    pub type Hkey = *mut core::ffi::c_void;
    pub const HKEY_CURRENT_USER: Hkey = (-2147483647i32 as isize) as Hkey;
    pub const KEY_READ: u32 = 0x0002_0019;
    pub const KEY_SET_VALUE: u32 = 0x0002;
    pub const REG_SZ: u32 = 1;
    pub const REG_EXPAND_SZ: u32 = 2;
    pub const ERROR_FILE_NOT_FOUND: i32 = 2;
    const APPMODEL_ERROR_NO_PACKAGE: i32 = 15700;

    pub const SUBKEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
    pub const VALUE_NAME: &str = "Stock TW";

    #[link(name = "advapi32")]
    unsafe extern "system" {
        pub fn RegOpenKeyExW(
            hkey: Hkey,
            lp_sub_key: *const u16,
            ul_options: u32,
            sam_desired: u32,
            phk_result: *mut Hkey,
        ) -> i32;
        pub fn RegQueryValueExW(
            hkey: Hkey,
            lp_value_name: *const u16,
            lp_reserved: *mut u32,
            lp_type: *mut u32,
            lp_data: *mut u8,
            lpcb_data: *mut u32,
        ) -> i32;
        pub fn RegCreateKeyExW(
            hkey: Hkey,
            lp_sub_key: *const u16,
            reserved: u32,
            lp_class: *const u16,
            dw_options: u32,
            sam_desired: u32,
            lp_security_attributes: *const core::ffi::c_void,
            phk_result: *mut Hkey,
            lpdw_disposition: *mut u32,
        ) -> i32;
        pub fn RegSetValueExW(
            hkey: Hkey,
            lp_value_name: *const u16,
            reserved: u32,
            dw_type: u32,
            lp_data: *const u8,
            cb_data: u32,
        ) -> i32;
        pub fn RegDeleteValueW(hkey: Hkey, lp_value_name: *const u16) -> i32;
        pub fn RegCloseKey(hkey: Hkey) -> i32;
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentPackageFullName(length: *mut u32, name: *mut u16) -> i32;
    }

    /// 是否以 MSIX 封裝身分執行（封裝應用程式的 HKCU Run 寫入會被虛擬化而不生效）
    pub fn is_packaged() -> bool {
        let mut len: u32 = 0;
        unsafe { GetCurrentPackageFullName(&mut len, ptr::null_mut()) != APPMODEL_ERROR_NO_PACKAGE }
    }

    /// 讀取目前登錄的啟動命令（未設定時回傳 `None`）
    pub fn read_command() -> Option<String> {
        let subkey = to_wide(SUBKEY);
        let val_name = to_wide(VALUE_NAME);
        unsafe {
            let mut hkey: Hkey = ptr::null_mut();
            if RegOpenKeyExW(HKEY_CURRENT_USER, subkey.as_ptr(), 0, KEY_READ, &mut hkey) != 0 {
                return None;
            }
            let mut val_type: u32 = 0;
            let mut cb: u32 = 0;
            let st = RegQueryValueExW(
                hkey,
                val_name.as_ptr(),
                ptr::null_mut(),
                &mut val_type,
                ptr::null_mut(),
                &mut cb,
            );
            let mut result = None;
            if st == 0 && (val_type == REG_SZ || val_type == REG_EXPAND_SZ) && cb > 2 {
                let mut buf = vec![0u16; (cb as usize).div_ceil(2) + 1];
                let mut cb2 = (buf.len() * 2) as u32;
                let st2 = RegQueryValueExW(
                    hkey,
                    val_name.as_ptr(),
                    ptr::null_mut(),
                    &mut val_type,
                    buf.as_mut_ptr() as *mut u8,
                    &mut cb2,
                );
                if st2 == 0 {
                    let n = (cb2 as usize / 2).min(buf.len());
                    let s = String::from_utf16_lossy(&buf[..n]);
                    result = Some(s.trim_end_matches('\0').to_string());
                }
            }
            RegCloseKey(hkey);
            result
        }
    }
}

fn autostart_command() -> Option<String> {
    std::env::current_exe()
        .ok()
        .map(|p| format!("\"{}\"", p.display()))
}

/// 目前執行環境是否支援以登錄檔設定開機自動啟動（MSIX 封裝版不支援）
pub fn is_autostart_supported() -> bool {
    !autostart_reg::is_packaged()
}

/// 查詢是否已啟用 Windows 開機自動啟動（讀取 HKCU\...\Run 中的 "Stock TW" 機碼）
pub fn is_windows_autostart_enabled() -> bool {
    autostart_reg::read_command().is_some()
}

/// 設定或取消 Windows 開機自動啟動；啟用時一律以目前執行檔路徑覆寫，
/// 以修正搬移安裝目錄或從開發路徑執行後遺留的失效路徑。回傳是否成功。
pub fn set_windows_autostart(enabled: bool) -> bool {
    use autostart_reg::*;
    let subkey = to_wide(SUBKEY);
    let val_name = to_wide(VALUE_NAME);
    unsafe {
        let mut hkey: Hkey = ptr::null_mut();
        let status = RegCreateKeyExW(
            HKEY_CURRENT_USER,
            subkey.as_ptr(),
            0,
            ptr::null(),
            0,
            KEY_SET_VALUE,
            ptr::null(),
            &mut hkey,
            ptr::null_mut(),
        );
        if status != 0 {
            return false;
        }
        let ok = if enabled {
            match autostart_command() {
                Some(cmd) => {
                    let data_w = to_wide(&cmd);
                    let byte_len = (data_w.len() * std::mem::size_of::<u16>()) as u32;
                    RegSetValueExW(
                        hkey,
                        val_name.as_ptr(),
                        0,
                        REG_SZ,
                        data_w.as_ptr() as *const u8,
                        byte_len,
                    ) == 0
                }
                None => false,
            }
        } else {
            let del = RegDeleteValueW(hkey, val_name.as_ptr());
            del == 0 || del == ERROR_FILE_NOT_FOUND
        };
        RegCloseKey(hkey);
        ok
    }
}

/// 設定或解除防止系統與螢幕進入休眠（透過 Windows `SetThreadExecutionState`）
pub fn set_prevent_sleep(prevent: bool) {
    const ES_CONTINUOUS: u32 = 0x8000_0000;
    const ES_SYSTEM_REQUIRED: u32 = 0x0000_0001;
    const ES_DISPLAY_REQUIRED: u32 = 0x0000_0002;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn SetThreadExecutionState(es_flags: u32) -> u32;
    }
    let flags = if prevent {
        ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED
    } else {
        ES_CONTINUOUS
    };
    unsafe {
        SetThreadExecutionState(flags);
    }
}

/// 將數值格式化為帶千分位逗號的字串
pub fn format_number(val: f64, decimals: usize) -> String {
    let neg = val < 0.0;
    let abs = val.abs();
    let formatted = format!("{abs:.decimals$}");
    let mut parts = formatted.split('.');
    let int_part = parts.next().unwrap_or("0");
    let dec_part = parts.next();

    let mut with_commas = String::with_capacity(int_part.len() + int_part.len() / 3);
    for (i, ch) in int_part.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            with_commas.push(',');
        }
        with_commas.push(ch);
    }
    let int_commas: String = with_commas.chars().rev().collect();

    let prefix = if neg { "-" } else { "" };
    match dec_part {
        Some(d) => format!("{prefix}{int_commas}.{d}"),
        None => format!("{prefix}{int_commas}"),
    }
}

/// 格式化持股數量（整數不顯示小數點，零股自動去除尾端多餘 0）
pub fn format_shares(shares: f64) -> String {
    if (shares - shares.round()).abs() < 1e-6 {
        format_number(shares, 0)
    } else {
        format!("{shares:.4}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}
