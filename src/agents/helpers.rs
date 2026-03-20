use chrono::{DateTime, Utc};

/// Re-export from the shared formatting module for backwards compatibility.
pub fn format_relative_time(dt: &DateTime<Utc>) -> String {
    crate::formatting::format_relative_time(dt)
}

pub fn calculate_release_frequency(dates: &[DateTime<Utc>]) -> String {
    if dates.len() < 2 {
        return "\u{2014}".to_string(); // em dash
    }
    let intervals: Vec<i64> = dates
        .windows(2)
        .map(|w| (w[0] - w[1]).num_hours().abs())
        .collect();
    let avg_hours = intervals.iter().sum::<i64>() / intervals.len() as i64;

    if avg_hours < 1 {
        "~<1h".to_string()
    } else if avg_hours < 24 {
        format!("~{}h", avg_hours)
    } else if avg_hours < 24 * 7 {
        format!("~{}d", avg_hours / 24)
    } else if avg_hours < 24 * 30 {
        format!("~{}w", avg_hours / (24 * 7))
    } else {
        format!("~{}mo", avg_hours / (24 * 30))
    }
}

pub fn is_within_24h(dt: &DateTime<Utc>) -> bool {
    Utc::now().signed_duration_since(*dt).num_hours() < 24
}

/// Re-export from the shared formatting module for backwards compatibility.
pub fn parse_date(date_str: &str) -> Option<DateTime<Utc>> {
    crate::formatting::parse_date(date_str)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn format_relative_time_minutes() {
        let dt = Utc::now() - Duration::minutes(5);
        assert_eq!(format_relative_time(&dt), "5m ago");
    }

    #[test]
    fn format_relative_time_minimum_one_minute() {
        let dt = Utc::now() - Duration::seconds(10);
        assert_eq!(format_relative_time(&dt), "1m ago");
    }

    #[test]
    fn format_relative_time_hours() {
        let dt = Utc::now() - Duration::hours(3);
        assert_eq!(format_relative_time(&dt), "3h ago");
    }

    #[test]
    fn format_relative_time_days() {
        let dt = Utc::now() - Duration::days(2);
        assert_eq!(format_relative_time(&dt), "2d ago");
    }

    #[test]
    fn format_relative_time_weeks() {
        let dt = Utc::now() - Duration::weeks(3);
        assert_eq!(format_relative_time(&dt), "3w ago");
    }

    #[test]
    fn format_relative_time_months() {
        let dt = Utc::now() - Duration::days(90);
        assert_eq!(format_relative_time(&dt), "3mo ago");
    }

    #[test]
    fn release_frequency_regular_daily() {
        let now = Utc::now();
        let dates: Vec<DateTime<Utc>> = (0..5).map(|i| now - Duration::days(i)).collect();
        assert_eq!(calculate_release_frequency(&dates), "~1d");
    }

    #[test]
    fn release_frequency_regular_weekly() {
        let now = Utc::now();
        let dates: Vec<DateTime<Utc>> = (0..4).map(|i| now - Duration::weeks(i)).collect();
        assert_eq!(calculate_release_frequency(&dates), "~1w");
    }

    #[test]
    fn release_frequency_too_few_dates() {
        assert_eq!(calculate_release_frequency(&[]), "\u{2014}");
        assert_eq!(calculate_release_frequency(&[Utc::now()]), "\u{2014}");
    }

    #[test]
    fn is_within_24h_recent() {
        let dt = Utc::now() - Duration::hours(1);
        assert!(is_within_24h(&dt));
    }

    #[test]
    fn is_within_24h_old() {
        let dt = Utc::now() - Duration::hours(25);
        assert!(!is_within_24h(&dt));
    }

    #[test]
    fn parse_date_iso() {
        let result = parse_date("2024-06-15");
        assert!(result.is_some());
        let dt = result.unwrap();
        assert_eq!(dt.format("%Y-%m-%d").to_string(), "2024-06-15");
    }

    #[test]
    fn parse_date_rfc3339() {
        let result = parse_date("2024-06-15T12:30:00Z");
        assert!(result.is_some());
        let dt = result.unwrap();
        assert_eq!(
            dt.format("%Y-%m-%dT%H:%M:%S").to_string(),
            "2024-06-15T12:30:00"
        );
    }

    #[test]
    fn parse_date_rfc3339_positive_offset() {
        let result = parse_date("2024-06-15T12:30:00+05:30");
        assert!(result.is_some());
        let dt = result.unwrap();
        assert_eq!(
            dt.format("%Y-%m-%dT%H:%M:%S").to_string(),
            "2024-06-15T07:00:00"
        );
    }

    #[test]
    fn parse_date_rfc3339_negative_offset() {
        let result = parse_date("2024-06-15T12:30:00-07:00");
        assert!(result.is_some());
        let dt = result.unwrap();
        assert_eq!(
            dt.format("%Y-%m-%dT%H:%M:%S").to_string(),
            "2024-06-15T19:30:00"
        );
    }

    #[test]
    fn parse_date_invalid() {
        assert!(parse_date("not-a-date").is_none());
        assert!(parse_date("").is_none());
    }
}
