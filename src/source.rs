use crate::log::Log;
use anyhow::Context;
use crossbeam::channel::Receiver;
use hotwatch::{Event, Hotwatch};
use std::{
    fmt::Debug,
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::JoinHandle,
};
use tracing::debug;

pub trait LogSource {
    fn update_log(&mut self, log: &Log) -> anyhow::Result<Option<Log>>;
}

#[derive(Debug)]
pub struct StaticLogSource;

impl StaticLogSource {
    /// Creates a new static log source from a file path.
    pub fn from_file(path: &Path) -> anyhow::Result<(Self, Log)> {
        Log::parse_file(path)
            .map(|log| (StaticLogSource, log))
            .context("Error parsing log")
    }

    /// Creates a new static log source from a string.
    pub fn from_string(raw: String) -> anyhow::Result<(Self, Log)> {
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

#[derive(Debug)]
pub struct FollowedLogSource {
    path: PathBuf,
    _hotwatch: Hotwatch,
    needs_reload: Arc<AtomicBool>,
}

impl FollowedLogSource {
    pub fn new(path: PathBuf) -> anyhow::Result<(Self, Log)> {
        // Create file watcher
        let needs_reload = Arc::new(AtomicBool::new(false));
        let mut hotwatch = Hotwatch::new().context("error initializing file watcher")?;
        hotwatch
            .watch(&path, {
                let needs_reload = needs_reload.clone();
                move |event| match event {
                    Event::NoticeWrite(_) | Event::Create(_) | Event::Write(_) => {
                        needs_reload.store(true, Ordering::Relaxed);
                    }
                    Event::Remove(_) => {
                        needs_reload.store(false, Ordering::Relaxed);
                    }
                    _ => {}
                }
            })
            .context("error watching file")?;

        // Parse log
        let log = Log::parse_file(&path).context("error parsing log file")?;
        let source = FollowedLogSource {
            path,
            _hotwatch: hotwatch,
            needs_reload,
        };
        Ok((source, log))
    }
}

impl LogSource for FollowedLogSource {
    fn update_log(&mut self, _log: &Log) -> anyhow::Result<Option<Log>> {
        if self.needs_reload.load(Ordering::Relaxed) {
            if let Ok(log) = Log::parse_file(&self.path) {
                return Ok(Some(log));
            }
        }

        Ok(None)
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
