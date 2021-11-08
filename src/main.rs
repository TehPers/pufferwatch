#![forbid(unsafe_code)]
#![deny(clippy::all, clippy::pedantic, clippy::perf)]
#![allow(
    clippy::module_name_repetitions,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_lossless
)]

mod ast;
mod events;
mod log;
mod parse;
mod source;
mod startup;
mod widgets;

fn main() -> anyhow::Result<()> {
    startup::start()
}
