use std::collections::HashSet;

use rust_breach_protocol::engine::{
    BreachConfig, Cell, ConstraintMode, GameEngine, GameError, TerminalReason, Token,
};

fn token(s: &str) -> Token {
    s.to_string()
}

fn tokens(slice: &[&str]) -> Vec<Token> {
    slice.iter().map(|s| token(s)).collect()
}

fn default_alphabet() -> Vec<Token> {
    tokens(&["55", "1C", "7A", "BD", "E9", "FF"])
}

fn square_matrix(size: usize, fill: &str) -> Vec<Vec<Token>> {
    vec![vec![token(fill); size]; size]
}

fn config(size: usize, buffer: usize, time: u64) -> BreachConfig {
    BreachConfig {
        matrix_size: size,
        buffer_size: buffer,
        time_limit_seconds: time,
        token_alphabet: default_alphabet(),
        seed: None,
    }
}

fn make_engine(
    size: usize,
    buffer: usize,
    matrix: Vec<Vec<Token>>,
    sequences: Vec<Vec<Token>>,
    values: Vec<u32>,
) -> GameEngine {
    GameEngine::new(config(size, buffer, 30), matrix, sequences, values).unwrap()
}

#[test]
fn new_rejects_empty_alphabet() {
    let cfg = BreachConfig {
        matrix_size: 4,
        buffer_size: 6,
        time_limit_seconds: 30,
        token_alphabet: vec![],
        seed: None,
    };
    let m = square_matrix(4, "55");
    let err = GameEngine::new(cfg, m, vec![], vec![]).unwrap_err();
    assert!(matches!(err, GameError::InvalidConfig(_)));
}

#[test]
fn new_rejects_matrix_size_mismatch() {
    let m = square_matrix(3, "55");
    let err = GameEngine::new(config(4, 6, 30), m, vec![], vec![]).unwrap_err();
    assert!(matches!(err, GameError::InvalidConfig(_)));
}

#[test]
fn new_rejects_non_square_matrix() {
    let m = vec![tokens(&["55", "1C", "7A"]), tokens(&["BD", "E9"])];
    let err = GameEngine::new(config(3, 6, 30), m, vec![], vec![]).unwrap_err();
    assert!(matches!(err, GameError::InvalidConfig(_)));
}

#[test]
fn new_rejects_mismatched_sequences_and_values() {
    let m = square_matrix(4, "55");
    let seqs = vec![tokens(&["55", "1C"])];
    let vals = vec![10, 20];
    let err = GameEngine::new(config(4, 6, 30), m, seqs, vals).unwrap_err();
    assert!(matches!(err, GameError::InvalidConfig(_)));
}

#[test]
fn new_initial_state() {
    let m = square_matrix(4, "55");
    let seqs = vec![tokens(&["55", "1C"])];
    let vals = vec![30];
    let engine = GameEngine::new(config(4, 6, 30), m, seqs, vals).unwrap();

    assert_eq!(engine.state.buffer_tokens.len(), 0);
    assert!(engine.state.selected_cells.is_empty());
    assert_eq!(engine.state.current_constraint, ConstraintMode::Row);
    assert_eq!(engine.state.current_index, None);
    assert_eq!(engine.state.terminal_reason, None);
    assert_eq!(engine.state.uploaded_results, vec![false]);
}

#[test]
fn legal_moves_first_move_is_row_zero() {
    let engine = make_engine(4, 6, square_matrix(4, "55"), vec![], vec![]);
    let legal = engine.legal_moves();
    let expected: HashSet<Cell> = (0..4).map(|col| (0, col)).collect();
    assert_eq!(legal, expected);
}

#[test]
fn legal_moves_after_pick_flips_to_column() {
    let mut engine = make_engine(4, 6, square_matrix(4, "55"), vec![], vec![]);
    engine.apply_move((0, 2)).unwrap();
    let legal = engine.legal_moves();
    let mut expected = HashSet::new();
    expected.insert((1, 2));
    expected.insert((2, 2));
    expected.insert((3, 2));
    assert_eq!(legal, expected);
}

#[test]
fn legal_moves_alternates_row_column() {
    let mut engine = make_engine(4, 6, square_matrix(4, "55"), vec![], vec![]);
    engine.apply_move((0, 2)).unwrap();
    engine.apply_move((3, 2)).unwrap();
    let legal = engine.legal_moves();
    let mut expected = HashSet::new();
    expected.insert((3, 0));
    expected.insert((3, 1));
    expected.insert((3, 3));
    assert_eq!(legal, expected);
}

#[test]
fn legal_moves_excludes_selected() {
    let mut engine = make_engine(4, 6, square_matrix(4, "55"), vec![], vec![]);
    engine.apply_move((0, 0)).unwrap();
    engine.apply_move((3, 0)).unwrap();
    let legal = engine.legal_moves();
    assert!(!legal.contains(&(3, 0)));
}

#[test]
fn has_legal_moves_agrees_with_legal_moves() {
    let mut engine = make_engine(3, 6, square_matrix(3, "55"), vec![], vec![]);
    assert_eq!(engine.has_legal_moves(), !engine.legal_moves().is_empty());

    engine.apply_move((0, 1)).unwrap();
    assert_eq!(engine.has_legal_moves(), !engine.legal_moves().is_empty());

    engine.apply_move((2, 1)).unwrap();
    assert_eq!(engine.has_legal_moves(), !engine.legal_moves().is_empty());
}

#[test]
fn has_legal_moves_zero_size_matrix() {
    let cfg = BreachConfig {
        matrix_size: 0,
        buffer_size: 4,
        time_limit_seconds: 30,
        token_alphabet: default_alphabet(),
        seed: None,
    };
    let engine = GameEngine::new(cfg, vec![], vec![], vec![]).unwrap();
    assert!(!engine.has_legal_moves());
}

#[test]
fn apply_move_rejects_invalid_cell() {
    let mut engine = make_engine(4, 6, square_matrix(4, "55"), vec![], vec![]);
    let err = engine.apply_move((2, 1)).unwrap_err();
    assert!(matches!(err, GameError::InvalidMove((2, 1))));
}

#[test]
fn apply_move_rejects_already_selected() {
    let mut engine = make_engine(4, 6, square_matrix(4, "55"), vec![], vec![]);
    engine.apply_move((0, 1)).unwrap();
    engine.apply_move((3, 1)).unwrap();
    let err = engine.apply_move((3, 1)).unwrap_err();
    assert!(matches!(err, GameError::InvalidMove((3, 1))));
}

#[test]
fn apply_move_appends_token_to_buffer() {
    let mut matrix = square_matrix(4, "55");
    matrix[0][2] = token("E9");
    matrix[1][2] = token("1C");

    let mut engine = make_engine(4, 6, matrix, vec![], vec![]);
    engine.apply_move((0, 2)).unwrap();
    engine.apply_move((1, 2)).unwrap();

    assert_eq!(engine.state.buffer_tokens, tokens(&["E9", "1C"]));
}

#[test]
fn apply_move_rejects_when_buffer_full() {
    let mut engine = make_engine(3, 1, square_matrix(3, "55"), vec![], vec![]);
    engine.apply_move((0, 0)).unwrap();
    let err = engine.apply_move((1, 0)).unwrap_err();
    assert!(matches!(err, GameError::InvalidMove((1, 0))));
}

#[test]
fn apply_move_rejects_after_all_sequences_uploaded() {
    let mut matrix = square_matrix(3, "FF");
    matrix[0][0] = token("55");
    let mut engine = make_engine(3, 4, matrix, vec![tokens(&["55"])], vec![10]);
    engine.apply_move((0, 0)).unwrap();
    let err = engine.apply_move((1, 0)).unwrap_err();
    assert!(matches!(err, GameError::InvalidMove((1, 0))));
}

#[test]
fn apply_move_toggles_constraint_each_step() {
    let mut engine = make_engine(4, 6, square_matrix(4, "55"), vec![], vec![]);
    assert_eq!(engine.state.current_constraint, ConstraintMode::Row);
    engine.apply_move((0, 0)).unwrap();
    assert_eq!(engine.state.current_constraint, ConstraintMode::Column);
    engine.apply_move((1, 0)).unwrap();
    assert_eq!(engine.state.current_constraint, ConstraintMode::Row);
    engine.apply_move((1, 3)).unwrap();
    assert_eq!(engine.state.current_constraint, ConstraintMode::Column);
}

#[test]
fn evaluate_uploaded_exact_match() {
    let mut engine = make_engine(
        3,
        4,
        square_matrix(3, "FF"),
        vec![tokens(&["E9", "1C"])],
        vec![20],
    );
    engine.state.buffer_tokens = tokens(&["E9", "1C"]);
    engine.evaluate_uploaded();
    assert!(engine.state.uploaded_results[0]);
}

#[test]
fn evaluate_uploaded_embedded_in_buffer() {
    let mut engine = make_engine(
        3,
        6,
        square_matrix(3, "FF"),
        vec![tokens(&["E9", "1C"])],
        vec![20],
    );
    engine.state.buffer_tokens = tokens(&["BD", "E9", "1C", "7A"]);
    engine.evaluate_uploaded();
    assert!(engine.state.uploaded_results[0]);
}

#[test]
fn evaluate_uploaded_no_match() {
    let mut engine = make_engine(
        3,
        6,
        square_matrix(3, "FF"),
        vec![tokens(&["E9", "1C"])],
        vec![20],
    );
    engine.state.buffer_tokens = tokens(&["BD", "7A", "FF"]);
    engine.evaluate_uploaded();
    assert!(!engine.state.uploaded_results[0]);
}

#[test]
fn evaluate_uploaded_partial_no_match() {
    let mut engine = make_engine(
        3,
        6,
        square_matrix(3, "FF"),
        vec![tokens(&["E9", "1C"])],
        vec![20],
    );
    engine.state.buffer_tokens = tokens(&["E9", "BD", "1C"]);
    engine.evaluate_uploaded();
    assert!(!engine.state.uploaded_results[0]);
}

#[test]
fn evaluate_uploaded_multiple_sequences() {
    let seqs = vec![tokens(&["55", "1C"]), tokens(&["1C", "7A"])];
    let vals = vec![20, 30];
    let mut engine = make_engine(3, 6, square_matrix(3, "FF"), seqs, vals);
    engine.state.buffer_tokens = tokens(&["55", "1C", "7A"]);
    engine.evaluate_uploaded();
    assert!(engine.state.uploaded_results[0]);
    assert!(engine.state.uploaded_results[1]);
}

#[test]
fn evaluate_uploaded_uses_token_boundaries() {
    let seqs = vec![tokens(&["AB", "C"])];
    let vals = vec![20];
    let mut engine = make_engine(3, 6, square_matrix(3, "FF"), seqs, vals);
    engine.state.buffer_tokens = tokens(&["A", "BC"]);
    engine.evaluate_uploaded();
    assert!(!engine.state.uploaded_results[0]);
}

#[test]
fn evaluate_uploaded_does_not_reupload() {
    let mut engine = make_engine(
        3,
        6,
        square_matrix(3, "FF"),
        vec![tokens(&["E9"])],
        vec![10],
    );
    engine.state.buffer_tokens = tokens(&["E9"]);
    engine.evaluate_uploaded();
    assert!(engine.state.uploaded_results[0]);

    engine.state.uploaded_results[0] = true;
    engine.state.buffer_tokens = tokens(&["55"]);
    engine.evaluate_uploaded();
    assert!(engine.state.uploaded_results[0]);
}

#[test]
fn is_terminal_buffer_full() {
    let mut engine = make_engine(3, 1, square_matrix(3, "55"), vec![], vec![]);
    engine.apply_move((0, 0)).unwrap();
    assert!(engine.is_terminal());
    assert_eq!(engine.terminal_reason(), Some(TerminalReason::BufferFull));
}

#[test]
fn is_terminal_all_uploaded() {
    let mut matrix = square_matrix(3, "FF");
    matrix[0][0] = token("55");
    let mut engine = make_engine(3, 4, matrix, vec![tokens(&["55"])], vec![10]);
    engine.apply_move((0, 0)).unwrap();
    assert!(engine.is_terminal());
    assert_eq!(engine.terminal_reason(), Some(TerminalReason::AllUploaded));
}

#[test]
fn is_terminal_not_terminal_if_moves_remain() {
    let engine = make_engine(
        3,
        4,
        square_matrix(3, "55"),
        vec![tokens(&["E9"])],
        vec![10],
    );
    assert!(!engine.is_terminal());
}

#[test]
fn is_terminal_when_no_unuploaded_sequence_can_fit_remaining_buffer() {
    let mut engine = make_engine(
        3,
        2,
        square_matrix(3, "55"),
        vec![tokens(&["55"]), tokens(&["1C", "BD"])],
        vec![10, 20],
    );
    engine.apply_move((0, 0)).unwrap();

    assert!(engine.is_terminal());
    assert_eq!(
        engine.terminal_reason(),
        Some(TerminalReason::NoCompletableSequences)
    );
    assert_eq!(engine.state.buffer_tokens.len(), 1);
    assert_eq!(engine.compute_score(), 10);
}

#[test]
fn is_not_terminal_when_suffix_can_complete_unuploaded_sequence() {
    let mut matrix = square_matrix(3, "FF");
    matrix[0][0] = token("55");
    matrix[1][0] = token("1C");
    let mut engine = make_engine(3, 2, matrix, vec![tokens(&["55", "1C"])], vec![20]);

    engine.apply_move((0, 0)).unwrap();

    assert!(!engine.is_terminal());
    assert!(engine.can_sequence_still_upload(0));
}

#[test]
fn sequence_progress_is_token_suffix_based() {
    let mut engine = make_engine(
        3,
        6,
        square_matrix(3, "FF"),
        vec![tokens(&["1C", "7A", "BD"])],
        vec![30],
    );
    engine.state.buffer_tokens = tokens(&["55", "1C", "7A"]);

    assert_eq!(engine.sequence_progress(0), 2);
}

#[test]
fn sequence_progress_respects_overlap_and_token_boundaries() {
    let mut engine = make_engine(
        3,
        6,
        square_matrix(3, "FF"),
        vec![tokens(&["55", "1C", "55", "BD"])],
        vec![40],
    );
    engine.state.buffer_tokens = tokens(&["AA", "55", "1C", "55"]);
    assert_eq!(engine.sequence_progress(0), 3);

    engine.state.buffer_tokens = tokens(&["5", "51", "C"]);
    assert_eq!(engine.sequence_progress(0), 0);
}

#[test]
fn is_terminal_out_of_time() {
    let mut engine = make_engine(3, 4, square_matrix(3, "55"), vec![], vec![]);
    engine.force_timeout();
    assert!(engine.is_terminal());
    assert_eq!(engine.terminal_reason(), Some(TerminalReason::OutOfTime));
}

#[test]
fn compute_score_sums_uploaded_values() {
    let seqs = vec![tokens(&["55"]), tokens(&["1C"]), tokens(&["7A"])];
    let vals = vec![10, 20, 30];
    let mut engine = make_engine(3, 6, square_matrix(3, "FF"), seqs, vals);
    engine.state.buffer_tokens = tokens(&["55", "1C"]);
    engine.evaluate_uploaded();
    assert_eq!(engine.compute_score(), 30);
}

#[test]
fn compute_score_zero_when_none_uploaded() {
    let seqs = vec![tokens(&["55"])];
    let vals = vec![10];
    let engine = make_engine(3, 6, square_matrix(3, "FF"), seqs, vals);
    assert_eq!(engine.compute_score(), 0);
}

#[test]
fn get_game_result_success_means_score_gt_zero() {
    let seqs = vec![tokens(&["55"])];
    let vals = vec![10];
    let mut engine = make_engine(3, 6, square_matrix(3, "FF"), seqs, vals);
    engine.state.buffer_tokens = tokens(&["55"]);
    engine.evaluate_uploaded();
    let result = engine.get_game_result();
    assert!(result.success);
    assert_eq!(result.score, 10);
    assert_eq!(result.sequences_uploaded, 1);
    assert_eq!(result.total_sequences, 1);
}

#[test]
fn get_game_result_failure_when_no_score() {
    let engine = make_engine(
        3,
        6,
        square_matrix(3, "FF"),
        vec![tokens(&["55"])],
        vec![10],
    );
    let result = engine.get_game_result();
    assert!(!result.success);
}

#[test]
fn is_valid_move_accepts_legal_cell() {
    let engine = make_engine(3, 6, square_matrix(3, "55"), vec![], vec![]);
    assert!(engine.is_valid_move((0, 0)));
    assert!(!engine.is_valid_move((1, 0)));
}
