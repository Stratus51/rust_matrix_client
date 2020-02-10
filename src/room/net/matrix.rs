use crate::event::{self, NetEventKind, NewRoom, Presence};
use crate::net_matrix_dbg as dbg;
use crate::room;
use crate::sequence_number::SequenceNumber;
use ruma_client::{
    api::r0,
    events::{
        room::message::{MessageEventContent, TextMessageEventContent},
        EventType,
    },
};
use ruma_client_api::r0::sync::sync_events::{self, Filter, IncomingResponse, SetPresence};
use ruma_events::collections::all::RoomEvent;
pub use ruma_events::presence::PresenceState as MatrixPresence;
use ruma_events::EventResult;
use ruma_identifiers::RoomId as MatrixRoomId;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Mutex;

// =============================================================================
// Server
// =============================================================================
struct Error {
    id: room::Id,
    error: String,
}

struct ErrorBatch {
    errors: Vec<Error>,
}

impl<S> From<(room::Id, S)> for ErrorBatch
where
    S: AsRef<str>,
{
    fn from((id, s): (room::Id, S)) -> Self {
        ErrorBatch {
            errors: vec![Error {
                id,
                error: s.as_ref().to_string(),
            }],
        }
    }
}

#[derive(Debug)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

pub struct Conf {
    pub url: url::Url,
    pub credentials: Option<Credentials>,
    pub sync_period: u64,
}

pub struct Server {
    id: room::Id,
    // Connection parameters
    conf: Conf,

    // Current connection state
    last_sync: Option<String>,
    client: Option<ruma_client::Client<hyper_tls::HttpsConnector<hyper::client::HttpConnector>>>,

    // Thread handles
    sync_thread_stop: Option<mpsc::Sender<()>>,
    io_thread_stop: Option<mpsc::Sender<()>>,

    // Server room
    input: mpsc::Sender<room::Event>,
    request: mpsc::Receiver<room::net::Action>,
    request_sender: mpsc::Sender<room::net::Action>,

    // Rooms data
    rooms_by_name: HashMap<MatrixRoomId, usize>,
    rooms_by_id: HashMap<usize, MatrixRoomId>,
    room_sn: Arc<Mutex<SequenceNumber>>,
    msg_sn: SequenceNumber,
}

impl Server {
    pub fn new(
        id: room::Id,
        conf: Conf,
        handle: room::ServerHandle,
        self_sender: mpsc::Sender<room::net::Action>,
        room_sn: Arc<Mutex<SequenceNumber>>,
    ) -> Result<Self, String> {
        Ok(Self {
            id,
            conf,

            last_sync: None,
            client: None,

            sync_thread_stop: None,
            io_thread_stop: None,

            input: handle.input,
            request: handle.request,
            request_sender: self_sender,

            rooms_by_name: HashMap::new(),
            rooms_by_id: HashMap::new(),
            room_sn,
            msg_sn: SequenceNumber::default(),
        })
    }

    async fn add_room_name(&mut self, name: &MatrixRoomId) -> Result<room::Id, String> {
        if self.rooms_by_name.get(&name).is_some() {
            return Err(format!("Room '{}' is already opened", name));
        }
        let sn = self.room_sn.lock().await.next().unwrap();
        self.rooms_by_name.insert(name.clone(), sn);
        self.rooms_by_id.insert(sn, name.clone());
        Ok(sn)
    }

    async fn send_as(&mut self, id: room::Id, date: usize, event: NetEventKind) {
        self.input
            .send(event.to_event(id, date, None))
            .await
            .unwrap();
    }

    async fn send_current(&mut self, event: NetEventKind) {
        self.input
            .send(event.to_current_event(self.id.clone(), None))
            .await
            .unwrap();
    }

    async fn send_current_by(&mut self, source: String, event: NetEventKind) {
        self.input
            .send(event.to_current_event(self.id.clone(), Some(source)))
            .await
            .unwrap();
    }

    async fn send_current_as(&mut self, id: room::Id, event: NetEventKind) {
        self.input
            .send(event.to_current_event(id, None))
            .await
            .unwrap();
    }

    async fn send_error(&mut self, error: &str) {
        self.send_current(NetEventKind::Error(error.to_string()))
            .await
    }

    async fn send_error_as(&mut self, id: usize, error: &str) {
        self.send_current_as(id, NetEventKind::Error(error.to_string()))
            .await
    }

    async fn spawn_room(
        &mut self,
        name: &MatrixRoomId,
        alias: Option<String>,
    ) -> Result<(), String> {
        let id = self.add_room_name(name).await?;

        self.send_current(NetEventKind::NewRoom(NewRoom {
            id: Some(id),
            alias: alias.unwrap_or_else(|| name.to_string()),
            requester: self.request_sender.clone(),
        }))
        .await;
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

    fn start_sync_stimuli(&mut self, period: u64) {
        let mut sender = self.request_sender.clone();

        // Save new configuration
        self.conf.sync_period = period;

        // Stop previous sync thread
        self.sync_thread_stop.take();

        // Build new sync thread handle
        let (tx, mut rx) = mpsc::channel(1);
        self.sync_thread_stop = Some(tx);

        // Start stimuli thread
        let period = std::time::Duration::from_millis(period);
        let room = self.id;
        tokio::spawn(async move {
            loop {
                match rx.try_recv() {
                    Ok(_) | Err(mpsc::error::TryRecvError::Closed) => break,
                    _ => (),
                }
                dbg!("sync tick");
                sender
                    .send(room::net::Action {
                        room,
                        action: room::net::ActionKind::Sync,
                    })
                    .await
                    .expect("Main matrix room should not die before sending stop signal.");
                tokio::time::delay_for(period).await;
            }
        });
    }

    async fn process_server_command(&mut self, line: &str) {
        dbg!("process_server_command: {}", line);
        let cmd = line.split(' ').take(1).collect::<Vec<_>>()[0];
        match cmd {
            x => {
                let error = format!("Unsupported command: {}", x);
                self.send_error(&error).await;
            }
        }
    }

    async fn process_server_action(
        &mut self,
        action: room::net::ActionKind,
    ) -> Result<(), ErrorBatch> {
        dbg!("process_server_action");
        match action {
            room::net::ActionKind::Connect => {
                dbg!("connect with {:?}", self.conf.credentials);
                self.client = Some(ruma_client::Client::https(self.conf.url.clone(), None));
                let session_res = match &self.conf.credentials {
                    None => self.client.as_ref().unwrap().register_guest().await,
                    Some(c) => {
                        self.client
                            .as_ref()
                            .unwrap()
                            .log_in(c.username.clone(), c.password.clone(), None, None) // TODO ID management
                            .await
                    }
                };
                match session_res {
                    Ok(s) => {
                        dbg!("Starting sync thread");
                        self.start_sync_stimuli(self.conf.sync_period);
                        Some(s)
                    }
                    Err(e) => {
                        let error =
                            format!("Unable to connect to server '{}': '{:?}'", self.conf.url, e);
                        self.send_error(&error).await;
                        None
                    }
                };
            }
            room::net::ActionKind::Disconnect => {
                dbg!("disconnect");
                self.sync_thread_stop.take();
                self.client.take();
            }
            room::net::ActionKind::Publish(msg) => {
                dbg!("publish");
                self.process_server_command(&msg).await
            }
            room::net::ActionKind::NewRoom(room) => {
                dbg!("new_room");
                let room::net::NewRoom { alias, command } = room;
                let room_id = match MatrixRoomId::try_from(command[0].as_str()) {
                    Ok(id) => id,
                    Err(e) => {
                        return Err(ErrorBatch::from((
                            self.id,
                            format!("Bad matrix room id: {:?}", e),
                        )));
                    }
                };
                self.spawn_room(&room_id, Some(alias)).await.unwrap();
            }
            room::net::ActionKind::Sync => self.sync().await?,
        }
        Ok(())
    }

    async fn process_sub_room_action(
        &mut self,
        action: room::net::Action,
    ) -> Result<(), ErrorBatch> {
        dbg!("process_sub_room_action");
        let room::net::Action { room, action } = action;
        if self.rooms_by_id.get(&room).is_none() {
            return Err(ErrorBatch::from((
                self.id,
                format!("Unknown room {}", room),
            )));
        }
        match action {
            room::net::ActionKind::Connect => {
                dbg!("connect");
                let room_name = self.rooms_by_id.get(&room).cloned().unwrap();
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
                self.send_current_as(
                    room,
                    match res {
                        Ok(_) => NetEventKind::Connected,
                        Err(e) => NetEventKind::Error(format!(
                            "Failed to join room '{}': '{}'",
                            room_name, e
                        )),
                    },
                )
                .await;
            }
            room::net::ActionKind::Disconnect => {
                dbg!("disconnect");
                let room_name = self.rooms_by_id.get(&room).cloned().unwrap();
                let res = self
                    .client
                    .as_ref()
                    .unwrap()
                    .request(r0::membership::leave_room::Request {
                        room_id: room_name.clone(),
                    })
                    .await;
                self.send_current_as(
                    room,
                    match res {
                        Ok(_) => NetEventKind::Disconnected,
                        Err(e) => NetEventKind::Error(format!(
                            "Failed to join room '{}': '{}'",
                            room_name, e
                        )),
                    },
                )
                .await;
            }
            room::net::ActionKind::Publish(msg) => {
                dbg!("publish");
                match self
                    .client
                    .as_ref()
                    .unwrap()
                    .request(r0::message::create_message_event::Request {
                        room_id: self.rooms_by_id.get(&room).unwrap().clone(),
                        event_type: EventType::RoomMessage,
                        txn_id: self.msg_sn.next().unwrap().to_string(),
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
                    Err(e) => return Err(ErrorBatch::from((room, e.to_string()))),
                };
            }
            room::net::ActionKind::NewRoom(_) => {
                return Err(ErrorBatch::from((
                    room,
                    "How could a matrix room generate another chat room?".to_string(),
                )))
            }
            room::net::ActionKind::Sync => {
                return Err(ErrorBatch::from((
                    room,
                    "A matrix room cannot sync by itself",
                )))
            }
        }
        Ok(())
    }

    async fn sync_request(
        &self,
        filter: Option<Filter>,
        since: Option<String>,
        set_presence: bool,
    ) -> Result<Option<IncomingResponse>, ErrorBatch> {
        if self.client.is_none() {
            return Ok(None);
        }
        let client = self.client.as_ref().unwrap().clone();
        let filter = filter.clone();

        let res = client
            .request(sync_events::Request {
                filter,
                since,
                full_state: None,
                set_presence: if set_presence {
                    None
                } else {
                    Some(SetPresence::Offline)
                },
                timeout: None,
            })
            .await;

        match res {
            Ok(response) => Ok(Some(response)),
            Err(e) => {
                let error = format!("{:?}", e);
                Err(ErrorBatch::from((self.id, error)))
            }
        }
    }

    async fn sync(&mut self) -> Result<(), ErrorBatch> {
        dbg!("sync");
        if self.client.is_none() {
            return Ok(());
        }
        let mut errors = vec![];
        let resp = self
            .sync_request(None, self.last_sync.clone(), true)
            .await?;
        if resp.is_none() {
            dbg!("empty sync");
            return Ok(());
        }
        let resp = resp.unwrap();
        dbg!("sync resp!");
        for (name, _) in resp.rooms.leave.iter() {
            dbg!("{} room left", name);
            let id = self.rooms_by_name.get(name).copied();
            if let Some(id) = id {
                self.send_current_as(id, NetEventKind::Disconnected).await
            }
        }
        for (name, data) in resp.rooms.join.iter() {
            dbg!("{} room joined", name);
            if self.rooms_by_name.get(name).is_none() {
                self.spawn_room(name, None).await.unwrap();
            }
            let id = self.rooms_by_name.get(name).copied().unwrap();
            self.send_current_as(id, NetEventKind::Connected).await;
            for e in data.timeline.events.iter() {
                match e {
                    EventResult::Ok(e) => match e {
                        RoomEvent::RoomMessage(m) => {
                            let content = match m.content.clone() {
                                MessageEventContent::Text(c) => c.body,
                                x => format!("Unsupported message: {:?}", x),
                            };
                            dbg!("Send msg as {}", id);
                            self.send_as(
                                id,
                                u64::try_from(m.origin_server_ts).expect("Date should fit in a u64")
                                    as usize,
                                NetEventKind::Message(event::Message { content }),
                            )
                            .await
                        }
                        x => errors.push(Error {
                            id,
                            error: format!("Unmanaged room event type: {:?}", x),
                        }),
                    },
                    EventResult::Err(e) => errors.push(Error {
                        id,
                        error: format!("room timeline error: {:?}", e),
                    }),
                }
            }
        }
        for (name, _) in resp.rooms.invite.iter() {
            dbg!("{} room invitation", name);
            let id = self.rooms_by_name.get(name).copied();
            if let Some(id) = id {
                self.send_current_as(id, NetEventKind::Invite).await;
            }
        }
        for presence in resp.presence.events.iter() {
            dbg!("Presence: {:?}", presence);
            match presence {
                // TODO Distribute event to the correct rooms
                EventResult::Ok(p) => {
                    self.send_current_by(
                        p.sender.to_string(),
                        NetEventKind::Presence(Presence {
                            id: p.sender.to_string(),
                            display_name: p.content.displayname.clone(),
                            active: p.content.currently_active,
                            status_msg: p.content.status_msg.clone(),
                            presence: p.content.presence,
                        }),
                    )
                    .await
                }
                EventResult::Err(err) => errors.push(Error {
                    id: self.id,
                    error: err.to_string(),
                }),
            }
        }
        self.last_sync = Some(resp.next_batch);

        if errors.is_empty() {
            Ok(())
        } else {
            Err(ErrorBatch { errors })
        }
    }

    pub async fn start(mut self) {
        // TODO Watch out for disconnections (trigger reconnect)
        dbg!("Starting matrix thread");
        while let Some(action) = self.request.recv().await {
            dbg!("ev!");
            let res = if action.room == self.id {
                self.process_server_action(action.action).await
            } else {
                self.process_sub_room_action(action).await
            };
            if let Err(e) = res {
                for e in e.errors.iter() {
                    if e.id == self.id {
                        self.send_error(&e.error).await;
                    } else {
                        self.send_error_as(e.id, &e.error).await;
                    }
                }
            }
        }

        dbg!("Stopping matrix thread");
        self.stop();
    }

    fn stop(&mut self) {
        // TODO Unnecessary
        self.sync_thread_stop.take();
        self.io_thread_stop.take();
    }
}
