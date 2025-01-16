use rust_breach_protocol::engine::{BreachConfig, GameEngine};
use rust_breach_protocol::generator::{PuzzleSpec, generate_verified_puzzle, sequence_lengths_for};
use rust_breach_protocol::solver::SolverOptions;

const TIME_LIMIT_OPTIONS: [u64; 5] = [0, 15, 30, 45, 60];
const TOKENS: [&str; 10] = ["55", "1C", "7A", "BD", "E9", "FF", "00", "9A", "A1", "C3"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Field {
    MatrixSize,
    BufferSize,
    Daemons,
    TokenVariety,
    TimeLimit,
}

impl Field {
    pub(super) const ALL: [Field; 5] = [
        Field::MatrixSize,
        Field::BufferSize,
        Field::Daemons,
        Field::TokenVariety,
        Field::TimeLimit,
    ];

    pub(super) fn label(self) -> &'static str {
        match self {
            Field::MatrixSize => "Matrix Size",
            Field::BufferSize => "Buffer Size",
            Field::Daemons => "Daemons",
            Field::TokenVariety => "Token Variety",
            Field::TimeLimit => "Time Limit",
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct Settings {
    pub(super) matrix_size: usize,
    pub(super) buffer_size: usize,
    pub(super) time_limit_index: usize,
    pub(super) sequences: usize,
    pub(super) token_variety: usize,
}

impl Default for Settings {
    fn default() -> Self {
        let sequences = 2;
        Self {
            matrix_size: 5,
            buffer_size: buffer_bounds_for(5, sequences).1,
            time_limit_index: 2,
            sequences,
            token_variety: 6,
        }
    }
}

impl Settings {
    pub(super) fn step(&mut self, field: Field, delta: i32) -> bool {
        if delta == 0 || !self.can_step(field, delta) {
            return false;
        }
        match field {
            Field::MatrixSize => {
                self.matrix_size = (self.matrix_size as i32 + delta).clamp(3, 9) as usize;
                self.clamp_buffer_size();
            }
            Field::BufferSize => {
                let bounds = self.buffer_bounds();
                self.buffer_size = (self.buffer_size as i32 + delta)
                    .clamp(bounds.0 as i32, bounds.1 as i32)
                    as usize;
            }
            Field::Daemons => {
                self.sequences = (self.sequences as i32 + delta).clamp(1, 5) as usize;
                self.clamp_buffer_size();
            }
            Field::TokenVariety => {
                self.token_variety = (self.token_variety as i32 + delta).clamp(2, 10) as usize;
            }
            Field::TimeLimit => {
                self.time_limit_index = (self.time_limit_index as i32 + delta)
                    .clamp(0, (TIME_LIMIT_OPTIONS.len() - 1) as i32)
                    as usize;
            }
        }
        true
    }

    pub(super) fn can_step(&self, field: Field, delta: i32) -> bool {
        match field {
            Field::MatrixSize => can_step(self.matrix_size, 3, 9, delta),
            Field::BufferSize => {
                let (min, max) = self.buffer_bounds();
                can_step(self.buffer_size, min, max, delta)
            }
            Field::Daemons => can_step(self.sequences, 1, 5, delta),
            Field::TokenVariety => can_step(self.token_variety, 2, 10, delta),
            Field::TimeLimit => {
                let next = self.time_limit_index as i32 + delta;
                (0..TIME_LIMIT_OPTIONS.len() as i32).contains(&next)
            }
        }
    }

    pub(super) fn value_label(&self, field: Field) -> String {
        match field {
            Field::MatrixSize => format!("{}x{}", self.matrix_size, self.matrix_size),
            Field::BufferSize => self.buffer_size.to_string(),
            Field::Daemons => self.sequences.to_string(),
            Field::TokenVariety => self.token_variety.to_string(),
            Field::TimeLimit => match TIME_LIMIT_OPTIONS[self.time_limit_index] {
                0 => "No Limit".to_string(),
                seconds => format!("{seconds}s"),
            },
        }
    }

    fn clamp_buffer_size(&mut self) {
        let (min, max) = self.buffer_bounds();
        self.buffer_size = self.buffer_size.clamp(min, max);
    }

    fn buffer_bounds(&self) -> (usize, usize) {
        buffer_bounds_for(self.matrix_size, self.sequences)
    }
}

fn can_step(value: usize, min: usize, max: usize, delta: i32) -> bool {
    let next = value as i32 + delta;
    (min as i32..=max as i32).contains(&next)
}

pub(super) fn buffer_bounds_for(matrix_size: usize, sequences: usize) -> (usize, usize) {
    let lengths = sequence_lengths_for(sequences, usize::MAX);
    let longest = lengths.iter().copied().max().unwrap_or(1);
    let total_tokens: usize = lengths.iter().sum();
    let path_cap = matrix_size
        .saturating_mul(matrix_size)
        .saturating_sub(1)
        .max(1);
    let max = total_tokens.min(path_cap).min(16).max(longest);
    (longest, max)
}

pub(super) fn build_engine(settings: &Settings, seed: Option<u64>) -> Result<GameEngine, String> {
    let config = BreachConfig {
        matrix_size: settings.matrix_size,
        buffer_size: settings.buffer_size,
        time_limit_seconds: TIME_LIMIT_OPTIONS[settings.time_limit_index],
        token_alphabet: TOKENS[..settings.token_variety]
            .iter()
            .map(|token| (*token).to_string())
            .collect(),
        seed,
    };
    let spec = PuzzleSpec::new(config.clone(), settings.sequences)
        .with_solver_options(SolverOptions {
            node_limit: 25_000,
            solution_cap: 500,
        })
        .with_attempt_budget(32)
        .allow_uncertain_difficulty(true);
    let puzzle = generate_verified_puzzle(&spec).map_err(|err| err.to_string())?;
    GameEngine::new(config, puzzle.matrix, puzzle.sequences, puzzle.values)
        .map_err(|err| err.to_string())
}

#[cfg(test)]
mod tests {
    use macroquad::prelude::vec2;

    use super::super::layout::{SettingsHit, settings_hit, settings_layout};
    use super::*;

    #[test]
    fn settings_hit_test_matches_layout() {
        let settings = Settings::default();
        let layout = settings_layout(1440.0, 900.0);
        let matrix_row = layout
            .rows
            .iter()
            .find(|row| row.field == Field::MatrixSize)
            .unwrap();
        assert_eq!(
            settings_hit(
                &settings,
                &layout,
                vec2(
                    matrix_row.plus_button.x + matrix_row.plus_button.w * 0.5,
                    matrix_row.plus_button.y + matrix_row.plus_button.h * 0.5,
                ),
            ),
            Some(SettingsHit::Step(Field::MatrixSize, 1)),
        );
        assert_eq!(
            settings_hit(
                &settings,
                &layout,
                vec2(
                    layout.start_button.x + layout.start_button.w * 0.5,
                    layout.start_button.y + layout.start_button.h * 0.5,
                ),
            ),
            Some(SettingsHit::Start),
        );
    }

    #[test]
    fn buffer_bounds_follow_sequence_token_budget() {
        assert_eq!(buffer_bounds_for(5, 1), (2, 2));
        assert_eq!(buffer_bounds_for(5, 2), (2, 4));
        assert_eq!(buffer_bounds_for(5, 3), (3, 7));
        assert_eq!(buffer_bounds_for(5, 4), (4, 12));
        assert_eq!(buffer_bounds_for(5, 5), (5, 16));
        assert_eq!(buffer_bounds_for(3, 5), (5, 8));
    }

    #[test]
    fn settings_clamp_buffer_when_daemon_count_changes() {
        let mut settings = Settings::default();
        assert_eq!(settings.buffer_size, 4);
        assert!(settings.step(Field::Daemons, 1));
        assert_eq!(settings.buffer_size, 4);
        assert!(settings.step(Field::BufferSize, 1));
        assert_eq!(settings.buffer_size, 5);
        assert!(settings.step(Field::Daemons, -1));
        assert_eq!(settings.buffer_size, 4);
        assert!(!settings.can_step(Field::BufferSize, 1));
    }

    #[test]
    fn settings_clamp_buffer_when_matrix_shrinks() {
        let mut settings = Settings::default();
        assert!(settings.step(Field::Daemons, 1));
        while settings.can_step(Field::BufferSize, 1) {
            assert!(settings.step(Field::BufferSize, 1));
        }
        assert_eq!(settings.buffer_size, 7);

        while settings.can_step(Field::MatrixSize, -1) {
            assert!(settings.step(Field::MatrixSize, -1));
        }
        assert_eq!(settings.matrix_size, 3);
        assert_eq!(settings.buffer_size, 7);

        assert!(settings.step(Field::Daemons, 1));
        while settings.can_step(Field::BufferSize, 1) {
            assert!(settings.step(Field::BufferSize, 1));
        }
        assert_eq!(settings.buffer_size, 8);
    }
}
