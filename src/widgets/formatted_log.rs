use crate::{
    ast::{Level, Message},
    events::AppEvent,
    log::Log,
    widgets::{BindingDisplay, IconPack, LazyParagraph, LazyParagraphState, State, WithLog},
};
use crossterm::event::{Event, KeyCode};
use indexmap::IndexMap;
use itertools::{Either, Itertools};
use tracing::trace;
use tui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Span, Spans},
    widgets::{Block, StatefulWidget},
};
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Debug, Default)]
pub struct FormattedLog<'i> {
    block: Option<Block<'i>>,
    default_style: Style,
    show_colors: bool,
}

impl<'i> FormattedLog<'i> {
    #[allow(dead_code)]
    pub fn block(mut self, block: Block<'i>) -> Self {
        self.block = Some(block);
        self
    }

    pub fn default_style(mut self, style: Style) -> Self {
        self.default_style = style;
        self
    }

    pub fn show_colors(mut self, show_colors: bool) -> Self {
        self.show_colors = show_colors;
        self
    }

    fn get_level_color(level: Level) -> Color {
        match level {
            Level::Trace | Level::Debug => Color::DarkGray,
            Level::Info => Color::White,
            Level::Alert => Color::Magenta,
            Level::Warn => Color::Yellow,
            Level::Error => Color::Red,
        }
    }

    fn render_logs(
        &self,
        area: Rect,
        buf: &mut Buffer,
        state: &mut <Self as StatefulWidget>::State,
    ) {
        let style_override = (!self.show_colors).then_some(self.default_style);
        let paragraph = LazyParagraph::new(|index| {
            let formatted_line = state.lines.get(index)?;
            let spans = match *formatted_line {
                FormattedLine::Start { message, line } => {
                    let mut spans = Vec::with_capacity(7);

                    // Timestamp
                    spans.push(Span::styled(
                        format!("{}", message.timestamp),
                        self.default_style,
                    ));

                    // Padding
                    spans.push(Span::styled(" ", self.default_style));

                    // Level
                    let level_style = style_override.unwrap_or_else(|| {
                        self.default_style.fg(Self::get_level_color(message.level))
                    });
                    spans.push(Span::styled(format!("{:5}", message.level), level_style));

                    // Padding
                    spans.push(Span::styled(" ", self.default_style));

                    // Source
                    spans.push(Span::styled(
                        message.source.as_ref(),
                        style_override.unwrap_or_else(|| self.default_style.fg(Color::Green)),
                    ));

                    // Padding
                    spans.push(Span::styled(
                        " ".repeat(
                            state
                                .source_width
                                .saturating_sub(message.source.len())
                                .saturating_add(1),
                        ),
                        self.default_style,
                    ));

                    // Message
                    spans.push(Span::styled(line, level_style));

                    spans
                }
                FormattedLine::Continued { message, line } => {
                    let mut spans = Vec::with_capacity(2);
                    let ellipsis_style =
                        style_override.unwrap_or_else(|| self.default_style.fg(Color::DarkGray));

                    // Timestamp (8)
                    spans.push(Span::styled("...     ", ellipsis_style));

                    // Padding (1)
                    spans.push(Span::raw(" "));

                    // Level (5)
                    spans.push(Span::styled("...  ", ellipsis_style));

                    // Padding (1)
                    spans.push(Span::raw(" "));

                    // Source (source_width)
                    spans.push(Span::styled("...", ellipsis_style));
                    spans.push(Span::raw(" ".repeat(state.source_width.saturating_sub(3))));

                    // Padding (1)
                    spans.push(Span::raw(" "));

                    // Message
                    spans.push(Span::styled(
                        line,
                        style_override.unwrap_or_else(|| {
                            self.default_style.fg(Self::get_level_color(message.level))
                        }),
                    ));

                    spans
                }
            };

            Some(spans.into())
        })
        .style(self.default_style.bg(Color::Black));
        let paragraph = if let Some(block) = self.block.clone() {
            paragraph.block(block)
        } else {
            paragraph
        };
        paragraph.render(area, buf, &mut state.paragraph_state);
    }
}

impl<'i> StatefulWidget for FormattedLog<'i> {
    type State = FormattedLogState<'i>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if state.filters_list_state.is_none() {
            // Logs only
            self.render_logs(area, buf, state);
        } else {
            // Logs + filters
            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0), Constraint::Length(1)].as_ref())
                .split(area);
            self.render_logs(layout[0], buf, state);
            let filters_list_state = state.filters_list_state.as_mut().unwrap();
            let style_override = (!self.show_colors).then_some(self.default_style);
            FiltersList::new(&mut state.filters)
                .style(
                    style_override
                        .unwrap_or_else(|| self.default_style.fg(Color::Black).bg(Color::White)),
                )
                .selected_style(
                    style_override
                        .unwrap_or_else(|| self.default_style.fg(Color::White).bg(Color::LightRed)),
                )
                .enabled_style(
                    style_override.unwrap_or_else(|| {
                        self.default_style.fg(Color::Black).bg(Color::LightGreen)
                    }),
                )
                .more_label_style(self.default_style.fg(Color::White))
                .render(layout[1], buf, filters_list_state);
        }
    }
}

#[derive(Clone, Debug)]
pub struct FormattedLogState<'i> {
    log: &'i Log,
    lines: Vec<FormattedLine<'i>>,
    source_width: usize,
    paragraph_state: LazyParagraphState,
    filters: LogFilters<'i>,
    filters_list_state: Option<FiltersListState>,
}

impl<'i> FormattedLogState<'i> {
    pub fn new(log: &'i Log) -> Self {
        let filters = LogFilters {
            levels: Level::ALL.into_iter().map(|level| (level, true)).collect(),
            sources: log
                .sources()
                .into_iter()
                .sorted()
                .map(|source| (source, true))
                .collect(),
        };
        let (lines, source_width) = Self::format_lines(log, filters.clone());
        let paragraph_state = LazyParagraphState::new(lines.len(), true);
        Self {
            log,
            lines,
            source_width,
            paragraph_state,
            filters,
            filters_list_state: None,
        }
    }

    pub fn apply_filter(&mut self) {
        let (lines, source_width) = Self::format_lines(self.log, self.filters.clone());
        self.lines = lines;
        self.source_width = source_width;
        trace!(lines=%self.lines.len(), max_source_width=%self.source_width, "Applied filter to formatted log");

        // TODO: set the offset to the line closest to the current line's offset
        let auto_scroll = self.paragraph_state.auto_scroll;
        self.paragraph_state = LazyParagraphState::new(self.lines.len(), true);
        self.paragraph_state.auto_scroll = auto_scroll;
    }

    fn format_lines(log: &'i Log, filters: LogFilters<'i>) -> (Vec<FormattedLine<'i>>, usize) {
        let mut lines = Vec::new();
        let mut source_width = 0;
        for message in filters.apply(log) {
            // Source width
            let source = message.source.as_ref();
            source_width = source_width.max(source.len());

            // Formatted lines
            let mut first_line = true;
            for contents in message.contents.lines() {
                if first_line {
                    first_line = false;
                    lines.push(FormattedLine::Start {
                        message,
                        line: contents,
                    });
                } else {
                    lines.push(FormattedLine::Continued {
                        message,
                        line: contents,
                    });
                }
            }
        }

        (lines, source_width)
    }
}

impl<'i> State for FormattedLogState<'i> {
    fn update(&mut self, event: &AppEvent) -> bool {
        // Events handled by the formatted log widget
        #[allow(clippy::single_match)] // TODO: Add mouse support
        match *event {
            AppEvent::TermEvent(Event::Key(key_event)) => match key_event.code {
                KeyCode::Char('f') if self.filters_list_state.is_none() => {
                    self.filters_list_state = Some(FiltersListState::levels());
                    return true;
                }
                KeyCode::Char('f') => {
                    match self.filters_list_state.take().unwrap().source {
                        FiltersListSource::Levels => {
                            self.filters_list_state = Some(FiltersListState::sources());
                        }
                        FiltersListSource::Sources => {
                            self.filters_list_state = Some(FiltersListState::levels());
                        }
                    }
                    return true;
                }
                KeyCode::Char(' ') if self.filters_list_state.is_some() => {
                    self.filters_list_state
                        .as_ref()
                        .unwrap()
                        .toggle(&mut self.filters);
                    self.apply_filter();
                    return true;
                }
                KeyCode::Esc if self.filters_list_state.is_some() => {
                    self.filters_list_state = None;
                    return true;
                }
                _ => {}
            },
            _ => {}
        }

        // Children events
        match self.filters_list_state.as_mut() {
            None => self.paragraph_state.update(event),
            Some(filters_list_state) => filters_list_state.update(event),
        }
    }

    fn add_controls<I: IconPack>(&self, controls: &mut IndexMap<BindingDisplay<I>, &'static str>) {
        match self.filters_list_state.as_ref() {
            None => {
                controls.insert(BindingDisplay::simple_key(KeyCode::Char('f')), "Filters");
                self.paragraph_state.add_controls(controls);
            }
            Some(filters_list_state) => {
                controls.insert(BindingDisplay::simple_key(KeyCode::Char('f')), "Next");
                controls.insert(BindingDisplay::simple_key(KeyCode::Char(' ')), "Toggle");
                controls.insert(BindingDisplay::simple_key(KeyCode::Esc), "Close");
                filters_list_state.add_controls(controls);
            }
        }
    }
}

impl<'i, 'j> WithLog<'j> for FormattedLogState<'i> {
    type Result = FormattedLogState<'j>;

    fn with_log(self, log: &'j Log) -> Self::Result {
        let filters = self.filters.with_log(log);
        let (lines, source_width) = FormattedLogState::format_lines(log, filters.clone());
        let mut paragraph_state = LazyParagraphState::new(lines.len(), true);
        paragraph_state.offset = self.paragraph_state.offset;
        paragraph_state.auto_scroll = self.paragraph_state.auto_scroll;
        FormattedLogState {
            log,
            filters,
            filters_list_state: self.filters_list_state.with_log(log),
            lines,
            source_width,
            paragraph_state,
        }
    }
}

#[derive(Clone, Debug)]
enum FormattedLine<'i> {
    Start {
        message: &'i Message<'i>,
        line: &'i str,
    },
    Continued {
        message: &'i Message<'i>,
        line: &'i str,
    },
}

#[derive(Clone, Debug)]
pub struct LogFilters<'i> {
    pub levels: IndexMap<Level, bool>,
    pub sources: IndexMap<&'i str, bool>,
}

impl<'i> LogFilters<'i> {
    /// Checks if a level is enabled for this log.
    pub fn level_enabled(&self, level: Level) -> bool {
        self.levels.get(&level).copied().unwrap_or(true)
    }

    /// Checks if a source is enabled for this log.
    pub fn source_enabled(&self, source: &'i str) -> bool {
        self.sources.get(source).copied().unwrap_or(true)
    }

    /// Applies the filters to the given log.
    pub fn apply(self, log: &'i Log) -> impl IntoIterator<Item = &'i Message<'i>> {
        log.messages().iter().filter(move |&message| {
            self.level_enabled(message.level) && self.source_enabled(message.source.as_ref())
        })
    }
}

impl<'i, 'j> WithLog<'j> for LogFilters<'i> {
    type Result = LogFilters<'j>;

    fn with_log(self, log: &'j Log) -> Self::Result {
        LogFilters {
            levels: self.levels,
            sources: log
                .sources()
                .into_iter()
                .sorted()
                .map(|source| (source, self.sources.get(source).copied().unwrap_or(true)))
                .collect(),
        }
    }
}

#[derive(Debug)]
struct FiltersList<'f, 'i: 'f> {
    style: Style,
    selected_style: Style,
    enabled_style: Style,
    more_label_style: Style,
    filters: &'f mut LogFilters<'i>,
}

impl<'f, 'i: 'f> FiltersList<'f, 'i> {
    pub fn new(filters: &'f mut LogFilters<'i>) -> Self {
        Self {
            style: Style::default(),
            selected_style: Style::default(),
            enabled_style: Style::default(),
            more_label_style: Style::default(),
            filters,
        }
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn selected_style(mut self, style: Style) -> Self {
        self.selected_style = style;
        self
    }

    pub fn enabled_style(mut self, style: Style) -> Self {
        self.enabled_style = style;
        self
    }

    pub fn more_label_style(mut self, style: Style) -> Self {
        self.more_label_style = style;
        self
    }
}

impl<'f, 'i: 'f> StatefulWidget for FiltersList<'f, 'i> {
    type State = FiltersListState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        // Get labels for each control
        let labels = match &state.source {
            FiltersListSource::Levels => {
                state.selected = state.selected.min(Level::ALL.len().saturating_sub(1));
                Either::Left(self.filters.levels.iter().enumerate().map(
                    |(index, (&level, &enabled))| {
                        Span::styled(
                            level.to_string(),
                            if state.selected == index {
                                self.selected_style
                            } else if enabled {
                                self.enabled_style
                            } else {
                                self.style
                            },
                        )
                    },
                ))
            }
            FiltersListSource::Sources => {
                state.selected = state
                    .selected
                    .min(self.filters.sources.len().saturating_sub(1));
                Either::Right(self.filters.sources.iter().enumerate().map(
                    |(index, (&source, &enabled))| {
                        Span::styled(
                            source,
                            if state.selected == index {
                                self.selected_style
                            } else if enabled {
                                self.enabled_style
                            } else {
                                self.style
                            },
                        )
                    },
                ))
            }
        };
        let more_label = Span::styled("...", self.more_label_style);

        // Get the available width for the controls, excluding the "More" label
        let controls_width = usize::from(area.width).saturating_sub(more_label.content.width() * 2);
        if controls_width == 0 {
            return;
        }

        // Group controls into lines
        let mut start_line = 0_usize;
        let lines = labels
            .enumerate()
            .map(|(index, label)| {
                let label_width = label.content.width();
                (index, label, label_width)
            })
            .peekable()
            .batching(|labels| {
                let mut remaining_width = controls_width;
                let mut line = Vec::new();
                while let Some(&(index, _, label_width)) = labels.peek() {
                    // Check if the label fits on the current line
                    if let Some(new_remaining_width) = remaining_width.checked_sub(label_width) {
                        // Label fits on the current line
                        remaining_width = new_remaining_width.saturating_sub(1);

                        // Add "More" label (for previous page)
                        if index > 0 && line.is_empty() {
                            line.push(more_label.clone());
                        }

                        // Add label and padding
                        let (_, label, _) = labels.next().unwrap();
                        line.push(label);
                        line.push(Span::raw(" "));
                    } else {
                        // Check if empty page because area isn't big enough
                        if line.is_empty() {
                            return None;
                        }

                        // Track the start line
                        if index <= state.selected {
                            start_line = start_line.saturating_add(1);
                        }

                        // Add "More" label (for next page)
                        line.push(more_label.clone());
                        return Some(Spans::from(line));
                    }
                }

                if line.is_empty() {
                    None
                } else {
                    Some(Spans::from(line))
                }
            })
            .collect_vec();

        // TODO: Get which rows to render
        // let start_row = start_line * usize::from(area.height);
        let start_row = start_line;
        let line_after_end_row = start_row
            .saturating_add(area.height.into())
            .min(lines.len());
        let start_row = line_after_end_row.saturating_sub(area.height.into());

        // Render the controls
        let rendered_lines = lines.get(start_row..line_after_end_row);
        for (y, spans) in rendered_lines.into_iter().flatten().enumerate() {
            buf.set_spans(area.x, area.y.saturating_add(y as u16), spans, area.width);
        }
    }
}

#[derive(Clone, Debug)]
struct FiltersListState {
    selected: usize,
    source: FiltersListSource,
}

impl FiltersListState {
    pub fn levels() -> Self {
        Self {
            selected: 0,
            source: FiltersListSource::Levels,
        }
    }

    pub fn sources() -> Self {
        Self {
            selected: 0,
            source: FiltersListSource::Sources,
        }
    }

    pub fn toggle<'i>(&self, filters: &mut LogFilters<'i>) {
        match &self.source {
            FiltersListSource::Levels => {
                if let Some((_, enabled)) = filters.levels.get_index_mut(self.selected) {
                    *enabled = !*enabled;
                }
            }
            FiltersListSource::Sources => {
                if let Some((_, enabled)) = filters.sources.get_index_mut(self.selected) {
                    *enabled = !*enabled;
                }
            }
        }
    }
}

impl State for FiltersListState {
    fn update(&mut self, event: &AppEvent) -> bool {
        match event {
            AppEvent::TermEvent(Event::Key(key_event)) => match key_event.code {
                KeyCode::Left => {
                    self.selected = self.selected.saturating_sub(1);
                    true
                }
                KeyCode::Right => {
                    self.selected = self.selected.saturating_add(1);
                    true
                }
                _ => false,
            },
            _ => false,
        }
    }

    fn add_controls<I: IconPack>(&self, controls: &mut IndexMap<BindingDisplay<I>, &'static str>) {
        controls.insert(BindingDisplay::Custom(I::LEFT_RIGHT), "Nav");
    }
}

impl<'j> WithLog<'j> for FiltersListState {
    type Result = Self;

    fn with_log(self, _log: &'j Log) -> Self::Result {
        match self.source {
            FiltersListSource::Levels => FiltersListState {
                selected: self.selected,
                source: FiltersListSource::Levels,
            },
            FiltersListSource::Sources => FiltersListState {
                selected: self.selected,
                source: FiltersListSource::Sources,
            },
        }
    }
}

#[derive(Clone, Debug)]
enum FiltersListSource {
    Levels,
    Sources,
}
