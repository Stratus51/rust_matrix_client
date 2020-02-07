use crate::text::editable_text::{EditableText, StringBlockItem};

#[derive(Debug)]
pub struct ViewPosition {
    line: usize,
    gline: usize,
    char: usize,
}

#[derive(Debug)]
pub struct Text {
    pub text: EditableText,
    pub view_pos: ViewPosition,
    pub show_cursor: bool,
    pub one_line: bool,
}

impl Text {
    pub fn new(text: &str) -> Self {
        Self {
            text: EditableText::new(text),
            view_pos: ViewPosition {
                line: 0,
                gline: 0,
                char: 0,
            },
            show_cursor: false,
            one_line: false,
        }
    }

    pub fn height(&self, width: u16) -> usize {
        self.text.height(width)
    }

    fn fix_view_pos(&mut self, area: tui::layout::Rect, one_line: bool) {
        if one_line {
            if self.view_pos.char > self.text.cursor.char {
                self.view_pos.char = self.text.cursor.char;
            } else if self.view_pos.char + (area.width as usize) < self.text.cursor.char {
                self.view_pos.char = self.text.cursor.char - area.width as usize;
            }
        } else {
            let cursor_gline = self.text.cursor_graphic_line(area.width);

            if self.view_pos.gline > cursor_gline {
                self.view_pos.gline = cursor_gline;
            } else if self.view_pos.gline + (area.height as usize) < cursor_gline {
                self.view_pos.gline = cursor_gline - area.height as usize;
            }
        }
    }
}

impl tui::widgets::Widget for Text {
    fn draw(&mut self, area: tui::layout::Rect, buf: &mut tui::buffer::Buffer) {
        // Fix text_cursor to follow terminal size changes and cursor movements
        eprintln!("self.fix_view_pos");
        self.fix_view_pos(area, self.one_line);

        // Draw
        let lines = if self.one_line {
            eprintln!("self.text.to_line_block");
            self.text.to_line_block(
                area,
                self.text.cursor.line,
                self.view_pos.char,
                area.width,
                self.show_cursor,
            )
        } else {
            eprintln!("self.text.to_block");
            self.text.to_block(
                area,
                self.view_pos.line,
                self.view_pos.gline,
                self.show_cursor,
            )
        };
        eprintln!("area: {:?}, Buf: {:?}", area, buf.area());
        for StringBlockItem {
            x,
            y,
            s: line,
            style,
        } in lines.into_iter()
        {
            eprintln!("line: [{};{}]: {}", x, y, line);
            eprintln!(
                "buf.set_stringn {} {} {} {} {:?}",
                area.x + x,
                area.y + y,
                line,
                area.width as usize,
                style
            );
            buf.set_stringn(x, y, line, area.width as usize, style);
        }
        eprintln!("done");
    }
}
