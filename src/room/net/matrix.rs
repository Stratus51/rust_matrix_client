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
use ruma_identifiers::RoomId as MatrixRoomId;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::sync::mpsc;

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
    // Connection parameters
    conf: Conf,

    // Current connection state
    connected: bool,

    // Thread handles
    sync_thread_stop: Option<mpsc::Sender<()>>,
    io_thread_stop: Option<mpsc::Sender<()>>,

    // Server room
    input: mpsc::Sender<room::Event>,
    request: Option<mpsc::Receiver<room::Request>>,
    input_notification: mpsc::Sender<crate::event::Event>,

    // Rooms data
    rooms: HashMap<room::Id, room::ServerHandle>,
    rooms_by_name: HashMap<MatrixRoomId, room::Id>,
    rooms_by_id: HashMap<room::Id, MatrixRoomId>,
    room_sn: SequenceNumber,
}

enum InternRequest {
    Sync,
    Room(room::Request),
}

#[derive(Debug)]
pub enum Error {
    BadUrl,
}

impl Server {
    pub fn new(
        conf: Conf,
        handle: room::ServerHandle,
        input_notification: mpsc::Sender<crate::event::Event>,
    ) -> Result<Self, Error> {
        // match url.parse() {
        //     Ok(_) => (),
        //     Err(_) => return Err(Error::BadUrl),
        // };
        Ok(Self {
            conf,

            connected: false,
            sync_thread_stop: None,
            io_thread_stop: None,

            input: handle.input,
            request: Some(handle.request),
            input_notification,

            rooms: HashMap::new(),
            rooms_by_name: HashMap::new(),
            rooms_by_id: HashMap::new(),
            room_sn: SequenceNumber::new(),
        })
    }

    fn add_room_name(&mut self, name: &room::StringId) -> Result<room::Id, String> {
        let room_id = match MatrixRoomId::try_from(name.as_str()) {
            Ok(id) => id,
            Err(e) => return Err(format!("Bad room ID '{}': {}", name, e)),
        };
        if self.rooms_by_name.get(&room_id).is_some() {
            return Err(format!("Room '{}' is already opened", name));
        }
        let sn = self.room_sn.next();
        self.rooms_by_name.insert(room_id.clone(), sn);
        self.rooms_by_id.insert(sn, room_id);
        Ok(sn)
    }

    fn remove_room(&mut self, id: &room::Id) -> Option<MatrixRoomId> {
        match self.rooms_by_id.remove(id) {
            Some(room_name) => {
                self.rooms_by_name.remove(&room_name);
                Some(room_name)
            }
            None => None,
        }
    }

    fn start_io_stimuli(&mut self, sender: mpsc::Sender<InternRequest>) {
        let receiver = self.request.take().unwrap();

        // Stop previous io thread
        self.io_thread_stop.take();

        // Build new io thread handle
        let (tx, rx) = mpsc::channel();
        self.io_thread_stop = Some(tx);

        // Start stimuli thread
        std::thread::spawn(move || loop {
            while let Ok(request) = receiver.recv() {
                sender.send(InternRequest::Room(request));

                // TODO This delays the garbage collection of this thread to the next I/O input
                // (which may never happen)
                match rx.try_recv() {
                    Ok(_) | Err(mpsc::TryRecvError::Disconnected) => break,
                    _ => (),
                }
            }
        });
    }

    fn start_sync_stimuli(&mut self, period: u64, sender: mpsc::Sender<InternRequest>) {
        // Save new configuration
        self.conf.sync_period = period;

        // Stop previous sync thread
        self.sync_thread_stop.take();

        // Build new sync thread handle
        let (tx, rx) = mpsc::channel();
        self.sync_thread_stop = Some(tx);

        // Start stimuli thread
        let period = std::time::Duration::from_millis(period);
        std::thread::spawn(move || loop {
            std::thread::sleep(period);
            match rx.try_recv() {
                Ok(_) | Err(mpsc::TryRecvError::Disconnected) => break,
                _ => (),
            }
        });
    }

    async fn start(&mut self) {
        let client = ruma_client::Client::new(self.conf.url.parse().unwrap(), None);

        // Start stimuli threads
        let (tx, rx) = mpsc::channel();
        self.start_sync_stimuli(self.conf.sync_period, tx.clone());
        self.start_io_stimuli(tx.clone());

        // Start user watching loop
        let mut session = None;
        let mut last_sync = None;

        // TODO Watch out for disconnections (trigger reconnect)
        loop {
            let ev = match rx.recv() {
                Ok(ev) => ev,
                Err(_) => break,
            };

            match ev {
                InternRequest::Sync => {
                    if session.is_some() {
                        let mut sync_stream = Box::pin(client.sync(None, None, true));
                        loop {
                            match sync_stream.try_next().await {
                                Ok(opt) => match opt {
                                    None => break,
                                    Some(resp) => {
                                        last_sync = Some(resp.next_batch);
                                    }
                                },
                                Err(e) => panic!("ERROR: {:#?}", e),
                            }
                        }
                        self.input_notification
                            .send(crate::event::Event::Notification)
                            .unwrap();
                    }
                }
                InternRequest::Room(req) => match req {
                    room::Request::Connect(id) => match id {
                        ServerRoomId => {
                            let session_res = match &self.conf.credentials {
                                None => client.register_guest().await,
                                Some(c) => {
                                    client
                                        .log_in(c.username.clone(), c.password.clone(), None)
                                        .await
                                }
                            };
                            session = match session_res {
                                Ok(s) => Some(s),
                                Err(e) => {
                                    self.rooms.get(&ServerRoomId).unwrap().input.send(
                                        room::Event::Error(format!(
                                            "Unable to connect to server '{}': '{}'",
                                            self.conf.url, e
                                        )),
                                    );
                                    None
                                }
                            }
                        }
                        id => {
                            let room_name = self.rooms_by_id.get(&id).unwrap();
                            let res = client
                                .request(r0::membership::join_room_by_id::Request {
                                    room_id: room_name.clone(),
                                    // TODO
                                    third_party_signed: None,
                                })
                                .await;
                            self.rooms
                                .get(&id)
                                .unwrap()
                                .input
                                .send(match res {
                                    Ok(_) => room::Event::Connected,
                                    Err(e) => room::Event::Error(format!(
                                        "Failed to join room '{}': '{}'",
                                        room_name, e
                                    )),
                                })
                                .unwrap();
                        }
                    },
                    room::Request::Disconnect(id) => match id {
                        ServerRoomId => session = None,
                        id => {
                            let room_name = self.rooms_by_id.get(&id).unwrap();
                            let res = client
                                .request(r0::membership::leave_room::Request {
                                    room_id: room_name.clone(),
                                })
                                .await;
                            self.rooms
                                .get(&id)
                                .unwrap()
                                .input
                                .send(match res {
                                    Ok(_) => room::Event::Connected,
                                    Err(e) => room::Event::Error(format!(
                                        "Failed to leave to room '{}': '{}'",
                                        room_name, e
                                    )),
                                })
                                .unwrap();
                        }
                    },
                    room::Request::Message(req) => match req.room {
                        ServerRoomId => panic!("Undefined behavior for server room publish action"),
                        id => match self.rooms.get_mut(&id) {
                            Some(room) => {
                                match client
                                    .request(r0::send::send_message_event::Request {
                                        room_id: self.rooms_by_id.get(&id).unwrap().clone(),
                                        event_type: EventType::RoomMessage,
                                        txn_id: room.request_id().to_string(),
                                        data: MessageEventContent::Text(TextMessageEventContent {
                                            body: req.msg,
                                            // TODO
                                            format: None,
                                            formatted_body: None,
                                            relates_to: None,
                                        }),
                                    })
                                    .await
                                {
                                    Ok(_) => (),
                                    Err(e) => {
                                        room.input.send(room::Event::Error(e.to_string())).unwrap()
                                    }
                                };
                            }
                            None => panic!("Unknown room with ID {}.", id),
                        },
                    },
                },
            }
        }

        self.stop();
    }

    fn stop(&mut self) {
        self.sync_thread_stop.take();
    }
}
