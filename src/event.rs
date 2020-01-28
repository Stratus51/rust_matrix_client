use crate::room;
use std::io::stdin;
use std::sync::mpsc;
use termion::event::MouseEvent;
use termion::input::TermRead;

pub type Key = termion::event::Key;

// ==============================================================================================
// Event related definitions
// ==============================================================================================
#[derive(Debug)]
pub enum Event {
    Key(Key),
    Mouse(MouseEvent),
    Notification,
}

#[derive(Debug)]
pub enum Action {
    CopyBufferSet(String),
    StatusSet(String),
    Message(room::event::MessageRequest),
    TriggerMessage,
    FocusLoss,
}

pub trait EventProcessor {
    fn process_event(&mut self, event: &Event) -> Vec<Action>;
}

// ==============================================================================================
// Helpers
// ==============================================================================================
pub fn io_to_sink(sender: &mpsc::Sender<Event>) {
    let stdin = stdin();
    for c in stdin.events() {
        eprintln!("EV {:?}", c);
        let evt = c.unwrap();
        sender
            .send(match evt {
                termion::event::Event::Key(k) => Event::Key(k),
                termion::event::Event::Mouse(me) => Event::Mouse(me),
                e => panic!("Unknown event type: {:#?}", e),
            })
            .expect("Event sending should never fail!")
    }
}
