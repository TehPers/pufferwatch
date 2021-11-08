#![forbid(unsafe_code)]
#![cfg_attr(
    not(debug_assertions),
    deny(clippy::all, clippy::pedantic, clippy::perf)
)]
#![cfg_attr(debug_assertions, warn(clippy::all, clippy::pedantic, clippy::perf))]

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
