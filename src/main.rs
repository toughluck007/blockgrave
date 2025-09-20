mod app;
mod ui;

use std::io::{self, Stdout};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use anyhow::Result;
use app::App;
use crossterm::event::{self, Event as CEvent};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::ui::draw;

enum Event<I> {
    Input(I),
    Tick,
}

fn main() -> Result<()> {
    let mut app = App::new()?;
    let mut terminal = setup_terminal()?;
    let res = run_app(&mut terminal, &mut app);
    restore_terminal(&mut terminal)?;
    res
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<Stdout>>, app: &mut App) -> Result<()> {
    let (tx, rx) = mpsc::channel();
    let tick_rate = Duration::from_millis(200);

    let input_tx = tx.clone();
    thread::spawn(move || {
        loop {
            if !event::poll(Duration::from_millis(250)).unwrap_or(false) {
                continue;
            }
            match event::read() {
                Ok(CEvent::Key(key)) => {
                    if input_tx.send(Event::Input(key)).is_err() {
                        break;
                    }
                }
                Ok(_) => {}
                Err(_) => {}
            }
        }
    });

    thread::spawn(move || {
        loop {
            if tx.send(Event::Tick).is_err() {
                break;
            }
            thread::sleep(tick_rate);
        }
    });

    loop {
        terminal.draw(|f| draw(f, app))?;

        match rx.recv()? {
            Event::Input(key) => {
                app.on_key(key);
            }
            Event::Tick => {
                app.on_tick(tick_rate);
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
