//! 台股與美股開盤時段偵測模組
//! 用於智慧省電：非開盤時段自動停止背景輪詢，減少網路喚醒與電力消耗。

use chrono::{Datelike, FixedOffset, Timelike, Utc, Weekday};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsPhase {
    Regular,
    Extended,
    Closed,
}

#[derive(Debug, Clone, Copy)]
pub struct MarketStatus {
    pub tw_open: bool,
    pub us_open: bool,
    pub us_phase: UsPhase,
}

/// 由 Yahoo Finance `meta.currentTradingPeriod` 解析出的實際交易時段（Unix 秒）
/// 可自動反映台美股國定假日休市、颱風假與美股提前收盤。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TradingPeriod {
    pub pre_start: i64,
    pub pre_end: i64,
    pub reg_start: i64,
    pub reg_end: i64,
    pub post_start: i64,
    pub post_end: i64,
}

impl TradingPeriod {
    /// 判斷目前時間是否處於台股正規交易時段
    pub fn is_tw_open(self, now_ts: i64) -> bool {
        self.reg_start > 0 && now_ts >= self.reg_start && now_ts <= self.reg_end
    }

    /// 判斷目前時間是否處於美股交易時段（含或不含盤前/盤後）
    pub fn is_us_open(self, now_ts: i64, include_extended: bool) -> bool {
        let in_reg = self.reg_start > 0 && now_ts >= self.reg_start && now_ts <= self.reg_end;
        if !include_extended {
            return in_reg;
        }
        let in_pre = self.pre_start > 0 && now_ts >= self.pre_start && now_ts <= self.pre_end;
        let in_post = self.post_start > 0 && now_ts >= self.post_start && now_ts <= self.post_end;
        in_pre || in_reg || in_post
    }
}

/// Check if a UTC date is within US Eastern Daylight Time (2nd Sunday in March to 1st Sunday in November).
fn is_us_dst(utc: chrono::DateTime<Utc>) -> bool {
    let month = utc.month();
    if month < 3 || month > 11 {
        return false;
    }
    if month > 3 && month < 11 {
        return true;
    }
    let day = utc.day();
    let weekday_from_sun = utc.weekday().num_days_from_sunday();
    if month == 3 {
        let prev_sunday = day as i32 - weekday_from_sun as i32;
        if prev_sunday > 8 {
            return true;
        }
        if prev_sunday == 8 {
            return utc.hour() >= 7;
        }
        false
    } else {
        let prev_sunday = day as i32 - weekday_from_sun as i32;
        if prev_sunday < 1 {
            return true;
        }
        if prev_sunday == 1 && weekday_from_sun == 0 {
            return utc.hour() < 6;
        }
        false
    }
}

pub fn current_market_status(include_us_extended: bool) -> MarketStatus {
    let now_utc = Utc::now();

    // Taiwan: UTC+8, Mon-Fri 08:55 - 13:35
    let tw_offset = FixedOffset::east_opt(8 * 3600).unwrap();
    let tw_now = now_utc.with_timezone(&tw_offset);
    let tw_weekday = !matches!(tw_now.weekday(), Weekday::Sat | Weekday::Sun);
    let tw_mins = tw_now.hour() * 60 + tw_now.minute();
    let tw_open = tw_weekday && (8 * 60 + 55..=13 * 60 + 35).contains(&tw_mins);

    // US Eastern: UTC-4 (DST) or UTC-5 (EST)
    let us_offset_secs = if is_us_dst(now_utc) {
        -4 * 3600
    } else {
        -5 * 3600
    };
    let us_offset = FixedOffset::east_opt(us_offset_secs).unwrap();
    let us_now = now_utc.with_timezone(&us_offset);
    let us_weekday = !matches!(us_now.weekday(), Weekday::Sat | Weekday::Sun);
    let us_mins = us_now.hour() * 60 + us_now.minute();

    let us_regular = us_weekday && (9 * 60 + 30..=16 * 60 + 5).contains(&us_mins);
    let us_extended = us_weekday && (4 * 60..=20 * 60).contains(&us_mins);

    let (us_open, us_phase) = if us_regular {
        (true, UsPhase::Regular)
    } else if include_us_extended && us_extended {
        (true, UsPhase::Extended)
    } else {
        (false, UsPhase::Closed)
    };

    MarketStatus {
        tw_open,
        us_open,
        us_phase,
    }
}
