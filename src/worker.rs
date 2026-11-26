use crate::config::{AppConfig, Market, StockItem};
use crate::market_hours::current_market_status;
use crate::quote::{QuoteData, build_agent, fetch_tw_batch, fetch_us_quotes};
use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

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
    Status(WorkerStatus),
    Error(String),
}

pub struct QuoteWorker {
    pub tx: Sender<WorkerCommand>,
    pub rx: Receiver<WorkerEvent>,
}

impl QuoteWorker {
    pub fn spawn(initial_config: AppConfig, egui_ctx: egui::Context) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel::<WorkerCommand>();
        let (evt_tx, evt_rx) = mpsc::channel::<WorkerEvent>();

        thread::spawn(move || {
            let agent = build_agent();
            let mut config = initial_config;
            let mut minimized = false;
            let mut initial_fetch_done = false;
            let mut consecutive_errors: u32 = 0;
            let mut last_fetch = Instant::now() - Duration::from_secs(3600);

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
                    let force_all = !initial_fetch_done || !config.smart_market_hours;
                    let tw_items: Vec<&StockItem> = config
                        .watchlist
                        .iter()
                        .filter(|s| s.market == Market::Tw && (force_all || status.tw_open))
                        .collect();
                    let us_items: Vec<&StockItem> = config
                        .watchlist
                        .iter()
                        .filter(|s| s.market == Market::Us && (force_all || status.us_open))
                        .collect();

                    if tw_items.is_empty() && us_items.is_empty() {
                        let _ = evt_tx.send(WorkerEvent::Status(WorkerStatus::IdleClosed));
                        egui_ctx.request_repaint();
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
                            let (map, err) = fetch_us_quotes(&agent, &us_items);
                            merged.extend(map);
                            if let Some(e) = err {
                                errors.push(e);
                            }
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
                        egui_ctx.request_repaint();
                    }

                    initial_fetch_done = true;
                    last_fetch = Instant::now();
                }

                // Wait efficiently until next command or next poll tick (max 1s granularity, zero busy-wait)
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
                            // Immediately refresh when restored from minimized
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
