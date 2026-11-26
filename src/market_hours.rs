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
