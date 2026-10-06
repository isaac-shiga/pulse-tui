mod job;

use std::io;
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use ratatui::crossterm::event::{self, Event, KeyEvent, KeyEventKind};

use crate::app::{App, Msg};
use crate::{api, config, ui};

/// How often animations advance and timers are checked.
const TICK: Duration = Duration::from_millis(100);

/// Everything that wakes the loop.
pub(crate) enum Incoming {
    Key(KeyEvent),
    Msg(Box<Msg>),
    /// The terminal changed size, so the screen must be drawn again.
    Redraw,
}

pub fn run() -> io::Result<()> {
    let mut app = App::new(config::Config::load(), true);
    let http = api::http();
    let (tx, rx) = mpsc::channel();
    ratatui::run(|terminal| -> io::Result<()> {
        // Start reading only once `ratatui::run` has put the terminal in raw mode.
        read_terminal(tx.clone());
        let mut next_tick = Instant::now() + TICK;
        while !app.quit {
            terminal.draw(|frame| ui::render(frame, &app))?;
            // Sleep until a key, an API answer, or the next tick, whichever comes first.
            let first = match rx.recv_timeout(next_tick.saturating_duration_since(Instant::now())) {
                Ok(incoming) => Some(incoming),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => break,
            };
            app.now = Instant::now();
            for incoming in first.into_iter().chain(rx.try_iter()) {
                match incoming {
                    Incoming::Key(key) => app.on_key(key),
                    Incoming::Msg(message) => app.on_msg(*message),
                    Incoming::Redraw => {}
                }
            }
            if app.now >= next_tick {
                app.tick(app.now);
                next_tick = app.now + TICK;
            }
            for pending in app.take_jobs() {
                let client = api::Client::new(http.clone(), app.env(), app.key());
                job::spawn(pending, client, tx.clone());
            }
        }
        Ok(())
    })
}

/// Reads the terminal on its own thread, so the loop can sleep on one channel.
fn read_terminal(tx: Sender<Incoming>) {
    thread::spawn(move || {
        while let Ok(event) = event::read() {
            let incoming = match event {
                Event::Key(key) if key.kind == KeyEventKind::Press => Incoming::Key(key),
                Event::Resize(..) => Incoming::Redraw,
                _ => continue,
            };
            if tx.send(incoming).is_err() {
                break;
            }
        }
    });
}
