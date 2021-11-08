use std::ops::Range;
use tui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    symbols::block::FULL,
    widgets::Widget,
};

#[derive(Clone, Debug)]
pub struct Scrollbar {
    visible: Range<f32>,
    max: f32,
    track_style: Style,
    bar_style: Style,
}

impl Scrollbar {
    pub fn new(visible: Range<f32>, max: f32) -> Self {
        assert!(visible.start >= 0.0);
        assert!(visible.end <= max);
        assert!(visible.start <= visible.end);
        Scrollbar {
            visible,
            max,
            track_style: Style::default().fg(Color::DarkGray),
            bar_style: Style::default().fg(Color::White),
        }
    }

    pub fn set_track_style(mut self, style: Style) -> Self {
        self.track_style = style;
        self
    }

    pub fn set_bar_style(mut self, style: Style) -> Self {
        self.bar_style = style;
        self
    }
}

impl Widget for Scrollbar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Render track
        for y in area.top()..area.bottom() {
            buf.get_mut(area.left(), y)
                .set_symbol(FULL)
                .set_style(self.track_style);
        }

        // Render bar
        let height = area.height as f32;
        let ratio = height / self.max;
        let bar_height = (((self.visible.end - self.visible.start) * ratio).ceil() as u16)
            .max(1)
            .min(area.height);
        let bar_top = ((self.visible.start * ratio).floor() as u16)
            .min(area.height.saturating_sub(bar_height));
        for y in 0..bar_height {
            buf.get_mut(
                area.left(),
                area.top().saturating_add(bar_top).saturating_add(y),
            )
            .set_symbol(FULL)
            .set_style(self.bar_style);
        }
    }
}
