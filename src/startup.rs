use crate::{
    events::{AppEvent, EventController},
    log::Log,
    source::{FollowedLogSource, LogSource, StaticLogSource, StdinLogSource},
    widgets::{Root, RootState, State, WithLog},
};
use anyhow::Context;
use clap::{crate_authors, crate_description, crate_version, App, Arg, ArgMatches};
use crossterm::{
    event::{Event, KeyCode, KeyModifiers},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ouroboros::self_referencing;
use std::{io::stdout, path::PathBuf, time::Duration};
use tracing::{info, trace};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};
use tui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};

const LOG_ARG: &str = "log";
const FOLLOW_ARG: &str = "follow";
const STDIN_ARG: &str = "stdin";
const OUTPUT_LOG_ARG: &str = "output-log";

pub fn start() -> anyhow::Result<()> {
    // Parse options
    let matches = parse_args();

    // Setup tracing
    setup_tracing(&matches)?;

    // Setup log source
    info!("Starting SMAPI Log Parser");
    let log_path = matches
        .value_of(LOG_ARG)
        .map(PathBuf::from)
        .or_else(default_log_path)
        .context("unable to find log path")?;
    let (mut source, log): (Box<dyn LogSource>, Log) = if matches.is_present(STDIN_ARG) {
        let source = StdinLogSource::new();
        let log = Log::parse(String::new()).context("error parsing empty log")?;
        (Box::new(source), log)
    } else if matches.is_present(FOLLOW_ARG) {
        let (source, log) =
            FollowedLogSource::new(log_path).context("error creating log source")?;
        (Box::new(source), log)
    } else {
        let (source, log) = StaticLogSource::new(&log_path).context("error creating log source")?;
        (Box::new(source), log)
    };

    // Initialize TUI
    trace!("Initializing TUI");
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    // Prepare alternate screen
    trace!("Entering alternate screen");
    terminal.backend_mut().execute(EnterAlternateScreen)?;
    crossterm::terminal::enable_raw_mode()?;
    terminal.hide_cursor()?;
    terminal.clear()?;

    // TUI event loop
    let mut force_redraw = true;
    let (event_rx, _event_controller) = EventController::start();
    let mut renderer = Renderer::from(log);
    loop {
        // Read event
        trace!("Reading event");
        let event = event_rx.recv().context("Error reading event")?;
        match event {
            // Check if quitting
            AppEvent::TermEvent(Event::Key(key_event)) => {
                if key_event.code == KeyCode::Char('c')
                    && key_event.modifiers == KeyModifiers::CONTROL
                {
                    // Quit
                    break;
                }
            }
            // Check for resize
            AppEvent::TermEvent(Event::Resize(_, _)) => {
                force_redraw = true;
            }
            _ => {}
        }

        // Update log from source if needed
        renderer = renderer
            .update_from(source.as_mut())
            .context("error updating renderer with new log")?;

        // Draw terminal
        renderer
            .render(&mut terminal, &event, force_redraw)
            .context("error rendering frame")?;
    }

    // Exit alternate screen
    std::thread::sleep(Duration::from_secs(2));
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    crossterm::terminal::disable_raw_mode()?;
    Ok(())
}

fn parse_args() -> ArgMatches<'static> {
    App::new("Pufferwatch")
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .arg(
            Arg::with_name(LOG_ARG)
                .help("The path to the log file.")
                .index(1)
                .takes_value(true)
                .value_name("LOG PATH"),
        )
        .arg(
            Arg::with_name(STDIN_ARG)
                .help("Read from stdin instead of a log file.")
                .long("stdin")
        )
        .arg(
            Arg::with_name(FOLLOW_ARG)
                .long("follow")
                .help("Watch the log file for changes. This is not needed with --stdin.")
                .short("f"),
        )
        .arg(
            Arg::with_name(OUTPUT_LOG_ARG)
                .help("The path to output this application's logs to (not SMAPI logs). Set RUST_LOG to configure the output.")
                .long("output-log")
                .takes_value(true)
                .value_name("OUTPUT LOG PATH"),
        )
        .get_matches()
}

fn setup_tracing(matches: &ArgMatches) -> anyhow::Result<()> {
    if let Some(output_logs) = matches.value_of(OUTPUT_LOG_ARG) {
        let log_path = PathBuf::from(output_logs);
        if let Some(parent_dir) = log_path.parent() {
            std::fs::create_dir_all(parent_dir).with_context(|| {
                format!(
                    "Failed to create output logs directory: {}",
                    parent_dir.display()
                )
            })?;
        }
        let output_logs = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(&log_path)
            .with_context(|| format!("Failed to open output logs file: {}", log_path.display()))?;
        let fmt_layer = tracing_subscriber::fmt::layer()
            .compact()
            .with_ansi(false)
            .with_writer(output_logs);
        Registry::default()
            .with(EnvFilter::from_default_env())
            .with(fmt_layer)
            .try_init()
            .context("error initializing tracing")
    } else {
        Registry::default()
            .try_init()
            .context("error initializing tracing")
    }
}

fn default_log_path() -> Option<PathBuf> {
    #[cfg(not(target_os = "macos"))]
    let mut path = dirs::config_dir()?;

    #[cfg(target_os = "macos")]
    let mut path = {
        let mut path = dirs::home_dir()?;
        path.push(".config");
        path
    };

    path.push("StardewValley/ErrorLogs/SMAPI-latest.txt");
    Some(path)
}

#[self_referencing]
struct Renderer {
    log: Log,
    #[borrows(log)]
    #[covariant]
    root_state: Option<RootState<'this>>,
}

impl Renderer {
    pub fn render<'t, B: Backend>(
        &mut self,
        terminal: &'t mut Terminal<B>,
        event: &AppEvent,
        force_redraw: bool,
    ) -> anyhow::Result<()> {
        self.with_root_state_mut(|root_state| {
            let root_state = root_state.as_mut().context("missing root state")?;
            if root_state.update(event) || force_redraw {
                terminal
                    .draw(|f| f.render_stateful_widget(Root::default(), f.size(), root_state))
                    .context("error rendering frame")?;
            }

            Ok(())
        })
    }

    pub fn update_from(mut self, source: &mut dyn LogSource) -> anyhow::Result<Self> {
        let new_log = self.with_log(|log| source.update_log(log))?;
        if let Some(new_log) = new_log {
            self.with_root_state_mut(|root_state| {
                let root_state = root_state.take().context("missing root state")?;
                Ok(RendererBuilder {
                    log: new_log,
                    root_state_builder: |log| Some(root_state.with_log(log)),
                }
                .build())
            })
        } else {
            Ok(self)
        }
    }
}

impl From<Log> for Renderer {
    fn from(log: Log) -> Self {
        RendererBuilder {
            log,
            root_state_builder: |log| Some(RootState::new(log)),
        }
        .build()
    }
}
