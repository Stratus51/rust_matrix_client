use itertools::Itertools;
use tui::style::{Color, Modifier, Style};
use unicode_width::UnicodeWidthChar;
use unicode_width::UnicodeWidthStr;

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

fn char_width(c: char) -> usize {
    UnicodeWidthChar::width_cjk(c).unwrap_or(0)
}

fn string_width(s: &str) -> usize {
    UnicodeWidthStr::width_cjk(s)
}

pub struct BlockPosition {
    line: usize,
    char: usize,
}

#[derive(Debug, Clone)]
pub struct Line {
    line: String,
}

impl Line {
    fn new() -> Self {
        Self {
            line: String::new(),
        }
    }
    fn len(&self) -> usize {
        self.len()
    }
    fn push(&mut self, c: char) {
        self.line.push(c)
    }
    fn chars(&self) -> std::str::Chars {
        self.line.chars()
    }
    fn as_str<'a>(&'a self) -> &'a str {
        self.line.as_str()
    }

    fn width(&self) -> usize {
        self.line.chars().fold(0, |acc, c| acc + char_width(c))
    }

    fn str_to_block(line: &str, width: u16) -> Vec<String> {
        let mut chunks = vec![];
        let mut chunk = vec![];
        let mut chunk_w = 0;

        for c in line.chars() {
            let c_w = char_width(c);
            chunk_w += c_w;

            // If we overflow the line, add the character to the next line
            if chunk_w > width as usize {
                chunks.push(chunk.into_iter().collect::<String>());
                chunk = vec![c];
                chunk_w = c_w;
            // Else, add it to the current line
            } else {
                chunk.push(c);
                chunk_w += c_w;
            }
        }
        if chunk.len() > 0 {
            chunks.push(chunk.into_iter().collect::<String>());
        }
        chunks
    }

    fn str_to_cursor_block(
        line: &str,
        width: u16,
        pos: usize,
    ) -> (Vec<String>, Option<BlockPosition>) {
        let mut chunks = vec![];
        let mut chunk = vec![];
        let mut chunk_w = 0;
        let mut block_pos = None;

        for (i, c) in line.chars().enumerate() {
            let c_w = char_width(c);
            chunk_w += c_w;

            // If we overflow the line, add the character to the next line
            if chunk_w > width as usize {
                chunks.push(chunk.into_iter().collect::<String>());
                chunk = vec![c];
                chunk_w = c_w;
            // Else, add it to the current line
            } else {
                chunk.push(c);
                chunk_w += c_w;
            }

            // If this is the position, save the block position
            if i == pos {
                block_pos = Some(BlockPosition {
                    line: chunks.len(),
                    char: chunk_w - c_w,
                });
            }
        }
        if chunk.len() > 0 {
            chunks.push(chunk.into_iter().collect::<String>());
        }
        (chunks, block_pos)
    }

    fn to_block(&self, width: u16) -> Vec<String> {
        Self::str_to_block(&self.line, width)
    }

    fn to_cursor_block(&self, width: u16, pos: usize) -> (Vec<String>, Option<BlockPosition>) {
        Self::str_to_cursor_block(&self.line, width, pos)
    }
}

impl std::convert::From<String> for Line {
    fn from(line: String) -> Self {
        Self { line }
    }
}

impl std::iter::FromIterator<char> for Line {
    fn from_iter<I: IntoIterator<Item = char>>(iter: I) -> Self {
        Self {
            line: String::from_iter(iter),
        }
    }
}

#[derive(Debug)]
pub struct TextCursor {
    pub line: usize,
    pub c: usize,
}

#[derive(Clone, Debug)]
pub struct StringBlockItem {
    pub x: u16,
    pub y: u16,
    pub s: String,
    pub style: Style,
}

pub struct EditableText {
    pub lines: Vec<Line>,
    pub lines_widths: Vec<usize>,
    pub cursor: TextCursor,
    pub allow_cursor_over_limit: bool,
}

impl EditableText {
    pub fn new() -> Self {
        Self {
            lines: vec![Line::new()],
            lines_widths: vec![0],
            cursor: TextCursor { line: 0, c: 0 },
            allow_cursor_over_limit: false,
        }
    }

    pub fn reset(&mut self) {
        self.lines = vec![Line::new()];
        self.lines_widths = vec![0];
        self.cursor = TextCursor { line: 0, c: 0 };
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
            .fold(0, |tot, n| tot + n)
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
        let TextCursor { line: l_i, c: c_i } = &self.cursor;
        let line = &mut self.lines[*l_i];
        if line.len() == *c_i {
            self.lines.insert(l_i + 1, Line::new());
            self.lines_widths.insert(l_i + 1, 0);
            self.cursor.line += 1;
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
            self.cursor.c = 0;
        }
    }

    pub fn insert(&mut self, c: char) {
        if c == '\n' {
            self.line_feed();
        } else {
            let TextCursor { line: l_i, c: c_i } = &self.cursor;
            let line = &mut self.lines[*l_i];
            if line.len() == *c_i {
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
            self.cursor.c += 1;
            self.lines_widths[l_i] += char_width(c);
        }
    }

    pub fn replace(&mut self, c: char) {
        if c == '\n' {
            self.line_feed();
        } else {
            let TextCursor { line: l_i, c: c_i } = &self.cursor;
            let line = &mut self.lines[*l_i];
            if line.len() == *c_i {
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
            self.cursor.c += 1;
        }
    }

    pub fn backspace(&mut self) {
        let TextCursor { line: l_i, c: c_i } = &self.cursor;
        if *c_i == 0 {
            if *l_i > 0 {
                let l_i = *l_i;
                let line = self.remove_line(l_i);
                let prev_l_i = l_i - 1;
                self.cursor.line -= 1;
                self.cursor.c = self.lines[prev_l_i].chars().count();
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
            self.cursor.c -= 1;
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
        if self.cursor.c > line_lim {
            self.cursor.c = line_lim;
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
        if self.cursor.c < line_lim {
            self.cursor.c += 1;
        }
    }

    pub fn left(&mut self) {
        if self.cursor.c > 0 {
            self.cursor.c -= 1;
        }
    }
}

impl EditableText {
    pub fn cursor_graphic_line(&self, width: u16) -> usize {
        let prev_lines = self.lines_widths[..self.cursor.line]
            .iter()
            .map(|l_w| (l_w + width as usize - 1) / width as usize)
            .fold(0, |tot, n| tot + n);

        let current_line_width = self.lines[self.cursor.line]
            .chars()
            .take(self.cursor.c)
            .fold(0, |acc, c| acc + char_width(c));
        let current_line_height = (current_line_width + width as usize - 1) / width as usize;
        prev_lines + current_line_height
    }

    // TODO Anchoring view on line i is width dependent and therefore not resize pertinent
    // (but re-resize consistent)
    pub fn to_block(
        &self,
        area: tui::layout::Rect,
        line_i: u16,
        show_cursor: bool,
    ) -> Vec<StringBlockItem> {
        if self.is_empty() {
            return vec![];
        }
        todo!();
    }

    pub fn sized_string_block(&self, area: tui::layout::Rect) -> Vec<StringBlockItem> {
        if self.is_empty() {
            return vec![];
        }

        let mut ret = vec![];
        let w = area.width as usize;
        let mut y = 0;
        eprintln!("Text: {:?}; Cursor: {:?}", self.lines, self.cursor);
        for (text_line_i, line) in self.lines.iter().enumerate() {
            let (mut chunks, tmp, _) =
                line.chars()
                    .fold((vec![], vec![], 0), |(mut acc, mut tmp, mut tmp_w), c| {
                        let c_w = char_width(c);
                        tmp_w += c_w;
                        if tmp_w > w {
                            acc.push(tmp.into_iter().collect::<String>());
                            tmp = vec![];
                            tmp_w = 0;
                        } else {
                            tmp.push(c);
                            tmp_w += c_w;
                        }
                        (acc, tmp, tmp_w)
                    });
            chunks.push(tmp.into_iter().collect::<String>());
            eprintln!("Line {}: Chunks: {:?}", text_line_i, chunks);

            if text_line_i == self.cursor.line {
                eprintln!("THE LINE");
                let mut pre = vec![];
                let mut cursor = StringBlockItem {
                    x: 0,
                    y,
                    s: String::from(" "),
                    style: CURSOR_STYLE,
                };
                let mut post = vec![];
                let mut c_i = 0;
                for graphic_line in chunks.iter() {
                    let line_w = string_width(graphic_line);
                    if c_i + line_w < self.cursor.c {
                        pre.push(StringBlockItem {
                            x: 0,
                            y,
                            s: graphic_line.clone(),
                            style: SIMPLE_STYLE,
                        });
                    } else if c_i > self.cursor.c {
                        post.push(StringBlockItem {
                            x: 0,
                            y,
                            s: graphic_line.clone(),
                            style: SIMPLE_STYLE,
                        });
                    } else {
                        let mut pre_str = vec![];
                        let mut post_str = vec![];
                        let cursor_i = self.cursor.c - c_i;
                        for (i, c) in graphic_line.chars().enumerate() {
                            match i.cmp(&cursor_i) {
                                std::cmp::Ordering::Less => pre_str.push(c),
                                std::cmp::Ordering::Greater => post_str.push(c),
                                std::cmp::Ordering::Equal => cursor.s = c.to_string(),
                            }
                        }

                        let pre_str: String = pre_str.iter().collect();
                        let post_str: String = post_str.iter().collect();
                        let pre_str_width = string_width(pre_str.as_str());
                        eprintln!("pre str '{}' width: {}", pre_str, pre_str_width);

                        pre.push(StringBlockItem {
                            x: 0,
                            y,
                            s: pre_str,
                            style: SIMPLE_STYLE,
                        });
                        cursor.x = pre_str_width as u16;
                        post.push(StringBlockItem {
                            x: pre_str_width as u16 + string_width(&cursor.s) as u16,
                            y,
                            s: post_str,
                            style: SIMPLE_STYLE,
                        });
                        eprintln!(
                            "pre {}, cursor.x {}, post {}",
                            pre.len(),
                            cursor.x,
                            post.len()
                        );
                    }
                    c_i += line_w;
                    y += 1;
                    if y >= area.height {
                        break;
                    }
                }

                ret = [&ret[..], &pre[..], &[cursor], &post[..]].concat();
            } else {
                for graphic_line in chunks.iter() {
                    ret.push(StringBlockItem {
                        x: 0,
                        y,
                        s: graphic_line.clone(),
                        style: SIMPLE_STYLE,
                    });
                    y += 1;
                    if y >= area.height {
                        break;
                    }
                }
            }
            if y >= area.height {
                break;
            }
        }
        eprintln!("string blocks: {:?}", ret);
        ret
    }

    pub fn one_line_string(&self, from: usize, width: u16) -> Vec<StringBlockItem> {
        let line = &self.lines[0];
        let mut pre = vec![];
        let mut cursor = StringBlockItem {
            x: 0,
            y: 0,
            s: String::from(" "),
            style: CURSOR_STYLE,
        };
        let mut post = vec![];
        let mut w = 0;
        for (i, c) in line.chars().collect::<Vec<_>>().into_iter().enumerate() {
            match i.cmp(&self.cursor.c) {
                std::cmp::Ordering::Less => pre.push(c),
                std::cmp::Ordering::Greater => post.push(c),
                std::cmp::Ordering::Equal => cursor.s = c.to_string(),
            }
            w += char_width(c);
            if w >= width as usize {
                break;
            }
        }
        let pre_str: String = pre.iter().collect();
        let pre_str_width = string_width(&pre_str) as u16;
        let post_str: String = post.iter().collect();
        let pre = StringBlockItem {
            x: 0,
            y: 0,
            s: pre_str,
            style: SIMPLE_STYLE,
        };
        cursor.x = pre_str_width as u16;
        let post = StringBlockItem {
            x: pre_str_width + string_width(&cursor.s) as u16,
            y: 0,
            s: post_str,
            style: SIMPLE_STYLE,
        };
        vec![pre, cursor, post]
    }
}
