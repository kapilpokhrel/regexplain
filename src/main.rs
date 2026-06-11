mod types;
mod convert;
mod desc;
mod colorize;
mod tree;
mod tui;
mod textarea;


fn main() {
    if let Err(e) = tui::run() {
        eprintln!("tui error: {}", e);
        std::process::exit(1);
    }
}
