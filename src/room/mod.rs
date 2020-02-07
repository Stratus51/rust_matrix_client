pub use crate::event::Event;
use crate::sequence_number::SequenceNumber;
use tokio::sync::mpsc;

pub mod net;
pub mod ui;

pub type StringId = String;
pub type Id = Vec<usize>;

// =============================================================================
// Room handles
// =============================================================================
#[derive(Debug)]
pub struct ServerHandle {
    pub input: mpsc::Sender<Event>,
    pub request: mpsc::Receiver<net::Action>,
    pub request_sn: SequenceNumber,
}

impl ServerHandle {
    pub fn new(input: mpsc::Sender<Event>, request: mpsc::Receiver<net::Action>) -> Self {
        ServerHandle {
            input,
            request,
            request_sn: SequenceNumber::new(),
        }
    }
    pub fn request_id(&mut self) -> usize {
        self.request_sn.next()
    }
}

#[derive(Debug)]
pub struct ClientHandle {
    pub input: mpsc::Receiver<Event>,
    pub request: mpsc::Sender<net::Action>,
}

#[derive(Debug)]
pub struct Handle {
    pub server: ServerHandle,
    pub client: ClientHandle,
}

impl Handle {
    pub fn new() -> Self {
        let (input_sender, input_receiver) = mpsc::channel(10);
        let (request_sender, request_receiver) = mpsc::channel(10);
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
#[derive(Debug)]
pub struct Room {
    pub ui: ui::Room,
    pub requester: mpsc::Sender<net::Action>,
}
