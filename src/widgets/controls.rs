use crate::{
    events::AppEvent,
    widgets::{DefaultIconPack, IconPack, State},
};
use crossterm::event::{KeyCode, KeyModifiers, MouseButton};
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

        // Group controls into lines
        let mut lines = labels
            .scan((0_usize, controls_width), |state, label| {
                let (line, remaining_width) = *state;

                // Get label width + padding
                let label_width = label.content.width().saturating_add(1);

                // Add label to either the current line or the next line
                match remaining_width.checked_sub(label_width) {
                    // Too big for this line
                    None => {
                        *state = (
                            line.saturating_add(1),
                            controls_width.saturating_sub(label_width),
                        );
                        Some((state.0, label))
                    }
                    // Fits on this line
                    Some(remaining_width) => {
                        *state = (line, remaining_width);
                        Some((state.0, label))
                    }
                }
            })
            .group_by(|&(line, _)| line)
            .into_iter()
            .map(|(line, labels)| {
                let spans = labels
                    .map(|(_, label)| label)
                    .interleave_shortest(std::iter::repeat_with(|| Span::raw(" ")))
                    .collect_vec();
                (line, Spans::from(spans))
            })
            .collect_vec();
        lines.sort_unstable_by_key(|&(line, _)| line);

        // Get which rows to render
        let start_row = state.page * usize::from(area.height);
        let line_after_end_row = start_row
            .saturating_add(area.height.into())
            .min(lines.len());
        let start_row = line_after_end_row.saturating_sub(area.height.into());

        // Render the controls
        let rendered_lines = lines.get(start_row..line_after_end_row);
        for (y, spans) in rendered_lines.into_iter().flatten() {
            buf.set_spans(area.x, area.y.saturating_add(*y as u16), spans, area.width);
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
    fn update(&mut self, _event: &AppEvent) -> bool {
        // TODO
        false
    }
}
