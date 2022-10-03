use crate::log::Log;
use anyhow::Context;
use crossbeam::channel::Receiver;
use notify::{
    event::{MetadataKind, ModifyKind},
    Config, Event, EventKind, PollWatcher, RecursiveMode, Watcher,
};
use std::{
    fmt::Debug,
    fs::File,
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
    thread::JoinHandle,
    time::Duration,
};
use tracing::{debug, debug_span, info, instrument, trace, warn};

pub trait LogSource {
    fn update_log(&mut self, log: &Log) -> anyhow::Result<Option<Log>>;
}

#[derive(Debug)]
pub struct StaticLogSource;

impl StaticLogSource {
    /// Creates a new static log source from a file path.
    #[instrument(skip_all)]
    pub fn from_file(path: &Path) -> anyhow::Result<(Self, Log)> {
        info!(?path, "creating static log source");

        let file = File::open(path).context("failed to open log file")?;
        Log::parse_reader(file)
            .map(|log| (StaticLogSource, log))
            .context("Error parsing log")
    }

    /// Creates a new static log source from a string.
    #[instrument(skip_all)]
    pub fn from_string(raw: String) -> anyhow::Result<(Self, Log)> {
        info!(len=%raw.len(), "creating static log source");

        Log::parse(raw)
            .map(|log| (StaticLogSource, log))
            .context("Error parsing log")
    }
}

impl LogSource for StaticLogSource {
    fn update_log(&mut self, _log: &Log) -> anyhow::Result<Option<Log>> {
        Ok(None)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
enum FileUpdate {
    Removed,
    Updated,
}

#[derive(Debug)]
pub struct FollowedLogSource {
    path: PathBuf,
    _watcher: PollWatcher,
    rx: Receiver<FileUpdate>,
}

impl FollowedLogSource {
    pub fn new(path: PathBuf) -> anyhow::Result<(Self, Log)> {
        info!(?path, "creating followed log source");

        // Create file watcher
        let (tx, rx) = crossbeam::channel::bounded(10);
        let mut watcher = PollWatcher::new(
            {
                let path = path.clone();
                move |event| {
                    let _span = debug_span!("file_watcher", ?path, watcher_event=?event).entered();

                    // Get event
                    let event: Event = match event {
                        Ok(event) => event,
                        Err(error) => {
                            warn!(?error, "error watching log file");
                            return;
                        }
                    };
                    trace!("received file watcher event");

                    // Handle event
                    match event.kind {
                        EventKind::Remove(_) => drop(tx.send(FileUpdate::Removed)),
                        EventKind::Create(_)
                        | EventKind::Modify(ModifyKind::Metadata(MetadataKind::WriteTime))
                        | EventKind::Modify(ModifyKind::Metadata(MetadataKind::Any))
                        | EventKind::Modify(ModifyKind::Data(_))
                        | EventKind::Modify(ModifyKind::Any)
                        | EventKind::Any => drop(tx.send(FileUpdate::Updated)),
                        _ => {}
                    }
                }
            },
            Config::default()
                .with_poll_interval(Duration::from_secs(2))
                .with_compare_contents(true),
        )
        .context("error creating file watcher")?;
        watcher
            .watch(&path, RecursiveMode::NonRecursive)
            .context("error starting file watcher")?;

        // Parse log
        let file = File::open(&path).context("failed to open log file")?;
        let log = Log::parse_reader(file).context("error parsing log file")?;
        let source = FollowedLogSource {
            path,
            _watcher: watcher,
            rx,
        };
        Ok((source, log))
    }
}

impl LogSource for FollowedLogSource {
    #[instrument(skip_all, fields(path=?self.path))]
    fn update_log(&mut self, _log: &Log) -> anyhow::Result<Option<Log>> {
        macro_rules! try_or_warn {
            ($f:expr, $prev:expr, $($args:tt)*) => {
                match $f {
                    Ok(value) => value,
                    Err(_) => {
                        warn!($($args)*);
                        return Ok($prev);
                    }
                }
            }
        }

        // Check for updates
        self.rx.try_iter().try_fold(None, |new_log, event| {
            let _span = debug_span!("file_event", file_event=?event).entered();
            trace!("handling file event");
            match event {
                FileUpdate::Removed => {
                    // Reset
                    Ok(Some(Log::default()))
                }
                FileUpdate::Updated => {
                    // Open file and measure size
                    let file =
                        try_or_warn!(File::open(&self.path), new_log, "failed to open log file");

                    // Parse log
                    let log =
                        try_or_warn!(Log::parse_reader(file), new_log, "error parsing log file");

                    Ok(Some(log))
                }
            }
        })
    }
}

#[derive(Debug)]
pub struct ReaderLogSource {
    unparsed: String,
    rx: Receiver<anyhow::Result<String>>,
    _reader_thread: JoinHandle<()>,
}

impl ReaderLogSource {
    pub fn new<R: Read + Send + 'static>(reader: R) -> Self {
        let (tx, rx) = crossbeam::channel::unbounded::<anyhow::Result<String>>();
        let mut reader = BufReader::new(reader);
        let reader_thread = std::thread::spawn(move || loop {
            let mut buffer = String::new();
            match reader.read_line(&mut buffer) {
                Ok(0) => continue,
                Ok(_) => tx.send(Ok(buffer)).unwrap(),
                Err(error) => tx.send(Err(error.into())).unwrap(),
            }
        });

        Self {
            unparsed: String::new(),
            rx,
            _reader_thread: reader_thread,
        }
    }

    pub fn from_stdin() -> Self {
        ReaderLogSource::new(std::io::stdin())
    }
}

impl LogSource for ReaderLogSource {
    fn update_log(&mut self, log: &Log) -> anyhow::Result<Option<Log>> {
        // Try to get the next line
        let first_line = match self.rx.try_recv() {
            Ok(line) => line?,
            Err(_) => return Ok(None),
        };
        self.unparsed.push_str(&first_line);

        // Append to the unparsed buffer
        self.unparsed.push_str(&first_line);
        while let Ok(line) = self.rx.try_recv() {
            self.unparsed.push_str(&line?);
        }

        // Append to the log
        let mut raw = log.raw().to_string();
        raw.push_str(&self.unparsed);
        if let Ok(log) = Log::parse(raw) {
            self.unparsed.clear();
            Ok(Some(log))
        } else {
            debug!(?self.unparsed, "Unable to parse");
            Ok(None)
        }
    }
}
