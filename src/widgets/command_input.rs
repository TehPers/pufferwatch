use crate::{events::AppEvent, widgets::State};
use crossterm::event::{Event, KeyCode};
use indexmap::IndexMap;
use tui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, StatefulWidget, Widget},
};

use super::{BindingDisplay, IconPack};

#[derive(Clone, Default)]
pub struct CommandInput<'i> {
    block: Option<Block<'i>>,
    style: Style,
    focused: bool,
}

impl<'i> CommandInput<'i> {
    pub fn block(mut self, block: Block<'i>) -> Self {
        self.block = Some(block);
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }
}

impl<'i> StatefulWidget for CommandInput<'i> {
    type State = CommandInputState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        // Render block
        let inner_area = if let Some(block) = self.block {
            let inner = block.inner(area);
            block.render(area, buf);
            inner
        } else {
            area
        };

        // Render input
        let spans = if self.focused {
            vec![
                Span::styled(state.before_cursor(), self.style),
                Span::styled(
                    state.at_cursor().map_or_else(String::default, Into::into),
                    self.style.add_modifier(Modifier::REVERSED),
                ),
                Span::styled(state.after_cursor(), self.style),
            ]
        } else {
            vec![Span::styled(&state.text, self.style)]
        };
        let spans = spans.into();
        buf.set_spans(
            inner_area.left(),
            inner_area.top(),
            &spans,
            inner_area.width,
        );
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
enum EditMode {
    Insert,
    Overwrite,
}

impl Default for EditMode {
    fn default() -> Self {
        EditMode::Insert
    }
}

#[derive(Clone, Debug, Default)]
pub struct CommandInputState {
    text: String,
    // Cursor index with respect to characters (not bytes)
    cursor: usize,
    submitted: Vec<String>,
    edit_mode: EditMode,
}

impl CommandInputState {
    fn before_cursor(&self) -> String {
        self.text.chars().take(self.cursor).collect()
    }

    fn at_cursor(&self) -> Option<char> {
        self.text.chars().nth(self.cursor)
    }

    fn after_cursor(&self) -> String {
        self.text.chars().skip(self.cursor + 1).collect()
    }

    pub fn take_submitted(&mut self) -> impl IntoIterator<Item = String> + '_ {
        self.submitted.drain(..)
    }
}

impl State for CommandInputState {
    fn update(&mut self, event: &AppEvent) -> bool {
        let event = match event {
            AppEvent::TermEvent(event) => event,
            AppEvent::Ping => return false,
        };

        match event {
            Event::Key(event) => match event.code {
                KeyCode::Backspace if self.cursor > 0 => {
                    let chars = self.text.chars();
                    let before = chars.clone().take(self.cursor - 1);
                    let after = chars.skip(self.cursor);
                    self.text = before.chain(after).collect();
                    self.cursor -= 1;
                    true
                }
                KeyCode::Enter => {
                    self.submitted.push(std::mem::take(&mut self.text));
                    self.cursor = 0;
                    true
                }
                KeyCode::Left if self.cursor > 0 => {
                    self.cursor -= 1;
                    true
                }
                KeyCode::Right if self.cursor < self.text.len() => {
                    self.cursor += 1;
                    true
                }
                KeyCode::Home => {
                    self.cursor = 0;
                    true
                }
                KeyCode::End => {
                    self.cursor = self.text.len();
                    true
                }
                KeyCode::Delete if self.cursor < self.text.len() => {
                    self.text.remove(self.cursor);
                    true
                }
                KeyCode::Insert => {
                    self.edit_mode = match self.edit_mode {
                        EditMode::Insert => EditMode::Overwrite,
                        EditMode::Overwrite => EditMode::Insert,
                    };
                    true
                }
                KeyCode::Char(c) => {
                    match self.edit_mode {
                        EditMode::Insert => {
                            let chars = self.text.chars();
                            let before = chars.clone().take(self.cursor);
                            let inserted = [c];
                            let after = chars.skip(self.cursor);
                            self.text = before.chain(inserted).chain(after).collect();
                            self.cursor += 1;
                        }
                        EditMode::Overwrite => {
                            let chars = self.text.chars();
                            let before = chars.clone().take(self.cursor);
                            let inserted = [c];
                            let after = chars.skip(self.cursor + 1);
                            self.text = before.chain(inserted).chain(after).collect();
                            self.cursor += 1;
                        }
                    }

                    true
                }
                _ => false,
            },
            _ => false,
        }
    }

    fn add_controls<I: IconPack>(&self, controls: &mut IndexMap<BindingDisplay<I>, &'static str>) {
        controls.insert(BindingDisplay::simple_key(KeyCode::Enter), "Execute");
        controls.insert(BindingDisplay::Custom(I::LEFT_RIGHT), "Nav");
        controls.insert(
            BindingDisplay::simple_key(KeyCode::Insert),
            match self.edit_mode {
                EditMode::Insert => "Overwrite",
                EditMode::Overwrite => "Insert",
            },
        );
    }
}
