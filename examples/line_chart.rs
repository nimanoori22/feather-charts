//! Interactive line chart; pass --smoke for headless validation.
#[path = "support/chart_demo.rs"]
mod chart_demo;
fn main() -> iced::Result {
    chart_demo::run(false)
}
