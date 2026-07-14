use std::collections::HashMap;
use std::fs;
use std::io::{BufWriter, Stdout};
use std::io::{Write, stdout};
use std::time::Duration;

use anyhow::Context;
use anyhow::Ok;
use anyhow::Result;
use anyhow::bail;
use crossterm::QueueableCommand;
use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{KeyCode, KeyModifiers};
use crossterm::style::Stylize;
use crossterm::{
    cursor::{DisableBlinking, EnableBlinking},
    event::{Event, poll, read},
    style::{self, Color, Print},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};

use clap::Parser;

// POSSIBLE STRETCH GOALS
// Replace grid with bit field. Do the math for accessing individual bits and it takes up less memory. YOu can do hashmap or bit field you cant do both.
// how can you update individual pixels?
// Do you need to move the cursor to update it or can you just update the spot?. In memory hashmap or in mapped file.
// Modify hashmap and then update from map you can diff from the map
// movable viewport
// cursor to move around vs updating some in memory hashmap and pasting it does crossterm do this on the backend? There are ways to render to stdout that llow you to overwrite specifically spots in the terminal in a stdout file

// Proc macro acceptse a token stream and emits a token stream idealy mutates the token stream
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[arg(short, long, default_value_t = 120)]
    rows: usize,

    /// Number of times to greet
    #[arg(short, long, default_value_t = 240)]
    columns: usize,

    /// Number of times to greet
    #[arg(short, long, default_value = "./starting_states/r-pentomino.cells")]
    file_path: String,

    #[arg(short, long, default_value_t = 60)]
    tick_speed: u64,
}

// DONE
#[derive(Default, Clone, Debug)]
struct Cell {
    alive: bool,
}

// DONE
#[derive(Debug)]
struct PlayGrid {
    cells: HashMap<(usize, usize), Cell>,
    grid_cols: usize,
    grid_rows: usize,
    grid_offsets: [(i32, i32); 8],
    alive_color: Color,
    dead_color: Color,
}

impl PlayGrid {
    // DONE
    fn new(cells: HashMap<(usize, usize), Cell>, grid_cols: usize, grid_rows: usize) -> Self {
        Self {
            cells,
            grid_cols,
            grid_rows,
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
            alive_color: Color::Green,
            dead_color: Color::Black,
        }
    }

    fn render_cell(&self) -> Result<()> {
        Ok(())
    }

    // O(alive) instead of O(a*b)
    // You can parallelize all the render operations because they're all independent
    fn render(&self, out: &mut BufWriter<Stdout>) -> Result<()> {
        for ((x, y), cell) in self.cells.iter() {
            out.queue(MoveTo(*x as u16, *y as u16));
            out.queue(style::PrintStyledContent("██".with(self.alive_color)));
            out.queue(Print("\r\n"))?;
        }

        out.flush()?;
        Ok(())
    }

    // DONE
    fn setup_grid(&self, out: &mut BufWriter<Stdout>) -> Result<()> {
        out.queue(crossterm::cursor::MoveTo(0, 0))?;

        for _ in 0..self.grid_rows {
            for _ in 0..self.grid_cols {
                out.queue(style::PrintStyledContent("██".with(self.dead_color)))?;
            }
            out.queue(Print("\r\n"))?;
        }

        out.flush()?;
        Ok(())
    }
    // NO CHANGE NEEDED
    // Must take an i32 because we have to check for negatives
    fn inbounds(&self, row: i32, col: i32) -> bool {
        return row >= 0 && row < self.grid_rows as i32 && col >= 0 && col < self.grid_cols as i32;
    }

    // DONE
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
            // Ok in my count_neighbors function which now looks at a hashmap because its only being called on the list of values in the hashmap i think i can safely do this get without checking for none
                && self
                    .cells
                    .get(&(calced_row as usize, calced_col as usize))
                    .map_or(false, |cell| cell.alive)
            {
                //       add 1 to count
                alive_count += 1;
            }
        }
        return alive_count;
    }

    // DONE
    fn evaluate_into(&self, next_buffer: &mut PlayGrid) {
        for ((x, y), cell) in self.cells.iter() {
            // I think cells can only be in the hashmap if they're valid...
            let alive_count: usize = self.count_neighbors(*x as i32, *y as i32);

            let update_val;
            // rules, this into function. APply Rules function
            if cell.alive && alive_count < 2 {
                update_val = false;
            } else if cell.alive && (alive_count == 2 || cell.alive && alive_count == 3) {
                update_val = true;
            } else if cell.alive && alive_count > 3 {
                update_val = false;
            } else if !cell.alive && alive_count == 3 {
                update_val = true;
            } else {
                update_val = cell.alive;
            }

            // ASK CLAUDE: I think this works. it tries to get the entry for modification. If it isnt ther eit sets a default and inserts it then we can update it. if it is there we just update it and the or_insert doesnt come
            // ASK CLAUDE: Doesnt this make a new cell we want to update the old cell Does this wipe it out and is this wasteful?
            let entry_to_update = next_buffer
                .cells
                // ASK CLAUDE: It looks like this gets the entry for manipulation and then if the entry was none it inserts this default? But i want ot modify it if it is there so does this also insert the default if its not there?
                .entry((*x, *y))
                .or_insert(Cell::default());

            entry_to_update.alive = update_val;
        }
    }

    // DONE
    fn write_starting_state(&mut self, starting_state: &String) -> Result<()> {
        let starting_state_comments_removed: Vec<&str> = starting_state
            .lines()
            .filter(|l| !l.starts_with('!'))
            .collect();

        let pattern_cols = starting_state_comments_removed
            .iter()
            .map(|l| l.len())
            .max()
            .context("Largest row calulcation error")?;

        let pattern_rows = starting_state_comments_removed.len();
        if self.grid_rows < pattern_rows || self.grid_cols < pattern_cols {
            bail!(
                "Grid is not large enough to hold selected .cell file. Grid Row Size: {} File Row Size: {} Grid Column Size: {} File Column Size: {}",
                self.grid_rows,
                pattern_rows,
                self.grid_cols,
                pattern_cols
            );
        }

        let pattern_offset_rows = (self.grid_rows - pattern_rows) / 2;
        let pattern_offset_cols = (self.grid_cols - pattern_cols) / 2;

        for (row, line) in starting_state_comments_removed.iter().enumerate() {
            for (col, ch) in line.chars().enumerate() {
                if ch == '.' {
                    self.cells.insert(
                        (row + pattern_offset_rows, col + pattern_offset_cols),
                        Cell { alive: false },
                    );
                } else if ch == 'O' {
                    self.cells.insert(
                        (row + pattern_offset_rows, col + pattern_offset_cols),
                        Cell { alive: true },
                    );

                    // gather neighbors and put them in the hashmap too.
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
            .queue(EnterAlternateScreen)?;

        self.out.flush()?;

        Ok(())
    }
    fn teardown_terminal(&mut self) -> Result<()> {
        crossterm::terminal::disable_raw_mode()?;

        self.out
            .queue(Show)?
            .queue(EnableBlinking)?
            .queue(LeaveAlternateScreen)?;
        self.out.flush()?;

        self.exited = true;
        Ok(())
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        if !self.exited {
            let _ = self.teardown_terminal();
        }
    }
}

#[cfg(test)]
mod tests {

    // Tokio versions of tests to run these in a worker
    use super::*;

    #[test]
    fn inbounds_tests() {
        let grid1 = PlayGrid::new(HashMap::new(), 10, 10);

        let result1 = grid1.inbounds(2, 2);
        assert_eq!(result1, true);

        let result2 = grid1.inbounds(-1, 2);
        assert_eq!(result2, false);

        let result3 = grid1.inbounds(2, 11);
        assert_eq!(result3, false);
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    let mut term: Terminal = Terminal::new();
    term.setup_terminal()?;

    let mut buffer1: PlayGrid = PlayGrid::new(HashMap::new(), args.columns, args.rows);

    let mut buffer2: PlayGrid = PlayGrid::new(HashMap::new(), args.columns, args.rows);

    // Parse initial state
    let stating_state = fs::read_to_string(args.file_path)?;

    buffer1.write_starting_state(&stating_state)?;

    let active_buffer = &mut buffer1;
    let inactive_buffer = &mut buffer2;
    active_buffer.setup_grid(&mut term.out)?;
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
