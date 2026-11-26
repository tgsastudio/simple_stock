//! DPI 感知 + 深淺色主題原生 Win32 對話框模組
//! 包含三個與主視窗視覺一致的輕量原生對話框（零 OpenGL 開銷）：
//! 1. 編輯持股對話框（持有數量 / 持有成本 / 即時總成本與收益預覽）
//! 2. 新增自選股對話框（代碼輸入 / 市場切換 / 線上非同步搜尋 / 持股成本選填）
//! 3. 設定對話框（刷新頻率 / 省電開關 / 語系 / 配色 / 透明度 / 深淺色 / 置頂 / GitHub 檢查更新）

use crate::config::{AppConfig, ColorScheme, LanguageSetting, Market, RefreshMode, StockItem};
use crate::i18n::{Lang, resolve_lang};
use crate::quote::{SearchResult, guess_market, search_symbols};
use crate::updater::{UpdateCheckResult, check_for_updates};
use crate::win32_util::{
    apply_dark_title_bar, font_regular, font_semibold, format_number, format_shares,
    is_autostart_supported, is_windows_autostart_enabled, open_url_in_browser, rgb, set_windows_autostart, to_wide,
    virtual_screen_bounds,
};
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    CreateSolidBrush, DT_CENTER, DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, DeleteObject, DrawTextW,
    FillRect, HBRUSH, HDC, HFONT, InvalidateRect, SelectObject, SetBkColor, SetBkMode,
    SetTextColor, TRANSPARENT,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Controls::{DRAWITEMSTRUCT, ODS_SELECTED};
use windows_sys::Win32::UI::HiDpi::GetDpiForWindow;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{EnableWindow, SetFocus};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    BM_GETCHECK, BM_SETCHECK, BS_AUTOCHECKBOX, BS_DEFPUSHBUTTON, BS_OWNERDRAW, BS_PUSHBUTTON,
    CB_ADDSTRING, CB_GETCURSEL, CB_SETCURSEL, CBS_DROPDOWNLIST, CreateWindowExW, DefWindowProcW,
    DestroyWindow, DispatchMessageW, ES_AUTOHSCROLL, GWLP_USERDATA, GetClientRect, GetDlgItem,
    GetMessageW, GetWindowLongPtrW, GetWindowRect, GetWindowTextLengthW, GetWindowTextW, HMENU,
    IDC_ARROW, IsDialogMessageW, LB_ADDSTRING, LB_GETCURSEL, LB_RESETCONTENT, LBS_NOTIFY,
    LoadCursorW, MSG, PostMessageW, RegisterClassW, SW_HIDE, SW_SHOW, SendMessageW,
    MB_ICONWARNING, MB_OK, MessageBoxW, SetForegroundWindow, SetWindowLongPtrW, SetWindowTextW, ShowWindow, TranslateMessage, WM_APP,
    WM_CLOSE, WM_COMMAND, WM_CTLCOLORBTN, WM_CTLCOLORDLG, WM_CTLCOLOREDIT, WM_CTLCOLORLISTBOX,
    WM_CTLCOLORSTATIC, WM_DRAWITEM, WM_ERASEBKGND, WM_SETFONT, WNDCLASSW, WS_BORDER, WS_CAPTION,
    WS_CHILD, WS_EX_DLGMODALFRAME, WS_POPUP, WS_SYSMENU, WS_TABSTOP, WS_VISIBLE, WS_VSCROLL,
};

const WM_APP_DLG_ASYNC: u32 = WM_APP + 10;
const IDCANCEL: i32 = 2;
const BST_UNCHECKED: usize = 0;
const BST_CHECKED: usize = 1;
const EN_CHANGE: u16 = 0x0300;
const LBN_DBLCLK: u16 = 2;
const LBN_SELCHANGE: u16 = 1;
const SS_PATHELLIPSIS: u32 = 0x0000_8000;
const SS_NOPREFIX: u32 = 0x0000_0080;

/// 套用 Windows 10/11 原生深色控制項主題（按鈕、下拉選單與捲軸）
fn apply_ctrl_theme(ctrl: HWND, dark: bool, class: &str) {
    #[link(name = "uxtheme")]
    unsafe extern "system" {
        fn SetWindowTheme(
            hwnd: HWND,
            psz_sub_app_name: *const u16,
            psz_sub_id_list: *const u16,
        ) -> i32;
    }
    if ctrl.is_null() {
        return;
    }
    let theme_name = if dark {
        if class.eq_ignore_ascii_case("COMBOBOX") {
            "DarkMode_CFD"
        } else {
            "DarkMode_Explorer"
        }
    } else {
        "Explorer"
    };
    let w = to_wide(theme_name);
    unsafe {
        SetWindowTheme(ctrl, w.as_ptr(), ptr::null());
    }
}

/// 對話框深淺色主題畫刷快取
struct DlgTheme {
    bg_col: u32,
    edit_bg: u32,
    text_col: u32,
    bg_brush: HBRUSH,
    edit_brush: HBRUSH,
}

impl DlgTheme {
    fn new(dark: bool) -> Self {
        let (bg_col, edit_bg, text_col) = if dark {
            (rgb(24, 29, 39), rgb(35, 42, 56), rgb(232, 238, 246))
        } else {
            (rgb(246, 248, 252), rgb(255, 255, 255), rgb(24, 28, 36))
        };
        unsafe {
            Self {
                bg_col,
                edit_bg,
                text_col,
                bg_brush: CreateSolidBrush(bg_col),
                edit_brush: CreateSolidBrush(edit_bg),
            }
        }
    }

    fn handle_color_msg(&self, hwnd: HWND, msg: u32, wparam: WPARAM) -> Option<LRESULT> {
        unsafe {
            match msg {
                WM_ERASEBKGND => {
                    let hdc = wparam as HDC;
                    let mut rc: RECT = std::mem::zeroed();
                    GetClientRect(hwnd, &mut rc);
                    FillRect(hdc, &rc, self.bg_brush);
                    Some(1)
                }
                WM_CTLCOLORDLG | WM_CTLCOLORSTATIC | WM_CTLCOLORBTN => {
                    let hdc = wparam as HDC;
                    SetTextColor(hdc, self.text_col);
                    SetBkColor(hdc, self.bg_col);
                    SetBkMode(hdc, TRANSPARENT as i32);
                    Some(self.bg_brush as LRESULT)
                }
                WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX => {
                    let hdc = wparam as HDC;
                    SetTextColor(hdc, self.text_col);
                    SetBkColor(hdc, self.edit_bg);
                    Some(self.edit_brush as LRESULT)
                }
                _ => None,
            }
        }
    }

    fn destroy(&mut self) {
        unsafe {
            DeleteObject(self.bg_brush as _);
            DeleteObject(self.edit_brush as _);
        }
    }
}

fn dpi_of(parent: HWND) -> u32 {
    if parent.is_null() {
        96
    } else {
        unsafe { GetDpiForWindow(parent).max(96) }
    }
}

fn sc(dpi: u32, px: i32) -> i32 {
    ((px as f32) * (dpi as f32) / 96.0).round() as i32
}

fn get_ctrl_text(hwnd: HWND, id: i32) -> String {
    unsafe {
        let ctrl = GetDlgItem(hwnd, id);
        if ctrl.is_null() {
            return String::new();
        }
        let len = GetWindowTextLengthW(ctrl);
        if len <= 0 {
            return String::new();
        }
        let mut buf = vec![0u16; (len + 1) as usize];
        let copied = GetWindowTextW(ctrl, buf.as_mut_ptr(), buf.len() as i32);
        String::from_utf16_lossy(&buf[..copied.max(0) as usize])
    }
}

fn set_ctrl_text(hwnd: HWND, id: i32, text: &str) {
    unsafe {
        let ctrl = GetDlgItem(hwnd, id);
        if !ctrl.is_null() {
            let w = to_wide(text);
            SetWindowTextW(ctrl, w.as_ptr());
        }
    }
}

fn is_checked(hwnd: HWND, id: i32) -> bool {
    unsafe {
        let ctrl = GetDlgItem(hwnd, id);
        !ctrl.is_null() && SendMessageW(ctrl, BM_GETCHECK, 0, 0) as usize == BST_CHECKED
    }
}

fn set_checked(hwnd: HWND, id: i32, checked: bool) {
    unsafe {
        let ctrl = GetDlgItem(hwnd, id);
        if !ctrl.is_null() {
            SendMessageW(
                ctrl,
                BM_SETCHECK,
                if checked { BST_CHECKED } else { BST_UNCHECKED },
                0,
            );
        }
    }
}

fn create_ctrl(
    parent: HWND,
    dpi: u32,
    dark: bool,
    class: &str,
    text: &str,
    style: u32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    id: i32,
    font: HFONT,
) -> HWND {
    unsafe {
        let hinst = GetModuleHandleW(ptr::null());
        let wc = to_wide(class);
        let wt = to_wide(text);
        let ctrl = CreateWindowExW(
            0,
            wc.as_ptr(),
            wt.as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
            sc(dpi, x),
            sc(dpi, y),
            sc(dpi, w),
            sc(dpi, h),
            parent,
            id as isize as HMENU,
            hinst,
            ptr::null(),
        );
        if !ctrl.is_null() {
            if !font.is_null() {
                SendMessageW(ctrl, WM_SETFONT, font as usize, 1);
            }
            if class.eq_ignore_ascii_case("BUTTON")
                || class.eq_ignore_ascii_case("COMBOBOX")
                || class.eq_ignore_ascii_case("LISTBOX")
            {
                apply_ctrl_theme(ctrl, dark, class);
            }
        }
        ctrl
    }
}

fn center_pos(parent: HWND, dlg_w: i32, dlg_h: i32) -> (i32, i32) {
    unsafe {
        let (vx, vy, vw, vh) = virtual_screen_bounds();
        let mut r = RECT {
            left: vx + 100,
            top: vy + 100,
            right: vx + 580,
            bottom: vy + 600,
        };
        if !parent.is_null() {
            GetWindowRect(parent, &mut r);
        }
        let min_x = vx + 10;
        let max_x = (vx + vw - dlg_w - 10).max(min_x);
        let min_y = vy + 10;
        let max_y = (vy + vh - dlg_h - 10).max(min_y);
        let cx = (r.left + ((r.right - r.left) - dlg_w) / 2).clamp(min_x, max_x);
        let cy = (r.top + ((r.bottom - r.top) - dlg_h) / 2).clamp(min_y, max_y);
        (cx, cy)
    }
}

fn run_modal_loop(parent: HWND, dlg: HWND, done_flag: &AtomicBool) {
    unsafe {
        if !parent.is_null() {
            EnableWindow(parent, 0);
        }
        ShowWindow(dlg, SW_SHOW);
        SetForegroundWindow(dlg);

        let mut msg: MSG = std::mem::zeroed();
        while !done_flag.load(Ordering::Acquire) && GetMessageW(&mut msg, ptr::null_mut(), 0, 0) > 0
        {
            if IsDialogMessageW(dlg, &msg) == 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        if !parent.is_null() {
            EnableWindow(parent, 1);
            SetForegroundWindow(parent);
        }
        DestroyWindow(dlg);
    }
}

// ============================================================================
// 1. 編輯持股對話框（持有數量 / 持有成本 / 即時預覽）
// ============================================================================

const ID_EP_SHARES: i32 = 101;
const ID_EP_COST: i32 = 102;
const ID_EP_PREVIEW: i32 = 103;
const ID_EP_SAVE: i32 = 104;
const ID_EP_CLEAR: i32 = 105;
const ID_EP_CANCEL: i32 = 106;

struct EditPosState {
    item: StockItem,
    current_price: Option<f64>,
    lang: Lang,
    theme: DlgTheme,
    result: Option<(f64, f64)>,
    done: AtomicBool,
}

unsafe extern "system" fn edit_pos_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut EditPosState };
    if ptr.is_null() {
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }
    let st = unsafe { &mut *ptr };

    if let Some(res) = st.theme.handle_color_msg(hwnd, msg, wparam) {
        return res;
    }

    match msg {
        WM_COMMAND => {
            let id = (wparam & 0xFFFF) as i32;
            let code = ((wparam >> 16) & 0xFFFF) as u16;
            if (id == ID_EP_SHARES || id == ID_EP_COST) && code == EN_CHANGE {
                update_edit_pos_preview(hwnd, st);
            } else if id == ID_EP_SAVE {
                let shares = get_ctrl_text(hwnd, ID_EP_SHARES)
                    .trim()
                    .replace(',', "")
                    .parse::<f64>()
                    .unwrap_or(0.0)
                    .max(0.0);
                let cost = get_ctrl_text(hwnd, ID_EP_COST)
                    .trim()
                    .replace(',', "")
                    .parse::<f64>()
                    .unwrap_or(0.0)
                    .max(0.0);
                st.result = Some((shares, cost));
                st.done.store(true, Ordering::Release);
            } else if id == ID_EP_CLEAR {
                st.result = Some((0.0, 0.0));
                st.done.store(true, Ordering::Release);
            } else if id == ID_EP_CANCEL || id == IDCANCEL {
                st.done.store(true, Ordering::Release);
            }
            0
        }
        WM_CLOSE => {
            st.done.store(true, Ordering::Release);
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

fn update_edit_pos_preview(hwnd: HWND, st: &EditPosState) {
    let shares = get_ctrl_text(hwnd, ID_EP_SHARES)
        .trim()
        .replace(',', "")
        .parse::<f64>()
        .unwrap_or(0.0);
    let cost = get_ctrl_text(hwnd, ID_EP_COST)
        .trim()
        .replace(',', "")
        .parse::<f64>()
        .unwrap_or(0.0);
    if shares > 0.0 && cost > 0.0 {
        let dec = if st.item.market == Market::Tw { 0 } else { 2 };
        let basis = shares * cost;
        let mut text = format!(
            "{}{}",
            st.lang.label_cost_basis(),
            format_number(basis, dec)
        );
        if let Some(p) = st.current_price {
            let profit = shares * p - basis;
            let pct = (profit / basis) * 100.0;
            let sign = if profit > 0.0 { "+" } else { "" };
            text.push_str(&format!(
                "  |  {}: {sign}{} ({sign}{pct:.2}%)",
                st.lang.label_pnl(),
                format_number(profit, dec)
            ));
        }
        set_ctrl_text(hwnd, ID_EP_PREVIEW, &text);
    } else {
        set_ctrl_text(hwnd, ID_EP_PREVIEW, "");
    }
}

/// 開啟「編輯持股」對話框，回傳 `Some((shares, avg_cost))` 或 `None`
pub fn open_edit_position_dialog(
    parent: HWND,
    item: &StockItem,
    current_price: Option<f64>,
    cfg: &AppConfig,
) -> Option<(f64, f64)> {
    let lang = resolve_lang(cfg.language);
    let dpi = dpi_of(parent);
    let cls_name = to_wide("StockTW_EditPosDlg_v2");
    let hinst = unsafe { GetModuleHandleW(ptr::null()) };

    unsafe {
        let wc = WNDCLASSW {
            lpfnWndProc: Some(edit_pos_wndproc),
            hInstance: hinst,
            hCursor: LoadCursorW(ptr::null_mut(), IDC_ARROW),
            lpszClassName: cls_name.as_ptr(),
            ..std::mem::zeroed()
        };
        RegisterClassW(&wc);
    }

    let dlg_w = sc(dpi, 360);
    let dlg_h = sc(dpi, 200);
    let (px, py) = center_pos(parent, dlg_w, dlg_h);
    let title = to_wide(&format!(
        "{} - {} {}",
        lang.edit_pos_title(),
        item.symbol,
        item.name
    ));

    let dlg = unsafe {
        CreateWindowExW(
            WS_EX_DLGMODALFRAME,
            cls_name.as_ptr(),
            title.as_ptr(),
            WS_POPUP | WS_CAPTION | WS_SYSMENU,
            px,
            py,
            dlg_w,
            dlg_h,
            parent,
            ptr::null_mut(),
            hinst,
            ptr::null(),
        )
    };
    if dlg.is_null() {
        return None;
    }
    apply_dark_title_bar(dlg, cfg.dark_mode);

    let dark = cfg.dark_mode;
    let font = font_regular(sc(dpi, 13));
    let shares_init = if item.shares > 0.0 {
        format_shares(item.shares).replace(',', "")
    } else {
        String::new()
    };
    let cost_init = if item.avg_cost > 0.0 {
        format!("{:.2}", item.avg_cost)
    } else {
        String::new()
    };

    create_ctrl(dlg, dpi, dark, "STATIC", lang.label_shares(), SS_NOPREFIX, 18, 20, 92, 22, 0, font);
    let ed_shares = create_ctrl(
        dlg,
        dpi,
        dark,
        "EDIT",
        &shares_init,
        WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL as u32,
        114,
        17,
        214,
        24,
        ID_EP_SHARES,
        font,
    );

    create_ctrl(dlg, dpi, dark, "STATIC", lang.label_avg_cost(), SS_NOPREFIX, 18, 54, 92, 22, 0, font);
    create_ctrl(
        dlg,
        dpi,
        dark,
        "EDIT",
        &cost_init,
        WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL as u32,
        114,
        51,
        214,
        24,
        ID_EP_COST,
        font,
    );

    create_ctrl(dlg, dpi, dark, "STATIC", "", SS_NOPREFIX, 18, 86, 316, 24, ID_EP_PREVIEW, font);

    create_ctrl(
        dlg,
        dpi,
        dark,
        "BUTTON",
        lang.btn_save(),
        BS_DEFPUSHBUTTON as u32 | WS_TABSTOP,
        66,
        122,
        82,
        26,
        ID_EP_SAVE,
        font,
    );
    create_ctrl(
        dlg,
        dpi,
        dark,
        "BUTTON",
        lang.btn_clear_pos(),
        BS_PUSHBUTTON as u32 | WS_TABSTOP,
        156,
        122,
        82,
        26,
        ID_EP_CLEAR,
        font,
    );
    create_ctrl(
        dlg,
        dpi,
        dark,
        "BUTTON",
        lang.btn_cancel(),
        BS_PUSHBUTTON as u32 | WS_TABSTOP,
        246,
        122,
        82,
        26,
        ID_EP_CANCEL,
        font,
    );

    let mut state = Box::new(EditPosState {
        item: item.clone(),
        current_price,
        lang,
        theme: DlgTheme::new(cfg.dark_mode),
        result: None,
        done: AtomicBool::new(false),
    });
    update_edit_pos_preview(dlg, &state);

    unsafe {
        SetWindowLongPtrW(dlg, GWLP_USERDATA, (&mut *state as *mut EditPosState) as isize);
        SetFocus(ed_shares);
    }

    run_modal_loop(parent, dlg, &state.done);
    state.theme.destroy();
    unsafe {
        DeleteObject(font as _);
    }
    state.result
}

// ============================================================================
// 2. 新增自選股對話框（支援直接加入與背景線上搜尋）
// ============================================================================

const ID_ADD_SYM: i32 = 201;
const ID_ADD_SEARCH: i32 = 202;
const ID_ADD_MARKET: i32 = 203;
const ID_ADD_NAME: i32 = 204;
const ID_ADD_SHARES: i32 = 205;
const ID_ADD_COST: i32 = 206;
const ID_ADD_DIRECT: i32 = 207;
const ID_ADD_LIST: i32 = 208;
const ID_ADD_STATUS: i32 = 209;
const ID_ADD_CLOSE: i32 = 210;

struct AddStockState {
    lang: Lang,
    theme: DlgTheme,
    added: Vec<StockItem>,
    results: Vec<SearchResult>,
    search_rx: Option<Receiver<Result<Vec<SearchResult>, String>>>,
    done: AtomicBool,
}

unsafe extern "system" fn add_stock_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut AddStockState };
    if ptr.is_null() {
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }
    let st = unsafe { &mut *ptr };

    if let Some(res) = st.theme.handle_color_msg(hwnd, msg, wparam) {
        return res;
    }

    match msg {
        WM_APP_DLG_ASYNC => {
            if let Some(rx) = &st.search_rx {
                if let Ok(res) = rx.try_recv() {
                    st.search_rx = None;
                    let lb = unsafe { GetDlgItem(hwnd, ID_ADD_LIST) };
                    unsafe {
                        SendMessageW(lb, LB_RESETCONTENT, 0, 0);
                    }
                    match res {
                        Ok(list) => {
                            st.results = list;
                            for r in &st.results {
                                let line = format!(
                                    "[{}] {} - {} ({})",
                                    r.market.badge(),
                                    r.symbol,
                                    r.name,
                                    r.exchange
                                );
                                let w = to_wide(&line);
                                unsafe {
                                    SendMessageW(lb, LB_ADDSTRING, 0, w.as_ptr() as isize);
                                }
                            }
                            set_ctrl_text(
                                hwnd,
                                ID_ADD_STATUS,
                                if st.results.is_empty() {
                                    st.lang.search_no_results()
                                } else {
                                    st.lang.search_results_header()
                                },
                            );
                        }
                        Err(e) => {
                            set_ctrl_text(hwnd, ID_ADD_STATUS, &e);
                        }
                    }
                }
            }
            0
        }
        WM_COMMAND => {
            let id = (wparam & 0xFFFF) as i32;
            let code = ((wparam >> 16) & 0xFFFF) as u16;
            if id == ID_ADD_SYM && code == EN_CHANGE {
                let sym = get_ctrl_text(hwnd, ID_ADD_SYM);
                if !sym.trim().is_empty() {
                    let cb = unsafe { GetDlgItem(hwnd, ID_ADD_MARKET) };
                    let idx = if guess_market(&sym) == Market::Tw { 0 } else { 1 };
                    unsafe {
                        SendMessageW(cb, CB_SETCURSEL, idx, 0);
                    }
                }
            } else if id == ID_ADD_SEARCH {
                let query = get_ctrl_text(hwnd, ID_ADD_SYM).trim().to_string();
                if !query.is_empty() {
                    set_ctrl_text(hwnd, ID_ADD_STATUS, st.lang.searching());
                    let (tx, rx) = mpsc::channel();
                    st.search_rx = Some(rx);
                    let hwnd_num = hwnd as usize;
                    thread::spawn(move || {
                        let r = search_symbols(&query);
                        let _ = tx.send(r);
                        unsafe {
                            PostMessageW(hwnd_num as HWND, WM_APP_DLG_ASYNC, 0, 0);
                        }
                    });
                }
            } else if id == ID_ADD_DIRECT {
                let sym = get_ctrl_text(hwnd, ID_ADD_SYM).trim().to_uppercase();
                if !sym.is_empty() {
                    let cb = unsafe { GetDlgItem(hwnd, ID_ADD_MARKET) };
                    let mkt = if unsafe { SendMessageW(cb, CB_GETCURSEL, 0, 0) } == 0 {
                        Market::Tw
                    } else {
                        Market::Us
                    };
                    let name_in = get_ctrl_text(hwnd, ID_ADD_NAME).trim().to_string();
                    let name = if name_in.is_empty() { sym.clone() } else { name_in };
                    let shares = get_ctrl_text(hwnd, ID_ADD_SHARES)
                        .trim()
                        .replace(',', "")
                        .parse::<f64>()
                        .unwrap_or(0.0)
                        .max(0.0);
                    let avg_cost = get_ctrl_text(hwnd, ID_ADD_COST)
                        .trim()
                        .replace(',', "")
                        .parse::<f64>()
                        .unwrap_or(0.0)
                        .max(0.0);
                    st.added.push(StockItem {
                        symbol: sym,
                        name,
                        market: mkt,
                        shares,
                        avg_cost,
                    });
                    st.done.store(true, Ordering::Release);
                }
            } else if id == ID_ADD_LIST && (code == LBN_DBLCLK || code == LBN_SELCHANGE) {
                let lb = unsafe { GetDlgItem(hwnd, ID_ADD_LIST) };
                let sel = unsafe { SendMessageW(lb, LB_GETCURSEL, 0, 0) };
                if sel >= 0 && (sel as usize) < st.results.len() {
                    let r = &st.results[sel as usize];
                    if code == LBN_DBLCLK {
                        let shares = get_ctrl_text(hwnd, ID_ADD_SHARES)
                            .trim()
                            .replace(',', "")
                            .parse::<f64>()
                            .unwrap_or(0.0)
                            .max(0.0);
                        let avg_cost = get_ctrl_text(hwnd, ID_ADD_COST)
                            .trim()
                            .replace(',', "")
                            .parse::<f64>()
                            .unwrap_or(0.0)
                            .max(0.0);
                        st.added.push(StockItem {
                            symbol: r.symbol.clone(),
                            name: r.name.clone(),
                            market: r.market,
                            shares,
                            avg_cost,
                        });
                        st.done.store(true, Ordering::Release);
                    } else {
                        set_ctrl_text(hwnd, ID_ADD_SYM, &r.symbol);
                        set_ctrl_text(hwnd, ID_ADD_NAME, &r.name);
                        let cb = unsafe { GetDlgItem(hwnd, ID_ADD_MARKET) };
                        let idx = if r.market == Market::Tw { 0 } else { 1 };
                        unsafe {
                            SendMessageW(cb, CB_SETCURSEL, idx, 0);
                        }
                    }
                }
            } else if id == ID_ADD_CLOSE || id == IDCANCEL {
                st.done.store(true, Ordering::Release);
            }
            0
        }
        WM_CLOSE => {
            st.done.store(true, Ordering::Release);
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

/// 開啟「新增自選股」對話框
pub fn open_add_stock_dialog(parent: HWND, cfg: &AppConfig) -> Vec<StockItem> {
    let lang = resolve_lang(cfg.language);
    let dpi = dpi_of(parent);
    let cls_name = to_wide("StockTW_AddStockDlg_v2");
    let hinst = unsafe { GetModuleHandleW(ptr::null()) };

    unsafe {
        let wc = WNDCLASSW {
            lpfnWndProc: Some(add_stock_wndproc),
            hInstance: hinst,
            hCursor: LoadCursorW(ptr::null_mut(), IDC_ARROW),
            lpszClassName: cls_name.as_ptr(),
            ..std::mem::zeroed()
        };
        RegisterClassW(&wc);
    }

    let dlg_w = sc(dpi, 456);
    let dlg_h = sc(dpi, 410);
    let (px, py) = center_pos(parent, dlg_w, dlg_h);
    let title = to_wide(lang.add_title());

    let dlg = unsafe {
        CreateWindowExW(
            WS_EX_DLGMODALFRAME,
            cls_name.as_ptr(),
            title.as_ptr(),
            WS_POPUP | WS_CAPTION | WS_SYSMENU,
            px,
            py,
            dlg_w,
            dlg_h,
            parent,
            ptr::null_mut(),
            hinst,
            ptr::null(),
        )
    };
    if dlg.is_null() {
        return Vec::new();
    }
    apply_dark_title_bar(dlg, cfg.dark_mode);
    let dark = cfg.dark_mode;
    let font = font_regular(sc(dpi, 13));
    let small_font = font_regular(sc(dpi, 12));

    // 頂部輸入提示
    create_ctrl(dlg, dpi, dark, "STATIC", lang.add_prompt(), SS_NOPREFIX, 18, 14, 408, 20, 0, small_font);

    // 代碼輸入 + 線上搜尋按鈕
    let ed_sym = create_ctrl(
        dlg,
        dpi,
        dark,
        "EDIT",
        "",
        WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL as u32,
        18,
        38,
        306,
        25,
        ID_ADD_SYM,
        font,
    );
    create_ctrl(
        dlg,
        dpi,
        dark,
        "BUTTON",
        lang.btn_search(),
        BS_PUSHBUTTON as u32 | WS_TABSTOP,
        332,
        37,
        92,
        27,
        ID_ADD_SEARCH,
        font,
    );

    // 市場選擇（與下方欄位統一對齊 x=112）
    create_ctrl(dlg, dpi, dark, "STATIC", lang.label_market(), SS_NOPREFIX, 18, 78, 90, 22, 0, font);
    let cb_mkt = create_ctrl(
        dlg,
        dpi,
        dark,
        "COMBOBOX",
        "",
        CBS_DROPDOWNLIST as u32 | WS_TABSTOP | WS_VSCROLL,
        112,
        75,
        136,
        120,
        ID_ADD_MARKET,
        font,
    );
    unsafe {
        let w1 = to_wide(lang.market_tw());
        let w2 = to_wide(lang.market_us());
        SendMessageW(cb_mkt, CB_ADDSTRING, 0, w1.as_ptr() as isize);
        SendMessageW(cb_mkt, CB_ADDSTRING, 0, w2.as_ptr() as isize);
        SendMessageW(cb_mkt, CB_SETCURSEL, 0, 0);
    }

    // 自訂名稱（選填）
    create_ctrl(
        dlg,
        dpi,
        dark,
        "STATIC",
        lang.label_name_short(),
        SS_NOPREFIX,
        18,
        112,
        90,
        22,
        0,
        font,
    );
    create_ctrl(
        dlg,
        dpi,
        dark,
        "EDIT",
        "",
        WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL as u32,
        112,
        109,
        312,
        25,
        ID_ADD_NAME,
        font,
    );

    // 持有數量與持有成本（選填，整齊雙欄對齊）
    create_ctrl(dlg, dpi, dark, "STATIC", lang.label_shares(), SS_NOPREFIX, 18, 146, 90, 22, 0, font);
    create_ctrl(
        dlg,
        dpi,
        dark,
        "EDIT",
        "",
        WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL as u32,
        112,
        143,
        104,
        25,
        ID_ADD_SHARES,
        font,
    );
    create_ctrl(dlg, dpi, dark, "STATIC", lang.label_avg_cost(), SS_NOPREFIX, 230, 146, 82, 22, 0, font);
    create_ctrl(
        dlg,
        dpi,
        dark,
        "EDIT",
        "",
        WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL as u32,
        316,
        143,
        108,
        25,
        ID_ADD_COST,
        font,
    );

    // 線上搜尋結果列表（加大高度，雙擊直接加入）
    create_ctrl(dlg, dpi, dark, "STATIC", lang.search_results_header(), SS_NOPREFIX, 18, 182, 406, 20, ID_ADD_STATUS, small_font);
    create_ctrl(
        dlg,
        dpi,
        dark,
        "LISTBOX",
        "",
        WS_BORDER | WS_VSCROLL | WS_TABSTOP | LBS_NOTIFY as u32,
        18,
        204,
        406,
        112,
        ID_ADD_LIST,
        font,
    );

    // 底部操作按鈕（右對齊精巧比例）
    create_ctrl(
        dlg,
        dpi,
        dark,
        "BUTTON",
        lang.btn_add_direct(),
        BS_DEFPUSHBUTTON as u32 | WS_TABSTOP,
        214,
        330,
        118,
        27,
        ID_ADD_DIRECT,
        font,
    );
    create_ctrl(
        dlg,
        dpi,
        dark,
        "BUTTON",
        lang.btn_close(),
        BS_PUSHBUTTON as u32 | WS_TABSTOP,
        340,
        330,
        84,
        27,
        ID_ADD_CLOSE,
        font,
    );

    let mut state = Box::new(AddStockState {
        lang,
        theme: DlgTheme::new(cfg.dark_mode),
        added: Vec::new(),
        results: Vec::new(),
        search_rx: None,
        done: AtomicBool::new(false),
    });

    unsafe {
        SetWindowLongPtrW(dlg, GWLP_USERDATA, (&mut *state as *mut AddStockState) as isize);
        SetFocus(ed_sym);
    }

    run_modal_loop(parent, dlg, &state.done);
    state.theme.destroy();
    unsafe {
        DeleteObject(font as _);
        DeleteObject(small_font as _);
    }
    state.added
}

// ============================================================================
// 3. 設定對話框（分頁介面：一般 / 外觀 / 刷新 / 關於 + Windows 自動啟動）
// ============================================================================

const ID_SET_REFRESH: i32 = 301;
const ID_SET_SMART_HRS: i32 = 302;
const ID_SET_PAUSE_MIN: i32 = 303;
const ID_SET_US_EXT: i32 = 304;
const ID_SET_LANG: i32 = 305;
const ID_SET_COLOR: i32 = 306;
const ID_SET_OPACITY: i32 = 307;
const ID_SET_DARK: i32 = 308;
const ID_SET_TOP: i32 = 309;
const ID_SET_CHECK_UPD: i32 = 310;
const ID_SET_UPD_STATUS: i32 = 311;
const ID_SET_DOWNLOAD: i32 = 312;
const ID_SET_SAVE: i32 = 313;
const ID_SET_CANCEL: i32 = 314;
const ID_SET_AUTOSTART: i32 = 315;
const ID_SET_TAB_GENERAL: i32 = 320;
const ID_SET_TAB_APPEARANCE: i32 = 321;
const ID_SET_TAB_REFRESH: i32 = 322;
const ID_SET_TAB_ABOUT: i32 = 323;

const OPACITY_STEPS: [f32; 8] = [1.0, 0.92, 0.85, 0.75, 0.65, 0.50, 0.35, 0.25];

struct SettingsState {
    cfg: AppConfig,
    lang: Lang,
    dark: bool,
    dpi: u32,
    theme: DlgTheme,
    tab_font: HFONT,
    tab_font_active: HFONT,
    active_tab: usize,
    tab_btns: [HWND; 4],
    tab_ctrls: [Vec<HWND>; 4],
    initial_autostart: bool,
    autostart_supported: bool,
    saved: bool,
    done: AtomicBool,
    update_rx: Option<Receiver<Result<UpdateCheckResult, String>>>,
    download_url: Option<String>,
}

fn switch_settings_tab(st: &mut SettingsState, tab: usize) {
    if tab >= 4 {
        return;
    }
    st.active_tab = tab;
    unsafe {
        for (i, ctrls) in st.tab_ctrls.iter().enumerate() {
            let cmd = if i == tab { SW_SHOW } else { SW_HIDE };
            for &h in ctrls {
                if !h.is_null() {
                    ShowWindow(h, cmd);
                }
            }
        }
        for &tb in &st.tab_btns {
            if !tb.is_null() {
                InvalidateRect(tb, ptr::null(), 0);
            }
        }
    }
}

fn draw_settings_tab(st: &SettingsState, dis: &DRAWITEMSTRUCT, tab_idx: usize) {
    let is_active = tab_idx == st.active_tab;
    let is_pressed = (dis.itemState & ODS_SELECTED) != 0;
    let (bg_col, txt_col, accent_col) = if st.dark {
        (
            if is_active {
                rgb(38, 46, 62)
            } else if is_pressed {
                rgb(32, 39, 52)
            } else {
                rgb(24, 29, 39)
            },
            if is_active {
                rgb(238, 244, 255)
            } else {
                rgb(154, 166, 188)
            },
            rgb(78, 161, 255),
        )
    } else {
        (
            if is_active {
                rgb(255, 255, 255)
            } else if is_pressed {
                rgb(234, 240, 249)
            } else {
                rgb(246, 248, 252)
            },
            if is_active {
                rgb(22, 92, 192)
            } else {
                rgb(94, 104, 122)
            },
            rgb(28, 110, 214),
        )
    };

    let label = match tab_idx {
        0 => st.lang.tab_general(),
        1 => st.lang.tab_appearance(),
        2 => st.lang.tab_refresh(),
        _ => st.lang.tab_about(),
    };

    unsafe {
        let hdc = dis.hDC;
        let rc = dis.rcItem;
        let bg_brush = CreateSolidBrush(bg_col);
        FillRect(hdc, &rc, bg_brush);
        DeleteObject(bg_brush as _);

        if is_active {
            let bar_h = sc(st.dpi, 2).max(2);
            let bar_rc = RECT {
                left: rc.left + sc(st.dpi, 6),
                top: rc.bottom - bar_h,
                right: rc.right - sc(st.dpi, 6),
                bottom: rc.bottom,
            };
            let accent_brush = CreateSolidBrush(accent_col);
            FillRect(hdc, &bar_rc, accent_brush);
            DeleteObject(accent_brush as _);
        }

        let font = if is_active {
            st.tab_font_active
        } else {
            st.tab_font
        };
        let old_font = SelectObject(hdc, font as _);
        SetBkMode(hdc, TRANSPARENT as i32);
        SetTextColor(hdc, txt_col);
        let mut txt_rc = rc;
        let w = to_wide(label);
        DrawTextW(
            hdc,
            w.as_ptr(),
            (w.len() as i32) - 1,
            &mut txt_rc,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );
        SelectObject(hdc, old_font);
    }
}

unsafe extern "system" fn settings_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SettingsState };
    if ptr.is_null() {
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }
    let st = unsafe { &mut *ptr };

    if msg == WM_ERASEBKGND {
        let hdc = wparam as HDC;
        unsafe {
            let mut rc: RECT = std::mem::zeroed();
            GetClientRect(hwnd, &mut rc);
            FillRect(hdc, &rc, st.theme.bg_brush);
            let div_col = if st.dark {
                rgb(46, 54, 72)
            } else {
                rgb(216, 222, 232)
            };
            let div_brush = CreateSolidBrush(div_col);
            let div_h = sc(st.dpi, 1).max(1);
            let top_div = RECT {
                left: sc(st.dpi, 16),
                top: sc(st.dpi, 42),
                right: rc.right - sc(st.dpi, 16),
                bottom: sc(st.dpi, 42) + div_h,
            };
            FillRect(hdc, &top_div, div_brush);
            let bot_div = RECT {
                left: sc(st.dpi, 16),
                top: sc(st.dpi, 194),
                right: rc.right - sc(st.dpi, 16),
                bottom: sc(st.dpi, 194) + div_h,
            };
            FillRect(hdc, &bot_div, div_brush);
            DeleteObject(div_brush as _);
        }
        return 1;
    }

    if let Some(res) = st.theme.handle_color_msg(hwnd, msg, wparam) {
        return res;
    }

    match msg {
        WM_DRAWITEM => {
            if lparam != 0 {
                let dis = unsafe { &*(lparam as *const DRAWITEMSTRUCT) };
                let id = dis.CtlID as i32;
                if (ID_SET_TAB_GENERAL..=ID_SET_TAB_ABOUT).contains(&id) {
                    draw_settings_tab(st, dis, (id - ID_SET_TAB_GENERAL) as usize);
                    return 1;
                }
            }
            0
        }
        WM_APP_DLG_ASYNC => {
            if let Some(rx) = &st.update_rx {
                if let Ok(res) = rx.try_recv() {
                    st.update_rx = None;
                    let btn_dl = unsafe { GetDlgItem(hwnd, ID_SET_DOWNLOAD) };
                    match res {
                        Ok(info) if info.has_update => {
                            set_ctrl_text(
                                hwnd,
                                ID_SET_UPD_STATUS,
                                &st.lang.update_available(&info.latest_version),
                            );
                            st.download_url = Some(info.download_url);
                            unsafe {
                                EnableWindow(btn_dl, 1);
                            }
                        }
                        Ok(info) => {
                            set_ctrl_text(
                                hwnd,
                                ID_SET_UPD_STATUS,
                                &st.lang.update_up_to_date(&info.latest_version),
                            );
                        }
                        Err(e) => {
                            set_ctrl_text(hwnd, ID_SET_UPD_STATUS, &st.lang.update_failed(&e));
                        }
                    }
                }
            }
            0
        }
        WM_COMMAND => {
            let id = (wparam & 0xFFFF) as i32;
            if (ID_SET_TAB_GENERAL..=ID_SET_TAB_ABOUT).contains(&id) {
                switch_settings_tab(st, (id - ID_SET_TAB_GENERAL) as usize);
            } else if id == ID_SET_CHECK_UPD {
                if st.update_rx.is_none() {
                    set_ctrl_text(hwnd, ID_SET_UPD_STATUS, st.lang.update_checking());
                    let (tx, rx) = mpsc::channel();
                    st.update_rx = Some(rx);
                    let hwnd_num = hwnd as usize;
                    thread::spawn(move || {
                        let res = check_for_updates(env!("CARGO_PKG_VERSION"));
                        let _ = tx.send(res);
                        unsafe {
                            PostMessageW(hwnd_num as HWND, WM_APP_DLG_ASYNC, 0, 0);
                        }
                    });
                }
            } else if id == ID_SET_DOWNLOAD {
                if let Some(url) = &st.download_url {
                    open_url_in_browser(url);
                }
            } else if id == ID_SET_SAVE {
                let modes = RefreshMode::all();
                let r_idx = unsafe {
                    SendMessageW(GetDlgItem(hwnd, ID_SET_REFRESH), CB_GETCURSEL, 0, 0)
                } as usize;
                if r_idx < modes.len() {
                    st.cfg.refresh_mode = modes[r_idx];
                }
                st.cfg.smart_market_hours = is_checked(hwnd, ID_SET_SMART_HRS);
                st.cfg.pause_when_minimized = is_checked(hwnd, ID_SET_PAUSE_MIN);
                st.cfg.include_us_extended = is_checked(hwnd, ID_SET_US_EXT);

                let l_idx =
                    unsafe { SendMessageW(GetDlgItem(hwnd, ID_SET_LANG), CB_GETCURSEL, 0, 0) };
                st.cfg.language = match l_idx {
                    1 => LanguageSetting::ZhTw,
                    2 => LanguageSetting::ZhCn,
                    3 => LanguageSetting::En,
                    4 => LanguageSetting::Ja,
                    5 => LanguageSetting::Ko,
                    6 => LanguageSetting::Es,
                    _ => LanguageSetting::Auto,
                };

                let c_idx =
                    unsafe { SendMessageW(GetDlgItem(hwnd, ID_SET_COLOR), CB_GETCURSEL, 0, 0) };
                st.cfg.color_scheme = match c_idx {
                    1 => ColorScheme::UsGreenUp,
                    2 => ColorScheme::AutoByMarket,
                    _ => ColorScheme::TwRedUp,
                };

                let o_idx = unsafe {
                    SendMessageW(GetDlgItem(hwnd, ID_SET_OPACITY), CB_GETCURSEL, 0, 0)
                } as usize;
                if o_idx < OPACITY_STEPS.len() {
                    st.cfg.opacity = OPACITY_STEPS[o_idx];
                }

                st.cfg.dark_mode = is_checked(hwnd, ID_SET_DARK);
                st.cfg.always_on_top = is_checked(hwnd, ID_SET_TOP);

                // 啟用時一律重寫（修正失效路徑）；停用時僅在原本已啟用才移除
                if st.autostart_supported {
                    let new_autostart = is_checked(hwnd, ID_SET_AUTOSTART);
                    if (new_autostart || st.initial_autostart != new_autostart)
                        && !set_windows_autostart(new_autostart)
                    {
                        let msg = to_wide(st.lang.autostart_failed());
                        let ttl = to_wide(st.lang.settings_title());
                        unsafe {
                            MessageBoxW(hwnd, msg.as_ptr(), ttl.as_ptr(), MB_OK | MB_ICONWARNING);
                        }
                    }
                }

                st.saved = true;
                st.done.store(true, Ordering::Release);
            } else if id == ID_SET_CANCEL || id == IDCANCEL {
                st.done.store(true, Ordering::Release);
            }
            0
        }
        WM_CLOSE => {
            st.done.store(true, Ordering::Release);
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

/// 開啟「設定」對話框（四個分頁：一般 / 外觀 / 刷新 / 關於）
pub fn open_settings_dialog(parent: HWND, cfg: &AppConfig) -> Option<AppConfig> {
    let lang = resolve_lang(cfg.language);
    let dpi = dpi_of(parent);
    let cls_name = to_wide("StockTW_SettingsDlg_v2");
    let hinst = unsafe { GetModuleHandleW(ptr::null()) };

    unsafe {
        let wc = WNDCLASSW {
            lpfnWndProc: Some(settings_wndproc),
            hInstance: hinst,
            hCursor: LoadCursorW(ptr::null_mut(), IDC_ARROW),
            lpszClassName: cls_name.as_ptr(),
            ..std::mem::zeroed()
        };
        RegisterClassW(&wc);
    }

    let dlg_w = sc(dpi, 460);
    let dlg_h = sc(dpi, 286);
    let (px, py) = center_pos(parent, dlg_w, dlg_h);
    let title = to_wide(lang.settings_title());

    let dlg = unsafe {
        CreateWindowExW(
            WS_EX_DLGMODALFRAME,
            cls_name.as_ptr(),
            title.as_ptr(),
            WS_POPUP | WS_CAPTION | WS_SYSMENU,
            px,
            py,
            dlg_w,
            dlg_h,
            parent,
            ptr::null_mut(),
            hinst,
            ptr::null(),
        )
    };
    if dlg.is_null() {
        return None;
    }
    apply_dark_title_bar(dlg, cfg.dark_mode);
    let dark = cfg.dark_mode;
    let font = font_regular(sc(dpi, 13));
    let font_semi = font_semibold(sc(dpi, 13));
    let small_font = font_regular(sc(dpi, 12));

    // 頂部分頁列（一般 / 外觀 / 刷新 / 關於）
    let tab_labels = [
        lang.tab_general(),
        lang.tab_appearance(),
        lang.tab_refresh(),
        lang.tab_about(),
    ];
    let tab_ids = [
        ID_SET_TAB_GENERAL,
        ID_SET_TAB_APPEARANCE,
        ID_SET_TAB_REFRESH,
        ID_SET_TAB_ABOUT,
    ];
    let mut tab_btns = [ptr::null_mut(); 4];
    for i in 0..4 {
        tab_btns[i] = create_ctrl(
            dlg,
            dpi,
            dark,
            "BUTTON",
            tab_labels[i],
            BS_OWNERDRAW as u32 | WS_TABSTOP,
            16 + (i as i32) * 104,
            10,
            98,
            28,
            tab_ids[i],
            font,
        );
    }

    let mut tab_ctrls: [Vec<HWND>; 4] = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];

    // --- Tab 0: 一般（語系、視窗置頂、開機自動啟動）---
    tab_ctrls[0].push(create_ctrl(
        dlg,
        dpi,
        dark,
        "STATIC",
        lang.label_language(),
        SS_NOPREFIX,
        18,
        61,
        150,
        22,
        0,
        font,
    ));
    let cb_lang = create_ctrl(
        dlg,
        dpi,
        dark,
        "COMBOBOX",
        "",
        CBS_DROPDOWNLIST as u32 | WS_TABSTOP | WS_VSCROLL,
        178,
        58,
        248,
        200,
        ID_SET_LANG,
        font,
    );
    tab_ctrls[0].push(cb_lang);
    for ls in [
        LanguageSetting::Auto,
        LanguageSetting::ZhTw,
        LanguageSetting::ZhCn,
        LanguageSetting::En,
        LanguageSetting::Ja,
        LanguageSetting::Ko,
        LanguageSetting::Es,
    ] {
        let w = to_wide(lang.lang_setting_label(ls));
        unsafe {
            SendMessageW(cb_lang, CB_ADDSTRING, 0, w.as_ptr() as isize);
        }
    }
    let cur_lang = match cfg.language {
        LanguageSetting::Auto => 0,
        LanguageSetting::ZhTw => 1,
        LanguageSetting::ZhCn => 2,
        LanguageSetting::En => 3,
        LanguageSetting::Ja => 4,
        LanguageSetting::Ko => 5,
        LanguageSetting::Es => 6,
    };
    unsafe {
        SendMessageW(cb_lang, CB_SETCURSEL, cur_lang, 0);
    }

    tab_ctrls[0].push(create_ctrl(
        dlg,
        dpi,
        dark,
        "BUTTON",
        lang.chk_always_on_top(),
        BS_AUTOCHECKBOX as u32 | WS_TABSTOP,
        18,
        98,
        408,
        24,
        ID_SET_TOP,
        font,
    ));
    set_checked(dlg, ID_SET_TOP, cfg.always_on_top);

    let autostart_supported = is_autostart_supported();
    let initial_autostart = autostart_supported && is_windows_autostart_enabled();
    let chk_autostart = create_ctrl(
        dlg,
        dpi,
        dark,
        "BUTTON",
        if autostart_supported {
            lang.chk_autostart()
        } else {
            lang.autostart_unsupported()
        },
        BS_AUTOCHECKBOX as u32 | WS_TABSTOP,
        18,
        130,
        408,
        24,
        ID_SET_AUTOSTART,
        font,
    );
    tab_ctrls[0].push(chk_autostart);
    set_checked(dlg, ID_SET_AUTOSTART, initial_autostart);
    if !autostart_supported {
        unsafe {
            EnableWindow(chk_autostart, 0);
        }
    }

    // --- Tab 1: 外觀（漲跌配色、浮動透明度、深色主題）---
    tab_ctrls[1].push(create_ctrl(
        dlg,
        dpi,
        dark,
        "STATIC",
        lang.label_color_scheme(),
        SS_NOPREFIX,
        18,
        61,
        150,
        22,
        0,
        font,
    ));
    let cb_col = create_ctrl(
        dlg,
        dpi,
        dark,
        "COMBOBOX",
        "",
        CBS_DROPDOWNLIST as u32 | WS_TABSTOP | WS_VSCROLL,
        178,
        58,
        248,
        120,
        ID_SET_COLOR,
        font,
    );
    tab_ctrls[1].push(cb_col);
    for cs in [
        ColorScheme::TwRedUp,
        ColorScheme::UsGreenUp,
        ColorScheme::AutoByMarket,
    ] {
        let w = to_wide(lang.color_scheme_label(cs));
        unsafe {
            SendMessageW(cb_col, CB_ADDSTRING, 0, w.as_ptr() as isize);
        }
    }
    let cur_col = match cfg.color_scheme {
        ColorScheme::TwRedUp => 0,
        ColorScheme::UsGreenUp => 1,
        ColorScheme::AutoByMarket => 2,
    };
    unsafe {
        SendMessageW(cb_col, CB_SETCURSEL, cur_col, 0);
    }

    tab_ctrls[1].push(create_ctrl(
        dlg,
        dpi,
        dark,
        "STATIC",
        lang.slider_opacity(),
        SS_NOPREFIX,
        18,
        97,
        150,
        22,
        0,
        font,
    ));
    let cb_op = create_ctrl(
        dlg,
        dpi,
        dark,
        "COMBOBOX",
        "",
        CBS_DROPDOWNLIST as u32 | WS_TABSTOP | WS_VSCROLL,
        178,
        94,
        248,
        160,
        ID_SET_OPACITY,
        font,
    );
    tab_ctrls[1].push(cb_op);
    let mut best_op_idx = 0usize;
    let mut best_diff = f32::MAX;
    for (i, &val) in OPACITY_STEPS.iter().enumerate() {
        let label = format!("{}%", (val * 100.0).round() as i32);
        let w = to_wide(&label);
        unsafe {
            SendMessageW(cb_op, CB_ADDSTRING, 0, w.as_ptr() as isize);
        }
        let d = (val - cfg.opacity).abs();
        if d < best_diff {
            best_diff = d;
            best_op_idx = i;
        }
    }
    unsafe {
        SendMessageW(cb_op, CB_SETCURSEL, best_op_idx, 0);
    }

    tab_ctrls[1].push(create_ctrl(
        dlg,
        dpi,
        dark,
        "BUTTON",
        lang.chk_dark(),
        BS_AUTOCHECKBOX as u32 | WS_TABSTOP,
        18,
        134,
        408,
        24,
        ID_SET_DARK,
        font,
    ));
    set_checked(dlg, ID_SET_DARK, cfg.dark_mode);

    // --- Tab 2: 刷新（刷新頻率、開盤感知、最小化暫停、美股盤前盤後）---
    tab_ctrls[2].push(create_ctrl(
        dlg,
        dpi,
        dark,
        "STATIC",
        lang.sec_refresh(),
        SS_NOPREFIX,
        18,
        61,
        150,
        22,
        0,
        font,
    ));
    let cb_ref = create_ctrl(
        dlg,
        dpi,
        dark,
        "COMBOBOX",
        "",
        CBS_DROPDOWNLIST as u32 | WS_TABSTOP | WS_VSCROLL,
        178,
        58,
        248,
        180,
        ID_SET_REFRESH,
        font,
    );
    tab_ctrls[2].push(cb_ref);
    let modes = RefreshMode::all();
    let mut cur_ref = 1usize;
    for (i, m) in modes.iter().enumerate() {
        let w = to_wide(lang.refresh_label(*m));
        unsafe {
            SendMessageW(cb_ref, CB_ADDSTRING, 0, w.as_ptr() as isize);
        }
        if *m == cfg.refresh_mode {
            cur_ref = i;
        }
    }
    unsafe {
        SendMessageW(cb_ref, CB_SETCURSEL, cur_ref, 0);
    }

    tab_ctrls[2].push(create_ctrl(
        dlg,
        dpi,
        dark,
        "BUTTON",
        lang.chk_smart_hours(),
        BS_AUTOCHECKBOX as u32 | WS_TABSTOP,
        18,
        94,
        408,
        24,
        ID_SET_SMART_HRS,
        font,
    ));
    set_checked(dlg, ID_SET_SMART_HRS, cfg.smart_market_hours);

    tab_ctrls[2].push(create_ctrl(
        dlg,
        dpi,
        dark,
        "BUTTON",
        lang.chk_pause_min(),
        BS_AUTOCHECKBOX as u32 | WS_TABSTOP,
        18,
        122,
        408,
        24,
        ID_SET_PAUSE_MIN,
        font,
    ));
    set_checked(dlg, ID_SET_PAUSE_MIN, cfg.pause_when_minimized);

    tab_ctrls[2].push(create_ctrl(
        dlg,
        dpi,
        dark,
        "BUTTON",
        lang.chk_us_ext(),
        BS_AUTOCHECKBOX as u32 | WS_TABSTOP,
        18,
        150,
        408,
        24,
        ID_SET_US_EXT,
        font,
    ));
    set_checked(dlg, ID_SET_US_EXT, cfg.include_us_extended);

    // --- Tab 3: 關於（版本、檢查更新、DPAPI 加密提示、設定檔路徑）---
    let ver_str = format!("Stock TW v{}", env!("CARGO_PKG_VERSION"));
    tab_ctrls[3].push(create_ctrl(
        dlg,
        dpi,
        dark,
        "STATIC",
        &ver_str,
        SS_NOPREFIX,
        18,
        61,
        140,
        22,
        0,
        font,
    ));
    tab_ctrls[3].push(create_ctrl(
        dlg,
        dpi,
        dark,
        "BUTTON",
        lang.btn_check_update(),
        BS_PUSHBUTTON as u32 | WS_TABSTOP,
        234,
        58,
        92,
        26,
        ID_SET_CHECK_UPD,
        font,
    ));
    let btn_dl = create_ctrl(
        dlg,
        dpi,
        dark,
        "BUTTON",
        lang.btn_download_update(),
        BS_PUSHBUTTON as u32 | WS_TABSTOP,
        334,
        58,
        92,
        26,
        ID_SET_DOWNLOAD,
        font,
    );
    unsafe {
        EnableWindow(btn_dl, 0);
    }
    tab_ctrls[3].push(btn_dl);
    tab_ctrls[3].push(create_ctrl(
        dlg,
        dpi,
        dark,
        "STATIC",
        "",
        SS_NOPREFIX,
        18,
        92,
        408,
        22,
        ID_SET_UPD_STATUS,
        small_font,
    ));
    tab_ctrls[3].push(create_ctrl(
        dlg,
        dpi,
        dark,
        "STATIC",
        lang.dpapi_encrypted_note(),
        SS_NOPREFIX,
        18,
        118,
        408,
        34,
        0,
        small_font,
    ));
    let cfg_path_str = format!(
        "{}{}",
        lang.config_file_label(),
        AppConfig::config_path().display()
    );
    tab_ctrls[3].push(create_ctrl(
        dlg,
        dpi,
        dark,
        "STATIC",
        &cfg_path_str,
        SS_PATHELLIPSIS | SS_NOPREFIX,
        18,
        156,
        408,
        22,
        0,
        small_font,
    ));

    // 底部操作按鈕（跨分頁固定顯示）
    create_ctrl(
        dlg,
        dpi,
        dark,
        "BUTTON",
        lang.btn_save(),
        BS_DEFPUSHBUTTON as u32 | WS_TABSTOP,
        250,
        206,
        84,
        26,
        ID_SET_SAVE,
        font,
    );
    create_ctrl(
        dlg,
        dpi,
        dark,
        "BUTTON",
        lang.btn_cancel(),
        BS_PUSHBUTTON as u32 | WS_TABSTOP,
        342,
        206,
        84,
        26,
        ID_SET_CANCEL,
        font,
    );

    let mut state = Box::new(SettingsState {
        cfg: cfg.clone(),
        lang,
        dark,
        dpi,
        theme: DlgTheme::new(cfg.dark_mode),
        tab_font: font,
        tab_font_active: font_semi,
        active_tab: 0,
        tab_btns,
        tab_ctrls,
        initial_autostart,
        autostart_supported,
        saved: false,
        done: AtomicBool::new(false),
        update_rx: None,
        download_url: None,
    });
    switch_settings_tab(&mut state, 0);

    unsafe {
        SetWindowLongPtrW(dlg, GWLP_USERDATA, (&mut *state as *mut SettingsState) as isize);
    }

    run_modal_loop(parent, dlg, &state.done);
    state.theme.destroy();
    unsafe {
        DeleteObject(font as _);
        DeleteObject(font_semi as _);
        DeleteObject(small_font as _);
    }
    if state.saved {
        Some(state.cfg)
    } else {
        None
    }
}
