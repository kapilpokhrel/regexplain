mod types;
mod convert;
mod desc;

fn main() {
    let pattern = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: regexplain <pattern>");
        std::process::exit(1);
    });

    match convert::parse_and_convert(&pattern) {
        Ok(form) => {
            println!("pattern: {}\n", form.pattern);
            println!("{:#?}", form.root);
            println!("\nExplanation:\n");
            desc::DescNode::from(form.root).print(0);
        }
        Err(e) => {
            eprintln!("parse error: {}", e);
            std::process::exit(1);
        }
    }
}
