use crate::event::{Action, Event, EventProcessor, Key, NetEvent};
use crate::room;
use crate::widget::{scroll::Scroll, text::Text};
use std::collections::HashMap;

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
#[derive(Debug, Clone)]
pub struct Conf {
    pub alias: StringId,
    pub meta_width: u16,
}

#[derive(Debug)]
pub struct Room {
    pub id: Id,
    pub conf: Conf,

    pub state: HashMap<String, String>,
    pub events: Vec<NetEvent>,
    pub widget: Scroll,

    focused: bool,
}

impl Room {
    pub fn new(id: Id, conf: Conf) -> Self {
        Self {
            id,
            conf,
            state: HashMap::new(),
            events: vec![],
            widget: Scroll::new(vec![]),
            focused: false,
        }
    }
}

impl tui::widgets::Widget for Room {
    fn draw(&mut self, area: tui::layout::Rect, buf: &mut tui::buffer::Buffer) {
        self.widget.draw(area, buf);
    }
}

impl EventProcessor for Room {
    fn receive_focus(&mut self) {
        self.focused = true;
    }
    fn process_event(&mut self, event: Event) -> Vec<Action> {
        match event {
            Event::Key(k) => match k {
                Key::Up => self.widget.up(),
                Key::Down => self.widget.down(),
                Key::Esc => {
                    self.focused = false;
                    return vec![Action::FocusLoss];
                }
                _ => (),
            },
            Event::Mouse(_) => (),
            Event::Net(ev) => {
                self.events.push(ev);
            }
        };
        vec![]
    }
}
