use crate::log::Log;
use anyhow::Context;
use hotwatch::{Event, Hotwatch};
use std::{
    fmt::Debug,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

pub trait LogSource {
    fn update(&mut self) -> Option<Log>;
}

#[derive(Debug)]
pub struct StaticLogSource {
    log: Option<Log>,
}

impl StaticLogSource {
    pub fn new(path: &Path) -> anyhow::Result<Self> {
        Log::parse_file(path)
            .map(|log| StaticLogSource { log: Some(log) })
            .context("Error parsing log")
    }
}

impl LogSource for StaticLogSource {
    fn update(&mut self) -> Option<Log> {
        self.log.take()
    }
}

#[derive(Debug)]
pub struct FollowedLogSource {
    path: PathBuf,
    initial_log: Option<Log>,
    _hotwatch: Hotwatch,
    needs_reload: Arc<AtomicBool>,
}

impl FollowedLogSource {
    pub fn new(path: PathBuf) -> anyhow::Result<Self> {
        // let path = path
        //     .canonicalize()
        //     .with_context(|| format!("couldn't resolve path: {}", path.display()))?;

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

        Ok(FollowedLogSource {
            path,
            initial_log: Some(log),
            _hotwatch: hotwatch,
            needs_reload,
        })
    }
}

impl LogSource for FollowedLogSource {
    fn update(&mut self) -> Option<Log> {
        if self.needs_reload.load(Ordering::Relaxed) {
            self.initial_log.take();
            if let Ok(log) = Log::parse_file(&self.path) {
                return Some(log);
            }
        }

        self.initial_log.take()
    }
}
