//! Time helpers. See ARCHITECTURE.md §4:
//! instants are UTC milliseconds, calendar values are local `YYYY-MM-DD`.

use std::time::{SystemTime, UNIX_EPOCH};

use chrono::NaiveDate;

use crate::error::{AppError, AppResult};

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Today's date in the user's local time zone.
pub fn today() -> NaiveDate {
    chrono::Local::now().date_naive()
}

pub fn parse_date(s: &str) -> AppResult<NaiveDate> {
    NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d")
        .map_err(|_| AppError::Validation(format!("\"{s}\" is not a valid date (YYYY-MM-DD)")))
}

pub fn fmt_date(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}
