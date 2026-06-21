use crossterm::style::{ContentStyle, StyledContent};
use ratatui::backend::IntoCrossterm;
use ratatui::text::Span;

use crate::{
    colorize::{ColorGenerator, Colorizer},
    convert,
    desc::{DescGenerator, Describer, RegexDescriptionNode},
    matcher::{RegexMatchGrp, eval_regex},
};

trait CliPrint {
    fn print(&self, pattern: &str, indent: usize, cgen: &ColorGenerator);
}

impl CliPrint for RegexDescriptionNode {
    fn print(&self, pattern: &str, indent: usize, cgen: &ColorGenerator) {
        let colored_spans =
            cgen.ratatui_colored_slice(pattern, self.span.start, self.span.end, 1.0);
        if !self.desc.is_empty() {
            println!(
                "{}`{}`{}",
                " ".repeat(indent),
                span_to_string(&colored_spans),
                self.desc
            );
        }
        let child_indent = if self.desc.is_empty() {
            indent
        } else {
            indent + 1
        };
        for child in &self.nested_items {
            child.print(pattern, child_indent, cgen);
        }
    }
}

pub fn span_to_string(spans: &[Span]) -> String {
    let mut s = String::new();
    for span in spans.iter() {
        let cs: ContentStyle = span.style.into_crossterm();
        s.push_str(&format!("{}", StyledContent::new(cs, &span.content)));
    }
    s
}

pub fn run(pattern: impl Into<String>, text_to_match: impl Into<String>) {
    let pattern = pattern.into();
    let text_to_match = text_to_match.into();
    match convert::parse_and_convert(&pattern) {
        Ok(form) => {
            let mut cgen = ColorGenerator::new();
            cgen.colorize(&form);

            let colored_pattern_spans =
                cgen.ratatui_colored_slice(&form.pattern, 0, form.pattern.len(), 1.0);
            println!(" Pattern:\n  {}\n", span_to_string(&colored_pattern_spans));
            let regex_desc_root = DescGenerator::new().describe(form.root);

            println!(" Explanation:");
            regex_desc_root.print(&form.pattern, 2, &cgen);

            if !text_to_match.is_empty() {
                if let Ok(re) = regex::bytes::Regex::new(&pattern) {
                    let matches = eval_regex(&re, &text_to_match);
                    println!();
                    if matches.is_empty() {
                        println!(" no matches");
                    } else {
                        println!(" matches:");
                        for (i, m) in matches.iter().enumerate() {
                            print_match(i, m, &text_to_match, 2);
                        }
                    }
                } else {
                    println!("\n  (could not compile regex)");
                }
            }
        }
        Err(e) => {
            println!("{}", e);
        }
    }
}

fn print_match(index: usize, m: &RegexMatchGrp, text: &str, indent: usize) {
    let matched = &text[m.start_offset..m.end_offset];
    let pad = " ".repeat(indent);
    if m.label.is_empty() {
        println!("{pad}{index}: {matched:?}");
    } else {
        println!("{pad}{index} ({}): {matched:?}", m.label);
    }
    for (i, child) in m.groups.iter().enumerate() {
        print_match(i, child, text, indent + 2);
    }
}
