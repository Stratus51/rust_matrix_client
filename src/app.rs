use crate::event::{
    Action, AppAction, CommandAction, Event, EventProcessor, InputAction, Key, NetEvent,
    NetEventKind,
};
use crate::gui_dbg;
use crate::input::{command::Command, Input};
use crate::room;
use crate::sequence_number::SequenceNumber;
use crate::widget::Height;
use std::collections::HashMap;
use std::io;
use std::sync::Arc;
use termion::raw::IntoRawMode;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
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

impl std::fmt::Display for Focus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Focus::None => "Idle",
                Focus::Room => "Room",
                Focus::Input => "Message",
                Focus::Command => "Command",
            }
        )
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

pub struct Room {
    ui: room::ui::Room,
    net_sender: mpsc::Sender<room::net::Action>,
}

enum LoopAction {
    Quit,
    Dummy, // XXX Just for clippy to stop complaining
}

pub struct App {
    options: Options,

    context: Context,
    rooms: HashMap<room::Id, Room>,
    rooms_id: Vec<room::Id>,
    current_room: room::Id,

    input: Input,
    command: Command,
    focus: Focus,

    receiver: mpsc::Receiver<Event>,
    pub sender: mpsc::Sender<Event>,

    room_sn: Arc<Mutex<SequenceNumber>>,
}

impl App {
    pub fn new(options: Options) -> Self {
        let (sender, receiver) = mpsc::channel(100);
        let mut ret = Self {
            options,
            context: Context::new(),
            rooms: HashMap::new(),
            rooms_id: vec![],
            current_room: 0,
            input: Input::default(),
            command: Command::default(),
            focus: Focus::None,
            receiver,
            sender,
            room_sn: Arc::new(Mutex::new(SequenceNumber::default())),
        };
        ret.add_root_room();
        ret
    }

    fn add_root_room(&mut self) {
        let id = self.room_sn.try_lock().unwrap().next().unwrap();

        let (tx, rx) = mpsc::channel(10);
        let app = room::net::app::App::new(
            id,
            room::ServerHandle::new(self.sender.clone(), rx),
            self.room_sn.clone(),
        );
        app.start();
        self.add_room(id, "main".to_string(), tx);
    }

    fn add_room(&mut self, id: room::Id, name: String, requester: mpsc::Sender<room::net::Action>) {
        match self.rooms.insert(
            id,
            Room {
                ui: room::ui::Room::new(
                    id,
                    room::ui::Conf {
                        alias: name,
                        meta_width: 16,
                    },
                ),
                net_sender: requester,
            },
        ) {
            None => (),
            Some(_) => panic!("Room ID collision !"),
        }
        self.rooms_id.push(id);
    }

    fn get_mut_room(&mut self, id: room::Id) -> Option<&mut Room> {
        self.rooms.get_mut(&id)
    }

    fn get_room(&self, id: room::Id) -> Option<&Room> {
        self.rooms.get(&id)
    }

    fn mut_room(&mut self) -> &mut Room {
        self.get_mut_room(self.rooms_id[self.current_room]).unwrap()
    }

    fn room(&self) -> &Room {
        self.get_room(self.rooms_id[self.current_room]).unwrap()
    }

    async fn room_send(&mut self, action: room::net::ActionKind) {
        let action = room::net::Action {
            room: self.current_room,
            action,
        };
        self.mut_room()
            .net_sender
            .send(action)
            .await
            .expect("TODO Implement room exiting");
    }

    async fn execute_action(&mut self, ctx_mod: Action) -> Vec<LoopAction> {
        let mut ret = vec![];
        match ctx_mod {
            Action::Input(act) => match act {
                InputAction::Message(msg) => {
                    self.room_send(room::net::ActionKind::Publish(msg)).await
                }
            },
            Action::Command(act) => match act {
                CommandAction::Save => panic!(),
                CommandAction::Quit => ret.push(LoopAction::Quit),
                CommandAction::NewRoom(r) => {
                    self.room_send(room::net::ActionKind::NewRoom(r)).await
                }
                CommandAction::Connect => self.room_send(room::net::ActionKind::Connect).await,
                CommandAction::Disconnect => {
                    self.room_send(room::net::ActionKind::Disconnect).await
                }
            },
            Action::Room(_) => todo!(),
            Action::App(act) => match act {
                AppAction::CopyBufferSet(buf) => self.context.copy_buffer = buf,
                AppAction::StatusSet(status) => self.context.status = status,
            },
            Action::FocusLoss => self.focus = Focus::None,
        }
        ret
    }

    fn process_net_event(&mut self, event: NetEvent) -> Vec<Action> {
        let NetEvent {
            date,
            room,
            event,
            source,
        } = event;
        match event {
            ev @ NetEventKind::Connected
            | ev @ NetEventKind::Disconnected
            | ev @ NetEventKind::Invite
            | ev @ NetEventKind::Message(_)
            | ev @ NetEventKind::Presence(_)
            | ev @ NetEventKind::Error(_)
            | ev @ NetEventKind::Unknown(_) => match self.get_mut_room(room) {
                Some(r) => r.ui.process_event(ev.to_event(room, date, source)),
                None => {
                    eprintln!("Received message from dead room {}: {:?}", room, ev);
                    vec![]
                }
            },
            NetEventKind::NewRoom(r) => {
                self.add_room(r.id.unwrap(), r.alias, r.requester);
                vec![]
            }
        }
    }

    fn process_ui_event(&mut self, event: Event) -> Vec<Action> {
        match self.focus {
            Focus::None => self.process_context_less_event(event),
            Focus::Room => self.mut_room().ui.process_event(event),
            Focus::Input => self.input.process_event(event),
            Focus::Command => self.command.process_event(event),
        }
    }

    fn process_context_less_event(&mut self, event: Event) -> Vec<Action> {
        // TODO The ergonomy of these shortcuts is very debatable
        match event {
            Event::Key(k) => match k {
                Key::Char(c) => match c {
                    'm' => {
                        self.focus = Focus::Input;
                        self.input.receive_focus();
                        vec![]
                    }
                    'r' => {
                        self.mut_room().ui.receive_focus();
                        self.focus = Focus::Room;
                        vec![]
                    }
                    ':' => {
                        self.command.receive_focus();
                        self.focus = Focus::Command;
                        vec![]
                    }
                    _ => vec![],
                },
                Key::Down => {
                    if self.current_room < self.rooms_id.len() - 1 {
                        self.current_room += 1;
                    }
                    vec![]
                }
                Key::Up => {
                    if self.current_room > 0 {
                        self.current_room -= 1;
                    }
                    vec![]
                }
                Key::Esc => vec![Action::FocusLoss],
                _ => vec![],
            },
            _ => vec![],
        }
    }

    pub async fn run(&mut self) -> Result<(), Error> {
        // Initialization ------------------------------------------------------
        let stdout = io::stdout().into_raw_mode()?;
        let backend = TermionBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        terminal.clear()?;
        terminal.hide_cursor()?;

        'main: loop {
            gui_dbg!(
                "--------------------------------------------------------------------------------"
            );
            gui_dbg!(
                "================================================================================"
            );
            gui_dbg!("New frame");
            gui_dbg!(
                "================================================================================"
            );
            gui_dbg!(
                "--------------------------------------------------------------------------------"
            );
            // UI refresh -------------------------------------------------------
            terminal.draw(|mut f| {
                gui_dbg!("================================================================================");
                gui_dbg!("Widget precalculations");
                gui_dbg!("================================================================================");
                let (t_w, t_h) = match termion::terminal_size() {
                    Ok((w, h)) => (w, h),
                    Err(e) => panic!("{:#?}", e),
                };
                let  input_size = if let Focus::Input = self.focus {
                    usize::min(self.input.height(t_w), t_h as usize/2)
                } else if self.input.text_widget.text.is_empty() {
                    0
                } else {
                    usize::min(self.input.height(t_w), 5)
                };

                let main_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(
                        [
                            Constraint::Min(1),
                            Constraint::Length(input_size as u16),
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

                let room_list: Vec<_> = self
                    .rooms_id
                    .iter()
                    .map(|id| self.rooms.get(id).unwrap().ui.conf.alias.clone()).collect();

                // TODO OPTIM: Redraw only widget that have changed
                gui_dbg!("================================================================================");
                gui_dbg!("Rendering room list");
                gui_dbg!("================================================================================");
                SelectableList::default()
                    .block(Block::default().title("Room list").borders(Borders::ALL))
                    .items(&room_list)
                    .select(Some(self.current_room))
                    .style(Style::default().fg(Color::White))
                    .highlight_style(Style::default().modifier(Modifier::ITALIC).bg(Color::Blue))
                    .render(&mut f, content_layout[0]);

                gui_dbg!("================================================================================");
                gui_dbg!("Rendering current room");
                gui_dbg!("================================================================================");
                let mut block = Block::default().title(&self.room().ui.conf.alias).borders(Borders::ALL);
                block.render(&mut f, content_layout[1]);
                let room_space = block.inner(content_layout[1]);
                self.mut_room().ui.render(&mut f, room_space);

                gui_dbg!("================================================================================");
                gui_dbg!("Rendering input");
                gui_dbg!("================================================================================");
                self.input.render(&mut f, main_layout[1]);
                Paragraph::new(self.build_status_line().iter()).render(&mut f, main_layout[2]);

                if let Focus::Command = self.focus {
                    gui_dbg!("================================================================================");
                    gui_dbg!("Rendering command");
                    gui_dbg!("================================================================================");
                    let t_size = f.size();
                    if t_size.height >= 5 {
                        let command_layout = tui::layout::Rect {
                            x: 0,
                            y: t_size.height - 2,
                            width: t_size.width,
                            height: 1,
                        };
                        self.command.render(&mut f, command_layout);
                    }
                }
            })?;

            // Event processing -------------------------------------------------
            // Wait for events
            let events = match self.receiver.recv().await {
                Some(ev) => {
                    let mut events = vec![ev];
                    loop {
                        events.push(match self.receiver.try_recv() {
                            Ok(ev) => ev,
                            Err(fail) => match fail {
                                mpsc::error::TryRecvError::Empty => break events,
                                mpsc::error::TryRecvError::Closed => {
                                    return Err(Error::EventStarvation)
                                }
                            },
                        });
                    }
                }
                None => return Err(Error::EventStarvation),
            };

            // Process the events
            let mut actions = vec![];
            for ev in events.into_iter() {
                for action in self.process_event(ev).into_iter() {
                    actions.push(action);
                }
            }

            // Process the resulting actions
            let mut loop_actions = vec![];
            for action in actions.into_iter() {
                for laction in self.execute_action(action).await {
                    loop_actions.push(laction);
                }
            }

            // Process the loop actions
            for laction in loop_actions.into_iter() {
                match laction {
                    LoopAction::Quit => break 'main,
                    LoopAction::Dummy => (),
                }
            }
        }
        terminal.clear()?;
        terminal.show_cursor()?;
        Ok(())
    }

    fn build_status_line(&self) -> Vec<Text> {
        vec![Text::raw(
            [self.focus.to_string().as_str(), " | ", &self.context.status].concat(),
        )]
    }
}

impl EventProcessor for App {
    fn receive_focus(&mut self) {}
    fn process_event(&mut self, event: Event) -> Vec<Action> {
        if let Event::Key(Key::Ctrl('c')) = event {
            std::process::exit(0);
        }
        match event {
            Event::Key(Key::Ctrl('c')) => std::process::exit(0),
            Event::Key(k) => self.process_ui_event(Event::Key(k)),
            Event::Mouse(k) => self.process_ui_event(Event::Mouse(k)),
            Event::Net(ev) => self.process_net_event(ev),
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
