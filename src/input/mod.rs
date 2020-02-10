use crate::event::{Action, AppAction, CommandAction, Event, EventProcessor, InputAction, Key};
use crate::widget::{text::Text, Height};

pub mod command;
use command::Command;

pub enum EditMode {
    Insert,
    Replace,
}
impl std::fmt::Display for EditMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                EditMode::Insert => "insert",
                EditMode::Replace => "replace",
            }
        )
    }
}

pub enum Mode {
    None,
    Edit(EditMode),
    Command,
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Mode::None => String::new(),
                Mode::Edit(e) => format!("edit: {}", e.to_string()),
                Mode::Command => "command".to_string(),
            }
        )
    }
}

pub struct Input {
    mode: Mode,
    pub text_widget: Text,
    command: Command,
    focused: bool,
}

impl Default for Input {
    fn default() -> Self {
        Self {
            mode: Mode::None,
            text_widget: Text::new(&""),
            command: Command::default(),
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

        // If command mode, overwrite last line
        if let Mode::Command = &self.mode {
            let mut area = area;
            area.y = area.y + area.height - 1;
            area.height = 1;
            self.command.draw(area, buf);
        }
    }
}

impl Input {
    fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
        self.text_widget.show_cursor = focused;
    }

    // Mode event processing implementation
    fn process_none_event(&mut self, event: Event) -> Vec<Action> {
        match event {
            Event::Key(k) => match k {
                Key::Char(c) => {
                    match c {
                        'i' => {
                            self.text_widget.text.allow_cursor_over_limit = true;
                            self.mode = Mode::Edit(EditMode::Insert);
                        }
                        'a' => {
                            self.text_widget.text.allow_cursor_over_limit = true;
                            self.text_widget.text.right();
                            self.mode = Mode::Edit(EditMode::Insert);
                        }
                        'r' => self.mode = Mode::Edit(EditMode::Replace),
                        ':' => {
                            self.mode = Mode::Command;
                            self.command.receive_focus();
                        }
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
                Key::Esc => return vec![Action::FocusLoss],
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
                    self.mode = Mode::None;
                    return vec![Action::App(AppAction::StatusSet(self.mode.to_string()))];
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
                    self.mode = Mode::None;
                    return vec![Action::App(AppAction::StatusSet(self.mode.to_string()))];
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
            Mode::Edit(mode) => match mode {
                EditMode::Insert => self.process_insert_event(event),
                EditMode::Replace => self.process_replace_event(event),
            },
            Mode::Command => self.command.process_event(event),
        };

        // TODO It is debatable whether we should allow the use of multi layer commands (quit,
        // quit, quit in a single command, or even commands to higher layer because this layer
        // does not interpret it)
        let mut ret = vec![];
        let mut focus_loss = false;
        let mut trigger_message = false;
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
                Action::Command(cmd) => match cmd {
                    CommandAction::Save => {
                        if trigger_message {
                            ret.push(Action::Command(CommandAction::Save));
                        } else {
                            trigger_message = true;
                            ret.push(Action::Input(InputAction::Message(
                                self.text_widget.text.consume(),
                            )));
                        }
                    }
                    CommandAction::Quit => {
                        // TODO Don't know what to do
                        // Quitting input mode is a single Esc press.
                        // Throwing the input text should require a force option but then the
                        // simple quit won't mean anything anymore
                        ret.push(Action::Command(CommandAction::Quit))
                    }
                    x => ret.push(Action::Command(x)),
                },
                act => ret.push(act),
            }
        }
        ret
    }
}
