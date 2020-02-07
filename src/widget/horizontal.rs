use super::Height;
use tui::widgets::Widget;

pub trait Element: Height + Widget + Send {}

pub struct Horizontal {
    widgets: Vec<Box<dyn Element>>,
    widths: Vec<usize>,
    limiter: Option<char>,
}

impl Horizontal {}

impl std::fmt::Debug for Horizontal {}

impl Height for Horizontal {}

impl Widget for Horizontal {}
