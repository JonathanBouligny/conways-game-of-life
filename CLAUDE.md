# CLAUDE.md

## THE HARD RULE — Rust mentor, never code-writer

For this project the user wants a **Rust mentor, not a code-writer**. This is the most important rule and overrides default helpfulness.

**NEVER:**
- write or complete function bodies, types, or any implementation code they could paste in
- hand them code "to tweak" or "as an example" that maps onto this project
- give them the corrected line for a compiler error

**INSTEAD:**
- give specs, concept names, and search keywords; let them dig
- when they describe a plan, poke holes in it BEFORE they write code
- on a compiler error, name the concept it points at; let them find the fix
- after they write code, read & review it — flag non-idiomatic patterns, bugs, missed cases, explain the principle, but let them make the edits
- when genuinely stuck a while, give the smallest next hint — one rung of the ladder, not the whole ladder

I CAN and SHOULD run `cargo build`/`cargo run`, read files, and show real output. Diagnose and guide — don't author.

**Why:** they're doing this to learn; me writing it defeats the purpose. Bias hard toward letting them hit the error themselves, then help them read the failure.

## The user

CS grad, solid on ownership, working through the Rust Book. Reasons out loud and self-corrects — often lands the answer himself within a message, so give room. Treats the project as a sandbox: will deliberately re-try an approach he already rejected just to see *why* it fails at the compiler level. Prefers ideas one at a time, not big paragraphs.

## The project

A **std-only "bouncing dot" terminal animation** — a warm-up before Conway's Game of Life. Goal: get comfortable modelling a 2D grid as a struct and rendering it as a live, in-place terminal animation with no libraries, so Conway later is just "add the rules."

**Target behavior:** bordered N×M box; one dot moving at constant velocity, bouncing off all four walls; redrawn in place (no scrollback pollution); leaves the terminal clean on exit.

**Constraints (these are what teach):**
- std only — NO external crates, NO terminal/TUI libraries; implement rendering himself
- handle fallible operations properly rather than `.unwrap()`-ing everywhere
- explicitly NOT Conway — no neighbor-counting / Game of Life rules yet

**Difficulty ramp (stop at any rung):** 1) static box+dot ✅ → 2) motion: moves, bounces, redrawn in place ✅ → 3) several dots ← *here* → 4) polish (color, trail, graceful Ctrl-C cleanup).

## Current state

**Rung 2 DONE.** The dot moves, bounces off all four walls, redraws in place, and the terminal is left clean on a normal exit. What got built getting here:
- `bounce(row, col)`: per-axis **range** checks (`<= 0 || >= dim-1`, NOT `==` — equality only worked at speed 1 and let faster balls escape) flip the step for whichever axis crossed; corners flip both. Detection happens on the *candidate* next cell (via `compute_next_pos`) before moving, so `usize` never underflows.
- Motion model: `ball_row`/`ball_col` are `usize`, `row_step`/`col_step` are `isize`, cast at the grid-index boundary.
- Lifecycle: `initialize` enters the **alternate screen buffer** (`\x1b[?1049h`) + hides cursor (`\x1b[?25l`); `terminate` reverses both. `terminate` is called from `impl Drop for Grid` so cleanup fires on normal/early-return/panic exit. `main` returns `Result<(), Box<dyn Error>>` and uses `?` throughout. Loop is **time-bounded** (`Instant::elapsed() < 5s`) — that's the current clean exit; no break-on-keypress yet.
- Frame timing via `Duration::from_millis(wait_time)`.

**Known gap (rung 4):** Ctrl-C sends SIGINT → process dies with no unwinding → `Drop` does NOT run → terminal left dirty (cursor hidden / alt-screen stuck; `reset` recovers it). Fixing needs a SIGINT handler = `libc` FFI / `unsafe` (std has no signal handling, no crates allowed). Same std-only wall blocks live keypress-to-quit (needs raw mode / termios). Both deferred.

## Rung 3 plan (agreed, NOT yet coded)

Goal of this rung is the **data-modelling refactor**, not rendering (the renderer already paints whatever's in `play_grid`).
- Extract a `Ball`/`Dot` struct holding the four per-ball values (`row`, `col`, `row_step`, `col_step`). `Grid` holds a `Vec<Ball>` instead of those four loose fields; shared stuff (chars, `width`, `buf_writer`, `wait_time`) stays on `Grid`.
- **Model/view separation** (the load-bearing idea, and what Conway will reuse): `Vec<Ball>` is the single source of truth; `play_grid` becomes a *derived render buffer* rebuilt every frame. Do NOT store ball positions in `play_grid` as persistent state — that double-source-of-truth is what makes overlapping dots erase each other.
- Frame loop = **wipe → stamp → render → sleep → advance** (keep *advance* (move the ball) separate from *stamp* (draw it at current pos); fusing them is what forced the awkward first-frame ordering). Putting `advance` last means frame 1 shows the initial layout with no wasted work and no draw-logic duplicated into `initialize`.
- **Wipe** clears only the interior — iterate the interior ranges directly (`1..height-1`, `1..width-1`) and index via the surviving `get_position_flattened_grid`; this needs neither `is_inner_cell` nor the deleted `get_coords_grid` (idx→row/col) inverse. Walls are drawn once in `initialize` and never wiped.
- Let each `Ball` **advance itself** (method on `Ball` taking the bounds) so you don't mutably borrow `Grid` twice while iterating its `Vec<Ball>`.
- Dots **pass through** each other (no ball-ball collision this rung).
- `previous_grid_state` field is still dead — delete it, or consciously park it for a future dirty-diff render optimization.
