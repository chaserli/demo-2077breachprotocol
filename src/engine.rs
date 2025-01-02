use std::collections::HashSet;
use std::error::Error;
use std::fmt;
use std::time::{Duration, Instant};

pub type Token = String;
pub type Cell = (usize, usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConstraintMode {
    Row,
    Column,
}

#[derive(Clone, Debug)]
pub struct BreachConfig {
    pub matrix_size: usize,
    pub buffer_size: usize,
    pub time_limit_seconds: u64,
    pub token_alphabet: Vec<Token>,
    pub seed: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct BreachState {
    pub matrix: Vec<Vec<Token>>,
    pub sequences: Vec<Vec<Token>>,
    pub sequence_values: Vec<u32>,
    pub buffer_tokens: Vec<Token>,
    pub selected_cells: HashSet<Cell>,
    pub current_constraint: ConstraintMode,
    pub current_index: Option<Cell>,
    pub time_started: bool,
    pub time_started_at: Option<Instant>,
    pub time_completed_at: Option<Instant>,
    pub uploaded_results: Vec<bool>,
    pub timer_override: Option<Duration>,
}

#[derive(Debug)]
pub struct GameEngine {
    pub config: BreachConfig,
    pub state: BreachState,
}

#[derive(Debug)]
pub enum GameError {
    InvalidMove(Cell),
    InvalidConfig(String),
}

impl fmt::Display for GameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GameError::InvalidMove(cell) => write!(f, "Invalid move: {:?}", cell),
            GameError::InvalidConfig(message) => write!(f, "Invalid config: {}", message),
        }
    }
}

impl Error for GameError {}

#[derive(Debug, Clone)]
pub struct SequenceResult {
    pub sequence: Vec<Token>,
    pub uploaded: bool,
    pub value: u32,
}

#[derive(Debug, Clone)]
pub struct GameResult {
    pub success: bool,
    pub score: u32,
    pub sequences_uploaded: usize,
    pub total_sequences: usize,
    pub buffer_used: usize,
    pub buffer_full: bool,
    pub out_of_time: bool,
    pub sequence_results: Vec<SequenceResult>,
}

impl GameEngine {
    pub fn new(
        config: BreachConfig,
        matrix: Vec<Vec<Token>>,
        sequences: Vec<Vec<Token>>,
        sequence_values: Vec<u32>,
    ) -> Result<Self, GameError> {
        if config.token_alphabet.is_empty() {
            return Err(GameError::InvalidConfig(
                "Token alphabet cannot be empty".to_string(),
            ));
        }
        if matrix.len() != config.matrix_size {
            return Err(GameError::InvalidConfig(format!(
                "Matrix size mismatch: expected {}",
                config.matrix_size
            )));
        }
        for row in &matrix {
            if row.len() != config.matrix_size {
                return Err(GameError::InvalidConfig(format!(
                    "Matrix row size mismatch: expected {}",
                    config.matrix_size
                )));
            }
        }
        if sequences.len() != sequence_values.len() {
            return Err(GameError::InvalidConfig(
                "Sequences and values must align".to_string(),
            ));
        }

        let uploaded_results = vec![false; sequences.len()];

        let state = BreachState {
            matrix,
            sequences,
            sequence_values,
            buffer_tokens: Vec::new(),
            selected_cells: HashSet::new(),
            current_constraint: ConstraintMode::Row,
            current_index: None,
            time_started: false,
            time_started_at: None,
            time_completed_at: None,
            uploaded_results,
            timer_override: None,
        };

        Ok(Self { config, state })
    }

    pub fn legal_moves(&self) -> HashSet<Cell> {
        if self.state.buffer_tokens.is_empty() {
            return (0..self.config.matrix_size).map(|col| (0, col)).collect();
        }

        match self.state.current_constraint {
            ConstraintMode::Row => {
                if let Some((row, _)) = self.state.current_index {
                    (0..self.config.matrix_size)
                        .map(|col| (row, col))
                        .filter(|cell| !self.state.selected_cells.contains(cell))
                        .collect()
                } else {
                    HashSet::new()
                }
            }
            ConstraintMode::Column => {
                if let Some((_, col)) = self.state.current_index {
                    (0..self.config.matrix_size)
                        .map(|row| (row, col))
                        .filter(|cell| !self.state.selected_cells.contains(cell))
                        .collect()
                } else {
                    HashSet::new()
                }
            }
        }
    }

    pub fn is_valid_move(&self, cell: Cell) -> bool {
        self.legal_moves().contains(&cell)
    }

    pub fn apply_move(&mut self, cell: Cell) -> Result<(), GameError> {
        if !self.is_valid_move(cell) {
            return Err(GameError::InvalidMove(cell));
        }

        if !self.state.time_started {
            self.state.time_started = true;
            self.state.time_started_at = Some(Instant::now());
        }

        let (row, col) = cell;
        let token = self.state.matrix[row][col].clone();
        self.state.buffer_tokens.push(token);
        self.state.selected_cells.insert(cell);
        self.state.current_index = Some(cell);

        self.evaluate_uploaded();

        if self.state.uploaded_results.iter().all(|&done| done)
            && self.state.time_completed_at.is_none()
        {
            self.state.time_completed_at = Some(Instant::now());
        }

        self.state.current_constraint = match self.state.current_constraint {
            ConstraintMode::Row => ConstraintMode::Column,
            ConstraintMode::Column => ConstraintMode::Row,
        };

        Ok(())
    }

    pub fn get_remaining_time(&self) -> Duration {
        if self.config.time_limit_seconds == 0 {
            return Duration::MAX;
        }

        if !self.state.time_started {
            return Duration::from_secs(self.config.time_limit_seconds);
        }

        if let Some(override_time) = self.state.timer_override {
            return override_time;
        }

        if let Some(started) = self.state.time_started_at {
            let elapsed = if let Some(completed) = self.state.time_completed_at {
                completed.duration_since(started)
            } else {
                started.elapsed()
            };
            let total = Duration::from_secs(self.config.time_limit_seconds);
            if elapsed >= total {
                Duration::ZERO
            } else {
                total - elapsed
            }
        } else {
            Duration::from_secs(self.config.time_limit_seconds)
        }
    }

    pub fn is_terminal(&self) -> bool {
        if self.config.time_limit_seconds != 0 && self.get_remaining_time().is_zero() {
            return true;
        }

        if self.state.buffer_tokens.len() >= self.config.buffer_size {
            return true;
        }

        if self.legal_moves().is_empty() {
            return true;
        }

        self.state.uploaded_results.iter().all(|uploaded| *uploaded)
    }

    pub fn evaluate_uploaded(&mut self) {
        let buffer = self.state.buffer_tokens.join("");

        for (idx, sequence) in self.state.sequences.iter().enumerate() {
            if self.state.uploaded_results[idx] {
                continue;
            }

            let sequence_str = sequence.join("");
            if buffer.contains(&sequence_str) {
                self.state.uploaded_results[idx] = true;
            }
        }
    }

    pub fn compute_score(&self) -> u32 {
        self.state
            .uploaded_results
            .iter()
            .zip(&self.state.sequence_values)
            .filter_map(|(uploaded, value)| if *uploaded { Some(*value) } else { None })
            .sum()
    }

    pub fn get_game_result(&self) -> GameResult {
        let score = self.compute_score();
        let uploaded = self.state.uploaded_results.iter().filter(|&&u| u).count();

        let sequence_results = self
            .state
            .sequences
            .iter()
            .zip(&self.state.uploaded_results)
            .zip(&self.state.sequence_values)
            .map(|((sequence, uploaded), value)| SequenceResult {
                sequence: sequence.clone(),
                uploaded: *uploaded,
                value: *value,
            })
            .collect();

        GameResult {
            success: score > 0,
            score,
            sequences_uploaded: uploaded,
            total_sequences: self.state.sequences.len(),
            buffer_used: self.state.buffer_tokens.len(),
            buffer_full: self.state.buffer_tokens.len() >= self.config.buffer_size,
            out_of_time: self.get_remaining_time().is_zero(),
            sequence_results,
        }
    }

    pub fn set_timer_override(&mut self, duration: Duration) {
        self.state.timer_override = Some(duration);
    }
}
