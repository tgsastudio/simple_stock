//! 多語系模組（繁體中文 / 简体中文 / English / 日本語 / 한국어 / Español）
//! 預設呼叫 Windows 原生 `GetUserDefaultUILanguage()` 自動跟隨作業系統語言。

use crate::config::{ColorScheme, LanguageSetting, RefreshMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    ZhTw,
    ZhCn,
    En,
    Ja,
    Ko,
    Es,
}

#[cfg(target_os = "windows")]
unsafe extern "system" {
    fn GetUserDefaultUILanguage() -> u16;
}

/// 偵測作業系統介面語言（繁中 / 簡中 / 日文 / 韓文 / 西班牙文 / 英文）
pub fn detect_os_lang() -> Lang {
    #[cfg(target_os = "windows")]
    {
        let lang_id = unsafe { GetUserDefaultUILanguage() };
        let primary = lang_id & 0x03FF;
        let sub = (lang_id >> 10) & 0x003F;
        match primary {
            0x04 => {
                // 0x02 = zh-CN (0x0804), 0x04 = zh-SG (0x1004)
                if sub == 0x02 || sub == 0x04 {
                    Lang::ZhCn
                } else {
                    Lang::ZhTw
                }
            }
            0x11 => Lang::Ja,
            0x12 => Lang::Ko,
            0x0A => Lang::Es,
            _ => Lang::En,
        }
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
        LanguageSetting::ZhCn => Lang::ZhCn,
        LanguageSetting::En => Lang::En,
        LanguageSetting::Ja => Lang::Ja,
        LanguageSetting::Ko => Lang::Ko,
        LanguageSetting::Es => Lang::Es,
    }
}

impl Lang {
    pub fn mode_standard(self) -> &'static str {
        match self {
            Lang::ZhTw => "標準",
            Lang::ZhCn => "标准",
            Lang::En => "Std",
            Lang::Ja => "標準",
            Lang::Ko => "표준",
            Lang::Es => "Est",
        }
    }
    pub fn mode_floating(self) -> &'static str {
        match self {
            Lang::ZhTw => "浮動",
            Lang::ZhCn => "浮动",
            Lang::En => "Float",
            Lang::Ja => "浮動",
            Lang::Ko => "플로팅",
            Lang::Es => "Flot",
        }
    }

    pub fn btn_top_settings(self) -> &'static str {
        match self {
            Lang::ZhTw => "設定",
            Lang::ZhCn => "设置",
            Lang::En => "Set",
            Lang::Ja => "設定",
            Lang::Ko => "설정",
            Lang::Es => "Cfg",
        }
    }
    pub fn btn_top_refresh(self) -> &'static str {
        match self {
            Lang::ZhTw | Lang::ZhCn => "刷",
            Lang::En | Lang::Es => "R",
            Lang::Ja => "更",
            Lang::Ko => "새",
        }
    }
    pub fn btn_top_pin(self) -> &'static str {
        match self {
            Lang::ZhTw | Lang::ZhCn => "頂",
            Lang::En => "Pin",
            Lang::Ja => "固",
            Lang::Ko => "고정",
            Lang::Es => "Fij",
        }
    }

    pub fn filter_all(self) -> &'static str {
        match self {
            Lang::ZhTw | Lang::ZhCn => "全部",
            Lang::En => "All",
            Lang::Ja => "全て",
            Lang::Ko => "전체",
            Lang::Es => "Todo",
        }
    }
    pub fn filter_tw(self) -> &'static str {
        match self {
            Lang::ZhTw | Lang::ZhCn => "台股",
            Lang::En | Lang::Es => "TW",
            Lang::Ja => "台湾",
            Lang::Ko => "대만",
        }
    }
    pub fn filter_us(self) -> &'static str {
        match self {
            Lang::ZhTw | Lang::ZhCn => "美股",
            Lang::En => "US",
            Lang::Ja => "米国",
            Lang::Ko => "미국",
            Lang::Es => "EEUU",
        }
    }

    pub fn sort_custom(self) -> &'static str {
        match self {
            Lang::ZhTw => "自訂順序",
            Lang::ZhCn => "自定义顺序",
            Lang::En => "Custom Order",
            Lang::Ja => "カスタム順",
            Lang::Ko => "사용자 순서",
            Lang::Es => "Orden",
        }
    }
    pub fn sort_desc(self) -> &'static str {
        match self {
            Lang::ZhTw => "漲幅高→低",
            Lang::ZhCn => "涨幅高→低",
            Lang::En => "Change High→Low",
            Lang::Ja => "騰落率 高→低",
            Lang::Ko => "등락률 높→낮",
            Lang::Es => "Cambio ↓",
        }
    }
    pub fn sort_asc(self) -> &'static str {
        match self {
            Lang::ZhTw => "漲幅低→高",
            Lang::ZhCn => "涨幅低→高",
            Lang::En => "Change Low→High",
            Lang::Ja => "騰落率 低→高",
            Lang::Ko => "등락률 낮→높",
            Lang::Es => "Cambio ↑",
        }
    }

    pub fn empty_watchlist(self) -> &'static str {
        match self {
            Lang::ZhTw => "尚未設定自選股，請點擊右上角 ＋ 新增",
            Lang::ZhCn => "尚未设置自选股，请点击右上角 ＋ 新增",
            Lang::En => "Watchlist is empty. Click ＋ to add stocks.",
            Lang::Ja => "ウォッチリストが空です。右上の ＋ から追加してください。",
            Lang::Ko => "관심 종목이 비어 있습니다. 우측 상단 ＋로 추가하세요.",
            Lang::Es => "Lista vacía. Pulsa ＋ para agregar acciones.",
        }
    }

    pub fn prev_close(self) -> &'static str {
        match self {
            Lang::ZhTw | Lang::ZhCn => "昨收：",
            Lang::En => "Prev Close: ",
            Lang::Ja => "前日終値：",
            Lang::Ko => "전일종가: ",
            Lang::Es => "Cierre ant.: ",
        }
    }
    pub fn open_price(self) -> &'static str {
        match self {
            Lang::ZhTw => "開盤：",
            Lang::ZhCn => "开盘：",
            Lang::En => "Open: ",
            Lang::Ja => "始値：",
            Lang::Ko => "시가: ",
            Lang::Es => "Apertura: ",
        }
    }
    pub fn high_low(self) -> &'static str {
        match self {
            Lang::ZhTw | Lang::ZhCn => "最高/最低：",
            Lang::En => "High/Low: ",
            Lang::Ja => "高値/安値：",
            Lang::Ko => "고가/저가: ",
            Lang::Es => "Máx/Mín: ",
        }
    }
    pub fn volume(self) -> &'static str {
        match self {
            Lang::ZhTw | Lang::ZhCn => "成交量：",
            Lang::En => "Vol: ",
            Lang::Ja => "出来高：",
            Lang::Ko => "거래량: ",
            Lang::Es => "Vol.: ",
        }
    }
    pub fn source_updated(self, src: &str, time: &str) -> String {
        match self {
            Lang::ZhTw => format!("來源：{src} ・ 更新：{time}"),
            Lang::ZhCn => format!("来源：{src} ・ 更新：{time}"),
            Lang::En => format!("Source: {src} ・ Updated: {time}"),
            Lang::Ja => format!("提供：{src} ・ 更新：{time}"),
            Lang::Ko => format!("출처: {src} ・ 업데이트: {time}"),
            Lang::Es => format!("Fuente: {src} ・ Act.: {time}"),
        }
    }

    pub fn ctx_pin_top(self) -> &'static str {
        match self {
            Lang::ZhTw => "移至最頂",
            Lang::ZhCn => "移至顶部",
            Lang::En => "Move to Top",
            Lang::Ja => "一番上へ移動",
            Lang::Ko => "맨 위로 이동",
            Lang::Es => "Mover arriba",
        }
    }
    pub fn ctx_move_up(self) -> &'static str {
        match self {
            Lang::ZhTw => "↑ 上移一格",
            Lang::ZhCn => "↑ 上移一格",
            Lang::En => "↑ Move Up",
            Lang::Ja => "↑ 上へ移動",
            Lang::Ko => "↑ 위로 이동",
            Lang::Es => "↑ Subir",
        }
    }
    pub fn ctx_move_down(self) -> &'static str {
        match self {
            Lang::ZhTw => "↓ 下移一格",
            Lang::ZhCn => "↓ 下移一格",
            Lang::En => "↓ Move Down",
            Lang::Ja => "↓ 下へ移動",
            Lang::Ko => "↓ 아래로 이동",
            Lang::Es => "↓ Bajar",
        }
    }
    pub fn ctx_delete(self) -> &'static str {
        match self {
            Lang::ZhTw => "移除自選股",
            Lang::ZhCn => "移除自选股",
            Lang::En => "Remove",
            Lang::Ja => "リストから削除",
            Lang::Ko => "목록에서 제거",
            Lang::Es => "Eliminar",
        }
    }

    pub fn add_title(self) -> &'static str {
        match self {
            Lang::ZhTw => "新增自選股",
            Lang::ZhCn => "新增自选股",
            Lang::En => "Add Stock",
            Lang::Ja => "銘柄を追加",
            Lang::Ko => "종목 추가",
            Lang::Es => "Agregar acción",
        }
    }
    pub fn add_prompt(self) -> &'static str {
        match self {
            Lang::ZhTw => "輸入台股代碼 (如 2330, 0050) 或美股代碼 (如 AAPL, NVDA)：",
            Lang::ZhCn => "输入台股代码 (如 2330, 0050) 或美股代码 (如 AAPL, NVDA)：",
            Lang::En => "Enter TW code (e.g. 2330, 0050) or US ticker (e.g. AAPL, NVDA):",
            Lang::Ja => "台湾株コード (例: 2330) または米国株ティッカー (例: AAPL, NVDA):",
            Lang::Ko => "대만 종목코드(예: 2330) 또는 미국 티커(예: AAPL, NVDA) 입력:",
            Lang::Es => "Código TW (ej. 2330) o ticker EEUU (ej. AAPL, NVDA):",
        }
    }
    pub fn btn_search(self) -> &'static str {
        match self {
            Lang::ZhTw => "線上搜尋",
            Lang::ZhCn => "在线搜索",
            Lang::En => "Search",
            Lang::Ja => "オンライン検索",
            Lang::Ko => "온라인 검색",
            Lang::Es => "Buscar",
        }
    }
    pub fn label_market(self) -> &'static str {
        match self {
            Lang::ZhTw | Lang::ZhCn => "市場：",
            Lang::En => "Market:",
            Lang::Ja => "市場：",
            Lang::Ko => "시장:",
            Lang::Es => "Mercado:",
        }
    }
    pub fn market_tw(self) -> &'static str {
        match self {
            Lang::ZhTw | Lang::ZhCn => "台股 (TW)",
            Lang::En => "Taiwan (TW)",
            Lang::Ja => "台湾株 (TW)",
            Lang::Ko => "대만 (TW)",
            Lang::Es => "Taiwán (TW)",
        }
    }
    pub fn market_us(self) -> &'static str {
        match self {
            Lang::ZhTw | Lang::ZhCn => "美股 (US)",
            Lang::En => "US (US)",
            Lang::Ja => "米国株 (US)",
            Lang::Ko => "미국 (US)",
            Lang::Es => "EE.UU. (US)",
        }
    }
    pub fn label_name_short(self) -> &'static str {
        match self {
            Lang::ZhTw => "名稱 (選填)：",
            Lang::ZhCn => "名称 (选填)：",
            Lang::En => "Name (opt):",
            Lang::Ja => "名称 (任意)：",
            Lang::Ko => "이름 (선택):",
            Lang::Es => "Nombre (opc):",
        }
    }
    pub fn btn_add_direct(self) -> &'static str {
        match self {
            Lang::ZhTw => "直接加入自選股",
            Lang::ZhCn => "直接加入自选股",
            Lang::En => "Add to Watchlist",
            Lang::Ja => "リストに追加",
            Lang::Ko => "목록에 추가",
            Lang::Es => "Agregar",
        }
    }
    pub fn searching(self) -> &'static str {
        match self {
            Lang::ZhTw => "搜尋中...",
            Lang::ZhCn => "搜索中...",
            Lang::En => "Searching...",
            Lang::Ja => "検索中...",
            Lang::Ko => "검색 중...",
            Lang::Es => "Buscando...",
        }
    }
    pub fn search_no_results(self) -> &'static str {
        match self {
            Lang::ZhTw => "查無符合結果",
            Lang::ZhCn => "未找到匹配结果",
            Lang::En => "No results",
            Lang::Ja => "該当する結果がありません",
            Lang::Ko => "검색 결과가 없습니다",
            Lang::Es => "Sin resultados",
        }
    }
    pub fn search_results_header(self) -> &'static str {
        match self {
            Lang::ZhTw => "線上搜尋結果（單擊帶入、雙擊直接加入）：",
            Lang::ZhCn => "在线搜索结果（单击带入、双击直接加入）：",
            Lang::En => "Search results (click to fill, double-click to add):",
            Lang::Ja => "検索結果（クリックで入力、ダブルクリックで追加）：",
            Lang::Ko => "검색 결과 (클릭하여 입력, 더블클릭으로 추가):",
            Lang::Es => "Resultados (clic para llenar, doble clic para agregar):",
        }
    }

    pub fn settings_title(self) -> &'static str {
        match self {
            Lang::ZhTw => "設定",
            Lang::ZhCn => "设置",
            Lang::En => "Settings",
            Lang::Ja => "設定",
            Lang::Ko => "설정",
            Lang::Es => "Configuración",
        }
    }
    pub fn sec_refresh(self) -> &'static str {
        match self {
            Lang::ZhTw => "刷新頻率與省電",
            Lang::ZhCn => "刷新频率与省电",
            Lang::En => "Refresh & Power",
            Lang::Ja => "更新間隔・省電力",
            Lang::Ko => "새로고침 및 절전",
            Lang::Es => "Actualización",
        }
    }
    pub fn chk_smart_hours(self) -> &'static str {
        match self {
            Lang::ZhTw => "開盤時段感知（休市自動停止輪詢）",
            Lang::ZhCn => "开盘时段感知（休市自动停止轮询）",
            Lang::En => "Market-hours aware (pause when closed)",
            Lang::Ja => "取引時間連動（休場時は自動停止）",
            Lang::Ko => "장 시간 감지 (휴장 시 자동 일시정지)",
            Lang::Es => "Horario de mercado (pausar al cierre)",
        }
    }
    pub fn chk_pause_min(self) -> &'static str {
        match self {
            Lang::ZhTw => "視窗最小化時暫停刷新",
            Lang::ZhCn => "窗口最小化时暂停刷新",
            Lang::En => "Pause refresh when minimized",
            Lang::Ja => "最小化時に更新を一時停止",
            Lang::Ko => "최소화 시 새로고침 일시정지",
            Lang::Es => "Pausar al minimizar",
        }
    }
    pub fn chk_us_ext(self) -> &'static str {
        match self {
            Lang::ZhTw => "包含美股盤前/盤後時段",
            Lang::ZhCn => "包含美股盘前/盘后时段",
            Lang::En => "Include US pre/post-market hours",
            Lang::Ja => "米国株の時間外取引を含む",
            Lang::Ko => "미국 장전/장후 거래 포함",
            Lang::Es => "Incluir horario extendido de EE.UU.",
        }
    }
    pub fn label_language(self) -> &'static str {
        match self {
            Lang::ZhTw => "介面語言",
            Lang::ZhCn => "界面语言",
            Lang::En => "Language",
            Lang::Ja => "表示言語",
            Lang::Ko => "표시 언어",
            Lang::Es => "Idioma",
        }
    }
    pub fn label_color_scheme(self) -> &'static str {
        match self {
            Lang::ZhTw => "漲跌配色",
            Lang::ZhCn => "涨跌配色",
            Lang::En => "Color Scheme",
            Lang::Ja => "騰落カラー",
            Lang::Ko => "등락 색상",
            Lang::Es => "Esquema de color",
        }
    }
    pub fn slider_opacity(self) -> &'static str {
        match self {
            Lang::ZhTw => "浮動模式透明度",
            Lang::ZhCn => "浮动模式透明度",
            Lang::En => "Floating Opacity",
            Lang::Ja => "浮動モード透明度",
            Lang::Ko => "플로팅 투명도",
            Lang::Es => "Opacidad flotante",
        }
    }
    pub fn chk_dark(self) -> &'static str {
        match self {
            Lang::ZhTw => "深色主題",
            Lang::ZhCn => "深色主题",
            Lang::En => "Dark Theme",
            Lang::Ja => "ダークテーマ",
            Lang::Ko => "다크 테마",
            Lang::Es => "Tema oscuro",
        }
    }
    pub fn chk_always_on_top(self) -> &'static str {
        match self {
            Lang::ZhTw => "視窗置頂 (Always on Top)",
            Lang::ZhCn => "窗口置顶 (Always on Top)",
            Lang::En => "Always on Top",
            Lang::Ja => "常に最前面に表示",
            Lang::Ko => "항상 위에 표시",
            Lang::Es => "Siempre visible",
        }
    }
    pub fn config_file_label(self) -> &'static str {
        match self {
            Lang::ZhTw => "設定檔：",
            Lang::ZhCn => "配置文件：",
            Lang::En => "Config: ",
            Lang::Ja => "設定ファイル：",
            Lang::Ko => "설정 파일: ",
            Lang::Es => "Config.: ",
        }
    }

    pub fn status_init(self) -> &'static str {
        match self {
            Lang::ZhTw | Lang::ZhCn => "初始化中...",
            Lang::En => "Initializing...",
            Lang::Ja => "初期化中...",
            Lang::Ko => "초기화 중...",
            Lang::Es => "Inicializando...",
        }
    }
    pub fn status_updating(self) -> &'static str {
        match self {
            Lang::ZhTw | Lang::ZhCn => "更新中...",
            Lang::En => "Updating...",
            Lang::Ja => "更新中...",
            Lang::Ko => "업데이트 중...",
            Lang::Es => "Actualizando...",
        }
    }
    pub fn status_updated(self) -> &'static str {
        match self {
            Lang::ZhTw | Lang::ZhCn => "已更新",
            Lang::En => "Updated",
            Lang::Ja => "更新済み",
            Lang::Ko => "업데이트됨",
            Lang::Es => "Actualizado",
        }
    }
    pub fn status_idle_closed(self) -> &'static str {
        match self {
            Lang::ZhTw => "休市待機中 (省電暫停)",
            Lang::ZhCn => "休市待机中 (省电暂停)",
            Lang::En => "Market closed (eco standby)",
            Lang::Ja => "休場中（省電力待機）",
            Lang::Ko => "휴장 대기 중 (절전)",
            Lang::Es => "Mercado cerrado (ahorro)",
        }
    }

    pub fn refresh_label(self, mode: RefreshMode) -> &'static str {
        match (self, mode) {
            (Lang::ZhTw, RefreshMode::Fast5s) => "5 秒 (極速)",
            (Lang::ZhTw, RefreshMode::Balanced15s) => "15 秒 (均衡)",
            (Lang::ZhTw, RefreshMode::Eco30s) => "30 秒 (省電)",
            (Lang::ZhTw, RefreshMode::UltraEco60s) => "60 秒 (超省電)",
            (Lang::ZhTw, RefreshMode::Manual) => "手動刷新",
            (Lang::ZhCn, RefreshMode::Fast5s) => "5 秒 (极速)",
            (Lang::ZhCn, RefreshMode::Balanced15s) => "15 秒 (均衡)",
            (Lang::ZhCn, RefreshMode::Eco30s) => "30 秒 (省电)",
            (Lang::ZhCn, RefreshMode::UltraEco60s) => "60 秒 (超省电)",
            (Lang::ZhCn, RefreshMode::Manual) => "手动刷新",
            (Lang::Ja, RefreshMode::Fast5s) => "5秒 (高速)",
            (Lang::Ja, RefreshMode::Balanced15s) => "15秒 (標準)",
            (Lang::Ja, RefreshMode::Eco30s) => "30秒 (省電力)",
            (Lang::Ja, RefreshMode::UltraEco60s) => "60秒 (超省電力)",
            (Lang::Ja, RefreshMode::Manual) => "手動更新",
            (Lang::Ko, RefreshMode::Fast5s) => "5초 (고속)",
            (Lang::Ko, RefreshMode::Balanced15s) => "15초 (균형)",
            (Lang::Ko, RefreshMode::Eco30s) => "30초 (절전)",
            (Lang::Ko, RefreshMode::UltraEco60s) => "60초 (초절전)",
            (Lang::Ko, RefreshMode::Manual) => "수동 새로고침",
            (Lang::Es, RefreshMode::Fast5s) => "5s (Rápido)",
            (Lang::Es, RefreshMode::Balanced15s) => "15s (Equilibrado)",
            (Lang::Es, RefreshMode::Eco30s) => "30s (Eco)",
            (Lang::Es, RefreshMode::UltraEco60s) => "60s (Ultra Eco)",
            (Lang::Es, RefreshMode::Manual) => "Manual",
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
            (Lang::ZhCn, ColorScheme::TwRedUp) => "红涨绿跌 (中台习惯)",
            (Lang::ZhCn, ColorScheme::UsGreenUp) => "绿涨红跌 (美股习惯)",
            (Lang::ZhCn, ColorScheme::AutoByMarket) => "按市场自动配色",
            (Lang::Ja, ColorScheme::TwRedUp) => "赤上げ・緑下げ",
            (Lang::Ja, ColorScheme::UsGreenUp) => "緑上げ・赤下げ",
            (Lang::Ja, ColorScheme::AutoByMarket) => "市場別で自動切替",
            (Lang::Ko, ColorScheme::TwRedUp) => "빨강 상승 / 초록 하락",
            (Lang::Ko, ColorScheme::UsGreenUp) => "초록 상승 / 빨강 하락",
            (Lang::Ko, ColorScheme::AutoByMarket) => "시장별 자동 색상",
            (Lang::Es, ColorScheme::TwRedUp) => "Rojo sube / Verde baja",
            (Lang::Es, ColorScheme::UsGreenUp) => "Verde sube / Rojo baja",
            (Lang::Es, ColorScheme::AutoByMarket) => "Automático por mercado",
            (Lang::En, ColorScheme::TwRedUp) => "Red Up / Green Down (TW)",
            (Lang::En, ColorScheme::UsGreenUp) => "Green Up / Red Down (US)",
            (Lang::En, ColorScheme::AutoByMarket) => "Auto by Market",
        }
    }

    pub fn lang_setting_label(self, ls: LanguageSetting) -> &'static str {
        match ls {
            LanguageSetting::Auto => match self {
                Lang::ZhTw => "跟隨作業系統 (Auto)",
                Lang::ZhCn => "跟随操作系统 (Auto)",
                Lang::En => "Follow System (Auto)",
                Lang::Ja => "システム言語 (Auto)",
                Lang::Ko => "시스템 언어 (Auto)",
                Lang::Es => "Seguir sistema (Auto)",
            },
            LanguageSetting::ZhTw => "繁體中文",
            LanguageSetting::ZhCn => "简体中文",
            LanguageSetting::En => "English",
            LanguageSetting::Ja => "日本語",
            LanguageSetting::Ko => "한국어",
            LanguageSetting::Es => "Español",
        }
    }

    // 持股與收益相關標籤
    pub fn ctx_edit_position(self) -> &'static str {
        match self {
            Lang::ZhTw => "編輯持股 (持有量/成本)",
            Lang::ZhCn => "编辑持股 (持有量/成本)",
            Lang::En => "Edit Position (Qty/Cost)",
            Lang::Ja => "保有数を編集 (数量/取得単価)",
            Lang::Ko => "보유 정보 편집 (수량/평단가)",
            Lang::Es => "Editar posición (Cant./Costo)",
        }
    }
    pub fn btn_edit_pos_short(self) -> &'static str {
        match self {
            Lang::ZhTw | Lang::ZhCn => "持股",
            Lang::En => "Pos",
            Lang::Ja => "保有",
            Lang::Ko => "보유",
            Lang::Es => "Pos",
        }
    }
    pub fn edit_pos_title(self) -> &'static str {
        match self {
            Lang::ZhTw => "編輯持股",
            Lang::ZhCn => "编辑持股",
            Lang::En => "Edit Position",
            Lang::Ja => "保有情報を編集",
            Lang::Ko => "보유 정보 편집",
            Lang::Es => "Editar posición",
        }
    }
    pub fn label_shares(self) -> &'static str {
        match self {
            Lang::ZhTw => "持有數量：",
            Lang::ZhCn => "持有数量：",
            Lang::En => "Shares:",
            Lang::Ja => "保有数量：",
            Lang::Ko => "보유 수량:",
            Lang::Es => "Cantidad:",
        }
    }
    pub fn label_avg_cost(self) -> &'static str {
        match self {
            Lang::ZhTw | Lang::ZhCn => "持有成本：",
            Lang::En => "Avg Cost:",
            Lang::Ja => "取得単価：",
            Lang::Ko => "평균 단가:",
            Lang::Es => "Costo prom.:",
        }
    }
    pub fn label_cost_basis(self) -> &'static str {
        match self {
            Lang::ZhTw => "總成本：",
            Lang::ZhCn => "总成本：",
            Lang::En => "Cost Basis: ",
            Lang::Ja => "取得総額：",
            Lang::Ko => "총 매입가: ",
            Lang::Es => "Costo total: ",
        }
    }
    pub fn label_market_value(self) -> &'static str {
        match self {
            Lang::ZhTw => "總市值：",
            Lang::ZhCn => "总市值：",
            Lang::En => "Market Value: ",
            Lang::Ja => "評価額：",
            Lang::Ko => "평가 금액: ",
            Lang::Es => "Valor actual: ",
        }
    }
    pub fn label_pnl(self) -> &'static str {
        match self {
            Lang::ZhTw | Lang::ZhCn => "收益",
            Lang::En => "P&L",
            Lang::Ja => "損益",
            Lang::Ko => "손익",
            Lang::Es => "G/P",
        }
    }
    pub fn label_holding_short(self) -> &'static str {
        match self {
            Lang::ZhTw | Lang::ZhCn => "持股",
            Lang::En => "Qty",
            Lang::Ja => "保有",
            Lang::Ko => "보유",
            Lang::Es => "Cant.",
        }
    }
    pub fn label_total_pnl(self) -> &'static str {
        match self {
            Lang::ZhTw => "總收益",
            Lang::ZhCn => "总收益",
            Lang::En => "Total P&L",
            Lang::Ja => "合計損益",
            Lang::Ko => "총 손익",
            Lang::Es => "Total G/P",
        }
    }
    pub fn btn_save(self) -> &'static str {
        match self {
            Lang::ZhTw => "儲存",
            Lang::ZhCn => "保存",
            Lang::En => "Save",
            Lang::Ja => "保存",
            Lang::Ko => "저장",
            Lang::Es => "Guardar",
        }
    }
    pub fn btn_cancel(self) -> &'static str {
        match self {
            Lang::ZhTw | Lang::ZhCn => "取消",
            Lang::En => "Cancel",
            Lang::Ja => "キャンセル",
            Lang::Ko => "취소",
            Lang::Es => "Cancelar",
        }
    }
    pub fn btn_close(self) -> &'static str {
        match self {
            Lang::ZhTw => "關閉",
            Lang::ZhCn => "关闭",
            Lang::En => "Close",
            Lang::Ja => "閉じる",
            Lang::Ko => "닫기",
            Lang::Es => "Cerrar",
        }
    }
    pub fn btn_clear_pos(self) -> &'static str {
        match self {
            Lang::ZhTw | Lang::ZhCn => "清除持股",
            Lang::En => "Clear",
            Lang::Ja => "クリア",
            Lang::Ko => "초기화",
            Lang::Es => "Limpiar",
        }
    }
    pub fn dpapi_encrypted_note(self) -> &'static str {
        match self {
            Lang::ZhTw => "🔒 資料已透過 Windows DPAPI 加密儲存，防止其他程式讀取",
            Lang::ZhCn => "🔒 数据已通过 Windows DPAPI 加密保存，防止其他程序读取",
            Lang::En => "🔒 Data encrypted via Windows DPAPI to prevent unauthorized access",
            Lang::Ja => "🔒 データは Windows DPAPI で暗号化保存され、他アプリから保護されます",
            Lang::Ko => "🔒 데이터는 Windows DPAPI로 암호화되어 다른 프로그램의 접근을 차단합니다",
            Lang::Es => "🔒 Datos cifrados con Windows DPAPI para evitar accesos no autorizados",
        }
    }
    pub fn btn_check_update(self) -> &'static str {
        match self {
            Lang::ZhTw => "檢查更新",
            Lang::ZhCn => "检查更新",
            Lang::En => "Check Updates",
            Lang::Ja => "更新を確認",
            Lang::Ko => "업데이트 확인",
            Lang::Es => "Buscar act.",
        }
    }
    pub fn update_checking(self) -> &'static str {
        match self {
            Lang::ZhTw => "正在檢查 GitHub 最新版本...",
            Lang::ZhCn => "正在检查 GitHub 最新版本...",
            Lang::En => "Checking GitHub releases...",
            Lang::Ja => "GitHub の最新版を確認中...",
            Lang::Ko => "GitHub 최신 버전 확인 중...",
            Lang::Es => "Comprobando GitHub Releases...",
        }
    }
    pub fn update_up_to_date(self, ver: &str) -> String {
        match self {
            Lang::ZhTw => format!("目前已是最新版本 (v{ver})"),
            Lang::ZhCn => format!("当前已是最新版本 (v{ver})"),
            Lang::En => format!("Up to date (v{ver})"),
            Lang::Ja => format!("最新バージョンです (v{ver})"),
            Lang::Ko => format!("최신 버전입니다 (v{ver})"),
            Lang::Es => format!("Actualizado (v{ver})"),
        }
    }
    pub fn update_available(self, ver: &str) -> String {
        match self {
            Lang::ZhTw => format!("發現新版本 v{ver}！"),
            Lang::ZhCn => format!("发现新版本 v{ver}！"),
            Lang::En => format!("New version v{ver} available!"),
            Lang::Ja => format!("新しいバージョン v{ver} があります！"),
            Lang::Ko => format!("새 버전 v{ver} 사용 가능!"),
            Lang::Es => format!("¡Nueva versión v{ver} disponible!"),
        }
    }
    pub fn btn_download_update(self) -> &'static str {
        match self {
            Lang::ZhTw => "前往下載",
            Lang::ZhCn => "前往下载",
            Lang::En => "Download",
            Lang::Ja => "ダウンロード",
            Lang::Ko => "다운로드",
            Lang::Es => "Descargar",
        }
    }
    pub fn update_failed(self, err: &str) -> String {
        match self {
            Lang::ZhTw => format!("檢查更新失敗：{err}"),
            Lang::ZhCn => format!("检查更新失败：{err}"),
            Lang::En => format!("Update check failed: {err}"),
            Lang::Ja => format!("更新確認に失敗：{err}"),
            Lang::Ko => format!("업데이트 확인 실패: {err}"),
            Lang::Es => format!("Error al comprobar: {err}"),
        }
    }
}
