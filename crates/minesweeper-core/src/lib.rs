//! Classic minefield rules and records.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Difficulty {
    Beginner,
    Intermediate,
    Expert,
}

impl Difficulty {
    pub const ALL: [Self; 3] = [Self::Beginner, Self::Intermediate, Self::Expert];

    pub fn name(self) -> &'static str {
        match self {
            Self::Beginner => "Beginner",
            Self::Intermediate => "Intermediate",
            Self::Expert => "Expert",
        }
    }

    pub const fn size(self) -> (usize, usize, usize) {
        match self {
            Self::Beginner => (9, 9, 10),
            Self::Intermediate => (16, 16, 40),
            Self::Expert => (16, 30, 99),
        }
    }

    pub fn index(self) -> usize {
        self as usize
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    Ready,
    Playing,
    Won,
    Lost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Covered,
    Flag,
    Open(u8),
    Mine,
    Exploded,
    MistakenFlag,
}

#[derive(Default, Clone, Serialize, Deserialize)]
struct Cell {
    mine: bool,
    flag: bool,
    open: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Minefield {
    difficulty: Difficulty,
    seed: u64,
    elapsed: u64,
    status: Status,
    exploded: Option<usize>,
    cells: Vec<Cell>,
}

impl Minefield {
    pub fn new(difficulty: Difficulty, seed: u64) -> Self {
        let (rows, columns, _) = difficulty.size();
        Self {
            difficulty,
            seed: seed.max(1),
            elapsed: 0,
            status: Status::Ready,
            exploded: None,
            cells: vec![Cell::default(); rows * columns],
        }
    }

    pub fn is_valid(&self) -> bool {
        let (rows, columns, mines) = self.difficulty.size();
        if self.cells.len() != rows * columns || self.seed == 0 {
            return false;
        }
        if self
            .cells
            .iter()
            .any(|cell| cell.open && (cell.mine || cell.flag))
        {
            return false;
        }
        let actual_mines = self.cells.iter().filter(|cell| cell.mine).count();
        if self.status == Status::Ready {
            return actual_mines == 0
                && self.exploded.is_none()
                && self.elapsed == 0
                && self.cells.iter().all(|cell| !cell.open);
        }
        if actual_mines != mines {
            return false;
        }
        match self.status {
            Status::Playing => self.exploded.is_none() && !self.all_clear(),
            Status::Won => self.exploded.is_none() && self.all_clear(),
            Status::Lost => self
                .exploded
                .and_then(|index| self.cells.get(index))
                .is_some_and(|c| c.mine),
            Status::Ready => false,
        }
    }

    pub fn difficulty(&self) -> Difficulty {
        self.difficulty
    }

    pub fn status(&self) -> Status {
        self.status
    }

    pub fn elapsed(&self) -> u64 {
        self.elapsed
    }

    pub fn set_elapsed(&mut self, seconds: u64) {
        self.elapsed = seconds;
    }

    pub fn flags_left(&self) -> i32 {
        self.difficulty.size().2 as i32 - self.cells.iter().filter(|cell| cell.flag).count() as i32
    }

    pub fn view(&self, row: usize, column: usize) -> View {
        let index = self.index(row, column).expect("board coordinate");
        let cell = &self.cells[index];
        if self.exploded == Some(index) {
            View::Exploded
        } else if cell.flag {
            if self.status == Status::Lost && !cell.mine {
                View::MistakenFlag
            } else {
                View::Flag
            }
        } else if cell.open {
            View::Open(self.nearby_mines(index))
        } else if self.status == Status::Lost && cell.mine {
            View::Mine
        } else {
            View::Covered
        }
    }

    pub fn flag(&mut self, row: usize, column: usize) -> bool {
        let Some(index) = self.index(row, column) else {
            return false;
        };
        if matches!(self.status, Status::Won | Status::Lost) || self.cells[index].open {
            return false;
        }
        self.cells[index].flag = !self.cells[index].flag;
        true
    }

    pub fn reveal(&mut self, row: usize, column: usize) -> bool {
        let Some(index) = self.index(row, column) else {
            return false;
        };
        if matches!(self.status, Status::Won | Status::Lost)
            || self.cells[index].flag
            || self.cells[index].open
        {
            return false;
        }
        if self.status == Status::Ready {
            self.seed_mines(index);
            self.status = Status::Playing;
        }
        if self.cells[index].mine {
            self.status = Status::Lost;
            self.exploded = Some(index);
            return true;
        }
        let mut pending = vec![index];
        while let Some(tile) = pending.pop() {
            let cell = &mut self.cells[tile];
            if cell.open || cell.flag || cell.mine {
                continue;
            }
            cell.open = true;
            if self.nearby_mines(tile) == 0 {
                pending.extend(self.neighbors(tile));
            }
        }
        if self.all_clear() {
            self.status = Status::Won;
        }
        true
    }

    fn index(&self, row: usize, column: usize) -> Option<usize> {
        let (rows, columns, _) = self.difficulty.size();
        (row < rows && column < columns)
            .then_some(row.saturating_mul(columns).saturating_add(column))
    }

    fn all_clear(&self) -> bool {
        self.cells.iter().all(|cell| cell.mine || cell.open)
    }

    fn nearby_mines(&self, index: usize) -> u8 {
        self.neighbors(index)
            .filter(|&i| self.cells[i].mine)
            .count() as u8
    }

    fn neighbors(&self, index: usize) -> impl Iterator<Item = usize> + use<> {
        let (rows, columns, _) = self.difficulty.size();
        let (row, column) = (index / columns, index % columns);
        let row_end = (row + 1).min(rows - 1);
        let col_start = column.saturating_sub(1);
        let col_end = (column + 1).min(columns - 1);
        (row.saturating_sub(1)..=row_end).flat_map(move |r| {
            (col_start..=col_end)
                .filter_map(move |c| (r != row || c != column).then_some(r * columns + c))
        })
    }

    fn seed_mines(&mut self, safe: usize) {
        let (_, _, count) = self.difficulty.size();
        let safe_neighbors: Vec<_> = self.neighbors(safe).collect();
        let mut positions: Vec<_> = (0..self.cells.len())
            .filter(|i| *i != safe && !safe_neighbors.contains(i))
            .collect();
        let mut seed = self.seed;
        for i in 0..count {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let selected = i + (seed % (positions.len() - i) as u64) as usize;
            positions.swap(i, selected);
            self.cells[positions[i]].mine = true;
        }
    }
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct BestTimes([Option<u64>; 3]);

impl BestTimes {
    pub fn get(&self, difficulty: Difficulty) -> Option<u64> {
        self.0[difficulty.index()]
    }

    pub fn record(&mut self, difficulty: Difficulty, seconds: u64) -> bool {
        let best = &mut self.0[difficulty.index()];
        if best.is_none_or(|previous| seconds < previous) {
            *best = Some(seconds);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_tile_and_neighbors_are_safe_at_each_size() {
        for difficulty in Difficulty::ALL {
            let (rows, columns, mines) = difficulty.size();
            for (r, c) in [(0, 0), (rows / 2, columns / 2), (rows - 1, columns - 1)] {
                let mut game = Minefield::new(difficulty, 7);
                assert!(game.flag(r, c));
                assert!(!game.reveal(r, c));
                assert_eq!(game.status(), Status::Ready);
                assert!(game.flag(r, c));
                assert!(game.reveal(r, c));
                assert_eq!(game.view(r, c), View::Open(0));
                assert_eq!(game.cells.iter().filter(|cell| cell.mine).count(), mines);
                assert!(game.is_valid());
            }
        }
    }

    #[test]
    fn opening_a_mine_ends_play_and_shows_uncovered_mines() {
        let mut game = Minefield::new(Difficulty::Beginner, 42);
        game.reveal(0, 0);
        let mine = game.cells.iter().position(|c| c.mine).unwrap();
        let safe = game.cells.iter().position(|c| !c.mine && !c.open).unwrap();
        assert!(game.flag(safe / 9, safe % 9));
        assert!(game.reveal(mine / 9, mine % 9));
        assert_eq!(game.status(), Status::Lost);
        assert_eq!(game.view(mine / 9, mine % 9), View::Exploded);
        assert_eq!(game.view(safe / 9, safe % 9), View::MistakenFlag);
        assert!(!game.flag(0, 0));
        assert!(game.is_valid());
    }

    #[test]
    fn clearing_safe_tiles_wins_and_records_are_kept_by_size() {
        let mut game = Minefield::new(Difficulty::Intermediate, 1);
        game.reveal(0, 0);
        for index in 0..game.cells.len() {
            if !game.cells[index].mine {
                game.reveal(index / 16, index % 16);
            }
        }
        assert_eq!(game.status(), Status::Won);
        assert!(game.is_valid());
        let mut records = BestTimes::default();
        assert!(records.record(game.difficulty(), 66));
        assert!(!records.record(game.difficulty(), 70));
        assert!(records.record(game.difficulty(), 43));
        assert_eq!(records.get(Difficulty::Intermediate), Some(43));
        assert_eq!(records.get(Difficulty::Beginner), None);
    }
}
