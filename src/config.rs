use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StockItem {
    pub symbol: String,
    pub name: String,
    pub market: Market,
}

impl StockItem {
    pub fn key(&self) -> String {
        format!("{:?}:{}", self.market, self.symbol.to_uppercase())
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
    pub language: LanguageSetting,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            watchlist: vec![
                StockItem {
                    symbol: "2330".to_string(),
                    name: "台積電".to_string(),
                    market: Market::Tw,
                },
                StockItem {
                    symbol: "0050".to_string(),
                    name: "元大台灣50".to_string(),
                    market: Market::Tw,
                },
                StockItem {
                    symbol: "2317".to_string(),
                    name: "鴻海".to_string(),
                    market: Market::Tw,
                },
                StockItem {
                    symbol: "NVDA".to_string(),
                    name: "NVIDIA".to_string(),
                    market: Market::Us,
                },
                StockItem {
                    symbol: "AAPL".to_string(),
                    name: "Apple".to_string(),
                    market: Market::Us,
                },
                StockItem {
                    symbol: "TSLA".to_string(),
                    name: "Tesla".to_string(),
                    market: Market::Us,
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
            language: LanguageSetting::Auto,
        }
    }
}

impl AppConfig {
    pub fn config_path() -> PathBuf {
        // Portable mode: if config.json exists next to the executable, use it.
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let portable = dir.join("config.json");
                if portable.exists() {
                    return portable;
                }
            }
        }
        if let Some(proj) = directories::ProjectDirs::from("com", "tgsastudio", "simple_stock") {
            return proj.config_dir().join("config.json");
        }
        PathBuf::from("config.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if let Ok(text) = fs::read_to_string(&path) {
            if let Ok(cfg) = serde_json::from_str::<AppConfig>(&text) {
                return cfg;
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, json);
        }
    }
}
