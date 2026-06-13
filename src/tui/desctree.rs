use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Paragraph, Widget, Wrap},
};

use crate::{colorize::ColorGenerator, desc::DescNode};

pub struct TreeNode {
    pub text: Text<'static>,
    pub children: Vec<TreeNode>,
    pub opened: bool,
}

impl TreeNode {
    pub fn new(text: impl Into<Text<'static>>, children: Vec<TreeNode>) -> Self {
        Self {
            text: text.into(),
            children,
            opened: false,
        }
    }
}

struct Flat<'a> {
    node: &'a TreeNode,
    path: Vec<usize>,
    depth: usize,
    root: bool,
}

fn flatten_into<'a>(nodes: &'a [TreeNode], path: &[usize], depth: usize, out: &mut Vec<Flat<'a>>) {
    for (i, node) in nodes.iter().enumerate() {
        let mut p = path.to_vec();
        p.push(i);
        out.push(Flat {
            node,
            path: p.clone(),
            depth,
            root: i == 0,
        });
        if !node.children.is_empty() && node.opened {
            flatten_into(&node.children, &p, depth + 1, out);
        }
    }
}

fn flatten(nodes: &[TreeNode]) -> Vec<Flat<'_>> {
    let mut out = Vec::new();
    flatten_into(nodes, &[], 0, &mut out);
    out
}

fn compute_text_height(text: &Text<'static>, width: usize) -> usize {
    if width == 0 { return 0; }
    use textwrap::wrap;
    let mut total = 0usize;
    // `Text` is a collection of `Line`s, each `Line` is a collection of `Span`s.
    // Concatenate span contents per logical line, then wrap that line.
    for line in text.lines.iter() {
        let mut s = String::new();
        for span in line.spans.iter() {
            s.push_str(span.content.as_ref());
        }
        total += wrap(&s, width).len().max(1);
    }
    total
}

struct ItemLayout {
    sep: bool,
    node_h: usize,
    indent_width: usize,
    para_width: usize,
}

impl ItemLayout {
    fn layout_from_tree(tree: &DescTreeWidget, content_w: usize) -> (Vec<Self>, usize, Option<usize>) {
        let flat = flatten(&tree.nodes);

        let mut total_rows = 0usize;
        let mut sel_row = None;
        let item_layout: Vec<ItemLayout> = flat
            .iter()
            .map(|item| {
                let sep = !item.root || item.depth > 0;
                let indent_width = item.depth * 2 + 2; // + 2 because of the prefix symbol (▶/▼)
                let para_width = content_w.saturating_sub(indent_width).max(1);
                let node_h = compute_text_height(&item.node.text, para_width).max(1);
                if item.path == tree.selected {
                    sel_row = Some(total_rows + sep as usize);
                }
                total_rows += sep as usize + node_h;
                ItemLayout {
                    sep,
                    node_h,
                    indent_width,
                    para_width,
                }
            })
            .collect();
        (item_layout, total_rows, sel_row)
    }

    fn rects(&self, area: Rect, row: u16, visible: u16 ) -> (Rect, Rect) {
        let pfx_rect = Rect {
            x: area.x,
            y: row,
            width: self.indent_width as u16,
            height: visible,
        };
        let para_rect = Rect {
            x: area.x + self.indent_width as u16,
            y: row,
            width: self.para_width as u16,
            height: visible,
        };
        (pfx_rect, para_rect)
    }
}

pub struct DescTreeWidget {
    nodes: Vec<TreeNode>,
    selected: Vec<usize>,
    vscroll: usize,
    accent_color: Color,
}

impl DescTreeWidget {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            selected: Vec::new(),
            vscroll: 0,
            accent_color: Color::Yellow
        }
    }

    pub fn set_accent_color(&mut self, c: Color) {
        self.accent_color = c;
    }

    fn descnode_to_treenode(node: &DescNode, pattern: &str, cgen: &ColorGenerator) -> Vec<TreeNode> {
        let children: Vec<TreeNode> = node
            .nested_items
            .iter()
            .flat_map(|child| Self::descnode_to_treenode(child, pattern, cgen))
            .collect();

        if node.desc.is_empty() {
            // Transparent concat node — hoist children up
            return children;
        }

        let span = node.span;
        let mut spans = vec![Span::raw("`")];
        spans.extend(render_slice(pattern, span.start, span.end, cgen));
        spans.push(Span::raw(format!("` {}", node.desc)));

        vec![TreeNode::new(Line::from(spans), children)]

    }

    pub fn from_descnodes(desc_root: &DescNode, pattern: &str, cgen: &ColorGenerator) -> Self {
        let root_tree_node = Self::descnode_to_treenode(desc_root, pattern, cgen);
        Self {
           nodes: root_tree_node,
           ..Self::new()
        }
    }

    pub fn get_selected_span(&self, desc_root: &DescNode) -> crate::types::Span {
        // Treat nodes with an empty desc as transparent (hoisted children), mirroring desc_to_nodes.
        fn visible_children<'a>(node: &'a crate::desc::DescNode, out: &mut Vec<&'a crate::desc::DescNode>) {
            for child in &node.nested_items {
                if child.desc.is_empty() {
                    visible_children(child, out);
                } else {
                    out.push(child);
                }
            }
        }

        let mut cur = desc_root;
        for &idx in self.selected_path() {
            let mut vis = Vec::new();
            visible_children(cur, &mut vis);
            if idx >= vis.len() {
                break;
            }
            cur = vis[idx];
        }
        cur.span
    }

    pub fn set_nodes(&mut self, nodes: Vec<TreeNode>) {
        self.nodes = nodes;
        self.selected.clear();
        self.vscroll = 0;
    }

    /// Return the selected path (indices from root) inside the tree.
    pub fn selected_path(&self) -> &[usize] {
        &self.selected
    }

    /// Returns Selected, Total
    pub fn selected_index_total(&self) -> (Option<usize>, usize) {
        let flat = flatten(&self.nodes);
        let total = flat.len();
        if total == 0 { return (None, 0); }
        if self.selected.is_empty() { return (None, total); }
        let pos = flat.iter().position(|f| f.path == self.selected);
        match pos {
            Some(p) => (Some(p + 1), total),
            None => (None, total),
        }
    }

    fn get_node_mut(&mut self, path: &[usize]) -> Option<&mut TreeNode> {
        if path.is_empty() {
            return None;
        }
        let mut current = self.nodes.get_mut(path[0])?;
        for &idx in &path[1..] {
            current = current.children.get_mut(idx)?;
        }
        Some(current)
    }

    pub fn select_down(&mut self) {
        let flat = flatten(&self.nodes);
        if flat.is_empty() {
            return;
        }
        if self.selected.is_empty() {
            self.selected = flat[0].path.clone();
            return;
        }
        if let Some(i) = flat.iter().position(|f| f.path == self.selected)
            && i + 1 < flat.len()
        {
            self.selected = flat[i + 1].path.clone();
        }
    }

    pub fn select_up(&mut self) {
        let flat = flatten(&self.nodes);
        if flat.is_empty() {
            return;
        }
        if self.selected.is_empty() {
            self.selected = flat.last().unwrap().path.clone();
            return;
        }
        if let Some(i) = flat.iter().position(|f| f.path == self.selected)
            && i > 0
        {
            self.selected = flat[i - 1].path.clone();
        }
    }

    /// Collapse if open, else move to parent.
    pub fn select_left(&mut self) {
        if self.selected.is_empty() {
            return;
        }
        let path = self.selected.clone();
        if let Some(node) = self.get_node_mut(&path)
            && node.opened
        {
            node.opened = false;
            return;
        }
        if self.selected.len() > 1 {
            self.selected.pop();
        }
    }

    /// Expand if has children.
    pub fn select_right(&mut self) {
        let path = self.selected.clone();
        if let Some(node) = self.get_node_mut(&path)
            && !node.children.is_empty()
        {
            node.opened = true;
        }
    }

    pub fn scroll_up(&mut self) {
        self.vscroll = self.vscroll.saturating_sub(1);
    }
    pub fn scroll_down(&mut self) {
        self.vscroll += 1;
    }

    fn seperator_style() -> Style {
        Style::default().fg(Color::DarkGray)
    }

    fn selected_style() -> Style {
        Style::default().bg(Color::DarkGray)
    }

    fn render_seperator(buf: &mut Buffer, area: Rect, depth: usize, content_w: usize, row: u16) {
        let indent = "  ".repeat(depth);
        let rule_w = content_w.saturating_sub(indent.len());
        Paragraph::new(Line::from(vec![
            Span::raw(indent),
            Span::styled("─".repeat(rule_w), Self::seperator_style()),
        ]))
        .render(
            Rect {
                x: area.x,
                y: row,
                width: content_w as u16,
                height: 1,
            },
            buf,
        );
    }

    fn calculate_prefix_text(item: &Flat, height: usize, accent_color: Color) -> Text<'static> {
        let indent = "  ".repeat(item.depth);
        let prefix_sym = if item.node.children.is_empty() {
            "  "
        } else if item.node.opened {
            "▼ "
        } else {
            "▶ "
        };
        let prefix_str = format!("{}{}", indent, prefix_sym);

        let indent_w = indent.len() + 2;

        let blank = Span::raw(" ".repeat(indent_w));
        let mut prefix_lines = vec![Line::from(Span::styled(
            prefix_str,
            Style::default().fg(accent_color),
        ))];

        //prefix lien should be upto to the same height as node height, so we insert blank
        prefix_lines.extend((1..height).map(|_| Line::from(blank.clone())));
        Text::from(prefix_lines)
    }
}

impl Widget for &mut DescTreeWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 2 || area.height == 0 {
            return;
        }

        let content_w = area.width as usize;
        let content_h = area.height as usize;

        let (item_layout, total_rows, sel_row) = ItemLayout::layout_from_tree(self, content_w);

        // Auto-scroll: keep selected node visible.
        if let Some(row) = sel_row {
            if row < self.vscroll {
                self.vscroll = row;
            } else if row >= self.vscroll + content_h {
                self.vscroll = row + 1 - content_h;
            }
        }
        self.vscroll = self.vscroll.min(total_rows.saturating_sub(content_h));
        let mut viewport = Viewport::new(self.vscroll, area.y, area.y + content_h as u16);

        let flat = flatten(&self.nodes);
        for (item, lay) in flat.iter().zip(item_layout) {
            if viewport.is_full() {
                break
            }
            let item_height = lay.sep as usize + lay.node_h;
            if matches!(viewport.clip(item_height), Visibility::Hidden) {
                continue
            }

            // Separator
            if lay.sep && matches!(viewport.clip(1), Visibility::Visible{offset: _}) {
                DescTreeWidget::render_seperator(buf, area, item.depth, content_w, viewport.curr_row());
                viewport.advance(1);
                viewport.advance_offset(1);
                if viewport.is_full() {
                    break;
                }
            }
            // Node
            let Visibility::Visible { offset } = viewport.clip(lay.node_h) else {
                unreachable!("Hidden should never occur here");
            };
            let visible = viewport.visible_rows(lay.node_h, offset);
            viewport.advance_offset(offset);

            let (pfx_rect, para_rect) = lay.rects(area, viewport.curr_row(), visible as u16);

            let is_selected = item.path == self.selected;
            let style = if is_selected {
                DescTreeWidget::selected_style()
            } else {
                Style::default()
            };

            let prefix_text = DescTreeWidget::calculate_prefix_text(item, lay.node_h, self.accent_color);
            Paragraph::new(prefix_text)
                .scroll((offset as u16, 0))
                .style(style)
                .render(pfx_rect, buf);
            Paragraph::new(item.node.text.clone())
                .wrap(Wrap { trim: false })
                .scroll((offset as u16, 0))
                .style(style)
                .render(para_rect, buf);

            viewport.advance(visible as u16);
        }
    }
}

pub struct Viewport {
    /// Number of rows scrolled above the viewport.
    offset: usize,
    /// Y-coordinate of the next row to render.
    row: u16,
    /// Maximum Y-coordinate of the viewport (exclusive).
    bottom_row: u16,
}

#[derive(Debug, Clone, Copy)]
pub enum Visibility {
    /// The span is entirely above the viewport (not visible)
    Hidden,
    /// The span is at least partially visible.
    /// `offset` is the number of rows of the span that are hidden above the viewport.
    Visible { offset: usize },
}

impl Viewport {
    /// `offset` is the initial scroll (number of rows hidden above).
    /// `row` is the starting row to render (`area.y`).
    /// `bottom_row` is the bottom row of the viewport (`area.y + content_h`).
    pub fn new(offset: usize, row: u16, bottom_row: u16) -> Self {
        Self {
            offset,
            row,
            bottom_row,
        }
    }

    /// Does not reset the offset; you can explicitly call `reveal()` after consuming.
    pub fn clip(&mut self, span_h: usize) -> Visibility {
        if self.offset >= span_h {
            // The entire span is above the viewport; consume the offset
            self.offset -= span_h;
            Visibility::Hidden
        } else {
            // Part or all of the span is visible
            Visibility::Visible { offset: self.offset }
        }
    }

    /// Advance the offset (scroll) by `n` rows.
    /// Called after rendering part of the span.
    pub fn advance_offset(&mut self, n: usize) {
        self.offset = self.offset.saturating_sub(n);
    }

    pub fn curr_row(&self) -> u16 {
        self.row
    }

    /// Advance the cursor row after rendering `rows` rows.
    pub fn advance(&mut self, rows: u16) {
        self.row += rows;
    }

    /// Returns how many rows of `span_h` are actually visible in the viewport.
    /// `offset` is the number of rows hidden above the viewport within this span.
    pub fn visible_rows(&self, span_h: usize, offset: usize) -> usize {
        let available_rows = self.bottom_row.saturating_sub(self.row) as usize;
        (span_h - offset).min(available_rows)
    }

    pub fn is_full(&self) -> bool {
        self.row >= self.bottom_row
    }
}

fn render_slice(
    pattern: &str,
    start: usize,
    end: usize,
    cgen: &ColorGenerator,
) -> Vec<Span<'static>> {
    if start >= end {
        return Vec::new();
    }
    let mut spans: Vec<Span<'static>> = Vec::new();
    for idx in start..end {
        let (fg, bg) = cgen.char_color(idx);
        let mut style = Style::default();
        if let Some(f) = fg {
            style = style.fg(to_color(f));
        }
        if let Some(b) = bg {
            style = style.bg(to_color(b));
        }
        spans.push(Span::styled(pattern[idx..idx+1].to_string(), style));
    }
    spans
}

fn to_color(rgb: [f32; 3]) -> Color {
    let b = |v: f32| (v * 255.0).clamp(0.0, 255.0) as u8;
    Color::Rgb(b(rgb[0]), b(rgb[1]), b(rgb[2]))
}
