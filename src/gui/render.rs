use macroquad::prelude::*;
use rust_breach_protocol::engine::{Cell, ConstraintMode, GameEngine, GameResult, TerminalReason};
use std::time::Duration;

use super::assets::Assets;
use super::layout::{
    Layout, SettingRow, SettingsHit, point_in_rect, result_panel_rect, ui_scale_for,
};
use super::settings::Settings;

#[derive(Clone, Copy)]
pub(super) struct Palette {
    pub(super) accent: Color,
    pub(super) accent_dim: Color,
    pub(super) cyan: Color,
    pub(super) amber: Color,
    pub(super) green: Color,
    pub(super) red: Color,
    pub(super) bg: Color,
    pub(super) panel: Color,
    pub(super) text_dim: Color,
    pub(super) text_light: Color,
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            accent: Color::from_rgba(218, 255, 88, 245),
            accent_dim: Color::from_rgba(218, 255, 88, 162),
            cyan: Color::from_rgba(31, 255, 246, 230),
            amber: Color::from_rgba(255, 176, 65, 235),
            green: Color::from_rgba(35, 225, 126, 235),
            red: Color::from_rgba(255, 67, 80, 235),
            bg: Color::from_rgba(5, 6, 9, 255),
            panel: Color::from_rgba(7, 8, 12, 186),
            text_dim: Color::from_rgba(178, 196, 126, 214),
            text_light: Color::from_rgba(225, 226, 218, 220),
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct HudStyle {
    pub(super) colors: Palette,
    pub(super) line: f32,
    pub(super) hairline: f32,
    pub(super) corner: f32,
}

impl HudStyle {
    pub(super) fn new(scale: f32) -> Self {
        Self {
            colors: Palette::default(),
            line: (2.35 * scale).clamp(2.35, 3.35),
            hairline: (2.0 * scale).clamp(2.0, 2.8),
            corner: 16.0 * scale,
        }
    }
}

pub(super) fn fade(color: Color, alpha: f32) -> Color {
    Color::new(color.r, color.g, color.b, alpha.clamp(0.0, 1.0))
}

pub(super) fn line_fade(color: Color, alpha: f32) -> Color {
    fade(color, alpha.max(0.66))
}

pub(super) fn trace_fade(color: Color, alpha: f32) -> Color {
    fade(color, alpha.max(0.24))
}

pub(super) fn pulse(age: f32, duration: f32) -> f32 {
    if age < 0.0 || age > duration {
        return 0.0;
    }
    let t = 1.0 - age / duration;
    (t * std::f32::consts::PI).sin().max(0.0) * t
}

// common
pub(crate) fn background_color() -> Color {
    Palette::default().bg
}

pub(in crate::gui) fn draw_background(width: f32, height: f32) {
    let colors = Palette::default();
    draw_rectangle(0.0, 0.0, width, height, colors.bg);

    let scale = ui_scale_for(width, height);
    let grid_gap = (56.0 * scale).max(28.0);
    let line = Color::from_rgba(218, 255, 88, 18);
    let mut x = (width * 0.04) % grid_gap;
    while x < width {
        draw_line(x.round(), 0.0, x.round(), height, 1.0, line);
        x += grid_gap;
    }
    let mut y = (height * 0.03) % grid_gap;
    while y < height {
        draw_line(0.0, y.round(), width, y.round(), 1.0, line);
        y += grid_gap;
    }

    draw_rectangle(0.0, 0.0, width, height, Color::from_rgba(0, 0, 0, 58));
    draw_rectangle(
        0.0,
        0.0,
        width,
        height * 0.18,
        Color::from_rgba(0, 0, 0, 56),
    );
}

pub(in crate::gui) fn draw_hud_lines(width: f32, height: f32) {
    let scale = ui_scale_for(width, height);
    let style = HudStyle::new(scale);
    let colors = style.colors;
    let margin = (width * 0.035).clamp(18.0 * scale, 64.0 * scale);
    let outer = Rect::new(margin, margin, width - margin * 2.0, height - margin * 2.0);
    draw_bracket_rect(
        outer,
        line_fade(colors.accent, 0.64),
        0.0,
        42.0 * scale,
        style.hairline,
    );
    let tick = 70.0 * scale;
    draw_line(
        outer.x + outer.w * 0.11,
        outer.y,
        outer.x + outer.w * 0.11 + tick,
        outer.y,
        style.hairline,
        line_fade(colors.accent, 0.50),
    );
    draw_line(
        outer.x + outer.w * 0.89 - tick,
        outer.y + outer.h,
        outer.x + outer.w * 0.89,
        outer.y + outer.h,
        style.hairline,
        line_fade(colors.accent, 0.50),
    );
}

pub(in crate::gui) fn draw_panel_frame(rect: Rect, style: &HudStyle, fill: Color) {
    let accent = style.colors.accent;
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, fill);
    draw_rectangle_lines(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        style.hairline,
        line_fade(accent, 0.68),
    );
    draw_bracket_rect(rect, line_fade(accent, 0.92), 0.0, style.corner, style.line);
    draw_line(
        rect.x,
        rect.y,
        rect.x + rect.w * 0.16,
        rect.y,
        style.line,
        accent,
    );
    draw_line(
        rect.x + rect.w * 0.82,
        rect.y + rect.h,
        rect.x + rect.w,
        rect.y + rect.h,
        style.line,
        line_fade(accent, 0.82),
    );
}

pub(in crate::gui) fn draw_command_button(
    label: &str,
    rect: Rect,
    assets: &Assets,
    hovered: bool,
    scale: f32,
    time: f32,
) {
    let colors = Palette::default();
    let style = HudStyle::new(scale);
    let fill = if hovered {
        fade(colors.cyan, 0.10 + pulse((time * 2.6) % 1.0, 1.0) * 0.10)
    } else {
        Color::from_rgba(8, 12, 14, 62)
    };
    let outline = if hovered { colors.cyan } else { colors.accent };
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, fill);
    draw_rectangle_lines(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        style.hairline,
        fade(outline, 0.78),
    );
    draw_line(
        rect.x,
        rect.y,
        rect.x + rect.w * 0.34,
        rect.y,
        style.line,
        outline,
    );
    if hovered {
        draw_bracket_rect(rect, colors.cyan, 4.0 * scale, 13.0 * scale, style.line);
    }
    centered_text(
        label,
        rect,
        &assets.font_bold,
        (20.0 * scale).round() as u16,
        if hovered { colors.cyan } else { colors.accent },
        0.68,
    );
}
pub(in crate::gui) fn draw_text_fit(
    label: &str,
    rect: Rect,
    font: &Font,
    max_size: f32,
    color: Color,
    y_ratio: f32,
) {
    if rect.w <= 4.0 || rect.h <= 4.0 {
        return;
    }
    let mut size = max_size.round().max(8.0) as u16;
    while size > 8 && measure_text(label, Some(font), size, 1.0).width > rect.w {
        size -= 1;
    }
    if measure_text(label, Some(font), size, 1.0).width > rect.w {
        return;
    }
    draw_text_ex(
        label,
        rect.x,
        rect.y + rect.h * y_ratio,
        TextParams {
            font: Some(font),
            font_size: size,
            color,
            ..Default::default()
        },
    );
}
pub(in crate::gui) fn draw_panel_header(
    header: Rect,
    label: &str,
    icon: &Texture2D,
    assets: &Assets,
    scale: f32,
) {
    let colors = Palette::default();
    let style = HudStyle::new(scale);
    draw_rectangle(
        header.x,
        header.y,
        header.w,
        header.h,
        Color::from_rgba(5, 9, 12, 118),
    );
    draw_line(
        header.x,
        header.y + header.h,
        header.x + header.w,
        header.y + header.h,
        style.hairline,
        line_fade(colors.accent, 0.62),
    );
    draw_line(
        header.x,
        header.y,
        header.x + header.w * 0.22,
        header.y,
        style.line,
        line_fade(colors.accent, 0.96),
    );
    draw_texture_ex(
        icon,
        header.x + 14.0 * scale,
        header.y + 10.0 * scale,
        fade(colors.accent, 0.94),
        DrawTextureParams {
            dest_size: Some(vec2(21.0 * scale, 21.0 * scale)),
            ..Default::default()
        },
    );
    draw_text_ex(
        label,
        header.x + 46.0 * scale,
        header.y + 28.0 * scale,
        TextParams {
            font: Some(&assets.font_medium),
            font_size: (22.0 * scale).round() as u16,
            color: colors.accent,
            ..Default::default()
        },
    );
}

pub(in crate::gui) fn draw_bracket_rect(
    rect: Rect,
    color: Color,
    inset: f32,
    len: f32,
    thickness: f32,
) {
    let color = if color.a < 0.55 {
        Color::new(color.r, color.g, color.b, 0.66)
    } else {
        color
    };
    let thickness = thickness.max(2.0);
    let x0 = (rect.x + inset).round();
    let y0 = (rect.y + inset).round();
    let x1 = (rect.x + rect.w - inset).round();
    let y1 = (rect.y + rect.h - inset).round();
    draw_line(x0, y0, x0 + len, y0, thickness, color);
    draw_line(x0, y0, x0, y0 + len, thickness, color);
    draw_line(x1, y0, x1 - len, y0, thickness, color);
    draw_line(x1, y0, x1, y0 + len, thickness, color);
    draw_line(x0, y1, x0 + len, y1, thickness, color);
    draw_line(x0, y1, x0, y1 - len, thickness, color);
    draw_line(x1, y1, x1 - len, y1, thickness, color);
    draw_line(x1, y1, x1, y1 - len, thickness, color);
}

pub(in crate::gui) fn draw_aligned_rect_lines(rect: Rect, thickness: f32, color: Color) {
    let color = if color.a < 0.55 {
        Color::new(color.r, color.g, color.b, 0.66)
    } else {
        color
    };
    draw_rectangle_lines(
        rect.x.round(),
        rect.y.round(),
        rect.w.round(),
        rect.h.round(),
        thickness.max(2.0),
        color,
    );
}

pub(in crate::gui) fn draw_glow_rect(rect: Rect, color: Color) {
    draw_rectangle(
        rect.x - 7.0,
        rect.y - 7.0,
        rect.w + 14.0,
        rect.h + 14.0,
        Color::new(color.r, color.g, color.b, 0.10),
    );
    draw_rectangle(
        rect.x - 3.0,
        rect.y - 3.0,
        rect.w + 6.0,
        rect.h + 6.0,
        Color::new(color.r, color.g, color.b, 0.07),
    );
}

pub(in crate::gui) fn centered_text(
    label: &str,
    rect: Rect,
    font: &Font,
    font_size: u16,
    color: Color,
    y_ratio: f32,
) {
    let dims = measure_text(label, Some(font), font_size, 1.0);
    draw_text_ex(
        label,
        rect.x + (rect.w - dims.width) * 0.5,
        rect.y + rect.h * y_ratio,
        TextParams {
            font: Some(font),
            font_size,
            color,
            ..Default::default()
        },
    );
}

// settings
pub(in crate::gui) fn draw_setting_row(
    settings: &Settings,
    row: &SettingRow,
    assets: &Assets,
    hover: Option<SettingsHit>,
    scale: f32,
    time: f32,
) {
    let colors = Palette::default();
    let style = HudStyle::new(scale);
    draw_rectangle(
        row.row.x - 12.0 * scale,
        row.row.y - 2.0 * scale,
        row.row.w + 24.0 * scale,
        row.row.h + 4.0 * scale,
        Color::from_rgba(8, 12, 14, 46),
    );
    draw_line(
        row.row.x,
        row.row.y + row.row.h + 4.0,
        row.row.x + row.row.w,
        row.row.y + row.row.h + 4.0,
        style.hairline,
        colors.accent_dim,
    );
    draw_text_ex(
        row.field.label(),
        row.row.x,
        row.row.y + row.row.h * 0.72,
        TextParams {
            font: Some(&assets.font_medium),
            font_size: (21.0 * scale).round() as u16,
            color: colors.accent,
            ..Default::default()
        },
    );
    draw_rectangle(
        row.value_box.x,
        row.value_box.y,
        row.value_box.w,
        row.value_box.h,
        Color::from_rgba(7, 9, 13, 38),
    );
    draw_rectangle_lines(
        row.value_box.x,
        row.value_box.y,
        row.value_box.w,
        row.value_box.h,
        style.hairline,
        colors.accent_dim,
    );
    draw_text_ex(
        &settings.value_label(row.field),
        row.value_box.x + 12.0 * scale,
        row.value_box.y + row.value_box.h * 0.73,
        TextParams {
            font: Some(&assets.font_medium),
            font_size: (21.0 * scale).round() as u16,
            color: colors.accent,
            ..Default::default()
        },
    );
    draw_step_button(
        row.minus_button,
        "-",
        settings.can_step(row.field, -1),
        hover == Some(SettingsHit::Step(row.field, -1)),
        assets,
        scale,
        time,
    );
    draw_step_button(
        row.plus_button,
        "+",
        settings.can_step(row.field, 1),
        hover == Some(SettingsHit::Step(row.field, 1)),
        assets,
        scale,
        time,
    );
}

fn draw_step_button(
    rect: Rect,
    label: &str,
    enabled: bool,
    hovered: bool,
    assets: &Assets,
    scale: f32,
    time: f32,
) {
    let colors = Palette::default();
    let style = HudStyle::new(scale);
    let hovered = enabled && hovered;
    let hover_alpha = if hovered {
        0.09 + pulse((time * 2.0) % 1.0, 1.0) * 0.10
    } else {
        0.03
    };
    let fill = fade(
        if hovered { colors.cyan } else { colors.accent },
        hover_alpha,
    );
    let outline = if enabled {
        colors.accent_dim
    } else {
        Color::from_rgba(90, 100, 70, 105)
    };
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, fill);
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, style.hairline, outline);
    if hovered {
        draw_bracket_rect(rect, colors.cyan, 3.0 * scale, 9.0 * scale, style.line);
    }
    let text_color = if enabled {
        colors.accent
    } else {
        Color::from_rgba(90, 100, 70, 150)
    };
    centered_text(
        label,
        rect,
        &assets.font_bold,
        (22.0 * scale).round() as u16,
        text_color,
        0.70,
    );
}

// chrome
pub(in crate::gui) fn draw_top_rule(layout: &Layout) {
    let colors = Palette::default();
    let scale = layout.scale;
    let style = HudStyle::new(scale);
    let y = layout.timer_panel.y - 2.0 * scale;
    let h = (layout.timer_panel.h * 0.92).clamp(70.0 * scale, 126.0 * scale);
    let rect = Rect::new(
        layout.timer_panel.x,
        y,
        layout.sequence_panel.x + layout.sequence_panel.w - layout.timer_panel.x,
        h,
    );
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        Color::from_rgba(5, 8, 11, 88),
    );
    draw_line(
        rect.x,
        rect.y,
        rect.x + rect.w,
        rect.y,
        style.hairline,
        line_fade(colors.accent, 0.62),
    );
    draw_line(
        rect.x,
        rect.y + rect.h,
        rect.x + rect.w,
        rect.y + rect.h,
        style.hairline,
        line_fade(colors.accent, 0.58),
    );
}

pub(in crate::gui) fn draw_timer_panel(
    time_limit_seconds: u64,
    remaining_time: Duration,
    assets: &Assets,
    layout: &Layout,
) {
    let colors = Palette::default();
    let rect = layout.timer_panel;
    let scale = layout.scale;
    let style = HudStyle::new(scale);
    let (label, ratio, has_limit) = timer_snapshot(time_limit_seconds, remaining_time);
    let timer_color = timer_color(ratio, has_limit, colors);
    draw_text_fit(
        "BREACH TIME REMAINING",
        Rect::new(rect.x, rect.y + 8.0 * scale, rect.w, 20.0 * scale),
        &assets.font_medium,
        19.0 * scale,
        fade(colors.text_dim, 0.92),
        0.82,
    );
    let box_w = (118.0 * scale).clamp(92.0, 136.0).min(rect.w);
    let box_rect = Rect::new(rect.x, rect.y + 32.0 * scale, box_w, 34.0 * scale);
    draw_rectangle(
        box_rect.x,
        box_rect.y,
        box_rect.w,
        box_rect.h,
        fade(timer_color, 0.045),
    );
    draw_rectangle_lines(
        box_rect.x,
        box_rect.y,
        box_rect.w,
        box_rect.h,
        style.hairline,
        line_fade(timer_color, 0.88),
    );
    centered_text(
        &label,
        box_rect,
        &assets.font_bold,
        if has_limit {
            (25.0 * scale).round() as u16
        } else {
            (17.0 * scale).round() as u16
        },
        fade(timer_color, 0.96),
        0.72,
    );
    let bar = Rect::new(rect.x, rect.y + 74.0 * scale, rect.w, 6.0 * scale);
    draw_rectangle_lines(
        bar.x,
        bar.y,
        bar.w,
        bar.h,
        style.hairline,
        line_fade(timer_color, 0.62),
    );
    draw_rectangle(bar.x, bar.y, bar.w * ratio, bar.h, fade(timer_color, 0.88));
}

fn timer_snapshot(time_limit_seconds: u64, remaining_time: Duration) -> (String, f32, bool) {
    if time_limit_seconds == 0 {
        return ("NO LIMIT".to_string(), 1.0, false);
    }
    let remaining = remaining_time.as_secs_f32();
    (
        format!("{remaining:.2}"),
        (remaining / time_limit_seconds as f32).clamp(0.0, 1.0),
        true,
    )
}

pub(in crate::gui) fn draw_buffer_panel(
    engine: &GameEngine,
    assets: &Assets,
    layout: &Layout,
    time: f32,
    screen_age: f32,
    pick_age: f32,
) {
    let colors = Palette::default();
    let rect = layout.buffer_panel;
    let scale = layout.scale;
    let style = HudStyle::new(scale);
    draw_text_ex(
        "BUFFER",
        rect.x,
        rect.y + 24.0 * scale,
        TextParams {
            font: Some(&assets.font_medium),
            font_size: (23.0 * scale).round() as u16,
            color: fade(colors.text_dim, 0.94),
            ..Default::default()
        },
    );
    let buffer = buffer_geometry(engine, layout);
    draw_rectangle(
        buffer.start_x - 10.0 * scale,
        buffer.y - 10.0 * scale,
        buffer.total_w + 20.0 * scale,
        buffer.slot_size + 20.0 * scale,
        Color::from_rgba(6, 10, 12, 76),
    );
    draw_rectangle_lines(
        buffer.start_x - 10.0 * scale,
        buffer.y - 10.0 * scale,
        buffer.total_w + 20.0 * scale,
        buffer.slot_size + 20.0 * scale,
        style.hairline,
        line_fade(colors.amber, 0.60),
    );

    for idx in 0..engine.config.buffer_size {
        let slot = buffer.slot_rect(idx);
        let boot = pulse(screen_age - idx as f32 * 0.035, 0.42);
        draw_rectangle(
            slot.x,
            slot.y,
            slot.w,
            slot.h,
            fade(colors.amber, 0.018 + boot * 0.05),
        );
        draw_rectangle_lines(
            slot.x,
            slot.y,
            slot.w,
            slot.h,
            style.hairline,
            line_fade(colors.amber, 0.54 + boot * 0.20),
        );
        let Some(token) = engine.state.buffer_tokens.get(idx) else {
            continue;
        };
        let current = idx + 1 == engine.state.buffer_tokens.len();
        if current {
            let snap = pulse(pick_age, 0.42);
            draw_glow_rect(slot, colors.cyan);
            draw_rectangle_lines(
                slot.x - (2.0 + snap * 5.0) * scale,
                slot.y - (2.0 + snap * 5.0) * scale,
                slot.w + (4.0 + snap * 10.0) * scale,
                slot.h + (4.0 + snap * 10.0) * scale,
                style.line,
                fade(colors.cyan, 0.75 + snap * 0.25),
            );
        }
        centered_text(
            token,
            slot,
            &assets.font_bold,
            (buffer.slot_size * 0.62).round() as u16,
            if current {
                fade(colors.cyan, 0.92 + pulse(time % 1.2, 1.2) * 0.08)
            } else {
                colors.amber
            },
            0.72,
        );
    }
}

fn timer_color(ratio: f32, has_limit: bool, colors: Palette) -> Color {
    if !has_limit {
        colors.green
    } else if ratio <= 0.18 {
        colors.red
    } else if ratio <= 0.38 {
        colors.amber
    } else {
        colors.green
    }
}

#[derive(Clone, Copy, Debug)]
pub(in crate::gui) struct BufferGeometry {
    pub(in crate::gui) start_x: f32,
    y: f32,
    pub(in crate::gui) slot_size: f32,
    gap: f32,
    pub(in crate::gui) total_w: f32,
}

impl BufferGeometry {
    pub(in crate::gui) fn slot_rect(self, idx: usize) -> Rect {
        Rect::new(
            self.start_x + idx as f32 * (self.slot_size + self.gap),
            self.y,
            self.slot_size,
            self.slot_size,
        )
    }
}

pub(in crate::gui) fn buffer_geometry(engine: &GameEngine, layout: &Layout) -> BufferGeometry {
    let rect = layout.buffer_panel;
    let scale = layout.scale;
    let gap = if engine.config.buffer_size > 10 {
        (5.0 * scale).max(3.0)
    } else {
        (8.0 * scale).max(4.0)
    };
    let lane_w = layout.token_lane.w;
    let slot_size = ((lane_w - gap * engine.config.buffer_size.saturating_sub(1) as f32)
        / engine.config.buffer_size as f32)
        .clamp(12.0, 34.0 * scale);
    let total_w = engine.config.buffer_size as f32 * slot_size
        + engine.config.buffer_size.saturating_sub(1) as f32 * gap;
    BufferGeometry {
        start_x: layout.token_lane.x,
        y: rect.y + 42.0 * scale,
        slot_size,
        gap,
        total_w,
    }
}

// sequences
pub(in crate::gui) fn draw_sequences_panel(
    engine: &GameEngine,
    assets: &Assets,
    layout: &Layout,
    mouse_pos: Vec2,
    time: f32,
) -> Option<String> {
    let rect = layout.sequence_panel;
    let scale = layout.scale;
    let style = HudStyle::new(scale);
    let mut hovered_token = None;
    draw_panel_frame(rect, &style, Color::from_rgba(7, 10, 13, 128));
    let header = Rect::new(rect.x, rect.y, rect.w, 42.0 * scale);
    draw_panel_header(
        header,
        "SEQUENCE REQUIRED TO UPLOAD",
        &assets.icon_sequence,
        assets,
        scale,
    );
    let count = engine.state.sequences.len().max(1) as f32;
    let available_h = (rect.h - header.h - 24.0 * scale).max(100.0 * scale);
    let row_gap = (8.0 * scale).max(4.0);
    let row_h = ((available_h - row_gap * (count - 1.0)) / count).clamp(52.0 * scale, 72.0 * scale);
    let mut y = header.y + header.h + 12.0 * scale;
    let mut ctx = SequenceDrawCtx {
        assets,
        layout,
        mouse_pos,
        hovered_token: &mut hovered_token,
        scale,
        time,
    };
    for idx in 0..engine.state.sequences.len() {
        let row = Rect::new(rect.x + 14.0 * scale, y, rect.w - 28.0 * scale, row_h);
        draw_sequence_row(engine, idx, row, &mut ctx);
        y += row_h + row_gap;
    }
    hovered_token
}

struct SequenceDrawCtx<'a> {
    assets: &'a Assets,
    layout: &'a Layout,
    mouse_pos: Vec2,
    hovered_token: &'a mut Option<String>,
    scale: f32,
    time: f32,
}

fn draw_sequence_row(engine: &GameEngine, idx: usize, row: Rect, ctx: &mut SequenceDrawCtx) {
    let colors = Palette::default();
    let assets = ctx.assets;
    let scale = ctx.scale;
    let time = ctx.time;
    let style = HudStyle::new(scale);
    let sequence = &engine.state.sequences[idx];
    let progress = engine.sequence_progress(idx);
    let remaining_buffer = engine
        .config
        .buffer_size
        .saturating_sub(engine.state.buffer_tokens.len());
    let status = sequence_status(engine, idx, progress, remaining_buffer);

    let status_color = match status {
        SequenceStatus::Installed => colors.green,
        SequenceStatus::Failed => colors.red,
        SequenceStatus::Pending => colors.accent,
    };
    let row_fill = match status {
        SequenceStatus::Installed | SequenceStatus::Failed if engine.is_terminal() => {
            fade(status_color, 0.16)
        }
        SequenceStatus::Installed | SequenceStatus::Failed => fade(status_color, 0.10),
        SequenceStatus::Pending => Color::from_rgba(8, 12, 15, 78),
    };
    draw_rectangle(row.x, row.y, row.w, row.h, row_fill);
    draw_rectangle_lines(
        row.x,
        row.y,
        row.w,
        row.h,
        style.hairline,
        if status == SequenceStatus::Pending {
            line_fade(colors.accent, 0.56)
        } else {
            line_fade(status_color, 0.76)
        },
    );
    draw_rectangle(
        row.x,
        row.y,
        4.0 * scale,
        row.h,
        fade(
            status_color,
            if status == SequenceStatus::Pending {
                0.42
            } else {
                0.78
            },
        ),
    );
    if status == SequenceStatus::Installed {
        let sweep = ((time * 0.68 + idx as f32 * 0.17) % 1.0) * row.w;
        draw_rectangle(
            row.x + sweep,
            row.y,
            18.0 * scale,
            row.h,
            fade(colors.green, 0.08),
        );
    }
    if status == SequenceStatus::Pending {
        draw_sequence_tokens(engine, sequence, progress, row, ctx);
    } else {
        draw_text_ex(
            if status == SequenceStatus::Installed {
                "INSTALLED"
            } else {
                "FAILED"
            },
            row.x + 18.0 * scale,
            row.y + row.h * 0.62,
            TextParams {
                font: Some(&assets.font_bold),
                font_size: (20.0 * scale).round() as u16,
                color: if status == SequenceStatus::Installed {
                    fade(colors.green, 0.92)
                } else {
                    fade(colors.red, 0.92)
                },
                ..Default::default()
            },
        );
    }

    let value_label = format!("€${}", engine.state.sequence_values[idx]);
    draw_text_fit(
        &value_label,
        Rect::new(
            row.x + row.w - 92.0 * scale,
            row.y + row.h * 0.28,
            74.0 * scale,
            row.h * 0.38,
        ),
        &assets.font_bold,
        18.0 * scale,
        if status == SequenceStatus::Pending {
            fade(colors.text_light, 0.66)
        } else {
            fade(status_color, 0.76)
        },
        0.78,
    );
}

fn draw_sequence_tokens(
    engine: &GameEngine,
    sequence: &[String],
    progress: usize,
    row: Rect,
    ctx: &mut SequenceDrawCtx,
) {
    let colors = Palette::default();
    let assets = ctx.assets;
    let mouse_pos = ctx.mouse_pos;
    let scale = ctx.scale;
    let style = HudStyle::new(scale);
    let buffer = buffer_geometry(engine, ctx.layout);
    let aligned_start = engine.state.buffer_tokens.len().saturating_sub(progress);
    let y = row.y + row.h * 0.64;
    for (idx, token) in sequence.iter().enumerate() {
        let token_done = idx < progress;
        let slot_idx = aligned_start + idx;
        if slot_idx >= engine.config.buffer_size {
            continue;
        }
        let slot = buffer.slot_rect(slot_idx);
        let font_size = (buffer.slot_size * 0.78).clamp(12.0, 24.0 * scale).round() as u16;
        let dims = measure_text(token, Some(&assets.font_bold), font_size, 1.0);
        let token_baseline_y = y;
        let token_rect = Rect::new(
            slot.x + (slot.w - dims.width) * 0.5 - 4.0 * scale,
            token_baseline_y - dims.offset_y - 3.0 * scale,
            dims.width + 8.0 * scale,
            dims.height + 6.0 * scale,
        );
        if idx == progress && !token_done {
            let underline_y = token_rect.y + token_rect.h + 2.0 * scale;
            draw_line(
                token_rect.x,
                underline_y,
                token_rect.x + token_rect.w,
                underline_y,
                style.hairline,
                line_fade(colors.cyan, 0.60),
            );
            draw_line(
                token_rect.x,
                underline_y + 4.0 * scale,
                token_rect.x + token_rect.w * 0.64,
                underline_y + 4.0 * scale,
                style.hairline,
                trace_fade(colors.cyan, 0.30),
            );
        }
        if token_done {
            draw_bracket_rect(
                token_rect,
                fade(colors.amber, 0.58),
                1.0 * scale,
                6.0 * scale,
                1.0,
            );
        }
        if point_in_rect(mouse_pos, ctx.layout.sequence_panel)
            && point_in_rect(mouse_pos, token_rect)
        {
            *ctx.hovered_token = Some(token.clone());
            draw_rectangle(
                token_rect.x,
                token_rect.y,
                token_rect.w,
                token_rect.h,
                fade(colors.cyan, 0.045),
            );
            draw_rectangle_lines(
                token_rect.x - 3.0 * scale,
                token_rect.y - 3.0 * scale,
                token_rect.w + 6.0 * scale,
                token_rect.h + 6.0 * scale,
                style.hairline,
                colors.cyan,
            );
        }
        draw_text_ex(
            token,
            slot.x + (slot.w - dims.width) * 0.5,
            y,
            TextParams {
                font: Some(&assets.font_bold),
                font_size,
                color: if token_done {
                    colors.amber
                } else {
                    colors.text_light
                },
                ..Default::default()
            },
        );
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SequenceStatus {
    Pending,
    Installed,
    Failed,
}

fn sequence_status(
    engine: &GameEngine,
    idx: usize,
    progress: usize,
    remaining_buffer: usize,
) -> SequenceStatus {
    if engine.state.uploaded_results[idx] {
        return SequenceStatus::Installed;
    }
    let remaining_tokens = engine.state.sequences[idx].len().saturating_sub(progress);
    if engine.is_terminal()
        || remaining_tokens > remaining_buffer
        || !engine.can_sequence_still_upload(idx)
    {
        SequenceStatus::Failed
    } else {
        SequenceStatus::Pending
    }
}
// matrix
pub(in crate::gui) fn draw_matrix_panel(
    engine: &GameEngine,
    assets: &Assets,
    layout: &Layout,
    hover_cell: Option<Cell>,
    hovered_sequence_token: Option<&str>,
    pick_age: f32,
) {
    let style = HudStyle::new(layout.scale);
    draw_panel_frame(
        layout.matrix_panel,
        &style,
        Color::from_rgba(7, 10, 13, 134),
    );
    draw_panel_header(
        Rect::new(
            layout.matrix_panel.x,
            layout.matrix_panel.y,
            layout.matrix_panel.w,
            42.0 * layout.scale,
        ),
        "CODE MATRIX",
        &assets.icon_matrix,
        assets,
        layout.scale,
    );
    draw_constraint_overlay(engine, layout, hover_cell);
    draw_matrix_cells(
        engine,
        assets,
        layout,
        hover_cell,
        hovered_sequence_token,
        pick_age,
    );
}

fn draw_matrix_cells(
    engine: &GameEngine,
    assets: &Assets,
    layout: &Layout,
    hover_cell: Option<Cell>,
    hovered_sequence_token: Option<&str>,
    pick_age: f32,
) {
    let colors = Palette::default();
    let legal = engine.legal_moves();
    let grid_w = layout.cell_size * engine.config.matrix_size as f32;
    let style = HudStyle::new(layout.scale);
    for row in 0..engine.config.matrix_size {
        for col in 0..engine.config.matrix_size {
            let cell = (row, col);
            let rect = cell_rect(cell, layout);
            let selected = engine.state.selected_cells.contains(&cell);
            let token = &engine.state.matrix[row][col];
            let font_size = (layout.cell_size * 0.44).round() as u16;
            if selected {
                draw_cell_corners(
                    rect,
                    line_fade(colors.amber, 0.62),
                    layout.scale,
                    style.line,
                    CellCornerWeight::Resting,
                );
            }
            if let Some(token_match) = hovered_sequence_token
                && !selected
                && token == token_match
            {
                draw_token_corners(
                    token,
                    rect,
                    &assets.font_bold,
                    font_size,
                    line_fade(colors.cyan, 0.78),
                    layout.scale,
                    style.line,
                );
            }
            if hover_cell == Some(cell) {
                if legal.contains(&cell) {
                    let focus = cell_inner_rect(rect, layout.scale, CellCornerWeight::Focus);
                    draw_glow_rect(focus, colors.cyan);
                    draw_rectangle_lines(
                        focus.x,
                        focus.y,
                        focus.w,
                        focus.h,
                        style.line,
                        colors.cyan,
                    );
                    draw_cell_corners(
                        rect,
                        Color::from_rgba(31, 255, 246, 160),
                        layout.scale,
                        style.line,
                        CellCornerWeight::Focus,
                    );
                } else if !selected {
                    let warn = cell_inner_rect(rect, layout.scale, CellCornerWeight::Resting);
                    draw_rectangle_lines(
                        warn.x,
                        warn.y,
                        warn.w,
                        warn.h,
                        style.line,
                        line_fade(colors.red, 0.78),
                    );
                }
            }
            if Some(cell) == engine.state.current_index {
                let p = pulse(pick_age, 0.50);
                if p > 0.0 {
                    let pulse_rect = cell_inner_rect(rect, layout.scale, CellCornerWeight::Focus);
                    let grow = 7.0 * p * layout.scale;
                    draw_aligned_rect_lines(
                        Rect::new(
                            pulse_rect.x - grow,
                            pulse_rect.y - grow,
                            pulse_rect.w + grow * 2.0,
                            pulse_rect.h + grow * 2.0,
                        ),
                        style.line,
                        fade(colors.cyan, p * 0.70),
                    );
                }
            }
            let token_color = if selected {
                fade(colors.amber, 0.42)
            } else if !engine.state.buffer_tokens.is_empty() && !legal.contains(&cell) {
                Color::from_rgba(218, 255, 88, 128)
            } else {
                colors.accent
            };
            centered_text(token, rect, &assets.font_bold, font_size, token_color, 0.58);
        }
    }
    draw_rectangle_lines(
        layout.grid_origin.x,
        layout.grid_origin.y,
        grid_w,
        grid_w,
        HudStyle::new(layout.scale).hairline,
        Color::from_rgba(218, 255, 88, 118),
    );
}

fn draw_constraint_overlay(engine: &GameEngine, layout: &Layout, hover_cell: Option<Cell>) {
    let colors = Palette::default();
    let style = HudStyle::new(layout.scale);
    let active = Color::from_rgba(218, 255, 88, 24);
    let hover_overlay = Color::from_rgba(31, 255, 246, 22);
    let grid_w = layout.cell_size * engine.config.matrix_size as f32;

    if engine.state.buffer_tokens.is_empty() {
        draw_rectangle(
            layout.grid_origin.x,
            layout.grid_origin.y,
            grid_w,
            layout.cell_size,
            active,
        );
        draw_line(
            layout.grid_origin.x,
            layout.grid_origin.y + layout.cell_size,
            layout.grid_origin.x + grid_w,
            layout.grid_origin.y + layout.cell_size,
            style.hairline,
            colors.accent_dim,
        );
    } else if let Some((row, col)) = engine.state.current_index {
        let row_y = layout.grid_origin.y + row as f32 * layout.cell_size;
        let col_x = layout.grid_origin.x + col as f32 * layout.cell_size;
        match engine.state.current_constraint {
            ConstraintMode::Row => {
                draw_rectangle(
                    layout.grid_origin.x,
                    row_y,
                    grid_w,
                    layout.cell_size,
                    active,
                );
                draw_line(
                    layout.grid_origin.x,
                    row_y,
                    layout.grid_origin.x + grid_w,
                    row_y,
                    style.hairline,
                    colors.accent_dim,
                );
                draw_line(
                    layout.grid_origin.x,
                    row_y + layout.cell_size,
                    layout.grid_origin.x + grid_w,
                    row_y + layout.cell_size,
                    style.hairline,
                    colors.accent_dim,
                );
            }
            ConstraintMode::Column => {
                draw_rectangle(
                    col_x,
                    layout.grid_origin.y,
                    layout.cell_size,
                    grid_w,
                    active,
                );
                draw_line(
                    col_x,
                    layout.grid_origin.y,
                    col_x,
                    layout.grid_origin.y + grid_w,
                    style.hairline,
                    colors.accent_dim,
                );
                draw_line(
                    col_x + layout.cell_size,
                    layout.grid_origin.y,
                    col_x + layout.cell_size,
                    layout.grid_origin.y + grid_w,
                    style.hairline,
                    colors.accent_dim,
                );
            }
        }
    }

    if let Some((row, col)) = hover_cell
        && engine.legal_moves().contains(&(row, col))
    {
        let row_y = layout.grid_origin.y + row as f32 * layout.cell_size;
        let col_x = layout.grid_origin.x + col as f32 * layout.cell_size;
        draw_rectangle(
            layout.grid_origin.x,
            row_y,
            grid_w,
            layout.cell_size,
            hover_overlay,
        );
        draw_rectangle(
            col_x,
            layout.grid_origin.y,
            layout.cell_size,
            grid_w,
            hover_overlay,
        );
    }
}
enum CellCornerWeight {
    Resting,
    Focus,
}

fn cell_corner_metrics(rect: Rect, scale: f32, weight: CellCornerWeight) -> (f32, f32) {
    match weight {
        CellCornerWeight::Resting => (
            (rect.w * 0.17).clamp(6.0 * scale, 10.0 * scale),
            (rect.w * 0.24).clamp(9.0 * scale, 15.0 * scale),
        ),
        CellCornerWeight::Focus => (
            (rect.w * 0.10).clamp(4.0 * scale, 7.0 * scale),
            (rect.w * 0.30).clamp(12.0 * scale, 18.0 * scale),
        ),
    }
}

fn cell_inner_rect(rect: Rect, scale: f32, weight: CellCornerWeight) -> Rect {
    let (inset, _) = cell_corner_metrics(rect, scale, weight);
    Rect::new(
        (rect.x + inset).round(),
        (rect.y + inset).round(),
        (rect.w - inset * 2.0).round(),
        (rect.h - inset * 2.0).round(),
    )
}

fn draw_cell_corners(
    rect: Rect,
    color: Color,
    scale: f32,
    thickness: f32,
    weight: CellCornerWeight,
) {
    let (inset, len) = cell_corner_metrics(rect, scale, weight);
    draw_bracket_rect(rect, color, inset, len.round(), thickness);
}

fn draw_token_corners(
    token: &str,
    cell: Rect,
    font: &Font,
    font_size: u16,
    color: Color,
    scale: f32,
    thickness: f32,
) {
    let dims = measure_text(token, Some(font), font_size, 1.0);
    let baseline_y = cell.y + cell.h * 0.58;
    let pad_x = 8.0 * scale;
    let pad_y = 7.0 * scale;
    let token_rect = Rect::new(
        (cell.x + (cell.w - dims.width) * 0.5 - pad_x).round(),
        (baseline_y - dims.offset_y - pad_y).round(),
        (dims.width + pad_x * 2.0).round(),
        (dims.height + pad_y * 2.0).round(),
    );
    draw_bracket_rect(
        token_rect,
        color,
        0.0,
        (10.0 * scale).clamp(8.0, 14.0),
        thickness,
    );
}
fn cell_rect(cell: Cell, layout: &Layout) -> Rect {
    Rect::new(
        layout.grid_origin.x + cell.1 as f32 * layout.cell_size,
        layout.grid_origin.y + cell.0 as f32 * layout.cell_size,
        layout.cell_size,
        layout.cell_size,
    )
}

// result
pub(in crate::gui) fn draw_game_over(
    engine: &GameEngine,
    assets: &Assets,
    layout: &Layout,
    result_age: f32,
    time: f32,
) {
    let result = engine.get_game_result();
    let summary = terminal_summary(&result, engine.config.buffer_size);
    let colors = Palette::default();
    let scale = layout.scale;
    let status = match summary.tone {
        TerminalTone::Success | TerminalTone::Partial => colors.green,
        TerminalTone::Failure => colors.red,
    };
    let style = HudStyle::new(scale);
    let flash = pulse(result_age, 0.82);
    let reveal = (result_age / 0.65).clamp(0.0, 1.0);
    let footer_reveal = ((result_age - 0.38) / 0.34).clamp(0.0, 1.0);
    let panel = result_panel_rect(layout);
    let sweep_x = panel.x + ((time * 260.0) % panel.w.max(1.0));
    let body = Rect::new(
        panel.x,
        panel.y + 44.0 * scale,
        panel.w,
        panel.h - 88.0 * scale,
    );
    let footer = Rect::new(
        panel.x,
        panel.y + panel.h - 46.0 * scale,
        panel.w,
        46.0 * scale,
    );
    draw_rectangle(
        panel.x,
        panel.y,
        panel.w,
        panel.h * reveal,
        Color::from_rgba(4, 7, 9, 224),
    );
    draw_rectangle(
        body.x,
        body.y,
        body.w,
        body.h * reveal,
        Color::from_rgba(3, 6, 8, 232),
    );
    draw_rectangle(
        footer.x,
        footer.y,
        footer.w,
        footer.h * footer_reveal,
        fade(status, 0.12 + flash * 0.05),
    );
    draw_rectangle(
        panel.x,
        panel.y,
        panel.w,
        44.0 * scale * reveal,
        fade(status, 0.14 + flash * 0.06),
    );
    draw_rectangle(
        panel.x,
        panel.y,
        7.0 * scale,
        panel.h * reveal,
        fade(status, 0.96),
    );
    draw_rectangle(
        sweep_x,
        panel.y,
        18.0 * scale,
        panel.h * reveal,
        fade(colors.text_light, 0.12 + flash * 0.08),
    );
    draw_rectangle_lines(
        panel.x,
        panel.y,
        panel.w,
        panel.h,
        style.line,
        line_fade(status, 0.88),
    );
    draw_bracket_rect(
        panel,
        line_fade(status, 0.86),
        4.0 * scale,
        20.0 * scale,
        style.line,
    );
    let glitch_offset = if flash > 0.0 {
        ((time * 53.0).sin() * 2.0 * flash).round()
    } else {
        0.0
    };
    if reveal > 0.18 {
        draw_text_ex(
            summary.title,
            panel.x + 22.0 * scale + glitch_offset,
            panel.y + 29.0 * scale,
            TextParams {
                font: Some(&assets.font_bold),
                font_size: (20.0 * scale).round() as u16,
                color: line_fade(status, 0.92),
                ..Default::default()
            },
        );
    }
    let lines = result_report_lines(&result, engine.config.buffer_size);
    let row_h = (body.h / lines.len().max(1) as f32).clamp(26.0 * scale, 42.0 * scale);
    let label_x = body.x + 28.0 * scale;
    let value_x = body.x + body.w * 0.54;
    for (idx, (left, right)) in lines.iter().enumerate() {
        let row_y = body.y + idx as f32 * row_h;
        let y = row_y + row_h * 0.66;
        if y > body.y + body.h * reveal {
            continue;
        }
        let item = Rect::new(
            body.x + 14.0 * scale,
            row_y + 4.0 * scale,
            body.w - 28.0 * scale,
            row_h - 5.0 * scale,
        );
        draw_rectangle(
            item.x,
            item.y,
            item.w,
            item.h,
            Color::from_rgba(0, 0, 0, 28),
        );
        draw_text_ex(
            left,
            label_x,
            y,
            TextParams {
                font: Some(&assets.font_medium),
                font_size: (19.0 * scale).round() as u16,
                color: fade(status, 0.82),
                ..Default::default()
            },
        );
        draw_line(
            body.x + body.w * 0.34,
            y - 6.0 * scale,
            value_x - 16.0 * scale,
            y - 6.0 * scale,
            style.hairline,
            line_fade(status, 0.58),
        );
        draw_text_fit(
            right,
            Rect::new(
                value_x,
                row_y,
                body.x + body.w - value_x - 24.0 * scale,
                row_h,
            ),
            &assets.font_bold,
            22.0 * scale,
            fade(status, 0.92),
            0.66,
        );
    }
    if footer_reveal > 0.55 {
        centered_text(
            summary.reason,
            footer,
            &assets.font_bold,
            (20.0 * scale).round() as u16,
            line_fade(status, 0.90),
            0.64,
        );
    }
}

fn result_report_lines(result: &GameResult, buffer_size: usize) -> Vec<(String, String)> {
    let status = if result.success {
        "INSTALLED"
    } else {
        "FAILED"
    };
    let reason = if result.terminal_reason == Some(TerminalReason::AllUploaded) {
        "ALL DAEMONS"
    } else if result.out_of_time {
        "TIMEOUT"
    } else if result.buffer_full {
        "BUFFER FULL"
    } else if result.terminal_reason == Some(TerminalReason::NoCompletableSequences) {
        "NO FIT"
    } else {
        "PATH CLOSED"
    };
    vec![
        ("SCORE".to_string(), result.score.to_string()),
        (
            "DAEMONS".to_string(),
            format!("{}/{}", result.sequences_uploaded, result.total_sequences),
        ),
        (
            "BUFFER".to_string(),
            format!("{}/{}", result.buffer_used, buffer_size),
        ),
        ("EXIT".to_string(), reason.to_string()),
        ("STATUS".to_string(), status.to_string()),
    ]
}

struct TerminalSummary {
    title: &'static str,
    reason: &'static str,
    tone: TerminalTone,
}

#[derive(Clone, Copy)]
enum TerminalTone {
    Success,
    Partial,
    Failure,
}

fn terminal_summary(result: &GameResult, _buffer_size: usize) -> TerminalSummary {
    let all_uploaded =
        result.total_sequences > 0 && result.sequences_uploaded == result.total_sequences;
    TerminalSummary {
        title: if all_uploaded {
            "UPLOAD COMPLETE"
        } else if result.success {
            "UPLOAD PARTIAL"
        } else {
            "UPLOAD FAILED"
        },
        reason: if all_uploaded {
            "All daemons installed"
        } else if result.success {
            "Some daemons installed"
        } else if result.out_of_time {
            "Breach time expired"
        } else if result.buffer_full {
            "Buffer capacity reached"
        } else if result.terminal_reason == Some(TerminalReason::NoCompletableSequences) {
            "No daemon can still upload"
        } else {
            "No legal breach path remains"
        },
        tone: if all_uploaded {
            TerminalTone::Success
        } else if result.success {
            TerminalTone::Partial
        } else {
            TerminalTone::Failure
        },
    }
}

#[cfg(test)]
mod tests {
    use rust_breach_protocol::engine::{BreachConfig, GameEngine};

    use super::super::layout::game_layout;
    use super::*;

    fn engine(buffer_size: usize, sequences: Vec<Vec<&str>>) -> GameEngine {
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
                time_limit_seconds: 30,
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
    fn buffer_and_sequences_share_token_lane() {
        for (width, height) in [(900.0, 620.0), (1440.0, 900.0), (2560.0, 1440.0)] {
            let layout = game_layout(width, height, 9);
            let engine = engine(16, vec![vec!["AA", "BB", "DD"]]);
            let buffer = buffer_geometry(&engine, &layout);
            assert!(
                (buffer.start_x - layout.token_lane.x).abs() < 0.01,
                "{buffer:?} {:?}",
                layout.token_lane
            );
            assert!(
                buffer.total_w <= layout.token_lane.w + 0.01,
                "{buffer:?} {:?}",
                layout.token_lane
            );
            assert!(buffer.slot_size >= 14.0, "{buffer:?}");
        }
    }
}
