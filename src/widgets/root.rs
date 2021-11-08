use crate::{
    events::AppEvent,
    log::Log,
    widgets::{
        BindingDisplay, Controls, ControlsState, FormattedLog, FormattedLogState, RawLog,
        RawLogState, State, WithLog,
    },
};
use crossterm::event::{Event, KeyCode, KeyModifiers};
use indexmap::IndexMap;
use std::marker::PhantomData;
use tui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::StatefulWidget,
};

#[derive(Clone, Debug, Default)]
pub struct Root<'i> {
    marker: PhantomData<&'i Log>,
}

impl<'i> StatefulWidget for Root<'i> {
    type State = RootState<'i>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        // Get controls area
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)].as_ref())
            .split(area);
        let area = layout[0];
        let controls_area = layout[1];

        // Get vertical layout
        let constraints = match state.selected_widget {
            SelectedWidget::FormattedLog => {
                [Constraint::Percentage(80), Constraint::Percentage(20)]
            }
            SelectedWidget::RawLog => [Constraint::Percentage(20), Constraint::Percentage(80)],
        };
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints.as_ref())
            .split(area);
        let formatted_log_area = layout[0];
        let raw_log_area = layout[1];

        // Draw formatted log
        state
            .formatted_log_state
            .set_selected(state.selected_widget == SelectedWidget::FormattedLog);
        FormattedLog::default().render(formatted_log_area, buf, &mut state.formatted_log_state);

        // Draw raw log
        state
            .raw_log_state
            .set_selected(state.selected_widget == SelectedWidget::RawLog);
        RawLog::default().render(raw_log_area, buf, &mut state.raw_log_state);

        // Draw controls
        let mut controls = IndexMap::new();
        state.add_controls(&mut controls);
        state.controls_state.set_controls(controls);
        Controls::default()
            .style(Style::default().fg(Color::White).bg(Color::Blue))
            .render(controls_area, buf, &mut state.controls_state);
    }
}

#[derive(Clone, Debug)]
pub struct RootState<'i> {
    formatted_log_state: FormattedLogState<'i>,
    raw_log_state: RawLogState<'i>,
    controls_state: ControlsState,
    selected_widget: SelectedWidget,
}

impl<'i> RootState<'i> {
    pub fn new(log: &'i Log) -> Self {
        let mut state = Self {
            raw_log_state: RawLogState::new(log),
            formatted_log_state: FormattedLogState::new(log),
            controls_state: ControlsState::default(),
            selected_widget: SelectedWidget::default(),
        };

        // Select the selected widget to update the widget states
        state.select_widget(state.selected_widget);

        state
    }

    fn select_widget(&mut self, widget: SelectedWidget) {
        self.selected_widget = widget;
    }
}

impl<'i> State for RootState<'i> {
    fn update(&mut self, event: &AppEvent) -> bool {
        // TODO: mouse events
        let handled = match event {
            AppEvent::TermEvent(Event::Key(key_event)) => match key_event.code {
                KeyCode::Tab => {
                    match self.selected_widget {
                        SelectedWidget::FormattedLog => self.select_widget(SelectedWidget::RawLog),
                        SelectedWidget::RawLog => self.select_widget(SelectedWidget::FormattedLog),
                    }
                    true
                }
                _ => false,
            },
            _ => false,
        };

        if handled {
            true
        } else {
            match self.selected_widget {
                SelectedWidget::FormattedLog => self.formatted_log_state.update(event),
                SelectedWidget::RawLog => self.raw_log_state.update(event),
            }
        }
    }

    fn add_controls(&self, controls: &mut IndexMap<BindingDisplay, &'static str>) {
        // Root controls
        controls.insert(
            BindingDisplay::key(KeyCode::Char('c'), KeyModifiers::CONTROL),
            "Quit",
        );

        // Selected widget controls
        match self.selected_widget {
            SelectedWidget::FormattedLog => self.formatted_log_state.add_controls(controls),
            SelectedWidget::RawLog => self.raw_log_state.add_controls(controls),
        }
    }
}

impl<'i, 'j> WithLog<'j> for RootState<'i> {
    type Result = RootState<'j>;

    fn with_log(self, log: &'j Log) -> Self::Result {
        RootState {
            formatted_log_state: self.formatted_log_state.with_log(log),
            raw_log_state: self.raw_log_state.with_log(log),
            controls_state: self.controls_state,
            selected_widget: self.selected_widget,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
enum SelectedWidget {
    FormattedLog,
    RawLog,
}

impl Default for SelectedWidget {
    fn default() -> Self {
        SelectedWidget::FormattedLog
    }
}
