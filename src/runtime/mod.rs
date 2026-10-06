mod job;

use std::io;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use ratatui::crossterm::event::{self, Event, KeyEventKind};

use crate::app::App;
use crate::{api, config, ui};

pub fn run() -> io::Result<()> {
    let mut app = App::new(config::Config::load(), true);
    let http = api::http();
    let (tx, rx) = mpsc::channel();
    let mut terminal = ratatui::init();

    let result = (|| -> io::Result<()> {
        while !app.quit {
            terminal.draw(|frame| ui::render(frame, &app))?;
            if event::poll(Duration::from_millis(100))?
                && let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
            {
                app.on_key(key);
            }

            app.tick(Instant::now());
            while let Ok(message) = rx.try_recv() {
                app.on_msg(message);
            }
            for pending in app.take_jobs() {
                let client = api::Client::new(http.clone(), app.env(), app.key());
                job::spawn(pending, client, tx.clone());
            }
        }
        Ok(())
    })();

    ratatui::restore();
    result
}
