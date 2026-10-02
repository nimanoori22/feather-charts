use crate::formatters::format_date::{UtcDateTime, format_date};
#[derive(Clone, Debug)]
pub struct DateFormatter {
    format: String,
    locale: String,
}
impl DateFormatter {
    pub fn new(format: Option<impl Into<String>>, locale: Option<impl Into<String>>) -> Self {
        Self {
            format: format
                .map(Into::into)
                .unwrap_or_else(|| "yyyy-MM-dd".into()),
            locale: locale.map(Into::into).unwrap_or_else(|| "default".into()),
        }
    }
    pub fn format(&self, date: &UtcDateTime) -> String {
        format_date(date, &self.format, &self.locale)
    }
}
