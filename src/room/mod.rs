use crate::sequence_number::SequenceNumber;
use std::sync::mpsc;

pub mod event;
pub mod net;
pub mod ui;

pub type Event = event::Event;
pub type Request = event::Request;
pub type StringId = String;
pub type Id = usize;

// =============================================================================
// Room handles
// =============================================================================
#[derive(Debug)]
pub struct ServerHandle {
    pub input: mpsc::Sender<Event>,
    pub request: mpsc::Receiver<Request>,
    pub request_sn: SequenceNumber,
}

impl ServerHandle {
    pub fn request_id(&mut self) -> usize {
        self.request_sn.next()
    }
}

#[derive(Debug)]
pub struct ClientHandle {
    pub input: mpsc::Receiver<Event>,
    pub request: mpsc::Sender<Request>,
}

#[derive(Debug)]
pub struct Handle {
    pub server: ServerHandle,
    pub client: ClientHandle,
}

impl Handle {
    pub fn new() -> Self {
        let (input_sender, input_receiver) = mpsc::channel();
        let (request_sender, request_receiver) = mpsc::channel();
        Self {
            server: ServerHandle {
                input: input_sender,
                request: request_receiver,
                request_sn: SequenceNumber::new(),
            },
            client: ClientHandle {
                input: input_receiver,
                request: request_sender,
            },
        }
    }
}

// =============================================================================
// Room
// =============================================================================
pub struct Room {
    pub ui: ui::Room,
    pub handle: ClientHandle,
}
