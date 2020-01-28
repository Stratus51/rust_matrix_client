use crate::event::{Action, Event as IoEvent, EventProcessor, Key};
use std::collections::HashMap;
use tui::style::{Color, Modifier, Style};

pub type Event = super::event::Event;
use super::{Id, StringId};

// =============================================================================
// Defines
// =============================================================================
pub struct PublishRequest {
    pub room: Id,
    pub msg: String,
}

pub enum Request {
    Connect(Id),
    Disconnect(Id),
    Pub(PublishRequest),
}

pub enum Response {
    None,
}

pub enum RequestError {
    Unknown(String),
}

// =============================================================================
// Room
// =============================================================================
#[derive(Debug)]
pub struct RoomFormatOptions {
    pub alias: StringId,
    pub meta_width: u16,
}

#[derive(Debug)]
pub struct Room {
    pub fmt_options: RoomFormatOptions,

    pub state: HashMap<String, String>,
    pub events: Vec<super::event::Event>,
    pub cursor: Option<usize>,
}

fn pad(s: &str, size: u16) -> String {
    match s.len().cmp(&(size as usize)) {
        std::cmp::Ordering::Greater => s[0..s.len()].to_string(),
        std::cmp::Ordering::Less => [s, &" ".repeat(size as usize - s.len())].concat(),
        std::cmp::Ordering::Equal => s.to_string(),
    }
}

const SIMPLE_STYLE: Style = Style {
    fg: Color::Reset,
    bg: Color::Reset,
    modifier: Modifier::empty(),
};

const HIGHLIGHT_STYLE: Style = Style {
    fg: Color::Reset,
    bg: Color::Reset,
    modifier: Modifier::empty(),
};

impl Room {
    pub fn new(fmt_options: RoomFormatOptions) -> Self {
        Self {
            fmt_options,
            state: HashMap::new(),
            events: vec![],
            cursor: None,
        }
    }

    fn stringify_events(&self, area: tui::layout::Rect) -> Vec<(String, Style)> {
        let w = area.width - self.fmt_options.meta_width;
        let ret = vec![];
        // TODO
        todo!();
        for (i, ev) in self.events.iter().enumerate().rev() {
            let selected = self.cursor.is_some() && *self.cursor.as_ref().unwrap() == i;
            let (meta, content) = ev.to_string();
            let mut s_list = vec![];
            let style = if selected {
                HIGHLIGHT_STYLE
            } else {
                SIMPLE_STYLE
            };
            for (i, s) in content
                .chars()
                .collect::<Vec<_>>()
                .chunks(w as usize)
                .map(|chunk| chunk.iter().collect::<String>())
                .enumerate()
            {
                let s = if i == 0 {
                    [pad(&meta, self.fmt_options.meta_width).as_str(), ": ", &s].concat()
                } else {
                    [pad("", self.fmt_options.meta_width + 2).as_str(), &s].concat()
                };
                s_list.push((s, style));
            }
            if ret.len() == area.height as usize {
                break;
            }
        }
        ret
    }
}

impl tui::widgets::Widget for Room {
    fn draw(&mut self, area: tui::layout::Rect, buf: &mut tui::buffer::Buffer) {
        for (i, (s, style)) in self.stringify_events(area).into_iter().enumerate() {
            buf.set_string(0, i as u16, s, style);
        }
    }
}

impl EventProcessor for Room {
    fn process_event(&mut self, event: &IoEvent) -> Vec<Action> {
        match event {
            IoEvent::Key(k) => match k {
                Key::Up => {
                    self.cursor = if self.cursor.is_some() {
                        Some(self.cursor.unwrap() - 1)
                    } else {
                        Some(self.events.len() - 1)
                    };
                }
                Key::Down => {
                    if let Some(mut cursor) = self.cursor {
                        cursor += 1;
                        if cursor >= self.events.len() {
                            self.cursor = None;
                        } else {
                            self.cursor = Some(cursor);
                        }
                    }
                }
                Key::Esc => return vec![Action::FocusLoss],
                _ => (),
            },
            _ => (),
        };
        vec![]
    }
}
