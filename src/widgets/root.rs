use crate::{
    encoded_writer::EncodedWriter,
    events::AppEvent,
    log::Log,
    widgets::{
        BindingDisplay, CommandInput, CommandInputState, Controls, ControlsState, FormattedLog,
        FormattedLogState, IconPack, RawLog, RawLogState, State, WithLog,
    },
};
use crossterm::event::{Event, KeyCode, KeyModifiers};
use indexmap::IndexMap;
use std::{marker::PhantomData, process::ChildStdin};
use tracing::debug;
use tui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, StatefulWidget},
};

#[derive(Clone, Debug, Default)]
pub struct Root<'i> {
    marker: PhantomData<&'i Log>,
}

impl<'i> StatefulWidget for Root<'i> {
    type State = RootState<'i>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        // Get outer layout
        let mut layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints({
                let mut constraints = Vec::with_capacity(3);
                constraints.push(Constraint::Min(0));
                if state.command_input_state.is_some() {
                    constraints.push(Constraint::Length(3))
                }
                constraints.push(Constraint::Length(1));
                constraints
            })
            .split(area);
        let controls_area = layout.pop().unwrap();
        let command_input_area = state
            .command_input_state
            .is_some()
            .then(|| layout.pop().unwrap());
        let area = layout.pop().unwrap();

        // Get inner layout
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(match state.selected_widget {
                SelectedWidget::FormattedLog => {
                    vec![Constraint::Percentage(80), Constraint::Percentage(20)]
                }
                SelectedWidget::RawLog => {
                    vec![Constraint::Percentage(20), Constraint::Percentage(80)]
                }
                _ => vec![Constraint::Percentage(50), Constraint::Percentage(50)],
            })
            .split(area);
        let formatted_log_area = layout[0];
        let raw_log_area = layout[1];

        // Draw formatted log
        state
            .formatted_log_state
            .selected(state.selected_widget == SelectedWidget::FormattedLog);
        FormattedLog::default().render(formatted_log_area, buf, &mut state.formatted_log_state);

        // Draw raw log
        state
            .raw_log_state
            .set_selected(state.selected_widget == SelectedWidget::RawLog);
        RawLog::default().render(raw_log_area, buf, &mut state.raw_log_state);

        // Draw command input
        if let Some((command_input_state, _)) = state.command_input_state.as_mut() {
            let focused = state.selected_widget == SelectedWidget::CommandInput;
            let style = Style::default()
                .fg(if focused {
                    Color::White
                } else {
                    Color::DarkGray
                })
                .bg(Color::Black);
            CommandInput::default()
                .style(style)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(style)
                        .border_type(BorderType::Double)
                        .title("Command"),
                )
                .focused(focused)
                .render(command_input_area.unwrap(), buf, command_input_state);
        }

        // Draw controls
        let mut controls = IndexMap::new();
        state.add_controls(&mut controls);
        state.controls_state.set_controls(controls);
        Controls::default()
            .style(Style::default().fg(Color::White).bg(Color::Blue))
            .render(controls_area, buf, &mut state.controls_state);
    }
}

#[derive(Debug)]
pub struct RootState<'i> {
    formatted_log_state: FormattedLogState<'i>,
    raw_log_state: RawLogState<'i>,
    command_input_state: Option<(CommandInputState, EncodedWriter<ChildStdin>)>,
    controls_state: ControlsState,
    selected_widget: SelectedWidget,
}

impl<'i> RootState<'i> {
    pub fn new(log: &'i Log, command_stdin: Option<EncodedWriter<ChildStdin>>) -> Self {
        let mut state = Self {
            raw_log_state: RawLogState::new(log),
            formatted_log_state: FormattedLogState::new(log),
            command_input_state: command_stdin.map(|stdin| (CommandInputState::default(), stdin)),
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
        // Update root state
        let mut handled = match event {
            AppEvent::TermEvent(Event::Key(key_event)) => match key_event.code {
                KeyCode::Tab => {
                    match self.selected_widget {
                        SelectedWidget::FormattedLog => self.select_widget(SelectedWidget::RawLog),
                        SelectedWidget::RawLog if self.command_input_state.is_none() => {
                            self.select_widget(SelectedWidget::FormattedLog)
                        }
                        SelectedWidget::RawLog => self.select_widget(SelectedWidget::CommandInput),
                        SelectedWidget::CommandInput => {
                            self.select_widget(SelectedWidget::FormattedLog)
                        }
                    }
                    true
                }
                KeyCode::BackTab => {
                    match self.selected_widget {
                        SelectedWidget::FormattedLog if self.command_input_state.is_none() => {
                            self.select_widget(SelectedWidget::RawLog)
                        }
                        SelectedWidget::FormattedLog => {
                            self.select_widget(SelectedWidget::CommandInput)
                        }
                        SelectedWidget::RawLog => self.select_widget(SelectedWidget::FormattedLog),
                        SelectedWidget::CommandInput => self.select_widget(SelectedWidget::RawLog),
                    }
                    true
                }
                _ => false,
            },
            _ => false,
        };

        // Update selected widget
        if !handled {
            handled = match self.selected_widget {
                SelectedWidget::FormattedLog => self.formatted_log_state.update(event),
                SelectedWidget::RawLog => self.raw_log_state.update(event),
                SelectedWidget::CommandInput => self
                    .command_input_state
                    .as_mut()
                    .map(|(state, _)| state.update(event))
                    .unwrap_or(false),
            };
        }

        // Send commands if any
        if let Some((command_input_state, stdin)) = self.command_input_state.as_mut() {
            for cmd in command_input_state.take_submitted() {
                debug!(?cmd, "sending command");
                stdin.write_all(&cmd).unwrap();
                stdin.write_all("\n").unwrap();
                stdin.flush().unwrap();
            }
        }

        // Update controls state
        if !handled {
            handled = self.controls_state.update(event);
        }

        handled
    }

    fn add_controls<I: IconPack>(&self, controls: &mut IndexMap<BindingDisplay<I>, &'static str>) {
        // Root controls
        controls.insert(
            BindingDisplay::key(KeyCode::Char('c'), KeyModifiers::CONTROL),
            "Quit",
        );

        // Selected widget controls
        match self.selected_widget {
            SelectedWidget::FormattedLog => self.formatted_log_state.add_controls(controls),
            SelectedWidget::RawLog => self.raw_log_state.add_controls(controls),
            SelectedWidget::CommandInput => {
                if let Some((command_input_state, _)) = self.command_input_state.as_ref() {
                    command_input_state.add_controls(controls)
                }
            }
        }
    }
}

impl<'i, 'j> WithLog<'j> for RootState<'i> {
    type Result = RootState<'j>;

    fn with_log(self, log: &'j Log) -> Self::Result {
        RootState {
            formatted_log_state: self.formatted_log_state.with_log(log),
            raw_log_state: self.raw_log_state.with_log(log),
            command_input_state: self.command_input_state,
            controls_state: self.controls_state,
            selected_widget: self.selected_widget,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
enum SelectedWidget {
    FormattedLog,
    RawLog,
    CommandInput,
}

impl Default for SelectedWidget {
    fn default() -> Self {
        SelectedWidget::FormattedLog
    }
}
