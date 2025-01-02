use rand::prelude::*;

use crate::engine::{BreachConfig, GameError, Token};
use crate::engine::{Cell, GameEngine};

fn seeded_rng(seed: Option<u64>) -> StdRng {
    match seed {
        Some(value) => StdRng::seed_from_u64(value),
        None => StdRng::from_entropy(),
    }
}

fn generate_valid_path(matrix_size: usize, path_length: usize, rng: &mut StdRng) -> Vec<Cell> {
    let mut best_path = Vec::new();

    for _ in 0..200 {
        let mut path = Vec::with_capacity(path_length);
        let mut visited = std::collections::HashSet::new();

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

pub fn generate_matrix(
    config: &BreachConfig,
    rng: &mut StdRng,
) -> Result<Vec<Vec<Token>>, GameError> {
    if config.token_alphabet.is_empty() {
        return Err(GameError::InvalidConfig(
            "Token alphabet cannot be empty".to_string(),
        ));
    }
    let mut matrix = Vec::with_capacity(config.matrix_size);

    for _ in 0..config.matrix_size {
        let mut row = Vec::with_capacity(config.matrix_size);
        for _ in 0..config.matrix_size {
            let token = config
                .token_alphabet
                .choose(rng)
                .expect("Token alphabet cannot be empty")
                .clone();
            row.push(token);
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
) -> Result<(Vec<Vec<Token>>, Vec<u32>), GameError> {
    let all_tokens: Vec<Token> = matrix.iter().flatten().cloned().collect();

    if all_tokens.is_empty() {
        return Err(GameError::InvalidConfig(
            "Matrix must contain tokens to build sequences".to_string(),
        ));
    }

    let sequence_lengths = sequence_lengths_for(num_sequences, config.buffer_size);

    let mut sequences = Vec::with_capacity(num_sequences);
    let mut sequence_values = Vec::with_capacity(num_sequences);

    for length in sequence_lengths {
        let mut sequence = Vec::with_capacity(length);

        for _ in 0..length {
            let token = all_tokens
                .choose(rng)
                .expect("Should always have tokens")
                .clone();
            sequence.push(token);
        }

        sequence_values.push((length as u32) * 10);
        sequences.push(sequence);
    }

    Ok((sequences, sequence_values))
}

pub fn create_random_game(
    config: &BreachConfig,
    num_sequences: usize,
) -> Result<(Vec<Vec<Token>>, Vec<Vec<Token>>, Vec<u32>), GameError> {
    let mut rng = seeded_rng(config.seed);
    let matrix = generate_matrix(config, &mut rng)?;
    let (sequences, values) = generate_sequences(config, &matrix, num_sequences, &mut rng)?;

    Ok((matrix, sequences, values))
}

pub fn generate_solvable_game(
    config: &BreachConfig,
    num_sequences: usize,
    ensure_all_solvable: bool,
) -> Result<(Vec<Vec<Token>>, Vec<Vec<Token>>, Vec<u32>), GameError> {
    let (matrix, sequences, values, _) =
        generate_solvable_game_with_path(config, num_sequences, ensure_all_solvable)?;
    Ok((matrix, sequences, values))
}

pub fn generate_solvable_game_with_path(
    config: &BreachConfig,
    num_sequences: usize,
    ensure_all_solvable: bool,
) -> Result<(Vec<Vec<Token>>, Vec<Vec<Token>>, Vec<u32>, Vec<Cell>), GameError> {
    let max_retries = 100;
    let mut rng = seeded_rng(config.seed);

    let sequence_lengths = sequence_lengths_for(num_sequences, config.buffer_size);

    for attempt in 0..max_retries {
        if let Some(seed) = config.seed {
            rng = seeded_rng(Some(seed + attempt as u64));
        }

        let mut matrix = generate_matrix(config, &mut rng)?;
        let path = generate_valid_path(config.matrix_size, config.buffer_size, &mut rng);
        if path.len() < config.buffer_size.min(3) {
            continue;
        }

        let mut buffer_tokens = Vec::with_capacity(path.len());
        for _ in 0..path.len() {
            let token = config
                .token_alphabet
                .choose(&mut rng)
                .expect("Token alphabet cannot be empty")
                .clone();
            buffer_tokens.push(token);
        }

        for (idx, cell) in path.iter().enumerate() {
            let (row, col) = *cell;
            matrix[row][col] = buffer_tokens[idx].clone();
        }

        let mut sequences = Vec::with_capacity(num_sequences);
        let mut values = Vec::with_capacity(num_sequences);

        for length in &sequence_lengths {
            if *length == 0 || *length > buffer_tokens.len() {
                continue;
            }
            let start_idx = rng.gen_range(0..=buffer_tokens.len() - *length);
            let sequence = buffer_tokens[start_idx..start_idx + *length].to_vec();
            sequences.push(sequence);
            values.push((*length as u32) * 10);
        }

        if ensure_all_solvable && sequences.len() != num_sequences {
            continue;
        }

        let mut engine = GameEngine::new(
            config.clone(),
            matrix.clone(),
            sequences.clone(),
            values.clone(),
        )?;
        let mut path_valid = true;
        for cell in &path {
            if engine.is_valid_move(*cell) {
                if engine.apply_move(*cell).is_err() {
                    path_valid = false;
                    break;
                }
            } else {
                path_valid = false;
                break;
            }
        }

        if path_valid {
            let uploaded = engine.state.uploaded_results.iter().filter(|&&u| u).count();
            let required = if ensure_all_solvable {
                num_sequences
            } else {
                1
            };
            if uploaded >= required {
                return Ok((matrix, sequences, values, path));
            }
        }
    }

    if ensure_all_solvable {
        return Err(GameError::InvalidConfig(format!(
            "Failed to generate solvable game after {max_retries} attempts"
        )));
    }

    let (matrix, sequences, values) = create_random_game(config, num_sequences)?;
    Ok((matrix, sequences, values, Vec::new()))
}

fn sequence_lengths_for(num_sequences: usize, buffer_size: usize) -> Vec<usize> {
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
    matrix: Vec<Vec<Token>>,
    sequences: Vec<Vec<Token>>,
    sequence_values: Vec<u32>,
) -> Result<(Vec<Vec<Token>>, Vec<Vec<Token>>, Vec<u32>), GameError> {
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

    Ok((matrix, sequences, sequence_values))
}
