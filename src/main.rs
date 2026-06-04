use crate::desc::{DescGenerator, Describer};
use crate::colorize::{ColorGenerator, Colorizer};

mod types;
mod convert;
mod desc;
mod colorize;
mod tree;
mod tui;


fn main() {
    let pattern = match std::env::args().nth(1) {
        Some(p) => p,
        None => {
            if let Err(e) = tui::run() {
                eprintln!("tui error: {}", e);
                std::process::exit(1);
            }
            return;
        }
    };

    match convert::parse_and_convert(&pattern) {
        Ok(form) => {
            let mut color_generator = ColorGenerator::new();
            color_generator.colorize(&form.root);
            println!("{}", colorize::render_colored(0, &form.pattern, &color_generator, false));
            println!();

            let mut desc_generator = DescGenerator::new();
            desc_generator.describe(form.root).print(form.pattern, 0, &color_generator);
        }
        Err(e) => {
            eprintln!("parse error: {}", e);
            std::process::exit(1);
        }
    }
}
