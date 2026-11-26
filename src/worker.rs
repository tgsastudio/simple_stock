//! 背景報價輪詢執行緒
//! 負責依照使用者設定的刷新頻率、雙市場開盤時段感知與視窗最小化狀態，
//! 在背景批次抓取台股與美股報價，並在有新資料時透過 Win32 `PostMessageW` 喚醒主視窗重繪。

use crate::config::{AppConfig, Market, StockItem};
use crate::market_hours::{TradingPeriod, current_market_status};
use crate::quote::{
    QuoteData, build_agent, fetch_market_trading_period, fetch_tw_batch, fetch_us_quotes,
};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_APP};

/// 自訂 Win32 視窗訊息：背景執行緒有新報價或狀態更新時通知主視窗重繪
pub const WM_APP_WORKER_EVENT: u32 = WM_APP + 1;

pub enum WorkerCommand {
    UpdateConfig(AppConfig),
    ForceRefresh,
    SetMinimized(bool),
    Shutdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerStatus {
    Initializing,
    Updating,
    Updated,
    IdleClosed,
}

pub enum WorkerEvent {
    QuotesUpdated(HashMap<String, QuoteData>),
    MarketPeriods {
        tw: Option<TradingPeriod>,
        us: Option<TradingPeriod>,
    },
    Status(WorkerStatus),
    Error(String),
}

pub struct QuoteWorker {
    pub tx: Sender<WorkerCommand>,
    pub rx: Receiver<WorkerEvent>,
}

impl QuoteWorker {
    /// 啟動背景報價輪詢執行緒，`notify_hwnd` 為接收喚醒訊息的主視窗控制代碼
    pub fn spawn(initial_config: AppConfig, notify_hwnd: usize) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel::<WorkerCommand>();
        let (evt_tx, evt_rx) = mpsc::channel::<WorkerEvent>();

        let wake_ui = move || {
            if notify_hwnd != 0 {
                unsafe {
                    PostMessageW(notify_hwnd as HWND, WM_APP_WORKER_EVENT, 0, 0);
                }
            }
        };

        thread::spawn(move || {
            let agent = build_agent();
            let mut config = initial_config;
            let mut minimized = false;
            let mut initial_fetch_done = false;
            let mut consecutive_errors: u32 = 0;
            let mut last_fetch = Instant::now() - Duration::from_secs(3600);
            let mut tw_period: Option<TradingPeriod> = None;
            let mut us_period: Option<TradingPeriod> = None;
            let mut last_tw_probe = Instant::now() - Duration::from_secs(3600);
            let mut last_us_probe = Instant::now() - Duration::from_secs(3600);

            loop {
                let status = current_market_status(config.include_us_extended);
                let interval_secs = config.refresh_mode.interval_secs();

                let should_poll_now = if !initial_fetch_done {
                    true
                } else if minimized && config.pause_when_minimized {
                    false
                } else if let Some(base_secs) = interval_secs {
                    let backoff = (1u64 << consecutive_errors.min(3)).min(8);
                    let effective_secs = base_secs.saturating_mul(backoff);
                    last_fetch.elapsed() >= Duration::from_secs(effective_secs)
                } else {
                    false
                };

                if should_poll_now {
                    let now_ts = Utc::now().timestamp();
                    let mut periods_dirty = false;

                    if (!initial_fetch_done || status.tw_open)
                        && !tw_period.is_some_and(|tp| tp.is_tw_open(now_ts))
                        && last_tw_probe.elapsed() >= Duration::from_secs(60)
                    {
                        last_tw_probe = Instant::now();
                        if let Some(tp) = fetch_market_trading_period(&agent, "^TWII") {
                            if tw_period != Some(tp) {
                                tw_period = Some(tp);
                                periods_dirty = true;
                            }
                        }
                    }

                    let tw_open_now = tw_period.map_or(status.tw_open, |tp| tp.is_tw_open(now_ts));
                    let us_open_now = us_period.map_or(status.us_open, |tp| {
                        tp.is_us_open(now_ts, config.include_us_extended)
                    });

                    let force_all = !initial_fetch_done || !config.smart_market_hours;
                    let tw_items: Vec<&StockItem> = config
                        .watchlist
                        .iter()
                        .filter(|s| s.market == Market::Tw && (force_all || tw_open_now))
                        .collect();
                    let us_items: Vec<&StockItem> = config
                        .watchlist
                        .iter()
                        .filter(|s| s.market == Market::Us && (force_all || us_open_now))
                        .collect();

                    if tw_items.is_empty() && us_items.is_empty() {
                        if (!initial_fetch_done || status.us_open)
                            && !us_period
                                .is_some_and(|tp| tp.is_us_open(now_ts, config.include_us_extended))
                            && last_us_probe.elapsed() >= Duration::from_secs(60)
                        {
                            last_us_probe = Instant::now();
                            if let Some(tp) = fetch_market_trading_period(&agent, "^GSPC") {
                                if us_period != Some(tp) {
                                    us_period = Some(tp);
                                    periods_dirty = true;
                                }
                            }
                        }
                        if periods_dirty {
                            let _ = evt_tx.send(WorkerEvent::MarketPeriods {
                                tw: tw_period,
                                us: us_period,
                            });
                        }
                        let _ = evt_tx.send(WorkerEvent::Status(WorkerStatus::IdleClosed));
                        wake_ui();
                    } else {
                        let mut merged = HashMap::new();
                        let mut errors = Vec::new();

                        if !tw_items.is_empty() {
                            match fetch_tw_batch(&agent, &tw_items) {
                                Ok(map) => merged.extend(map),
                                Err(e) => errors.push(e),
                            }
                        }
                        if !us_items.is_empty() {
                            let (map, err) =
                                fetch_us_quotes(&agent, &us_items, config.include_us_extended);
                            for q in map.values() {
                                if let Some(tp) = q.trading_period {
                                    if us_period != Some(tp) {
                                        us_period = Some(tp);
                                        periods_dirty = true;
                                    }
                                    break;
                                }
                            }
                            merged.extend(map);
                            if let Some(e) = err {
                                errors.push(e);
                            }
                        } else if (!initial_fetch_done || status.us_open)
                            && !us_period
                                .is_some_and(|tp| tp.is_us_open(now_ts, config.include_us_extended))
                            && last_us_probe.elapsed() >= Duration::from_secs(60)
                        {
                            last_us_probe = Instant::now();
                            if let Some(tp) = fetch_market_trading_period(&agent, "^GSPC") {
                                if us_period != Some(tp) {
                                    us_period = Some(tp);
                                    periods_dirty = true;
                                }
                            }
                        }

                        if periods_dirty {
                            let _ = evt_tx.send(WorkerEvent::MarketPeriods {
                                tw: tw_period,
                                us: us_period,
                            });
                        }
                        if !merged.is_empty() {
                            consecutive_errors = 0;
                            let _ = evt_tx.send(WorkerEvent::QuotesUpdated(merged));
                        } else if !errors.is_empty() {
                            consecutive_errors = consecutive_errors.saturating_add(1);
                        }

                        if !errors.is_empty() {
                            let _ = evt_tx.send(WorkerEvent::Error(errors.join(" | ")));
                        } else {
                            let _ = evt_tx.send(WorkerEvent::Status(WorkerStatus::Updated));
                        }
                        wake_ui();
                    }

                    initial_fetch_done = true;
                    last_fetch = Instant::now();
                }

                let wait = if minimized && config.pause_when_minimized {
                    Duration::from_secs(5)
                } else if let Some(base_secs) = interval_secs {
                    let backoff = (1u64 << consecutive_errors.min(3)).min(8);
                    let target = Duration::from_secs(base_secs.saturating_mul(backoff));
                    target
                        .checked_sub(last_fetch.elapsed())
                        .unwrap_or(Duration::from_millis(200))
                        .min(Duration::from_secs(2))
                } else {
                    Duration::from_secs(5)
                };

                match cmd_rx.recv_timeout(wait) {
                    Ok(WorkerCommand::UpdateConfig(new_cfg)) => {
                        let list_changed = new_cfg.watchlist != config.watchlist;
                        config = new_cfg;
                        if list_changed {
                            initial_fetch_done = false;
                        }
                    }
                    Ok(WorkerCommand::ForceRefresh) => {
                        initial_fetch_done = false;
                        consecutive_errors = 0;
                    }
                    Ok(WorkerCommand::SetMinimized(is_min)) => {
                        let was_min = minimized;
                        minimized = is_min;
                        if was_min && !is_min && config.pause_when_minimized {
                            initial_fetch_done = false;
                        }
                    }
                    Ok(WorkerCommand::Shutdown) | Err(RecvTimeoutError::Disconnected) => break,
                    Err(RecvTimeoutError::Timeout) => {}
                }
            }
        });

        Self {
            tx: cmd_tx,
            rx: evt_rx,
        }
    }
}
