use chrono::{DateTime, FixedOffset, NaiveDate, TimeZone, Utc};

// Moscow timezone (UTC+3, no DST in Russia since 2014)
pub const MOSCOW_OFFSET_SECONDS: i32 = 3 * 3600;

// Get Moscow timezone
pub fn moscow_tz() -> FixedOffset {
    FixedOffset::east_opt(MOSCOW_OFFSET_SECONDS).unwrap()
}

// Get current time in Moscow timezone
pub fn moscow_now() -> DateTime<FixedOffset> {
    moscow_tz().from_utc_datetime(&Utc::now().naive_utc())
}

// Get current date in Moscow timezone
pub fn moscow_today() -> NaiveDate {
    moscow_now().date_naive()
}

// Convert Moscow date to start of day in RFC3339 format (for DB queries)
pub fn moscow_date_to_rfc3339_start(date: NaiveDate) -> String {
    moscow_tz()
        .from_local_datetime(&date.and_hms_opt(0, 0, 0).unwrap())
        .unwrap()
        .to_rfc3339()
}

// Convert Moscow date to end of day in RFC3339 format (for DB queries)
pub fn moscow_date_to_rfc3339_end(date: NaiveDate) -> String {
    moscow_tz()
        .from_local_datetime(&date.and_hms_opt(23, 59, 59).unwrap())
        .unwrap()
        .to_rfc3339()
}
