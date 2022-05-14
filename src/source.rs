use crate::log::Log;
use anyhow::Context;
use hotwatch::{Event, Hotwatch};
use std::{
    fmt::Debug,
    io::{BufRead, BufReader, Read, Stdin},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

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
pub struct StdinLogSource {
    stdin: BufReader<Stdin>,
}

impl StdinLogSource {
    pub fn new() -> Self {
        StdinLogSource {
            stdin: BufReader::new(std::io::stdin()),
        }
    }
}

impl LogSource for StdinLogSource {
    fn update_log(&mut self, log: &Log) -> anyhow::Result<Option<Log>> {
        // Try to read a byte from stdin
        let bytes_available = self
            .stdin
            .fill_buf()
            .ok()
            .filter(|buf| !buf.is_empty())
            .is_none();
        if bytes_available {
            // No changes
            return Ok(None);
        }

        // Append to the log file
        let mut raw = log.raw().to_string();
        self.stdin
            .read_to_string(&mut raw)
            .context("error reading from stdin")?;
        Log::parse(raw).context("error parsing log").map(Some)
    }
}
