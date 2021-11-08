use crate::{
    events::AppEvent,
    widgets::{BindingDisplay, Scrollbar, State},
};
use crossterm::event::{Event, KeyCode};
use indexmap::IndexMap;
use tui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::Spans,
    widgets::{Block, Clear, StatefulWidget, Widget},
};
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Debug)]
pub struct LazyParagraph<'i, F> {
    get_line: F,
    block: Option<Block<'i>>,
    style: Style,
}

impl<'i, F> LazyParagraph<'i, F>
where
    F: Fn(usize) -> Option<Spans<'i>>,
{
    /// Create a new lazy paragraph
    pub fn new(get_line: F) -> Self {
        LazyParagraph {
            get_line,
            block: Default::default(),
            style: Default::default(),
        }
    }

    /// Sets the block to be used.
    pub fn block(mut self, block: Block<'i>) -> Self {
        self.block = Some(block);
        self
    }

    /// Sets the style to be used.
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

impl<'i, F> StatefulWidget for LazyParagraph<'i, F>
where
    F: Fn(usize) -> Option<Spans<'i>>,
{
    type State = LazyParagraphState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        // Render block
        let has_block = self.block.is_some();
        let inner_area = if let Some(block) = self.block {
            let inner = block.inner(area);
            block.render(area, buf);
            inner
        } else {
            area
        };

        // Get rendered lines
        let height = inner_area.height.into();
        let line_after_last_line = state.offset.y.saturating_add(height).min(state.lines);
        let first_line = line_after_last_line.saturating_sub(height);
        state.offset.y = first_line;

        // Render paragraph
        let render_scrollbar = state.lines > height;
        let text_area = if render_scrollbar && !has_block {
            Rect {
                width: inner_area.width.saturating_sub(1),
                ..inner_area
            }
        } else {
            inner_area
        };
        for (line, i) in (first_line..line_after_last_line).zip(0..) {
            // Get line area
            let line_area = Rect::new(
                text_area.left(),
                text_area.top().saturating_add(i),
                text_area.width,
                1,
            );

            // Clear line area
            Clear.render(line_area, buf);

            // Get line contents
            let line_contents = match (self.get_line)(line) {
                Some(line_contents) => line_contents,
                None => continue,
            };

            // Render line contents
            let rendered_contents = line_contents
                .0
                .iter()
                .flat_map(|span| span.styled_graphemes(self.style))
                // Calculate x offset for each grapheme
                .scan(0_usize, |cur_x, grapheme| {
                    let width = grapheme.symbol.width();
                    let x = *cur_x;
                    *cur_x = cur_x.saturating_add(width);
                    Some((x, *cur_x, grapheme))
                })
                // Ignore content that is to the left of the area
                .filter(|&(_, cur_x, _)| cur_x >= state.offset.x)
                // Offset the remaining graphemes
                .map(|(x, _, grapheme)| {
                    let x = x.saturating_sub(state.offset.x);
                    (x, grapheme)
                });

            for (x, grapheme) in rendered_contents {
                let x: u16 = match x.try_into() {
                    Ok(x) => x,
                    Err(_) => break,
                };
                let x = x.saturating_add(line_area.x);
                let y = line_area.y;
                let remaining_width = line_area.width.saturating_add(1).saturating_sub(x);
                if remaining_width == 0 {
                    break;
                }

                buf.set_stringn(
                    x,
                    y,
                    grapheme.symbol,
                    remaining_width.into(),
                    grapheme.style,
                );
            }
        }

        // Render scrollbar
        if render_scrollbar {
            let scrollbar_area = Rect::new(inner_area.right(), inner_area.y, 1, inner_area.height);
            let y = state.offset.y as f32;
            Scrollbar::new(y..(y + height as f32), state.lines as f32).render(scrollbar_area, buf);
        }
    }
}

#[derive(Clone, Debug)]
pub struct LazyParagraphState {
    pub lines: usize,
    pub offset: Offset,
}

impl LazyParagraphState {
    pub fn new(lines: usize) -> Self {
        LazyParagraphState {
            lines,
            offset: Default::default(),
        }
    }

    /// Scrolls the paragraph down by the given amount.
    pub fn scroll_down(&mut self, lines: usize) {
        self.offset.y = self.offset.y.saturating_add(lines);
        if self.offset.y > self.lines {
            self.offset.y = self.lines.saturating_sub(1);
        }
    }

    /// Scrolls the paragraph up by the given amount.
    pub fn scroll_up(&mut self, lines: usize) {
        self.offset.y = self.offset.y.saturating_sub(lines);
    }

    /// Scrolls the paragraph left by the given amount.
    pub fn scroll_left(&mut self, lines: usize) {
        self.offset.x = self.offset.x.saturating_sub(lines);
    }

    /// Scrolls the paragraph right by the given amount.
    pub fn scroll_right(&mut self, lines: usize) {
        self.offset.x = self.offset.x.saturating_add(lines);
    }

    /// Scrolls the paragraph to the top.
    pub fn scroll_to_top(&mut self) {
        self.offset.y = 0;
    }

    /// Scrolls the paragraph to the bottom.
    pub fn scroll_to_bottom(&mut self) {
        self.offset.y = self.lines.saturating_sub(1);
    }
}

impl State for LazyParagraphState {
    fn update(&mut self, event: &AppEvent) -> bool {
        match event {
            AppEvent::TermEvent(Event::Key(key_event)) => match key_event.code {
                KeyCode::Up => {
                    self.scroll_up(1);
                    true
                }
                KeyCode::Down => {
                    self.scroll_down(1);
                    true
                }
                KeyCode::Left => {
                    self.scroll_left(1);
                    true
                }
                KeyCode::Right => {
                    self.scroll_right(1);
                    true
                }
                KeyCode::PageUp => {
                    self.scroll_up(10);
                    true
                }
                KeyCode::PageDown => {
                    self.scroll_down(10);
                    true
                }
                KeyCode::Home => {
                    self.scroll_to_top();
                    true
                }
                KeyCode::End => {
                    self.scroll_to_bottom();
                    true
                }
                _ => false,
            },
            _ => false,
        }
    }

    fn add_controls(&self, controls: &mut IndexMap<BindingDisplay, &'static str>) {
        controls.insert(BindingDisplay::Custom(BindingDisplay::ARROWS), "Nav");
        controls.insert(BindingDisplay::simple_key(KeyCode::PageUp), "Up 10");
        controls.insert(BindingDisplay::simple_key(KeyCode::PageDown), "Down 10");
        controls.insert(BindingDisplay::simple_key(KeyCode::Home), "Top");
        controls.insert(BindingDisplay::simple_key(KeyCode::End), "Bottom");
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, Default)]
pub struct Offset {
    pub x: usize,
    pub y: usize,
}
