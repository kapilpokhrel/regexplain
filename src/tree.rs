use std::collections::HashSet;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

// ── Public node type ──────────────────────────────────────────────────────────

pub struct Node {
    pub line: Line<'static>,
    pub children: Vec<Node>,
}

// ── Internal flattened view ───────────────────────────────────────────────────

struct Flat<'a> {
    node: &'a Node,
    path: Vec<usize>,
    depth: usize,
    first_sibling: bool, // true → no separator before this item
}

fn flatten<'a>(
    nodes: &'a [Node],
    opened: &HashSet<Vec<usize>>,
    path: &[usize],
    depth: usize,
    out: &mut Vec<Flat<'a>>,
) {
    for (i, node) in nodes.iter().enumerate() {
        let mut p = path.to_vec();
        p.push(i);
        out.push(Flat { node, path: p.clone(), depth, first_sibling: i == 0 });
        if !node.children.is_empty() && opened.contains(&p) {
            flatten(&node.children, opened, &p, depth + 1, out);
        }
    }
}

// ── Line wrapping ─────────────────────────────────────────────────────────────

fn wrap_line(line: Line<'static>, width: usize) -> Vec<Line<'static>> {
    if width == 0 { return vec![line]; }
    let mut result: Vec<Line<'static>> = Vec::new();
    let mut current: Vec<Span<'static>> = Vec::new();
    let mut col = 0usize;

    for span in line.spans {
        let style = span.style;
        let mut remaining = span.content.into_owned();
        while !remaining.is_empty() {
            let space = width.saturating_sub(col);
            if space == 0 {
                result.push(Line::from(std::mem::take(&mut current)));
                col = 0;
                continue;
            }
            let take_n = remaining.chars().count().min(space);
            let byte_end = remaining.char_indices()
                .nth(take_n).map(|(i, _)| i).unwrap_or(remaining.len());
            let chunk = remaining[..byte_end].to_string();
            remaining = remaining[byte_end..].to_string();
            col += chunk.chars().count();
            current.push(Span::styled(chunk, style));
            if col >= width {
                result.push(Line::from(std::mem::take(&mut current)));
                col = 0;
            }
        }
    }
    if !current.is_empty() || result.is_empty() {
        result.push(Line::from(current));
    }
    result
}

// ── Scrollbar ─────────────────────────────────────────────────────────────────

fn draw_vscrollbar(buf: &mut Buffer, x: u16, y: u16, height: u16, total: usize, offset: usize) {
    if height == 0 { return; }
    let track = Style::default().fg(Color::DarkGray);
    let thumb = Style::default().fg(Color::Gray);
    if total <= height as usize {
        for row in 0..height {
            if let Some(c) = buf.cell_mut((x, y + row)) { c.set_char('│'); c.set_style(track); }
        }
        return;
    }
    let th = ((height as f64 * height as f64) / total as f64).ceil() as u16;
    let th = th.max(1).min(height);
    let range = total - height as usize;
    let tt = ((offset as f64 / range as f64) * (height - th) as f64).round() as u16;
    for row in 0..height {
        let (ch, st) = if row >= tt && row < tt + th { ('█', thumb) } else { ('│', track) };
        if let Some(c) = buf.cell_mut((x, y + row)) { c.set_char(ch); c.set_style(st); }
    }
}

// ── TreeWidget ────────────────────────────────────────────────────────────────

pub struct TreeWidget {
    nodes: Vec<Node>,
    opened: HashSet<Vec<usize>>,
    selected: Vec<usize>,
    vscroll: usize,
}

impl TreeWidget {
    pub fn new() -> Self {
        Self { nodes: Vec::new(), opened: HashSet::new(), selected: Vec::new(), vscroll: 0 }
    }

    pub fn set_nodes(&mut self, nodes: Vec<Node>) {
        self.nodes = nodes;
        self.opened.clear();
        self.selected.clear();
        self.vscroll = 0;
    }

    fn flat(&self) -> Vec<Flat<'_>> {
        let mut out = Vec::new();
        flatten(&self.nodes, &self.opened, &[], 0, &mut out);
        out
    }

    fn get_node(&self, path: &[usize]) -> Option<&Node> {
        let mut nodes = &self.nodes[..];
        let mut node = None;
        for &idx in path {
            node = nodes.get(idx);
            nodes = node.map(|n| n.children.as_slice()).unwrap_or(&[]);
        }
        node
    }

    pub fn select_down(&mut self) {
        let flat = self.flat();
        if flat.is_empty() { return; }
        if self.selected.is_empty() {
            self.selected = flat[0].path.clone(); return;
        }
        if let Some(i) = flat.iter().position(|f| f.path == self.selected) {
            if i + 1 < flat.len() { self.selected = flat[i + 1].path.clone(); }
        }
    }

    pub fn select_up(&mut self) {
        let flat = self.flat();
        if flat.is_empty() { return; }
        if self.selected.is_empty() {
            self.selected = flat.last().unwrap().path.clone(); return;
        }
        if let Some(i) = flat.iter().position(|f| f.path == self.selected) {
            if i > 0 { self.selected = flat[i - 1].path.clone(); }
        }
    }

    /// Collapse if open, else move to parent.
    pub fn select_left(&mut self) {
        if self.selected.is_empty() { return; }
        if !self.opened.remove(&self.selected) && self.selected.len() > 1 {
            self.selected.pop();
        }
    }

    /// Expand if has children.
    pub fn select_right(&mut self) {
        if self.selected.is_empty() { return; }
        if self.get_node(&self.selected).map(|n| !n.children.is_empty()).unwrap_or(false) {
            self.opened.insert(self.selected.clone());
        }
    }

    pub fn scroll_up(&mut self)   { self.vscroll = self.vscroll.saturating_sub(1); }
    pub fn scroll_down(&mut self) { self.vscroll += 1; }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer, focused: bool) {
        if area.width < 2 || area.height == 0 { return; }

        let content_w = (area.width - 1) as usize; // rightmost col = scrollbar
        let content_h = area.height as usize;

        let flat = self.flat();

        let sep_style  = Style::default().fg(Color::DarkGray);
        let sel_style  = Style::default().bg(Color::DarkGray);
        let pfx_style  = if focused { Style::default().fg(Color::Yellow) }
                         else        { Style::default().fg(Color::DarkGray) };

        struct VisLine { line: Line<'static>, selected: bool }
        let mut vis: Vec<VisLine> = Vec::new();
        let mut sel_row: Option<usize> = None;

        for item in &flat {
            let indent_w = item.depth * 2;
            let indent   = "  ".repeat(item.depth);

            // ── separator: between siblings, and between parent and first child
            if !item.first_sibling || item.depth > 0 {
                let rule_w = content_w.saturating_sub(indent_w);
                let sep = Line::from(vec![
                    Span::raw(indent.clone()),
                    Span::styled("─".repeat(rule_w), sep_style),
                ]);
                vis.push(VisLine { line: sep, selected: false });
            }

            // ── item lines ────────────────────────────────────────────────
            let is_sel  = item.path == self.selected;
            let has_ch  = !item.node.children.is_empty();
            let is_open = self.opened.contains(&item.path);

            let sym      = if has_ch { if is_open { "▼ " } else { "▶ " } } else { "  " };
            let prefix   = format!("{}{}", indent, sym);
            let prefix_w = prefix.chars().count();
            let inner_w  = content_w.saturating_sub(prefix_w).max(1);

            let wrapped = wrap_line(item.node.line.clone(), inner_w);

            let first = vis.len();
            if is_sel { sel_row = Some(first); }

            for (i, wline) in wrapped.into_iter().enumerate() {
                let mut spans: Vec<Span<'static>> = Vec::new();
                if i == 0 {
                    spans.push(Span::styled(prefix.clone(), pfx_style));
                } else {
                    spans.push(Span::raw(" ".repeat(prefix_w)));
                }
                spans.extend(wline.spans);
                vis.push(VisLine { line: Line::from(spans), selected: is_sel });
            }
        }

        let total = vis.len();

        // Auto-scroll to keep selection visible
        if let Some(row) = sel_row {
            if row < self.vscroll {
                self.vscroll = row;
            } else if row >= self.vscroll + content_h {
                self.vscroll = row + 1 - content_h;
            }
        }
        self.vscroll = self.vscroll.min(total.saturating_sub(content_h));

        let lines: Vec<Line<'static>> = vis.into_iter()
            .skip(self.vscroll)
            .take(content_h)
            .map(|vl| if vl.selected { vl.line.style(sel_style) } else { vl.line })
            .collect();

        Paragraph::new(lines).render(
            Rect { x: area.x, y: area.y, width: content_w as u16, height: content_h as u16 },
            buf,
        );

        draw_vscrollbar(buf, area.x + content_w as u16, area.y,
                        content_h as u16, total.max(content_h), self.vscroll);
    }
}
