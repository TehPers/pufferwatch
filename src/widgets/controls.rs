use crate::{
    events::AppEvent,
    widgets::{DefaultIconPack, IconPack, State},
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseButton};
use indexmap::IndexMap;
use itertools::Itertools;
use std::{
    fmt::{Display, Formatter},
    marker::PhantomData,
};
use tui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Span, Spans},
    widgets::StatefulWidget,
};
use unicode_width::UnicodeWidthStr;

#[allow(dead_code)] // TODO: Add support for mouse events
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum BindingDisplay<I: IconPack> {
    Key {
        key_code: KeyCode,
        modifiers: KeyModifiers,
    },
    Mouse(MouseButton),
    Custom(&'static str),
    #[doc(hidden)]
    __Marker(PhantomData<*const I>),
}

impl<I: IconPack> BindingDisplay<I> {
    const MODIFIER_DISPLAYS: [(KeyModifiers, &'static str); 3] = [
        (KeyModifiers::CONTROL, I::CONTROL_ICON),
        (KeyModifiers::ALT, I::ALT_ICON),
        (KeyModifiers::SHIFT, I::SHIFT_ICON),
    ];

    pub fn key(key_code: KeyCode, modifiers: KeyModifiers) -> Self {
        BindingDisplay::Key {
            key_code,
            modifiers,
        }
    }

    pub fn simple_key(key_code: KeyCode) -> Self {
        BindingDisplay::Key {
            key_code,
            modifiers: KeyModifiers::empty(),
        }
    }
}

impl<I: IconPack> Display for BindingDisplay<I> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BindingDisplay::Key {
                key_code,
                modifiers,
            } => {
                // Write modifiers
                let modifier_icons = Self::MODIFIER_DISPLAYS
                    .into_iter()
                    .filter(|&(modifier, _)| modifiers.contains(modifier))
                    .map(|(_, modifier_icon)| modifier_icon);
                for icon in modifier_icons {
                    write!(f, "{}", icon)?;
                }

                // Write key code
                match key_code {
                    KeyCode::BackTab => write!(f, "{}", I::BACKTAB_ICON),
                    KeyCode::Backspace => write!(f, "{}", I::BACKSPACE_ICON),
                    KeyCode::Char(' ') => write!(f, "{}", I::SPACE_ICON),
                    KeyCode::Char(c) => write!(f, "{}", c),
                    KeyCode::Delete => write!(f, "{}", I::DELETE_ICON),
                    KeyCode::Down => write!(f, "{}", I::DOWN_ICON),
                    KeyCode::End => write!(f, "{}", I::END_ICON),
                    KeyCode::Enter => write!(f, "{}", I::ENTER_ICON),
                    KeyCode::Esc => write!(f, "{}", I::ESC_ICON),
                    KeyCode::F(n) => write!(f, "F{}", n),
                    KeyCode::Home => write!(f, "{}", I::HOME_ICON),
                    KeyCode::Insert => write!(f, "{}", I::INSERT_ICON),
                    KeyCode::Left => write!(f, "{}", I::LEFT_ICON),
                    KeyCode::Null => write!(f, "{}", I::NULL_ICON),
                    KeyCode::PageDown => write!(f, "{}", I::PAGEDOWN_ICON),
                    KeyCode::PageUp => write!(f, "{}", I::PAGEUP_ICON),
                    KeyCode::Right => write!(f, "{}", I::RIGHT_ICON),
                    KeyCode::Tab => write!(f, "{}", I::TAB_ICON),
                    KeyCode::Up => write!(f, "{}", I::UP_ICON),
                }
            }
            BindingDisplay::Mouse(MouseButton::Left) => write!(f, "M1"),
            BindingDisplay::Mouse(MouseButton::Right) => write!(f, "M2"),
            BindingDisplay::Mouse(MouseButton::Middle) => write!(f, "M3"),
            BindingDisplay::Custom(label) => write!(f, "{}", label),
            _ => Ok(()),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Controls {
    style: Style,
}

impl Controls {
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

impl StatefulWidget for Controls {
    type State = ControlsState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if area.height <= 0 {
            return;
        }

        // Create the "More" label
        let more_label = Span::styled("More [.]", self.style);

        // Get the available width for the controls, excluding the "More" label
        let controls_width = usize::from(area.width).saturating_sub(more_label.content.width());
        if controls_width == 0 {
            return;
        }

        // Get labels for each control
        let labels = state
            .controls
            .iter()
            .map(|(control, label)| {
                format!("{label} [{control}]", control = control, label = label)
            })
            .map(|label| Span::styled(label, self.style));

        // Group labels into lines
        let mut multi_page = false;
        let lines = labels
            .map(|label| {
                let label_width = label.content.width();
                (label, label_width)
            })
            .peekable()
            .batching(|labels| {
                let mut remaining_width = controls_width;
                let mut line = Vec::new();
                while let Some(&(_, label_width)) = labels.peek() {
                    // Check if the label fits on the current line
                    if let Some(new_remaining_width) = remaining_width.checked_sub(label_width) {
                        // Label fits on the current line
                        remaining_width = new_remaining_width.saturating_sub(1);

                        // Add label and padding
                        let (label, _) = labels.next().unwrap();
                        line.push(label);
                        line.push(Span::raw(" "));
                    } else {
                        // Check if empty page because area isn't big enough
                        if line.is_empty() {
                            return None;
                        }

                        // Add "More" label (for next page)
                        line.push(more_label.clone());
                        multi_page = true;
                        return Some(Spans::from(line));
                    }
                }

                if line.is_empty() {
                    None
                } else {
                    if multi_page {
                        // Add "More" label (for first page)
                        line.push(more_label.clone());
                    }

                    Some(Spans::from(line))
                }
            })
            .enumerate()
            .collect_vec();

        // Get which rows to render
        let area_height = usize::from(area.height);
        let pages = lines.len() / area_height;
        state.page %= pages;
        let start_row = state.page * area_height;

        // Render the controls
        let rendered_lines = lines.get(start_row..(start_row + area_height));
        for (y, spans) in rendered_lines.into_iter().flatten() {
            let y = match u16::try_from(y % area_height) {
                Ok(y) => y,
                Err(_) => break,
            };
            buf.set_spans(area.x, area.y.saturating_add(y), spans, area.width);
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ControlsState {
    controls: IndexMap<BindingDisplay<DefaultIconPack>, &'static str>,
    page: usize,
}

impl ControlsState {
    pub fn set_controls(
        &mut self,
        controls: IndexMap<BindingDisplay<DefaultIconPack>, &'static str>,
    ) -> &mut Self {
        self.controls = controls;
        self
    }
}

impl State for ControlsState {
    fn update(&mut self, event: &AppEvent) -> bool {
        if let AppEvent::TermEvent(Event::Key(KeyEvent {
            code: KeyCode::Char('.'),
            ..
        })) = event
        {
            self.page = self.page.wrapping_add(1);
            return true;
        }

        false
    }
}
