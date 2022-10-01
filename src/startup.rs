use crate::{
    config::{
        App, AppCommand, CommandEncoding, MonitorCommand, RemoteCommand, RunCommand, StdinCommand,
    },
    encoded_writer::{ByteOrder, EncodedWriter},
    events::{AppEvent, EventController},
    install_path::get_install_paths,
    log::Log,
    source::{FollowedLogSource, LogSource, ReaderLogSource, StaticLogSource},
    widgets::{Root, RootState, State, WithLog},
};
use anyhow::Context;
use crossterm::{
    event::{Event, KeyCode, KeyModifiers},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ouroboros::self_referencing;
use reqwest::blocking::Client;
use std::{
    ffi::OsStr,
    io::{stdout, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Stdio},
};
use tracing::{info, trace};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};
use tui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};

pub fn start(config: App) -> anyhow::Result<()> {
    // Setup tracing
    setup_tracing(config.output_log.as_ref().map(AsRef::as_ref))?;
    info!("starting pufferwatch");

    // Setup log source
    let (source, log, child_stdin) = get_source(config.command)?;

    // Initialize TUI
    trace!("initializing TUI");
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    // Prepare alternate screen
    trace!("entering alternate screen");
    terminal.backend_mut().execute(EnterAlternateScreen)?;
    crossterm::terminal::enable_raw_mode()?;
    terminal.hide_cursor()?;
    terminal.clear()?;

    // TUI event loop
    let result = render_loop(log, source, child_stdin, &mut terminal);

    // Exit alternate screen
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    crossterm::terminal::disable_raw_mode()?;
    result
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
        trace!("reading event");
        let event = event_rx.recv().context("error reading event")?;
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
    command: AppCommand,
) -> Result<(Box<dyn LogSource>, Log, Option<EncodedWriter<ChildStdin>>), anyhow::Error> {
    fn resolve_log_path(log_path: Option<PathBuf>) -> anyhow::Result<PathBuf> {
        log_path
            .map(PathBuf::from)
            .or_else(default_log_path)
            .context("unable to find log path")
    }

    Ok(match command {
        AppCommand::Monitor(MonitorCommand { log: path, follow }) => {
            let log_path = resolve_log_path(path)?;
            if follow {
                let (source, log) =
                    FollowedLogSource::new(log_path).context("error creating log source")?;
                (Box::new(source), log, None)
            } else {
                let (source, log) =
                    StaticLogSource::from_file(&log_path).context("error creating log source")?;
                (Box::new(source), log, None)
            }
        }
        AppCommand::Stdin(StdinCommand) => {
            let source = ReaderLogSource::from_stdin();
            let log = Log::empty();
            (Box::new(source), log, None)
        }
        AppCommand::Remote(RemoteCommand { url }) => {
            println!("Fetching remote log...");
            info!("fetching remote log");
            let contents = Client::new()
                .get(url)
                .send()
                .context("error retrieving remote log")?
                .text()
                .context("error reading remote log")?;
            let (source, log) =
                StaticLogSource::from_string(contents).context("error creating log source")?;
            (Box::new(source), log, None)
        }
        AppCommand::Run(RunCommand {
            smapi_path,
            smapi_args,
            log,
            encoding,
        }) => {
            // Start SMAPI
            let smapi_path = smapi_path
                .or_else(|| get_install_paths().into_iter().next().map(executable_path))
                .context("unable to find game path")?;
            info!(smapi_path=%smapi_path.display(), "starting SMAPI");
            let process = spawn_smapi(&smapi_path, smapi_args.iter().map(AsRef::as_ref))?;

            // Follow log file
            let log_path = resolve_log_path(log)?;
            let (source, log) =
                FollowedLogSource::new(log_path).context("error creating log source")?;
            (
                Box::new(source),
                log,
                process
                    .stdin
                    .map(|stdin| create_encoded_writer(stdin, encoding)),
            )
        }
    })
}

#[cfg(windows)]
fn executable_path(install_path: impl AsRef<Path>) -> PathBuf {
    install_path.as_ref().join("StardewModdingAPI.exe")
}

#[cfg(unix)]
fn executable_path(install_path: impl AsRef<Path>) -> PathBuf {
    install_path.as_ref().join("StardewValley")
}

fn spawn_smapi<'a>(
    smapi_path: &'a Path,
    args: impl IntoIterator<Item = &'a OsStr>,
) -> anyhow::Result<Child> {
    let mut cmd = std::process::Command::new(smapi_path);
    let cmd = args.into_iter().fold(&mut cmd, |cmd, arg| cmd.arg(arg));
    let child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .context("error starting SMAPI")?;
    Ok(child)
}

fn create_encoded_writer<W>(writer: W, encoding: CommandEncoding) -> EncodedWriter<W>
where
    W: Write,
{
    match encoding {
        CommandEncoding::Utf8 => EncodedWriter::utf8(writer),
        CommandEncoding::Utf16Le => EncodedWriter::utf16(writer, ByteOrder::LittleEndian),
        CommandEncoding::Utf16Be => EncodedWriter::utf16(writer, ByteOrder::BigEndian),
    }
}

fn setup_tracing(log_path: Option<&Path>) -> anyhow::Result<()> {
    if let Some(log_path) = log_path {
        if let Some(parent_dir) = log_path.parent() {
            std::fs::create_dir_all(parent_dir).with_context(|| {
                format!(
                    "failed to create output logs directory: {}",
                    parent_dir.display()
                )
            })?;
        }
        let log_file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(log_path)
            .with_context(|| format!("failed to open output logs file: {}", log_path.display()))?;
        let fmt_layer = tracing_subscriber::fmt::layer()
            .compact()
            .with_ansi(false)
            .with_writer(log_file);
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
