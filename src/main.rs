mod types;
mod convert;
mod desc;
mod colorize;
mod tui;


fn main() {
    if let Err(e) = crate::tui::app::run() {
        eprintln!("tui error: {}", e);
        std::process::exit(1);
    }
}
