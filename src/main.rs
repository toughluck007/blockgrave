mod app;
mod ui;

use std::io::{self, Stdout};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use anyhow::Result;
use app::App;

use crossterm::event::{self, Event as CEvent};
#[cfg(not(windows))]
use crossterm::event::{
    KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
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
    let (mut terminal, keyboard_enhanced) = setup_terminal()?;
    let res = run_app(&mut terminal, &mut app);
    restore_terminal(&mut terminal, keyboard_enhanced)?;
    res
}

fn setup_terminal() -> Result<(Terminal<CrosstermBackend<Stdout>>, bool)> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(&mut stdout, EnterAlternateScreen)?;
    let keyboard_enhanced = try_enable_keyboard_enhancement(&mut stdout)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok((terminal, keyboard_enhanced))
}

fn restore_terminal(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    keyboard_enhanced: bool,
) -> Result<()> {
    disable_raw_mode()?;

    #[cfg(windows)]
    let _ = keyboard_enhanced;
    #[cfg(not(windows))]
    if keyboard_enhanced {
        execute!(terminal.backend_mut(), PopKeyboardEnhancementFlags)?;
    }
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    terminal.show_cursor()?;
    Ok(())
}

#[cfg(windows)]
fn try_enable_keyboard_enhancement(_: &mut Stdout) -> Result<bool> {
    Ok(false)
}

#[cfg(not(windows))]
fn try_enable_keyboard_enhancement(stdout: &mut Stdout) -> Result<bool> {
    let result = execute!(
        stdout,
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::REPORT_EVENT_TYPES),
    );
    let keyboard_enhanced = match result {
        Ok(()) => true,
        Err(err) => {
            if keyboard_enhancement_unsupported(&err) {
                false
            } else {
                return Err(err.into());
            }
        }
    };
    Ok(keyboard_enhanced)
}

#[cfg(windows)]
fn keyboard_enhancement_unsupported(err: &std::io::Error) -> bool {
    use std::io::ErrorKind;

    err.kind() == ErrorKind::Unsupported
        || err
            .to_string()
            .contains("Keyboard progressive enhancement not implemented")
}
#[cfg(not(windows))]
fn keyboard_enhancement_unsupported(_: &std::io::Error) -> bool {
    false
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
