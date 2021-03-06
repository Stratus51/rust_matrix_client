use crate::text::editable_text::{EditableText, StringBlockItem};
use crate::widget::{
    scroll::{Element, PartialWidget},
    Height,
};

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

    pub fn set_text(&mut self, text: &str) {
        self.text.set_text(text);
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
        self.fix_view_pos(area, self.one_line);
        self.partial_draw(0, area, buf);
    }
}

impl Height for Text {
    fn height(&self, width: u16) -> usize {
        self.text.height(width)
    }
}

impl PartialWidget for Text {
    fn partial_draw(
        &mut self,
        y_offset: usize,
        mut area: tui::layout::Rect,
        buf: &mut tui::buffer::Buffer,
    ) {
        // Draw
        area.height += y_offset as u16;
        let lines = if self.one_line {
            self.text.to_line_block(
                area,
                self.text.cursor.line,
                self.view_pos.char,
                area.width,
                self.show_cursor,
            )
        } else {
            self.text.to_block(
                area,
                self.view_pos.line,
                self.view_pos.gline,
                self.show_cursor,
            )
        };
        for (
            y_i,
            StringBlockItem {
                x,
                y,
                s: line,
                style,
            },
        ) in lines.into_iter().enumerate()
        {
            if y_i >= y_offset {
                buf.set_stringn(x, y - y_offset as u16, line, area.width as usize, style);
            }
        }
    }
}

impl Element for Text {}
