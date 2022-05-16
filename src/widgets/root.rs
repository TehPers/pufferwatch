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
    widgets::{Block, BorderType, Borders, StatefulWidget, Tabs, Widget},
};

#[derive(Clone, Debug, Default)]
pub struct Root<'i> {
    marker: PhantomData<&'i Log>,
}

impl<'i> StatefulWidget for Root<'i> {
    type State = RootState<'i>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        // Styles
        let active_style = Style::default().fg(Color::White).bg(Color::Black);
        let inactive_style = active_style.fg(Color::DarkGray);

        // Get vertical layout
        let mut layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints({
                let mut constraints = Vec::with_capacity(3);
                constraints.push(Constraint::Min(0));
                if state.command_input_state.is_some() {
                    constraints.push(Constraint::Length(3));
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
        let log_area = layout.pop().unwrap();

        // Draw tabs
        let tabs_block = Block::default()
            .borders(Borders::ALL)
            .border_style(if state.selected_widget == SelectedWidget::Log {
                active_style
            } else {
                inactive_style
            })
            .border_type(BorderType::Double);
        let log_inner_area = tabs_block.inner(log_area);
        Tabs::new(vec!["Log".into(), "Raw".into()])
            .block(tabs_block)
            .style(active_style)
            .divider("|")
            .highlight_style(active_style.fg(Color::Black).bg(Color::White))
            .select(match state.selected_tab {
                SelectedTab::FormattedLog => 0,
                SelectedTab::RawLog => 1,
            })
            .render(log_area, buf);

        // Draw selected tab's contents
        let log_inner_area = Rect {
            x: log_inner_area.x,
            y: log_inner_area.y + 1,
            width: log_inner_area.width,
            height: log_inner_area.height.saturating_sub(1),
        };
        match state.selected_tab {
            SelectedTab::FormattedLog => {
                // Draw formatted log
                FormattedLog::default()
                    .default_style(if state.selected_widget == SelectedWidget::Log {
                        active_style
                    } else {
                        inactive_style
                    })
                    .show_colors(state.selected_widget == SelectedWidget::Log)
                    .render(log_inner_area, buf, &mut state.formatted_log_state);
            }
            SelectedTab::RawLog => {
                // Draw raw log
                RawLog::default()
                    .style(if state.selected_widget == SelectedWidget::Log {
                        active_style
                    } else {
                        inactive_style
                    })
                    .render(log_inner_area, buf, &mut state.raw_log_state);
            }
        }

        // Draw command input
        if let Some((command_input_state, _)) = state.command_input_state.as_mut() {
            let focused = state.selected_widget == SelectedWidget::CommandInput;
            let style = if focused {
                active_style
            } else {
                inactive_style
            };
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
    selected_tab: SelectedTab,
}

impl<'i> RootState<'i> {
    pub fn new(log: &'i Log, command_stdin: Option<EncodedWriter<ChildStdin>>) -> Self {
        RootState {
            raw_log_state: RawLogState::new(log),
            formatted_log_state: FormattedLogState::new(log),
            command_input_state: command_stdin.map(|stdin| (CommandInputState::default(), stdin)),
            controls_state: ControlsState::default(),
            selected_widget: SelectedWidget::default(),
            selected_tab: SelectedTab::default(),
        }
    }
}

impl<'i> State for RootState<'i> {
    fn update(&mut self, event: &AppEvent) -> bool {
        // TODO: mouse events
        // Update root state
        let mut handled = match event {
            AppEvent::TermEvent(Event::Key(key_event)) => match key_event.code {
                KeyCode::Tab if self.selected_widget == SelectedWidget::Log => {
                    self.selected_tab = match self.selected_tab {
                        SelectedTab::FormattedLog => SelectedTab::RawLog,
                        SelectedTab::RawLog => SelectedTab::FormattedLog,
                    };
                    true
                }
                KeyCode::BackTab if self.selected_widget == SelectedWidget::Log => {
                    self.selected_tab = match self.selected_tab {
                        SelectedTab::FormattedLog => SelectedTab::RawLog,
                        SelectedTab::RawLog => SelectedTab::FormattedLog,
                    };
                    true
                }
                KeyCode::Char('i') if self.selected_widget == SelectedWidget::Log => {
                    self.selected_widget = SelectedWidget::CommandInput;
                    true
                }
                KeyCode::Esc if self.selected_widget == SelectedWidget::CommandInput => {
                    self.selected_widget = SelectedWidget::Log;
                    true
                }
                _ => false,
            },
            _ => false,
        };

        // Update selected widget
        if !handled {
            handled = match self.selected_widget {
                SelectedWidget::Log => match self.selected_tab {
                    SelectedTab::FormattedLog => self.formatted_log_state.update(event),
                    SelectedTab::RawLog => self.raw_log_state.update(event),
                },
                SelectedWidget::CommandInput => self
                    .command_input_state
                    .as_mut()
                    .map_or(false, |(state, _)| state.update(event)),
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
            SelectedWidget::Log => {
                controls.insert(BindingDisplay::simple_key(KeyCode::Tab), "Next tab");
                controls.insert(BindingDisplay::simple_key(KeyCode::BackTab), "Previous tab");
                if self.command_input_state.is_some() {
                    controls.insert(BindingDisplay::simple_key(KeyCode::Char('i')), "Command");
                }

                match self.selected_tab {
                    SelectedTab::FormattedLog => self.formatted_log_state.add_controls(controls),
                    SelectedTab::RawLog => self.raw_log_state.add_controls(controls),
                }
            }
            SelectedWidget::CommandInput => {
                controls.insert(BindingDisplay::simple_key(KeyCode::Esc), "Back");
                if let Some((command_input_state, _)) = self.command_input_state.as_ref() {
                    command_input_state.add_controls(controls);
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
            selected_tab: self.selected_tab,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
enum SelectedTab {
    FormattedLog,
    RawLog,
}

impl Default for SelectedTab {
    fn default() -> Self {
        SelectedTab::FormattedLog
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
enum SelectedWidget {
    Log,
    CommandInput,
}

impl Default for SelectedWidget {
    fn default() -> Self {
        SelectedWidget::Log
    }
}
