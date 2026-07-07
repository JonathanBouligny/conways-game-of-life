use std::fs;
use std::io::{BufWriter, Stdout};
use std::io::{Write, stdout};
use std::thread::sleep;
use std::time::{Duration, Instant};

use anyhow::Context;
use anyhow::Ok;
use anyhow::Result;
use anyhow::bail;
use crossterm::cursor::{Hide, Show};
use crossterm::event::{KeyCode, KeyModifiers};
use crossterm::style::Stylize;
use crossterm::{
    ExecutableCommand,
    cursor::{DisableBlinking, EnableBlinking, MoveTo, RestorePosition, SavePosition},
    event::{
        DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, EnableBracketedPaste,
        EnableFocusChange, EnableMouseCapture, Event, poll, read,
    },
    execute,
    style::{self, Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::Clear,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};
use crossterm::{QueueableCommand, queue};

use clap::Parser;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[arg(short, long, default_value_t = 20)]
    rows: usize,

    /// Number of times to greet
    #[arg(short, long, default_value_t = 20)]
    columns: usize,

    /// Number of times to greet
    #[arg(short, long)]
    file_path: String,

    #[arg(short, long, default_value_t = 90)]
    tick_speed: u64,
}

#[derive(Default, Clone, Debug)]
struct Cell {
    alive: bool,
}

#[derive(Debug)]
struct PlayGrid {
    cells: Vec<Vec<Cell>>,
    width: usize,
    height: usize,
    grid_offsets: [(i32, i32); 8],
}

impl PlayGrid {
    fn new(cells: Vec<Vec<Cell>>, width: usize, height: usize) -> Self {
        Self {
            cells,
            width,
            height,
            grid_offsets: [
                (-1, -1),
                (-1, 0),
                (-1, 1),
                (0, -1),
                (0, 1),
                (1, -1),
                (1, 0),
                (1, 1),
            ],
        }
    }

    fn render(&self, out: &mut BufWriter<Stdout>) -> Result<()> {
        out.queue(crossterm::terminal::BeginSynchronizedUpdate)?;
        out.queue(crossterm::cursor::MoveTo(0, 0))?;
        for (row_index, cell_row) in self.cells.iter().enumerate() {
            for cell in cell_row {
                if !cell.alive {
                    out.queue(style::PrintStyledContent("██".grey()))?;
                } else {
                    out.queue(style::PrintStyledContent("██".red()))?;
                }
            }
            out.queue(Print("\r\n"))?;
        }
        out.queue(crossterm::terminal::EndSynchronizedUpdate)?;

        out.flush()?;
        Ok(())
    }

    // Must take an i32 because we have to check for negatives
    fn inbounds(&self, row: i32, col: i32) -> bool {
        return row >= 0 && row < self.height as i32 && col >= 0 && col < self.width as i32;
    }

    fn count_neighbors(&self, row: i32, col: i32) -> usize {
        let mut alive_count = 0;
        // loop over neighbors
        for neighbor in self.grid_offsets.iter() {
            // TODO: type problems we need to size things correctly we also need to make sure we're indexing at 0,0 for the top right
            let calced_row = row + neighbor.0;
            let calced_col = col + neighbor.1;
            // if neighbor is inbounds then it is also able to go into usize if neighbor.alive
            // Ok so we can do the inbounds calculation which checks with negatives and if its true that we can sucessfully conver tot usize so the second if statement is valid.
            // it basically reads if inbounds then we also know we can convert to usize and we do so.
            if self.inbounds(calced_row, calced_col)
                && self.cells[calced_row as usize][calced_col as usize].alive
            {
                //       add 1 to count
                alive_count += 1;
            }
        }
        return alive_count;
    }

    fn evaluate_into(&self, next_buffer: &mut PlayGrid) {
        for (row_index, row) in self.cells.iter().enumerate() {
            for (col_index, cell) in row.iter().enumerate() {
                // TODO: Fix this borrow issue
                if self.inbounds(row_index as i32, col_index as i32) {
                    let alive_count = self.count_neighbors(row_index as i32, col_index as i32);

                    // rules
                    if cell.alive && alive_count < 2 {
                        next_buffer.cells[row_index][col_index].alive = false;
                    } else if cell.alive && (alive_count == 2 || cell.alive && alive_count == 3) {
                        next_buffer.cells[row_index][col_index].alive = true;
                    } else if cell.alive && alive_count > 3 {
                        next_buffer.cells[row_index][col_index].alive = false;
                    } else if !cell.alive && alive_count == 3 {
                        next_buffer.cells[row_index][col_index].alive = true;
                    } else {
                        next_buffer.cells[row_index][col_index].alive =
                            self.cells[row_index][col_index].alive;
                    }
                }
            }
        }
    }

    fn write_starting_state(&mut self, starting_state: &String) -> Result<()> {
        let starting_state_comments_removed =
            starting_state.lines().filter(|l| !l.starts_with('!'));

        let largest_row = starting_state_comments_removed
            .map(|l| l.len())
            .max()
            .context("Largest row calulcation error")?;

        if self.height < starting_state_comments_removed.count() || self.width < 1 {
            bail!(
                "Grid is not large enough to hold selected .cell file. Grid Row Size: {} File Row Size: {} Grid Column Size: {} File Column Size: {}",
                self.height,
                starting_state.len(),
                self.width,
                largest_row
            );
        }

        for (row_index, line) in starting_state_comments_removed.enumerate() {
            for (col_index, ch) in line.chars().enumerate() {
                if ch == '.' {
                    self.cells[row_index][col_index].alive = false;
                } else if ch == 'O' {
                    self.cells[row_index][col_index].alive = true;
                } else {
                    bail!(
                        "Incorrect character found in file. File must be .cell format. https://conwaylife.com/wiki/"
                    );
                }
            }
        }

        Ok(())
    }
}

struct Terminal {
    out: BufWriter<Stdout>,
    exited: bool,
}

impl Terminal {
    fn new() -> Self {
        Self {
            exited: false,
            out: BufWriter::new(stdout()),
        }
    }
    fn setup_terminal(&mut self) -> Result<()> {
        crossterm::terminal::enable_raw_mode()?;

        self.out
            .queue(Hide)?
            .queue(DisableBlinking)?
            // .queue(EnableFocusChange)?
            // .queue(EnableMouseCapture)?
            // .queue(EnableBracketedPaste)?
            .queue(EnterAlternateScreen)?;

        self.out.flush()?;

        Ok(())
    }
    fn teardown_terminal(&mut self) -> Result<()> {
        crossterm::terminal::disable_raw_mode()?;

        self.out
            .queue(Show)?
            .queue(EnableBlinking)?
            // .queue(DisableFocusChange)?
            // .queue(DisableMouseCapture)?
            // .queue(DisableBracketedPaste)?
            .queue(LeaveAlternateScreen)?;
        self.out.flush()?;

        self.exited = true;
        Ok(())
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    let mut term: Terminal = Terminal::new();
    term.setup_terminal()?;

    let mut buffer1: PlayGrid = PlayGrid::new(
        vec![vec![Cell::default(); args.columns]; args.rows],
        args.columns,
        args.rows,
    );

    let mut buffer2: PlayGrid = PlayGrid::new(
        vec![vec![Cell::default(); args.columns]; args.rows],
        args.columns,
        args.rows,
    );

    // Parse initial state
    let stating_state = fs::read_to_string(args.file_path)?;

    buffer1.write_starting_state(&stating_state)?;

    let mut active_buffer = &mut buffer1;
    let mut inactive_buffer = &mut buffer2;
    loop {
        active_buffer.render(&mut term.out)?;
        active_buffer.evaluate_into(inactive_buffer);
        std::mem::swap(&mut active_buffer, &mut inactive_buffer);
        std::thread::sleep(Duration::from_millis(args.tick_speed));

        if poll(Duration::from_millis(10))? {
            // It's guaranteed that the `read()` won't block when the `poll()`
            // function returns `true`
            if let Event::Key(k) = read()? {
                if k.code == KeyCode::Char('c') && k.modifiers.contains(KeyModifiers::CONTROL) {
                    break;
                }
            }
        }
    }

    term.teardown_terminal()?;

    Ok(())
}
