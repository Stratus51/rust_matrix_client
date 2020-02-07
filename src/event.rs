pub use crate::io;
use crate::room;
pub use ruma_events::collections::only::Event as MatrixEvent;
pub use ruma_events::presence::PresenceState as MatrixPresence;
pub use termion::event::{Key, MouseEvent};
use tokio::sync::mpsc;

// ==============================================================================================
// Events
// ==============================================================================================
#[derive(Debug)]
pub enum Event {
    Key(Key),
    Mouse(MouseEvent),
    Net(NetEvent),
}

#[derive(Debug)]
pub struct Message {
    pub date: String,
    pub source: String,
    pub message: String,
}

#[derive(Debug)]
pub struct Unknown {
    pub ty: String,
    pub data: String,
}

#[derive(Debug)]
pub enum PresenceState {
    Offline,
    Online,
    Unavailable,
}

#[derive(Debug)]
pub struct Presence {
    pub id: String,
    pub display_name: Option<String>,
    pub active: Option<bool>,
    pub status_msg: Option<String>,
    pub presence: MatrixPresence,
}

#[derive(Debug)]
pub struct NewRoom {
    pub id: Option<room::Id>,
    pub alias: String,
    pub requester: mpsc::Sender<room::net::Action>,
}

// TODO source? timestamp?
#[derive(Debug)]
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
    pub room: room::Id,
    pub event: NetEventKind,
}

impl NetEventKind {
    pub fn to_string(&self) -> (String, String) {
        match self {
            NetEventKind::Connected => ("ROOM".to_string(), "Connected".to_string()),
            NetEventKind::Disconnected => ("ROOM".to_string(), "Disconnected".to_string()),
            NetEventKind::Invite => ("ROOM".to_string(), "Invite".to_string()),
            NetEventKind::Message(ev) => (
                [ev.date.as_str(), ": ", &ev.source, ": "].concat(),
                ev.message.clone(),
            ),
            NetEventKind::NewRoom(r) => ("SPAWN".to_string(), format!("Room  {:?}", r)),
            NetEventKind::Presence(p) => ("PRES".to_string(), format!("Presence  {:?}", p)),
            NetEventKind::Error(s) => ("ERROR: ".to_string(), s.clone()),
            NetEventKind::Unknown(ev) => {
                (["UNKNOWN EVENT: ", &ev.ty, ": "].concat(), ev.data.clone())
            }
        }
    }
}

impl NetEvent {
    pub fn to_event(room: room::Id, event: NetEventKind) -> Event {
        Event::Net(NetEvent { room, event })
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
    Save,
    Quit,
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
