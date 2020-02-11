use crate::event::{Action, AppAction, CommandAction, Event, EventProcessor, Key};
use crate::widget::text::Text;
use tui::style::Style;

pub struct Command {
    text_widget: Text,
    focused: bool,
    history: Vec<String>,
    history_cursor: usize,
}

impl Command {
    fn history_up(&mut self) {
        if self.history_cursor > 0 {
            let hist_max = self.history.len() - 1;
            if self.history_cursor == hist_max {
                self.history[hist_max] = self.text_widget.text.consume();
            }

            self.history_cursor -= 1;
            self.text_widget
                .set_text(&self.history[self.history_cursor]);
            self.text_widget.text.end();
        }
    }

    fn history_down(&mut self) {
        if self.history_cursor < self.history.len() - 1 {
            self.history_cursor += 1;
            self.text_widget
                .set_text(&self.history[self.history_cursor]);
            self.text_widget.text.end();
        }
    }
}

impl Default for Command {
    fn default() -> Self {
        let mut text_widget = Text::new(&"");
        text_widget.one_line = true;
        text_widget.text.allow_cursor_over_limit = true;
        Self {
            text_widget,
            focused: false,
            // XXX Remove debug example
            history: vec![
                "spawn matrix matrix https://matrix.com.fr.gogor.ovh igor".to_string(),
                String::new(),
            ],
            history_cursor: 0 + 1,
        }
    }
}

impl tui::widgets::Widget for Command {
    fn draw(&mut self, mut area: tui::layout::Rect, buf: &mut tui::buffer::Buffer) {
        if area.width >= 2 && area.height > 0 {
            buf.set_string(area.x, area.y, ":", Style::default());
        }
        area.x += 1;
        self.text_widget.draw(area, buf);
    }
}

impl EventProcessor for Command {
    fn receive_focus(&mut self) {
        self.set_focus(true);
    }
    fn process_event(&mut self, event: Event) -> Vec<Action> {
        match event {
            Event::Key(k) => match k {
                Key::Char(c) => match c {
                    '\n' => {
                        self.set_focus(false);
                        return self.execute_command();
                    }
                    c => self.text_widget.text.insert(c),
                },
                Key::Backspace => self.text_widget.text.backspace(),
                Key::Up => self.history_up(),
                Key::Down => self.history_down(),
                Key::Right => self.text_widget.text.right(),
                Key::Left => self.text_widget.text.left(),
                Key::Home => self.text_widget.text.home(),
                Key::End => self.text_widget.text.end(),
                Key::Esc => {
                    self.set_focus(false);
                    self.text_widget.text.reset();
                    return vec![Action::FocusLoss];
                }
                _ => (),
            },
            Event::Mouse(_) => (), // TODO
            Event::Net(_) => panic!(),
        };
        vec![]
    }
}

// Command implementations
impl Command {
    fn set_focus(&mut self, focused: bool) {
        self.focused = focused;
        self.text_widget.show_cursor = focused;
    }

    fn execute_command(&mut self) -> Vec<Action> {
        let cmd_str = self.text_widget.text.consume();
        let hist_max = self.history.len() - 1;
        self.history[hist_max] = cmd_str.clone();
        self.history.push(String::new());
        let mut words: Vec<_> = cmd_str.split(' ').collect();
        let mut ret = vec![];

        if !words.is_empty() {
            let cmd = words.remove(0);
            let mut args = words;
            let mut unknown_cmd = false;

            // TODO Support multi character commands
            let actions = match cmd {
                "w" | "x" => vec![Action::Command(CommandAction::Save)],
                // TODO This x shortcut is too annoying as it sends a quit signal
                // 'x' => vec![
                //     Action::Command(CommandAction::Save),
                //     Action::Command(CommandAction::Quit),
                // ],
                "q" => vec![Action::Command(CommandAction::Quit)],
                "spawn" => {
                    if args.is_empty() {
                        vec![Action::App(AppAction::StatusSet(
                            "Syntax: spawn <alias> ...".to_string(),
                        ))]
                    } else {
                        let alias = args.remove(0).to_string();
                        vec![Action::Command(CommandAction::NewRoom(
                            crate::room::net::NewRoom {
                                alias,
                                command: args.iter().map(|&s| s.to_string()).collect(),
                            },
                        ))]
                    }
                }
                "connect" => vec![Action::Command(CommandAction::Connect)],
                "disconnect" => vec![Action::Command(CommandAction::Disconnect)],
                _ => {
                    unknown_cmd = true;
                    vec![]
                }
            };
            for action in actions.into_iter() {
                ret.push(action);
            }
            if unknown_cmd {
                ret.push(Action::App(AppAction::StatusSet(format!(
                    "Unknown command '{}'",
                    cmd
                ))))
            } else {
                ret.push(Action::App(AppAction::StatusSet(String::new())))
            }
        }
        ret.push(Action::FocusLoss);
        ret
    }
}
