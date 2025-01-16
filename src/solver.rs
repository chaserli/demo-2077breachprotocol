use std::collections::HashMap;

use crate::engine::{BreachConfig, Cell, ConstraintMode, GameEngine, GameError, Token};

#[derive(Clone, Copy, Debug)]
pub struct SolverOptions {
    pub node_limit: usize,
    pub solution_cap: usize,
}

impl Default for SolverOptions {
    fn default() -> Self {
        Self {
            node_limit: 200_000,
            solution_cap: 1_000,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DifficultyTier {
    Trivial,
    Easy,
    Medium,
    Hard,
    Extreme,
    Uncertain,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DifficultyReport {
    pub rating: f32,
    pub tier: DifficultyTier,
    pub average_branching_factor: f32,
    pub branch_pressure: f32,
    pub buffer_pressure: f32,
    pub sequence_pressure: f32,
    pub token_ambiguity: f32,
    pub solution_rarity: f32,
    pub optimal_rarity: f32,
    pub score_pressure: f32,
    pub trap_move_ratio: f32,
    pub dead_end_ratio: f32,
    pub forced_state_ratio: f32,
    pub time_pressure: f32,
    pub confidence: f32,
}

impl Default for DifficultyReport {
    fn default() -> Self {
        Self {
            rating: 0.0,
            tier: DifficultyTier::Uncertain,
            average_branching_factor: 0.0,
            branch_pressure: 0.0,
            buffer_pressure: 0.0,
            sequence_pressure: 0.0,
            token_ambiguity: 0.0,
            solution_rarity: 0.0,
            optimal_rarity: 0.0,
            score_pressure: 0.0,
            trap_move_ratio: 0.0,
            dead_end_ratio: 0.0,
            forced_state_ratio: 0.0,
            time_pressure: 0.0,
            confidence: 0.0,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SolveReport {
    pub total_score: u32,
    pub max_score: u32,
    pub expected_score: f32,
    pub expected_score_ratio: f32,
    pub all_sequences_solvable: bool,
    pub min_moves_to_all: Option<usize>,
    pub min_moves_to_max_score: Option<usize>,
    pub solution_count: usize,
    pub solution_count_capped: bool,
    pub terminal_paths: usize,
    pub terminal_paths_capped: bool,
    pub max_score_paths: usize,
    pub max_score_paths_capped: bool,
    pub dead_ends: usize,
    pub nodes_searched: usize,
    pub forced_states: usize,
    pub branch_points: usize,
    pub total_branches: usize,
    pub max_branching_factor: usize,
    pub optimal_moves: usize,
    pub trap_moves: usize,
    pub node_cap_reached: bool,
    pub difficulty: DifficultyReport,
}

#[derive(Clone, Debug, Default)]
struct SearchOutcome {
    best_score: u32,
    min_moves_to_best: Option<usize>,
    min_moves_to_all: Option<usize>,
    terminal_paths: CappedCount,
    all_paths: CappedCount,
    best_paths: CappedCount,
    terminal_score_sum: f64,
}

#[derive(Clone, Copy, Debug, Default)]
struct CappedCount {
    value: usize,
    capped: bool,
}

impl CappedCount {
    fn one() -> Self {
        Self {
            value: 1,
            capped: false,
        }
    }

    fn add(&mut self, other: Self, cap: usize) {
        let cap = cap.max(1);
        self.capped |= other.capped;
        match self.value.checked_add(other.value) {
            Some(sum) if sum <= cap => self.value = sum,
            _ => {
                self.value = cap;
                self.capped = true;
            }
        }
    }
}

#[derive(Clone, Hash, Eq, PartialEq)]
struct SearchKey {
    selected_mask: u128,
    current_index: Option<usize>,
    current_constraint: ConstraintMode,
    uploaded_mask: u64,
    suffix: Vec<Token>,
}

pub fn analyze_puzzle(
    config: &BreachConfig,
    matrix: &[Vec<Token>],
    sequences: &[Vec<Token>],
    sequence_values: &[u32],
    options: SolverOptions,
) -> Result<SolveReport, GameError> {
    let engine = GameEngine::new(
        config.clone(),
        matrix.to_vec(),
        sequences.to_vec(),
        sequence_values.to_vec(),
    )?;
    let max_suffix_len = sequences
        .iter()
        .map(|sequence| sequence.len().saturating_sub(1))
        .max()
        .unwrap_or(0);
    let total_score = sequence_values.iter().sum();

    let mut report = SolveReport::default();
    let mut memo = HashMap::new();
    let outcome = solve_state(engine, options, max_suffix_len, &mut memo, &mut report);

    report.total_score = total_score;
    report.max_score = outcome.best_score;
    report.expected_score = if outcome.terminal_paths.value == 0 {
        0.0
    } else {
        (outcome.terminal_score_sum / outcome.terminal_paths.value as f64) as f32
    };
    report.expected_score_ratio = if total_score == 0 {
        0.0
    } else {
        (report.expected_score / total_score as f32).clamp(0.0, 1.0)
    };
    report.all_sequences_solvable = outcome.min_moves_to_all.is_some();
    report.min_moves_to_all = outcome.min_moves_to_all;
    report.min_moves_to_max_score = outcome.min_moves_to_best;
    report.solution_count = outcome.all_paths.value;
    report.solution_count_capped = outcome.all_paths.capped;
    report.terminal_paths = outcome.terminal_paths.value;
    report.terminal_paths_capped = outcome.terminal_paths.capped;
    report.max_score_paths = outcome.best_paths.value;
    report.max_score_paths_capped = outcome.best_paths.capped;
    report.difficulty = evaluate_difficulty(config, matrix, sequences, total_score, &report);

    Ok(report)
}

fn solve_state(
    engine: GameEngine,
    options: SolverOptions,
    max_suffix_len: usize,
    memo: &mut HashMap<SearchKey, SearchOutcome>,
    report: &mut SolveReport,
) -> SearchOutcome {
    if report.node_cap_reached {
        return incomplete_outcome(&engine);
    }
    if report.nodes_searched >= options.node_limit {
        report.node_cap_reached = true;
        return incomplete_outcome(&engine);
    }

    if engine.is_terminal() {
        report.nodes_searched += 1;
        let score = engine.compute_score();
        let moves = engine.state.buffer_tokens.len();
        let all_uploaded = engine.has_uploaded_all_sequences();
        if !all_uploaded {
            report.dead_ends += 1;
        }

        return SearchOutcome {
            best_score: score,
            min_moves_to_best: Some(moves),
            min_moves_to_all: all_uploaded.then_some(moves),
            terminal_paths: CappedCount::one(),
            all_paths: if all_uploaded {
                CappedCount::one()
            } else {
                CappedCount::default()
            },
            best_paths: CappedCount::one(),
            terminal_score_sum: score as f64,
        };
    }

    let key = search_key(&engine, max_suffix_len);
    if let Some(outcome) = memo.get(&key) {
        return outcome.clone();
    }

    report.nodes_searched += 1;

    let mut legal_moves: Vec<Cell> = engine.legal_moves().into_iter().collect();
    legal_moves.sort_unstable();

    if legal_moves.is_empty() {
        report.dead_ends += 1;
        let moves = engine.state.buffer_tokens.len();
        let score = engine.compute_score();
        let outcome = SearchOutcome {
            best_score: score,
            min_moves_to_best: Some(moves),
            terminal_paths: CappedCount::one(),
            best_paths: CappedCount::one(),
            terminal_score_sum: score as f64,
            ..Default::default()
        };
        memo.insert(key, outcome.clone());
        return outcome;
    }

    if legal_moves.len() == 1 {
        report.forced_states += 1;
    } else {
        report.branch_points += 1;
        report.total_branches += legal_moves.len();
    }
    report.max_branching_factor = report.max_branching_factor.max(legal_moves.len());

    let cap = options.solution_cap;
    let mut child_outcomes = Vec::with_capacity(legal_moves.len());
    for cell in legal_moves {
        let mut branch = engine.clone();
        if branch.apply_move(cell).is_ok() {
            child_outcomes.push(solve_state(branch, options, max_suffix_len, memo, report));
        }
    }

    let mut outcome = SearchOutcome {
        best_score: engine.compute_score(),
        min_moves_to_best: Some(engine.state.buffer_tokens.len()),
        ..Default::default()
    };
    for child in &child_outcomes {
        let accepted_terminal_paths =
            capped_addend(outcome.terminal_paths, child.terminal_paths, cap);
        if accepted_terminal_paths > 0 && child.terminal_paths.value > 0 {
            let child_average_score = child.terminal_score_sum / child.terminal_paths.value as f64;
            outcome.terminal_score_sum += child_average_score * accepted_terminal_paths as f64;
        }
        outcome.terminal_paths.add(child.terminal_paths, cap);
        outcome.all_paths.add(child.all_paths, cap);

        if child.min_moves_to_all.is_some() {
            outcome.min_moves_to_all = min_option(outcome.min_moves_to_all, child.min_moves_to_all);
        }

        if outcome.min_moves_to_best.is_none() || child.best_score > outcome.best_score {
            outcome.best_score = child.best_score;
            outcome.min_moves_to_best = child.min_moves_to_best;
            outcome.best_paths = child.best_paths;
        } else if child.best_score == outcome.best_score {
            outcome.min_moves_to_best =
                min_option(outcome.min_moves_to_best, child.min_moves_to_best);
            outcome.best_paths.add(child.best_paths, cap);
        }
    }

    for child in &child_outcomes {
        if child.best_score == outcome.best_score {
            report.optimal_moves += 1;
        } else {
            report.trap_moves += 1;
        }
    }

    memo.insert(key, outcome.clone());
    outcome
}

fn incomplete_outcome(engine: &GameEngine) -> SearchOutcome {
    SearchOutcome {
        best_score: engine.compute_score(),
        min_moves_to_best: Some(engine.state.buffer_tokens.len()),
        ..Default::default()
    }
}

fn capped_addend(current: CappedCount, incoming: CappedCount, cap: usize) -> usize {
    let cap = cap.max(1);
    if current.value >= cap {
        0
    } else {
        incoming.value.min(cap - current.value)
    }
}

fn min_option(left: Option<usize>, right: Option<usize>) -> Option<usize> {
    match (left, right) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

fn ratio(part: usize, whole: usize) -> f32 {
    if whole == 0 {
        0.0
    } else {
        (part as f32 / whole as f32).clamp(0.0, 1.0)
    }
}

fn evaluate_difficulty(
    config: &BreachConfig,
    matrix: &[Vec<Token>],
    sequences: &[Vec<Token>],
    total_score: u32,
    report: &SolveReport,
) -> DifficultyReport {
    let average_branching_factor = average_branching_factor(report);
    let branch_pressure = ((average_branching_factor - 1.0)
        / config.matrix_size.saturating_sub(1).max(1) as f32)
        .clamp(0.0, 1.0);
    let buffer_pressure = report
        .min_moves_to_all
        .or(report.min_moves_to_max_score)
        .map(|moves| moves as f32 / config.buffer_size.max(1) as f32)
        .unwrap_or(1.0)
        .clamp(0.0, 1.0);
    let sequence_pressure = sequence_pressure(sequences, config.buffer_size);
    let token_ambiguity = token_ambiguity(matrix, sequences);
    let forced_state_ratio = ratio(
        report.forced_states,
        report.forced_states + report.branch_points,
    );
    let time_pressure = time_pressure(
        config,
        sequences.len(),
        report.min_moves_to_all.or(report.min_moves_to_max_score),
        average_branching_factor,
        token_ambiguity,
    );

    if report.node_cap_reached {
        return DifficultyReport {
            average_branching_factor,
            branch_pressure,
            buffer_pressure,
            sequence_pressure,
            token_ambiguity,
            forced_state_ratio,
            time_pressure,
            tier: DifficultyTier::Uncertain,
            ..Default::default()
        };
    }

    let solution_rarity = 1.0 - ratio(report.solution_count, report.terminal_paths);
    let optimal_rarity = 1.0 - ratio(report.max_score_paths, report.terminal_paths);
    let trap_move_ratio = ratio(report.trap_moves, report.trap_moves + report.optimal_moves);
    let dead_end_ratio = ratio(report.dead_ends, report.terminal_paths);
    let score_coverage = if total_score == 0 {
        1.0
    } else {
        report.max_score as f32 / total_score as f32
    }
    .clamp(0.0, 1.0);
    let score_pressure =
        (0.7 * (1.0 - report.expected_score_ratio) + 0.3 * (1.0 - score_coverage)).clamp(0.0, 1.0);
    let confidence = if report.terminal_paths_capped
        || report.solution_count_capped
        || report.max_score_paths_capped
    {
        0.75
    } else {
        1.0
    };

    let weighted_pressure = 0.16 * solution_rarity
        + 0.14 * optimal_rarity
        + 0.13 * score_pressure
        + 0.13 * trap_move_ratio
        + 0.11 * dead_end_ratio
        + 0.10 * buffer_pressure
        + 0.09 * branch_pressure
        + 0.07 * token_ambiguity
        + 0.04 * sequence_pressure
        + 0.03 * time_pressure;
    let forced_relief = 0.08 * forced_state_ratio;
    let rating = (100.0 * (weighted_pressure - forced_relief).clamp(0.0, 1.0)).min(100.0);

    let tier = if rating < 15.0 {
        DifficultyTier::Trivial
    } else if rating < 35.0 {
        DifficultyTier::Easy
    } else if rating < 60.0 {
        DifficultyTier::Medium
    } else if rating < 80.0 {
        DifficultyTier::Hard
    } else {
        DifficultyTier::Extreme
    };

    DifficultyReport {
        rating,
        tier,
        average_branching_factor,
        branch_pressure,
        buffer_pressure,
        sequence_pressure,
        token_ambiguity,
        solution_rarity,
        optimal_rarity,
        score_pressure,
        trap_move_ratio,
        dead_end_ratio,
        forced_state_ratio,
        time_pressure,
        confidence,
    }
}

fn average_branching_factor(report: &SolveReport) -> f32 {
    if report.branch_points == 0 {
        1.0
    } else {
        report.total_branches as f32 / report.branch_points as f32
    }
}

fn sequence_pressure(sequences: &[Vec<Token>], buffer_size: usize) -> f32 {
    if sequences.is_empty() || buffer_size == 0 {
        return 0.0;
    }
    let sequence_count_pressure = (sequences.len() as f32 / 5.0).clamp(0.0, 1.0);
    let average_len = sequences
        .iter()
        .map(|sequence| sequence.len())
        .sum::<usize>() as f32
        / sequences.len() as f32;
    let length_pressure = (average_len / buffer_size as f32).clamp(0.0, 1.0);
    let total_token_pressure = (sequences
        .iter()
        .map(|sequence| sequence.len())
        .sum::<usize>() as f32
        / buffer_size.max(1) as f32)
        .clamp(0.0, 2.0)
        / 2.0;

    (0.40 * sequence_count_pressure + 0.35 * length_pressure + 0.25 * total_token_pressure)
        .clamp(0.0, 1.0)
}

fn token_ambiguity(matrix: &[Vec<Token>], sequences: &[Vec<Token>]) -> f32 {
    if matrix.is_empty() || sequences.is_empty() {
        return 0.0;
    }

    let mut counts: HashMap<&str, usize> = HashMap::new();
    let mut cell_count = 0usize;
    for token in matrix.iter().flatten() {
        *counts.entry(token.as_str()).or_default() += 1;
        cell_count += 1;
    }
    if cell_count == 0 {
        return 0.0;
    }

    let mut sequence_token_occurrences = 0usize;
    let mut repeated_choice_pressure = 0.0f32;
    let mut matching_cells = HashMap::new();
    for token in sequences.iter().flatten() {
        let occurrences = *counts.get(token.as_str()).unwrap_or(&0);
        sequence_token_occurrences += 1;
        repeated_choice_pressure +=
            occurrences.saturating_sub(1) as f32 / matrix.len().saturating_sub(1).max(1) as f32;
        matching_cells.insert(token.as_str(), occurrences);
    }

    if sequence_token_occurrences == 0 {
        return 0.0;
    }

    let repeated_choice_pressure =
        (repeated_choice_pressure / sequence_token_occurrences as f32).clamp(0.0, 1.0);
    let sequence_token_density =
        (matching_cells.values().sum::<usize>() as f32 / cell_count as f32).clamp(0.0, 1.0);
    let token_reuse_pressure = (1.0 - counts.len() as f32 / cell_count as f32).clamp(0.0, 1.0);

    (0.50 * repeated_choice_pressure + 0.30 * sequence_token_density + 0.20 * token_reuse_pressure)
        .clamp(0.0, 1.0)
}

fn time_pressure(
    config: &BreachConfig,
    sequence_count: usize,
    required_moves: Option<usize>,
    average_branching_factor: f32,
    token_ambiguity: f32,
) -> f32 {
    if config.time_limit_seconds == 0 {
        return 0.0;
    }

    let moves = required_moves.unwrap_or(config.buffer_size).max(1) as f32;
    let estimated_human_seconds = moves * 0.85
        + sequence_count as f32 * 0.65
        + average_branching_factor.ln_1p() * 2.4
        + token_ambiguity * 4.0;

    (estimated_human_seconds / config.time_limit_seconds as f32).clamp(0.0, 1.0)
}

fn search_key(engine: &GameEngine, max_suffix_len: usize) -> SearchKey {
    let matrix_size = engine.config.matrix_size;
    let mut selected_mask = 0u128;
    for &(row, col) in &engine.state.selected_cells {
        let bit = row * matrix_size + col;
        if bit < u128::BITS as usize {
            selected_mask |= 1u128 << bit;
        }
    }

    let current_index = engine
        .state
        .current_index
        .map(|(row, col)| row * matrix_size + col);
    let uploaded_mask =
        engine
            .state
            .uploaded_results
            .iter()
            .enumerate()
            .fold(0u64, |mask, (idx, uploaded)| {
                if *uploaded && idx < u64::BITS as usize {
                    mask | (1u64 << idx)
                } else {
                    mask
                }
            });
    let suffix_start = engine
        .state
        .buffer_tokens
        .len()
        .saturating_sub(max_suffix_len);

    SearchKey {
        selected_mask,
        current_index,
        current_constraint: engine.state.current_constraint,
        uploaded_mask,
        suffix: engine.state.buffer_tokens[suffix_start..].to_vec(),
    }
}
