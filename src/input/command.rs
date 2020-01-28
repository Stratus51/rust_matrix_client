use super::editable_text::{EditableText, StringBlockItem};
use crate::event::{Action, Event, EventProcessor, Key};

pub struct Command {
    text: EditableText,
    text_cursor: usize,
}

impl Command {
    pub fn new() -> Self {
        Self {
            text: EditableText::new(),
            text_cursor: 0,
        }
    }

    fn fix_text_cursor(&mut self, width: usize) {
        if self.text_cursor + width < self.text.cursor.c {
            self.text_cursor = self.text.cursor.c - width;
        } else if self.text_cursor > self.text.cursor.c {
            self.text_cursor = self.text.cursor.c;
        }
    }
}

impl tui::widgets::Widget for Command {
    fn draw(&mut self, area: tui::layout::Rect, buf: &mut tui::buffer::Buffer) {
        // Fix text_cursor to follow terminal size changes and cursor movements
        self.fix_text_cursor(area.width as usize);
        for StringBlockItem {
            x,
            y: _y,
            s: line,
            style,
        } in self.text.one_line_string(self.text_cursor, area.width)
        {
            buf.set_stringn(
                area.x + x,
                area.y + area.height - 1,
                line,
                area.width as usize,
                style,
            );
        }
    }
}

impl EventProcessor for Command {
    fn process_event(&mut self, event: &Event) -> Vec<Action> {
        match event {
            Event::Key(k) => match k {
                Key::Char(c) => match c {
                    '\n' => {
                        return self.execute_command();
                    }
                    c => self.text.insert(*c),
                },
                Key::Backspace => self.text.backspace(),
                Key::Up => self.text.up(),
                Key::Down => self.text.down(),
                Key::Right => self.text.right(),
                Key::Left => self.text.left(),
                Key::Esc => {
                    self.text.reset();
                    return vec![Action::FocusLoss];
                }
                _ => (),
            },
            _ => (),
        };
        vec![]
    }
}

// Command implementations
impl Command {
    fn execute_command(&mut self) -> Vec<Action> {
        let cmd = self.text.lines[0].clone();

        self.text.reset();

        let mut ret = vec![];
        match cmd.as_str() {
            "x" | "wq" => ret.push(Action::TriggerMessage),
            x => ret.push(Action::StatusSet(format!("Unknown command '{}'", x))),
        }
        ret.push(Action::FocusLoss);
        ret
    }
}
