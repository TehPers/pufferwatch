use crate::{
    events::AppEvent,
    log::Log,
    widgets::{BindingDisplay, IconPack, LazyParagraph, LazyParagraphState, State, WithLog},
};
use indexmap::IndexMap;
use std::marker::PhantomData;
use tui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, StatefulWidget},
};

#[derive(Clone, Debug, Default)]
pub struct RawLog<'i> {
    marker: PhantomData<&'i Log>,
}

impl<'i> StatefulWidget for RawLog<'i> {
    type State = RawLogState<'i>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let fg_color = if state.selected {
            Color::White
        } else {
            Color::DarkGray
        };
        LazyParagraph::new(|index| state.lines.get(index).copied().map(Into::into))
            .block(
                Block::default()
                    .title("Raw")
                    .style(Style::default().fg(fg_color).bg(Color::Black))
                    .borders(Borders::all())
                    .border_type(BorderType::Double),
            )
            .style(Style::default().fg(fg_color).bg(Color::Black))
            .render(area, buf, &mut state.paragraph_state);
    }
}

#[derive(Clone, Debug)]
pub struct RawLogState<'i> {
    lines: Vec<&'i str>,
    paragraph_state: LazyParagraphState,
    selected: bool,
}

impl<'i> RawLogState<'i> {
    pub fn new(log: &'i Log) -> Self {
        let lines: Vec<_> = log.raw().lines().collect();
        let paragraph_state = LazyParagraphState::new(lines.len());
        RawLogState {
            lines,
            paragraph_state,
            selected: Default::default(),
        }
    }

    pub fn set_selected(&mut self, selected: bool) {
        self.selected = selected;
    }
}

impl<'i> State for RawLogState<'i> {
    fn update(&mut self, event: &AppEvent) -> bool {
        self.paragraph_state.update(event)
    }

    fn add_controls<I: IconPack>(&self, controls: &mut IndexMap<BindingDisplay<I>, &'static str>) {
        self.paragraph_state.add_controls(controls);
    }
}

impl<'i, 'j> WithLog<'j> for RawLogState<'i> {
    type Result = RawLogState<'j>;

    fn with_log(self, log: &'j Log) -> Self::Result {
        RawLogState {
            selected: self.selected,
            paragraph_state: LazyParagraphState {
                offset: self.paragraph_state.offset,
                ..LazyParagraphState::new(log.raw().lines().count())
            },
            ..RawLogState::new(log)
        }
    }
}
