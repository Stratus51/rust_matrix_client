use unicode_width::UnicodeWidthChar;
use unicode_width::UnicodeWidthStr;

pub fn char_width(c: char) -> usize {
    UnicodeWidthChar::width(c).unwrap_or(0)
}

pub fn string_width(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}

#[derive(Debug, Clone)]
pub struct CharPosition {
    pub c: char,
    pub line: usize,
    pub char: usize,
}

#[derive(Debug, Clone)]
pub struct Line {
    pub line: String,
}

impl Line {
    pub fn new(s: &str) -> Self {
        Self {
            line: s.to_string(),
        }
    }
    pub fn len(&self) -> usize {
        self.line.len()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn push(&mut self, c: char) {
        self.line.push(c)
    }
    pub fn chars(&self) -> std::str::Chars {
        self.line.chars()
    }
    pub fn as_str(&self) -> &str {
        self.line.as_str()
    }

    pub fn width(&self) -> usize {
        self.line.chars().fold(0, |acc, c| acc + char_width(c))
    }

    pub fn str_to_block(line: &str, width: u16) -> Vec<String> {
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
            }
        }
        if !chunk.is_empty() || chunks.is_empty() {
            chunks.push(chunk.into_iter().collect::<String>());
        }
        chunks
    }

    pub fn str_to_cursor_block(
        line: &str,
        width: u16,
        pos: usize,
    ) -> (Vec<String>, Option<CharPosition>) {
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
            }

            // If this is the position, save the block position
            if i == pos {
                block_pos = Some(CharPosition {
                    c,
                    line: chunks.len(),
                    char: chunk_w - c_w,
                });
            }
        }

        if pos >= line.chars().count() {
            if chunk_w >= width as usize {
                chunks.push(String::new());
                chunk_w = 0;
            }
            block_pos = Some(CharPosition {
                c: ' ',
                line: chunks.len(),
                char: chunk_w,
            });
        }

        if !chunk.is_empty() || chunks.is_empty() {
            chunks.push(chunk.into_iter().collect::<String>());
        }
        (chunks, block_pos)
    }

    pub fn to_block(&self, width: u16) -> Vec<String> {
        Self::str_to_block(&self.line, width)
    }

    pub fn to_cursor_block(&self, width: u16, pos: usize) -> (Vec<String>, Option<CharPosition>) {
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
