use crossbeam::channel::{Receiver, Sender};
use crossterm::event::Event;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::JoinHandle,
    time::Duration,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum AppEvent {
    Ping,
    TermEvent(Event),
}

pub struct EventController {
    running: Arc<AtomicBool>,
    thread_handle: Option<JoinHandle<()>>,
}

impl EventController {
    const POLL_INTERVAL: Duration = Duration::from_millis(50);

    pub fn start() -> (Receiver<AppEvent>, Self) {
        let running = Arc::new(AtomicBool::new(true));
        let (event_tx, event_rx) = crossbeam::channel::unbounded();
        let ping_handle = std::thread::spawn({
            let running = running.clone();
            move || Self::read_events(event_tx, running)
        });

        (
            event_rx,
            EventController {
                running,
                thread_handle: Some(ping_handle),
            },
        )
    }

    fn read_events(event_tx: Sender<AppEvent>, running: Arc<AtomicBool>) {
        while running.load(Ordering::Relaxed) {
            // Poll for an event
            let poll = crossterm::event::poll(Self::POLL_INTERVAL)
                .expect("error polling for terminal events");
            if poll {
                let event = crossterm::event::read().expect("error reading terminal event");
                event_tx.send(AppEvent::TermEvent(event)).unwrap();
            } else {
                event_tx.send(AppEvent::Ping).unwrap();
            }
        }
    }
}

impl Drop for EventController {
    fn drop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.thread_handle.take() {
            handle.join().unwrap();
        }
    }
}
