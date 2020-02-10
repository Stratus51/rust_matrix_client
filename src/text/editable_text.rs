use itertools::Itertools;
use tui::style::{Color, Modifier, Style};

use super::line::{char_width, Line};

const SIMPLE_STYLE: Style = Style {
    fg: Color::Reset,
    bg: Color::Reset,
    modifier: Modifier::empty(),
};

const CURSOR_STYLE: Style = Style {
    fg: Color::Black,
    bg: Color::White,
    modifier: Modifier::empty(),
};

#[derive(Debug)]
pub struct TextCursor {
    pub line: usize,
    pub char: usize,
}

#[derive(Clone, Debug)]
pub struct StringBlockItem {
    pub x: u16,
    pub y: u16,
    pub s: String,
    pub style: Style,
}

#[derive(Debug)]
pub struct EditableText {
    pub lines: Vec<Line>,
    pub lines_widths: Vec<usize>,
    pub cursor: TextCursor,
    pub allow_cursor_over_limit: bool,
}

impl EditableText {
    pub fn new(text: &str) -> Self {
        let mut ret = Self {
            lines: vec![],
            lines_widths: vec![],
            cursor: TextCursor { line: 0, char: 0 },
            allow_cursor_over_limit: false,
        };
        ret.set_text(text);
        ret
    }

    pub fn set_text(&mut self, text: &str) {
        let lines: Vec<_> = text.split('\n').map(|s| Line::new(s)).collect();
        let nb_lines = lines.len();
        self.lines = lines;
        self.lines_widths = vec![0; nb_lines];
        for i in 0..nb_lines {
            self.update_line_width(i);
        }
    }

    pub fn reset(&mut self) {
        self.lines = vec![Line::new(&"")];
        self.lines_widths = vec![0];
        self.cursor = TextCursor { line: 0, char: 0 };
    }

    pub fn consume(&mut self) -> String {
        let ret = self
            .lines
            .clone()
            .into_iter()
            .map(|l| l.line)
            .intersperse("\n".to_string())
            .collect::<Vec<_>>()
            .concat();
        self.reset();
        ret
    }

    pub fn height(&self, width: u16) -> usize {
        self.lines_widths
            .iter()
            .map(|l_w| (l_w + width as usize - 1) / width as usize)
            .map(|h| usize::max(h, 1))
            .sum()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.len() == 1 && self.lines[0].len() == 0
    }

    pub fn update_line_width(&mut self, i: usize) {
        self.lines_widths[i] = self.lines[i].width();
    }

    pub fn remove_line(&mut self, i: usize) -> String {
        self.lines_widths.remove(i);
        self.lines.remove(i).line
    }

    pub fn line_feed(&mut self) {
        let TextCursor {
            line: l_i,
            char: c_i,
        } = &self.cursor;
        let line = &mut self.lines[*l_i];
        if line.chars().count() == *c_i {
            self.lines.insert(l_i + 1, Line::new(&""));
            self.lines_widths.insert(l_i + 1, 0);
            self.cursor.line += 1;
            self.cursor.char = 0;
        } else {
            let mut old_line = vec![];
            let mut new_line = vec![];
            for (i, lc) in line.chars().enumerate() {
                if i < *c_i {
                    old_line.push(lc);
                } else {
                    new_line.push(lc);
                }
            }
            let l_i = *l_i;
            let next_l_i = l_i + 1;

            self.lines[l_i] = old_line.into_iter().collect();
            self.update_line_width(l_i);

            self.lines.insert(next_l_i, new_line.into_iter().collect());
            self.lines_widths.insert(next_l_i, 0);
            self.update_line_width(next_l_i);

            self.cursor.line += 1;
            self.cursor.char = 0;
        }
    }

    pub fn insert(&mut self, c: char) {
        if c == '\n' {
            self.line_feed();
        } else {
            let TextCursor {
                line: l_i,
                char: c_i,
            } = &self.cursor;
            let line = &mut self.lines[*l_i];
            if line.chars().count() == *c_i {
                line.push(c);
            } else {
                let mut new_line = vec![];
                for (i, lc) in line.chars().enumerate() {
                    if i == *c_i {
                        new_line.push(c);
                    }
                    new_line.push(lc);
                }
                self.lines[*l_i] = new_line.into_iter().collect();
            }
            let l_i = *l_i;
            self.cursor.char += 1;
            self.lines_widths[l_i] += char_width(c);
        }
    }

    pub fn replace(&mut self, c: char) {
        if c == '\n' {
            self.line_feed();
        } else {
            let TextCursor {
                line: l_i,
                char: c_i,
            } = &self.cursor;
            let line = &mut self.lines[*l_i];
            if line.chars().count() == *c_i {
                line.push(c);
            } else {
                let mut new_line = vec![];
                for (i, lc) in line.chars().enumerate() {
                    if i == *c_i {
                        new_line.push(c);
                        let old_w = char_width(lc);
                        let new_w = char_width(c);
                        self.lines_widths[i] += new_w - old_w;
                    } else {
                        new_line.push(lc);
                    }
                }
                self.lines[*l_i] = new_line.into_iter().collect();
            }
            self.cursor.char += 1;
        }
    }

    pub fn backspace(&mut self) {
        let TextCursor {
            line: l_i,
            char: c_i,
        } = &self.cursor;
        if *c_i == 0 {
            if *l_i > 0 {
                let l_i = *l_i;
                let line = self.remove_line(l_i);
                let prev_l_i = l_i - 1;
                self.cursor.line -= 1;
                self.cursor.char = self.lines[prev_l_i].chars().count();
                self.lines[prev_l_i] = Line::from([self.lines[prev_l_i].as_str(), &line].concat());
            }
        } else {
            let line = &mut self.lines[*l_i];
            let mut new_line = vec![];
            for (i, lc) in line.chars().enumerate() {
                if i != *c_i - 1 {
                    new_line.push(lc);
                } else {
                    self.lines_widths[*l_i] -= char_width(lc);
                }
            }
            self.lines[*l_i] = new_line.into_iter().collect();
            self.cursor.char -= 1;
        }
    }

    pub fn delete(&mut self) {}

    fn line_limit(&self) -> usize {
        let mut line_lim = self.lines[self.cursor.line].chars().count();
        if !self.allow_cursor_over_limit && line_lim > 0 {
            line_lim -= 1;
        }
        line_lim
    }
    fn fix_cursor(&mut self) {
        let line_lim = self.line_limit();
        if self.cursor.char > line_lim {
            self.cursor.char = line_lim;
        }
    }

    pub fn up(&mut self) {
        if self.cursor.line != 0 {
            self.cursor.line -= 1;
            self.fix_cursor();
        }
    }

    pub fn down(&mut self) {
        if self.cursor.line < self.lines.len() - 1 {
            self.cursor.line += 1;
            self.fix_cursor();
        }
    }

    pub fn right(&mut self) {
        let line_lim = self.line_limit();
        if self.cursor.char < line_lim {
            self.cursor.char += 1;
        }
    }

    pub fn left(&mut self) {
        if self.cursor.char > 0 {
            self.cursor.char -= 1;
        }
    }

    pub fn home(&mut self) {
        self.cursor.char = 0;
    }

    pub fn end(&mut self) {
        self.cursor.char = self.line_limit();
    }
}

impl EditableText {
    pub fn cursor_graphic_line(&self, width: u16) -> usize {
        let current_line_width = self.lines[self.cursor.line]
            .chars()
            .take(self.cursor.char)
            .fold(0, |acc, c| acc + char_width(c));
        (current_line_width + width as usize - 1) / width as usize
    }

    fn lines_from_area<'a>(&'a self, area: tui::layout::Rect, line_i: usize) -> Vec<&'a Line> {
        let mut height = 0;
        let mut ret = vec![];
        for (w, line) in self.lines_widths.iter().zip(self.lines.iter()).skip(line_i) {
            ret.push(line);
            height += w / area.width as usize;
            if height >= area.height as usize {
                break;
            }
        }
        ret
    }

    // TODO Anchoring view on gline i is width dependent and therefore not resize pertinent
    // (but re-resize consistent)
    pub fn to_block(
        &self,
        area: tui::layout::Rect,
        line_i: usize,
        gline_i: usize,
        show_cursor: bool,
    ) -> Vec<StringBlockItem> {
        if self.is_empty() {
            if show_cursor {
                return vec![StringBlockItem {
                    x: area.x,
                    y: area.y,
                    s: " ".to_string(),
                    style: CURSOR_STYLE,
                }];
            } else {
                return vec![];
            }
        }

        let mut ret = vec![];
        let mut height = 0;
        let mut cursor_block = None;
        for (i, line) in self.lines_from_area(area, line_i).iter().enumerate() {
            // Build graphic line blocks
            let (list, cursor) = if show_cursor && i == self.cursor.line {
                line.to_cursor_block(area.width, self.cursor.char)
            } else {
                (line.to_block(area.width), None)
            };

            // Add cursor if present
            if let Some(cursor) = cursor {
                if cursor.line < area.height as usize {
                    cursor_block = Some(StringBlockItem {
                        x: area.x + cursor.char as u16,
                        y: area.y + height + cursor.line as u16,
                        s: cursor.c.to_string(),
                        style: CURSOR_STYLE,
                    });
                }
            }

            // Build StringBlockItems
            if list.is_empty() {
                height += 1;
            } else {
                let skip = if i == 0 { gline_i } else { 0 };
                for gline in list.into_iter().skip(skip) {
                    ret.push(StringBlockItem {
                        x: area.x,
                        y: area.y + height as u16,
                        s: gline,
                        style: SIMPLE_STYLE,
                    });
                    height += 1;
                    if height >= area.height {
                        break;
                    }
                }
            }

            // Early exit
            if height >= area.height {
                break;
            }
        }
        if let Some(block) = cursor_block {
            ret.push(block);
        }
        ret
    }

    pub fn to_line_block(
        &self,
        area: tui::layout::Rect,
        line_i: usize,
        pos: usize,
        width: u16,
        show_cursor: bool,
    ) -> Vec<StringBlockItem> {
        let mut ret = vec![];
        let mut line_string = vec![];
        let mut line_width = 0;
        let mut cursor_block = None;
        for (i, c) in self.lines[line_i]
            .chars()
            .skip(pos)
            .take(area.width as usize)
            .enumerate()
        {
            let c_w = char_width(c);
            if line_width + c_w > width as usize {
                break;
            }

            line_string.push(c);
            if show_cursor && i == self.cursor.char {
                cursor_block = Some(StringBlockItem {
                    x: area.x + line_width as u16,
                    y: area.y,
                    s: c.to_string(),
                    style: CURSOR_STYLE,
                });
            }
            line_width += c_w;
        }
        ret.insert(
            0,
            StringBlockItem {
                x: area.x,
                y: area.y,
                s: line_string.iter().collect(),
                style: SIMPLE_STYLE,
            },
        );
        if self.cursor.char == self.lines[line_i].chars().count() && line_width < width as usize {
            cursor_block = Some(StringBlockItem {
                x: area.x + line_width as u16,
                y: area.y,
                s: " ".to_string(),
                style: CURSOR_STYLE,
            });
        }
        if let Some(block) = cursor_block {
            ret.push(block);
        }
        ret
    }
}
