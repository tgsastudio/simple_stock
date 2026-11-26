use crate::config::{ColorScheme, LanguageSetting, RefreshMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    ZhTw,
    En,
}

#[cfg(target_os = "windows")]
unsafe extern "system" {
    fn GetUserDefaultUILanguage() -> u16;
}

/// Detect the OS UI language (Chinese => ZhTw, otherwise => En).
pub fn detect_os_lang() -> Lang {
    #[cfg(target_os = "windows")]
    {
        let lang_id = unsafe { GetUserDefaultUILanguage() };
        let primary = lang_id & 0x03FF;
        // 0x04 = LANG_CHINESE (zh-TW, zh-HK, zh-MO, zh-CN, etc.)
        if primary == 0x04 {
            return Lang::ZhTw;
        }
        Lang::En
    }
    #[cfg(not(target_os = "windows"))]
    {
        Lang::En
    }
}

pub fn resolve_lang(setting: LanguageSetting) -> Lang {
    match setting {
        LanguageSetting::Auto => detect_os_lang(),
        LanguageSetting::ZhTw => Lang::ZhTw,
        LanguageSetting::En => Lang::En,
    }
}

impl Lang {
    pub fn is_zh(self) -> bool {
        self == Lang::ZhTw
    }

    pub fn tw_open(self) -> &'static str {
        if self.is_zh() { "台股盤中" } else { "TW Open" }
    }
    pub fn tw_closed(self) -> &'static str {
        if self.is_zh() { "台股休市" } else { "TW Closed" }
    }
    pub fn us_open(self) -> &'static str {
        if self.is_zh() { "美股盤中" } else { "US Open" }
    }
    pub fn us_extended(self) -> &'static str {
        if self.is_zh() { "美股盤前後" } else { "US Extended" }
    }
    pub fn us_closed(self) -> &'static str {
        if self.is_zh() { "美股休市" } else { "US Closed" }
    }

    pub fn mode_standard(self) -> &'static str {
        if self.is_zh() { "標準" } else { "Std" }
    }
    pub fn mode_sidebar(self) -> &'static str {
        if self.is_zh() { "側邊" } else { "Side" }
    }
    pub fn mode_floating(self) -> &'static str {
        if self.is_zh() { "浮動" } else { "Float" }
    }

    pub fn tip_close(self) -> &'static str {
        if self.is_zh() { "關閉" } else { "Close" }
    }
    pub fn tip_settings(self) -> &'static str {
        if self.is_zh() { "設定" } else { "Settings" }
    }
    pub fn tip_add(self) -> &'static str {
        if self.is_zh() { "新增自選股" } else { "Add Stock" }
    }
    pub fn tip_refresh(self) -> &'static str {
        if self.is_zh() { "立即刷新" } else { "Refresh Now" }
    }
    pub fn tip_pin(self) -> &'static str {
        if self.is_zh() { "切換視窗置頂" } else { "Toggle Always on Top" }
    }

    pub fn filter_all(self) -> &'static str {
        if self.is_zh() { "全部" } else { "All" }
    }
    pub fn filter_tw(self) -> &'static str {
        if self.is_zh() { "台股" } else { "TW" }
    }
    pub fn filter_us(self) -> &'static str {
        if self.is_zh() { "美股" } else { "US" }
    }

    pub fn sort_custom(self) -> &'static str {
        if self.is_zh() { "自訂順序" } else { "Custom Order" }
    }
    pub fn sort_desc(self) -> &'static str {
        if self.is_zh() { "漲幅高→低" } else { "Change High→Low" }
    }
    pub fn sort_asc(self) -> &'static str {
        if self.is_zh() { "漲幅低→高" } else { "Change Low→High" }
    }

    pub fn empty_watchlist(self) -> &'static str {
        if self.is_zh() {
            "尚未設定自選股，請點擊右上角 ＋ 新增"
        } else {
            "Watchlist is empty. Click ＋ to add stocks."
        }
    }
    pub fn tip_drag_reorder(self) -> &'static str {
        if self.is_zh() { "拖曳調整排序" } else { "Drag to reorder" }
    }

    pub fn prev_close(self) -> &'static str {
        if self.is_zh() { "昨收：" } else { "Prev Close: " }
    }
    pub fn open_price(self) -> &'static str {
        if self.is_zh() { "開盤：" } else { "Open: " }
    }
    pub fn high_low(self) -> &'static str {
        if self.is_zh() { "最高 / 最低：" } else { "High / Low: " }
    }
    pub fn volume(self) -> &'static str {
        if self.is_zh() { "成交量：" } else { "Volume: " }
    }
    pub fn source_updated(self, src: &str, time: &str) -> String {
        if self.is_zh() {
            format!("來源：{src} ・ 更新：{time}")
        } else {
            format!("Source: {src} ・ Updated: {time}")
        }
    }

    pub fn ctx_pin_top(self) -> &'static str {
        if self.is_zh() { "移至最頂" } else { "Move to Top" }
    }
    pub fn ctx_move_up(self) -> &'static str {
        if self.is_zh() { "↑ 上移一格" } else { "↑ Move Up" }
    }
    pub fn ctx_move_down(self) -> &'static str {
        if self.is_zh() { "↓ 下移一格" } else { "↓ Move Down" }
    }
    pub fn ctx_delete(self) -> &'static str {
        if self.is_zh() { "移除自選股" } else { "Remove" }
    }

    pub fn add_title(self) -> &'static str {
        if self.is_zh() { "新增自選股" } else { "Add Stock" }
    }
    pub fn add_prompt(self) -> &'static str {
        if self.is_zh() {
            "輸入台股代碼 (如 2330, 0050) 或美股代碼 (如 AAPL, NVDA)："
        } else {
            "Enter TW code (e.g. 2330, 0050) or US ticker (e.g. AAPL, NVDA):"
        }
    }
    pub fn btn_search(self) -> &'static str {
        if self.is_zh() { "線上搜尋" } else { "Search" }
    }
    pub fn label_market(self) -> &'static str {
        if self.is_zh() { "市場：" } else { "Market:" }
    }
    pub fn market_tw(self) -> &'static str {
        if self.is_zh() { "台股 (TW)" } else { "Taiwan (TW)" }
    }
    pub fn market_us(self) -> &'static str {
        if self.is_zh() { "美股 (US)" } else { "US (US)" }
    }
    pub fn label_name_opt(self) -> &'static str {
        if self.is_zh() { "名稱 (可留空自動抓)：" } else { "Name (optional):" }
    }
    pub fn btn_add_direct(self) -> &'static str {
        if self.is_zh() { "直接加入自選股" } else { "Add to Watchlist" }
    }
    pub fn searching(self) -> &'static str {
        if self.is_zh() { "搜尋中..." } else { "Searching..." }
    }
    pub fn search_results_header(self) -> &'static str {
        if self.is_zh() { "搜尋結果（點擊加入）：" } else { "Results (click to add):" }
    }

    pub fn settings_title(self) -> &'static str {
        if self.is_zh() { "設定" } else { "Settings" }
    }
    pub fn sec_refresh(self) -> &'static str {
        if self.is_zh() { "刷新頻率與省電" } else { "Refresh & Power Saving" }
    }
    pub fn chk_smart_hours(self) -> &'static str {
        if self.is_zh() {
            "開盤時段感知（休市自動停止輪詢）"
        } else {
            "Market-hours aware (pause when closed)"
        }
    }
    pub fn chk_pause_min(self) -> &'static str {
        if self.is_zh() {
            "視窗最小化時暫停刷新"
        } else {
            "Pause refresh when minimized"
        }
    }
    pub fn chk_us_ext(self) -> &'static str {
        if self.is_zh() {
            "包含美股盤前/盤後時段"
        } else {
            "Include US pre/post-market hours"
        }
    }
    pub fn sec_appearance(self) -> &'static str {
        if self.is_zh() { "外觀、配色與語言" } else { "Appearance & Language" }
    }
    pub fn label_language(self) -> &'static str {
        if self.is_zh() { "介面語言" } else { "Language" }
    }
    pub fn slider_opacity(self) -> &'static str {
        if self.is_zh() { "視窗透明度" } else { "Opacity" }
    }
    pub fn chk_dark(self) -> &'static str {
        if self.is_zh() { "深色主題" } else { "Dark Theme" }
    }
    pub fn chk_always_on_top(self) -> &'static str {
        if self.is_zh() { "視窗置頂 (Always on Top)" } else { "Always on Top" }
    }
    pub fn config_file_label(self) -> &'static str {
        if self.is_zh() { "設定檔：" } else { "Config: " }
    }

    pub fn status_init(self) -> &'static str {
        if self.is_zh() { "初始化中..." } else { "Initializing..." }
    }
    pub fn status_updating(self) -> &'static str {
        if self.is_zh() { "更新中..." } else { "Updating..." }
    }
    pub fn status_updated(self) -> &'static str {
        if self.is_zh() { "已更新" } else { "Updated" }
    }
    pub fn status_idle_closed(self) -> &'static str {
        if self.is_zh() {
            "休市待機中 (省電暫停)"
        } else {
            "Market closed (eco standby)"
        }
    }

    pub fn refresh_label(self, mode: RefreshMode) -> &'static str {
        match (self, mode) {
            (Lang::ZhTw, RefreshMode::Fast5s) => "5 秒 (極速)",
            (Lang::ZhTw, RefreshMode::Balanced15s) => "15 秒 (均衡)",
            (Lang::ZhTw, RefreshMode::Eco30s) => "30 秒 (省電)",
            (Lang::ZhTw, RefreshMode::UltraEco60s) => "60 秒 (超省電)",
            (Lang::ZhTw, RefreshMode::Manual) => "手動刷新",
            (Lang::En, RefreshMode::Fast5s) => "5s (Fast)",
            (Lang::En, RefreshMode::Balanced15s) => "15s (Balanced)",
            (Lang::En, RefreshMode::Eco30s) => "30s (Eco)",
            (Lang::En, RefreshMode::UltraEco60s) => "60s (Ultra Eco)",
            (Lang::En, RefreshMode::Manual) => "Manual",
        }
    }

    pub fn color_scheme_label(self, cs: ColorScheme) -> &'static str {
        match (self, cs) {
            (Lang::ZhTw, ColorScheme::TwRedUp) => "紅漲綠跌 (台股習慣)",
            (Lang::ZhTw, ColorScheme::UsGreenUp) => "綠漲紅跌 (美股習慣)",
            (Lang::ZhTw, ColorScheme::AutoByMarket) => "依市場自動配色",
            (Lang::En, ColorScheme::TwRedUp) => "Red Up / Green Down (TW)",
            (Lang::En, ColorScheme::UsGreenUp) => "Green Up / Red Down (US)",
            (Lang::En, ColorScheme::AutoByMarket) => "Auto by Market",
        }
    }

    pub fn lang_setting_label(self, ls: LanguageSetting) -> &'static str {
        match (self, ls) {
            (Lang::ZhTw, LanguageSetting::Auto) => "跟隨作業系統 (Auto)",
            (Lang::En, LanguageSetting::Auto) => "Follow System (Auto)",
            (_, LanguageSetting::ZhTw) => "繁體中文",
            (_, LanguageSetting::En) => "English",
        }
    }

    // Position & P&L labels
    pub fn ctx_edit_position(self) -> &'static str {
        if self.is_zh() { "編輯持股 (持有量/成本)" } else { "Edit Position (Qty/Cost)" }
    }
    pub fn btn_edit_pos_short(self) -> &'static str {
        if self.is_zh() { "持股" } else { "Pos" }
    }
    pub fn tip_edit_position(self) -> &'static str {
        if self.is_zh() { "編輯持有數量與成本（也可雙擊卡片）" } else { "Edit shares & avg cost (or double-click card)" }
    }
    pub fn edit_pos_title(self) -> &'static str {
        if self.is_zh() { "編輯持股" } else { "Edit Position" }
    }
    pub fn label_shares(self) -> &'static str {
        if self.is_zh() { "持有數量：" } else { "Shares:" }
    }
    pub fn label_avg_cost(self) -> &'static str {
        if self.is_zh() { "持有成本：" } else { "Avg Cost:" }
    }
    pub fn label_cost_basis(self) -> &'static str {
        if self.is_zh() { "總成本：" } else { "Cost Basis: " }
    }
    pub fn label_market_value(self) -> &'static str {
        if self.is_zh() { "總市值：" } else { "Market Value: " }
    }
    pub fn label_pnl(self) -> &'static str {
        if self.is_zh() { "收益" } else { "P&L" }
    }
    pub fn label_holding_short(self) -> &'static str {
        if self.is_zh() { "持股" } else { "Qty" }
    }
    pub fn label_total_pnl(self) -> &'static str {
        if self.is_zh() { "總收益" } else { "Total P&L" }
    }
    pub fn btn_save(self) -> &'static str {
        if self.is_zh() { "儲存" } else { "Save" }
    }
    pub fn btn_clear_pos(self) -> &'static str {
        if self.is_zh() { "清除持股" } else { "Clear" }
    }
    pub fn dpapi_encrypted_note(self) -> &'static str {
        if self.is_zh() {
            "🔒 資料已透過 Windows DPAPI 加密儲存，防止其他程式讀取"
        } else {
            "🔒 Data encrypted via Windows DPAPI to prevent unauthorized access"
        }
    }
    pub fn btn_check_update(self) -> &'static str {
        if self.is_zh() { "檢查更新" } else { "Check Updates" }
    }
    pub fn update_checking(self) -> &'static str {
        if self.is_zh() { "正在檢查 GitHub 最新版本..." } else { "Checking GitHub releases..." }
    }
    pub fn update_up_to_date(self, ver: &str) -> String {
        if self.is_zh() {
            format!("目前已是最新版本 (v{ver})")
        } else {
            format!("Up to date (v{ver})")
        }
    }
    pub fn update_available(self, ver: &str) -> String {
        if self.is_zh() {
            format!("發現新版本 v{ver}！")
        } else {
            format!("New version v{ver} available!")
        }
    }
    pub fn btn_download_update(self) -> &'static str {
        if self.is_zh() { "前往下載" } else { "Download" }
    }
    pub fn update_failed(self, err: &str) -> String {
        if self.is_zh() {
            format!("檢查更新失敗：{err}")
        } else {
            format!("Update check failed: {err}")
        }
    }
}
