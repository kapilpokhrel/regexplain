use ratatui::text::Span;
use crossterm::style::{ContentStyle, StyledContent};
use ratatui::backend::IntoCrossterm;

use crate::{colorize::{ColorGenerator, Colorizer}, convert, desc::{DescGenerator, Describer, RegexDescriptionNode}};

trait CliPrint {
    fn print(&self, pattern: &str, indent: usize, cgen: &ColorGenerator);
}

impl CliPrint for RegexDescriptionNode {
    fn print(&self, pattern: &str, indent: usize, cgen: &ColorGenerator) {
        let colored_spans = cgen.ratatui_colored_slice(pattern, self.span.start, self.span.end, 1.0);
        if !self.desc.is_empty() {
            println!("{}`{}`{}", " ".repeat(indent), span_to_string(&colored_spans), self.desc);
        }
        let child_indent = if self.desc.is_empty() { indent } else { indent + 1 };
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

pub fn run(pattern: impl Into<String>, _text_to_match: impl Into<String>) {
    match convert::parse_and_convert(&pattern.into()) {
        Ok(form) => {
            let mut cgen = ColorGenerator::new();
            cgen.colorize(&form);


            let colored_pattern_spans = cgen.ratatui_colored_slice(&form.pattern, 0, form.pattern.len(), 1.0);
            println!("{}\n", span_to_string(&colored_pattern_spans));
            let regex_desc_root = DescGenerator::new().describe(form.root);
            regex_desc_root.print(&form.pattern, 0, &cgen);
        }
        Err(e) => {
            println!("{}", e);
        }
    }
}
