use rust_breach_protocol::engine::{BreachConfig, Token};
use rust_breach_protocol::solver::{DifficultyTier, SolverOptions, analyze_puzzle};

fn token(s: &str) -> Token {
    s.to_string()
}

fn tokens(slice: &[&str]) -> Vec<Token> {
    slice.iter().map(|s| token(s)).collect()
}

fn config(size: usize, buffer: usize) -> BreachConfig {
    BreachConfig {
        matrix_size: size,
        buffer_size: buffer,
        time_limit_seconds: 30,
        token_alphabet: tokens(&["A", "B", "C", "D", "E", "F", "X", "Y", "Z"]),
        seed: Some(1),
    }
}

fn fixture_matrix() -> Vec<Vec<Token>> {
    vec![
        tokens(&["A", "B", "C"]),
        tokens(&["X", "D", "Y"]),
        tokens(&["Z", "E", "F"]),
    ]
}

#[test]
fn solver_finds_all_sequences_and_min_moves() {
    let report = analyze_puzzle(
        &config(3, 5),
        &fixture_matrix(),
        &[tokens(&["A", "X", "D"]), tokens(&["X", "D"])],
        &[30, 20],
        SolverOptions::default(),
    )
    .unwrap();

    assert_eq!(report.max_score, 50);
    assert_eq!(report.total_score, 50);
    assert!((0.0..=50.0).contains(&report.expected_score));
    assert!((0.0..=1.0).contains(&report.expected_score_ratio));
    assert!(report.all_sequences_solvable);
    assert_eq!(report.min_moves_to_all, Some(3));
    assert_eq!(report.min_moves_to_max_score, Some(3));
    assert!(report.solution_count >= 1);
    assert!(report.terminal_paths >= report.solution_count);
    assert!(report.max_score_paths >= 1);
    assert_ne!(report.difficulty.tier, DifficultyTier::Uncertain);
}

#[test]
fn solver_reports_partial_best_score() {
    let report = analyze_puzzle(
        &config(3, 5),
        &fixture_matrix(),
        &[tokens(&["A", "X", "D"]), tokens(&["A", "B"])],
        &[30, 20],
        SolverOptions::default(),
    )
    .unwrap();

    assert_eq!(report.max_score, 30);
    assert!(!report.all_sequences_solvable);
    assert_eq!(report.min_moves_to_all, None);
    assert_eq!(report.min_moves_to_max_score, Some(3));
    assert!(report.difficulty.rating > 0.0);
    assert!(report.difficulty.score_pressure > 0.0);
}

#[test]
fn solver_counts_trap_moves_against_optimal_score() {
    let report = analyze_puzzle(
        &config(3, 5),
        &fixture_matrix(),
        &[tokens(&["A", "X", "D"])],
        &[30],
        SolverOptions::default(),
    )
    .unwrap();

    assert!(report.optimal_moves >= 1);
    assert!(report.trap_moves >= 1);
    assert!(report.difficulty.trap_move_ratio > 0.0);
}

#[test]
fn difficulty_tracks_token_ambiguity_from_repeated_matrix_tokens() {
    let unique = analyze_puzzle(
        &config(3, 5),
        &fixture_matrix(),
        &[tokens(&["A", "X", "D"])],
        &[30],
        SolverOptions::default(),
    )
    .unwrap();

    let repeated_matrix = vec![
        tokens(&["A", "A", "A"]),
        tokens(&["A", "B", "A"]),
        tokens(&["A", "A", "A"]),
    ];
    let repeated = analyze_puzzle(
        &config(3, 5),
        &repeated_matrix,
        &[tokens(&["A", "B"])],
        &[20],
        SolverOptions::default(),
    )
    .unwrap();

    assert!(repeated.difficulty.token_ambiguity > unique.difficulty.token_ambiguity);
}

#[test]
fn difficulty_tracks_time_pressure_without_changing_score() {
    let mut no_limit = config(3, 5);
    no_limit.time_limit_seconds = 0;
    let mut tight_limit = config(3, 5);
    tight_limit.time_limit_seconds = 5;

    let no_limit_report = analyze_puzzle(
        &no_limit,
        &fixture_matrix(),
        &[tokens(&["A", "X", "D"])],
        &[30],
        SolverOptions::default(),
    )
    .unwrap();
    let tight_limit_report = analyze_puzzle(
        &tight_limit,
        &fixture_matrix(),
        &[tokens(&["A", "X", "D"])],
        &[30],
        SolverOptions::default(),
    )
    .unwrap();

    assert_eq!(no_limit_report.max_score, tight_limit_report.max_score);
    assert_eq!(no_limit_report.difficulty.time_pressure, 0.0);
    assert!(tight_limit_report.difficulty.time_pressure > 0.0);
}

#[test]
fn solver_respects_node_cap() {
    let matrix = vec![
        tokens(&["A", "B", "C", "D"]),
        tokens(&["E", "F", "A", "B"]),
        tokens(&["C", "D", "E", "F"]),
        tokens(&["A", "B", "C", "D"]),
    ];
    let report = analyze_puzzle(
        &config(4, 6),
        &matrix,
        &[tokens(&["A", "E", "F"])],
        &[30],
        SolverOptions {
            node_limit: 1,
            solution_cap: 10,
        },
    )
    .unwrap();

    assert!(report.node_cap_reached);
    assert_eq!(report.difficulty.tier, DifficultyTier::Uncertain);
}
