use std::io::{Write, stdout};
use std::thread::sleep;
use std::time::{Duration, Instant};

use crossterm::QueueableCommand;
use crossterm::cursor::{Hide, Show};
use crossterm::event::{KeyCode, KeyModifiers};
use crossterm::{
    ExecutableCommand,
    cursor::{DisableBlinking, EnableBlinking, MoveTo, RestorePosition, SavePosition},
    event::{
        DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, EnableBracketedPaste,
        EnableFocusChange, EnableMouseCapture, Event, poll, read,
    },
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::Clear,
};

#[derive(Default, Copy, Clone)]
struct Cell {
    alive: bool,
}

fn setup_terminal() -> std::io::Result<()> {
    crossterm::terminal::enable_raw_mode()?;

    stdout()
        .queue(Hide)?
        .queue(DisableBlinking)?
        .queue(EnableFocusChange)?
        .queue(EnableMouseCapture)?
        .queue(EnableBracketedPaste)?;

    stdout().flush()?;

    Ok(())
}
fn teardown_terminal() -> std::io::Result<()> {
    crossterm::terminal::disable_raw_mode()?;

    stdout()
        .queue(Show)?
        .queue(EnableBlinking)?
        .queue(DisableFocusChange)?
        .queue(DisableMouseCapture)?
        .queue(DisableBracketedPaste)?;

    stdout().flush()?;

    Ok(())
}

fn main() -> std::io::Result<()> {
    let mut grid: [[Cell; 10]; 10] = [[Cell::default(); 10]; 10];

    setup_terminal()?;
    loop {
        // stdout()
        //     .execute(Clear(crossterm::terminal::ClearType::All))?
        //     .execute(MoveTo(0, 0))?
        //     .execute(SetForegroundColor(Color::Blue))?
        //     .execute(SetBackgroundColor(Color::Red))?
        //     .execute(Print("Swap 1"))?
        //     .execute(ResetColor)?
        //     .execute(DisableBlinking)?;

        // sleep(Duration::from_millis(1000));

        // stdout()
        //     .execute(Clear(crossterm::terminal::ClearType::All))?
        //     .execute(MoveTo(0, 0))?
        //     .execute(SetForegroundColor(Color::Green))?
        //     .execute(SetBackgroundColor(Color::Yellow))?
        //     .execute(Print("Swap 2"))?
        //     .execute(ResetColor)?;

        // sleep(Duration::from_millis(1000));
        if poll(Duration::from_millis(500))? {
            // It's guaranteed that the `read()` won't block when the `poll()`
            // function returns `true`
            match read()? {
                Event::FocusGained => println!("FocusGained"),
                Event::FocusLost => println!("FocusLost"),
                Event::Key(event) => {
                    if event.code == KeyCode::Char('c') && event.modifiers == KeyModifiers::CONTROL
                    {
                        break;
                    }

                    println!("{:?}", event)
                }
                Event::Mouse(event) => println!("{:?}", event),
                Event::Paste(data) => println!("Pasted {:?}", data),
                Event::Resize(width, height) => println!("New size {}x{}", width, height),
            }
        } else {
            // Timeout expired and no `Event` is available
        }
    }
    // or using functions

    teardown_terminal()?;
    Ok(())
}
