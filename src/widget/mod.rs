pub mod room_entry;
pub mod scroll;
pub mod text;

pub trait Height {
    fn height(&self, width: u16) -> usize;
}
