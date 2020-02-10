use crate::room;

pub mod app;
pub mod matrix;

#[derive(Debug)]
pub struct NewRoom {
    pub alias: String,
    pub command: Vec<String>,
}

#[derive(Debug)]
pub enum ActionKind {
    Sync,
    Connect,
    Disconnect,
    Publish(String),
    NewRoom(NewRoom),
    // TODO Add configuration action
    // Configuration(String),
}

#[derive(Debug)]
pub struct Action {
    pub room: room::Id,
    pub action: ActionKind,
}

#[derive(Debug)]
pub enum Error {
    BadId(String),
}
