//! Synchronized candle and volume panes; --smoke verifies the demo headlessly.
#[allow(dead_code)]
#[path = "support/chart_demo.rs"]
mod chart_demo;
fn main() -> iced::Result {
    chart_demo::run_price_volume()
}
