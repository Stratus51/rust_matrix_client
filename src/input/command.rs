use crate::event::{Action, AppAction, CommandAction, Event, EventProcessor, Key};
use crate::widget::text::Text;
use tui::style::Style;

pub struct Command {
    text_widget: Text,
    focused: bool,
}

impl Default for Command {
    fn default() -> Self {
        let mut text_widget = Text::new(&"");
        text_widget.one_line = true;
        Self {
            text_widget,
            focused: false,
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
                Key::Up => self.text_widget.text.up(),
                Key::Down => self.text_widget.text.down(),
                Key::Right => self.text_widget.text.right(),
                Key::Left => self.text_widget.text.left(),
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
        let cmd = self.text_widget.text.consume();

        let mut ret = vec![];
        // TODO Support multi character commands
        for c in cmd.chars() {
            let actions = match c {
                'w' => vec![Action::Command(CommandAction::Save)],
                'q' => vec![Action::Command(CommandAction::Quit)],
                'x' => vec![
                    Action::Command(CommandAction::Save),
                    Action::Command(CommandAction::Quit),
                ],
                x => {
                    return vec![
                        Action::App(AppAction::StatusSet(format!(
                            "Unknown command '{}' (from {})",
                            x, cmd
                        ))),
                        Action::FocusLoss,
                    ]
                }
            };
            for action in actions.into_iter() {
                ret.push(action);
            }
        }
        ret.push(Action::FocusLoss);
        ret
    }
}
