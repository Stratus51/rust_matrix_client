#[derive(Debug)]
pub struct Message {
    date: String,
    source: String,
    message: String,
}

#[derive(Debug)]
pub struct Unknown {
    ty: String,
    data: String,
}

// TODO source? timestamp?
#[derive(Debug)]
pub enum Event {
    Connected,
    Disconnected,
    Message(Message),
    Error(String),
    Unknown(Unknown),
    NewRoom(super::ui::Room),
}

impl Event {
    pub fn to_string(&self) -> (String, String) {
        match self {
            Event::Message(ev) => (
                [ev.date.as_str(), ": ", &ev.source, ": "].concat(),
                ev.message.clone(),
            ),
            Event::Error(s) => ("ERROR: ".to_string(), s.clone()),
            Event::Unknown(ev) => (["UNKNOWN EVENT: ", &ev.ty, ": "].concat(), ev.data.clone()),
            Event::Connected => ("CONN".to_string(), "Connected".to_string()),
            Event::Disconnected => ("CONN".to_string(), "Disconnected".to_string()),
            Event::NewRoom(r) => (
                "SPAWN".to_string(),
                format!("Room  {}", r.fmt_options.alias),
            ),
        }
    }
}

#[derive(Debug)]
pub struct MessageRequest {
    pub room: super::Id,
    pub msg: String,
}

#[derive(Debug)]
pub enum Request {
    Connect(super::Id),
    Disconnect(super::Id),
    Message(MessageRequest),
}
