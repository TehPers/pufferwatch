use crate::{ast::Message, parse::parse};
use anyhow::Context;
use itertools::Itertools;
use ouroboros::self_referencing;
use std::{collections::HashMap, fs::File, io::Read, path::Path};

/// A parsed SMAPI log.
#[self_referencing]
#[derive(Debug)]
pub struct Log {
    raw: String,
    #[borrows(raw)]
    #[covariant]
    messages: Vec<Message<'this>>,
    #[borrows(messages)]
    #[covariant]
    by_source: HashMap<&'this str, Vec<&'this Message<'this>>>,
}

impl Log {
    /// Creates a new empty log.
    pub fn empty() -> Self {
        LogBuilder {
            raw: String::new(),
            messages_builder: |_| Vec::new(),
            by_source_builder: |_| HashMap::new(),
        }
        .build()
    }

    /// Parses a log from a string.
    pub fn parse(raw: String) -> anyhow::Result<Self> {
        // Log is self-referential because the messages borrow from the raw string
        LogTryBuilder {
            raw,
            messages_builder: |source| parse(source).context("error parsing log file"),
            by_source_builder: |messages| {
                Ok(messages
                    .iter()
                    .group_by(|message| message.source.as_ref())
                    .into_iter()
                    .map(|(source, messages)| (source, messages.collect_vec()))
                    .collect())
            },
        }
        .try_build()
    }

    /// Parses a log from a file.
    pub fn parse_file(path: &Path) -> anyhow::Result<Self> {
        // Read log file
        let mut log_file = File::open(path)
            .with_context(|| format!("Failed to open log file: {}", path.display()))?;
        let mut log_contents = String::new();
        log_file
            .read_to_string(&mut log_contents)
            .context("Failed to read log file")?;

        // Parse log
        Log::parse(log_contents)
    }

    /// Gets the raw log contents.
    pub fn raw(&self) -> &str {
        self.borrow_raw()
    }

    /// Gets the messages in the log.
    pub fn messages(&self) -> &[Message] {
        self.borrow_messages()
    }

    /// Gets the log sources in the log.
    pub fn sources(&self) -> impl Iterator<Item = &str> {
        self.borrow_by_source().keys().copied()
    }
}
