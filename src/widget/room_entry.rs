use crate::widget::{
    scroll::{Element, PartialWidget},
    text::Text,
    Height,
};
use std::fmt;
use tui::{style::Style, widgets::Widget};

#[derive(Debug)]
pub struct Conf {
    pub meta_width: u16,
}

#[derive(Debug)]
pub struct Meta {
    pub date: usize,
    pub sender: Option<String>, // TODO Centralize naming for ease of alias renaming
}

impl fmt::Display for Meta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {}",
            self.date.to_string().as_str(),
            match self.sender.as_ref() {
                Some(s) => s.as_str(),
                None => &"",
            },
        )
    }
}

#[derive(Debug)]
pub struct RoomEntry {
    pub conf: Conf,
    pub meta: Meta,
    pub meta_widget: Text,
    pub content_widget: Text,
}

impl RoomEntry {
    pub fn new(meta: Meta, content: &str, conf: Conf) -> Self {
        let mut meta_widget = Text::new(&meta.to_string());
        meta_widget.one_line = true;
        Self {
            conf,
            meta,
            meta_widget,
            content_widget: Text::new(content),
        }
    }
}

impl tui::widgets::Widget for RoomEntry {
    fn draw(&mut self, area: tui::layout::Rect, buf: &mut tui::buffer::Buffer) {
        self.partial_draw(0, area, buf);
    }
}

impl Height for RoomEntry {
    fn height(&self, width: u16) -> usize {
        usize::max(self.content_widget.height(width), 1)
    }
}

impl PartialWidget for RoomEntry {
    fn partial_draw(
        &mut self,
        y_offset: usize,
        area: tui::layout::Rect,
        buf: &mut tui::buffer::Buffer,
    ) {
        // Draw meta
        let mut meta_area = area;
        meta_area.width = self.conf.meta_width;
        self.meta_widget.draw(meta_area, buf);

        // TODO There needs to be a hint to know whether the content is complete or not
        // (symbol/color on the first line, number of lines, etc ...).

        // Draw content
        let mut content_area = area;
        content_area.x += meta_area.width + 3;
        content_area.width -= meta_area.width + 3;
        self.content_widget
            .partial_draw(y_offset, content_area, buf);

        // Draw bar
        let bar_x = area.x + meta_area.width + 1;
        for y in area.y..area.y + area.height {
            buf.set_string(bar_x, y as u16, "|", Style::default());
        }
    }
}

impl Element for RoomEntry {}
