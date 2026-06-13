use tui_textarea::TextArea;

type CursorRange = ((usize, usize), (usize, usize));

pub trait TextAreaExt {
    fn get_cursor_pos(&self, offset: usize) -> (usize, usize);
    fn get_cursor_range_from_offsets(&self, start: usize, end: usize) -> CursorRange;
    fn get_flat_offset_from_cursor(&self) -> usize;
}

impl TextAreaExt for TextArea<'_> {
    fn get_cursor_pos(&self, offset: usize) -> (usize, usize) {
        let lines = self.lines();
        let mut x = offset;
        let mut row = 0usize;
        let mut col = 0;
        for line in lines {
            if x <= line.len() {
                col = x;
                break;
            } else {
                row += 1;
                x -= line.len() + 1; // +1 for the newline
            }
        }
        (row, col)
    }

    fn get_cursor_range_from_offsets(&self, start: usize, end: usize) -> CursorRange {
        (self.get_cursor_pos(start), self.get_cursor_pos(end))
    }

    fn get_flat_offset_from_cursor(&self) -> usize {
        let (row, col) = self.cursor();
        let lines = self.lines();
        let mut offset = 0;
        for (r, line) in lines.iter().enumerate() {
            if r == row {
                return offset + col.min(line.len());
            }
            offset += line.len() + 1; // +1 for newline
        }
        offset
    }
}
