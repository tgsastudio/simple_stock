//! 純 Windows 原生 Win32 GDI 精修即時看盤主視窗
//! 特點：
//! - 零 OpenGL 驅動與零內嵌字型緩衝，專用記憶體僅約 6–12 MB（降幅 >90%）
//! - 雙緩衝（Double Buffering）GDI 繪製，視覺配色、卡片層次與圓角對齊原 egui 設計
//! - 完整支援懸停回饋、手型游標、細緻捲軸、三種視窗模式、拖曳排序、持股收益與螢幕邊界保護

use crate::config::{AppConfig, Market, MarketFilter, ViewMode, WindowRect};
use crate::dialogs::{open_add_stock_dialog, open_edit_position_dialog, open_settings_dialog};
use crate::i18n::{Lang, resolve_lang};
use crate::market_hours::TradingPeriod;
use chrono::Utc;
use crate::quote::QuoteData;
use crate::win32_util::{
    apply_dark_title_bar, font_bold, font_regular, font_semibold, format_number, format_shares,
    rgb, to_wide, virtual_screen_bounds,
};
use crate::worker::{QuoteWorker, WM_APP_WORKER_EVENT, WorkerCommand, WorkerEvent, WorkerStatus};
use std::collections::HashMap;
use std::ptr;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreatePen, CreateRoundRectRgn,
    CreateSolidBrush, DeleteDC, DeleteObject, DrawTextW, DT_CENTER, DT_END_ELLIPSIS, DT_LEFT,
    DT_NOPREFIX, DT_RIGHT, DT_SINGLELINE, DT_VCENTER, Ellipse, EndPaint, FillRect, HDC, HFONT,
    IntersectClipRect, InvalidateRect, LineTo, MoveToEx, PAINTSTRUCT, PS_SOLID, RoundRect,
    RestoreDC, SRCCOPY, SaveDC, SelectObject, SetBkMode, SetTextColor, SetWindowRgn, TRANSPARENT,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::HiDpi::{
    AdjustWindowRectExForDpi, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForWindow,
    SetProcessDpiAwarenessContext,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    ReleaseCapture, SetCapture, TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CS_DBLCLKS, CS_HREDRAW, CS_VREDRAW, CreatePopupMenu, CreateWindowExW,
    DefWindowProcW, DestroyMenu, DestroyWindow, DispatchMessageW, GWL_STYLE, GWLP_USERDATA,
    GetClientRect, GetCursorPos, GetMessageW, GetWindowLongPtrW, GetWindowLongW, GetWindowRect,
    HTCAPTION, HWND_NOTOPMOST, HWND_TOPMOST, IDC_ARROW, IDC_HAND, IMAGE_ICON, LWA_ALPHA,
    LoadCursorW, LoadImageW, MF_SEPARATOR, MF_STRING, MINMAXINFO, MSG, PostQuitMessage,
    RegisterClassW, SIZE_MINIMIZED, SW_SHOW, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE,
    SWP_NOSIZE, SendMessageW, SetCursor, SetForegroundWindow, SetLayeredWindowAttributes,
    SetWindowLongPtrW, SetWindowLongW, SetWindowPos, ShowWindow, TPM_LEFTALIGN, TPM_RETURNCMD,
    TPM_TOPALIGN, TrackPopupMenu, TranslateMessage, WM_CLOSE, WM_DESTROY, WM_ERASEBKGND,
    WM_GETMINMAXINFO, WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL,
    WM_MOVE, WM_NCLBUTTONDOWN, WM_PAINT, WM_RBUTTONUP, WM_SETCURSOR, WM_SIZE, WNDCLASSW,
    WS_EX_LAYERED, WS_OVERLAPPEDWINDOW, WS_POPUP,
};

const WM_MOUSELEAVE: u32 = 0x02A3;
const WM_EXITSIZEMOVE: u32 = 0x0232;

/// 將已儲存的視窗座標套用虛擬螢幕邊界保護（容許 Windows DWM 隱形邊框負座標）
fn resolve_window_pos(saved: &WindowRect) -> (i32, i32) {
    let (vx, vy, vw, vh) = virtual_screen_bounds();
    let px = saved.x.round() as i32;
    let py = saved.y.round() as i32;
    let in_bounds = px >= vx - 60 && px <= vx + vw - 80 && py >= vy - 40 && py <= vy + vh - 60;
    if in_bounds {
        (px, py)
    } else {
        let max_x = (vx + vw - 160).max(vx);
        let max_y = (vy + vh - 120).max(vy);
        (px.clamp(vx, max_x), py.clamp(vy, max_y))
    }
}

// 右鍵選單指令 ID
const CMD_CTX_EDIT: u32 = 1001;
const CMD_CTX_TOP: u32 = 1002;
const CMD_CTX_UP: u32 = 1003;
const CMD_CTX_DOWN: u32 = 1004;
const CMD_CTX_DELETE: u32 = 1005;

/// 畫面上可互動元件的命中目標
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HitTarget {
    DragTitle,
    Mode(ViewMode),
    TogglePin,
    Refresh,
    AddStock,
    Settings,
    CloseApp,
    Filter(MarketFilter),
    DragHandle(usize),
    EditPos(usize),
    StockCard(usize),
}

impl HitTarget {
    fn is_button(self) -> bool {
        !matches!(self, HitTarget::DragTitle | HitTarget::StockCard(_))
    }
}

struct HitRect {
    rect: RECT,
    target: HitTarget,
}

/// 依視窗 DPI 快取的 GDI ClearType 字型組合
struct FontSet {
    title: HFONT,
    symbol: HFONT,
    price: HFONT,
    body: HFONT,
    small: HFONT,
    small_bold: HFONT,
    badge: HFONT,
    dpi: u32,
}

impl FontSet {
    fn new(dpi: u32) -> Self {
        let s = |px: i32| ((px as f32) * (dpi as f32) / 96.0).round() as i32;
        Self {
            title: font_bold(s(15)),
            symbol: font_semibold(s(14)),
            price: font_bold(s(16)),
            body: font_regular(s(13)),
            small: font_regular(s(12)),
            small_bold: font_bold(s(12)),
            badge: font_bold(s(10)),
            dpi,
        }
    }

    fn destroy(&mut self) {
        unsafe {
            DeleteObject(self.title as _);
            DeleteObject(self.symbol as _);
            DeleteObject(self.price as _);
            DeleteObject(self.body as _);
            DeleteObject(self.small as _);
            DeleteObject(self.small_bold as _);
            DeleteObject(self.badge as _);
        }
    }
}

/// 主視窗狀態與渲染控制器
struct AppState {
    hwnd: HWND,
    config: AppConfig,
    quotes: HashMap<String, QuoteData>,
    worker: Option<QuoteWorker>,
    worker_status: WorkerStatus,
    error_text: Option<String>,
    tw_period: Option<TradingPeriod>,
    us_period: Option<TradingPeriod>,
    fonts: FontSet,

    // 捲動、懸停與拖曳排序狀態
    scroll_y: i32,
    max_scroll: i32,
    hits: Vec<HitRect>,
    hover_target: Option<HitTarget>,
    hover_card: Option<usize>,
    dragging_idx: Option<usize>,
    tracking_mouse: bool,
    last_minimized: bool,
}

impl AppState {
    fn lang(&self) -> Lang {
        resolve_lang(self.config.language)
    }

    /// 邏輯像素換算為實體 DPI 像素
    fn sc(&self, px: i32) -> i32 {
        ((px as f32) * (self.fonts.dpi as f32) / 96.0).round() as i32
    }

    fn save_and_sync(&self) {
        self.config.save();
        if let Some(w) = &self.worker {
            let _ = w.tx.send(WorkerCommand::UpdateConfig(self.config.clone()));
        }
    }

    fn repaint(&self) {
        unsafe {
            InvalidateRect(self.hwnd, ptr::null(), 0);
        }
    }

    /// 計算指定邏輯客戶區尺寸對應的實際外框大小（含標題列與邊框）
    fn outer_size_for_client(&self, logical_w: f32, logical_h: f32, style: u32) -> (i32, i32) {
        let mut r = RECT {
            left: 0,
            top: 0,
            right: self.sc(logical_w.round() as i32),
            bottom: self.sc(logical_h.round() as i32),
        };
        unsafe {
            AdjustWindowRectExForDpi(&mut r, style, 0, WS_EX_LAYERED, self.fonts.dpi);
        }
        (r.right - r.left, r.bottom - r.top)
    }

    /// 套用視窗模式邊框、置頂、透明度、深色標題列與安全螢幕範圍內的尺寸位置
    fn apply_window_style(&self, resize_to_mode: bool) {
        unsafe {
            let mode = self.config.view_mode;
            let style = if mode == ViewMode::FloatingWidget {
                WS_POPUP
            } else {
                WS_OVERLAPPEDWINDOW
            };

            let cur_style = GetWindowLongW(self.hwnd, GWL_STYLE) as u32;
            let visible_flag = cur_style & 0x1000_0000;
            SetWindowLongW(self.hwnd, GWL_STYLE, (style | visible_flag) as i32);

            let z_after = if self.config.always_on_top {
                HWND_TOPMOST
            } else {
                HWND_NOTOPMOST
            };

            if resize_to_mode {
                let (lw, lh, pos) = if let Some(saved) = self.config.window_rect_for(mode) {
                    let safe_p = Some(resolve_window_pos(&saved));
                    (saved.width.clamp(220.0, 1400.0), saved.height.clamp(200.0, 1200.0), safe_p)
                } else {
                    let [dw, dh] = mode.default_size();
                    (dw, dh, None)
                };

                let (ow, oh) = self.outer_size_for_client(lw, lh, style);
                if let Some((px, py)) = pos {
                    SetWindowPos(
                        self.hwnd,
                        z_after,
                        px,
                        py,
                        ow,
                        oh,
                        SWP_FRAMECHANGED | SWP_NOACTIVATE,
                    );
                } else {
                    SetWindowPos(
                        self.hwnd,
                        z_after,
                        0,
                        0,
                        ow,
                        oh,
                        SWP_NOMOVE | SWP_FRAMECHANGED | SWP_NOACTIVATE,
                    );
                }
            } else {
                SetWindowPos(
                    self.hwnd,
                    z_after,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_FRAMECHANGED | SWP_NOACTIVATE,
                );
            }

            // 僅在浮動模式套用視窗透明度；標準模式維持 100% 完全不透明
            let alpha = if mode == ViewMode::FloatingWidget {
                (self.config.opacity.clamp(0.25, 1.0) * 255.0).round() as u8
            } else {
                255
            };
            SetLayeredWindowAttributes(self.hwnd, 0, alpha, LWA_ALPHA);
            apply_dark_title_bar(self.hwnd, self.config.dark_mode);
            self.update_floating_region();
        }
    }

    fn update_floating_region(&self) {
        unsafe {
            if self.config.view_mode == ViewMode::FloatingWidget {
                let mut r: RECT = std::mem::zeroed();
                GetClientRect(self.hwnd, &mut r);
                let rad = self.sc(16);
                let rgn = CreateRoundRectRgn(0, 0, r.right + 1, r.bottom + 1, rad, rad);
                SetWindowRgn(self.hwnd, rgn, 1);
            } else {
                SetWindowRgn(self.hwnd, ptr::null_mut(), 1);
            }
        }
    }

    /// 記錄目前可視螢幕內的視窗幾何位置與邏輯尺寸
    fn capture_geometry(&mut self) {
        if self.last_minimized {
            return;
        }
        unsafe {
            let mut out_r: RECT = std::mem::zeroed();
            let mut in_r: RECT = std::mem::zeroed();
            if GetWindowRect(self.hwnd, &mut out_r) != 0 && GetClientRect(self.hwnd, &mut in_r) != 0
            {
                let (vx, vy, vw, vh) = virtual_screen_bounds();
                let scale = self.fonts.dpi as f32 / 96.0;
                let w = (in_r.right - in_r.left) as f32 / scale;
                let h = (in_r.bottom - in_r.top) as f32 / scale;
                let x = out_r.left;
                let y = out_r.top;
                if w >= 200.0
                    && h >= 180.0
                    && x >= vx - 60
                    && x <= vx + vw - 80
                    && y >= vy - 40
                    && y <= vy + vh - 60
                {
                    self.config.set_window_rect_for(
                        self.config.view_mode,
                        WindowRect {
                            x: x as f32,
                            y: y as f32,
                            width: w,
                            height: h,
                        },
                    );
                }
            }
        }
    }

    fn drain_worker_events(&mut self) {
        let Some(w) = &self.worker else {
            return;
        };
        let mut dirty = false;
        while let Ok(evt) = w.rx.try_recv() {
            dirty = true;
            match evt {
                WorkerEvent::QuotesUpdated(map) => {
                    for (k, q) in map {
                        if let Some(item) = self.config.watchlist.iter_mut().find(|i| i.key() == k)
                        {
                            if (item.name.is_empty() || item.name == item.symbol)
                                && !q.display_name.is_empty()
                            {
                                item.name = q.display_name.clone();
                            }
                        }
                        self.quotes.insert(k, q);
                    }
                    self.error_text = None;
                }
                WorkerEvent::MarketPeriods { tw, us } => {
                    self.tw_period = tw;
                    self.us_period = us;
                }
                WorkerEvent::Status(s) => {
                    self.worker_status = s;
                }
                WorkerEvent::Error(e) => {
                    self.error_text = Some(e);
                }
            }
        }
        if dirty {
            self.repaint();
        }
    }

    fn visible_indices(&self) -> Vec<usize> {
        self.config
            .watchlist
            .iter()
            .enumerate()
            .filter(|(_, item)| match self.config.market_filter {
                MarketFilter::All => true,
                MarketFilter::TwOnly => item.market == Market::Tw,
                MarketFilter::UsOnly => item.market == Market::Us,
            })
            .map(|(i, _)| i)
            .collect()
    }

    fn price_color(&self, market: Market, change: f64) -> u32 {
        if change.abs() < 1e-6 {
            return if self.config.dark_mode {
                rgb(190, 195, 205)
            } else {
                rgb(80, 85, 95)
            };
        }
        let red_up = self.config.color_scheme.is_red_up(market);
        let is_up = change > 0.0;
        let use_red = (is_up && red_up) || (!is_up && !red_up);
        if use_red {
            if self.config.dark_mode {
                rgb(248, 92, 92)
            } else {
                rgb(205, 38, 38)
            }
        } else if self.config.dark_mode {
            rgb(52, 211, 130)
        } else {
            rgb(20, 145, 78)
        }
    }

    fn hit_test(&self, x: i32, y: i32) -> Option<HitTarget> {
        for h in self.hits.iter().rev() {
            if x >= h.rect.left && x <= h.rect.right && y >= h.rect.top && y <= h.rect.bottom {
                return Some(h.target);
            }
        }
        None
    }

    fn open_edit_position(&mut self, idx: usize) {
        if let Some(item) = self.config.watchlist.get(idx).cloned() {
            let cur_price = self.quotes.get(&item.key()).map(|q| q.price);
            if let Some((shares, cost)) =
                open_edit_position_dialog(self.hwnd, &item, cur_price, &self.config)
            {
                if let Some(target) = self.config.watchlist.get_mut(idx) {
                    target.shares = shares;
                    target.avg_cost = cost;
                }
                self.save_and_sync();
                self.repaint();
            }
        }
    }

    fn show_context_menu(&mut self, idx: usize) {
        let lang = self.lang();
        unsafe {
            let menu = CreatePopupMenu();
            if menu.is_null() {
                return;
            }
            let add_item = |id: u32, text: &str| {
                let w = to_wide(text);
                AppendMenuW(menu, MF_STRING, id as usize, w.as_ptr());
            };
            add_item(CMD_CTX_EDIT, lang.ctx_edit_position());
            AppendMenuW(menu, MF_SEPARATOR, 0, ptr::null());
            add_item(CMD_CTX_TOP, lang.ctx_pin_top());
            if idx > 0 {
                add_item(CMD_CTX_UP, lang.ctx_move_up());
            }
            if idx + 1 < self.config.watchlist.len() {
                add_item(CMD_CTX_DOWN, lang.ctx_move_down());
            }
            AppendMenuW(menu, MF_SEPARATOR, 0, ptr::null());
            add_item(CMD_CTX_DELETE, lang.ctx_delete());

            let mut pt = POINT { x: 0, y: 0 };
            GetCursorPos(&mut pt);
            SetForegroundWindow(self.hwnd);
            let cmd = TrackPopupMenu(
                menu,
                TPM_LEFTALIGN | TPM_TOPALIGN | TPM_RETURNCMD,
                pt.x,
                pt.y,
                0,
                self.hwnd,
                ptr::null(),
            ) as u32;
            DestroyMenu(menu);

            match cmd {
                CMD_CTX_EDIT => self.open_edit_position(idx),
                CMD_CTX_TOP if idx > 0 => {
                    let it = self.config.watchlist.remove(idx);
                    self.config.watchlist.insert(0, it);
                    self.save_and_sync();
                    self.repaint();
                }
                CMD_CTX_UP if idx > 0 => {
                    self.config.watchlist.swap(idx, idx - 1);
                    self.save_and_sync();
                    self.repaint();
                }
                CMD_CTX_DOWN if idx + 1 < self.config.watchlist.len() => {
                    self.config.watchlist.swap(idx, idx + 1);
                    self.save_and_sync();
                    self.repaint();
                }
                CMD_CTX_DELETE if idx < self.config.watchlist.len() => {
                    self.config.watchlist.remove(idx);
                    self.hover_card = None;
                    self.save_and_sync();
                    self.repaint();
                }
                _ => {}
            }
        }
    }

    /// 雙緩衝 GDI 畫面主繪製函式（配色與間距精準對齊原 egui 介面）
    fn paint(&mut self) {
        unsafe {
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            let hdc = BeginPaint(self.hwnd, &mut ps);
            if hdc.is_null() {
                return;
            }

            let mut rc: RECT = std::mem::zeroed();
            GetClientRect(self.hwnd, &mut rc);
            let width = (rc.right - rc.left).max(1);
            let height = (rc.bottom - rc.top).max(1);

            let mem_dc = CreateCompatibleDC(hdc);
            let mem_bmp = CreateCompatibleBitmap(hdc, width, height);
            let old_bmp = SelectObject(mem_dc, mem_bmp as _);
            SetBkMode(mem_dc, TRANSPARENT as i32);

            self.hits.clear();

            // 視覺調色盤（精準對齊原 egui 深/淺色風格）
            let dark = self.config.dark_mode;
            let bg_col = if dark { rgb(18, 22, 30) } else { rgb(246, 248, 252) };
            let card_col = if dark { rgb(28, 33, 44) } else { rgb(255, 255, 255) };
            let card_hover = if dark { rgb(34, 41, 55) } else { rgb(249, 251, 255) };
            let card_border = if dark { rgb(42, 50, 66) } else { rgb(222, 228, 238) };
            let divider_col = if dark { rgb(36, 43, 56) } else { rgb(226, 232, 240) };
            let text_primary = if dark { rgb(236, 241, 249) } else { rgb(24, 28, 36) };
            let text_muted = if dark { rgb(140, 150, 168) } else { rgb(105, 114, 128) };
            let pill_active = if dark { rgb(43, 76, 126) } else { rgb(214, 232, 255) };
            let pill_hover = if dark { rgb(42, 51, 68) } else { rgb(228, 236, 248) };
            let pill_idle = if dark { rgb(28, 34, 46) } else { rgb(236, 240, 247) };
            let tw_badge_bg = if dark { rgb(26, 49, 82) } else { rgb(219, 234, 254) };
            let tw_badge_fg = if dark { rgb(96, 165, 250) } else { rgb(37, 99, 235) };
            let us_badge_bg = if dark { rgb(49, 36, 86) } else { rgb(237, 233, 254) };
            let us_badge_fg = if dark { rgb(192, 132, 252) } else { rgb(124, 58, 237) };
            let accent_blue = rgb(90, 160, 255);

            let bg_brush = CreateSolidBrush(bg_col);
            FillRect(mem_dc, &rc, bg_brush);
            DeleteObject(bg_brush as _);

            let pad = self.sc(10);
            let mut y = self.sc(8);
            let mode = self.config.view_mode;
            let is_compact = mode != ViewMode::Standard;
            let lang = self.lang();
            let hover_target = self.hover_target;

            // ----------------------------------------------------------------
            // 1. 頂部工具列（響應式防重疊排版 + 懸停高亮）
            // ----------------------------------------------------------------
            let top_h = self.sc(24);
            let mut rx = width - pad;
            let rad5 = self.sc(5);
            let gap4 = self.sc(4);
            let small_font = self.fonts.small;

            let push_btn = |rx: &mut i32,
                            text: &str,
                            btn_w: i32,
                            active: bool,
                            fg: u32,
                            target: HitTarget,
                            hits: &mut Vec<HitRect>| {
                let bx = *rx - btn_w;
                let r = RECT {
                    left: bx,
                    top: y,
                    right: *rx,
                    bottom: y + top_h,
                };
                let is_hov = hover_target == Some(target);
                let fill = if active {
                    pill_active
                } else if is_hov {
                    pill_hover
                } else {
                    pill_idle
                };
                draw_round_box(
                    mem_dc,
                    &r,
                    fill,
                    if active || is_hov { card_border } else { fill },
                    rad5,
                );
                draw_text(
                    mem_dc,
                    text,
                    &r,
                    small_font,
                    if active || is_hov { text_primary } else { fg },
                    DT_SINGLELINE | DT_VCENTER | DT_CENTER,
                );
                hits.push(HitRect { rect: r, target });
                *rx = bx - gap4;
            };

            let push_btn_styled = |rx: &mut i32,
                                   text: &str,
                                   btn_w: i32,
                                   fill_idle: u32,
                                   fill_hov: u32,
                                   border: u32,
                                   fg: u32,
                                   target: HitTarget,
                                   hits: &mut Vec<HitRect>| {
                let bx = *rx - btn_w;
                let r = RECT {
                    left: bx,
                    top: y,
                    right: *rx,
                    bottom: y + top_h,
                };
                let is_hov = hover_target == Some(target);
                let fill = if is_hov { fill_hov } else { fill_idle };
                draw_round_box(mem_dc, &r, fill, border, rad5);
                draw_text(
                    mem_dc,
                    text,
                    &r,
                    small_font,
                    fg,
                    DT_SINGLELINE | DT_VCENTER | DT_CENTER,
                );
                hits.push(HitRect { rect: r, target });
                *rx = bx - gap4;
            };

            let w22 = self.sc(22);
            let w26 = self.sc(26);
            let w34 = self.sc(34);

            // 先加入標題區（浮動模式拖曳區）於底層，確保右側按鈕永遠優先命中不被遮擋
            let title_r = RECT {
                left: pad,
                top: y,
                right: width - pad,
                bottom: y + top_h,
            };
            self.hits.push(HitRect {
                rect: title_r,
                target: HitTarget::DragTitle,
            });

            if mode == ViewMode::FloatingWidget {
                push_btn(&mut rx, "✕", w22, false, text_muted, HitTarget::CloseApp, &mut self.hits);
            }
            push_btn(
                &mut rx,
                lang.btn_top_settings(),
                self.sc(32),
                false,
                text_muted,
                HitTarget::Settings,
                &mut self.hits,
            );
            push_btn(&mut rx, "＋", w22, false, text_primary, HitTarget::AddStock, &mut self.hits);
            if !is_compact {
                // 「刷」按鈕：獨立青藍色系（Teal/Cyan）方便辨識
                let (ref_bg, ref_hov, ref_bd, ref_fg) = if dark {
                    (rgb(20, 62, 74), rgb(28, 84, 98), rgb(44, 124, 144), rgb(104, 226, 242))
                } else {
                    (rgb(212, 244, 248), rgb(186, 236, 244), rgb(110, 198, 212), rgb(12, 112, 138))
                };
                push_btn_styled(
                    &mut rx,
                    lang.btn_top_refresh(),
                    w22,
                    ref_bg,
                    ref_hov,
                    ref_bd,
                    ref_fg,
                    HitTarget::Refresh,
                    &mut self.hits,
                );

                // 「頂」按鈕：獨立琥珀金黃色系（Amber/Gold），開啟置頂時更加醒目
                let (pin_bg, pin_hov, pin_bd, pin_fg) = match (dark, self.config.always_on_top) {
                    (true, true) => (rgb(132, 88, 18), rgb(156, 106, 24), rgb(235, 178, 52), rgb(255, 240, 180)),
                    (true, false) => (rgb(58, 44, 22), rgb(78, 58, 26), rgb(122, 92, 38), rgb(235, 188, 88)),
                    (false, true) => (rgb(254, 226, 146), rgb(252, 214, 118), rgb(217, 152, 24), rgb(138, 78, 8)),
                    (false, false) => (rgb(254, 245, 220), rgb(252, 236, 194), rgb(224, 190, 110), rgb(162, 104, 18)),
                };
                push_btn_styled(
                    &mut rx,
                    lang.btn_top_pin(),
                    w26,
                    pin_bg,
                    pin_hov,
                    pin_bd,
                    pin_fg,
                    HitTarget::TogglePin,
                    &mut self.hits,
                );
            }

            // 模式切換膠囊（標準 / 浮動）
            for m in [ViewMode::FloatingWidget, ViewMode::Standard] {
                let lbl = match m {
                    ViewMode::Standard => lang.mode_standard(),
                    ViewMode::FloatingWidget => lang.mode_floating(),
                };
                push_btn(
                    &mut rx,
                    lbl,
                    w34,
                    mode == m,
                    if mode == m { text_primary } else { text_muted },
                    HitTarget::Mode(m),
                    &mut self.hits,
                );
            }

            // 繪製左側標題文字（所有模式皆完整顯示 Stock TW，自動限制寬度不與右側按鈕重疊）
            let title_right = (rx - self.sc(4)).max(pad + self.sc(68));
            let title_draw_r = RECT {
                left: pad,
                top: y,
                right: title_right,
                bottom: y + top_h,
            };
            draw_text(
                mem_dc,
                "Stock TW",
                &title_draw_r,
                self.fonts.title,
                text_primary,
                DT_SINGLELINE | DT_VCENTER | DT_LEFT | DT_END_ELLIPSIS,
            );

            let now_ts = Utc::now().timestamp();
            let tw_is_open = self.tw_period.is_some_and(|tp| tp.is_tw_open(now_ts));
            let us_is_open = self
                .us_period
                .is_some_and(|tp| tp.is_us_open(now_ts, self.config.include_us_extended));

            // 空間足夠時顯示 TW / US 盤中指示燈
            if mode == ViewMode::Standard && title_right > pad + self.sc(142) {
                let dot_y = y + top_h / 2;
                let mut dx = pad + self.sc(78);
                let dot_r = self.sc(3).max(3);
                let tw_c = if tw_is_open { rgb(52, 211, 130) } else { rgb(115, 122, 135) };
                draw_dot(mem_dc, dx, dot_y, dot_r, tw_c);
                let tr = RECT {
                    left: dx + self.sc(6),
                    top: y,
                    right: dx + self.sc(30),
                    bottom: y + top_h,
                };
                draw_text(mem_dc, "TW", &tr, self.fonts.small, text_muted, DT_SINGLELINE | DT_VCENTER | DT_LEFT);

                dx += self.sc(32);
                let us_c = if us_is_open { rgb(52, 211, 130) } else { rgb(115, 122, 135) };
                draw_dot(mem_dc, dx, dot_y, dot_r, us_c);
                let ur = RECT {
                    left: dx + self.sc(6),
                    top: y,
                    right: dx + self.sc(30),
                    bottom: y + top_h,
                };
                draw_text(mem_dc, "US", &ur, self.fonts.small, text_muted, DT_SINGLELINE | DT_VCENTER | DT_LEFT);
            }

            y += top_h + self.sc(6);
            draw_hline(mem_dc, pad, width - pad, y, divider_col);
            y += self.sc(6);

            // ----------------------------------------------------------------
            // 2. 分頁篩選（所有模式皆可切換全部/台股/美股）與排序列（標準模式）
            // ----------------------------------------------------------------
            {
                let bar_h = self.sc(22);
                let mut fx = pad;
                let base_tab_w = self.sc(if is_compact { 42 } else { 48 });
                let open_dot_col = if dark { rgb(52, 211, 130) } else { rgb(22, 163, 74) };
                for (f, lbl) in [
                    (MarketFilter::All, lang.filter_all()),
                    (MarketFilter::TwOnly, lang.filter_tw()),
                    (MarketFilter::UsOnly, lang.filter_us()),
                ] {
                    let is_open = match f {
                        MarketFilter::TwOnly => tw_is_open,
                        MarketFilter::UsOnly => us_is_open,
                        MarketFilter::All => false,
                    };
                    let tab_w = if is_open {
                        base_tab_w + self.sc(8)
                    } else {
                        base_tab_w
                    };
                    let fr = RECT {
                        left: fx,
                        top: y,
                        right: fx + tab_w,
                        bottom: y + bar_h,
                    };
                    let active = self.config.market_filter == f;
                    let hov = hover_target == Some(HitTarget::Filter(f));
                    let fill = if active {
                        pill_active
                    } else if hov {
                        pill_hover
                    } else {
                        bg_col
                    };
                    draw_round_box(mem_dc, &fr, fill, if active { card_border } else { fill }, self.sc(5));
                    let txt_r = if is_open {
                        let dot_r = self.sc(3).max(2);
                        draw_dot(mem_dc, fr.left + self.sc(9), fr.top + bar_h / 2, dot_r, open_dot_col);
                        RECT {
                            left: fr.left + self.sc(10),
                            top: fr.top,
                            right: fr.right,
                            bottom: fr.bottom,
                        }
                    } else {
                        fr
                    };
                    draw_text(
                        mem_dc,
                        lbl,
                        &txt_r,
                        self.fonts.small,
                        if active || hov { text_primary } else { text_muted },
                        DT_SINGLELINE | DT_VCENTER | DT_CENTER,
                    );
                    self.hits.push(HitRect {
                        rect: fr,
                        target: HitTarget::Filter(f),
                    });
                    fx += tab_w + self.sc(4);
                }

                y += bar_h + self.sc(6);

                // 總收益摘要列（所有模式皆顯示，若有任一檔持股）
                let mut tw_cost = 0.0;
                let mut tw_val = 0.0;
                let mut us_cost = 0.0;
                let mut us_val = 0.0;
                for item in &self.config.watchlist {
                    if !item.has_position() {
                        continue;
                    }
                    if let Some(q) = self.quotes.get(&item.key()) {
                        match item.market {
                            Market::Tw => {
                                tw_cost += item.shares * item.avg_cost;
                                tw_val += item.shares * q.price;
                            }
                            Market::Us => {
                                us_cost += item.shares * item.avg_cost;
                                us_val += item.shares * q.price;
                            }
                        }
                    }
                }
                if tw_cost > 0.0 || us_cost > 0.0 {
                    let sum_h = self.sc(26);
                    let sum_r = RECT {
                        left: pad,
                        top: y,
                        right: width - pad,
                        bottom: y + sum_h,
                    };
                    draw_round_box(mem_dc, &sum_r, card_col, card_border, self.sc(6));
                    let mut parts = vec![lang.label_total_pnl().to_string()];
                    let sum_col = if dark {
                        rgb(250, 204, 72)
                    } else {
                        rgb(180, 118, 8)
                    };
                    if tw_cost > 0.0 {
                        let p = tw_val - tw_cost;
                        let pct = (p / tw_cost) * 100.0;
                        let sign = if p > 0.0 { "+" } else { "" };
                        parts.push(format!("TWD {sign}{} ({sign}{pct:.2}%)", format_number(p, 0)));
                    }
                    if us_cost > 0.0 {
                        let p = us_val - us_cost;
                        let pct = (p / us_cost) * 100.0;
                        let sign = if p > 0.0 { "+" } else { "" };
                        parts.push(format!("USD {sign}{} ({sign}{pct:.2}%)", format_number(p, 2)));
                    }
                    let txt_r = RECT {
                        left: sum_r.left + self.sc(8),
                        top: sum_r.top,
                        right: sum_r.right - self.sc(8),
                        bottom: sum_r.bottom,
                    };
                    draw_text(
                        mem_dc,
                        &parts.join("  |  "),
                        &txt_r,
                        self.fonts.small,
                        sum_col,
                        DT_SINGLELINE | DT_VCENTER | DT_LEFT | DT_END_ELLIPSIS,
                    );
                    y += sum_h + self.sc(6);
                }
            }

            // ----------------------------------------------------------------
            // 3. 自選股捲動清單卡片（帶裁切區域與細緻捲軸）
            // ----------------------------------------------------------------
            let bottom_bar_h = if mode == ViewMode::FloatingWidget {
                0
            } else {
                self.sc(24)
            };
            let list_top = y;
            let list_bottom = (height - bottom_bar_h - self.sc(4)).max(list_top + self.sc(40));
            let indices = self.visible_indices();

            let allow_drag = self.config.market_filter == MarketFilter::All && !is_compact;

            let gap = self.sc(if is_compact { 6 } else { 7 });
            let mut total_h = 0;
            for &idx in &indices {
                let item = &self.config.watchlist[idx];
                let has_pnl = item.has_position() && self.quotes.contains_key(&item.key());
                let card_h = self.sc(if is_compact {
                    if has_pnl { 72 } else { 48 }
                } else if has_pnl {
                    78
                } else {
                    54
                });
                total_h += card_h + gap;
            }
            let view_h = (list_bottom - list_top).max(1);
            self.max_scroll = (total_h - view_h).max(0);
            self.scroll_y = self.scroll_y.clamp(0, self.max_scroll);

            let sb_reserve = if self.max_scroll > 0 { self.sc(8) } else { 0 };
            let card_right_edge = width - pad - sb_reserve;

            let saved_dc = SaveDC(mem_dc);
            IntersectClipRect(mem_dc, 0, list_top, width, list_bottom);

            let mut hovered_card_rect: Option<RECT> = None;

            fn push_clipped_hit(
                hits: &mut Vec<HitRect>,
                r: RECT,
                clip_top: i32,
                clip_bottom: i32,
                target: HitTarget,
            ) {
                let top = r.top.max(clip_top);
                let bottom = r.bottom.min(clip_bottom);
                if bottom > top && r.right > r.left {
                    hits.push(HitRect {
                        rect: RECT {
                            left: r.left,
                            top,
                            right: r.right,
                            bottom,
                        },
                        target,
                    });
                }
            }

            if indices.is_empty() {
                let empty_r = RECT {
                    left: pad,
                    top: list_top,
                    right: width - pad,
                    bottom: list_bottom,
                };
                draw_text(
                    mem_dc,
                    lang.empty_watchlist(),
                    &empty_r,
                    self.fonts.body,
                    text_muted,
                    DT_SINGLELINE | DT_VCENTER | DT_CENTER,
                );
            } else {
                let mut cy = list_top - self.scroll_y;
                for &idx in &indices {
                    let item = &self.config.watchlist[idx];
                    let quote = self.quotes.get(&item.key());
                    let has_pnl = item.has_position() && quote.is_some();
                    let card_h = self.sc(if is_compact {
                        if has_pnl { 72 } else { 48 }
                    } else if has_pnl {
                        78
                    } else {
                        54
                    });

                    if cy + card_h >= list_top && cy <= list_bottom {
                        let card_r = RECT {
                            left: pad,
                            top: cy,
                            right: card_right_edge,
                            bottom: cy + card_h,
                        };
                        let is_hovered = self.hover_card == Some(idx);
                        if is_hovered {
                            hovered_card_rect = Some(card_r);
                        }
                        let fill_c = if is_hovered { card_hover } else { card_col };
                        let border_c = if self.dragging_idx == Some(idx) {
                            accent_blue
                        } else if is_hovered {
                            if dark { rgb(66, 80, 108) } else { rgb(180, 198, 224) }
                        } else {
                            card_border
                        };
                        draw_round_box(mem_dc, &card_r, fill_c, border_c, self.sc(8));

                        push_clipped_hit(&mut self.hits, card_r, list_top, list_bottom, HitTarget::StockCard(idx));

                        let mut lx = card_r.left + self.sc(10);
                        if allow_drag {
                            let drag_r = RECT {
                                left: lx - self.sc(2),
                                top: card_r.top + self.sc(10),
                                right: lx + self.sc(16),
                                bottom: card_r.top + self.sc(38),
                            };
                            draw_text(
                                mem_dc,
                                "☰",
                                &drag_r,
                                self.fonts.small,
                                text_muted,
                                DT_SINGLELINE | DT_VCENTER | DT_CENTER,
                            );
                            push_clipped_hit(&mut self.hits, drag_r, list_top, list_bottom, HitTarget::DragHandle(idx));
                            lx += self.sc(22);
                        }

                        // 第一列：市場膠囊 + 股票代碼 + 「持股」按鈕
                        let row1_top = card_r.top + self.sc(6);
                        let row1_h = self.sc(20);
                        let mut sx = lx;
                        if !is_compact {
                            let badge_r = RECT {
                                left: sx,
                                top: row1_top + self.sc(2),
                                right: sx + self.sc(24),
                                bottom: row1_top + row1_h - self.sc(2),
                            };
                            let (bbg, bfg) = match item.market {
                                Market::Tw => (tw_badge_bg, tw_badge_fg),
                                Market::Us => (us_badge_bg, us_badge_fg),
                            };
                            draw_round_box(mem_dc, &badge_r, bbg, bbg, self.sc(4));
                            draw_text(
                                mem_dc,
                                item.market.badge(),
                                &badge_r,
                                self.fonts.badge,
                                bfg,
                                DT_SINGLELINE | DT_VCENTER | DT_CENTER,
                            );
                            sx += self.sc(30);
                        }

                        let sym_w = self.sc(if is_compact { 72 } else { 64 });
                        let sym_r = RECT {
                            left: sx,
                            top: row1_top,
                            right: sx + sym_w,
                            bottom: row1_top + row1_h,
                        };
                        draw_text(
                            mem_dc,
                            &item.symbol,
                            &sym_r,
                            self.fonts.symbol,
                            text_primary,
                            DT_SINGLELINE | DT_VCENTER | DT_LEFT | DT_END_ELLIPSIS,
                        );

                        if !is_compact {
                            let pos_btn_r = RECT {
                                left: sx + sym_w + self.sc(2),
                                top: row1_top + self.sc(1),
                                right: sx + sym_w + self.sc(38),
                                bottom: row1_top + row1_h - self.sc(1),
                            };
                            let pos_hov = hover_target == Some(HitTarget::EditPos(idx));
                            let pos_bg = if item.has_position() {
                                pill_active
                            } else if pos_hov {
                                pill_hover
                            } else {
                                pill_idle
                            };
                            draw_round_box(mem_dc, &pos_btn_r, pos_bg, card_border, self.sc(4));
                            draw_text(
                                mem_dc,
                                lang.btn_edit_pos_short(),
                                &pos_btn_r,
                                self.fonts.badge,
                                if item.has_position() || pos_hov {
                                    tw_badge_fg
                                } else {
                                    text_muted
                                },
                                DT_SINGLELINE | DT_VCENTER | DT_CENTER,
                            );
                            push_clipped_hit(&mut self.hits, pos_btn_r, list_top, list_bottom, HitTarget::EditPos(idx));
                        }

                        // 第二列左：股票名稱
                        let name_str = quote
                            .map(|q| q.display_name.as_str())
                            .filter(|s| !s.is_empty())
                            .unwrap_or(&item.name);
                        let mid_x = card_r.left + (card_r.right - card_r.left) / 2 + self.sc(8);
                        let name_r = RECT {
                            left: lx,
                            top: row1_top + row1_h + self.sc(1),
                            right: mid_x,
                            bottom: row1_top + row1_h + self.sc(18),
                        };
                        draw_text(
                            mem_dc,
                            name_str,
                            &name_r,
                            self.fonts.small,
                            text_muted,
                            DT_SINGLELINE | DT_VCENTER | DT_LEFT | DT_END_ELLIPSIS,
                        );

                        // 右側：現價與漲跌幅
                        let right_pad = card_r.right - self.sc(10);
                        let right_left = mid_x - self.sc(18);
                        if let Some(q) = quote {
                            let pcol = self.price_color(item.market, q.change);
                            let price_r = RECT {
                                left: right_left,
                                top: row1_top,
                                right: right_pad,
                                bottom: row1_top + row1_h,
                            };
                            draw_text(
                                mem_dc,
                                &format!("{:.2}", q.price),
                                &price_r,
                                self.fonts.price,
                                pcol,
                                DT_SINGLELINE | DT_VCENTER | DT_RIGHT,
                            );

                            let sign = if q.change > 0.0 { "+" } else { "" };
                            let chg_str = if is_compact {
                                format!("{sign}{:.2}%", q.change_percent)
                            } else {
                                format!("{sign}{:.2} ({sign}{:.2}%)", q.change, q.change_percent)
                            };
                            let chg_r = RECT {
                                left: right_left,
                                top: row1_top + row1_h + self.sc(1),
                                right: right_pad,
                                bottom: row1_top + row1_h + self.sc(18),
                            };
                            draw_text(
                                mem_dc,
                                &chg_str,
                                &chg_r,
                                self.fonts.small,
                                pcol,
                                DT_SINGLELINE | DT_VCENTER | DT_RIGHT,
                            );

                            // 第三列：持股與即時收益（嚴格位於第二列下方，絕不重疊）
                            if let Some((profit, pct)) = item.pnl(q.price) {
                                let dec = if item.market == Market::Tw { 0 } else { 2 };
                                let p_sign = if profit > 0.0 { "+" } else { "" };
                                let pnl_col = self.price_color(item.market, profit);
                                let div_y = row1_top + row1_h + self.sc(21);
                                let pnl_div_col = if dark {
                                    rgb(64, 76, 102)
                                } else {
                                    rgb(194, 204, 218)
                                };
                                let div_r = RECT {
                                    left: lx,
                                    top: div_y,
                                    right: right_pad,
                                    bottom: div_y + self.sc(1).max(1),
                                };
                                let br = CreateSolidBrush(pnl_div_col);
                                FillRect(mem_dc, &div_r, br);
                                DeleteObject(br as _);

                                let row3_top = div_y + self.sc(3);
                                let row3_bot = card_r.bottom - self.sc(4);
                                if !is_compact {
                                    let hold_r = RECT {
                                        left: lx,
                                        top: row3_top,
                                        right: right_left,
                                        bottom: row3_bot,
                                    };
                                    let hold_txt = format!(
                                        "{} {} @ {:.2}",
                                        lang.label_holding_short(),
                                        format_shares(item.shares),
                                        item.avg_cost
                                    );
                                    draw_text(
                                        mem_dc,
                                        &hold_txt,
                                        &hold_r,
                                        self.fonts.small,
                                        text_muted,
                                        DT_SINGLELINE | DT_VCENTER | DT_LEFT | DT_END_ELLIPSIS,
                                    );
                                }
                                let pnl_r = RECT {
                                    left: if is_compact { lx } else { right_left - self.sc(42) },
                                    top: row3_top,
                                    right: right_pad,
                                    bottom: row3_bot,
                                };
                                let pnl_txt = format!(
                                    "{} {p_sign}{} ({p_sign}{pct:.2}%)",
                                    lang.label_pnl(),
                                    format_number(profit, dec)
                                );
                                draw_text(
                                    mem_dc,
                                    &pnl_txt,
                                    &pnl_r,
                                    self.fonts.small_bold,
                                    pnl_col,
                                    DT_SINGLELINE | DT_VCENTER | DT_RIGHT,
                                );
                            }
                        } else {
                            let wait_r = RECT {
                                left: right_left,
                                top: row1_top,
                                right: right_pad,
                                bottom: row1_top + row1_h,
                            };
                            draw_text(
                                mem_dc,
                                "--",
                                &wait_r,
                                self.fonts.price,
                                text_muted,
                                DT_SINGLELINE | DT_VCENTER | DT_RIGHT,
                            );
                        }
                    }

                    cy += card_h + gap;
                }
            }

            // 細緻現代捲軸（僅在內容超出時顯示）
            if self.max_scroll > 0 && total_h > 0 {
                let thumb_h = ((view_h as f32 * view_h as f32 / total_h as f32) as i32)
                    .clamp(self.sc(24), view_h);
                let ratio = self.scroll_y as f32 / self.max_scroll as f32;
                let thumb_y = list_top + ((view_h - thumb_h) as f32 * ratio).round() as i32;
                let sb_r = RECT {
                    left: width - pad - self.sc(4),
                    top: thumb_y,
                    right: width - pad,
                    bottom: thumb_y + thumb_h,
                };
                let sb_col = if dark { rgb(68, 78, 98) } else { rgb(185, 195, 210) };
                draw_round_box(mem_dc, &sb_r, sb_col, sb_col, self.sc(3));
            }

            RestoreDC(mem_dc, saved_dc);

            // ----------------------------------------------------------------
            // 4. 懸停詳細資訊浮動卡（自動避開目前懸停的卡片）
            // ----------------------------------------------------------------
            if let (Some(hidx), Some(card_r)) = (self.hover_card, hovered_card_rect) {
                if self.dragging_idx.is_none() {
                    if let Some(item) = self.config.watchlist.get(hidx) {
                        if let Some(q) = self.quotes.get(&item.key()) {
                            let mut lines = vec![format!(
                                "{} ({})  {}{:.2}",
                                q.display_name,
                                q.symbol,
                                lang.prev_close(),
                                q.prev_close
                            )];
                            let mut ohl = Vec::new();
                            if let Some(o) = q.open {
                                ohl.push(format!("{}{o:.2}", lang.open_price()));
                            }
                            if let (Some(h), Some(l)) = (q.high, q.low) {
                                ohl.push(format!("{}{h:.2}/{l:.2}", lang.high_low()));
                            }
                            if let Some(v) = q.volume {
                                ohl.push(format!("{}{v}", lang.volume()));
                            }
                            if !ohl.is_empty() {
                                lines.push(ohl.join("  "));
                            }
                            if item.has_position() {
                                let dec = if item.market == Market::Tw { 0 } else { 2 };
                                lines.push(format!(
                                    "{}{}  {}{}",
                                    lang.label_cost_basis(),
                                    format_number(item.shares * item.avg_cost, dec),
                                    lang.label_market_value(),
                                    format_number(item.shares * q.price, dec)
                                ));
                            }
                            lines.push(lang.source_updated(q.source, &q.updated_at));

                            let line_h = self.sc(17);
                            let tip_h = lines.len() as i32 * line_h + self.sc(12);
                            // 若下方有空間則顯示在卡片下方，否則顯示在卡片上方
                            let tip_top = if card_r.bottom + self.sc(4) + tip_h <= list_bottom {
                                card_r.bottom + self.sc(4)
                            } else {
                                (card_r.top - tip_h - self.sc(4)).max(list_top)
                            };
                            let tip_r = RECT {
                                left: pad + self.sc(4),
                                top: tip_top,
                                right: width - pad - self.sc(4),
                                bottom: (tip_top + tip_h).min(list_bottom),
                            };
                            let tip_bg = if dark { rgb(20, 26, 38) } else { rgb(252, 253, 255) };
                            draw_round_box(mem_dc, &tip_r, tip_bg, accent_blue, self.sc(7));
                            for (i, ln) in lines.iter().enumerate() {
                                let lr = RECT {
                                    left: tip_r.left + self.sc(10),
                                    top: tip_r.top + self.sc(6) + (i as i32) * line_h,
                                    right: tip_r.right - self.sc(10),
                                    bottom: tip_r.top + self.sc(6) + ((i as i32) + 1) * line_h,
                                };
                                draw_text(
                                    mem_dc,
                                    ln,
                                    &lr,
                                    self.fonts.small,
                                    if i == 0 { text_primary } else { text_muted },
                                    DT_SINGLELINE | DT_VCENTER | DT_LEFT | DT_END_ELLIPSIS,
                                );
                            }
                        }
                    }
                }
            }

            // ----------------------------------------------------------------
            // 5. 底部狀態列
            // ----------------------------------------------------------------
            if bottom_bar_h > 0 {
                draw_hline(mem_dc, pad, width - pad, height - bottom_bar_h, divider_col);
                let bar_r = RECT {
                    left: 0,
                    top: height - bottom_bar_h + 1,
                    right: width,
                    bottom: height,
                };
                let status_str = if let Some(err) = &self.error_text {
                    err.as_str()
                } else {
                    match self.worker_status {
                        WorkerStatus::Initializing => lang.status_init(),
                        WorkerStatus::Updating => lang.status_updating(),
                        WorkerStatus::Updated => lang.status_updated(),
                        WorkerStatus::IdleClosed => lang.status_idle_closed(),
                    }
                };
                let status_col = if self.error_text.is_some() {
                    rgb(245, 130, 100)
                } else {
                    text_muted
                };
                let left_r = RECT {
                    left: pad,
                    top: bar_r.top,
                    right: width - pad - self.sc(92),
                    bottom: bar_r.bottom,
                };
                draw_text(
                    mem_dc,
                    status_str,
                    &left_r,
                    self.fonts.small,
                    status_col,
                    DT_SINGLELINE | DT_VCENTER | DT_LEFT | DT_END_ELLIPSIS,
                );

                let right_r = RECT {
                    left: width - pad - self.sc(104),
                    top: bar_r.top,
                    right: width - pad,
                    bottom: bar_r.bottom,
                };
                draw_text(
                    mem_dc,
                    lang.refresh_label(self.config.refresh_mode),
                    &right_r,
                    self.fonts.small,
                    text_muted,
                    DT_SINGLELINE | DT_VCENTER | DT_RIGHT,
                );
            }

            BitBlt(hdc, 0, 0, width, height, mem_dc, 0, 0, SRCCOPY);
            SelectObject(mem_dc, old_bmp);
            DeleteObject(mem_bmp as _);
            DeleteDC(mem_dc);
            EndPaint(self.hwnd, &ps);
        }
    }
}

unsafe fn draw_hline(hdc: HDC, x1: i32, x2: i32, y: i32, color: u32) {
    unsafe {
        let pen = CreatePen(PS_SOLID, 1, color);
        let old = SelectObject(hdc, pen as _);
        MoveToEx(hdc, x1, y, ptr::null_mut());
        LineTo(hdc, x2, y);
        SelectObject(hdc, old);
        DeleteObject(pen as _);
    }
}

unsafe fn draw_round_box(hdc: HDC, r: &RECT, fill: u32, border: u32, radius: i32) {
    unsafe {
        let brush = CreateSolidBrush(fill);
        let pen = CreatePen(PS_SOLID, 1, border);
        let old_b = SelectObject(hdc, brush as _);
        let old_p = SelectObject(hdc, pen as _);
        RoundRect(hdc, r.left, r.top, r.right, r.bottom, radius, radius);
        SelectObject(hdc, old_b);
        SelectObject(hdc, old_p);
        DeleteObject(brush as _);
        DeleteObject(pen as _);
    }
}

unsafe fn draw_dot(hdc: HDC, cx: i32, cy: i32, r: i32, color: u32) {
    unsafe {
        let brush = CreateSolidBrush(color);
        let pen = CreatePen(PS_SOLID, 1, color);
        let old_b = SelectObject(hdc, brush as _);
        let old_p = SelectObject(hdc, pen as _);
        Ellipse(hdc, cx - r, cy - r, cx + r, cy + r);
        SelectObject(hdc, old_b);
        SelectObject(hdc, old_p);
        DeleteObject(brush as _);
        DeleteObject(pen as _);
    }
}

unsafe fn draw_text(hdc: HDC, text: &str, rect: &RECT, font: HFONT, color: u32, flags: u32) {
    unsafe {
        let old_f = SelectObject(hdc, font as _);
        SetTextColor(hdc, color);
        let mut r = *rect;
        let w = to_wide(text);
        DrawTextW(hdc, w.as_ptr(), (w.len() - 1) as i32, &mut r, flags | DT_NOPREFIX);
        SelectObject(hdc, old_f);
    }
}

unsafe extern "system" fn main_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut AppState };
    if ptr.is_null() {
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }
    let st = unsafe { &mut *ptr };

    match msg {
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            st.paint();
            0
        }
        WM_APP_WORKER_EVENT => {
            st.drain_worker_events();
            0
        }
        WM_SETCURSOR => {
            if (lparam & 0xFFFF) == 1 /* HTCLIENT */ {
                if let Some(t) = st.hover_target {
                    if t.is_button() {
                        unsafe {
                            SetCursor(LoadCursorW(ptr::null_mut(), IDC_HAND));
                        }
                        return 1;
                    }
                }
            }
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
        WM_GETMINMAXINFO => {
            let mmi = unsafe { &mut *(lparam as *mut MINMAXINFO) };
            mmi.ptMinTrackSize.x = st.sc(230);
            mmi.ptMinTrackSize.y = st.sc(200);
            0
        }
        WM_SIZE => {
            let is_min = wparam == SIZE_MINIMIZED as usize;
            if is_min != st.last_minimized {
                st.last_minimized = is_min;
                if let Some(w) = &st.worker {
                    let _ = w.tx.send(WorkerCommand::SetMinimized(is_min));
                }
            }
            if !is_min {
                st.update_floating_region();
                st.capture_geometry();
                st.repaint();
            }
            0
        }
        WM_MOVE => {
            st.capture_geometry();
            0
        }
        WM_EXITSIZEMOVE => {
            st.capture_geometry();
            st.config.save();
            0
        }
        WM_MOUSEWHEEL => {
            let delta = ((wparam >> 16) as i16) as i32;
            st.scroll_y = (st.scroll_y - delta / 3).clamp(0, st.max_scroll);
            st.repaint();
            0
        }
        WM_MOUSEMOVE => {
            let x = (lparam & 0xFFFF) as i16 as i32;
            let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;

            if !st.tracking_mouse {
                st.tracking_mouse = true;
                let mut tme = TRACKMOUSEEVENT {
                    cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                    dwFlags: TME_LEAVE,
                    hwndTrack: hwnd,
                    dwHoverTime: 0,
                };
                unsafe {
                    TrackMouseEvent(&mut tme);
                }
            }

            if let Some(from_idx) = st.dragging_idx {
                if let Some(HitTarget::StockCard(to_idx) | HitTarget::DragHandle(to_idx)) =
                    st.hit_test(x, y)
                {
                    if from_idx != to_idx
                        && from_idx < st.config.watchlist.len()
                        && to_idx < st.config.watchlist.len()
                    {
                        let item = st.config.watchlist.remove(from_idx);
                        st.config.watchlist.insert(to_idx, item);
                        st.dragging_idx = Some(to_idx);
                        st.save_and_sync();
                        st.repaint();
                    }
                }
            } else {
                let hit = st.hit_test(x, y);
                let new_hover_card = match hit {
                    Some(
                        HitTarget::StockCard(i)
                        | HitTarget::EditPos(i)
                        | HitTarget::DragHandle(i),
                    ) => Some(i),
                    _ => None,
                };
                if hit != st.hover_target || new_hover_card != st.hover_card {
                    st.hover_target = hit;
                    st.hover_card = new_hover_card;
                    st.repaint();
                }
            }
            0
        }
        WM_MOUSELEAVE => {
            st.tracking_mouse = false;
            if st.hover_target.is_some() || st.hover_card.is_some() {
                st.hover_target = None;
                st.hover_card = None;
                st.repaint();
            }
            0
        }
        WM_LBUTTONDOWN => {
            let x = (lparam & 0xFFFF) as i16 as i32;
            let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;
            match st.hit_test(x, y) {
                Some(HitTarget::DragTitle) if st.config.view_mode == ViewMode::FloatingWidget => {
                    unsafe {
                        ReleaseCapture();
                        SendMessageW(hwnd, WM_NCLBUTTONDOWN, HTCAPTION as usize, 0);
                    }
                }
                Some(HitTarget::Mode(new_mode)) => {
                    if st.config.view_mode != new_mode {
                        st.capture_geometry();
                        st.config.view_mode = new_mode;
                        if new_mode == ViewMode::FloatingWidget {
                            st.config.always_on_top = true;
                        }
                        st.apply_window_style(true);
                        st.save_and_sync();
                        st.repaint();
                    }
                }
                Some(HitTarget::TogglePin) => {
                    st.config.always_on_top = !st.config.always_on_top;
                    st.apply_window_style(false);
                    st.save_and_sync();
                    st.repaint();
                }
                Some(HitTarget::Refresh) => {
                    if let Some(w) = &st.worker {
                        let _ = w.tx.send(WorkerCommand::ForceRefresh);
                    }
                    st.worker_status = WorkerStatus::Updating;
                    st.repaint();
                }
                Some(HitTarget::AddStock) => {
                    let added = open_add_stock_dialog(hwnd, &st.config);
                    if !added.is_empty() {
                        for item in added {
                            let k = item.key();
                            if !st.config.watchlist.iter().any(|x| x.key() == k) {
                                st.config.watchlist.push(item);
                            }
                        }
                        st.save_and_sync();
                        st.repaint();
                    }
                }
                Some(HitTarget::Settings) => {
                    if let Some(new_cfg) = open_settings_dialog(hwnd, &st.config) {
                        st.config = new_cfg;
                        st.apply_window_style(false);
                        st.save_and_sync();
                        st.repaint();
                    }
                }
                Some(HitTarget::CloseApp) => unsafe {
                    SendMessageW(hwnd, WM_CLOSE, 0, 0);
                },
                Some(HitTarget::Filter(f)) => {
                    st.config.market_filter = f;
                    st.scroll_y = 0;
                    st.save_and_sync();
                    st.repaint();
                }
                Some(HitTarget::EditPos(idx)) => {
                    st.open_edit_position(idx);
                }
                Some(HitTarget::DragHandle(idx)) => {
                    st.dragging_idx = Some(idx);
                    unsafe {
                        SetCapture(hwnd);
                    }
                    st.repaint();
                }
                _ => {}
            }
            0
        }
        WM_LBUTTONUP => {
            if st.dragging_idx.is_some() {
                st.dragging_idx = None;
                unsafe {
                    ReleaseCapture();
                }
                st.repaint();
            }
            0
        }
        WM_LBUTTONDBLCLK => {
            let x = (lparam & 0xFFFF) as i16 as i32;
            let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;
            if let Some(HitTarget::StockCard(idx) | HitTarget::EditPos(idx)) = st.hit_test(x, y) {
                st.open_edit_position(idx);
            }
            0
        }
        WM_RBUTTONUP => {
            let x = (lparam & 0xFFFF) as i16 as i32;
            let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;
            if let Some(
                HitTarget::StockCard(idx) | HitTarget::EditPos(idx) | HitTarget::DragHandle(idx),
            ) = st.hit_test(x, y)
            {
                st.show_context_menu(idx);
            }
            0
        }
        WM_CLOSE => {
            st.capture_geometry();
            st.config.save();
            unsafe {
                DestroyWindow(hwnd);
            }
            0
        }
        WM_DESTROY => {
            if let Some(w) = &st.worker {
                let _ = w.tx.send(WorkerCommand::Shutdown);
            }
            st.fonts.destroy();
            unsafe {
                PostQuitMessage(0);
            }
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

/// 啟動純 Win32 原生看盤主視窗
pub fn run_win32_app() {
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);

        let hinst = GetModuleHandleW(ptr::null());
        let cls_name = to_wide("StockTW_MainWindow_v2");
        let app_icon = LoadImageW(hinst, 1 as *const u16, IMAGE_ICON, 0, 0, 0x0000_8040);

        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW | CS_DBLCLKS,
            lpfnWndProc: Some(main_wndproc),
            hInstance: hinst,
            hIcon: app_icon as _,
            hCursor: LoadCursorW(ptr::null_mut(), IDC_ARROW),
            lpszClassName: cls_name.as_ptr(),
            ..std::mem::zeroed()
        };
        RegisterClassW(&wc);

        let initial_cfg = AppConfig::load();
        let style = if initial_cfg.view_mode == ViewMode::FloatingWidget {
            WS_POPUP
        } else {
            WS_OVERLAPPEDWINDOW
        };

        let (vx, vy, _, _) = virtual_screen_bounds();
        let (x, y) = if let Some(saved) = initial_cfg.window_rect_for(initial_cfg.view_mode) {
            resolve_window_pos(&saved)
        } else {
            (vx + 120, vy + 120)
        };

        let title = to_wide("Stock TW");
        let hwnd = CreateWindowExW(
            WS_EX_LAYERED,
            cls_name.as_ptr(),
            title.as_ptr(),
            style,
            x,
            y,
            480,
            560,
            ptr::null_mut(),
            ptr::null_mut(),
            hinst,
            ptr::null(),
        );
        if hwnd.is_null() {
            return;
        }

        let dpi = GetDpiForWindow(hwnd).max(96);
        let fonts = FontSet::new(dpi);

        let mut state = Box::new(AppState {
            hwnd,
            config: initial_cfg.clone(),
            quotes: HashMap::new(),
            worker: None,
            worker_status: WorkerStatus::Initializing,
            error_text: None,
            tw_period: None,
            us_period: None,
            fonts,
            scroll_y: 0,
            max_scroll: 0,
            hits: Vec::new(),
            hover_target: None,
            hover_card: None,
            dragging_idx: None,
            tracking_mouse: false,
            last_minimized: false,
        });

        state.worker = Some(QuoteWorker::spawn(initial_cfg, hwnd as usize));
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, (&mut *state as *mut AppState) as isize);

        state.apply_window_style(true);
        ShowWindow(hwnd, SW_SHOW);
        SetForegroundWindow(hwnd);
        state.repaint();

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
