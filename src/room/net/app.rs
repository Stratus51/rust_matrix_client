use super::Error;
use crate::event::{self, Event, Message, NetEvent, NetEventKind};
use crate::room::{
    self,
    net::{Action, ActionKind},
};
use crate::sequence_number::SequenceNumber;

pub struct App {
    id: room::Id,
    handle: room::ServerHandle,
    room_sn: SequenceNumber,
}

impl App {
    pub fn new(id: room::Id, handle: room::ServerHandle) -> Self {
        Self {
            id,
            handle,
            room_sn: SequenceNumber::new(),
        }
    }
}

impl App {
    async fn send(&mut self, event: Event) {
        self.handle.input.send(event).await.unwrap()
    }

    pub fn start(mut self) {
        tokio::spawn(async move {
            loop {
                eprintln!("Yor");
                let action = match self.handle.request.recv().await {
                    Some(action) => action,
                    None => break,
                };

                let Action { action, .. } = action;
                match action {
                    ActionKind::Connect => {
                        self.send(NetEvent::to_event(
                            self.id.clone(),
                            NetEventKind::Error("Cannot disconnect from main room".to_string()),
                        ))
                        .await
                    }
                    ActionKind::Disconnect => {
                        self.send(NetEvent::to_event(
                            self.id.clone(),
                            NetEventKind::Error("Cannot disconnect from main room".to_string()),
                        ))
                        .await
                    }
                    ActionKind::Publish(packet) => {
                        self.send(NetEvent::to_event(
                            self.id.clone(),
                            NetEventKind::Message(Message {
                                date: "Yesterday".to_string(),
                                source: "Me".to_string(),
                                message: packet,
                            }),
                        ))
                        .await
                    }
                    ActionKind::NewRoom(room) => match self.spawn(room) {
                        Ok(room) => {
                            self.send(NetEvent::to_event(
                                self.id.clone(),
                                NetEventKind::NewRoom(room),
                            ))
                            .await
                        }
                        Err(e) => {
                            let error = format!("{:?}", e);
                            self.send(NetEvent::to_event(
                                self.id.clone(),
                                NetEventKind::Error(error),
                            ))
                            .await
                        }
                    },
                }
            }
        });
    }

    fn spawn(&mut self, room: room::net::NewRoom) -> Result<event::NewRoom, Error> {
        let room::net::NewRoom { alias, command } = room;
        let mut tokens: Vec<_> = command.split(' ').collect();
        if tokens.len() < 2 {
            return Err(Error::BadId("No server type specified!".to_string()));
        }
        let s_type = tokens.remove(0);
        match s_type {
            "matrix" => {
                if tokens.is_empty() {
                    return Err(Error::BadId(
                        "Syntax: matrix <url> [username [password]]".to_string(),
                    ));
                }
                let credentials = if tokens.len() >= 2 {
                    let username = tokens[1].to_string();
                    let password = if tokens.len() >= 3 { tokens[2] } else { "" }.to_string();
                    Some(super::matrix::Credentials { username, password })
                } else {
                    None
                };
                let id = [&self.id[..], &[self.room_sn.next()]].concat();
                let room::Handle { client, server } = room::Handle::new();
                let server = super::matrix::Server::new(
                    id.clone(),
                    super::matrix::Conf {
                        url: tokens[0].to_string(),
                        sync_period: 1024,
                        credentials,
                    },
                    server,
                    client.request.clone(),
                );
                let server = match server {
                    Ok(s) => s,
                    Err(e) => return Err(Error::BadId(format!("Bad matrix ID: {:?}", e))),
                };
                let server_thread = server.start();
                tokio::spawn(server_thread);
                Ok(event::NewRoom {
                    id: Some(id),
                    alias,
                    requester: client.request,
                })
            }
            s_type => Err(Error::BadId(format!("Unknown server type '{}'", s_type))),
        }
    }
}
