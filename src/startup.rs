use crate::{
    encoded_writer::{ByteOrder, EncodedWriter},
    events::{AppEvent, EventController},
    log::Log,
    source::{FollowedLogSource, LogSource, ReaderLogSource, StaticLogSource},
    widgets::{Root, RootState, State, WithLog},
};
use anyhow::{bail, Context};
use clap::{command, Arg, ArgMatches, PossibleValue, ValueHint};
use crossterm::{
    event::{Event, KeyCode, KeyModifiers},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ouroboros::self_referencing;
use reqwest::blocking::Client;
use std::{
    io::{stdout, Write},
    path::PathBuf,
    process::{Child, ChildStdin, Stdio},
};
use tracing::{info, trace};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};
use tui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};

const LOG_ARG: &str = "log";
const FOLLOW_ARG: &str = "follow";
const STDIN_ARG: &str = "stdin";
const REMOTE_ARG: &str = "remote";
const SMAPI_ARG: &str = "execute";
const SMAPI_ARGS_ARG: &str = "execute-args";
const SMAPI_ENCODING_ARG: &str = "encoding";
const OUTPUT_LOG_ARG: &str = "output-log";

pub fn start() -> anyhow::Result<()> {
    // Parse options
    let matches = parse_args();

    // Setup tracing
    setup_tracing(&matches)?;

    // Spawn SMAPI if needed
    let smapi_child = if matches.is_present(SMAPI_ARG) {
        Some(spawn_smapi(&matches)?)
    } else {
        None
    };

    // Setup log source
    info!("Starting SMAPI Log Parser");
    let log_path = matches.value_of(LOG_ARG);
    let (source, log) = get_source(&matches, log_path)?;

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
    let result = render_loop(
        log,
        source,
        smapi_child
            .and_then(|child| child.stdin)
            .map(|stdin| create_encoded_writer(&matches, stdin)),
        &mut terminal,
    );

    // Exit alternate screen
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    crossterm::terminal::disable_raw_mode()?;
    result
}

fn spawn_smapi(matches: &ArgMatches) -> anyhow::Result<Child> {
    let smapi_path = matches.value_of(SMAPI_ARG).context("missing SMAPI path")?;
    let args = matches.values_of(SMAPI_ARGS_ARG).into_iter().flatten();
    let mut cmd = std::process::Command::new(smapi_path);
    let cmd = args.fold(&mut cmd, |cmd, arg| cmd.arg(arg));
    let child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .context("error starting SMAPI")?;
    Ok(child)
}

fn create_encoded_writer<W>(matches: &ArgMatches, writer: W) -> EncodedWriter<W>
where
    W: Write,
{
    match matches.value_of(SMAPI_ENCODING_ARG) {
        Some("utf8") => EncodedWriter::utf8(writer),
        Some("utf16-le") => EncodedWriter::utf16(writer, ByteOrder::LittleEndian),
        Some("utf16-be") => EncodedWriter::utf16(writer, ByteOrder::BigEndian),
        #[cfg(windows)]
        None => EncodedWriter::utf16(writer, ByteOrder::LittleEndian),
        #[cfg(not(windows))]
        None => EncodedWriter::utf8(writer),
        Some(encoding) => unreachable!("unexpected encoding {encoding}"),
    }
}

fn render_loop(
    log: Log,
    mut source: Box<dyn LogSource>,
    smapi_stdin: Option<EncodedWriter<ChildStdin>>,
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
) -> Result<(), anyhow::Error> {
    let mut force_redraw = true;
    let (event_rx, _event_controller) = EventController::start();
    let mut renderer = Renderer::from_log(log, smapi_stdin);
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
            .render(terminal, &event, force_redraw)
            .context("error rendering frame")?;
    }

    Ok(())
}

fn get_source(
    matches: &ArgMatches,
    log_path: Option<&str>,
) -> Result<(Box<dyn LogSource>, Log), anyhow::Error> {
    let stdin_flag = matches.is_present(STDIN_ARG);
    let follow_flag = matches.is_present(FOLLOW_ARG);
    let remote_flag = matches.is_present(REMOTE_ARG);
    let too_many_sources = [stdin_flag, follow_flag, remote_flag]
        .into_iter()
        .filter(|&f| f)
        .skip(1)
        .next()
        .is_some();
    if too_many_sources {
        bail!(
            "only one of --{}, --{}, --{}, --{} can be specified",
            STDIN_ARG,
            FOLLOW_ARG,
            REMOTE_ARG,
            SMAPI_ARG
        );
    }
    let (source, log): (Box<dyn LogSource>, Log) = if stdin_flag {
        let source = ReaderLogSource::from_stdin();
        let log = Log::empty();
        (Box::new(source), log)
    } else if follow_flag {
        let log_path = log_path
            .map(PathBuf::from)
            .or_else(default_log_path)
            .context("unable to find log path")?;
        let (source, log) =
            FollowedLogSource::new(log_path).context("error creating log source")?;
        (Box::new(source), log)
    } else if remote_flag {
        let log_path = log_path.context("unable to find log path")?;
        println!("Fetching remote log...");
        let contents = Client::new()
            .get(log_path)
            .send()
            .context("error retrieving remote log")?
            .text()
            .context("error reading remote log")?;
        let (source, log) =
            StaticLogSource::from_string(contents).context("error creating log source")?;
        (Box::new(source), log)
    } else {
        let log_path = log_path
            .map(PathBuf::from)
            .or_else(default_log_path)
            .context("unable to find log path")?;
        let (source, log) =
            StaticLogSource::from_file(&log_path).context("error creating log source")?;
        (Box::new(source), log)
    };
    Ok((source, log))
}

fn parse_args() -> ArgMatches {
    command!()
        .arg(
            Arg::new(LOG_ARG)
                .help("The path to the log file.")
                .index(1)
                .takes_value(true)
                .value_name("LOG PATH")
                .value_hint(ValueHint::FilePath),
        )
        .arg(
            Arg::new(OUTPUT_LOG_ARG)
                .help("The path to output this application's logs to (not SMAPI logs). Set RUST_LOG to configure the output.")
                .long("output-log")
                .takes_value(true)
                .value_name("OUTPUT LOG PATH")
                .value_hint(ValueHint::FilePath),
        )
        .arg(
            Arg::new(STDIN_ARG)
                .help("Read from stdin instead of a log file.")
                .long(STDIN_ARG)
        )
        .arg(
            Arg::new(FOLLOW_ARG)
                .long(FOLLOW_ARG)
                .help("Watch the log file for changes. This is not needed with --stdin.")
                .short('f'),
        )
        .arg(
            Arg::new(REMOTE_ARG)
                .help("Request the log from a remote source.")
                .long(REMOTE_ARG)
                .short('r'),
        )
        .arg(
            Arg::new(SMAPI_ARG)
                .help("Run SMAPI and track its output. The full SMAPI command must be provided (including any arguments to it).")
                .long(SMAPI_ARG)
                .visible_alias("exec")
                .short('e')
                .takes_value(true)
                .value_name("SMAPI CMD")
                .value_hint(ValueHint::CommandName),
        )
        .arg(
            Arg::new(SMAPI_ARGS_ARG)
                .help("The arguments to pass to the SMAPI command.")
                .index(2)
                .last(true)
                .multiple_values(true)
                .value_name("SMAPI ARGS")
                .value_hint(ValueHint::Unknown),
        )
        .arg(
            Arg::new(SMAPI_ENCODING_ARG)
                .help("The encoding to use when sending commands to SMAPI.")
                .long(SMAPI_ENCODING_ARG)
                .requires(SMAPI_ARG)
                .takes_value(true)
                .value_name("ENCODING")
                .possible_values([
                    PossibleValue::new("utf8").help("UTF-8"),
                    PossibleValue::new("utf16-le").help("UTF-16 little endian").alias("utf16"),
                    PossibleValue::new("utf16-be").help("UTF-16 big endian"),
                ])
                .value_hint(ValueHint::Other),
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
    pub fn from_log(log: Log, smapi_stdin: Option<EncodedWriter<ChildStdin>>) -> Self {
        Renderer::new(log, |log| Some(RootState::new(log, smapi_stdin)))
    }

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
                Ok(Renderer::new(new_log, |log| Some(root_state.with_log(log))))
            })
        } else {
            Ok(self)
        }
    }
}
