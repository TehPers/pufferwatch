#![forbid(unsafe_code)]
#![deny(clippy::all, clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_lossless,
    clippy::type_complexity
)]

mod ast;
mod config;
mod encoded_writer;
mod events;
mod install_path;
mod log;
mod parse;
mod source;
mod startup;
mod widgets;

fn main() -> anyhow::Result<()> {
    use crate::config::App;
    use clap::Parser;

    let config = App::parse();
    startup::start(config)
}
