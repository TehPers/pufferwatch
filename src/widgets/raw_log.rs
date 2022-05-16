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
    style::Style,
    widgets::{Block, StatefulWidget},
};

#[derive(Clone, Debug, Default)]
pub struct RawLog<'i> {
    block: Option<Block<'i>>,
    style: Style,
    marker: PhantomData<&'i Log>,
}

impl<'i> RawLog<'i> {
    #[allow(dead_code)]
    pub fn block(mut self, block: Block<'i>) -> Self {
        self.block = Some(block);
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

impl<'i> StatefulWidget for RawLog<'i> {
    type State = RawLogState<'i>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let paragraph = LazyParagraph::new(|index| state.lines.get(index).copied().map(Into::into))
            .style(self.style);
        let paragraph = if let Some(block) = self.block {
            paragraph.block(block)
        } else {
            paragraph
        };
        paragraph.render(area, buf, &mut state.paragraph_state);
    }
}

#[derive(Clone, Debug)]
pub struct RawLogState<'i> {
    lines: Vec<&'i str>,
    paragraph_state: LazyParagraphState,
}

impl<'i> RawLogState<'i> {
    pub fn new(log: &'i Log) -> Self {
        let lines: Vec<_> = log.raw().lines().collect();
        let paragraph_state = LazyParagraphState::new(lines.len(), true);
        RawLogState {
            lines,
            paragraph_state,
        }
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
            paragraph_state: LazyParagraphState {
                offset: self.paragraph_state.offset,
                auto_scroll: self.paragraph_state.auto_scroll,
                ..LazyParagraphState::new(log.raw().lines().count(), true)
            },
            ..RawLogState::new(log)
        }
    }
}
