use crate::{
    events::AppEvent,
    log::Log,
    widgets::{BindingDisplay, IconPack},
};
use indexmap::IndexMap;

pub trait State {
    fn update(&mut self, event: &AppEvent) -> bool;

    fn add_controls<I: IconPack>(&self, _controls: &mut IndexMap<BindingDisplay<I>, &'static str>) {
    }
}

impl State for () {
    fn update(&mut self, _event: &AppEvent) -> bool {
        false
    }
}

impl<T> State for Option<T>
where
    T: State,
{
    fn update(&mut self, event: &AppEvent) -> bool {
        if let Some(state) = self {
            state.update(event)
        } else {
            false
        }
    }
}

impl<T, E> State for Result<T, E>
where
    T: State,
{
    fn update(&mut self, event: &AppEvent) -> bool {
        if let Ok(state) = self {
            state.update(event)
        } else {
            false
        }
    }
}

impl<T> State for [T]
where
    T: State,
{
    fn update(&mut self, event: &AppEvent) -> bool {
        self.iter_mut().any(|state| state.update(event))
    }
}

impl<T> State for Box<T>
where
    T: State,
{
    fn update(&mut self, event: &AppEvent) -> bool {
        self.as_mut().update(event)
    }
}

pub trait WithLog<'i> {
    type Result;

    fn with_log(self, log: &'i Log) -> Self::Result;
}

impl<'i> WithLog<'i> for () {
    type Result = ();

    fn with_log(self, _log: &'i Log) -> Self::Result {}
}

impl<'i, T> WithLog<'i> for Option<T>
where
    T: WithLog<'i>,
{
    type Result = Option<T::Result>;

    fn with_log(self, log: &'i Log) -> Self::Result {
        self.map(|t| t.with_log(log))
    }
}

impl<'i, T, E> WithLog<'i> for Result<T, E>
where
    T: WithLog<'i>,
{
    type Result = Result<T::Result, E>;

    fn with_log(self, log: &'i Log) -> Self::Result {
        self.map(|t| t.with_log(log))
    }
}

impl<'i, T> WithLog<'i> for Vec<T>
where
    T: WithLog<'i>,
{
    type Result = Vec<T::Result>;

    fn with_log(self, log: &'i Log) -> Self::Result {
        self.into_iter().map(|t| t.with_log(log)).collect()
    }
}

impl<'i, T> WithLog<'i> for Box<T>
where
    T: WithLog<'i>,
{
    type Result = Box<T::Result>;

    fn with_log(self, log: &'i Log) -> Self::Result {
        Box::new((*self).with_log(log))
    }
}
