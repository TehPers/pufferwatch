use clap::{Args, Parser, Subcommand, ValueEnum};
use reqwest::Url;
use std::{ffi::OsString, path::PathBuf};

/// A CLI application for filtering and monitoring SMAPI logs.
///
/// Pufferwatch can be used as an alternative terminal to the SMAPI console, as
/// a standalone log viewer/monitor, or as a debugging tool during mod
/// development.
///
/// To use pufferwatch automatically from Steam, update your launch options by
/// adding "pufferwatch run -- " to the beginning of the existing options. For
/// example, on Windows, your launch options might look like this:
///
/// pufferwatch run -- "C:\Program Files (x86)\Steam\steamapps\common\Stardew Valley\StardewModdingAPI.exe" %command%
///
/// If you uninstall pufferwatch, remove the "pufferwatch run -- " part from
/// your launch options.
#[derive(Clone, Debug, Parser)]
#[command(author, version, about)]
pub struct App {
    /// The command to execute.
    #[command(subcommand)]
    pub command: AppCommand,
    /// The path to output this application's logs to (not SMAPI logs). Set
    /// the RUST_LOG environment variable to configure the output.
    #[arg(long)]
    pub output_log: Option<PathBuf>,
}

/// A command to execute.
#[derive(Clone, Debug, Subcommand)]
pub enum AppCommand {
    /// Read or monitor a local log file.
    ///
    /// If no log file is specified, pufferwatch will search for it. On Windows,
    /// it checks %APPDATA%. On Linux, it checks $XDG_DATA_HOME or
    /// $HOME/.config. On Mac, it checks $HOME/.config.
    Monitor(MonitorCommand),
    /// Read from stdin.
    ///
    /// The log will be parsed directly from stdin as it is received. On
    /// Windows, many applications used from Powershell and cmd buffer their
    /// output, so you might not see any logs until the application is closed.
    Stdin(StdinCommand),
    /// Read from a remote log file.
    ///
    /// Logs from https://smapi.io/log/ are supported, but the URL must have
    /// ?format=RawDownload added to the end of it. In other words, those URLs
    /// should be in the format https://smapi.io/log/123456?format=RawDownload.
    Remote(RemoteCommand),
    /// Run SMAPI and monitor the logs.
    ///
    /// If no path to SMAPI is specified, pufferwatch will search for it. On
    /// Windows, it will search for the path in the registry and in common
    /// installation directories. On Linux and Mac, it will search in common
    /// installation directories only.
    ///
    /// In all operating systems, pufferwatch will respect the
    /// stardewvalley.targets file if it exists in the user's home directory.
    ///
    /// Additionally, the log file will be monitored for changes automatically.
    /// The rules for searching for the log file are specified in the monitor
    /// command.
    Run(RunCommand),
}

/// Read or monitor a local log file.
#[derive(Clone, Debug, Args)]
pub struct MonitorCommand {
    // The path to the log file.
    #[arg(short, long)]
    pub log: Option<PathBuf>,
    /// Watch the log file for changes.
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
