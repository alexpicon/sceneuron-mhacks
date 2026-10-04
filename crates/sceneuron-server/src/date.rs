// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The `Date` response header, in the IMF-fixdate format of RFC 9110.

use std::time::{SystemTime, UNIX_EPOCH};

/// Day names starting at Thursday, because 1970-01-01 was a Thursday.
const WEEKDAYS: [&str; 7] = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"];

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

const SECONDS_PER_DAY: u64 = 86_400;

/// The current time, such as `Sun, 06 Nov 1994 08:49:37 GMT`.
pub(crate) fn now() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    format_unix(seconds)
}

fn format_unix(seconds: u64) -> String {
    let days = seconds / SECONDS_PER_DAY;
    let time = seconds % SECONDS_PER_DAY;
    let (year, month, day) = civil_from_days(days);
    let weekday = WEEKDAYS[(days % 7) as usize];
    let month = MONTHS[month - 1];
    let (hour, minute, second) = (time / 3600, time / 60 % 60, time % 60);
    format!("{weekday}, {day:02} {month} {year} {hour:02}:{minute:02}:{second:02} GMT")
}

/// The Gregorian (year, month 1-12, day) that is `days` after 1970-01-01.
///
/// Counts from 0000-03-01 so that leap days fall at the end of each year; every 400 year
/// era has the same 146097 days.
fn civil_from_days(days: u64) -> (u64, usize, u64) {
    let shifted = days + 719_468;
    let era = shifted / 146_097;
    let day_of_era = shifted % 146_097;
    let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let march_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * march_month + 2) / 5 + 1;
    let month = if march_month < 10 {
        march_month + 3
    } else {
        march_month - 9
    };
    let year = era * 400 + year_of_era + u64::from(month <= 2);
    (year, month as usize, day)
}

#[cfg(test)]
mod tests {
    use super::format_unix;

    #[test]
    fn formats_known_instants() {
        assert_eq!(format_unix(0), "Thu, 01 Jan 1970 00:00:00 GMT");
        assert_eq!(format_unix(784_111_777), "Sun, 06 Nov 1994 08:49:37 GMT");
        assert_eq!(format_unix(951_782_400), "Tue, 29 Feb 2000 00:00:00 GMT");
        assert_eq!(format_unix(951_868_799), "Tue, 29 Feb 2000 23:59:59 GMT");
        assert_eq!(format_unix(1_709_164_800), "Thu, 29 Feb 2024 00:00:00 GMT");
        assert_eq!(format_unix(4_102_444_800), "Fri, 01 Jan 2100 00:00:00 GMT");
    }
}
