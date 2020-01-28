use std::sync::mpsc;

mod app;
mod event;
mod input;
mod room;
mod sequence_number;

fn main() -> Result<(), app::Error> {
    let mut app = app::App::new(app::Options {
        max_input_height: 10,
    });

    // Catch UI I/Os
    let io_sender = app.sender.clone();
    std::thread::spawn(move || {
        event::io_to_sink(&io_sender);
    });

    app.handle_events()
}
