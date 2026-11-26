//! Stock TW 主畫面介面模組（egui）
//! 包含標準 / 側邊條 / 透明浮動三種模式、自選股拖曳排序、持股成本與收益計算、
//! 新增自選股線上搜尋、設定面板與 GitHub Release 自動更新檢查。

use crate::config::{
    AppConfig, ColorScheme, LanguageSetting, Market, MarketFilter, RefreshMode, SortDisplay,
    StockItem, ViewMode, WindowRect,
};
use crate::fonts::configure_fonts;
use crate::i18n::{Lang, resolve_lang};
use crate::market_hours::{UsPhase, current_market_status};
use crate::quote::{QuoteData, SearchResult, guess_market, search_symbols};
use crate::updater::{UpdateCheckResult, check_for_updates};
use crate::worker::{QuoteWorker, WorkerCommand, WorkerEvent, WorkerStatus};
use egui::{
    Align, Color32, Context, CornerRadius, Frame, Id, Layout, Margin, OpenUrl, Pos2, RichText,
    Sense, Stroke, Vec2, ViewportCommand, WindowLevel,
};
use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver};
use std::thread;

fn format_number(val: f64, decimals: usize) -> String {
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

fn format_shares(shares: f64) -> String {
    if (shares - shares.round()).abs() < 1e-6 {
        format_number(shares, 0)
    } else {
        format!("{shares:.4}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

pub struct StockApp {
    config: AppConfig,
    quotes: HashMap<String, QuoteData>,
    worker: QuoteWorker,
    worker_status: WorkerStatus,
    error_text: Option<String>,
    last_minimized: bool,
    applied_initial_viewport: bool,

    // UI dialogs
    show_add_dialog: bool,
    show_settings_dialog: bool,

    // Add stock form
    add_symbol_input: String,
    add_name_input: String,
    add_shares_input: String,
    add_cost_input: String,
    add_market_override: Option<Market>,
    search_results: Vec<SearchResult>,
    search_loading: bool,
    search_error: Option<String>,
    search_rx: Option<Receiver<Result<Vec<SearchResult>, String>>>,

    // Edit position dialog
    edit_pos_idx: Option<usize>,
    edit_shares_input: String,
    edit_cost_input: String,

    // Update checker state
    update_checking: bool,
    update_result: Option<Result<UpdateCheckResult, String>>,
    update_rx: Option<Receiver<Result<UpdateCheckResult, String>>>,

    // Drag-and-drop state
    dragging_idx: Option<usize>,
}

impl StockApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        configure_fonts(&cc.egui_ctx);
        let config = AppConfig::load();
        let worker = QuoteWorker::spawn(config.clone(), cc.egui_ctx.clone());

        Self {
            config,
            quotes: HashMap::new(),
            worker,
            worker_status: WorkerStatus::Initializing,
            error_text: None,
            last_minimized: false,
            applied_initial_viewport: false,
            show_add_dialog: false,
            show_settings_dialog: false,
            add_symbol_input: String::new(),
            add_name_input: String::new(),
            add_shares_input: String::new(),
            add_cost_input: String::new(),
            add_market_override: None,
            search_results: Vec::new(),
            search_loading: false,
            search_error: None,
            search_rx: None,
            edit_pos_idx: None,
            edit_shares_input: String::new(),
            edit_cost_input: String::new(),
            update_checking: false,
            update_result: None,
            update_rx: None,
            dragging_idx: None,
        }
    }

    fn lang(&self) -> Lang {
        resolve_lang(self.config.language)
    }

    fn save_and_sync(&self) {
        self.config.save();
        let _ = self
            .worker
            .tx
            .send(WorkerCommand::UpdateConfig(self.config.clone()));
    }

    fn capture_window_geometry(&mut self, ctx: &Context) {
        let (minimized, outer, inner) = ctx.input(|i| {
            let vp = i.viewport();
            (
                vp.minimized.unwrap_or(false),
                vp.outer_rect,
                vp.inner_rect,
            )
        });
        if minimized {
            return;
        }
        if let (Some(out_r), Some(in_r)) = (outer, inner) {
            let w = in_r.width();
            let h = in_r.height();
            let x = out_r.min.x;
            let y = out_r.min.y;
            if w >= 200.0 && h >= 180.0 && x >= -100.0 && x < 4000.0 && y >= -100.0 && y < 3000.0 {
                let rect = WindowRect {
                    x,
                    y,
                    width: w,
                    height: h,
                };
                self.config.set_window_rect_for(self.config.view_mode, rect);
            }
        }
    }

    fn apply_viewport_for_mode(&mut self, ctx: &Context, resize: bool) {
        let mode = self.config.view_mode;
        let decorations = mode != ViewMode::FloatingWidget;
        ctx.send_viewport_cmd(ViewportCommand::Decorations(decorations));

        let level = if self.config.always_on_top {
            WindowLevel::AlwaysOnTop
        } else {
            WindowLevel::Normal
        };
        ctx.send_viewport_cmd(ViewportCommand::WindowLevel(level));

        if resize {
            if let Some(saved) = self.config.window_rect_for(mode) {
                ctx.send_viewport_cmd(ViewportCommand::InnerSize(Vec2::new(
                    saved.width.max(220.0),
                    saved.height.max(200.0),
                )));
                if saved.x > -2000.0 && saved.x < 8000.0 && saved.y > -2000.0 && saved.y < 8000.0 {
                    ctx.send_viewport_cmd(ViewportCommand::OuterPosition(Pos2::new(
                        saved.x, saved.y,
                    )));
                }
            } else {
                let [w, h] = mode.default_size();
                ctx.send_viewport_cmd(ViewportCommand::InnerSize(Vec2::new(w, h)));
            }
        }
    }

    fn switch_view_mode(&mut self, ctx: &Context, new_mode: ViewMode) {
        if self.config.view_mode == new_mode {
            return;
        }
        self.capture_window_geometry(ctx);
        self.config.view_mode = new_mode;
        if new_mode == ViewMode::FloatingWidget {
            self.config.always_on_top = true;
        }
        self.apply_viewport_for_mode(ctx, true);
        self.save_and_sync();
    }

    fn start_update_check(&mut self, ctx: &Context) {
        if self.update_checking {
            return;
        }
        self.update_checking = true;
        self.update_result = None;
        let (tx, rx) = mpsc::channel();
        self.update_rx = Some(rx);
        let ctx_clone = ctx.clone();
        thread::spawn(move || {
            let res = check_for_updates(env!("CARGO_PKG_VERSION"));
            let _ = tx.send(res);
            ctx_clone.request_repaint();
        });
    }

    fn open_edit_position(&mut self, idx: usize) {
        if let Some(item) = self.config.watchlist.get(idx) {
            self.edit_pos_idx = Some(idx);
            self.edit_shares_input = if item.shares > 0.0 {
                format_shares(item.shares).replace(',', "")
            } else {
                String::new()
            };
            self.edit_cost_input = if item.avg_cost > 0.0 {
                format!("{:.2}", item.avg_cost)
            } else {
                String::new()
            };
        }
    }

    fn drain_worker_events(&mut self) {
        while let Ok(evt) = self.worker.rx.try_recv() {
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
                WorkerEvent::Status(s) => {
                    self.worker_status = s;
                }
                WorkerEvent::Error(e) => {
                    self.error_text = Some(e);
                }
            }
        }

        if let Some(rx) = &self.search_rx {
            if let Ok(res) = rx.try_recv() {
                self.search_loading = false;
                self.search_rx = None;
                match res {
                    Ok(list) => {
                        self.search_results = list;
                        self.search_error = None;
                    }
                    Err(e) => {
                        self.search_error = Some(e);
                    }
                }
            }
        }

        if let Some(rx) = &self.update_rx {
            if let Ok(res) = rx.try_recv() {
                self.update_checking = false;
                self.update_rx = None;
                self.update_result = Some(res);
            }
        }
    }

    fn price_color(&self, market: Market, change: f64) -> Color32 {
        if change.abs() < 1e-6 {
            return if self.config.dark_mode {
                Color32::from_rgb(190, 195, 205)
            } else {
                Color32::from_rgb(80, 85, 95)
            };
        }
        let red_up = self.config.color_scheme.is_red_up(market);
        let is_up = change > 0.0;
        let use_red = (is_up && red_up) || (!is_up && !red_up);
        if use_red {
            if self.config.dark_mode {
                Color32::from_rgb(248, 92, 92)
            } else {
                Color32::from_rgb(205, 38, 38)
            }
        } else if self.config.dark_mode {
            Color32::from_rgb(52, 211, 130)
        } else {
            Color32::from_rgb(20, 145, 78)
        }
    }

    fn bg_color(&self) -> Color32 {
        let alpha = (self.config.opacity.clamp(0.25, 1.0) * 255.0) as u8;
        if self.config.dark_mode {
            Color32::from_rgba_unmultiplied(18, 22, 30, alpha)
        } else {
            Color32::from_rgba_unmultiplied(246, 248, 252, alpha)
        }
    }

    fn card_color(&self) -> Color32 {
        let alpha = ((self.config.opacity * 0.92).clamp(0.2, 1.0) * 255.0) as u8;
        if self.config.dark_mode {
            Color32::from_rgba_unmultiplied(28, 33, 44, alpha)
        } else {
            Color32::from_rgba_unmultiplied(255, 255, 255, alpha)
        }
    }

    fn add_stock_item(
        &mut self,
        symbol: String,
        name: String,
        market: Market,
        shares: f64,
        avg_cost: f64,
    ) {
        let clean_sym = symbol.trim().to_uppercase();
        if clean_sym.is_empty() {
            return;
        }
        let clean_name = if name.trim().is_empty() {
            clean_sym.clone()
        } else {
            name.trim().to_string()
        };
        let item = StockItem {
            symbol: clean_sym,
            name: clean_name,
            market,
            shares: shares.max(0.0),
            avg_cost: avg_cost.max(0.0),
        };
        let key = item.key();
        if !self.config.watchlist.iter().any(|x| x.key() == key) {
            self.config.watchlist.push(item);
            self.save_and_sync();
        }
    }

    fn start_search(&mut self, ctx: &Context) {
        let query = self.add_symbol_input.trim().to_string();
        if query.is_empty() {
            return;
        }
        self.search_loading = true;
        self.search_error = None;
        let (tx, rx) = mpsc::channel();
        self.search_rx = Some(rx);
        let ctx_clone = ctx.clone();
        thread::spawn(move || {
            let res = search_symbols(&query);
            let _ = tx.send(res);
            ctx_clone.request_repaint();
        });
    }

    fn render_top_bar(&mut self, ui: &mut egui::Ui, ctx: &Context) {
        let mode = self.config.view_mode;
        let lang = self.lang();
        let status = current_market_status(self.config.include_us_extended);

        ui.horizontal(|ui| {
            let title_resp = ui.add(
                egui::Label::new(
                    RichText::new(if mode == ViewMode::Standard {
                        "Stock TW"
                    } else {
                        "📈"
                    })
                    .strong()
                    .size(14.0),
                )
                .sense(Sense::drag()),
            );
            if mode == ViewMode::FloatingWidget && title_resp.drag_started() {
                ctx.send_viewport_cmd(ViewportCommand::StartDrag);
            }

            if mode == ViewMode::Standard {
                let tw_dot = if status.tw_open {
                    Color32::from_rgb(52, 211, 130)
                } else {
                    Color32::from_rgb(130, 135, 145)
                };
                let us_dot = if status.us_open {
                    Color32::from_rgb(52, 211, 130)
                } else {
                    Color32::from_rgb(130, 135, 145)
                };
                let tw_tip = if status.tw_open {
                    lang.tw_open()
                } else {
                    lang.tw_closed()
                };
                let us_tip = match status.us_phase {
                    UsPhase::Regular => lang.us_open(),
                    UsPhase::Extended => lang.us_extended(),
                    UsPhase::Closed => lang.us_closed(),
                };
                ui.label(RichText::new("●").color(tw_dot).size(10.0))
                    .on_hover_text(tw_tip);
                ui.label(RichText::new("TW").size(11.0).weak());
                ui.label(RichText::new("●").color(us_dot).size(10.0))
                    .on_hover_text(us_tip);
                ui.label(RichText::new("US").size(11.0).weak());
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if mode == ViewMode::FloatingWidget {
                    if ui
                        .small_button("✕")
                        .on_hover_text(lang.tip_close())
                        .clicked()
                    {
                        ctx.send_viewport_cmd(ViewportCommand::Close);
                    }
                }

                if ui
                    .small_button("⚙")
                    .on_hover_text(lang.tip_settings())
                    .clicked()
                {
                    self.show_settings_dialog = !self.show_settings_dialog;
                }

                if ui.small_button("＋").on_hover_text(lang.tip_add()).clicked() {
                    self.show_add_dialog = !self.show_add_dialog;
                }

                if ui
                    .small_button("⟳")
                    .on_hover_text(lang.tip_refresh())
                    .clicked()
                {
                    let _ = self.worker.tx.send(WorkerCommand::ForceRefresh);
                    self.worker_status = WorkerStatus::Updating;
                }

                let pin_text = if self.config.always_on_top {
                    RichText::new("📌").color(Color32::from_rgb(250, 190, 60))
                } else {
                    RichText::new("📌").weak()
                };
                if ui
                    .small_button(pin_text)
                    .on_hover_text(lang.tip_pin())
                    .clicked()
                {
                    self.config.always_on_top = !self.config.always_on_top;
                    self.apply_viewport_for_mode(ctx, false);
                    self.save_and_sync();
                }

                for m in [
                    ViewMode::FloatingWidget,
                    ViewMode::Sidebar,
                    ViewMode::Standard,
                ] {
                    let selected = self.config.view_mode == m;
                    let label = match m {
                        ViewMode::Standard => lang.mode_standard(),
                        ViewMode::Sidebar => lang.mode_sidebar(),
                        ViewMode::FloatingWidget => lang.mode_floating(),
                    };
                    if ui
                        .selectable_label(selected, RichText::new(label).size(11.0))
                        .clicked()
                    {
                        self.switch_view_mode(ctx, m);
                    }
                }
            });
        });
    }

    fn render_filter_bar(&mut self, ui: &mut egui::Ui) {
        if self.config.view_mode != ViewMode::Standard {
            return;
        }
        let lang = self.lang();
        ui.horizontal(|ui| {
            for (f, label) in [
                (MarketFilter::All, lang.filter_all()),
                (MarketFilter::TwOnly, lang.filter_tw()),
                (MarketFilter::UsOnly, lang.filter_us()),
            ] {
                if ui
                    .selectable_label(self.config.market_filter == f, label)
                    .clicked()
                {
                    self.config.market_filter = f;
                    self.save_and_sync();
                }
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let mut sort_changed = false;
                egui::ComboBox::from_id_salt("sort_display_combo")
                    .width(116.0)
                    .selected_text(match self.config.sort_display {
                        SortDisplay::Custom => lang.sort_custom(),
                        SortDisplay::ChangeDesc => lang.sort_desc(),
                        SortDisplay::ChangeAsc => lang.sort_asc(),
                    })
                    .show_ui(ui, |ui| {
                        sort_changed |= ui
                            .selectable_value(
                                &mut self.config.sort_display,
                                SortDisplay::Custom,
                                lang.sort_custom(),
                            )
                            .changed();
                        sort_changed |= ui
                            .selectable_value(
                                &mut self.config.sort_display,
                                SortDisplay::ChangeDesc,
                                lang.sort_desc(),
                            )
                            .changed();
                        sort_changed |= ui
                            .selectable_value(
                                &mut self.config.sort_display,
                                SortDisplay::ChangeAsc,
                                lang.sort_asc(),
                            )
                            .changed();
                    });
                if sort_changed {
                    self.save_and_sync();
                }
            });
        });
    }

    fn render_portfolio_summary(&self, ui: &mut egui::Ui) -> bool {
        if self.config.view_mode != ViewMode::Standard {
            return false;
        }
        let mut tw_cost = 0.0;
        let mut tw_val = 0.0;
        let mut us_cost = 0.0;
        let mut us_val = 0.0;

        for item in &self.config.watchlist {
            if !item.has_position() {
                continue;
            }
            if let Some(q) = self.quotes.get(&item.key()) {
                let cost = item.shares * item.avg_cost;
                let val = item.shares * q.price;
                match item.market {
                    Market::Tw => {
                        tw_cost += cost;
                        tw_val += val;
                    }
                    Market::Us => {
                        us_cost += cost;
                        us_val += val;
                    }
                }
            }
        }

        if tw_cost <= 0.0 && us_cost <= 0.0 {
            return false;
        }

        let lang = self.lang();
        Frame::new()
            .fill(self.card_color())
            .corner_radius(CornerRadius::same(6))
            .inner_margin(Margin::symmetric(10, 5))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(lang.label_total_pnl()).strong().size(11.5));
                    if tw_cost > 0.0 {
                        let profit = tw_val - tw_cost;
                        let pct = (profit / tw_cost) * 100.0;
                        let sign = if profit > 0.0 { "+" } else { "" };
                        let col = self.price_color(Market::Tw, profit);
                        ui.label(
                            RichText::new(format!(
                                "TWD {sign}{} ({sign}{pct:.2}%)",
                                format_number(profit, 0)
                            ))
                            .color(col)
                            .strong()
                            .size(11.5),
                        );
                    }
                    if us_cost > 0.0 {
                        let profit = us_val - us_cost;
                        let pct = (profit / us_cost) * 100.0;
                        let sign = if profit > 0.0 { "+" } else { "" };
                        let col = self.price_color(Market::Us, profit);
                        ui.label(
                            RichText::new(format!(
                                "USD {sign}{} ({sign}{pct:.2}%)",
                                format_number(profit, 2)
                            ))
                            .color(col)
                            .strong()
                            .size(11.5),
                        );
                    }
                });
            });
        true
    }

    fn render_watchlist(&mut self, ui: &mut egui::Ui) {
        let mode = self.config.view_mode;
        let is_compact = mode != ViewMode::Standard;
        let lang = self.lang();

        let mut indices: Vec<usize> = self
            .config
            .watchlist
            .iter()
            .enumerate()
            .filter(|(_, item)| match self.config.market_filter {
                MarketFilter::All => true,
                MarketFilter::TwOnly => item.market == Market::Tw,
                MarketFilter::UsOnly => item.market == Market::Us,
            })
            .map(|(i, _)| i)
            .collect();

        match self.config.sort_display {
            SortDisplay::Custom => {}
            SortDisplay::ChangeDesc => {
                indices.sort_by(|&a, &b| {
                    let qa = self
                        .quotes
                        .get(&self.config.watchlist[a].key())
                        .map(|q| q.change_percent)
                        .unwrap_or(f64::NEG_INFINITY);
                    let qb = self
                        .quotes
                        .get(&self.config.watchlist[b].key())
                        .map(|q| q.change_percent)
                        .unwrap_or(f64::NEG_INFINITY);
                    qb.partial_cmp(&qa).unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            SortDisplay::ChangeAsc => {
                indices.sort_by(|&a, &b| {
                    let qa = self
                        .quotes
                        .get(&self.config.watchlist[a].key())
                        .map(|q| q.change_percent)
                        .unwrap_or(f64::INFINITY);
                    let qb = self
                        .quotes
                        .get(&self.config.watchlist[b].key())
                        .map(|q| q.change_percent)
                        .unwrap_or(f64::INFINITY);
                    qa.partial_cmp(&qb).unwrap_or(std::cmp::Ordering::Equal)
                });
            }
        }

        if indices.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(RichText::new(lang.empty_watchlist()).weak());
            });
            return;
        }

        let mut action_move: Option<(usize, usize)> = None;
        let mut action_pin_top: Option<usize> = None;
        let mut action_delete: Option<usize> = None;
        let mut action_edit_pos: Option<usize> = None;
        let allow_drag = self.config.sort_display == SortDisplay::Custom
            && self.config.market_filter == MarketFilter::All;

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = if is_compact { 4.0 } else { 6.0 };

                for &idx in &indices {
                    let item = self.config.watchlist[idx].clone();
                    let quote = self.quotes.get(&item.key()).cloned();
                    let card_bg = self.card_color();
                    let is_being_dragged = self.dragging_idx == Some(idx);

                    let stroke = if is_being_dragged {
                        Stroke::new(1.5_f32, Color32::from_rgb(90, 160, 255))
                    } else if self.config.dark_mode {
                        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 18))
                    } else {
                        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(0, 0, 0, 20))
                    };

                    let frame_resp = Frame::new()
                        .fill(card_bg)
                        .stroke(stroke)
                        .corner_radius(CornerRadius::same(6))
                        .inner_margin(Margin::symmetric(
                            if is_compact { 8 } else { 10 },
                            if is_compact { 5 } else { 7 },
                        ))
                        .show(ui, |ui| {
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    if allow_drag && !is_compact {
                                        let handle = ui.add(
                                            egui::Label::new(RichText::new("☰").weak().size(13.0))
                                                .sense(Sense::drag()),
                                        );
                                        if handle.drag_started() {
                                            self.dragging_idx = Some(idx);
                                        }
                                        handle.on_hover_text(lang.tip_drag_reorder());
                                    }

                                    ui.vertical(|ui| {
                                        ui.horizontal(|ui| {
                                            let badge_color = match item.market {
                                                Market::Tw => Color32::from_rgb(76, 154, 255),
                                                Market::Us => Color32::from_rgb(174, 128, 255),
                                            };
                                            if !is_compact {
                                                ui.label(
                                                    RichText::new(item.market.badge())
                                                        .color(badge_color)
                                                        .size(10.0)
                                                        .strong(),
                                                );
                                            }
                                            ui.label(
                                                RichText::new(&item.symbol)
                                                    .strong()
                                                    .size(if is_compact { 13.0 } else { 14.0 }),
                                            );
                                            if !is_compact {
                                                let pos_btn_text = if item.has_position() {
                                                    RichText::new(lang.btn_edit_pos_short())
                                                        .color(Color32::from_rgb(100, 195, 255))
                                                        .size(10.5)
                                                } else {
                                                    RichText::new(lang.btn_edit_pos_short())
                                                        .weak()
                                                        .size(10.5)
                                                };
                                                if ui
                                                    .small_button(pos_btn_text)
                                                    .on_hover_text(lang.tip_edit_position())
                                                    .clicked()
                                                {
                                                    action_edit_pos = Some(idx);
                                                }
                                            }
                                        });
                                        let name_display = quote
                                            .as_ref()
                                            .map(|q| q.display_name.as_str())
                                            .filter(|s| !s.is_empty())
                                            .unwrap_or(&item.name);
                                        let truncated: String =
                                            name_display.chars().take(12).collect();
                                        ui.label(
                                            RichText::new(truncated)
                                                .weak()
                                                .size(if is_compact { 10.5 } else { 11.5 }),
                                        );
                                    });

                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        if let Some(q) = &quote {
                                            let color = self.price_color(item.market, q.change);
                                            ui.vertical(|ui| {
                                                ui.with_layout(
                                                    Layout::right_to_left(Align::Min),
                                                    |ui| {
                                                        ui.label(
                                                            RichText::new(format!("{:.2}", q.price))
                                                                .color(color)
                                                                .strong()
                                                                .size(if is_compact {
                                                                    14.0
                                                                } else {
                                                                    15.5
                                                                }),
                                                        );
                                                    },
                                                );
                                                ui.with_layout(
                                                    Layout::right_to_left(Align::Min),
                                                    |ui| {
                                                        let sign =
                                                            if q.change > 0.0 { "+" } else { "" };
                                                        let detail = if is_compact {
                                                            format!("{sign}{:.2}%", q.change_percent)
                                                        } else {
                                                            format!(
                                                                "{sign}{:.2} ({sign}{:.2}%)",
                                                                q.change, q.change_percent
                                                            )
                                                        };
                                                        ui.label(
                                                            RichText::new(detail)
                                                                .color(color)
                                                                .size(if is_compact {
                                                                    11.0
                                                                } else {
                                                                    11.5
                                                                }),
                                                        );
                                                    },
                                                );
                                            });
                                        } else {
                                            ui.label(RichText::new("--").weak());
                                        }
                                    });
                                });

                                // Position & P&L sub-row when holdings are configured
                                if item.has_position() {
                                    if let Some(q) = &quote {
                                        if let Some((profit, pct)) = item.pnl(q.price) {
                                            let pnl_color = self.price_color(item.market, profit);
                                            let sign = if profit > 0.0 { "+" } else { "" };
                                            let dec = if item.market == Market::Tw { 0 } else { 2 };
                                            ui.add_space(2.0);
                                            ui.horizontal(|ui| {
                                                if !is_compact {
                                                    ui.label(
                                                        RichText::new(format!(
                                                            "{} {} @ {:.2}",
                                                            lang.label_holding_short(),
                                                            format_shares(item.shares),
                                                            item.avg_cost
                                                        ))
                                                        .weak()
                                                        .size(10.5),
                                                    );
                                                }
                                                ui.with_layout(
                                                    Layout::right_to_left(Align::Center),
                                                    |ui| {
                                                        let pnl_str = if is_compact {
                                                            format!(
                                                                "{} {sign}{} ({sign}{pct:.1}%)",
                                                                lang.label_pnl(),
                                                                format_number(profit, dec)
                                                            )
                                                        } else {
                                                            format!(
                                                                "{} {sign}{} ({sign}{pct:.2}%)",
                                                                lang.label_pnl(),
                                                                format_number(profit, dec)
                                                            )
                                                        };
                                                        ui.label(
                                                            RichText::new(pnl_str)
                                                                .color(pnl_color)
                                                                .size(10.5),
                                                        );
                                                    },
                                                );
                                            });
                                        }
                                    }
                                }
                            });
                        });

                    let row_resp = frame_resp.response.interact(Sense::click());

                    if row_resp.double_clicked() {
                        action_edit_pos = Some(idx);
                    }

                    if let Some(drag_from) = self.dragging_idx {
                        if drag_from != idx && row_resp.hovered() {
                            action_move = Some((drag_from, idx));
                        }
                    }

                    if let Some(q) = &quote {
                        row_resp.clone().on_hover_ui(|ui| {
                            ui.label(
                                RichText::new(format!("{} ({})", q.display_name, q.symbol))
                                    .strong(),
                            );
                            ui.separator();
                            ui.label(format!("{}{:.2}", lang.prev_close(), q.prev_close));
                            if let Some(o) = q.open {
                                ui.label(format!("{}{o:.2}", lang.open_price()));
                            }
                            if let (Some(h), Some(l)) = (q.high, q.low) {
                                ui.label(format!("{}{h:.2} / {l:.2}", lang.high_low()));
                            }
                            if let Some(v) = q.volume {
                                ui.label(format!("{}{v}", lang.volume()));
                            }
                            if item.has_position() {
                                ui.separator();
                                let dec = if item.market == Market::Tw { 0 } else { 2 };
                                let cost_basis = item.shares * item.avg_cost;
                                let mkt_val = item.shares * q.price;
                                ui.label(format!(
                                    "{}{} @ {:.2}",
                                    lang.label_shares(),
                                    format_shares(item.shares),
                                    item.avg_cost
                                ));
                                ui.label(format!(
                                    "{}{}",
                                    lang.label_cost_basis(),
                                    format_number(cost_basis, dec)
                                ));
                                ui.label(format!(
                                    "{}{}",
                                    lang.label_market_value(),
                                    format_number(mkt_val, dec)
                                ));
                                if let Some((profit, pct)) = item.pnl(q.price) {
                                    let sign = if profit > 0.0 { "+" } else { "" };
                                    ui.label(
                                        RichText::new(format!(
                                            "{}: {sign}{} ({sign}{pct:.2}%)",
                                            lang.label_pnl(),
                                            format_number(profit, dec)
                                        ))
                                        .color(self.price_color(item.market, profit))
                                        .strong(),
                                    );
                                }
                            }
                            ui.separator();
                            ui.label(
                                RichText::new(lang.source_updated(q.source, &q.updated_at))
                                    .weak()
                                    .size(11.0),
                            );
                        });
                    }

                    row_resp.context_menu(|ui| {
                        if ui.button(lang.ctx_edit_position()).clicked() {
                            action_edit_pos = Some(idx);
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button(lang.ctx_pin_top()).clicked() {
                            action_pin_top = Some(idx);
                            ui.close_menu();
                        }
                        if idx > 0 && ui.button(lang.ctx_move_up()).clicked() {
                            action_move = Some((idx, idx - 1));
                            ui.close_menu();
                        }
                        if idx + 1 < self.config.watchlist.len()
                            && ui.button(lang.ctx_move_down()).clicked()
                        {
                            action_move = Some((idx, idx + 1));
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui
                            .button(
                                RichText::new(lang.ctx_delete())
                                    .color(Color32::from_rgb(240, 90, 90)),
                            )
                            .clicked()
                        {
                            action_delete = Some(idx);
                            ui.close_menu();
                        }
                    });
                }
            });

        if self.dragging_idx.is_some() && !ui.input(|i| i.pointer.primary_down()) {
            self.dragging_idx = None;
        }

        if let Some(idx) = action_edit_pos {
            self.open_edit_position(idx);
        } else if let Some((from, to)) = action_move {
            if from < self.config.watchlist.len() && to < self.config.watchlist.len() {
                let item = self.config.watchlist.remove(from);
                self.config.watchlist.insert(to, item);
                if self.dragging_idx == Some(from) {
                    self.dragging_idx = Some(to);
                }
                self.save_and_sync();
            }
        } else if let Some(idx) = action_pin_top {
            if idx < self.config.watchlist.len() && idx > 0 {
                let item = self.config.watchlist.remove(idx);
                self.config.watchlist.insert(0, item);
                self.save_and_sync();
            }
        } else if let Some(idx) = action_delete {
            if idx < self.config.watchlist.len() {
                self.config.watchlist.remove(idx);
                if self.edit_pos_idx == Some(idx) {
                    self.edit_pos_idx = None;
                }
                self.save_and_sync();
            }
        }
    }

    fn render_edit_position_dialog(&mut self, ctx: &Context) {
        let Some(idx) = self.edit_pos_idx else {
            return;
        };
        let Some(item) = self.config.watchlist.get(idx).cloned() else {
            self.edit_pos_idx = None;
            return;
        };

        let lang = self.lang();
        let mut open = true;
        let mut save_clicked = false;
        let mut clear_clicked = false;

        egui::Window::new(format!(
            "{} - {} {}",
            lang.edit_pos_title(),
            item.symbol,
            item.name
        ))
        .id(Id::new("edit_position_window"))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(270.0)
        .show(ctx, |ui| {
            let mut enter_pressed = false;
            ui.horizontal(|ui| {
                ui.label(lang.label_shares());
                let r = ui.text_edit_singleline(&mut self.edit_shares_input);
                if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    enter_pressed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label(lang.label_avg_cost());
                let r = ui.text_edit_singleline(&mut self.edit_cost_input);
                if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    enter_pressed = true;
                }
            });

            let parsed_shares = self
                .edit_shares_input
                .trim()
                .replace(',', "")
                .parse::<f64>()
                .unwrap_or(0.0);
            let parsed_cost = self
                .edit_cost_input
                .trim()
                .replace(',', "")
                .parse::<f64>()
                .unwrap_or(0.0);
            if parsed_shares > 0.0 && parsed_cost > 0.0 {
                let dec = if item.market == Market::Tw { 0 } else { 2 };
                let cost_basis = parsed_shares * parsed_cost;
                ui.label(
                    RichText::new(format!(
                        "{}{}",
                        lang.label_cost_basis(),
                        format_number(cost_basis, dec)
                    ))
                    .weak()
                    .size(11.5),
                );
                if let Some(q) = self.quotes.get(&item.key()) {
                    let profit = parsed_shares * q.price - cost_basis;
                    let pct = (profit / cost_basis) * 100.0;
                    let sign = if profit > 0.0 { "+" } else { "" };
                    ui.label(
                        RichText::new(format!(
                            "{}: {sign}{} ({sign}{pct:.2}%)",
                            lang.label_pnl(),
                            format_number(profit, dec)
                        ))
                        .color(self.price_color(item.market, profit))
                        .strong()
                        .size(11.5),
                    );
                }
            }

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button(lang.btn_save()).clicked() || enter_pressed {
                    save_clicked = true;
                }
                if ui.button(lang.btn_clear_pos()).clicked() {
                    clear_clicked = true;
                }
            });
        });

        if save_clicked {
            let shares = self
                .edit_shares_input
                .trim()
                .replace(',', "")
                .parse::<f64>()
                .unwrap_or(0.0)
                .max(0.0);
            let avg_cost = self
                .edit_cost_input
                .trim()
                .replace(',', "")
                .parse::<f64>()
                .unwrap_or(0.0)
                .max(0.0);
            if let Some(target) = self.config.watchlist.get_mut(idx) {
                target.shares = shares;
                target.avg_cost = avg_cost;
            }
            self.save_and_sync();
            self.edit_pos_idx = None;
        } else if clear_clicked {
            if let Some(target) = self.config.watchlist.get_mut(idx) {
                target.shares = 0.0;
                target.avg_cost = 0.0;
            }
            self.save_and_sync();
            self.edit_pos_idx = None;
        } else if !open {
            self.edit_pos_idx = None;
        }
    }

    fn render_add_dialog(&mut self, ctx: &Context) {
        if !self.show_add_dialog {
            return;
        }
        let lang = self.lang();
        let mut open = self.show_add_dialog;
        egui::Window::new(lang.add_title())
            .id(Id::new("add_stock_window"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(310.0)
            .show(ctx, |ui| {
                ui.label(lang.add_prompt());
                ui.horizontal(|ui| {
                    let resp = ui.text_edit_singleline(&mut self.add_symbol_input);
                    if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.start_search(ctx);
                    }
                    if ui.button(lang.btn_search()).clicked() {
                        self.start_search(ctx);
                    }
                });

                let guessed = guess_market(&self.add_symbol_input);
                let selected_market = self.add_market_override.unwrap_or(guessed);
                ui.horizontal(|ui| {
                    ui.label(lang.label_market());
                    if ui
                        .selectable_label(selected_market == Market::Tw, lang.market_tw())
                        .clicked()
                    {
                        self.add_market_override = Some(Market::Tw);
                    }
                    if ui
                        .selectable_label(selected_market == Market::Us, lang.market_us())
                        .clicked()
                    {
                        self.add_market_override = Some(Market::Us);
                    }
                });

                ui.horizontal(|ui| {
                    ui.label(lang.label_name_opt());
                    ui.text_edit_singleline(&mut self.add_name_input);
                });

                ui.horizontal(|ui| {
                    ui.label(lang.label_shares());
                    ui.add(
                        egui::TextEdit::singleline(&mut self.add_shares_input).desired_width(80.0),
                    );
                    ui.label(lang.label_avg_cost());
                    ui.add(
                        egui::TextEdit::singleline(&mut self.add_cost_input).desired_width(80.0),
                    );
                });

                let shares_val = self
                    .add_shares_input
                    .trim()
                    .replace(',', "")
                    .parse::<f64>()
                    .unwrap_or(0.0);
                let cost_val = self
                    .add_cost_input
                    .trim()
                    .replace(',', "")
                    .parse::<f64>()
                    .unwrap_or(0.0);

                if ui.button(lang.btn_add_direct()).clicked()
                    && !self.add_symbol_input.trim().is_empty()
                {
                    let sym = self.add_symbol_input.clone();
                    let name = self.add_name_input.clone();
                    self.add_stock_item(sym, name, selected_market, shares_val, cost_val);
                    self.add_symbol_input.clear();
                    self.add_name_input.clear();
                    self.add_shares_input.clear();
                    self.add_cost_input.clear();
                    self.add_market_override = None;
                }

                if self.search_loading {
                    ui.separator();
                    ui.label(lang.searching());
                } else if let Some(err) = &self.search_error {
                    ui.separator();
                    ui.label(RichText::new(err).color(Color32::from_rgb(240, 100, 100)));
                } else if !self.search_results.is_empty() {
                    ui.separator();
                    ui.label(
                        RichText::new(lang.search_results_header())
                            .strong()
                            .size(12.0),
                    );
                    let results = self.search_results.clone();
                    egui::ScrollArea::vertical().max_height(180.0).show(ui, |ui| {
                        for r in results {
                            let btn_text = format!(
                                "[{}] {} - {} ({})",
                                r.market.badge(),
                                r.symbol,
                                r.name,
                                r.exchange
                            );
                            if ui.button(btn_text).clicked() {
                                self.add_stock_item(r.symbol, r.name, r.market, shares_val, cost_val);
                            }
                        }
                    });
                }
            });
        self.show_add_dialog = open;
    }

    fn render_settings_dialog(&mut self, ctx: &Context) {
        if !self.show_settings_dialog {
            return;
        }
        let lang = self.lang();
        let mut open = self.show_settings_dialog;
        let mut changed = false;
        let mut viewport_changed = false;

        egui::Window::new(lang.settings_title())
            .id(Id::new("settings_window"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(310.0)
            .show(ctx, |ui| {
                ui.label(RichText::new(lang.sec_refresh()).strong());
                egui::ComboBox::from_id_salt("refresh_mode_combo")
                    .selected_text(lang.refresh_label(self.config.refresh_mode))
                    .show_ui(ui, |ui| {
                        for mode in RefreshMode::all() {
                            if ui
                                .selectable_value(
                                    &mut self.config.refresh_mode,
                                    mode,
                                    lang.refresh_label(mode),
                                )
                                .changed()
                            {
                                changed = true;
                            }
                        }
                    });

                if ui
                    .checkbox(&mut self.config.smart_market_hours, lang.chk_smart_hours())
                    .changed()
                {
                    changed = true;
                }
                if ui
                    .checkbox(&mut self.config.pause_when_minimized, lang.chk_pause_min())
                    .changed()
                {
                    changed = true;
                }
                if ui
                    .checkbox(&mut self.config.include_us_extended, lang.chk_us_ext())
                    .changed()
                {
                    changed = true;
                }

                ui.separator();
                ui.label(RichText::new(lang.sec_appearance()).strong());

                ui.horizontal(|ui| {
                    ui.label(lang.label_language());
                    egui::ComboBox::from_id_salt("language_combo")
                        .selected_text(lang.lang_setting_label(self.config.language))
                        .show_ui(ui, |ui| {
                            for ls in [
                                LanguageSetting::Auto,
                                LanguageSetting::ZhTw,
                                LanguageSetting::En,
                            ] {
                                if ui
                                    .selectable_value(
                                        &mut self.config.language,
                                        ls,
                                        lang.lang_setting_label(ls),
                                    )
                                    .changed()
                                {
                                    changed = true;
                                }
                            }
                        });
                });

                egui::ComboBox::from_id_salt("color_scheme_combo")
                    .selected_text(lang.color_scheme_label(self.config.color_scheme))
                    .show_ui(ui, |ui| {
                        for cs in [
                            ColorScheme::TwRedUp,
                            ColorScheme::UsGreenUp,
                            ColorScheme::AutoByMarket,
                        ] {
                            if ui
                                .selectable_value(
                                    &mut self.config.color_scheme,
                                    cs,
                                    lang.color_scheme_label(cs),
                                )
                                .changed()
                            {
                                changed = true;
                            }
                        }
                    });

                if ui
                    .add(
                        egui::Slider::new(&mut self.config.opacity, 0.25..=1.0)
                            .text(lang.slider_opacity())
                            .fixed_decimals(2),
                    )
                    .changed()
                {
                    changed = true;
                }

                if ui
                    .checkbox(&mut self.config.dark_mode, lang.chk_dark())
                    .changed()
                {
                    changed = true;
                }

                if ui
                    .checkbox(&mut self.config.always_on_top, lang.chk_always_on_top())
                    .changed()
                {
                    changed = true;
                    viewport_changed = true;
                }

                ui.separator();
                ui.label(
                    RichText::new(lang.dpapi_encrypted_note())
                        .color(Color32::from_rgb(100, 195, 145))
                        .size(10.5),
                );
                let path = AppConfig::config_path();
                ui.label(
                    RichText::new(format!("{}{}", lang.config_file_label(), path.display()))
                        .weak()
                        .size(10.0),
                );
                ui.add_space(4.0);
                let mut trigger_check = false;
                let mut open_download_url: Option<String> = None;
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("Stock TW v{}", env!("CARGO_PKG_VERSION")))
                            .weak()
                            .size(11.0),
                    );
                    if ui
                        .add_enabled(!self.update_checking, egui::Button::new(lang.btn_check_update()))
                        .clicked()
                    {
                        trigger_check = true;
                    }
                });
                if self.update_checking {
                    ui.label(RichText::new(lang.update_checking()).weak().size(11.0));
                } else if let Some(res) = &self.update_result {
                    match res {
                        Ok(info) if info.has_update => {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(lang.update_available(&info.latest_version))
                                        .color(Color32::from_rgb(250, 190, 60))
                                        .strong()
                                        .size(11.0),
                                );
                                if ui.small_button(lang.btn_download_update()).clicked() {
                                    open_download_url = Some(info.download_url.clone());
                                }
                            });
                        }
                        Ok(info) => {
                            ui.label(
                                RichText::new(lang.update_up_to_date(&info.latest_version))
                                    .color(Color32::from_rgb(100, 195, 145))
                                    .size(11.0),
                            );
                        }
                        Err(e) => {
                            ui.label(
                                RichText::new(lang.update_failed(e))
                                    .color(Color32::from_rgb(245, 130, 100))
                                    .size(10.5),
                            );
                        }
                    }
                }
                if trigger_check {
                    self.start_update_check(ctx);
                }
                if let Some(url) = open_download_url {
                    ctx.open_url(OpenUrl::new_tab(url));
                }
            });

        self.show_settings_dialog = open;
        if viewport_changed {
            self.apply_viewport_for_mode(ctx, false);
        }
        if changed {
            self.save_and_sync();
        }
    }

    fn render_bottom_bar(&self, ui: &mut egui::Ui) {
        if self.config.view_mode == ViewMode::FloatingWidget {
            return;
        }
        let lang = self.lang();
        ui.horizontal(|ui| {
            if let Some(err) = &self.error_text {
                ui.label(
                    RichText::new(err)
                        .color(Color32::from_rgb(245, 130, 100))
                        .size(10.5),
                );
            } else {
                let status_str = match self.worker_status {
                    WorkerStatus::Initializing => lang.status_init(),
                    WorkerStatus::Updating => lang.status_updating(),
                    WorkerStatus::Updated => lang.status_updated(),
                    WorkerStatus::IdleClosed => lang.status_idle_closed(),
                };
                ui.label(RichText::new(status_str).weak().size(10.5));
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new(lang.refresh_label(self.config.refresh_mode))
                        .weak()
                        .size(10.5),
                );
            });
        });
    }
}

impl eframe::App for StockApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        Color32::TRANSPARENT.to_normalized_gamma_f32()
    }

    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        if !self.applied_initial_viewport {
            self.applied_initial_viewport = true;
            if self.config.always_on_top {
                ctx.send_viewport_cmd(ViewportCommand::WindowLevel(WindowLevel::AlwaysOnTop));
            }
            ctx.send_viewport_cmd(ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd(ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(ViewportCommand::Focus);
        }

        let is_minimized = ctx.input(|i| i.viewport().minimized.unwrap_or(false));
        if is_minimized != self.last_minimized {
            self.last_minimized = is_minimized;
            let _ = self
                .worker
                .tx
                .send(WorkerCommand::SetMinimized(is_minimized));
        }

        self.drain_worker_events();
        self.capture_window_geometry(ctx);

        if self.config.dark_mode {
            ctx.set_visuals(egui::Visuals::dark());
        } else {
            ctx.set_visuals(egui::Visuals::light());
        }

        let rounding = if self.config.view_mode == ViewMode::FloatingWidget {
            CornerRadius::same(10)
        } else {
            CornerRadius::ZERO
        };

        let outer_frame = Frame::new()
            .fill(self.bg_color())
            .corner_radius(rounding)
            .inner_margin(Margin::same(8));

        egui::CentralPanel::default()
            .frame(outer_frame)
            .show(ctx, |ui| {
                self.render_top_bar(ui, ctx);
                ui.add_space(4.0);
                self.render_filter_bar(ui);
                if self.config.view_mode == ViewMode::Standard {
                    ui.add_space(4.0);
                    if self.render_portfolio_summary(ui) {
                        ui.add_space(4.0);
                    }
                }
                let bottom_reserve = if self.config.view_mode == ViewMode::FloatingWidget {
                    0.0
                } else {
                    20.0
                };
                let avail = (ui.available_height() - bottom_reserve).max(60.0);
                ui.allocate_ui(Vec2::new(ui.available_width(), avail), |ui| {
                    self.render_watchlist(ui);
                });
                if bottom_reserve > 0.0 {
                    ui.separator();
                    self.render_bottom_bar(ui);
                }
            });

        self.render_add_dialog(ctx);
        self.render_edit_position_dialog(ctx);
        self.render_settings_dialog(ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        let _ = self.worker.tx.send(WorkerCommand::Shutdown);
        self.config.save();
    }
}
