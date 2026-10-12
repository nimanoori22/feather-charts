//! Interactive candlestick chart; pass --smoke for headless validation.
#[allow(dead_code)]
#[path = "support/chart_demo.rs"]
mod chart_demo;
fn main() -> iced::Result {
    chart_demo::run(true)
}
