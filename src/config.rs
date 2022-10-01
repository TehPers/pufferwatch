use clap::{Args, Parser, Subcommand, ValueEnum};
use reqwest::Url;
use std::{ffi::OsString, path::PathBuf};

/// A CLI application for filtering and monitoring SMAPI logs.
#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct App {
    /// The command to execute.
    #[command(subcommand)]
    pub command: AppCommand,
    /// The path to output this application's logs to (not SMAPI logs). Set
    /// the RUST_LOG environment variable to configure the output.
    #[arg(long)]
    pub output_log: Option<PathBuf>,
}

#[derive(Clone, Debug, Subcommand)]
pub enum AppCommand {
    /// Read from a local log file.
    Log(LogCommand),
    /// Read from stdin.
    Stdin(StdinCommand),
    /// Read from a remote log file.
    Remote(RemoteCommand),
    /// Run SMAPI and monitor the logs.
    Run(RunCommand),
}

#[derive(Clone, Debug, Args)]
pub struct LogCommand {
    // The path to the log file.
    #[arg(short, long)]
    pub log: Option<PathBuf>,
    /// Watch the log file for changes. This is enabled when executing a command.
    #[arg(short, long)]
    pub follow: bool,
}

/// Read the log from stdin.
#[derive(Clone, Debug, Args)]
pub struct StdinCommand;

/// Download the log from a remote source.
#[derive(Clone, Debug, Args)]
pub struct RemoteCommand {
    /// The URL of the log file.
    pub url: Url,
}

/// Run SMAPI and watches the output.
#[derive(Clone, Debug, Args)]
pub struct RunCommand {
    /// The path to the SMAPI executable.
    pub smapi_path: Option<PathBuf>,
    /// The arguments to pass to SMAPI.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub smapi_args: Vec<OsString>,
    // The path to the log file.
    #[arg(short, long)]
    pub log: Option<PathBuf>,
    /// The encoding to use when sending commmands to SMAPI.
    #[arg(long, value_enum)]
    #[cfg_attr(windows, arg(default_value_t = CommandEncoding::Utf16Be))]
    #[cfg_attr(not(windows), arg(default_value_t = CommandEncoding::Utf8))]
    pub encoding: CommandEncoding,
}

/// The encoding to use when sending commands.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, ValueEnum)]
pub enum CommandEncoding {
    /// UTF-8 encoding.
    Utf8,
    /// UTF-16 (little endian) encoding.
    Utf16Le,
    /// UTF-16 (big endian) encoding.
    Utf16Be,
}
