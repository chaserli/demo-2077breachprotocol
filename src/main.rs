use macroquad::prelude::*;

use rust_breach_protocol::engine::{BreachConfig, Cell, ConstraintMode, GameEngine};
use rust_breach_protocol::generator::generate_solvable_game;

const ASSET_BG_BYTES: &[u8] = include_bytes!("../assets/img/tutorial_bg1.png");
const ASSET_BUFFER_BYTES: &[u8] = include_bytes!("../assets/img/Buffer-empty.png");
const ASSET_ICON_MATRIX_BYTES: &[u8] = include_bytes!("../assets/img/icon-code-matrix.png");
const ASSET_ICON_SEQUENCE_BYTES: &[u8] = include_bytes!("../assets/img/icon-code-sequnce.png");
const FONT_MEDIUM_BYTES: &[u8] = include_bytes!("../assets/font/Rajdhani-Medium.ttf");
const FONT_BOLD_BYTES: &[u8] = include_bytes!("../assets/font/Rajdhani-Bold.ttf");

fn window_conf() -> Conf {
    Conf {
        window_title: "Breach Protocol".to_string(),
        window_width: 1920,
        window_height: 1080,
        window_resizable: true,
        high_dpi: true,
        // Keep the bundled app icon instead of miniquad's default.
        icon: None,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let assets = load_assets().await;

    let mut settings = SettingsState::new(5, 8, 30, 2);
    let cli_seed = None;
    let mut engine: Option<GameEngine> = None;
    let mut screen = Screen::Settings;
    let mut mouse_down_prev = false;
    loop {
        clear_background(Color::from_rgba(8, 10, 12, 255));
        draw_background(&assets);
        match screen {
            Screen::Settings => {
                if draw_settings(&mut settings, &assets) {
                    match build_engine_from_settings(&settings, cli_seed) {
                        Ok(new_engine) => {
                            engine = Some(new_engine);
                            screen = Screen::Playing;
                        }
                        Err(err) => {
                            eprintln!("Failed to build game: {}", err);
                        }
                    }
                }
            }
            Screen::Playing => {
                let engine_ref = engine.as_mut().expect("engine missing");
                let frame = layout_for(engine_ref);
                let legal = engine_ref.legal_moves();
                let hover_cell = hover_at(
                    frame.grid_origin,
                    frame.cell_size,
                    engine_ref.config.matrix_size,
                );
                draw_hud_lines(&frame);
                draw_timer_panel(engine_ref, &assets, &frame);
                draw_buffer_panel(engine_ref, &assets, &frame);
                let sequence_hover = draw_sequences_panel(engine_ref, &assets, &frame);
                draw_grid(
                    engine_ref,
                    &legal,
                    hover_cell,
                    &frame,
                    &assets,
                    sequence_hover.hovered_token.as_deref(),
                );

                if !engine_ref.is_terminal() {
                    let mouse_down = is_mouse_button_down(MouseButton::Left);
                    if !mouse_down && mouse_down_prev {
                        let click_cell = hover_at(
                            frame.grid_origin,
                            frame.cell_size,
                            engine_ref.config.matrix_size,
                        );
                        if let Some(cell) = click_cell {
                            if legal.contains(&cell) {
                                let _ = engine_ref.apply_move(cell);
                            }
                        }
                    }
                    mouse_down_prev = mouse_down;
                } else {
                    draw_game_over(engine_ref, &frame, &assets);
                    if draw_restart_button(&assets) {
                        screen = Screen::Settings;
                    }
                }
            }
        }

        next_frame().await;
    }
}

enum Screen {
    Settings,
    Playing,
}

struct SettingsState {
    matrix_size: usize,
    buffer_size: usize,
    time_limit_index: usize,
    sequences: usize,
    token_variety: usize,
}

impl SettingsState {
    fn new(matrix_size: usize, buffer_size: usize, time_limit: u64, sequences: usize) -> Self {
        let time_limits = time_limit_options();
        let time_limit_index = time_limits
            .iter()
            .position(|&v| v == time_limit)
            .unwrap_or(2);
        Self {
            matrix_size: matrix_size.clamp(3, 9),
            buffer_size: buffer_size.clamp(4, 16),
            time_limit_index,
            sequences: sequences.clamp(1, 5),
            token_variety: 6,
        }
    }
}

fn time_limit_options() -> Vec<u64> {
    vec![0, 15, 30, 45, 60]
}

fn build_engine_from_settings(
    settings: &SettingsState,
    seed: Option<u64>,
) -> Result<GameEngine, String> {
    let time_limit = time_limit_options()[settings.time_limit_index];
    let base_tokens = ["55", "1C", "7A", "BD", "E9", "FF", "00", "9A", "A1", "C3"];
    let variety = settings.token_variety.clamp(2, base_tokens.len());
    let token_alphabet = base_tokens[..variety]
        .iter()
        .map(|token| (*token).to_string())
        .collect();
    let config = BreachConfig {
        matrix_size: settings.matrix_size,
        buffer_size: settings.buffer_size,
        time_limit_seconds: time_limit,
        token_alphabet,
        seed,
    };

    let (matrix, sequences, values) =
        generate_solvable_game(&config, settings.sequences, true).map_err(|err| err.to_string())?;
    GameEngine::new(config, matrix, sequences, values).map_err(|err| err.to_string())
}

fn draw_settings(settings: &mut SettingsState, assets: &Assets) -> bool {
    let panel_w = 520.0;
    let panel_h = 360.0;
    let panel = Rect::new(
        (screen_width() - panel_w) * 0.5,
        (screen_height() - panel_h) * 0.5,
        panel_w,
        panel_h,
    );
    draw_rectangle(
        panel.x,
        panel.y,
        panel.w,
        panel.h,
        Color::from_rgba(10, 12, 16, 220),
    );
    draw_rectangle_lines(
        panel.x,
        panel.y,
        panel.w,
        panel.h,
        2.0,
        Color::from_rgba(200, 255, 120, 220),
    );

    let title_params = TextParams {
        font: Some(&assets.font_bold),
        font_size: 26,
        color: Color::from_rgba(200, 255, 120, 255),
        ..Default::default()
    };
    draw_text_ex(
        "BREACH SETTINGS",
        panel.x + 20.0,
        panel.y + 36.0,
        title_params,
    );

    let label_params = TextParams {
        font: Some(&assets.font_medium),
        font_size: 20,
        color: Color::from_rgba(210, 255, 140, 255),
        ..Default::default()
    };
    let row_start = panel.y + 80.0;
    let row_gap = 46.0;

    let matrix_row = Rect::new(panel.x + 20.0, row_start, panel.w - 40.0, 34.0);
    let delta = draw_setting_row(
        "Matrix Size",
        &format!("{}x{}", settings.matrix_size, settings.matrix_size),
        matrix_row,
        label_params.clone(),
        settings.matrix_size < 9,
        settings.matrix_size > 3,
    );
    if delta != 0 {
        settings.matrix_size = (settings.matrix_size as i32 + delta).clamp(3, 9) as usize;
    }

    let buffer_row = Rect::new(panel.x + 20.0, row_start + row_gap, panel.w - 40.0, 34.0);
    let delta = draw_setting_row(
        "Buffer Size",
        &format!("{}", settings.buffer_size),
        buffer_row,
        label_params.clone(),
        settings.buffer_size < 16,
        settings.buffer_size > 4,
    );
    if delta != 0 {
        settings.buffer_size = (settings.buffer_size as i32 + delta).clamp(4, 16) as usize;
    }

    let seq_row = Rect::new(
        panel.x + 20.0,
        row_start + row_gap * 2.0,
        panel.w - 40.0,
        34.0,
    );
    let delta = draw_setting_row(
        "Sequences",
        &format!("{}", settings.sequences),
        seq_row,
        label_params.clone(),
        settings.sequences < 5,
        settings.sequences > 1,
    );
    if delta != 0 {
        settings.sequences = (settings.sequences as i32 + delta).clamp(1, 5) as usize;
    }

    let token_row = Rect::new(
        panel.x + 20.0,
        row_start + row_gap * 3.0,
        panel.w - 40.0,
        34.0,
    );
    let delta = draw_setting_row(
        "Token Variety",
        &format!("{}", settings.token_variety),
        token_row,
        label_params.clone(),
        settings.token_variety < 10,
        settings.token_variety > 2,
    );
    if delta != 0 {
        settings.token_variety = (settings.token_variety as i32 + delta).clamp(2, 10) as usize;
    }

    let time_limits = time_limit_options();
    let time_label = if time_limits[settings.time_limit_index] == 0 {
        "No Limit".to_string()
    } else {
        format!("{}s", time_limits[settings.time_limit_index])
    };
    let time_row = Rect::new(
        panel.x + 20.0,
        row_start + row_gap * 4.0,
        panel.w - 40.0,
        34.0,
    );
    let delta = draw_setting_row(
        "Time Limit",
        &time_label,
        time_row,
        label_params.clone(),
        settings.time_limit_index + 1 < time_limits.len(),
        settings.time_limit_index > 0,
    );
    if delta != 0 {
        let next = settings.time_limit_index as i32 + delta;
        settings.time_limit_index = next.clamp(0, (time_limits.len() - 1) as i32) as usize;
    }

    let start_button = Rect::new(
        panel.x + panel.w - 160.0,
        panel.y + panel.h - 50.0,
        140.0,
        34.0,
    );
    draw_rectangle(
        start_button.x,
        start_button.y,
        start_button.w,
        start_button.h,
        Color::from_rgba(200, 255, 120, 200),
    );
    let start_params = TextParams {
        font: Some(&assets.font_bold),
        font_size: 18,
        color: Color::from_rgba(8, 10, 12, 255),
        ..Default::default()
    };
    draw_text_ex(
        "START",
        start_button.x + 40.0,
        start_button.y + 24.0,
        start_params,
    );

    if is_mouse_button_pressed(MouseButton::Left) {
        let (mx, my) = mouse_position();
        if mx >= start_button.x
            && mx <= start_button.x + start_button.w
            && my >= start_button.y
            && my <= start_button.y + start_button.h
        {
            return true;
        }
    }

    false
}

fn draw_setting_row(
    label: &str,
    value: &str,
    rect: Rect,
    label_params: TextParams,
    can_inc: bool,
    can_dec: bool,
) -> i32 {
    draw_text_ex(label, rect.x, rect.y + 24.0, label_params.clone());
    draw_text_ex(value, rect.x + 220.0, rect.y + 24.0, label_params);

    let minus = Rect::new(rect.x + rect.w - 80.0, rect.y + 4.0, 26.0, 26.0);
    let plus = Rect::new(rect.x + rect.w - 40.0, rect.y + 4.0, 26.0, 26.0);
    draw_rectangle_lines(
        minus.x,
        minus.y,
        minus.w,
        minus.h,
        1.0,
        Color::from_rgba(200, 255, 120, 200),
    );
    draw_rectangle_lines(
        plus.x,
        plus.y,
        plus.w,
        plus.h,
        1.0,
        Color::from_rgba(200, 255, 120, 200),
    );
    draw_text(
        "-",
        minus.x + 8.0,
        minus.y + 20.0,
        20.0,
        Color::from_rgba(200, 255, 120, 255),
    );
    draw_text(
        "+",
        plus.x + 7.0,
        plus.y + 20.0,
        20.0,
        Color::from_rgba(200, 255, 120, 255),
    );

    if is_mouse_button_pressed(MouseButton::Left) {
        let (mx, my) = mouse_position();
        if mx >= plus.x && mx <= plus.x + plus.w && my >= plus.y && my <= plus.y + plus.h && can_inc
        {
            return 1;
        }
        if mx >= minus.x
            && mx <= minus.x + minus.w
            && my >= minus.y
            && my <= minus.y + minus.h
            && can_dec
        {
            return -1;
        }
    }
    0
}

struct Layout {
    grid_origin: Vec2,
    cell_size: f32,
    matrix_panel: Rect,
    sequence_panel: Rect,
    timer_panel: Rect,
    buffer_panel: Rect,
    colors: Palette,
}

struct Palette {
    grid_fill: Color,
    grid_outline: Color,
    grid_disabled: Color,
    grid_hover: Color,
    grid_selected: Color,
    legal_outline: Color,
    panel_text: Color,
    accent: Color,
}

struct Assets {
    background: Texture2D,
    buffer_slot: Texture2D,
    icon_matrix: Texture2D,
    icon_sequence: Texture2D,
    font_medium: Font,
    font_bold: Font,
}

struct SequenceHover {
    hovered_token: Option<String>,
}

async fn load_assets() -> Assets {
    let background = Texture2D::from_file_with_format(ASSET_BG_BYTES, Some(ImageFormat::Png));
    let buffer_slot = Texture2D::from_file_with_format(ASSET_BUFFER_BYTES, Some(ImageFormat::Png));
    let icon_matrix =
        Texture2D::from_file_with_format(ASSET_ICON_MATRIX_BYTES, Some(ImageFormat::Png));
    let icon_sequence =
        Texture2D::from_file_with_format(ASSET_ICON_SEQUENCE_BYTES, Some(ImageFormat::Png));
    let font_medium = load_ttf_font_from_bytes(FONT_MEDIUM_BYTES).unwrap();
    let font_bold = load_ttf_font_from_bytes(FONT_BOLD_BYTES).unwrap();

    background.set_filter(FilterMode::Linear);
    buffer_slot.set_filter(FilterMode::Linear);
    icon_matrix.set_filter(FilterMode::Linear);
    icon_sequence.set_filter(FilterMode::Linear);

    Assets {
        background,
        buffer_slot,
        icon_matrix,
        icon_sequence,
        font_medium,
        font_bold,
    }
}

fn layout_for(engine: &GameEngine) -> Layout {
    let screen_w = screen_width();
    let screen_h = screen_height();
    let margin = 32.0;
    let top_bar = 120.0;
    let matrix_width = screen_w * 0.42;
    let seq_width = screen_w * 0.42;
    let matrix_height = screen_h - top_bar - margin * 2.0;
    let seq_height = matrix_height;
    let matrix_panel = Rect::new(margin, top_bar, matrix_width, matrix_height);
    let sequence_panel = Rect::new(
        screen_w - seq_width - margin,
        top_bar,
        seq_width,
        seq_height,
    );
    let timer_panel = Rect::new(margin, margin * 1.2, matrix_width, 70.0);
    let buffer_panel = Rect::new(sequence_panel.x, margin * 1.2, sequence_panel.w, 70.0);
    let grid_area_w = matrix_panel.w - 60.0;
    let grid_area_h = matrix_panel.h - 120.0;
    let size = engine.config.matrix_size as f32;
    let cell_size = (grid_area_w / size)
        .min(grid_area_h / size)
        .clamp(36.0, 96.0);
    let grid_origin = vec2(matrix_panel.x + 30.0, matrix_panel.y + 80.0);

    Layout {
        grid_origin,
        cell_size,
        matrix_panel,
        sequence_panel,
        timer_panel,
        buffer_panel,
        colors: Palette {
            grid_fill: Color::from_rgba(18, 20, 24, 255),
            grid_outline: Color::from_rgba(110, 130, 70, 180),
            grid_disabled: Color::from_rgba(18, 20, 24, 255),
            grid_hover: Color::from_rgba(120, 255, 200, 200),
            grid_selected: Color::from_rgba(60, 120, 110, 220),
            legal_outline: Color::from_rgba(200, 255, 120, 220),
            panel_text: Color::from_rgba(215, 255, 140, 255),
            accent: Color::from_rgba(200, 255, 120, 255),
        },
    }
}

fn hover_at(origin: Vec2, cell_size: f32, matrix_size: usize) -> Option<Cell> {
    let (mx, my) = mouse_position();
    let x = mx - origin.x;
    let y = my - origin.y;
    if x < 0.0 || y < 0.0 {
        return None;
    }

    let col = (x / cell_size) as usize;
    let row = (y / cell_size) as usize;
    if row < matrix_size && col < matrix_size {
        Some((row, col))
    } else {
        None
    }
}

fn draw_background(assets: &Assets) {
    let screen_w = screen_width();
    let screen_h = screen_height();
    let params = DrawTextureParams {
        dest_size: Some(vec2(screen_w, screen_h)),
        ..Default::default()
    };
    draw_texture_ex(
        &assets.background,
        0.0,
        0.0,
        Color::from_rgba(255, 255, 255, 120),
        params,
    );
}

fn draw_hud_lines(layout: &Layout) {
    let line = layout.colors.accent;
    let outer = Rect::new(20.0, 20.0, screen_width() - 40.0, screen_height() - 40.0);
    draw_rectangle_lines(outer.x, outer.y, outer.w, outer.h, 1.0, line);
    draw_rectangle_lines(
        outer.x + 10.0,
        outer.y + 10.0,
        outer.w - 20.0,
        outer.h - 20.0,
        1.0,
        line,
    );
}

fn draw_timer_panel(engine: &GameEngine, assets: &Assets, layout: &Layout) {
    let rect = layout.timer_panel;
    let title_params = TextParams {
        font: Some(&assets.font_medium),
        font_size: 22,
        color: layout.colors.panel_text,
        ..Default::default()
    };
    draw_text_ex("BREACH TIME REMAINING", rect.x, rect.y + 24.0, title_params);

    let total = engine.config.time_limit_seconds as f32;
    let (remaining_label, ratio) = if engine.config.time_limit_seconds == 0 {
        ("NO LIMIT".to_string(), 1.0)
    } else {
        let remaining = engine.get_remaining_time().as_secs_f32();
        (
            format!("{:.2}", remaining),
            (remaining / total).clamp(0.0, 1.0),
        )
    };

    let box_w = 80.0;
    let box_rect = Rect::new(rect.x + rect.w - box_w, rect.y + 6.0, box_w, 30.0);
    draw_rectangle_lines(
        box_rect.x,
        box_rect.y,
        box_rect.w,
        box_rect.h,
        1.0,
        layout.colors.panel_text,
    );
    let time_params = TextParams {
        font: Some(&assets.font_bold),
        font_size: 20,
        color: layout.colors.panel_text,
        ..Default::default()
    };
    draw_text_ex(
        &remaining_label,
        box_rect.x + 8.0,
        box_rect.y + 22.0,
        time_params,
    );

    let bar_rect = Rect::new(rect.x, rect.y + 40.0, rect.w, 6.0);
    draw_rectangle_lines(
        bar_rect.x,
        bar_rect.y,
        bar_rect.w,
        bar_rect.h,
        1.0,
        layout.colors.panel_text,
    );
    draw_rectangle(
        bar_rect.x,
        bar_rect.y,
        bar_rect.w * ratio,
        bar_rect.h,
        layout.colors.panel_text,
    );
}

fn draw_buffer_panel(engine: &GameEngine, assets: &Assets, layout: &Layout) {
    let rect = layout.buffer_panel;
    let title_params = TextParams {
        font: Some(&assets.font_medium),
        font_size: 22,
        color: layout.colors.panel_text,
        ..Default::default()
    };
    draw_text_ex("BUFFER", rect.x + 140.0, rect.y + 24.0, title_params);

    let slot_size = 26.0;
    let start_x = rect.x + 40.0;
    let y = rect.y + 32.0;
    for i in 0..engine.config.buffer_size {
        let x = start_x + i as f32 * (slot_size + 6.0);
        let params = DrawTextureParams {
            dest_size: Some(vec2(slot_size, slot_size)),
            ..Default::default()
        };
        draw_texture_ex(&assets.buffer_slot, x, y, WHITE, params);
        if let Some(token) = engine.state.buffer_tokens.get(i) {
            let params = TextParams {
                font: Some(&assets.font_bold),
                font_size: 18,
                color: layout.colors.panel_text,
                ..Default::default()
            };
            draw_text_ex(token, x + 4.0, y + 18.0, params);
        }
    }
}

fn draw_grid(
    engine: &GameEngine,
    legal: &std::collections::HashSet<Cell>,
    hover: Option<Cell>,
    layout: &Layout,
    assets: &Assets,
    hovered_sequence_token: Option<&str>,
) {
    let size = engine.config.matrix_size;
    let font_size = (layout.cell_size * 0.35).max(16.0);

    draw_constraint_overlay(engine, legal, hover, layout);

    for row in 0..size {
        for col in 0..size {
            let cell = (row, col);
            let x = layout.grid_origin.x + col as f32 * layout.cell_size;
            let y = layout.grid_origin.y + row as f32 * layout.cell_size;
            let rect = Rect::new(x, y, layout.cell_size, layout.cell_size);

            let mut fill = layout.colors.grid_fill;
            if engine.state.selected_cells.contains(&cell) {
                fill = layout.colors.grid_selected;
            } else if !legal.contains(&cell) && !engine.state.buffer_tokens.is_empty() {
                fill = layout.colors.grid_disabled;
            }

            draw_rectangle(rect.x, rect.y, rect.w, rect.h, fill);
            draw_rectangle_lines(
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                1.0,
                layout.colors.grid_outline,
            );

            if legal.contains(&cell) {
                draw_rectangle_lines(
                    rect.x + 2.0,
                    rect.y + 2.0,
                    rect.w - 4.0,
                    rect.h - 4.0,
                    2.0,
                    layout.colors.legal_outline,
                );
                draw_rectangle_lines(
                    rect.x - 1.0,
                    rect.y - 1.0,
                    rect.w + 2.0,
                    rect.h + 2.0,
                    1.0,
                    Color::from_rgba(200, 255, 120, 90),
                );
            }

            if hover == Some(cell) {
                if legal.contains(&cell) {
                    draw_rectangle_lines(
                        rect.x + 3.0,
                        rect.y + 3.0,
                        rect.w - 6.0,
                        rect.h - 6.0,
                        2.0,
                        layout.colors.grid_hover,
                    );
                    draw_rectangle_lines(
                        rect.x - 2.0,
                        rect.y - 2.0,
                        rect.w + 4.0,
                        rect.h + 4.0,
                        1.0,
                        Color::from_rgba(140, 255, 220, 120),
                    );
                } else {
                    draw_rectangle_lines(
                        rect.x + 2.0,
                        rect.y + 2.0,
                        rect.w - 4.0,
                        rect.h - 4.0,
                        2.0,
                        Color::from_rgba(255, 120, 80, 200),
                    );
                }
            }

            let token = &engine.state.matrix[row][col];
            let font_ref = &assets.font_bold;
            let params = TextParams {
                font: Some(font_ref),
                font_size: font_size as u16,
                color: layout.colors.panel_text,
                ..Default::default()
            };
            let dims = measure_text(token, Some(font_ref), params.font_size, 1.0);
            let tx = rect.x + (rect.w - dims.width) * 0.5;
            let ty = rect.y + (rect.h + dims.height) * 0.5 - 4.0;
            draw_text_ex(token, tx, ty, params);

            if let Some(token_match) = hovered_sequence_token {
                if token == token_match {
                    draw_rectangle(
                        rect.x + 1.0,
                        rect.y + 1.0,
                        rect.w - 2.0,
                        rect.h - 2.0,
                        Color::from_rgba(255, 160, 80, 80),
                    );
                }
            }
        }
    }

    draw_matrix_header(layout, assets);
}

fn draw_constraint_overlay(
    engine: &GameEngine,
    legal: &std::collections::HashSet<Cell>,
    hover: Option<Cell>,
    layout: &Layout,
) {
    let size = engine.config.matrix_size;
    let overlay = Color::from_rgba(140, 255, 160, 30);
    let active = Color::from_rgba(200, 255, 120, 45);

    if engine.state.buffer_tokens.is_empty() {
        let y = layout.grid_origin.y;
        draw_rectangle(
            layout.grid_origin.x,
            y,
            layout.cell_size * size as f32,
            layout.cell_size,
            active,
        );
        return;
    }

    if let Some((row, col)) = engine.state.current_index {
        let row_y = layout.grid_origin.y + row as f32 * layout.cell_size;
        let col_x = layout.grid_origin.x + col as f32 * layout.cell_size;
        match engine.state.current_constraint {
            ConstraintMode::Row => draw_rectangle(
                layout.grid_origin.x,
                row_y,
                layout.cell_size * size as f32,
                layout.cell_size,
                active,
            ),
            ConstraintMode::Column => draw_rectangle(
                col_x,
                layout.grid_origin.y,
                layout.cell_size,
                layout.cell_size * size as f32,
                active,
            ),
        }
    }

    if let Some(cell) = hover {
        if legal.contains(&cell) {
            let row_y = layout.grid_origin.y + cell.0 as f32 * layout.cell_size;
            let col_x = layout.grid_origin.x + cell.1 as f32 * layout.cell_size;
            draw_rectangle(
                layout.grid_origin.x,
                row_y,
                layout.cell_size * size as f32,
                layout.cell_size,
                overlay,
            );
            draw_rectangle(
                col_x,
                layout.grid_origin.y,
                layout.cell_size,
                layout.cell_size * size as f32,
                overlay,
            );
        }
    }
}

fn draw_matrix_header(layout: &Layout, assets: &Assets) {
    let header = Rect::new(
        layout.matrix_panel.x,
        layout.matrix_panel.y,
        layout.matrix_panel.w,
        40.0,
    );
    draw_rectangle(header.x, header.y, header.w, header.h, layout.colors.accent);
    let params = DrawTextureParams {
        dest_size: Some(vec2(24.0, 24.0)),
        ..Default::default()
    };
    draw_texture_ex(
        &assets.icon_matrix,
        header.x + 12.0,
        header.y + 8.0,
        BLACK,
        params,
    );
    let text_params = TextParams {
        font: Some(&assets.font_medium),
        font_size: 22,
        color: BLACK,
        ..Default::default()
    };
    draw_text_ex("CODE MATRIX", header.x + 46.0, header.y + 26.0, text_params);
    draw_rectangle_lines(
        layout.matrix_panel.x,
        layout.matrix_panel.y,
        layout.matrix_panel.w,
        layout.matrix_panel.h,
        1.0,
        layout.colors.accent,
    );
}

fn draw_sequences_panel(engine: &GameEngine, assets: &Assets, layout: &Layout) -> SequenceHover {
    let rect = layout.sequence_panel;
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, layout.colors.accent);
    let header = Rect::new(rect.x, rect.y, rect.w, 40.0);
    draw_rectangle(
        header.x,
        header.y,
        header.w,
        header.h,
        Color::from_rgba(18, 20, 24, 220),
    );
    draw_rectangle_lines(
        header.x,
        header.y,
        header.w,
        header.h,
        1.0,
        layout.colors.accent,
    );
    let params = DrawTextureParams {
        dest_size: Some(vec2(24.0, 24.0)),
        ..Default::default()
    };
    draw_texture_ex(
        &assets.icon_sequence,
        header.x + 12.0,
        header.y + 8.0,
        WHITE,
        params,
    );
    let text_params = TextParams {
        font: Some(&assets.font_medium),
        font_size: 18,
        color: layout.colors.panel_text,
        ..Default::default()
    };
    draw_text_ex(
        "SEQUENCE REQUIRED TO UPLOAD",
        header.x + 44.0,
        header.y + 26.0,
        text_params,
    );

    let mut cursor_y = header.y + 60.0;
    let mut hovered_token: Option<String> = None;
    let (mx, my) = mouse_position();
    for (idx, sequence) in engine.state.sequences.iter().enumerate() {
        let uploaded = engine.state.uploaded_results[idx];
        let progress = sequence_progress(&engine.state.buffer_tokens, sequence);
        let bar_color = if uploaded {
            Color::from_rgba(20, 120, 70, 200)
        } else {
            Color::from_rgba(30, 30, 30, 180)
        };
        let bar = Rect::new(rect.x + 16.0, cursor_y, rect.w - 32.0, 46.0);
        draw_rectangle(bar.x, bar.y, bar.w, bar.h, bar_color);
        draw_rectangle_lines(bar.x, bar.y, bar.w, bar.h, 1.0, layout.colors.accent);

        let mut token_x = bar.x + 10.0;
        for (token_idx, token) in sequence.iter().enumerate() {
            let params = TextParams {
                font: Some(&assets.font_bold),
                font_size: 20,
                color: layout.colors.panel_text,
                ..Default::default()
            };
            let dims = measure_text(token, Some(&assets.font_bold), params.font_size, 1.0);
            let token_rect = Rect::new(token_x - 2.0, bar.y + 8.0, dims.width + 4.0, 26.0);
            if uploaded || token_idx < progress {
                draw_rectangle(
                    token_rect.x,
                    token_rect.y,
                    token_rect.w,
                    token_rect.h,
                    Color::from_rgba(80, 200, 140, 90),
                );
            }
            if mx >= token_rect.x
                && mx <= token_rect.x + token_rect.w
                && my >= token_rect.y
                && my <= token_rect.y + token_rect.h
            {
                hovered_token = Some(token.clone());
                draw_rectangle(
                    token_rect.x,
                    token_rect.y,
                    token_rect.w,
                    token_rect.h,
                    Color::from_rgba(255, 200, 80, 50),
                );
            }
            draw_text_ex(token, token_x, bar.y + 30.0, params);
            token_x += dims.width + 12.0;
        }
        cursor_y += 54.0;
    }
    SequenceHover { hovered_token }
}

fn sequence_progress(buffer: &[String], sequence: &[String]) -> usize {
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

fn draw_game_over(engine: &GameEngine, layout: &Layout, assets: &Assets) {
    let result = engine.get_game_result();
    let overlay = Color::from_rgba(0, 0, 0, 180);
    draw_rectangle(0.0, 0.0, screen_width(), screen_height(), overlay);

    let text = if result.success {
        "UPLOAD SUCCESS"
    } else {
        "UPLOAD FAILED"
    };
    let size = 42.0;
    let dims = measure_text(text, None, size as u16, 1.0);
    let x = (screen_width() - dims.width) * 0.5;
    let y = screen_height() * 0.4;
    let title_params = TextParams {
        font: Some(&assets.font_bold),
        font_size: size as u16,
        color: layout.colors.panel_text,
        ..Default::default()
    };
    draw_text_ex(text, x, y, title_params);

    let detail = format!(
        "Score: {}  |  Uploaded: {}/{}",
        result.score, result.sequences_uploaded, result.total_sequences
    );
    let detail_dims = measure_text(&detail, None, 22, 1.0);
    let detail_params = TextParams {
        font: Some(&assets.font_medium),
        font_size: 22,
        color: layout.colors.panel_text,
        ..Default::default()
    };
    draw_text_ex(
        &detail,
        (screen_width() - detail_dims.width) * 0.5,
        y + 36.0,
        detail_params,
    );
}

fn draw_restart_button(assets: &Assets) -> bool {
    let button = Rect::new(
        (screen_width() - 220.0) * 0.5,
        screen_height() * 0.6,
        220.0,
        44.0,
    );
    draw_rectangle(
        button.x,
        button.y,
        button.w,
        button.h,
        Color::from_rgba(200, 255, 120, 220),
    );
    draw_rectangle_lines(
        button.x,
        button.y,
        button.w,
        button.h,
        2.0,
        Color::from_rgba(8, 10, 12, 255),
    );
    let params = TextParams {
        font: Some(&assets.font_bold),
        font_size: 20,
        color: Color::from_rgba(8, 10, 12, 255),
        ..Default::default()
    };
    draw_text_ex("NEW GAME", button.x + 48.0, button.y + 28.0, params);

    if is_mouse_button_pressed(MouseButton::Left) {
        let (mx, my) = mouse_position();
        if mx >= button.x
            && mx <= button.x + button.w
            && my >= button.y
            && my <= button.y + button.h
        {
            return true;
        }
    }
    false
}
