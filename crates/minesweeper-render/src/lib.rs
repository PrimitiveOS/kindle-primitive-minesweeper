use std::{
    fs, io,
    path::PathBuf,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use minesweeper_core::{BestTimes, Difficulty, Minefield, Status, View as Tile};
use primitive_app::{App, Change, WIDTH};
use primitive_ui::{
    back_button, header, screen_rect, Canvas, Input, Rect, TextStyle, BLACK, CAPTION, HAIRLINE,
    PAPER,
};

const VISIBLE: usize = 11;
const TILE: i32 = 105;
const BOARD_Y: i32 = 240;
const POPUP: Rect = Rect::new(170, 430, 924, 820);

#[derive(Clone, Copy)]
enum Panel {
    NewGame,
    BestTimes,
}

pub struct View {
    game: Minefield,
    records: BestTimes,
    data: Option<PathBuf>,
    row: usize,
    column: usize,
    flag_mode: bool,
    panel: Option<Panel>,
    started: Option<Instant>,
    displayed_seconds: u64,
    new_best: bool,
    save_error: bool,
}

impl View {
    pub fn new(game: Minefield, records: BestTimes, data: Option<PathBuf>) -> Self {
        let started = (game.status() == Status::Playing).then(Instant::now);
        let displayed_seconds = game.elapsed();
        Self {
            game,
            records,
            data,
            row: 0,
            column: 0,
            flag_mode: false,
            panel: None,
            started,
            displayed_seconds,
            new_best: false,
            save_error: false,
        }
    }

    pub fn load(data: PathBuf) -> Self {
        let (game, records) = fs::read(&data)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<(Minefield, BestTimes)>(&bytes).ok())
            .filter(|(game, _)| game.is_valid())
            .unwrap_or_else(|| {
                (
                    Minefield::new(Difficulty::Beginner, seed()),
                    BestTimes::default(),
                )
            });
        Self::new(game, records, Some(data))
    }

    pub fn game(&self) -> &Minefield {
        &self.game
    }

    pub fn origin(&self) -> (usize, usize) {
        (self.row, self.column)
    }

    fn elapsed(&self, now: Instant) -> u64 {
        self.game.elapsed().saturating_add(
            self.started
                .map_or(0, |start| now.saturating_duration_since(start).as_secs()),
        )
    }

    fn stop_clock(&mut self, now: Instant) {
        self.game.set_elapsed(self.elapsed(now));
        self.displayed_seconds = self.game.elapsed();
        self.started = None;
    }

    fn record(&mut self) {
        self.save_error = self.save().is_err();
    }

    fn move_page(&mut self, dr: i32, dc: i32) -> Change {
        let shift = |position: usize, amount: i32, length: usize| {
            (position as i32 + amount * (VISIBLE as i32 - 1))
                .clamp(0, length.saturating_sub(VISIBLE) as i32) as usize
        };
        let (rows, columns, _) = self.game.difficulty().size();
        let row = shift(self.row, dr, rows);
        let column = shift(self.column, dc, columns);
        if (row, column) == (self.row, self.column) {
            return Change::None;
        }
        (self.row, self.column) = (row, column);
        Change::Full
    }

    fn choose(&mut self, row: usize, column: usize, flag: bool) -> Change {
        let changed = if flag {
            self.game.flag(row, column)
        } else {
            self.game.reveal(row, column)
        };
        if !changed {
            return Change::None;
        }
        let now = Instant::now();
        match self.game.status() {
            Status::Ready => {}
            Status::Playing => {
                self.started.get_or_insert(now);
                self.displayed_seconds = self.elapsed(now);
            }
            Status::Won | Status::Lost => {
                self.stop_clock(now);
                if self.game.status() == Status::Won {
                    self.new_best = self
                        .records
                        .record(self.game.difficulty(), self.game.elapsed());
                }
            }
        }
        self.record();
        Change::Partial
    }

    fn board_in_play(&self) -> bool {
        self.panel.is_none() && matches!(self.game.status(), Status::Ready | Status::Playing)
    }

    fn board_position(&self, width: i32, x: f32, y: f32) -> Option<(usize, usize)> {
        let (rows, columns, _) = self.game.difficulty().size();
        (0..rows.min(VISIBLE))
            .flat_map(|row| (0..columns.min(VISIBLE)).map(move |column| (row, column)))
            .find(|&(row, column)| board_rect(width, rows, columns, row, column).contains(x, y))
            .map(|(row, column)| (self.row + row, self.column + column))
    }

    fn draw_tile(&self, canvas: &mut Canvas, cell: Rect, view: Tile, accepts_taps: bool) {
        let dark_tile = matches!(view, Tile::Covered | Tile::Flag | Tile::MistakenFlag);
        let fill = if dark_tile { 239 } else { PAPER };
        let line = if dark_tile { BLACK } else { HAIRLINE };
        let pressed = accepts_taps
            && matches!(view, Tile::Covered | Tile::Flag)
            && canvas.control_pressed(cell, false);
        let highlighted = pressed || view == Tile::Exploded;
        canvas.fill(cell, line);
        canvas.fill(
            cell.inset(canvas.px(2)),
            if highlighted { BLACK } else { fill },
        );
        let ink = if highlighted { PAPER } else { BLACK };
        let inside = |x, y, w, h| {
            Rect::new(
                cell.x + canvas.px(x),
                cell.y + canvas.px(y),
                canvas.px(w),
                canvas.px(h),
            )
        };
        match view {
            Tile::Flag => {
                let parts = [
                    inside(35, 23, 5, 58),
                    inside(35, 24, 35, 10),
                    inside(35, 34, 28, 10),
                    inside(35, 44, 22, 9),
                    inside(24, 77, 36, 5),
                ];
                for part in parts {
                    canvas.fill(part, ink);
                }
                return;
            }
            Tile::Mine | Tile::Exploded => {
                let body = inside(31, 31, 43, 43);
                let spike_x = inside(22, 49, 61, 7);
                let spike_y = inside(49, 22, 7, 61);
                canvas.circle(body, ink);
                canvas.fill(spike_x, ink);
                canvas.fill(spike_y, ink);
                return;
            }
            _ => {}
        }
        let mark = match view {
            Tile::MistakenFlag => "×".into(),
            Tile::Open(digit) if digit != 0 => digit.to_string(),
            _ => return,
        };
        if !mark.is_empty() {
            let text = Rect::new(
                cell.x,
                cell.y + canvas.px(13),
                cell.w,
                cell.h - canvas.px(13),
            );
            canvas.text(text, &mark, TextStyle::new(54).bold().center().ink(ink));
        }
    }
}

fn seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

fn board_rect(w: i32, rows: usize, columns: usize, row: usize, column: usize) -> Rect {
    let width = columns.min(VISIBLE) as i32 * TILE;
    let height = rows.min(VISIBLE) as i32 * TILE;
    screen_rect(
        w,
        (1264 - width) / 2 + column as i32 * TILE,
        BOARD_Y + (VISIBLE as i32 * TILE - height) / 2 + row as i32 * TILE,
        TILE,
        TILE,
    )
}

fn action(w: i32, index: i32) -> Rect {
    screen_rect(w, 40 + index * 399, 1539, 386, 100)
}

fn pan(w: i32, index: i32) -> Rect {
    screen_rect(w, 280 + index * 180, 1418, 166, 100)
}

fn choice(w: i32, index: usize) -> Rect {
    screen_rect(w, 278, 638 + index as i32 * 138, 708, 100)
}

fn dismiss(w: i32) -> Rect {
    screen_rect(w, 488, 1096, 288, 100)
}

fn clock(seconds: u64) -> String {
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

impl App for View {
    fn draw(&self, canvas: &mut Canvas) {
        let w = canvas.bounds().w;
        header(canvas, "Minesweeper", "Exit");
        let (rows, columns, mines) = self.game.difficulty().size();
        let timer = format!(
            "{} · {} × {} · {} mines",
            self.game.difficulty().name(),
            rows,
            columns,
            mines
        );
        canvas.text(screen_rect(w, 40, 150, 660, 45), &timer, CAPTION);
        let score = format!(
            "{} flags left   {}",
            self.game.flags_left(),
            clock(self.displayed_seconds)
        );
        canvas.text(
            screen_rect(w, 680, 150, 544, 45),
            &score,
            TextStyle::new(31).bold().right(),
        );
        let notice = if self.save_error {
            "Could not save this game."
        } else {
            match self.game.status() {
                Status::Ready => "Choose a square to start. The first square is safe.",
                Status::Playing if self.flag_mode => "Tap to place or remove a flag.",
                Status::Playing => "Tap to reveal. Hold to flag.",
                Status::Won if self.new_best => "Minefield cleared. New best time!",
                Status::Won => "Minefield cleared.",
                Status::Lost => "Mine hit. Start a new game to try again.",
            }
        };
        canvas.text(screen_rect(w, 40, 191, 1184, 44), notice, CAPTION);
        let accepts_taps = self.board_in_play();
        for row in 0..rows.min(VISIBLE) {
            for column in 0..columns.min(VISIBLE) {
                let view = self.game.view(self.row + row, self.column + column);
                self.draw_tile(
                    canvas,
                    board_rect(w, rows, columns, row, column),
                    view,
                    accepts_taps,
                );
            }
        }
        if rows > VISIBLE || columns > VISIBLE {
            let dirs = [
                ("Left", self.column > 0),
                ("Up", self.row > 0),
                ("Down", self.row + VISIBLE < rows),
                ("Right", self.column + VISIBLE < columns),
            ];
            for (index, (label, enabled)) in dirs.into_iter().enumerate() {
                let button = pan(w, index as i32);
                if enabled {
                    canvas.button(button, label, false, false);
                } else {
                    canvas.text(
                        Rect::new(button.x, button.y + canvas.px(25), button.w, button.h),
                        label,
                        CAPTION.center(),
                    );
                }
            }
        }
        canvas.button(action(w, 0), "Flag mode", self.flag_mode, false);
        canvas.button(action(w, 1), "Best times", false, false);
        canvas.button(action(w, 2), "New game", false, false);
        if let Some(panel) = self.panel {
            canvas.popup(screen_rect(w, POPUP.x, POPUP.y, POPUP.w, POPUP.h));
            let title = match panel {
                Panel::NewGame => "Start a new game?",
                Panel::BestTimes => "Best times",
            };
            canvas.text(
                screen_rect(w, 235, 507, 794, 90),
                title,
                TextStyle::new(50).bold().center(),
            );
            for (index, difficulty) in Difficulty::ALL.into_iter().enumerate() {
                let bounds = choice(w, index);
                match panel {
                    Panel::NewGame => {
                        let (rows, columns, _) = difficulty.size();
                        let label = format!("{} · {} × {}", difficulty.name(), rows, columns);
                        canvas.button(bounds, &label, true, false);
                    }
                    Panel::BestTimes => {
                        let best = self
                            .records
                            .get(difficulty)
                            .map_or_else(|| "—".into(), clock);
                        let label = format!("{}   {}", difficulty.name(), best);
                        canvas.text(bounds, &label, TextStyle::new(40).center());
                    }
                }
            }
            canvas.button(
                dismiss(w),
                if matches!(panel, Panel::NewGame) {
                    "Cancel"
                } else {
                    "Close"
                },
                false,
                false,
            );
        }
    }

    fn press_point(&self, input: Input) -> Option<(f32, f32)> {
        let (x, y, held) = match input {
            Input::Tap(x, y) => (x, y, false),
            Input::LongTap(x, y) => (x, y, true),
            _ => return None,
        };
        if self.board_in_play() {
            if let Some((row, column)) = self.board_position(WIDTH as i32, x, y) {
                let tile = self.game.view(row, column);
                return (tile == Tile::Covered || (tile == Tile::Flag && (held || self.flag_mode)))
                    .then_some((x, y));
            }
        }
        (!held).then_some((x, y))
    }

    fn input(&mut self, input: Input) -> Change {
        let w = WIDTH as i32;
        match input {
            Input::Sleep => {
                self.stop_clock(Instant::now());
                return Change::None;
            }
            Input::Wake if self.game.status() == Status::Playing => {
                self.started.get_or_insert_with(Instant::now);
                return Change::Full;
            }
            _ => {}
        }
        if let Some(panel) = self.panel {
            let Input::Tap(x, y) = input else {
                return Change::None;
            };
            if matches!(panel, Panel::NewGame) {
                for (index, difficulty) in Difficulty::ALL.into_iter().enumerate() {
                    if choice(w, index).contains(x, y) {
                        self.game = Minefield::new(difficulty, seed());
                        self.row = 0;
                        self.column = 0;
                        self.flag_mode = false;
                        self.panel = None;
                        self.started = None;
                        self.displayed_seconds = 0;
                        self.new_best = false;
                        self.record();
                        return Change::Full;
                    }
                }
            }
            if dismiss(w).contains(x, y)
                || !screen_rect(w, POPUP.x, POPUP.y, POPUP.w, POPUP.h).contains(x, y)
            {
                self.panel = None;
                return Change::Full;
            }
            return Change::None;
        }
        let (x, y, held) = match input {
            Input::Tap(x, y) => (x, y, false),
            Input::LongTap(x, y) => (x, y, true),
            Input::PageForward => return self.move_page(0, 1),
            Input::PageBack => return self.move_page(0, -1),
            Input::Swipe { dx, dy, start_y } if (240.0..1400.0).contains(&start_y) => {
                if dx.abs() >= dy.abs() {
                    return self.move_page(0, -(dx.signum() as i32));
                }
                return self.move_page(-(dy.signum() as i32), 0);
            }
            _ => return Change::None,
        };
        if !held {
            if back_button(w).contains(x, y) {
                return Change::Exit;
            }
            if action(w, 0).contains(x, y) {
                self.flag_mode = !self.flag_mode;
                return Change::Partial;
            }
            if [1, 2]
                .into_iter()
                .any(|index| action(w, index).contains(x, y))
            {
                self.panel = Some(if action(w, 1).contains(x, y) {
                    Panel::BestTimes
                } else {
                    Panel::NewGame
                });
                return Change::Full;
            }
            for (index, (r, c)) in [(0, -1), (-1, 0), (1, 0), (0, 1)].into_iter().enumerate() {
                if pan(w, index as i32).contains(x, y) {
                    return self.move_page(r, c);
                }
            }
        }
        if let Some((row, column)) = self.board_position(w, x, y) {
            return self.choose(row, column, held || self.flag_mode);
        }
        Change::None
    }

    fn deadline(&self) -> Option<Instant> {
        self.started?.checked_add(Duration::from_secs(
            self.displayed_seconds
                .saturating_sub(self.game.elapsed())
                .saturating_add(1),
        ))
    }

    fn tick(&mut self, now: Instant) -> Change {
        let seconds = self.elapsed(now);
        if seconds == self.displayed_seconds {
            return Change::None;
        }
        self.displayed_seconds = seconds;
        Change::Partial
    }

    fn save(&mut self) -> io::Result<()> {
        if let Some(path) = &self.data {
            let mut snapshot = self.game.clone();
            snapshot.set_elapsed(self.elapsed(Instant::now()));
            primitive_app::save(path, &serde_json::to_vec(&(snapshot, &self.records))?)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hold_feedback_targets_only_a_tile_that_can_toggle_a_flag() {
        let difficulty = Difficulty::Beginner;
        let (rows, columns, _) = difficulty.size();
        let mut view = View::new(Minefield::new(difficulty, 5), BestTimes::default(), None);
        let cell = board_rect(WIDTH as i32, rows, columns, 0, 0);
        let (x, y) = ((cell.x + cell.w / 2) as f32, (cell.y + cell.h / 2) as f32);
        assert_eq!(view.press_point(Input::LongTap(x, y)), Some((x, y)));
        assert_eq!(view.input(Input::LongTap(x, y)), Change::Partial);
        assert_eq!(view.game.view(0, 0), Tile::Flag);
        assert_eq!(view.press_point(Input::Tap(x, y)), None);
        assert_eq!(view.press_point(Input::LongTap(x, y)), Some((x, y)));
        assert_eq!(view.press_point(Input::LongTap(50.0, 50.0)), None);
        view.panel = Some(Panel::NewGame);
        assert_eq!(view.press_point(Input::LongTap(x, y)), None);
    }
}
