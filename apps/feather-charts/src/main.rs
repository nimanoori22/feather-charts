mod app;
mod chart;
mod load;
fn main() -> iced::Result {
    let path = match load::arguments(std::env::args_os().skip(1)) {
        Ok(path) => path,
        Err(message) => {
            eprintln!("{message}");
            if std::env::args_os().any(|s| s == "--help" || s == "-h") {
                return Ok(());
            }
            std::process::exit(2);
        }
    };
    iced::application(
        move || app::App::new(path.clone()),
        app::App::update,
        app::App::view,
    )
    .title("Feather Charts")
    .subscription(app::App::subscription)
    .run()
}
