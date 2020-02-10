use super::Height;
use tui::widgets::Widget;

pub trait PartialWidget {
    fn partial_draw(
        &mut self,
        y_offset: usize,
        area: tui::layout::Rect,
        buf: &mut tui::buffer::Buffer,
    );
}

pub trait Element: PartialWidget + Height + Widget + Send {}

#[derive(Debug)]
struct Cursor {
    widget: usize,
    y: usize,
}

pub struct Scroll {
    cursor: Cursor,
    widgets: Vec<Box<dyn Element>>,
    next_move: isize,
}

impl Scroll {
    pub fn new(widgets: Vec<Box<dyn Element>>) -> Self {
        Self {
            cursor: Cursor { widget: 0, y: 0 },
            widgets,
            next_move: 0,
        }
    }

    pub fn push(&mut self, element: Box<dyn Element>) {
        self.widgets.push(element)
    }

    fn _up(&mut self, width: u16) {
        if self.cursor.y > 0 {
            self.cursor.y -= 1;
        } else if self.cursor.widget > 0 {
            self.cursor.widget -= 1;
            self.cursor.y = self.widgets[self.cursor.widget].height(width) - 1;
        }
    }

    fn _down(&mut self, width: u16) {
        if self.cursor.y < self.widgets[self.cursor.widget].height(width) - 1 {
            self.cursor.y += 1;
        } else if self.cursor.widget < self.widgets.len() - 1 {
            self.cursor.widget += 1;
            self.cursor.y = 0;
        }
    }

    pub fn up(&mut self) {
        self.next_move += 1;
    }

    pub fn down(&mut self) {
        self.next_move -= 1;
    }
}

impl std::fmt::Debug for Scroll {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Scroll {{ cursor: {:?}, widgets: Vec<Element; {}> }}",
            self.cursor,
            self.widgets.len()
        )
    }
}

impl Height for Scroll {
    fn height(&self, width: u16) -> usize {
        self.widgets.iter().map(|w| w.height(width)).sum()
    }
}

impl Widget for Scroll {
    fn draw(&mut self, area: tui::layout::Rect, buf: &mut tui::buffer::Buffer) {
        // Move view
        let view_move = self.next_move;
        self.next_move = 0;
        match view_move.cmp(&0) {
            std::cmp::Ordering::Less => {
                for _ in 0..-view_move {
                    self._down(area.width);
                }
            }
            std::cmp::Ordering::Greater => {
                for _ in 0..view_move {
                    self._up(area.width);
                }
            }
            std::cmp::Ordering::Equal => (),
        }

        // Draw
        let mut height = 0;
        for (i, widget) in self.widgets.iter_mut().skip(self.cursor.widget).enumerate() {
            let w_h = widget.height(area.width);

            if i == 0 {
                let mut area = area;
                area.height = usize::min(w_h - self.cursor.y, area.height as usize) as u16;
                widget.partial_draw(self.cursor.y, area, buf);
                height += area.height;
            } else {
                let mut area = area;
                area.y += height;

                area.height = usize::min(w_h, (area.height - height) as usize) as u16;
                height += area.height;

                widget.draw(area, buf);
            }

            if height >= area.height {
                break;
            }
        }
    }
}
