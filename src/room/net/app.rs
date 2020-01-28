use super::Error;
use crate::sequence_number::SequenceNumber;
use std::collections::HashMap;
use std::sync::mpsc;

use crate::room;
pub struct App {
    handle: room::ServerHandle,
    rooms: HashMap<room::Id, room::ServerHandle>,
    room_sn: SequenceNumber,
    input_notification: mpsc::Sender<crate::event::Event>,
}

impl App {
    pub fn new(
        input_notification: mpsc::Sender<crate::event::Event>,
        handle: room::ServerHandle,
    ) -> Self {
        Self {
            handle,
            rooms: HashMap::new(),
            room_sn: SequenceNumber::new(),
            input_notification,
        }
    }
}

impl App {
    pub fn start(self) {
        std::thread::spawn(move || loop {
            eprintln!("Yor");
            let ev = match self.handle.request.recv() {
                Ok(ev) => ev,
                Err(_) => break,
            };

            match ev {
                x => eprintln!("APP REQUEST: {:#?}", x),
            }
        });
    }

    fn _spawn(
        &mut self,
        id: room::StringId,
        handle: room::ServerHandle,
    ) -> Result<room::Id, Error> {
        let mut tokens: Vec<_> = id.split(' ').collect();
        if tokens.is_empty() {
            return Err(Error::BadId(format!("No server type specified!")));
        }
        let room_id = self.room_sn.next();
        let s_type = tokens.remove(0);
        match s_type {
            "matrix" => {
                if tokens.len() < 1 {
                    return Err(Error::BadId(format!(
                        "Syntax: matrix <url> [username [password]]"
                    )));
                }
                let credentials = if tokens.len() >= 2 {
                    let username = tokens[1].to_string();
                    let password = if tokens.len() >= 3 { tokens[2] } else { "" }.to_string();
                    Some(super::matrix::Credentials { username, password })
                } else {
                    None
                };
                let server = super::matrix::Server::new(
                    super::matrix::Conf {
                        url: tokens[0].to_string(),
                        sync_period: 1024,
                        credentials,
                    },
                    handle,
                    self.input_notification.clone(),
                );
                let server = match server {
                    Ok(s) => s,
                    Err(e) => return Err(Error::BadId(format!("Bad matrix ID: {:?}", e))),
                };
            }
            s_type => return Err(Error::BadId(format!("Unknown server type '{}'", s_type))),
        }
        // TODO Start server client
        Ok(room_id)
    }
}
