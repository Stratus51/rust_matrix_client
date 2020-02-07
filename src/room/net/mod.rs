use crate::room;

pub mod app;
pub mod matrix;

#[derive(Debug)]
pub struct NewRoom {
    alias: String,
    command: String,
}

#[derive(Debug)]
pub enum ActionKind {
    Connect,
    Disconnect,
    Publish(String),
    NewRoom(NewRoom),
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
