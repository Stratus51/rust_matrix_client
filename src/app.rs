use crate::event::{Action, Event, EventProcessor, Key};
use crate::input::Input;
use crate::room;
use crate::sequence_number::SequenceNumber;
use std::io;
use std::sync::mpsc;
use termion::raw::IntoRawMode;
use tui::backend::TermionBackend;
use tui::layout::{Constraint, Direction, Layout};
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Block, Borders, Paragraph, SelectableList, Text, Widget};
use tui::Terminal;

enum Focus {
    None,
    Room,
    Input,
    Command,
}

impl Focus {
    fn to_string(&self) -> String {
        match self {
            Focus::None => "Idle",
            Focus::Room => "Room",
            Focus::Input => "Message",
            Focus::Command => "Command",
        }
        .to_string()
    }
}

struct Context {
    copy_buffer: String,
    status: String,
}

impl Context {
    fn new() -> Self {
        Self {
            copy_buffer: String::new(),
            status: String::new(),
        }
    }
}

pub struct Options {
    pub max_input_height: u16,
}

pub struct App {
    options: Options,

    context: Context,
    rooms: Vec<room::Room>,
    current_room: room::Id,
    room_sn: SequenceNumber,

    input: Input,
    focus: Focus,

    receiver: mpsc::Receiver<Event>,
    pub sender: mpsc::Sender<Event>,
}

const AppRoomId: usize = 0;

impl App {
    pub fn new(options: Options) -> Self {
        let (sender, receiver) = mpsc::channel();
        let mut ret = Self {
            options,
            context: Context::new(),
            rooms: vec![],
            current_room: AppRoomId,
            room_sn: SequenceNumber::new(),
            input: Input::new(),
            focus: Focus::None,
            receiver,
            sender,
        };
        ret.add_app_room();
        ret
    }

    fn add_room(&mut self, room: room::Room) {
        self.rooms.push(room);
    }

    fn add_app_room(&mut self) {
        let room::Handle { server, client } = room::Handle::new();
        let app = room::net::app::App::new(self.sender.clone(), server);
        let room = room::Room {
            ui: room::ui::Room::new(room::ui::RoomFormatOptions {
                alias: "Main".to_string(),
                meta_width: 10,
            }),
            handle: client,
        };
        self.add_room(room);
        app.start();
    }

    fn update_context(&mut self, ctx_mod: Action) {
        match ctx_mod {
            Action::CopyBufferSet(buf) => self.context.copy_buffer = buf,
            Action::StatusSet(status) => self.context.status = status,
            Action::FocusLoss => self.focus = Focus::None,
            Action::Message(msg) => {
                self.rooms[self.current_room]
                    .handle
                    .request
                    .send(room::Request::Message(msg));
            }
            Action::TriggerMessage => panic!(),
        }
    }

    fn process_context_less_event(&mut self, event: &Event) -> Vec<Action> {
        match event {
            Event::Key(k) => match k {
                Key::Char(c) => match c {
                    'm' => {
                        self.focus = Focus::Input;
                        vec![]
                    }
                    'r' => {
                        self.focus = Focus::Room;
                        vec![]
                    }
                    ':' => {
                        self.focus = Focus::Command;
                        vec![]
                    }
                    _ => vec![],
                },
                Key::Esc => vec![Action::FocusLoss],
                _ => vec![],
            },
            _ => vec![],
        }
    }

    pub fn handle_events(&mut self) -> Result<(), Error> {
        // Initialization ------------------------------------------------------
        let stdout = io::stdout().into_raw_mode()?;
        let backend = TermionBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        // terminal.clear()?;
        // terminal.hide_cursor()?;

        loop {
            eprintln!("plip plop draw");
            // UI refresh -------------------------------------------------------
            terminal.draw(|mut f| {
                eprintln!("termion size");
                let (t_w, _t_h) = match termion::terminal_size() {
                    Ok((w, h)) => (w, h),
                    Err(e) => panic!("{:#?}", e),
                };
                eprintln!("wanted size");
                let mut input_size = self.input.wanted_size(t_w);
                if input_size > self.options.max_input_height {
                    input_size = self.options.max_input_height;
                }
                eprintln!("input size: {}", input_size);

                let main_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(
                        [
                            Constraint::Min(1),
                            Constraint::Length(input_size),
                            Constraint::Length(1),
                        ]
                        .as_ref(),
                    )
                    .split(f.size());

                let content_layout = Layout::default()
                    .direction(Direction::Horizontal)
                    .margin(1)
                    .constraints([Constraint::Percentage(20), Constraint::Percentage(80)].as_ref())
                    .split(main_layout[0]);

                SelectableList::default()
                    .block(Block::default().title("Room list").borders(Borders::ALL))
                    .items(
                        &self
                            .rooms
                            .iter()
                            .map(|r| &r.ui.fmt_options.alias)
                            .collect::<Vec<_>>(),
                    )
                    .select(Some(self.current_room))
                    .style(Style::default().fg(Color::White))
                    .highlight_style(Style::default().modifier(Modifier::ITALIC).bg(Color::Blue))
                    .render(&mut f, content_layout[0]);

                let mut block = Block::default().title("Room").borders(Borders::ALL);
                block.render(&mut f, content_layout[1]);
                let room_space = block.inner(content_layout[1]);
                if self.rooms.len() <= self.current_room {
                    self.current_room = 0;
                }
                eprintln!("print rooms");
                self.rooms[self.current_room].ui.render(&mut f, room_space);

                eprintln!("print input");
                self.input.render(&mut f, main_layout[1]);
                Paragraph::new(self.build_status_line().iter()).render(&mut f, main_layout[2]);
            })?;

            // Event processing -------------------------------------------------
            // Wait for events
            eprintln!("plip plop event wait");
            let event = self.receiver.recv()?;
            eprintln!("plip plop event caught");
            let ctx_mods = self.process_event(&event);
            for ctx_mod in ctx_mods.into_iter() {
                self.update_context(ctx_mod);
            }

            // Process all queued events
            loop {
                eprintln!("ev !");
                let ctx_mods = match self.receiver.try_recv() {
                    Ok(ev) => self.process_event(&ev),
                    Err(fail) => match fail {
                        mpsc::TryRecvError::Empty => break,
                        mpsc::TryRecvError::Disconnected => return Err(Error::EventStarvation),
                    },
                };
                eprintln!("Update CTX: {:#?}", ctx_mods);
                for ctx_mod in ctx_mods.into_iter() {
                    self.update_context(ctx_mod);
                }
            }
        }
    }

    fn build_status_line(&self) -> Vec<Text> {
        vec![Text::raw(
            [self.focus.to_string().as_str(), " | ", &self.context.status].concat(),
        )]
    }
}

impl EventProcessor for App {
    fn process_event(&mut self, event: &Event) -> Vec<Action> {
        if let Event::Key(Key::Ctrl('c')) = event {
            std::process::exit(0);
        }
        match self.focus {
            Focus::None => self.process_context_less_event(event),
            Focus::Room => {
                if self.rooms.len() > self.current_room {
                    return self.rooms[self.current_room].ui.process_event(event);
                }
                self.current_room = 0;
                self.focus = Focus::None;
                vec![]
            }
            Focus::Input => self.input.process_event(event),
            Focus::Command => todo!(),
        }
    }
}

#[derive(Debug)]
pub enum Error {
    EventStarvation,
    Io(std::io::Error),
}

impl std::convert::From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl std::convert::From<mpsc::RecvError> for Error {
    fn from(_: mpsc::RecvError) -> Self {
        Self::EventStarvation
    }
}
