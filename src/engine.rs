use std::collections::HashSet;
use std::error::Error;
use std::fmt;

pub type Token = String;
pub type Cell = (usize, usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ConstraintMode {
    Row,
    Column,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TerminalReason {
    AllUploaded,
    BufferFull,
    OutOfTime,
    NoLegalMoves,
    NoCompletableSequences,
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
    pub uploaded_results: Vec<bool>,
    pub terminal_reason: Option<TerminalReason>,
}

#[derive(Clone, Debug)]
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
    pub terminal_reason: Option<TerminalReason>,
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
            uploaded_results,
            terminal_reason: None,
        };

        let mut engine = Self { config, state };
        engine.update_terminal_reason();
        Ok(engine)
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

    pub fn has_legal_moves(&self) -> bool {
        if self.state.buffer_tokens.is_empty() {
            return self.config.matrix_size > 0;
        }

        match self.state.current_constraint {
            ConstraintMode::Row => {
                if let Some((row, _)) = self.state.current_index {
                    (0..self.config.matrix_size)
                        .any(|col| !self.state.selected_cells.contains(&(row, col)))
                } else {
                    false
                }
            }
            ConstraintMode::Column => {
                if let Some((_, col)) = self.state.current_index {
                    (0..self.config.matrix_size)
                        .any(|row| !self.state.selected_cells.contains(&(row, col)))
                } else {
                    false
                }
            }
        }
    }

    pub fn apply_move(&mut self, cell: Cell) -> Result<(), GameError> {
        if self.state.terminal_reason.is_some() {
            return Err(GameError::InvalidMove(cell));
        }

        if !self.is_valid_move(cell) {
            return Err(GameError::InvalidMove(cell));
        }

        let (row, col) = cell;
        let token = self.state.matrix[row][col].clone();
        self.state.buffer_tokens.push(token);
        self.state.selected_cells.insert(cell);
        self.state.current_index = Some(cell);

        self.evaluate_uploaded();

        self.state.current_constraint = match self.state.current_constraint {
            ConstraintMode::Row => ConstraintMode::Column,
            ConstraintMode::Column => ConstraintMode::Row,
        };
        self.update_terminal_reason();

        Ok(())
    }

    pub fn is_terminal(&self) -> bool {
        self.state.terminal_reason.is_some()
    }

    pub fn evaluate_uploaded(&mut self) {
        for (idx, sequence) in self.state.sequences.iter().enumerate() {
            if self.state.uploaded_results[idx] {
                continue;
            }
            if sequence_in_buffer(&self.state.buffer_tokens, sequence) {
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
            out_of_time: self.state.terminal_reason == Some(TerminalReason::OutOfTime),
            terminal_reason: self.state.terminal_reason,
            sequence_results,
        }
    }

    pub fn terminal_reason(&self) -> Option<TerminalReason> {
        self.state.terminal_reason
    }

    pub fn force_timeout(&mut self) {
        if self.state.terminal_reason.is_none() {
            self.state.terminal_reason = Some(TerminalReason::OutOfTime);
        }
    }

    pub fn has_uploaded_all_sequences(&self) -> bool {
        !self.state.sequences.is_empty()
            && self.state.uploaded_results.iter().all(|uploaded| *uploaded)
    }

    fn has_unuploaded_sequences(&self) -> bool {
        self.state
            .uploaded_results
            .iter()
            .any(|uploaded| !*uploaded)
    }

    pub fn can_sequence_still_upload(&self, sequence_index: usize) -> bool {
        let Some(sequence) = self.state.sequences.get(sequence_index) else {
            return false;
        };
        if self
            .state
            .uploaded_results
            .get(sequence_index)
            .copied()
            .unwrap_or(false)
        {
            return true;
        }

        sequence_can_still_upload(
            &self.state.buffer_tokens,
            sequence,
            self.config
                .buffer_size
                .saturating_sub(self.state.buffer_tokens.len()),
        )
    }

    pub fn sequence_progress(&self, sequence_index: usize) -> usize {
        let Some(sequence) = self.state.sequences.get(sequence_index) else {
            return 0;
        };
        sequence_suffix_progress(&self.state.buffer_tokens, sequence)
    }

    pub fn has_completable_sequence(&self) -> bool {
        self.state.sequences.iter().enumerate().any(|(idx, _)| {
            !self.state.uploaded_results[idx] && self.can_sequence_still_upload(idx)
        })
    }

    fn update_terminal_reason(&mut self) {
        if self.state.terminal_reason == Some(TerminalReason::OutOfTime) {
            return;
        }

        self.state.terminal_reason = if self.has_uploaded_all_sequences() {
            Some(TerminalReason::AllUploaded)
        } else if self.state.buffer_tokens.len() >= self.config.buffer_size {
            Some(TerminalReason::BufferFull)
        } else if !self.has_legal_moves() {
            Some(TerminalReason::NoLegalMoves)
        } else if self.has_unuploaded_sequences() && !self.has_completable_sequence() {
            Some(TerminalReason::NoCompletableSequences)
        } else {
            None
        };
    }
}

fn sequence_in_buffer(buffer: &[Token], sequence: &[Token]) -> bool {
    !sequence.is_empty()
        && sequence.len() <= buffer.len()
        && buffer
            .windows(sequence.len())
            .any(|window| window == sequence)
}

fn sequence_can_still_upload(buffer: &[Token], sequence: &[Token], remaining_slots: usize) -> bool {
    if sequence.is_empty() || sequence_in_buffer(buffer, sequence) {
        return true;
    }
    if sequence.len() <= remaining_slots {
        return true;
    }

    let progress = sequence_suffix_progress(buffer, sequence);
    progress > 0 && sequence.len().saturating_sub(progress) <= remaining_slots
}

fn sequence_suffix_progress(buffer: &[Token], sequence: &[Token]) -> usize {
    if sequence.is_empty() {
        return 0;
    }
    let max_len = sequence.len().min(buffer.len());
    for len in (1..=max_len).rev() {
        if buffer[buffer.len() - len..] == sequence[..len] {
            return len;
        }
    }
    0
}
