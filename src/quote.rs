//! 免登入即時報價抓取引擎
//! - 台股：優先透過證交所/櫃買中心公開即時 API 單次批次查詢，查無資料時自動切換 Yahoo Finance 備援。
//! - 美股：透過 Yahoo Finance v8 chart 公開端點查詢（無需登入或 crumb）。

use crate::config::{Market, StockItem};
use crate::market_hours::TradingPeriod;
use chrono::{Local, Utc};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct QuoteData {
    pub symbol: String,
    pub display_name: String,
    #[allow(dead_code)]
    pub market: Market,
    pub price: f64,
    pub prev_close: f64,
    pub change: f64,
    pub change_percent: f64,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub volume: Option<u64>,
    pub updated_at: String,
    pub source: &'static str,
    pub trading_period: Option<TradingPeriod>,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub symbol: String,
    pub name: String,
    pub market: Market,
    pub exchange: String,
}

pub fn build_agent() -> ureq::Agent {
    let tls = std::sync::Arc::new(
        native_tls::TlsConnector::new().expect("Failed to initialize Windows TLS"),
    );
    ureq::AgentBuilder::new()
        .tls_connector(tls)
        .timeout_connect(Duration::from_secs(6))
        .timeout_read(Duration::from_secs(8))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .build()
}

fn parse_f64_opt(v: Option<&Value>) -> Option<f64> {
    match v? {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() || trimmed == "-" {
                None
            } else {
                trimmed.parse::<f64>().ok()
            }
        }
        _ => None,
    }
}

fn parse_first_book_price(v: Option<&Value>) -> Option<f64> {
    let s = v?.as_str()?;
    for part in s.split('_') {
        if let Ok(p) = part.trim().parse::<f64>() {
            if p > 0.0 {
                return Some(p);
            }
        }
    }
    None
}

fn now_hms() -> String {
    Local::now().format("%H:%M:%S").to_string()
}

/// Fetch Taiwan stock quotes in a single batch from Cnyes real-time API,
/// falling back to TWSE/TPEx MIS and Yahoo Finance for any missing symbols.
pub fn fetch_tw_batch(
    agent: &ureq::Agent,
    items: &[&StockItem],
) -> Result<HashMap<String, QuoteData>, String> {
    let mut result = HashMap::new();
    if items.is_empty() {
        return Ok(result);
    }

    // 1. Primary: Cnyes batch real-time quote API (supports both TWSE & TPEx in real time)
    if let Ok(cnyes_map) = fetch_cnyes_tw_batch(agent, items) {
        result.extend(cnyes_map);
    }

    let missing: Vec<&StockItem> = items
        .iter()
        .copied()
        .filter(|i| !result.contains_key(&i.key()))
        .collect();
    if missing.is_empty() {
        return Ok(result);
    }

    // 2. Fallback: TWSE/TPEx MIS public API for any missing symbols
    let mut channels = Vec::with_capacity(missing.len() * 2);
    for item in &missing {
        let code = item.symbol.trim().to_uppercase();
        channels.push(format!("tse_{code}.tw"));
        channels.push(format!("otc_{code}.tw"));
    }
    let ts = Utc::now().timestamp_millis();
    let url = format!(
        "https://mis.twse.com.tw/stock/api/getStockInfo.jsp?ex_ch={}&json=1&delay=0&_={ts}",
        channels.join("|")
    );

    if let Ok(resp) = agent
        .get(&url)
        .set("Referer", "https://mis.twse.com.tw/")
        .set("Cache-Control", "no-cache")
        .call()
    {
        if let Ok(body) = resp.into_json::<Value>() {
            let updated_at = now_hms();
            if let Some(arr) = body.get("msgArray").and_then(|v| v.as_array()) {
                for entry in arr {
                    let code = match entry.get("c").and_then(|v| v.as_str()) {
                        Some(c) if !c.is_empty() => c.to_uppercase(),
                        _ => continue,
                    };
                    let prev_close = match parse_f64_opt(entry.get("y")) {
                        Some(y) if y > 0.0 => y,
                        _ => continue,
                    };
                    let price = parse_f64_opt(entry.get("z"))
                        .filter(|&p| p > 0.0)
                        .or_else(|| parse_first_book_price(entry.get("b")))
                        .or_else(|| parse_first_book_price(entry.get("a")))
                        .unwrap_or(prev_close);

                    let name = entry
                        .get("n")
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty())
                        .unwrap_or(&code)
                        .to_string();

                    let open = parse_f64_opt(entry.get("o")).filter(|&v| v > 0.0);
                    let high = parse_f64_opt(entry.get("h")).filter(|&v| v > 0.0);
                    let low = parse_f64_opt(entry.get("l")).filter(|&v| v > 0.0);
                    let volume = entry
                        .get("v")
                        .and_then(|v| v.as_str())
                        .and_then(|s| s.parse::<u64>().ok());

                    let change = price - prev_close;
                    let change_percent = if prev_close > 0.0 {
                        (change / prev_close) * 100.0
                    } else {
                        0.0
                    };

                    let key = format!("{:?}:{}", Market::Tw, code);
                    result.insert(
                        key,
                        QuoteData {
                            symbol: code,
                            display_name: name,
                            market: Market::Tw,
                            price,
                            prev_close,
                            change,
                            change_percent,
                            open,
                            high,
                            low,
                            volume,
                            updated_at: updated_at.clone(),
                            source: "TWSE",
                            trading_period: None,
                        },
                    );
                }
            }
        }
    }

    // 3. Fallback: Yahoo Finance for any still-missing TW symbols
    for item in items {
        let key = item.key();
        if !result.contains_key(&key) {
            let code = item.symbol.trim().to_uppercase();
            if let Ok(q) = fetch_yahoo_single(agent, &format!("{code}.TW"), item, false)
                .or_else(|_| fetch_yahoo_single(agent, &format!("{code}.TWO"), item, false))
            {
                result.insert(key, q);
            }
        }
    }

    if result.is_empty() && !items.is_empty() {
        return Err("台股報價連線失敗".to_string());
    }

    Ok(result)
}

fn fetch_cnyes_tw_batch(
    agent: &ureq::Agent,
    items: &[&StockItem],
) -> Result<HashMap<String, QuoteData>, String> {
    let syms: Vec<String> = items
        .iter()
        .map(|i| format!("TWS:{}:STOCK", i.symbol.trim().to_uppercase()))
        .collect();
    let ts = Utc::now().timestamp_millis();
    let url = format!(
        "https://ws.api.cnyes.com/ws/api/v1/quote/quotes/{}?_={ts}",
        syms.join(",")
    );

    let resp = agent
        .get(&url)
        .set("Cache-Control", "no-cache")
        .call()
        .map_err(|e| format!("Cnyes 連線失敗: {e}"))?;

    let body: Value = resp
        .into_json()
        .map_err(|e| format!("Cnyes JSON 解析失敗: {e}"))?;

    let mut result = HashMap::new();
    let updated_at = now_hms();
    if let Some(arr) = body.get("data").and_then(|v| v.as_array()) {
        for entry in arr {
            let raw_sym = match entry.get("0").and_then(|v| v.as_str()) {
                Some(s) => s,
                None => continue,
            };
            let code = raw_sym
                .strip_prefix("TWS:")
                .and_then(|s| s.strip_suffix(":STOCK"))
                .unwrap_or(raw_sym)
                .to_uppercase();
            if code.is_empty() {
                continue;
            }
            let price = match parse_f64_opt(entry.get("6")).filter(|&p| p > 0.0) {
                Some(p) => p,
                None => continue,
            };
            let prev_close = parse_f64_opt(entry.get("21"))
                .filter(|&p| p > 0.0)
                .unwrap_or(price);
            let name = entry
                .get("200009")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .unwrap_or(&code)
                .to_string();
            let open = parse_f64_opt(entry.get("19")).filter(|&v| v > 0.0);
            let high = parse_f64_opt(entry.get("12")).filter(|&v| v > 0.0);
            let low = parse_f64_opt(entry.get("13")).filter(|&v| v > 0.0);
            let volume = entry.get("800001").and_then(|v| v.as_u64());

            let change = price - prev_close;
            let change_percent = if prev_close > 0.0 {
                (change / prev_close) * 100.0
            } else {
                0.0
            };

            let key = format!("{:?}:{}", Market::Tw, code);
            result.insert(
                key,
                QuoteData {
                    symbol: code,
                    display_name: name,
                    market: Market::Tw,
                    price,
                    prev_close,
                    change,
                    change_percent,
                    open,
                    high,
                    low,
                    volume,
                    updated_at: updated_at.clone(),
                    source: "Cnyes",
                    trading_period: None,
                },
            );
        }
    }

    Ok(result)
}

/// Fetch a single symbol from Yahoo Finance v8 chart API (no login / no crumb required).
/// When `include_extended` is true, requests pre/post-market bars and uses the latest
/// extended-hours close if its timestamp is newer than `regularMarketTime`.
pub fn fetch_yahoo_single(
    agent: &ureq::Agent,
    yahoo_symbol: &str,
    item: &StockItem,
    include_extended: bool,
) -> Result<QuoteData, String> {
    let encoded = yahoo_symbol.replace('^', "%5E");
    let ext_param = if include_extended {
        "&includePrePost=true"
    } else {
        ""
    };
    let ts = Utc::now().timestamp_millis();
    let url = format!(
        "https://query1.finance.yahoo.com/v8/finance/chart/{encoded}?interval=1m&range=1d{ext_param}&_={ts}"
    );

    let resp = agent
        .get(&url)
        .set("Cache-Control", "no-cache")
        .call()
        .map_err(|e| format!("Yahoo ({yahoo_symbol}) 連線失敗: {e}"))?;

    let body: Value = resp
        .into_json()
        .map_err(|e| format!("Yahoo ({yahoo_symbol}) JSON 解析失敗: {e}"))?;

    let meta = body
        .pointer("/chart/result/0/meta")
        .ok_or_else(|| format!("找不到 {yahoo_symbol} 報價資料"))?;

    let reg_price = parse_f64_opt(meta.get("regularMarketPrice"))
        .ok_or_else(|| format!("{yahoo_symbol} 無現價資料"))?;
    let reg_time = meta
        .get("regularMarketTime")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    let price = if include_extended {
        latest_extended_price(&body, reg_time).unwrap_or(reg_price)
    } else {
        reg_price
    };
    let prev_close = parse_f64_opt(meta.get("chartPreviousClose"))
        .or_else(|| parse_f64_opt(meta.get("previousClose")))
        .unwrap_or(price);

    let change = price - prev_close;
    let change_percent = if prev_close > 0.0 {
        (change / prev_close) * 100.0
    } else {
        0.0
    };

    let display_name = if !item.name.is_empty() && item.name != item.symbol {
        item.name.clone()
    } else {
        meta.get("shortName")
            .or_else(|| meta.get("longName"))
            .and_then(|v| v.as_str())
            .unwrap_or(&item.symbol)
            .to_string()
    };

    let high = parse_f64_opt(meta.get("regularMarketDayHigh")).filter(|&v| v > 0.0);
    let low = parse_f64_opt(meta.get("regularMarketDayLow")).filter(|&v| v > 0.0);
    let open = body
        .pointer("/chart/result/0/indicators/quote/0/open/0")
        .and_then(|v| v.as_f64())
        .filter(|&v| v > 0.0);
    let volume = meta
        .get("regularMarketVolume")
        .and_then(|v| v.as_u64());

    let trading_period = parse_trading_period(meta);

    Ok(QuoteData {
        symbol: item.symbol.to_uppercase(),
        display_name,
        market: item.market,
        price,
        prev_close,
        change,
        change_percent,
        open,
        high,
        low,
        volume,
        updated_at: now_hms(),
        source: "Yahoo",
        trading_period,
    })
}

/// 解析 Yahoo Finance `meta.currentTradingPeriod`
pub fn parse_trading_period(meta: &Value) -> Option<TradingPeriod> {
    let ctp = meta.get("currentTradingPeriod")?;
    let bounds = |key: &str| -> (i64, i64) {
        let obj = ctp.get(key);
        let s = obj.and_then(|o| o.get("start")).and_then(|v| v.as_i64()).unwrap_or(0);
        let e = obj.and_then(|o| o.get("end")).and_then(|v| v.as_i64()).unwrap_or(0);
        (s, e)
    };
    let (pre_start, pre_end) = bounds("pre");
    let (reg_start, reg_end) = bounds("regular");
    let (post_start, post_end) = bounds("post");
    if reg_start <= 0 || reg_end <= 0 {
        return None;
    }
    Some(TradingPeriod {
        pre_start,
        pre_end,
        reg_start,
        reg_end,
        post_start,
        post_end,
    })
}

/// 查詢指定市場指數（如 `^TWII`、`^GSPC`）的最新實際交易時段
pub fn fetch_market_trading_period(agent: &ureq::Agent, yahoo_symbol: &str) -> Option<TradingPeriod> {
    let encoded = yahoo_symbol.replace('^', "%5E");
    let url = format!(
        "https://query1.finance.yahoo.com/v8/finance/chart/{encoded}?interval=1d&range=1d"
    );
    let resp = agent.get(&url).call().ok()?;
    let body: Value = resp.into_json().ok()?;
    let meta = body.pointer("/chart/result/0/meta")?;
    parse_trading_period(meta)
}

/// 從 Yahoo chart JSON 取得比 `regularMarketTime` 更新的盤前/盤後最新成交價
fn latest_extended_price(body: &Value, reg_time: i64) -> Option<f64> {
    let timestamps = body.pointer("/chart/result/0/timestamp")?.as_array()?;
    let closes = body
        .pointer("/chart/result/0/indicators/quote/0/close")?
        .as_array()?;
    let n = timestamps.len().min(closes.len());
    for i in (0..n).rev() {
        if let Some(p) = closes[i].as_f64().filter(|&v| v > 0.0) {
            let ts = timestamps[i].as_i64().unwrap_or(0);
            if ts > reg_time {
                return Some(p);
            }
            break;
        }
    }
    None
}

/// Fetch US stock quotes via Yahoo Finance.
pub fn fetch_us_quotes(
    agent: &ureq::Agent,
    items: &[&StockItem],
    include_extended: bool,
) -> (HashMap<String, QuoteData>, Option<String>) {
    let mut map = HashMap::new();
    let mut last_err = None;
    for item in items {
        let sym = item.symbol.trim().to_uppercase();
        match fetch_yahoo_single(agent, &sym, item, include_extended) {
            Ok(q) => {
                map.insert(item.key(), q);
            }
            Err(e) => {
                last_err = Some(e);
            }
        }
    }
    (map, last_err)
}

/// Search symbols (TW & US) via Yahoo Finance public search endpoint.
pub fn search_symbols(query: &str) -> Result<Vec<SearchResult>, String> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    let agent = build_agent();
    let encoded: String = trimmed
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' => (b as char).to_string(),
            b' ' => "%20".to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect();
    let url = format!(
        "https://query1.finance.yahoo.com/v1/finance/search?q={encoded}&quotesCount=8&newsCount=0"
    );

    let resp = agent
        .get(&url)
        .call()
        .map_err(|e| format!("搜尋連線失敗: {e}"))?;
    let body: Value = resp
        .into_json()
        .map_err(|e| format!("搜尋結果解析失敗: {e}"))?;

    let mut out = Vec::new();
    if let Some(arr) = body.get("quotes").and_then(|v| v.as_array()) {
        for q in arr {
            let raw_sym = match q.get("symbol").and_then(|v| v.as_str()) {
                Some(s) => s,
                None => continue,
            };
            let exchange = q
                .get("exchDisp")
                .or_else(|| q.get("exchange"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let name = q
                .get("shortname")
                .or_else(|| q.get("longname"))
                .and_then(|v| v.as_str())
                .unwrap_or(raw_sym)
                .to_string();

            if let Some(code) = raw_sym
                .strip_suffix(".TW")
                .or_else(|| raw_sym.strip_suffix(".TWO"))
            {
                out.push(SearchResult {
                    symbol: code.to_uppercase(),
                    name,
                    market: Market::Tw,
                    exchange,
                });
            } else if !raw_sym.contains('.') {
                out.push(SearchResult {
                    symbol: raw_sym.to_uppercase(),
                    name,
                    market: Market::Us,
                    exchange,
                });
            }
        }
    }
    Ok(out)
}

/// Heuristic to guess market from user input (digits => TW, letters => US).
pub fn guess_market(symbol: &str) -> Market {
    let s = symbol.trim();
    if !s.is_empty() && s.chars().next().unwrap().is_ascii_digit() {
        Market::Tw
    } else {
        Market::Us
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_live_tw_and_us_quotes() {
        let agent = build_agent();
        let tw_item = StockItem {
            symbol: "2330".into(),
            name: "台積電".into(),
            market: Market::Tw,
            shares: 1000.0,
            avg_cost: 1000.0,
        };
        let us_item = StockItem {
            symbol: "NVDA".into(),
            name: "NVIDIA".into(),
            market: Market::Us,
            shares: 10.0,
            avg_cost: 120.0,
        };

        let tw_map = fetch_tw_batch(&agent, &[&tw_item]).expect("TW fetch failed");
        let tw_q = tw_map.get(&tw_item.key()).expect("2330 quote missing");
        println!("TW 2330: {} @ {:.2} ({:+.2}%) via {}", tw_q.display_name, tw_q.price, tw_q.change_percent, tw_q.source);
        assert!(tw_q.price > 0.0);
        let (tw_pnl, tw_pct) = tw_item.pnl(tw_q.price).expect("TW pnl missing");
        println!("TW 2330 P&L: {:+.0} ({:+.2}%)", tw_pnl, tw_pct);

        let (us_map, us_err) = fetch_us_quotes(&agent, &[&us_item], true);
        let us_q = us_map.get(&us_item.key()).unwrap_or_else(|| panic!("NVDA quote missing: {:?}", us_err));
        println!("US NVDA: {} @ {:.2} ({:+.2}%) via {}", us_q.display_name, us_q.price, us_q.change_percent, us_q.source);
        assert!(us_q.price > 0.0);
        let (us_pnl, us_pct) = us_item.pnl(us_q.price).expect("US pnl missing");
        println!("US NVDA P&L: {:+.2} ({:+.2}%)", us_pnl, us_pct);
    }
}
