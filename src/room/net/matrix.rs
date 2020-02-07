use crate::event::{Event, NetEvent, NetEventKind, NewRoom, Presence};
use crate::room;
use crate::sequence_number::SequenceNumber;
use futures_util::stream::TryStreamExt as _;
use ruma_client::{
    api::r0,
    events::{
        room::message::{MessageEventContent, TextMessageEventContent},
        EventType,
    },
};
use ruma_events::collections::only::Event as MatrixEvent;
pub use ruma_events::presence::PresenceState as MatrixPresence;
use ruma_events::EventResult;
use ruma_identifiers::RoomId as MatrixRoomId;
use std::collections::HashMap;
use std::convert::TryFrom;
use tokio::sync::mpsc;

// =============================================================================
// Server
// =============================================================================
pub struct Credentials {
    pub username: String,
    pub password: String,
}

pub struct Conf {
    pub url: String,
    pub credentials: Option<Credentials>,
    pub sync_period: u64,
}

pub struct Server {
    id: room::Id,
    // Connection parameters
    conf: Conf,

    // Current connection state
    last_sync: Option<String>,
    client: Option<ruma_client::Client<hyper::client::HttpConnector>>,

    // Thread handles
    sync_thread_stop: Option<mpsc::Sender<()>>,
    io_thread_stop: Option<mpsc::Sender<()>>,

    // Server room
    input: mpsc::Sender<room::Event>,
    request: Option<mpsc::Receiver<room::net::Action>>,
    request_sender: mpsc::Sender<room::net::Action>,

    // Rooms data
    rooms_by_name: HashMap<MatrixRoomId, usize>,
    rooms_by_id: HashMap<usize, MatrixRoomId>,
    room_sn: SequenceNumber,
    msg_sn: SequenceNumber,
}

#[derive(Debug)]
enum InternRequest {
    Sync,
    Room(room::net::Action),
}

#[derive(Debug)]
pub enum Error {
    BadUrl,
}

impl Server {
    pub fn new(
        id: room::Id,
        conf: Conf,
        handle: room::ServerHandle,
        self_sender: mpsc::Sender<room::net::Action>,
    ) -> Result<Self, Error> {
        // match url.parse() {
        //     Ok(_) => (),
        //     Err(_) => return Err(Error::BadUrl),
        // };
        Ok(Self {
            id,
            conf,

            last_sync: None,
            client: None,

            sync_thread_stop: None,
            io_thread_stop: None,

            input: handle.input,
            request: Some(handle.request),
            request_sender: self_sender,

            rooms_by_name: HashMap::new(),
            rooms_by_id: HashMap::new(),
            room_sn: SequenceNumber::new(),
            msg_sn: SequenceNumber::new(),
        })
    }

    fn add_room_name(&mut self, name: &MatrixRoomId) -> Result<room::Id, String> {
        if self.rooms_by_name.get(&name).is_some() {
            return Err(format!("Room '{}' is already opened", name));
        }
        let sn = self.room_sn.next();
        self.rooms_by_name.insert(name.clone(), sn);
        self.rooms_by_id.insert(sn, name.clone());
        Ok(self.new_room_id(sn))
    }

    fn new_room_id(&self, id: usize) -> room::Id {
        [&self.id[..], &[id]].concat()
    }

    fn get_room_id(&self, name: &MatrixRoomId) -> Option<room::Id> {
        let id = self.rooms_by_name.get(name)?;
        Some([&self.id[..], &[*id]].concat())
    }

    async fn spawn_room(
        &mut self,
        name: &MatrixRoomId,
        alias: Option<String>,
    ) -> Result<(), String> {
        let id = self.add_room_name(name)?;

        self.input
            .send(Event::Net(NetEvent {
                room: self.id.clone(),
                event: NetEventKind::NewRoom(NewRoom {
                    id: Some(id),
                    alias: alias.unwrap_or_else(|| name.to_string()),
                    requester: self.request_sender.clone(),
                }),
            }))
            .await
            .unwrap();
        Ok(())
    }

    fn remove_room(&mut self, id: usize) -> Option<MatrixRoomId> {
        match self.rooms_by_id.remove(&id) {
            Some(room_name) => {
                self.rooms_by_name.remove(&room_name);
                Some(room_name)
            }
            None => None,
        }
    }

    fn start_io_stimuli(&mut self, mut sender: mpsc::Sender<InternRequest>) {
        let mut receiver = self.request.take().unwrap();

        // Stop previous io thread
        self.io_thread_stop.take();

        // Build new io thread handle
        let (tx, mut rx) = mpsc::channel(1);
        self.io_thread_stop = Some(tx);

        // Start stimuli thread
        tokio::spawn(async move {
            loop {
                while let Some(request) = receiver.recv().await {
                    sender
                        .send(InternRequest::Room(request))
                        .await
                        .expect("Main app should not quit while rooms are open");

                    // TODO This delays the garbage collection of this thread to the next I/O input
                    // (which may never happen)
                    match rx.try_recv() {
                        Ok(_) | Err(mpsc::error::TryRecvError::Closed) => break,
                        _ => (),
                    }
                }
            }
        });
    }

    fn start_sync_stimuli(&mut self, period: u64, mut sender: mpsc::Sender<InternRequest>) {
        // Save new configuration
        self.conf.sync_period = period;

        // Stop previous sync thread
        self.sync_thread_stop.take();

        // Build new sync thread handle
        let (tx, mut rx) = mpsc::channel(1);
        self.sync_thread_stop = Some(tx);

        // Start stimuli thread
        let period = std::time::Duration::from_millis(period);
        tokio::spawn(async move {
            loop {
                std::thread::sleep(period);
                match rx.try_recv() {
                    Ok(_) | Err(mpsc::error::TryRecvError::Closed) => break,
                    _ => (),
                }
                sender
                    .send(InternRequest::Sync)
                    .await
                    .expect("Main matrix room should not die before sending stop signal.");
            }
        });
    }

    pub async fn process_server_command(&mut self, line: &str) {
        let cmd = line.split(' ').take(1).collect::<Vec<_>>()[0];
        match cmd {
            x => {
                let error = format!("Unsupported command: {}", x);
                self.input
                    .send(NetEvent::to_event(
                        self.id.clone(),
                        NetEventKind::Error(error),
                    ))
                    .await
                    .unwrap();
            }
        }
    }

    pub async fn process_server_action(&mut self, action: room::net::ActionKind) {
        match action {
            room::net::ActionKind::Connect => {
                self.client = Some(ruma_client::Client::new(
                    self.conf.url.parse().unwrap(),
                    None,
                ));
                let session_res = match &self.conf.credentials {
                    None => self.client.as_ref().unwrap().register_guest().await,
                    Some(c) => {
                        self.client
                            .as_ref()
                            .unwrap()
                            .log_in(c.username.clone(), c.password.clone(), None)
                            .await
                    }
                };
                match session_res {
                    Ok(s) => Some(s),
                    Err(e) => {
                        let error =
                            format!("Unable to connect to server '{}': '{}'", self.conf.url, e);
                        self.input
                            .send(NetEvent::to_event(
                                self.id.clone(),
                                NetEventKind::Error(error),
                            ))
                            .await
                            .unwrap();
                        None
                    }
                };
            }
            room::net::ActionKind::Disconnect => {
                self.client.take();
            }
            room::net::ActionKind::Publish(msg) => self.process_server_command(&msg).await,
            room::net::ActionKind::NewRoom(room) => {
                let room::net::NewRoom { alias, command } = room;
                let words: Vec<_> = command.split(' ').collect();
                let room_id = match MatrixRoomId::try_from(words[0]) {
                    Ok(id) => id,
                    Err(e) => {
                        let error = format!("Bad matrix room id: {:?}", e);
                        self.input
                            .send(NetEvent::to_event(
                                self.id.clone(),
                                NetEventKind::Error(error),
                            ))
                            .await
                            .unwrap();
                        return;
                    }
                };
                self.spawn_room(&room_id, Some(alias)).await.unwrap();
            }
        }
    }

    pub async fn process_sub_room_action(&mut self, action: room::net::Action) {
        let room::net::Action { mut room, action } = action;
        if room.len() > 1 {
            todo!();
        }
        let id = room.pop().unwrap();
        match action {
            room::net::ActionKind::Connect => {
                let id = room[room.len() - 1];
                let room_name = self.rooms_by_id.get(&id).unwrap();
                let res = self
                    .client
                    .as_ref()
                    .unwrap()
                    .request(r0::membership::join_room_by_id::Request {
                        room_id: room_name.clone(),
                        // TODO
                        third_party_signed: None,
                    })
                    .await;
                self.input
                    .send(NetEvent::to_event(
                        room,
                        match res {
                            Ok(_) => NetEventKind::Connected,
                            Err(e) => NetEventKind::Error(format!(
                                "Failed to join room '{}': '{}'",
                                room_name, e
                            )),
                        },
                    ))
                    .await
                    .unwrap();
            }
            room::net::ActionKind::Disconnect => {
                let room_name = self.rooms_by_id.get(&id).unwrap();
                let res = self
                    .client
                    .as_ref()
                    .unwrap()
                    .request(r0::membership::leave_room::Request {
                        room_id: room_name.clone(),
                    })
                    .await;
                self.input
                    .send(NetEvent::to_event(
                        room,
                        match res {
                            Ok(_) => NetEventKind::Disconnected,
                            Err(e) => NetEventKind::Error(format!(
                                "Failed to join room '{}': '{}'",
                                room_name, e
                            )),
                        },
                    ))
                    .await
                    .unwrap();
            }
            room::net::ActionKind::Publish(msg) => {
                match self
                    .client
                    .as_ref()
                    .unwrap()
                    .request(r0::send::send_message_event::Request {
                        room_id: self.rooms_by_id.get(&id).unwrap().clone(),
                        event_type: EventType::RoomMessage,
                        txn_id: self.msg_sn.next().to_string(),
                        data: MessageEventContent::Text(TextMessageEventContent {
                            body: msg,
                            // TODO
                            format: None,
                            formatted_body: None,
                            relates_to: None,
                        }),
                    })
                    .await
                {
                    Ok(_) => (),
                    Err(e) => self
                        .input
                        .send(NetEvent::to_event(room, NetEventKind::Error(e.to_string())))
                        .await
                        .unwrap(),
                };
            }
            room::net::ActionKind::NewRoom(_) => {
                panic!("How could a matrix room generate another chat room?".to_string())
            }
        }
    }

    pub async fn sync(&mut self) {
        if self.client.is_some() {
            let mut sync_stream = Box::pin(self.client.as_ref().unwrap().sync(
                None,
                self.last_sync.clone(),
                true,
            ));
            loop {
                match sync_stream.try_next().await {
                    Ok(opt) => match opt {
                        None => break,
                        Some(resp) => {
                            for (name, _) in resp.rooms.leave.iter() {
                                if let Some(id) = self.get_room_id(name) {
                                    self.input
                                        .send(NetEvent::to_event(id, NetEventKind::Disconnected))
                                        .await
                                        .unwrap()
                                }
                            }
                            for (name, _) in resp.rooms.join.iter() {
                                if self.rooms_by_name.get(name).is_none() {
                                    self.spawn_room(name, None).await.unwrap();
                                }
                                let id = self.get_room_id(name).unwrap();
                                self.input
                                    .send(NetEvent::to_event(id, NetEventKind::Connected))
                                    .await
                                    .unwrap();
                            }
                            for (name, _) in resp.rooms.invite.iter() {
                                if let Some(id) = self.get_room_id(name) {
                                    self.input
                                        .send(NetEvent::to_event(id, NetEventKind::Invite))
                                        .await
                                        .unwrap();
                                }
                            }
                            for presence in resp.presence.events.iter() {
                                match presence {
                                    EventResult::Ok(ev) => match ev {
                                        // TODO Distribute event to the correct rooms
                                        MatrixEvent::Presence(p) => self
                                            .input
                                            .send(NetEvent::to_event(
                                                self.id.clone(),
                                                NetEventKind::Presence(Presence {
                                                    id: p.sender.to_string(),
                                                    display_name: p.content.displayname.clone(),
                                                    active: p.content.currently_active,
                                                    status_msg: p.content.status_msg.clone(),
                                                    presence: p.content.presence,
                                                }),
                                            ))
                                            .await
                                            .unwrap(),
                                        x => {
                                            let err = format!("Unmanaged event type: {:?}", x);
                                            self.input
                                                .send(NetEvent::to_event(
                                                    self.id.clone(),
                                                    NetEventKind::Error(err),
                                                ))
                                                .await
                                                .unwrap()
                                        }
                                    },
                                    EventResult::Err(err) => self
                                        .input
                                        .send(NetEvent::to_event(
                                            self.id.clone(),
                                            NetEventKind::Error(err.message()),
                                        ))
                                        .await
                                        .unwrap(),
                                }
                            }
                            self.last_sync = Some(resp.next_batch);
                        }
                    },
                    Err(e) => panic!("ERROR: {:#?}", e),
                }
            }
        }
    }

    pub async fn start(mut self) {
        // Start stimuli threads
        let (tx, mut rx) = mpsc::channel(100);
        self.start_sync_stimuli(self.conf.sync_period, tx.clone());
        self.start_io_stimuli(tx.clone());

        // TODO Watch out for disconnections (trigger reconnect)
        while let Some(ev) = rx.recv().await {
            match ev {
                InternRequest::Sync => self.sync().await,
                InternRequest::Room(act) => {
                    if act.room.is_empty() {
                        self.process_server_action(act.action).await
                    } else if act.room.len() == 1 {
                        self.process_sub_room_action(act).await
                    } else {
                        todo!()
                    }
                }
            }
        }

        self.stop();
    }

    fn stop(&mut self) {
        self.sync_thread_stop.take();
    }
}
