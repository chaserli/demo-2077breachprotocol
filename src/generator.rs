use std::collections::HashSet;

use rand::prelude::*;

use crate::engine::{BreachConfig, Cell, GameEngine, GameError, Token};
use crate::solver::{DifficultyTier, SolveReport, SolverOptions, analyze_puzzle};

pub type Matrix = Vec<Vec<Token>>;
pub type Sequences = Vec<Vec<Token>>;
pub type SequenceValues = Vec<u32>;
pub type GameData = (Matrix, Sequences, SequenceValues);
pub type GameDataWithPath = (Matrix, Sequences, SequenceValues, Vec<Cell>);

#[derive(Clone, Debug)]
pub struct PuzzleSpec {
    pub config: BreachConfig,
    pub num_sequences: usize,
    pub require_all_sequences: bool,
    pub solver_options: SolverOptions,
    pub max_attempts: usize,
    pub allow_uncertain_difficulty: bool,
}

impl PuzzleSpec {
    pub fn new(config: BreachConfig, num_sequences: usize) -> Self {
        Self {
            config,
            num_sequences,
            require_all_sequences: true,
            solver_options: SolverOptions::default(),
            max_attempts: 300,
            allow_uncertain_difficulty: false,
        }
    }

    pub fn with_solver_options(mut self, solver_options: SolverOptions) -> Self {
        self.solver_options = solver_options;
        self
    }

    pub fn with_attempt_budget(mut self, max_attempts: usize) -> Self {
        self.max_attempts = max_attempts.max(1);
        self
    }

    pub fn allow_uncertain_difficulty(mut self, allow: bool) -> Self {
        self.allow_uncertain_difficulty = allow;
        self
    }
}

#[derive(Clone, Debug)]
pub struct GeneratedPuzzle {
    pub matrix: Matrix,
    pub sequences: Sequences,
    pub values: SequenceValues,
    pub planted_path: Vec<Cell>,
    pub solve_report: SolveReport,
}

fn seeded_rng(seed: Option<u64>) -> StdRng {
    match seed {
        Some(value) => StdRng::seed_from_u64(value),
        None => StdRng::from_entropy(),
    }
}

fn validate_generation_request(
    config: &BreachConfig,
    num_sequences: usize,
) -> Result<(), GameError> {
    if config.matrix_size == 0 {
        return Err(GameError::InvalidConfig(
            "Matrix size must be greater than zero".to_string(),
        ));
    }
    if config.buffer_size == 0 {
        return Err(GameError::InvalidConfig(
            "Buffer size must be greater than zero".to_string(),
        ));
    }
    if config.token_alphabet.is_empty() {
        return Err(GameError::InvalidConfig(
            "Token alphabet cannot be empty".to_string(),
        ));
    }
    if num_sequences == 0 {
        return Err(GameError::InvalidConfig(
            "At least one sequence is required".to_string(),
        ));
    }
    if num_sequences > u64::BITS as usize {
        return Err(GameError::InvalidConfig(format!(
            "At most {} sequences can be analyzed",
            u64::BITS
        )));
    }
    Ok(())
}

fn generate_valid_path(matrix_size: usize, path_length: usize, rng: &mut StdRng) -> Vec<Cell> {
    let mut best_path = Vec::new();
    if matrix_size == 0 || path_length == 0 {
        return best_path;
    }

    for _ in 0..300 {
        let mut path = Vec::with_capacity(path_length);
        let mut visited = HashSet::new();

        let start_col = rng.gen_range(0..matrix_size);
        let mut current = (0, start_col);
        path.push(current);
        visited.insert(current);

        let mut column_constraint = true;

        for _ in 1..path_length {
            let mut available: Vec<Cell> = Vec::new();
            if column_constraint {
                let col = current.1;
                for row in 0..matrix_size {
                    let cell = (row, col);
                    if !visited.contains(&cell) {
                        available.push(cell);
                    }
                }
            } else {
                let row = current.0;
                for col in 0..matrix_size {
                    let cell = (row, col);
                    if !visited.contains(&cell) {
                        available.push(cell);
                    }
                }
            }

            if available.is_empty() {
                break;
            }

            current = *available.choose(rng).expect("available not empty");
            path.push(current);
            visited.insert(current);
            column_constraint = !column_constraint;
        }

        if path.len() > best_path.len() {
            best_path = path.clone();
        }
        if path.len() == path_length {
            return path;
        }
    }

    best_path
}

pub fn generate_matrix(config: &BreachConfig, rng: &mut StdRng) -> Result<Matrix, GameError> {
    validate_generation_request(config, 1)?;

    let mut matrix = Vec::with_capacity(config.matrix_size);
    for _ in 0..config.matrix_size {
        let mut row = Vec::with_capacity(config.matrix_size);
        for _ in 0..config.matrix_size {
            row.push(
                config
                    .token_alphabet
                    .choose(rng)
                    .expect("Token alphabet cannot be empty")
                    .clone(),
            );
        }
        matrix.push(row);
    }
    Ok(matrix)
}

pub fn generate_sequences(
    config: &BreachConfig,
    matrix: &[Vec<Token>],
    num_sequences: usize,
    rng: &mut StdRng,
) -> Result<(Sequences, SequenceValues), GameError> {
    validate_generation_request(config, num_sequences)?;
    let all_tokens: Vec<Token> = matrix.iter().flatten().cloned().collect();
    if all_tokens.is_empty() {
        return Err(GameError::InvalidConfig(
            "Matrix must contain tokens to build sequences".to_string(),
        ));
    }

    let mut sequences = Vec::with_capacity(num_sequences);
    let mut values = Vec::with_capacity(num_sequences);
    for length in sequence_lengths_for(num_sequences, config.buffer_size) {
        let mut sequence = Vec::with_capacity(length);
        for _ in 0..length {
            sequence.push(
                all_tokens
                    .choose(rng)
                    .expect("Matrix token list cannot be empty")
                    .clone(),
            );
        }
        values.push(length as u32 * 10);
        sequences.push(sequence);
    }
    Ok((sequences, values))
}

pub fn create_random_game(
    config: &BreachConfig,
    num_sequences: usize,
) -> Result<GameData, GameError> {
    generate_solvable_game(config, num_sequences, true)
}

pub fn generate_solvable_game(
    config: &BreachConfig,
    num_sequences: usize,
    ensure_all_solvable: bool,
) -> Result<GameData, GameError> {
    let spec = PuzzleSpec {
        config: config.clone(),
        num_sequences,
        require_all_sequences: ensure_all_solvable,
        solver_options: SolverOptions::default(),
        max_attempts: 300,
        allow_uncertain_difficulty: false,
    };
    let puzzle = generate_verified_puzzle(&spec)?;
    Ok((puzzle.matrix, puzzle.sequences, puzzle.values))
}

pub fn generate_solvable_game_with_path(
    config: &BreachConfig,
    num_sequences: usize,
    ensure_all_solvable: bool,
) -> Result<GameDataWithPath, GameError> {
    let spec = PuzzleSpec {
        config: config.clone(),
        num_sequences,
        require_all_sequences: ensure_all_solvable,
        solver_options: SolverOptions::default(),
        max_attempts: 300,
        allow_uncertain_difficulty: false,
    };
    let puzzle = generate_verified_puzzle(&spec)?;
    Ok((
        puzzle.matrix,
        puzzle.sequences,
        puzzle.values,
        puzzle.planted_path,
    ))
}

pub fn generate_verified_puzzle(spec: &PuzzleSpec) -> Result<GeneratedPuzzle, GameError> {
    validate_generation_request(&spec.config, spec.num_sequences)?;

    let sequence_lengths = sequence_lengths_for(spec.num_sequences, spec.config.buffer_size);
    let max_sequence_len = sequence_lengths.iter().copied().max().unwrap_or(0);
    let cell_count = spec.config.matrix_size * spec.config.matrix_size;
    let max_path_len = if cell_count > 1 {
        spec.config.buffer_size.min(cell_count - 1)
    } else {
        1
    };
    if max_sequence_len > max_path_len {
        return Err(GameError::InvalidConfig(format!(
            "Longest sequence length {max_sequence_len} does not fit available path length {max_path_len}"
        )));
    }

    let mut rng = seeded_rng(spec.config.seed);
    for attempt in 0..spec.max_attempts {
        if let Some(seed) = spec.config.seed {
            rng = seeded_rng(Some(seed.wrapping_add(attempt as u64)));
        }

        let mut matrix = generate_matrix(&spec.config, &mut rng)?;
        let target_path_len = max_path_len.max(max_sequence_len);
        let path = generate_valid_path(spec.config.matrix_size, target_path_len, &mut rng);
        if path.len() < target_path_len {
            continue;
        }

        let buffer_tokens = generate_path_tokens(&spec.config.token_alphabet, path.len(), &mut rng);
        for (idx, &(row, col)) in path.iter().enumerate() {
            matrix[row][col] = buffer_tokens[idx].clone();
        }

        let Some(windows) =
            choose_sequence_windows(&sequence_lengths, buffer_tokens.len(), &mut rng)
        else {
            continue;
        };
        let mut sequences = Vec::with_capacity(windows.len());
        let mut values = Vec::with_capacity(windows.len());
        for (start, length) in windows {
            sequences.push(buffer_tokens[start..start + length].to_vec());
            values.push(length as u32 * 10);
        }

        if has_duplicate_sequences(&sequences) {
            continue;
        }
        if !place_meaningful_decoy(&mut matrix, &path, &sequences, &mut rng) {
            continue;
        }

        let report = analyze_puzzle(
            &spec.config,
            &matrix,
            &sequences,
            &values,
            spec.solver_options,
        )?;
        let report =
            with_planted_solution_proof(report, &spec.config, &matrix, &sequences, &values, &path)?;
        let total_score: u32 = values.iter().sum();
        let required_solved = if spec.require_all_sequences {
            report.all_sequences_solvable && report.max_score == total_score
        } else {
            report.max_score > 0
        };
        let difficulty_is_known = !report.node_cap_reached;
        let too_trivial = difficulty_is_known
            && (matches!(report.difficulty.tier, DifficultyTier::Trivial)
                || report.max_branching_factor <= 1
                || report.trap_moves == 0);
        if required_solved
            && (difficulty_is_known || spec.allow_uncertain_difficulty)
            && !too_trivial
            && report.min_moves_to_all.unwrap_or(usize::MAX) <= spec.config.buffer_size
        {
            return Ok(GeneratedPuzzle {
                matrix,
                sequences,
                values,
                planted_path: path,
                solve_report: report,
            });
        }
    }

    Err(GameError::InvalidConfig(format!(
        "Failed to generate verified puzzle after {} attempts",
        spec.max_attempts
    )))
}

fn with_planted_solution_proof(
    mut report: SolveReport,
    config: &BreachConfig,
    matrix: &[Vec<Token>],
    sequences: &[Vec<Token>],
    values: &[u32],
    path: &[Cell],
) -> Result<SolveReport, GameError> {
    let total_score: u32 = values.iter().sum();
    let mut engine = GameEngine::new(
        config.clone(),
        matrix.to_vec(),
        sequences.to_vec(),
        values.to_vec(),
    )?;

    for (idx, &cell) in path.iter().enumerate() {
        engine.apply_move(cell)?;
        if engine.has_uploaded_all_sequences() {
            let moves = idx + 1;
            report.max_score = report.max_score.max(total_score);
            report.all_sequences_solvable = true;
            report.min_moves_to_all = min_option(report.min_moves_to_all, Some(moves));
            report.min_moves_to_max_score = min_option(report.min_moves_to_max_score, Some(moves));
            return Ok(report);
        }
    }

    Ok(report)
}

fn min_option(left: Option<usize>, right: Option<usize>) -> Option<usize> {
    match (left, right) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

fn generate_path_tokens(alphabet: &[Token], len: usize, rng: &mut StdRng) -> Vec<Token> {
    (0..len)
        .map(|_| alphabet.choose(rng).expect("alphabet not empty").clone())
        .collect()
}

fn choose_sequence_windows(
    lengths: &[usize],
    path_len: usize,
    rng: &mut StdRng,
) -> Option<Vec<(usize, usize)>> {
    if lengths
        .iter()
        .any(|&length| length == 0 || length > path_len)
    {
        return None;
    }

    let mut windows = Vec::with_capacity(lengths.len());
    for (idx, &length) in lengths.iter().enumerate() {
        let max_start = path_len - length;
        let base = if lengths.len() <= 1 || max_start == 0 {
            rng.gen_range(0..=max_start)
        } else {
            idx * max_start / (lengths.len() - 1)
        };
        let jitter_min = base.saturating_sub(1);
        let jitter_max = (base + 1).min(max_start);
        windows.push((rng.gen_range(jitter_min..=jitter_max), length));
    }
    Some(windows)
}

fn has_duplicate_sequences(sequences: &[Vec<Token>]) -> bool {
    let mut seen = HashSet::new();
    sequences
        .iter()
        .any(|sequence| !seen.insert(sequence.clone()))
}

fn place_meaningful_decoy(
    matrix: &mut [Vec<Token>],
    path: &[Cell],
    sequences: &[Vec<Token>],
    rng: &mut StdRng,
) -> bool {
    let path_cells: HashSet<Cell> = path.iter().copied().collect();
    let mut off_path = Vec::new();
    for (row, cells) in matrix.iter().enumerate() {
        for col in 0..cells.len() {
            if !path_cells.contains(&(row, col)) {
                off_path.push((row, col));
            }
        }
    }
    let sequence_tokens: Vec<Token> = sequences.iter().flatten().cloned().collect();
    let Some(&(row, col)) = off_path.choose(rng) else {
        return false;
    };
    let Some(token) = sequence_tokens.choose(rng) else {
        return false;
    };
    matrix[row][col] = token.clone();
    true
}

pub fn sequence_lengths_for(num_sequences: usize, buffer_size: usize) -> Vec<usize> {
    let base = match num_sequences {
        1 => vec![2],
        2 => vec![2, 2],
        3 => vec![2, 2, 3],
        4 => vec![2, 3, 3, 4],
        _ => vec![3, 3, 4, 4, 5],
    };

    base.into_iter()
        .take(num_sequences)
        .map(|len| len.min(buffer_size.max(1)))
        .collect()
}

pub fn create_fixed_game(
    config: &BreachConfig,
    matrix: Matrix,
    sequences: Sequences,
    sequence_values: SequenceValues,
) -> Result<GameData, GameError> {
    GameEngine::new(
        config.clone(),
        matrix.clone(),
        sequences.clone(),
        sequence_values.clone(),
    )?;
    Ok((matrix, sequences, sequence_values))
}
