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
