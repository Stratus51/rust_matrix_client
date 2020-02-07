pub mod app;
pub mod event;
pub mod input;
pub mod io;
pub mod room;
pub mod sequence_number;
pub mod text;
pub mod widget;

#[tokio::main]
async fn main() -> Result<(), app::Error> {
    let mut app = app::App::new(app::Options {
        max_input_height: 10,
    });

    // Catch UI I/Os
    let io_sender = app.sender.clone();
    std::thread::spawn(move || {
        io::io_to_sink(io_sender);
    });

    app.run().await
}
