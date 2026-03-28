//! # ACE-Time
//!
//! Utilitarios de data/hora em UTC sem dependencia de `chrono`.
//! Implementa:
//! - Wrapper de `SystemTime`: now(), duration_since_epoch(), unix timestamps
//! - Formatacao ISO 8601
//! - Formatacao HTTP-date (RFC 7231 IMF-fixdate)
//! - Parse de datas HTTP (IMF-fixdate, RFC 850 e asctime)
//! - Calculo de weekday, dias por mes e leap year

use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Weekday {
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
    Sat,
    Sun,
}

impl Weekday {
    pub fn short_name(self) -> &'static str {
        match self {
            Weekday::Mon => "Mon",
            Weekday::Tue => "Tue",
            Weekday::Wed => "Wed",
            Weekday::Thu => "Thu",
            Weekday::Fri => "Fri",
            Weekday::Sat => "Sat",
            Weekday::Sun => "Sun",
        }
    }

    pub fn long_name(self) -> &'static str {
        match self {
            Weekday::Mon => "Monday",
            Weekday::Tue => "Tuesday",
            Weekday::Wed => "Wednesday",
            Weekday::Thu => "Thursday",
            Weekday::Fri => "Friday",
            Weekday::Sat => "Saturday",
            Weekday::Sun => "Sunday",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateTimeParts {
    pub year: i32,
    pub month: u8,  // 1..=12
    pub day: u8,    // 1..=31
    pub hour: u8,   // 0..=23
    pub minute: u8, // 0..=59
    pub second: u8, // 0..=59
}

impl DateTimeParts {
    pub fn weekday(&self) -> Weekday {
        weekday_from_ymd(self.year, self.month, self.day)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AceTime(pub SystemTime);

impl AceTime {
    pub fn now() -> Self {
        Self(SystemTime::now())
    }

    pub fn from_system_time(time: SystemTime) -> Self {
        Self(time)
    }

    pub fn into_system_time(self) -> SystemTime {
        self.0
    }

    pub fn duration_since_epoch(self) -> Duration {
        self.0
            .duration_since(UNIX_EPOCH)
            .unwrap_or_else(|_| Duration::from_secs(0))
    }

    pub fn unix_timestamp(self) -> i64 {
        self.duration_since_epoch().as_secs() as i64
    }

    pub fn unix_timestamp_millis(self) -> i64 {
        self.duration_since_epoch().as_millis() as i64
    }

    pub fn format_iso8601(self) -> String {
        format_iso8601(self.0)
    }

    pub fn format_http_date(self) -> String {
        format_http_date(self.0)
    }
}

pub fn now() -> AceTime {
    AceTime::now()
}

pub fn duration_since_epoch(time: SystemTime) -> Duration {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
}

pub fn unix_timestamp(time: SystemTime) -> i64 {
    duration_since_epoch(time).as_secs() as i64
}

pub fn unix_timestamp_millis(time: SystemTime) -> i64 {
    duration_since_epoch(time).as_millis() as i64
}

pub fn format_iso8601(time: SystemTime) -> String {
    let d = unix_seconds_to_datetime(unix_timestamp(time));
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        d.year, d.month, d.day, d.hour, d.minute, d.second
    )
}

pub fn format_http_date(time: SystemTime) -> String {
    let d = unix_seconds_to_datetime(unix_timestamp(time));
    let w = d.weekday().short_name();
    let m = month_short_name(d.month);
    format!(
        "{}, {:02} {} {:04} {:02}:{:02}:{:02} GMT",
        w, d.day, m, d.year, d.hour, d.minute, d.second
    )
}

pub fn parse_http_date(input: &str) -> Result<SystemTime, &'static str> {
    parse_imf_fixdate(input)
        .or_else(|_| parse_rfc850_date(input))
        .or_else(|_| parse_asctime_date(input))
}

pub fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

pub fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

pub fn weekday_from_ymd(year: i32, month: u8, day: u8) -> Weekday {
    // 1970-01-01 foi quinta (Thu)
    let days = days_from_civil(year, month, day);
    match (days + 3).rem_euclid(7) {
        0 => Weekday::Mon,
        1 => Weekday::Tue,
        2 => Weekday::Wed,
        3 => Weekday::Thu,
        4 => Weekday::Fri,
        5 => Weekday::Sat,
        _ => Weekday::Sun,
    }
}

fn parse_imf_fixdate(input: &str) -> Result<SystemTime, &'static str> {
    // Sun, 06 Nov 1994 08:49:37 GMT
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.len() != 6 {
        return Err("Invalid IMF-fixdate");
    }
    let _weekday = parts[0].trim_end_matches(',');
    let day = parse_u8(parts[1])?;
    let month = parse_month(parts[2])?;
    let year = parse_i32(parts[3])?;
    let (h, m, s) = parse_hms(parts[4])?;
    if parts[5] != "GMT" {
        return Err("Invalid timezone in HTTP date");
    }
    validate_ymdhms(year, month, day, h, m, s)?;
    datetime_to_system_time(year, month, day, h, m, s)
}

fn parse_rfc850_date(input: &str) -> Result<SystemTime, &'static str> {
    // Sunday, 06-Nov-94 08:49:37 GMT
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.len() != 4 {
        return Err("Invalid RFC 850 date");
    }
    let _weekday_long = parts[0].trim_end_matches(',');
    let dmy: Vec<&str> = parts[1].split('-').collect();
    if dmy.len() != 3 {
        return Err("Invalid RFC 850 date body");
    }
    let day = parse_u8(dmy[0])?;
    let month = parse_month(dmy[1])?;
    let yy = parse_u8(dmy[2])? as i32;
    // RFC 7231: 2-digit year interpreted with 50-year sliding window
    let year = if yy >= 70 { 1900 + yy } else { 2000 + yy };
    let (h, m, s) = parse_hms(parts[2])?;
    if parts[3] != "GMT" {
        return Err("Invalid timezone in HTTP date");
    }
    validate_ymdhms(year, month, day, h, m, s)?;
    datetime_to_system_time(year, month, day, h, m, s)
}

fn parse_asctime_date(input: &str) -> Result<SystemTime, &'static str> {
    // Sun Nov  6 08:49:37 1994
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.len() != 5 {
        return Err("Invalid asctime date");
    }
    let _weekday = parts[0];
    let month = parse_month(parts[1])?;
    let day = parse_u8(parts[2])?;
    let (h, m, s) = parse_hms(parts[3])?;
    let year = parse_i32(parts[4])?;
    validate_ymdhms(year, month, day, h, m, s)?;
    datetime_to_system_time(year, month, day, h, m, s)
}

fn parse_hms(hms: &str) -> Result<(u8, u8, u8), &'static str> {
    let segs: Vec<&str> = hms.split(':').collect();
    if segs.len() != 3 {
        return Err("Invalid time");
    }
    let h = parse_u8(segs[0])?;
    let m = parse_u8(segs[1])?;
    let s = parse_u8(segs[2])?;
    Ok((h, m, s))
}

fn parse_u8(s: &str) -> Result<u8, &'static str> {
    s.parse::<u8>().map_err(|_| "Invalid number")
}

fn parse_i32(s: &str) -> Result<i32, &'static str> {
    s.parse::<i32>().map_err(|_| "Invalid number")
}

fn parse_month(m: &str) -> Result<u8, &'static str> {
    match m {
        "Jan" => Ok(1),
        "Feb" => Ok(2),
        "Mar" => Ok(3),
        "Apr" => Ok(4),
        "May" => Ok(5),
        "Jun" => Ok(6),
        "Jul" => Ok(7),
        "Aug" => Ok(8),
        "Sep" => Ok(9),
        "Oct" => Ok(10),
        "Nov" => Ok(11),
        "Dec" => Ok(12),
        _ => Err("Invalid month"),
    }
}

fn month_short_name(month: u8) -> &'static str {
    match month {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "???",
    }
}

fn validate_ymdhms(
    year: i32,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
) -> Result<(), &'static str> {
    if !(1..=12).contains(&month) {
        return Err("Invalid month");
    }
    let max_day = days_in_month(year, month);
    if day == 0 || day > max_day {
        return Err("Invalid day");
    }
    if hour > 23 || minute > 59 || second > 59 {
        return Err("Invalid time");
    }
    Ok(())
}

fn datetime_to_system_time(
    year: i32,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
) -> Result<SystemTime, &'static str> {
    let days = days_from_civil(year, month, day);
    if days < 0 {
        return Err("Dates before 1970 are unsupported");
    }
    let secs = (days as i64) * 86_400 + (hour as i64) * 3600 + (minute as i64) * 60 + second as i64;
    Ok(UNIX_EPOCH + Duration::from_secs(secs as u64))
}

fn unix_seconds_to_datetime(unix_secs: i64) -> DateTimeParts {
    let days = unix_secs.div_euclid(86_400);
    let sec_of_day = unix_secs.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = (sec_of_day / 3600) as u8;
    let minute = ((sec_of_day % 3600) / 60) as u8;
    let second = (sec_of_day % 60) as u8;
    DateTimeParts {
        year,
        month,
        day,
        hour,
        minute,
        second,
    }
}

// Howard Hinnant algorithms (UTC, proleptic Gregorian)
fn days_from_civil(year: i32, month: u8, day: u8) -> i64 {
    let y = year - if month <= 2 { 1 } else { 0 };
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let yoe = y - era * 400;
    let m = month as i32;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + day as i32 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    (era * 146097 + doe - 719468) as i64
}

fn civil_from_days(days: i64) -> (i32, u8, u8) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    let year = (y + if m <= 2 { 1 } else { 0 }) as i32;
    (year, m as u8, d as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_leap_year_and_days_in_month() {
        assert!(is_leap_year(2000));
        assert!(!is_leap_year(1900));
        assert!(is_leap_year(2024));
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(2023, 2), 28);
        assert_eq!(days_in_month(2023, 11), 30);
        assert_eq!(days_in_month(2023, 12), 31);
    }

    #[test]
    fn test_weekday() {
        // 1970-01-01 Thu
        assert_eq!(weekday_from_ymd(1970, 1, 1), Weekday::Thu);
        // 2026-03-27 Fri
        assert_eq!(weekday_from_ymd(2026, 3, 27), Weekday::Fri);
    }

    #[test]
    fn test_iso_and_http_format() {
        let t = UNIX_EPOCH + Duration::from_secs(784_111_777); // 1994-11-06 08:49:37 UTC
        assert_eq!(format_iso8601(t), "1994-11-06T08:49:37Z");
        assert_eq!(format_http_date(t), "Sun, 06 Nov 1994 08:49:37 GMT");
    }

    #[test]
    fn test_parse_http_date_variants() {
        let a = parse_http_date("Sun, 06 Nov 1994 08:49:37 GMT").unwrap();
        let b = parse_http_date("Sunday, 06-Nov-94 08:49:37 GMT").unwrap();
        let c = parse_http_date("Sun Nov  6 08:49:37 1994").unwrap();
        assert_eq!(a, b);
        assert_eq!(b, c);
        assert_eq!(format_http_date(a), "Sun, 06 Nov 1994 08:49:37 GMT");
    }

    #[test]
    fn test_parse_invalid_http_date() {
        assert!(parse_http_date("Sun, 06 Nov 1994 08:49:37 PST").is_err());
        assert!(parse_http_date("Sun, 32 Nov 1994 08:49:37 GMT").is_err());
        assert!(parse_http_date("garbage").is_err());
    }
}
