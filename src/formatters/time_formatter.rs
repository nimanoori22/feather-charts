use crate::formatters::format_date::UtcDateTime;
use chrono::Timelike;
#[derive(Clone, Debug)]
pub struct TimeFormatter {
    format: String,
}
impl TimeFormatter {
    pub fn new(format: Option<impl Into<String>>) -> Self {
        Self {
            format: format.map(Into::into).unwrap_or_else(|| "%h:%m:%s".into()),
        }
    }
    pub fn format(&self, date: &UtcDateTime) -> String {
        let value = date.inner();
        self.format
            .replace("%h", &format!("{:02}", value.hour()))
            .replace("%m", &format!("{:02}", value.minute()))
            .replace("%s", &format!("{:02}", value.second()))
    }
}

#[cfg(test)]
mod tests {
    use super::TimeFormatter;
    use crate::formatters::format_date::UtcDateTime;
    #[test]
    fn formats_utc_midnight_and_year_rollover() {
        let value = UtcDateTime::from_unix_timestamp(1_577_836_800.0).unwrap();
        assert_eq!(
            TimeFormatter::new(None::<String>).format(&value),
            "00:00:00"
        );
    }
}
