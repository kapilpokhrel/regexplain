mod types;

fn main() {
    let pattern = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: regexplain <pattern>");
        std::process::exit(1);
    });
    println!("pattern: {}", pattern);
}
