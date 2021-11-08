use std::{
    borrow::Cow,
    fmt::{Display, Formatter},
};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Level {
    Trace,
    Debug,
    Info,
    Alert,
    Warn,
    Error,
}

impl Level {
    /// Array of all levels
    pub const ALL: [Level; 6] = [
        Level::Trace,
        Level::Debug,
        Level::Info,
        Level::Alert,
        Level::Warn,
        Level::Error,
    ];
}

impl Display for Level {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Level::Trace => "TRACE".fmt(f),
            Level::Debug => "DEBUG".fmt(f),
            Level::Info => "INFO".fmt(f),
            Level::Alert => "ALERT".fmt(f),
            Level::Warn => "WARN".fmt(f),
            Level::Error => "ERROR".fmt(f),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct Timestamp {
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl Display for Timestamp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{hh:02}:{mm:02}:{ss:02}",
            hh = self.hour,
            mm = self.minute,
            ss = self.second,
        )
    }
}

#[derive(Clone, Debug)]
pub struct Message<'a> {
    pub timestamp: Timestamp,
    pub level: Level,
    pub source: Cow<'a, str>,
    pub contents: Cow<'a, str>,
}
