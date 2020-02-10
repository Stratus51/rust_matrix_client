pub use crate::io;
use crate::room;
use chrono::offset::Utc;
pub use ruma_events::collections::only::Event as MatrixEvent;
pub use ruma_events::presence::PresenceState as MatrixPresence;
use std::fmt;
pub use termion::event::{Key, MouseEvent};
use tokio::sync::mpsc;

fn now() -> usize {
    Utc::now().timestamp() as usize
}

// ==============================================================================================
// Events
// ==============================================================================================
#[derive(Debug)]
pub enum Event {
    Key(Key),
    Mouse(MouseEvent),
    Net(NetEvent),
}

#[derive(Debug, Clone)]
pub struct Message {
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct Unknown {
    pub ty: String,
    pub data: String,
}

#[derive(Debug, Clone)]
pub enum PresenceState {
    Offline,
    Online,
    Unavailable,
}

#[derive(Debug, Clone)]
pub struct Presence {
    pub id: String,
    pub display_name: Option<String>,
    pub active: Option<bool>,
    pub status_msg: Option<String>,
    pub presence: MatrixPresence,
}

#[derive(Debug, Clone)]
pub struct NewRoom {
    pub id: Option<room::Id>,
    pub alias: String,
    pub requester: mpsc::Sender<room::net::Action>,
}

// TODO source? timestamp?
#[derive(Debug, Clone)]
pub enum NetEventKind {
    Connected,
    Disconnected,
    Invite,
    Message(Message),
    NewRoom(NewRoom),
    Presence(Presence),
    Error(String),
    Unknown(Unknown),
}

#[derive(Debug)]
pub struct NetEvent {
    pub date: usize,
    pub room: room::Id,
    pub source: Option<String>, // TODO ID instead
    pub event: NetEventKind,
}

impl fmt::Display for NetEventKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                NetEventKind::Connected => "Room connected".to_string(),
                NetEventKind::Disconnected => "Room disconnected".to_string(),
                NetEventKind::Invite => "Room invitation".to_string(),
                NetEventKind::Message(ev) => {
                    ev.content.clone()
                }
                NetEventKind::NewRoom(r) => format!("Spawned room  {:?}", r),
                NetEventKind::Presence(p) => format!("Presence  {:?}", p),
                NetEventKind::Error(s) => ["ERROR: ".to_string(), s.clone()].concat(),
                NetEventKind::Unknown(ev) => ["UNKNOWN EVENT: ", &ev.ty, ": ", &ev.data].concat(),
            }
        )
    }
}

impl NetEventKind {
    pub fn to_event(&self, room: room::Id, date: usize, source: Option<String>) -> Event {
        Event::Net(NetEvent {
            date,
            room,
            source,
            event: self.clone(),
        })
    }

    pub fn to_current_event(&self, room: room::Id, source: Option<String>) -> Event {
        self.to_event(room, now(), source)
    }
}

// ==============================================================================================
// Actions
// ==============================================================================================

// TODO Properly categorize actions. Category should probably be the receiving process
#[derive(Debug)]
pub enum Action {
    Command(CommandAction),
    Input(InputAction),
    Room(RoomAction),
    App(AppAction),
    FocusLoss,
}

#[derive(Debug)]
pub enum CommandAction {
    Connect,
    Disconnect,
    NewRoom(room::net::NewRoom),
    Quit,
    Save,
}

#[derive(Debug)]
pub enum InputAction {
    Message(String),
}

#[derive(Debug)]
pub struct RoomPublish {
    pub id: crate::room::Id,
    pub msg: String,
}

#[derive(Debug)]
pub enum RoomAction {
    Publish(RoomPublish),
}

#[derive(Debug)]
pub enum AppAction {
    CopyBufferSet(String),
    StatusSet(String),
}

// ==============================================================================================
// Event processor trait
// ==============================================================================================
pub trait EventProcessor {
    fn receive_focus(&mut self);
    fn process_event(&mut self, event: Event) -> Vec<Action>;
}
