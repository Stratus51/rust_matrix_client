use crate::event::Event;
use std::io::stdin;
use termion::event::Event as TermEvent;
use termion::input::TermRead;
use tokio::sync::mpsc;

// ==============================================================================================
// Helpers
// ==============================================================================================
pub fn io_to_sink(mut sender: mpsc::Sender<crate::event::Event>) {
    let stdin = stdin();
    for c in stdin.events() {
        let evt = c.unwrap();
        // TODO Manage error case
        sender
            .try_send(match evt {
                TermEvent::Key(k) => Event::Key(k),
                TermEvent::Mouse(me) => Event::Mouse(me),
                e => panic!("Unknown event type: {:#?}", e),
            })
            .expect("Event sending should never fail!")
    }
}
