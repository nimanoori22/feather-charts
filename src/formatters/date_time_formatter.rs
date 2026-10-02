use crate::formatters::{
    date_formatter::DateFormatter, format_date::UtcDateTime, time_formatter::TimeFormatter,
};
#[derive(Clone, Debug)]
pub struct DateTimeFormatter {
    date: DateFormatter,
    time: TimeFormatter,
    separator: String,
}
impl DateTimeFormatter {
    pub fn new(
        date_format: Option<impl Into<String>>,
        time_format: Option<impl Into<String>>,
        separator: Option<impl Into<String>>,
        locale: Option<impl Into<String>>,
    ) -> Self {
        Self {
            date: DateFormatter::new(date_format, locale),
            time: TimeFormatter::new(time_format),
            separator: separator.map(Into::into).unwrap_or_else(|| " ".into()),
        }
    }
    pub fn format(&self, date: &UtcDateTime) -> String {
        format!(
            "{}{}{}",
            self.date.format(date),
            self.separator,
            self.time.format(date)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::DateTimeFormatter;
    use crate::formatters::format_date::UtcDateTime;
    #[test]
    fn joins_configured_date_and_time_tokens() {
        let value = UtcDateTime::from_unix_timestamp(1_577_836_800.0).unwrap();
        let formatter =
            DateTimeFormatter::new(Some("dd/MM/yyyy"), Some("%h:%m"), Some("T"), Some("en-US"));
        assert_eq!(formatter.format(&value), "01/01/2020T00:00");
    }
}
