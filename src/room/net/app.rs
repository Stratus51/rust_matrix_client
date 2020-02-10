use crate::event::{self, Event, Message, NetEventKind};
use crate::room::{
    self,
    net::{Action, ActionKind},
};
use crate::sequence_number::SequenceNumber;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Mutex;

pub struct App {
    id: room::Id,
    handle: room::ServerHandle,
    room_sn: Arc<Mutex<SequenceNumber>>,
}

impl App {
    pub fn new(
        id: room::Id,
        handle: room::ServerHandle,
        room_sn: Arc<Mutex<SequenceNumber>>,
    ) -> Self {
        Self {
            id,
            handle,
            room_sn,
        }
    }
}

impl App {
    async fn send(&mut self, event: Event) {
        self.handle.input.send(event).await.unwrap()
    }

    async fn send_current(&mut self, event: NetEventKind) {
        self.send(event.to_current_event(self.id.clone(), None))
            .await;
    }

    async fn send_current_by_me(&mut self, event: NetEventKind) {
        self.send(event.to_current_event(self.id.clone(), Some("Me".to_string())))
            .await;
    }

    async fn send_error(&mut self, error: &str) {
        self.send_current(NetEventKind::Error(error.to_string()))
            .await
    }

    pub fn start(mut self) {
        tokio::spawn(async move {
            while let Some(action) = self.handle.request.recv().await {
                let Action { action, .. } = action;
                match action {
                    ActionKind::Connect => {
                        self.send_error("Cannot connect to the main room (it is a local room)")
                            .await
                    }
                    ActionKind::Disconnect => {
                        self.send_error("Cannot disconnect from main room (it is a local room)")
                            .await
                    }
                    ActionKind::Publish(packet) => {
                        self.send_current_by_me(NetEventKind::Message(Message { content: packet }))
                            .await
                    }
                    ActionKind::NewRoom(room) => match self.spawn(room).await {
                        Ok(room) => self.send_current(NetEventKind::NewRoom(room)).await,
                        Err(e) => {
                            let error = format!("{:?}", e);
                            self.send_error(&error).await
                        }
                    },
                    ActionKind::Sync => {
                        self.send_error(
                            "Thou shall stop bothering local residents with syncing matter",
                        )
                        .await
                    }
                }
            }
        });
    }

    async fn spawn(&mut self, room: room::net::NewRoom) -> Result<event::NewRoom, String> {
        let room::net::NewRoom { alias, command } = room;
        let mut tokens = command;
        if tokens.is_empty() {
            return Err("No server type specified! Syntax: <server_type> [...args]".to_string());
        }
        let s_type = tokens.remove(0);
        match s_type.as_str() {
            "matrix" => {
                if tokens.is_empty() {
                    return Err(
                        "Bad syntax. Syntax: matrix <url> [username [password]]".to_string()
                    );
                }
                let credentials = if tokens.len() >= 2 {
                    let username = tokens[1].to_string();
                    let password = if tokens.len() >= 3 {
                        tokens[2].as_str()
                    } else {
                        ""
                    }
                    .to_string();
                    Some(super::matrix::Credentials { username, password })
                } else {
                    None
                };
                let id = self.room_sn.lock().await.next().unwrap();
                let (mut room_tx, room_rx) = mpsc::channel(100);
                let server = super::matrix::Server::new(
                    id,
                    super::matrix::Conf {
                        url: tokens[0]
                            .to_string()
                            .parse()
                            .map_err(|e| format!("{}", e))?,
                        sync_period: 8024,
                        credentials,
                    },
                    room::ServerHandle {
                        input: self.handle.input.clone(),
                        request: room_rx,
                        request_sn: SequenceNumber::default(),
                    },
                    room_tx.clone(),
                    self.room_sn.clone(),
                );
                let server = match server {
                    Ok(s) => s,
                    Err(e) => return Err(format!("Matrix server creation failed: {}", e)),
                };
                tokio::spawn(server.start());
                match room_tx
                    .send(room::net::Action {
                        room: id,
                        action: room::net::ActionKind::Connect,
                    })
                    .await
                {
                    Ok(_) => (),
                    Err(e) => eprintln!("Matrix start send failed: {:?}", e),
                };
                Ok(event::NewRoom {
                    id: Some(id),
                    alias,
                    requester: room_tx,
                })
            }
            s_type => Err(format!("Unknown server type '{}'", s_type)),
        }
    }
}
