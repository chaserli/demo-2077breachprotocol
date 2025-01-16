use std::time::{Duration, Instant};

use macroquad::audio::{PlaySoundParams, play_sound, stop_sound};
use macroquad::prelude::*;
use rust_breach_protocol::engine::{Cell, GameEngine, GameError};

use super::assets::Assets;
use super::layout::{
    SettingsHit, cell_at_point, game_layout, point_in_rect, restart_button_rect, settings_hit,
    settings_layout,
};
use super::render::*;
use super::settings::{Settings, build_engine};

pub(crate) struct GuiApp {
    screen: Screen,
    settings: Settings,
    session: Option<Session>,
    generation_error: Option<String>,
    screen_started_at: Instant,
    bgm_playing: bool,
}

impl GuiApp {
    pub(crate) fn new() -> Self {
        Self {
            screen: Screen::Settings,
            settings: Settings::default(),
            session: None,
            generation_error: None,
            screen_started_at: Instant::now(),
            bgm_playing: false,
        }
    }

    pub(crate) fn update(
        &mut self,
        assets: &Assets,
        mouse_pos: Vec2,
        left_pressed: bool,
        width: f32,
        height: f32,
    ) {
        if let Some(session) = self.session.as_mut() {
            session.tick_timer();
        }
        if left_pressed {
            self.click(assets, mouse_pos, width, height);
        }
    }

    pub(crate) fn draw(&self, assets: &Assets, mouse_pos: Vec2, width: f32, height: f32) {
        draw_background(width, height);
        draw_hud_lines(width, height);
        match self.screen {
            Screen::Settings => self.draw_settings(assets, mouse_pos, width, height),
            Screen::Playing => {
                if let Some(session) = self.session.as_ref() {
                    self.draw_game(session, assets, mouse_pos, width, height);
                } else {
                    self.draw_settings(assets, mouse_pos, width, height);
                }
            }
        }
    }

    fn click(&mut self, assets: &Assets, mouse_pos: Vec2, width: f32, height: f32) {
        match self.screen {
            Screen::Settings => {
                match settings_hit(&self.settings, &settings_layout(width, height), mouse_pos) {
                    Some(SettingsHit::Start) => {
                        Self::play_click(assets);
                        self.start_game(assets);
                    }
                    Some(SettingsHit::Step(field, delta)) if self.settings.step(field, delta) => {
                        Self::play_click(assets);
                        self.generation_error = None;
                    }
                    Some(SettingsHit::Step(_, _)) => {}
                    None => {}
                }
            }
            Screen::Playing => {
                let Some(session) = self.session.as_mut() else {
                    self.screen = Screen::Settings;
                    return;
                };
                let layout = game_layout(width, height, session.engine.config.matrix_size);
                let restart = restart_button_rect(&layout);
                if session.engine.is_terminal() {
                    if point_in_rect(mouse_pos, restart) {
                        Self::play_click(assets);
                        self.session = None;
                        self.screen = Screen::Settings;
                        self.screen_started_at = Instant::now();
                        self.stop_bgm(assets);
                    }
                    return;
                }
                if let Some(cell) = cell_at_point(
                    layout.grid_origin,
                    layout.cell_size,
                    session.engine.config.matrix_size,
                    mouse_pos,
                ) && session.engine.legal_moves().contains(&cell)
                    && session.apply_move(cell).is_ok()
                {
                    Self::play_click(assets);
                }
            }
        }
    }

    fn start_bgm(&mut self, assets: &Assets) {
        if self.bgm_playing {
            return;
        }
        play_sound(
            &assets.audio.bgm,
            PlaySoundParams {
                looped: true,
                volume: 0.55,
            },
        );
        self.bgm_playing = true;
    }

    fn stop_bgm(&mut self, assets: &Assets) {
        if self.bgm_playing {
            stop_sound(&assets.audio.bgm);
            self.bgm_playing = false;
        }
    }

    fn play_click(assets: &Assets) {
        play_sound(
            &assets.audio.click,
            PlaySoundParams {
                looped: false,
                volume: 0.78,
            },
        );
    }

    fn start_game(&mut self, assets: &Assets) {
        match build_engine(&self.settings, None) {
            Ok(engine) => {
                self.session = Some(Session::new(engine));
                self.screen = Screen::Playing;
                self.generation_error = None;
                self.screen_started_at = Instant::now();
                self.start_bgm(assets);
            }
            Err(err) => {
                self.session = None;
                self.screen = Screen::Settings;
                self.generation_error = Some(format!("Unable to generate puzzle: {err}"));
                self.stop_bgm(assets);
            }
        }
    }

    fn draw_settings(&self, assets: &Assets, mouse_pos: Vec2, width: f32, height: f32) {
        let layout = settings_layout(width, height);
        let hover = settings_hit(&self.settings, &layout, mouse_pos);
        let style = HudStyle::new(layout.scale);
        let colors = style.colors;
        let scale = layout.scale;
        let time = self.screen_started_at.elapsed().as_secs_f32();

        draw_panel_frame(layout.panel, &style, colors.panel);
        draw_text_ex(
            "BREACH PROTOCOL",
            layout.panel.x + 36.0 * scale,
            layout.panel.y + 58.0 * scale,
            TextParams {
                font: Some(&assets.font_display),
                font_size: (30.0 * scale).round() as u16,
                color: colors.accent,
                ..Default::default()
            },
        );
        draw_text_ex(
            "Verified access-point puzzle",
            layout.panel.x + 36.0 * scale,
            layout.panel.y + 88.0 * scale,
            TextParams {
                font: Some(&assets.font_medium),
                font_size: (17.0 * scale).round() as u16,
                color: colors.text_dim,
                ..Default::default()
            },
        );

        if let Some(error) = &self.generation_error {
            draw_rectangle(
                layout.error_rect.x,
                layout.error_rect.y,
                layout.error_rect.w,
                layout.error_rect.h,
                Color::from_rgba(255, 67, 80, 28),
            );
            draw_rectangle_lines(
                layout.error_rect.x,
                layout.error_rect.y,
                layout.error_rect.w,
                layout.error_rect.h,
                1.0,
                colors.red,
            );
            draw_text_ex(
                error,
                layout.error_rect.x + 10.0 * scale,
                layout.error_rect.y + layout.error_rect.h * 0.72,
                TextParams {
                    font: Some(&assets.font_medium),
                    font_size: (15.0 * scale).round() as u16,
                    color: colors.red,
                    ..Default::default()
                },
            );
        }

        for row in &layout.rows {
            draw_setting_row(&self.settings, row, assets, hover, scale, time);
        }
        draw_command_button(
            "START",
            layout.start_button,
            assets,
            hover == Some(SettingsHit::Start),
            scale,
            time,
        );
    }

    fn draw_game(
        &self,
        session: &Session,
        assets: &Assets,
        mouse_pos: Vec2,
        width: f32,
        height: f32,
    ) {
        let engine = &session.engine;
        let layout = game_layout(width, height, engine.config.matrix_size);
        let now = Instant::now();
        let time = now.duration_since(self.screen_started_at).as_secs_f32();
        let pick_age = session.last_pick_at.map_or(f32::INFINITY, |instant| {
            now.duration_since(instant).as_secs_f32()
        });
        let result_age = session.ended_at.map_or(f32::INFINITY, |instant| {
            now.duration_since(instant).as_secs_f32()
        });
        let hover_cell = if engine.is_terminal() {
            None
        } else {
            cell_at_point(
                layout.grid_origin,
                layout.cell_size,
                engine.config.matrix_size,
                mouse_pos,
            )
        };

        draw_top_rule(&layout);
        draw_timer_panel(
            engine.config.time_limit_seconds,
            session.remaining_time(),
            assets,
            &layout,
        );
        draw_buffer_panel(engine, assets, &layout, time, time, pick_age);
        let sequence_hover = draw_sequences_panel(engine, assets, &layout, mouse_pos, time);
        draw_matrix_panel(
            engine,
            assets,
            &layout,
            hover_cell,
            sequence_hover.as_deref(),
            pick_age,
        );

        if engine.is_terminal() {
            draw_game_over(engine, assets, &layout, result_age, time);
            draw_command_button(
                "NEW GAME",
                restart_button_rect(&layout),
                assets,
                point_in_rect(mouse_pos, restart_button_rect(&layout)),
                layout.scale,
                time,
            );
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen {
    Settings,
    Playing,
}

pub(super) struct Session {
    pub(super) engine: GameEngine,
    pub(super) started_at: Option<Instant>,
    pub(super) ended_at: Option<Instant>,
    pub(super) last_pick_at: Option<Instant>,
}

impl Session {
    pub(super) fn new(engine: GameEngine) -> Self {
        Self {
            engine,
            started_at: None,
            ended_at: None,
            last_pick_at: None,
        }
    }

    pub(super) fn apply_move(&mut self, cell: Cell) -> Result<(), GameError> {
        self.engine.apply_move(cell)?;
        self.last_pick_at = Some(Instant::now());
        if self.started_at.is_none() {
            self.started_at = Some(Instant::now());
        }
        self.capture_terminal_end();
        Ok(())
    }

    pub(super) fn tick_timer(&mut self) {
        if self.engine.config.time_limit_seconds == 0
            || self.started_at.is_none()
            || self.ended_at.is_some()
        {
            return;
        }
        if self.remaining_time().is_zero() {
            self.engine.force_timeout();
            self.capture_terminal_end();
        }
    }

    pub(super) fn remaining_time(&self) -> Duration {
        let total = self.engine.config.time_limit_seconds;
        if total == 0 {
            return Duration::MAX;
        }
        let Some(started_at) = self.started_at else {
            return Duration::from_secs(total);
        };
        let elapsed = self
            .ended_at
            .map_or_else(|| started_at.elapsed(), |ended_at| ended_at - started_at);
        Duration::from_secs(total).saturating_sub(elapsed)
    }

    #[cfg(test)]
    pub(super) fn has_ended(&self) -> bool {
        self.ended_at.is_some()
    }

    fn capture_terminal_end(&mut self) {
        if self.engine.is_terminal() && self.ended_at.is_none() {
            self.ended_at = Some(Instant::now());
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use rust_breach_protocol::engine::{BreachConfig, GameEngine, TerminalReason};

    use super::*;

    fn engine(
        buffer_size: usize,
        time_limit_seconds: u64,
        sequences: Vec<Vec<&str>>,
    ) -> GameEngine {
        let matrix = vec![
            vec!["AA".to_string(), "BB".to_string()],
            vec!["CC".to_string(), "DD".to_string()],
        ];
        let sequences = sequences
            .into_iter()
            .map(|sequence| sequence.into_iter().map(str::to_string).collect())
            .collect::<Vec<Vec<String>>>();
        let values = vec![100; sequences.len()];
        GameEngine::new(
            BreachConfig {
                matrix_size: 2,
                buffer_size,
                time_limit_seconds,
                token_alphabet: vec![
                    "AA".to_string(),
                    "BB".to_string(),
                    "CC".to_string(),
                    "DD".to_string(),
                ],
                seed: None,
            },
            matrix,
            sequences,
            values,
        )
        .unwrap()
    }

    #[test]
    fn session_timer_freezes_after_terminal() {
        let mut session = Session::new(engine(4, 30, vec![vec!["AA"]]));
        session.apply_move((0, 0)).unwrap();
        assert!(session.has_ended());
        let first = session.remaining_time();
        std::thread::sleep(Duration::from_millis(5));
        assert_eq!(session.remaining_time(), first);
    }

    #[test]
    fn session_timer_freezes_after_timeout() {
        let mut session = Session::new(engine(4, 1, vec![vec!["BB", "DD"]]));
        session.apply_move((0, 0)).unwrap();
        session.started_at = Some(Instant::now() - Duration::from_secs(2));
        session.tick_timer();
        assert_eq!(
            session.engine.terminal_reason(),
            Some(TerminalReason::OutOfTime)
        );
        assert!(session.has_ended());
    }

    #[test]
    fn terminal_capture_records_buffer_full() {
        let mut session = Session::new(engine(1, 30, vec![vec!["DD"]]));
        session.apply_move((0, 0)).unwrap();
        assert_eq!(
            session.engine.terminal_reason(),
            Some(TerminalReason::BufferFull)
        );
        assert!(session.has_ended());
    }
}
