use crate::event::{
    Action, AppAction, CommandAction, Event, EventProcessor, InputAction, Key, NetEvent,
    NetEventKind,
};
use crate::input::{command::Command, Input};
use crate::room;
use crate::widget::Height;
use std::collections::HashMap;
use std::io;
use termion::raw::IntoRawMode;
use tokio::sync::mpsc;
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

pub struct RoomTree {
    ui: room::ui::Room,
    net_sender: mpsc::Sender<room::net::Action>,
    children: HashMap<usize, RoomTree>,
}

impl RoomTree {
    fn new(
        id: room::Id,
        room_conf: room::ui::Conf,
        net_sender: mpsc::Sender<room::net::Action>,
    ) -> Self {
        Self {
            ui: room::ui::Room::new(id, room_conf),
            net_sender,
            children: HashMap::new(),
        }
    }

    fn to_string_list(&self, cursor: Option<&[usize]>) -> (Vec<String>, Option<usize>) {
        let cursor = cursor.filter(|c| !c.is_empty());
        let mut ret = vec![];
        ret.push(self.ui.conf.alias.clone());

        let (cursor_index, node_cursor) = match cursor {
            Some(list) => (Some(list[0]), &list[1..]),
            None => (None, &[] as &[usize]),
        };
        let mut final_cursor = None;
        for (name, node) in self.children.iter() {
            let node_ret = if let Some(index) = cursor_index {
                if *name == index {
                    let (node_ret, c_i) = node.to_string_list(Some(node_cursor));
                    final_cursor = c_i.map(|index| index + ret.len());
                    node_ret.iter().map(|s| [" ", &s].concat()).collect()
                } else {
                    node.to_string_list(None).0
                }
            } else {
                node.to_string_list(None).0
            };

            ret.extend_from_slice(&node_ret[..]);
        }
        (ret, final_cursor)
    }
}

enum LoopAction {
    Quit,
    Dummy, // XXX Just for clippy to stop complaining
}

pub struct App {
    options: Options,

    context: Context,
    room_tree: RoomTree,
    current_room: room::Id,

    input: Input,
    command: Command,
    focus: Focus,

    receiver: mpsc::Receiver<Event>,
    pub sender: mpsc::Sender<Event>,
}

impl App {
    pub fn new(options: Options) -> Self {
        let (sender, receiver) = mpsc::channel(100);
        Self {
            options,
            context: Context::new(),
            room_tree: Self::root_room_tree(sender.clone()),
            current_room: vec![],
            input: Input::default(),
            command: Command::default(),
            focus: Focus::None,
            receiver,
            sender,
        }
    }

    fn root_room_tree(sender: mpsc::Sender<Event>) -> RoomTree {
        let (tx, rx) = mpsc::channel(10);
        let app = room::net::app::App::new(vec![], room::ServerHandle::new(sender, rx));
        app.start();

        RoomTree::new(
            vec![],
            room::ui::Conf {
                alias: "main".to_string(),
                meta_width: 8,
            },
            tx,
        )
    }

    fn get_room(&self, id: &[usize]) -> Option<&RoomTree> {
        let mut node = &self.room_tree;
        for i in id.iter() {
            // TODO manage error paths
            node = node.children.get(i)?;
        }
        Some(node)
    }

    fn get_mut_room(&mut self, id: &[usize]) -> Option<&mut RoomTree> {
        let mut node = &mut self.room_tree;
        for i in id.iter() {
            // TODO manage error paths
            node = node.children.get_mut(i)?;
        }
        Some(node)
    }

    fn add_room(&mut self, id: room::Id, name: String, requester: mpsc::Sender<room::net::Action>) {
        let child_id = id[id.len() - 1];
        let parent_id = id[..id.len() - 1].to_vec();
        self.get_mut_room(&parent_id)
            .unwrap() // TODO Manage this with more tolerance?
            .children
            .insert(
                child_id,
                RoomTree::new(
                    id,
                    room::ui::Conf {
                        alias: name,
                        meta_width: 8,
                    },
                    requester,
                ),
            );
    }

    fn room(&mut self) -> &RoomTree {
        self.get_room(&self.current_room).unwrap()
    }

    fn mut_room(&mut self) -> &mut RoomTree {
        self.get_mut_room(&self.current_room.clone()).unwrap()
    }

    async fn room_send(&mut self, action: room::net::ActionKind) {
        let action = room::net::Action {
            room: self.current_room.clone(),
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
        match event.event {
            NetEventKind::Connected => todo!(),
            NetEventKind::Disconnected => todo!(),
            NetEventKind::Invite => todo!(),
            NetEventKind::Message(_) => todo!(),
            NetEventKind::NewRoom(r) => {
                self.add_room(r.id.unwrap(), r.alias, r.requester);
                vec![]
            }
            NetEventKind::Presence(_) => todo!(),
            NetEventKind::Error(_) => todo!(),
            NetEventKind::Unknown(_) => todo!(),
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
                        // self.command.receive_focus();
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

    pub async fn run(&mut self) -> Result<(), Error> {
        // Initialization ------------------------------------------------------
        let stdout = io::stdout().into_raw_mode()?;
        let backend = TermionBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        terminal.clear()?;
        terminal.hide_cursor()?;

        'main: loop {
            eprintln!("plip plop draw");
            // UI refresh -------------------------------------------------------
            terminal.draw(|mut f| {
                eprintln!("termion size");
                let (t_w, _t_h) = match termion::terminal_size() {
                    Ok((w, h)) => (w, h),
                    Err(e) => panic!("{:#?}", e),
                };
                eprintln!("wanted size");
                let mut input_size = self.input.height(t_w);
                if input_size > self.options.max_input_height as usize {
                    input_size = self.options.max_input_height as usize;
                }
                eprintln!("input size: {}", input_size);

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

                let (room_list, room_cursor) =
                    self.room_tree.to_string_list(Some(&self.current_room[..]));

                SelectableList::default()
                    .block(Block::default().title("Room list").borders(Borders::ALL))
                    .items(&room_list)
                    .select(room_cursor)
                    .style(Style::default().fg(Color::White))
                    .highlight_style(Style::default().modifier(Modifier::ITALIC).bg(Color::Blue))
                    .render(&mut f, content_layout[0]);

                let mut block = Block::default().title("Room").borders(Borders::ALL);
                block.render(&mut f, content_layout[1]);
                let room_space = block.inner(content_layout[1]);
                eprintln!("print rooms");
                self.mut_room().ui.render(&mut f, room_space);

                eprintln!("print input");
                self.input.render(&mut f, main_layout[1]);
                Paragraph::new(self.build_status_line().iter()).render(&mut f, main_layout[2]);

                if let Focus::Command = self.focus {
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
