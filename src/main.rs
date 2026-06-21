mod types;
mod convert;
mod desc;
mod colorize;
mod tui;
mod cli;

use std::path::PathBuf;
use std::fs;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct CliArgs {
    /// regex pattern
    #[arg(short, long, default_value_t = String::default())]
    pattern: String,

    /// text to match
    #[arg(short, long, conflicts_with="file_to_match")]
    text_to_match: Option<String>,

    /// takes text to match form the contents of the file
    #[arg(short, long, conflicts_with="text_to_match")]
    file_to_match: Option<PathBuf>,

    #[arg(short, long, requires="pattern", default_value_t = false)]
    no_tui: bool
}

fn main() {
    let args = CliArgs::parse();
    let initial_text_to_match = if let Some(raw_text) = args.text_to_match {
        raw_text
    } else if let Some(file_path) = args.file_to_match {
        fs::read_to_string(&file_path).unwrap_or_else(|err| {
            eprintln!("Error reading file '{:?}': {}", file_path, err);
            std::process::exit(1);
        })
    } else {
        String::new()
    };

    if args.no_tui {
        crate::cli::run(args.pattern, initial_text_to_match);
    } else {
        if let Err(e) = crate::tui::app::run(args.pattern, initial_text_to_match) {
            eprintln!("tui error: {}", e);
            std::process::exit(1);
        }
    }
}
