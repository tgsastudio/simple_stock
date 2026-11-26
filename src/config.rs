//! 應用程式設定與自選股資料模型
//! 負責管理自選股清單、持股數量/成本、刷新模式、視窗幾何位置與加密讀寫。

use crate::storage;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// 市場分類（台股 / 美股）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Market {
    Tw,
    Us,
}

impl Market {
    pub fn badge(self) -> &'static str {
        match self {
            Market::Tw => "TW",
            Market::Us => "US",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StockItem {
    pub symbol: String,
    pub name: String,
    pub market: Market,
    #[serde(default)]
    pub shares: f64,
    #[serde(default)]
    pub avg_cost: f64,
}

impl StockItem {
    pub fn key(&self) -> String {
        format!("{:?}:{}", self.market, self.symbol.to_uppercase())
    }

    pub fn has_position(&self) -> bool {
        self.shares > 0.0 && self.avg_cost > 0.0
    }

    /// Returns (profit_amount, profit_percent) given current price.
    pub fn pnl(&self, current_price: f64) -> Option<(f64, f64)> {
        if !self.has_position() || current_price <= 0.0 {
            return None;
        }
        let cost_basis = self.shares * self.avg_cost;
        let market_value = self.shares * current_price;
        let profit = market_value - cost_basis;
        let pct = (profit / cost_basis) * 100.0;
        Some((profit, pct))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RefreshMode {
    Fast5s,
    Balanced15s,
    Eco30s,
    UltraEco60s,
    Manual,
}

impl RefreshMode {
    pub fn interval_secs(self) -> Option<u64> {
        match self {
            RefreshMode::Fast5s => Some(5),
            RefreshMode::Balanced15s => Some(15),
            RefreshMode::Eco30s => Some(30),
            RefreshMode::UltraEco60s => Some(60),
            RefreshMode::Manual => None,
        }
    }

    pub fn all() -> [RefreshMode; 5] {
        [
            RefreshMode::Fast5s,
            RefreshMode::Balanced15s,
            RefreshMode::Eco30s,
            RefreshMode::UltraEco60s,
            RefreshMode::Manual,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewMode {
    Standard,
    Sidebar,
    FloatingWidget,
}

impl ViewMode {
    pub fn default_size(self) -> [f32; 2] {
        match self {
            ViewMode::Standard => [480.0, 560.0],
            ViewMode::Sidebar => [250.0, 520.0],
            ViewMode::FloatingWidget => [270.0, 340.0],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorScheme {
    TwRedUp,
    UsGreenUp,
    AutoByMarket,
}

impl ColorScheme {
    pub fn is_red_up(self, market: Market) -> bool {
        match self {
            ColorScheme::TwRedUp => true,
            ColorScheme::UsGreenUp => false,
            ColorScheme::AutoByMarket => market == Market::Tw,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarketFilter {
    All,
    TwOnly,
    UsOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortDisplay {
    Custom,
    ChangeDesc,
    ChangeAsc,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WindowRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LanguageSetting {
    Auto,
    ZhTw,
    En,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub watchlist: Vec<StockItem>,
    pub refresh_mode: RefreshMode,
    pub smart_market_hours: bool,
    pub pause_when_minimized: bool,
    pub include_us_extended: bool,
    pub view_mode: ViewMode,
    pub color_scheme: ColorScheme,
    pub always_on_top: bool,
    pub opacity: f32,
    pub dark_mode: bool,
    pub market_filter: MarketFilter,
    pub sort_display: SortDisplay,
    pub language: LanguageSetting,
    pub window_standard: Option<WindowRect>,
    pub window_sidebar: Option<WindowRect>,
    pub window_floating: Option<WindowRect>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            watchlist: vec![
                StockItem {
                    symbol: "2330".to_string(),
                    name: "台積電".to_string(),
                    market: Market::Tw,
                    shares: 0.0,
                    avg_cost: 0.0,
                },
                StockItem {
                    symbol: "0050".to_string(),
                    name: "元大台灣50".to_string(),
                    market: Market::Tw,
                    shares: 0.0,
                    avg_cost: 0.0,
                },
                StockItem {
                    symbol: "2317".to_string(),
                    name: "鴻海".to_string(),
                    market: Market::Tw,
                    shares: 0.0,
                    avg_cost: 0.0,
                },
                StockItem {
                    symbol: "NVDA".to_string(),
                    name: "NVIDIA".to_string(),
                    market: Market::Us,
                    shares: 0.0,
                    avg_cost: 0.0,
                },
                StockItem {
                    symbol: "AAPL".to_string(),
                    name: "Apple".to_string(),
                    market: Market::Us,
                    shares: 0.0,
                    avg_cost: 0.0,
                },
                StockItem {
                    symbol: "TSLA".to_string(),
                    name: "Tesla".to_string(),
                    market: Market::Us,
                    shares: 0.0,
                    avg_cost: 0.0,
                },
            ],
            refresh_mode: RefreshMode::Balanced15s,
            smart_market_hours: true,
            pause_when_minimized: true,
            include_us_extended: true,
            view_mode: ViewMode::Standard,
            color_scheme: ColorScheme::TwRedUp,
            always_on_top: false,
            opacity: 0.92,
            dark_mode: true,
            market_filter: MarketFilter::All,
            sort_display: SortDisplay::Custom,
            language: LanguageSetting::Auto,
            window_standard: None,
            window_sidebar: None,
            window_floating: None,
        }
    }
}

impl AppConfig {
    pub fn window_rect_for(&self, mode: ViewMode) -> Option<WindowRect> {
        match mode {
            ViewMode::Standard => self.window_standard,
            ViewMode::Sidebar => self.window_sidebar,
            ViewMode::FloatingWidget => self.window_floating,
        }
    }

    pub fn set_window_rect_for(&mut self, mode: ViewMode, rect: WindowRect) {
        match mode {
            ViewMode::Standard => self.window_standard = Some(rect),
            ViewMode::Sidebar => self.window_sidebar = Some(rect),
            ViewMode::FloatingWidget => self.window_floating = Some(rect),
        }
    }
    pub fn config_path() -> PathBuf {
        // Portable mode: if config.dat or legacy config.json exists next to the executable, use it.
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let portable_dat = dir.join("config.dat");
                let portable_json = dir.join("config.json");
                if portable_dat.exists() || portable_json.exists() {
                    return portable_dat;
                }
            }
        }
        if let Some(proj) = directories::ProjectDirs::from("com", "tgsastudio", "simple_stock") {
            return proj.config_dir().join("config.dat");
        }
        PathBuf::from("config.dat")
    }

    pub fn load() -> Self {
        let dat_path = Self::config_path();
        if dat_path.exists() {
            if let Ok(bytes) = storage::read_encrypted(&dat_path) {
                if let Ok(cfg) = serde_json::from_slice::<AppConfig>(&bytes) {
                    return cfg;
                }
            }
        }

        // Seamless migration from legacy plaintext config.json -> encrypted config.dat
        let legacy_json = dat_path.with_extension("json");
        if legacy_json.exists() {
            if let Ok(text) = fs::read_to_string(&legacy_json) {
                if let Ok(cfg) = serde_json::from_str::<AppConfig>(&text) {
                    cfg.save();
                    let _ = fs::remove_file(&legacy_json);
                    return cfg;
                }
            }
        }

        Self::default()
    }

    pub fn save(&self) {
        let path = Self::config_path();
        if let Ok(bytes) = serde_json::to_vec(self) {
            let _ = storage::write_encrypted(&path, &bytes);
        }
    }
}
