use crate::event::{Action, AppAction, Event, EventProcessor, Key};
use crate::widget::{text::Text, Height};

pub mod command;

pub enum Mode {
    None,
    Insert,
    Replace,
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Mode::None => "",
                Mode::Insert => "insert",
                Mode::Replace => "replace",
            }
        )
    }
}

pub struct Input {
    mode: Mode,
    pub text_widget: Text,
    focused: bool,
}

impl Default for Input {
    fn default() -> Self {
        Self {
            mode: Mode::None,
            text_widget: Text::new(&""),
            focused: false,
        }
    }
}

impl Height for Input {
    fn height(&self, width: u16) -> usize {
        self.text_widget.height(width)
    }
}

impl tui::widgets::Widget for Input {
    fn draw(&mut self, area: tui::layout::Rect, buf: &mut tui::buffer::Buffer) {
        self.text_widget.draw(area, buf);
    }
}

impl Input {
    fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
        self.text_widget.show_cursor = focused;
    }

    pub fn set_append_mode(&mut self) {
        if let Mode::None = self.mode {
            self.set_insert_mode();
            self.text_widget.text.right();
        }
    }

    pub fn set_insert_mode(&mut self) {
        if let Mode::None = self.mode {
            self.text_widget.text.allow_cursor_over_limit = true;
            self.mode = Mode::Insert;
        }
    }

    pub fn set_replace_mode(&mut self) {
        if let Mode::None = self.mode {
            self.text_widget.text.allow_cursor_over_limit = true;
            self.mode = Mode::Replace;
        }
    }

    // Mode event processing implementation
    fn process_none_event(&mut self, event: Event) -> Vec<Action> {
        match event {
            Event::Key(k) => match k {
                Key::Char(c) => {
                    match c {
                        'h' => self.text_widget.text.left(),
                        'l' => self.text_widget.text.right(),
                        'j' => self.text_widget.text.down(),
                        'k' => self.text_widget.text.up(),
                        'i' => {
                            self.text_widget.text.allow_cursor_over_limit = true;
                            self.mode = Mode::Insert;
                        }
                        'a' => {
                            self.text_widget.text.allow_cursor_over_limit = true;
                            self.text_widget.text.right();
                            self.mode = Mode::Insert;
                        }
                        'r' => self.mode = Mode::Replace,
                        _ => (),
                    };
                    return vec![Action::App(AppAction::StatusSet(self.mode.to_string()))];
                }
                Key::Up => self.text_widget.text.up(),
                Key::Down => self.text_widget.text.down(),
                Key::Right => self.text_widget.text.right(),
                Key::Left => self.text_widget.text.left(),
                Key::Home => self.text_widget.text.home(),
                Key::End => self.text_widget.text.end(),
                _ => (),
            },
            Event::Mouse(_) => (),
            Event::Net(_) => panic!(),
        };
        vec![]
    }
    fn process_insert_event(&mut self, event: Event) -> Vec<Action> {
        match event {
            Event::Key(k) => match k {
                Key::Char(c) => self.text_widget.text.insert(c),
                Key::Backspace => self.text_widget.text.backspace(),
                Key::Up => self.text_widget.text.up(),
                Key::Down => self.text_widget.text.down(),
                Key::Right => self.text_widget.text.right(),
                Key::Left => self.text_widget.text.left(),
                Key::Home => self.text_widget.text.home(),
                Key::End => self.text_widget.text.end(),
                Key::Esc => {
                    self.text_widget.text.left();
                    self.mode = Mode::None;
                    return vec![Action::FocusLoss];
                }
                x => eprintln!("_ = {:?}", x),
            },
            Event::Mouse(_) => (),
            Event::Net(_) => panic!(),
        };
        vec![]
    }
    fn process_replace_event(&mut self, event: Event) -> Vec<Action> {
        match event {
            Event::Key(k) => match k {
                Key::Char(c) => self.text_widget.text.replace(c),
                Key::Up => self.text_widget.text.up(),
                Key::Down => self.text_widget.text.down(),
                Key::Right => self.text_widget.text.right(),
                Key::Left => self.text_widget.text.left(),
                Key::Home => self.text_widget.text.home(),
                Key::End => self.text_widget.text.end(),
                Key::Esc => {
                    self.text_widget.text.left();
                    self.mode = Mode::None;
                    return vec![Action::FocusLoss];
                }
                _ => (),
            },
            Event::Mouse(_) => (),
            Event::Net(_) => panic!(),
        };
        vec![]
    }
}

impl EventProcessor for Input {
    fn receive_focus(&mut self) {
        self.set_focused(true);
    }

    fn process_event(&mut self, event: Event) -> Vec<Action> {
        let tmp_ret = match &self.mode {
            Mode::None => self.process_none_event(event),
            Mode::Insert => self.process_insert_event(event),
            Mode::Replace => self.process_replace_event(event),
        };

        // TODO It is debatable whether we should allow the use of multi layer commands (quit,
        // quit, quit in a single command, or even commands to higher layer because this layer
        // does not interpret it)
        let mut ret = vec![];
        let mut focus_loss = false;
        for act in tmp_ret.into_iter() {
            match act {
                Action::FocusLoss => {
                    if focus_loss {
                        ret.push(Action::FocusLoss);
                    } else if let Mode::None = self.mode {
                        ret.push(Action::FocusLoss);
                        ret.push(Action::App(AppAction::StatusSet("".to_string())));
                    } else {
                        self.mode = Mode::None;
                        focus_loss = true;
                        self.text_widget.text.allow_cursor_over_limit = false;
                    }
                }
                act => ret.push(act),
            }
        }
        ret
    }
}
