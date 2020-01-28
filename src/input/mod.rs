use crate::event::{Action, Event, EventProcessor, Key};
use tui::style::{Color, Modifier, Style};
use unicode_width::UnicodeWidthChar;
use unicode_width::UnicodeWidthStr;

mod editable_text;
use editable_text::{EditableText, StringBlockItem};

mod command;
use command::Command;

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

pub enum EditMode {
    Insert,
    Replace,
}
impl EditMode {
    pub fn to_string(&self) -> String {
        match self {
            EditMode::Insert => "insert",
            EditMode::Replace => "replace",
        }
        .to_string()
    }
}

pub enum Mode {
    None,
    Edit(EditMode),
    Command,
}

impl Mode {
    pub fn to_string(&self) -> String {
        match self {
            Mode::None => String::new(),
            Mode::Edit(e) => format!("edit: {}", e.to_string()),
            Mode::Command => "command".to_string(),
        }
    }
}

pub struct Input {
    mode: Mode,
    text: EditableText,
    text_cursor: usize,
    command: Command,
}

impl Input {
    pub fn new() -> Self {
        Self {
            mode: Mode::None,
            text: EditableText::new(),
            text_cursor: 0,
            command: Command::new(),
        }
    }

    pub fn wanted_size(&self, width: u16) -> u16 {
        self.text.height(width)
    }

    fn min_text_cursor(&self, area: tui::layout::Rect) -> usize {
        let mut size = 0;
        for (i, line_w) in self.text.lines_widths[0..self.text.cursor.line]
            .iter()
            .enumerate()
            .rev()
        {
            let line_size = (line_w + area.width as usize - 1) / area.width as usize;
            if size + line_size >= area.height as usize {
                return i;
            }
            size += line_size;
        }
        0
    }

    fn max_text_cursor(&self) -> usize {
        self.text.cursor.line
    }

    fn fix_text_cursor(&mut self, area: tui::layout::Rect) {
        let min_text_cursor = self.min_text_cursor(area);
        if self.text_cursor < min_text_cursor {
            self.text_cursor = min_text_cursor;
        }
        let max_text_cursor = self.max_text_cursor();
        if self.text_cursor > max_text_cursor {
            self.text_cursor = max_text_cursor;
        }
    }
}

impl tui::widgets::Widget for Input {
    fn draw(&mut self, area: tui::layout::Rect, buf: &mut tui::buffer::Buffer) {
        eprintln!("Draw!");
        // Fix text_cursor to follow terminal size changes and cursor movements
        self.fix_text_cursor(area);

        // Draw
        let lines = self.text.sized_string_block(area);
        eprintln!("area: {:?}, Buf: {:?}", area, buf.area());
        for StringBlockItem {
            x,
            y,
            s: line,
            style,
        } in lines.iter()
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
            buf.set_stringn(area.x + x, area.y + y, line, area.width as usize, *style);
        }

        // If command mode, overwrite last line
        eprintln!("Draw command!");
        if let Mode::Command = &self.mode {
            let mut area = area.clone();
            area.y = area.y + area.height - 1;
            area.height = 1;
            self.command.draw(area, buf);
        }
        eprintln!("Done");
    }
}

impl Input {
    // Mode event processing implementation
    fn process_none_event(&mut self, event: &Event) -> Vec<Action> {
        match event {
            Event::Key(k) => match k {
                Key::Char(c) => {
                    match c {
                        'i' => self.mode = Mode::Edit(EditMode::Insert),
                        'a' => {
                            self.text.right();
                            self.mode = Mode::Edit(EditMode::Insert);
                        }
                        'r' => self.mode = Mode::Edit(EditMode::Replace),
                        ':' => self.mode = Mode::Command,
                        _ => (),
                    };
                    return vec![Action::StatusSet(self.mode.to_string())];
                }
                Key::Up => self.text.up(),
                Key::Down => self.text.down(),
                Key::Right => self.text.right(),
                Key::Left => self.text.left(),
                Key::Esc => return vec![Action::FocusLoss],
                _ => (),
            },
            _ => (),
        };
        vec![]
    }
    fn process_insert_event(&mut self, event: &Event) -> Vec<Action> {
        match event {
            Event::Key(k) => match k {
                Key::Char(c) => self.text.insert(*c),
                Key::Backspace => self.text.backspace(),
                Key::Up => self.text.up(),
                Key::Down => self.text.down(),
                Key::Right => self.text.right(),
                Key::Left => self.text.left(),
                Key::Esc => {
                    self.mode = Mode::None;
                    return vec![Action::StatusSet(self.mode.to_string())];
                }
                _ => (),
            },
            _ => (),
        };
        vec![]
    }
    fn process_replace_event(&mut self, event: &Event) -> Vec<Action> {
        match event {
            Event::Key(k) => match k {
                Key::Char(c) => self.text.replace(*c),
                Key::Up => self.text.up(),
                Key::Down => self.text.down(),
                Key::Right => self.text.right(),
                Key::Left => self.text.left(),
                Key::Esc => {
                    self.mode = Mode::None;
                    return vec![Action::StatusSet(self.mode.to_string())];
                }
                _ => (),
            },
            _ => (),
        };
        vec![]
    }
}

impl EventProcessor for Input {
    fn process_event(&mut self, event: &Event) -> Vec<Action> {
        eprintln!("PRoc event");
        let mut ret = match &self.mode {
            Mode::None => self.process_none_event(event),
            Mode::Edit(mode) => match mode {
                EditMode::Insert => self.process_insert_event(event),
                EditMode::Replace => self.process_replace_event(event),
            },
            Mode::Command => self.command.process_event(event),
        };
        if ret
            .iter()
            .filter(|act| match act {
                Action::FocusLoss => true,
                _ => false,
            })
            .count()
            > 0
        {
            if let Mode::None = self.mode {
                ret.push(Action::StatusSet("".to_string()));
                return ret;
            } else {
                self.mode = Mode::None;
                return ret
                    .into_iter()
                    .filter(|act| match act {
                        Action::FocusLoss => false,
                        _ => true,
                    })
                    .collect();
            }
        }
        ret
    }
}
