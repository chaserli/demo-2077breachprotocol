use std::collections::HashSet;

use rand::SeedableRng;
use rand::rngs::StdRng;
use rust_breach_protocol::engine::{BreachConfig, GameError};
use rust_breach_protocol::generator::{
    PuzzleSpec, create_fixed_game, generate_matrix, generate_verified_puzzle, sequence_lengths_for,
};
use rust_breach_protocol::solver::SolverOptions;

fn default_config(size: usize, buffer: usize) -> BreachConfig {
    BreachConfig {
        matrix_size: size,
        buffer_size: buffer,
        time_limit_seconds: 30,
        token_alphabet: vec![
            "55".to_string(),
            "1C".to_string(),
            "7A".to_string(),
            "BD".to_string(),
            "E9".to_string(),
            "FF".to_string(),
        ],
        seed: None,
    }
}

#[test]
fn sequence_lengths_single() {
    assert_eq!(sequence_lengths_for(1, 8), vec![2]);
}

#[test]
fn sequence_lengths_two() {
    assert_eq!(sequence_lengths_for(2, 8), vec![2, 2]);
}

#[test]
fn sequence_lengths_three() {
    assert_eq!(sequence_lengths_for(3, 8), vec![2, 2, 3]);
}

#[test]
fn sequence_lengths_four() {
    assert_eq!(sequence_lengths_for(4, 8), vec![2, 3, 3, 4]);
}

#[test]
fn generate_matrix_correct_dimensions() {
    let cfg = default_config(5, 8);
    let mut rng = StdRng::seed_from_u64(1);
    let matrix = generate_matrix(&cfg, &mut rng).unwrap();
    assert_eq!(matrix.len(), 5);
    for row in &matrix {
        assert_eq!(row.len(), 5);
    }
}

#[test]
fn generate_matrix_tokens_from_alphabet() {
    let cfg = default_config(4, 8);
    let mut rng = StdRng::seed_from_u64(1);
    let matrix = generate_matrix(&cfg, &mut rng).unwrap();
    for row in &matrix {
        for token in row {
            assert!(cfg.token_alphabet.contains(token));
        }
    }
}

#[test]
fn same_seed_same_verified_puzzle() {
    let mut cfg = default_config(5, 8);
    cfg.seed = Some(12345);
    let puzzle1 = generate_verified_puzzle(&PuzzleSpec::new(cfg.clone(), 3)).unwrap();
    let puzzle2 = generate_verified_puzzle(&PuzzleSpec::new(cfg, 3)).unwrap();
    assert_eq!(puzzle1.matrix, puzzle2.matrix);
    assert_eq!(puzzle1.sequences, puzzle2.sequences);
    assert_eq!(puzzle1.values, puzzle2.values);
    assert_eq!(puzzle1.planted_path, puzzle2.planted_path);
}

#[test]
fn different_seed_different_output() {
    let mut cfg = default_config(5, 8);
    cfg.seed = Some(1);
    let puzzle1 = generate_verified_puzzle(&PuzzleSpec::new(cfg.clone(), 3)).unwrap();
    cfg.seed = Some(99999);
    let puzzle2 = generate_verified_puzzle(&PuzzleSpec::new(cfg, 3)).unwrap();
    assert_ne!(puzzle1.matrix, puzzle2.matrix);
}

#[test]
fn generated_puzzles_are_all_daemon_solvable_for_seed_sweep() {
    for seed in 0..20 {
        let mut cfg = default_config(5, 8);
        cfg.seed = Some(seed);
        let puzzle = generate_verified_puzzle(&PuzzleSpec::new(cfg.clone(), 3)).unwrap();
        assert_eq!(puzzle.matrix.len(), cfg.matrix_size);
        assert_eq!(puzzle.sequences.len(), 3);
        assert!(puzzle.solve_report.all_sequences_solvable);
        assert_eq!(
            puzzle.solve_report.max_score,
            puzzle.values.iter().sum::<u32>()
        );
        assert!(!puzzle.solve_report.node_cap_reached);
    }
}

#[test]
fn large_gui_profile_uses_planted_proof_when_difficulty_is_capped() {
    let mut cfg = default_config(9, 8);
    cfg.token_alphabet = vec![
        "55".to_string(),
        "1C".to_string(),
        "7A".to_string(),
        "BD".to_string(),
        "E9".to_string(),
        "FF".to_string(),
        "00".to_string(),
        "9A".to_string(),
    ];
    cfg.seed = Some(9_848);

    let spec = PuzzleSpec::new(cfg.clone(), 4)
        .with_solver_options(SolverOptions {
            node_limit: 1,
            solution_cap: 32,
        })
        .with_attempt_budget(4)
        .allow_uncertain_difficulty(true);
    let puzzle = generate_verified_puzzle(&spec).unwrap();

    assert_eq!(puzzle.matrix.len(), 9);
    assert_eq!(puzzle.sequences.len(), 4);
    assert!(puzzle.solve_report.node_cap_reached);
    assert!(puzzle.solve_report.all_sequences_solvable);
    assert_eq!(
        puzzle.solve_report.max_score,
        puzzle.values.iter().sum::<u32>()
    );
    assert!(puzzle.solve_report.min_moves_to_all.unwrap() <= cfg.buffer_size);
}

#[test]
fn generated_path_follows_rules() {
    let mut cfg = default_config(5, 8);
    cfg.seed = Some(42);
    let puzzle = generate_verified_puzzle(&PuzzleSpec::new(cfg, 3)).unwrap();
    assert_eq!(puzzle.planted_path[0].0, 0);

    let mut expect_column_constraint = true;
    let mut seen = HashSet::new();
    for (idx, &cell) in puzzle.planted_path.iter().enumerate() {
        assert!(seen.insert(cell));
        if idx == 0 {
            continue;
        }
        let prev = puzzle.planted_path[idx - 1];
        if expect_column_constraint {
            assert_eq!(prev.1, cell.1);
        } else {
            assert_eq!(prev.0, cell.0);
        }
        expect_column_constraint = !expect_column_constraint;
    }
}

#[test]
fn create_fixed_game_validates_square_matrix() {
    let cfg = default_config(3, 4);
    let non_square = vec![
        vec!["55".to_string(), "1C".to_string()],
        vec!["BD".to_string(), "E9".to_string()],
    ];
    let err = create_fixed_game(&cfg, non_square, vec![], vec![]).unwrap_err();
    assert!(matches!(err, GameError::InvalidConfig(_)));
}

#[test]
fn create_fixed_game_validates_sequences_values_align() {
    let cfg = default_config(3, 4);
    let m = vec![vec!["55".to_string(); 3]; 3];
    let err = create_fixed_game(&cfg, m, vec![vec!["55".to_string()]], vec![10, 20]).unwrap_err();
    assert!(matches!(err, GameError::InvalidConfig(_)));
}

#[test]
fn rejected_config_fails_clearly() {
    let mut cfg = default_config(3, 4);
    cfg.token_alphabet.clear();
    let err = generate_verified_puzzle(&PuzzleSpec::new(cfg, 2)).unwrap_err();
    assert!(matches!(err, GameError::InvalidConfig(_)));
}
