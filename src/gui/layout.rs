use macroquad::prelude::*;
use rust_breach_protocol::engine::Cell;

use super::settings::{Field, Settings};

pub(super) struct Layout {
    pub(super) grid_origin: Vec2,
    pub(super) cell_size: f32,
    pub(super) matrix_panel: Rect,
    pub(super) sequence_panel: Rect,
    pub(super) timer_panel: Rect,
    pub(super) buffer_panel: Rect,
    pub(super) token_lane: Rect,
    pub(super) scale: f32,
}

pub(super) struct SettingsLayout {
    pub(super) panel: Rect,
    pub(super) rows: Vec<SettingRow>,
    pub(super) start_button: Rect,
    pub(super) error_rect: Rect,
    pub(super) scale: f32,
}

#[derive(Clone, Copy)]
pub(super) struct SettingRow {
    pub(super) field: Field,
    pub(super) row: Rect,
    pub(super) value_box: Rect,
    pub(super) minus_button: Rect,
    pub(super) plus_button: Rect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SettingsHit {
    Start,
    Step(Field, i32),
}

pub(super) fn ui_scale_for(width: f32, height: f32) -> f32 {
    (width / 1440.0).min(height / 900.0).clamp(0.52, 1.45)
}

pub(super) fn game_layout(width: f32, height: f32, matrix_size: usize) -> Layout {
    let scale = ui_scale_for(width, height);
    let margin = (width * 0.045)
        .clamp(16.0 * scale, 70.0 * scale)
        .min(width * 0.08)
        .min(height * 0.08);
    let gap = (width * 0.024)
        .clamp(10.0 * scale, 36.0 * scale)
        .min(width * 0.035);
    let top_h = (height * 0.15)
        .clamp(68.0 * scale, 146.0 * scale)
        .min(height * 0.22);
    let bottom_y = margin + top_h + 18.0 * scale;
    let bottom_h = (height - bottom_y - margin).max(0.0);

    let available_w = (width - margin * 2.0 - gap).max(0.0);
    let left_w = (available_w * 0.43).clamp(0.0, 700.0 * scale);
    let right_w = (available_w - left_w).max(0.0);
    let right_x = margin + left_w + gap;
    let timer_w = (available_w * 0.28).clamp(250.0 * scale, 430.0 * scale);

    let matrix_panel = Rect::new(margin, bottom_y, left_w, bottom_h);
    let sequence_panel = Rect::new(right_x, bottom_y, right_w, bottom_h);
    let token_lane_w = (right_w - 92.0 * scale)
        .max(210.0 * scale)
        .min(620.0 * scale);
    let token_lane = Rect::new(
        sequence_panel.x + 46.0 * scale,
        sequence_panel.y,
        token_lane_w.min((right_w - 92.0 * scale).max(210.0 * scale)),
        sequence_panel.h,
    );
    let header_h = (42.0 * scale).min(matrix_panel.h * 0.22);
    let pad_x = (28.0 * scale).min(matrix_panel.w * 0.12);
    let pad_y = (22.0 * scale).min(matrix_panel.h * 0.08);
    let grid_area = Rect::new(
        matrix_panel.x + pad_x,
        matrix_panel.y + header_h + pad_y,
        (matrix_panel.w - pad_x * 2.0).max(0.0),
        (matrix_panel.h - header_h - pad_y * 2.0).max(0.0),
    );
    let size = matrix_size.max(1) as f32;
    let cell_size = (grid_area.w / size)
        .min(grid_area.h / size)
        .max(1.0)
        .min(74.0 * scale);
    let grid_size = cell_size * size;

    Layout {
        grid_origin: vec2(
            grid_area.x + (grid_area.w - grid_size).max(0.0) * 0.5,
            grid_area.y + (grid_area.h - grid_size).max(0.0) * 0.5,
        ),
        cell_size,
        matrix_panel,
        sequence_panel,
        timer_panel: Rect::new(margin, margin, timer_w, top_h),
        buffer_panel: Rect::new(right_x, margin, right_w, top_h),
        token_lane,
        scale,
    }
}

pub(super) fn settings_layout(width: f32, height: f32) -> SettingsLayout {
    let scale = ui_scale_for(width, height);
    let margin = (width * 0.055).clamp(22.0 * scale, 86.0 * scale);
    let panel_w = (width * 0.60)
        .clamp(560.0 * scale, 900.0 * scale)
        .min(width - margin * 2.0);
    let panel_h = (height * 0.66)
        .clamp(430.0 * scale, 610.0 * scale)
        .min(height - margin * 2.0);
    let panel = Rect::new(
        ((width - panel_w) * 0.5).max(margin),
        (height - panel_h) * 0.52,
        panel_w,
        panel_h,
    );
    let row_start = panel.y + 142.0 * scale;
    let row_gap = ((panel.h - 230.0 * scale) / 5.0).clamp(46.0 * scale, 64.0 * scale);
    let button_w = 34.0 * scale;
    let button_gap = 8.0 * scale;
    let row_w = panel.w - 72.0 * scale;
    let value_box_w = (row_w * 0.24).clamp(92.0 * scale, 150.0 * scale);

    let rows = Field::ALL
        .iter()
        .enumerate()
        .map(|(idx, &field)| {
            let row = Rect::new(
                panel.x + 36.0 * scale,
                row_start + row_gap * idx as f32,
                row_w,
                38.0 * scale,
            );
            SettingRow {
                field,
                row,
                value_box: Rect::new(
                    row.x + row.w - value_box_w - button_w * 2.0 - button_gap * 3.0,
                    row.y + 2.0 * scale,
                    value_box_w,
                    row.h - 4.0 * scale,
                ),
                minus_button: Rect::new(
                    row.x + row.w - button_w * 2.0 - button_gap,
                    row.y + 2.0 * scale,
                    button_w,
                    row.h - 4.0 * scale,
                ),
                plus_button: Rect::new(
                    row.x + row.w - button_w,
                    row.y + 2.0 * scale,
                    button_w,
                    row.h - 4.0 * scale,
                ),
            }
        })
        .collect();

    SettingsLayout {
        panel,
        rows,
        start_button: Rect::new(
            panel.x + panel.w - 196.0 * scale,
            panel.y + panel.h - 58.0 * scale,
            160.0 * scale,
            40.0 * scale,
        ),
        error_rect: Rect::new(
            panel.x + 36.0 * scale,
            panel.y + 100.0 * scale,
            panel.w - 72.0 * scale,
            26.0 * scale,
        ),
        scale,
    }
}

pub(super) fn settings_hit(
    settings: &Settings,
    layout: &SettingsLayout,
    point: Vec2,
) -> Option<SettingsHit> {
    if point_in_rect(point, layout.start_button) {
        return Some(SettingsHit::Start);
    }
    layout.rows.iter().find_map(|row| {
        if settings.can_step(row.field, 1) && point_in_rect(point, row.plus_button) {
            Some(SettingsHit::Step(row.field, 1))
        } else if settings.can_step(row.field, -1) && point_in_rect(point, row.minus_button) {
            Some(SettingsHit::Step(row.field, -1))
        } else {
            None
        }
    })
}

pub(super) fn cell_at_point(
    origin: Vec2,
    cell_size: f32,
    matrix_size: usize,
    point: Vec2,
) -> Option<Cell> {
    let x = point.x - origin.x;
    let y = point.y - origin.y;
    if x < 0.0 || y < 0.0 {
        return None;
    }
    let col = (x / cell_size) as usize;
    let row = (y / cell_size) as usize;
    (row < matrix_size && col < matrix_size).then_some((row, col))
}

pub(super) fn restart_button_rect(layout: &Layout) -> Rect {
    let scale = layout.scale;
    let button_w = (230.0 * scale).min(layout.sequence_panel.w * 0.42);
    let button_h = 48.0 * scale;
    Rect::new(
        layout.sequence_panel.x + (layout.sequence_panel.w - button_w) * 0.5,
        layout.sequence_panel.y + layout.sequence_panel.h - button_h - 34.0 * scale,
        button_w,
        button_h,
    )
}

pub(super) fn result_panel_rect(layout: &Layout) -> Rect {
    let scale = layout.scale;
    let panel_w = (layout.matrix_panel.w - 32.0 * scale).max(260.0 * scale);
    let panel_h = (layout.matrix_panel.h * 0.54)
        .clamp(220.0 * scale, 360.0 * scale)
        .min(layout.matrix_panel.h - 150.0 * scale);
    Rect::new(
        layout.matrix_panel.x + 16.0 * scale,
        layout.matrix_panel.y + 70.0 * scale,
        panel_w,
        panel_h,
    )
}

pub(super) fn point_in_rect(point: Vec2, rect: Rect) -> bool {
    point.x >= rect.x
        && point.x <= rect.x + rect.w
        && point.y >= rect.y
        && point.y <= rect.y + rect.h
}

#[cfg(test)]
mod tests {
    use macroquad::prelude::{Rect, vec2};

    use super::*;

    fn assert_inside(rect: Rect, width: f32, height: f32) {
        assert!(rect.x >= -0.01, "{rect:?}");
        assert!(rect.y >= -0.01, "{rect:?}");
        assert!(rect.x + rect.w <= width + 0.01, "{rect:?}");
        assert!(rect.y + rect.h <= height + 0.01, "{rect:?}");
    }

    #[test]
    fn layout_stays_inside_supported_windows() {
        for (width, height) in [(900.0, 620.0), (1440.0, 900.0), (2560.0, 1440.0)] {
            let layout = game_layout(width, height, 9);
            assert_inside(layout.matrix_panel, width, height);
            assert_inside(layout.sequence_panel, width, height);
            assert_inside(layout.timer_panel, width, height);
            assert_inside(layout.buffer_panel, width, height);
            assert_inside(layout.token_lane, width, height);

            let settings = settings_layout(width, height);
            assert_inside(settings.panel, width, height);
            assert_inside(settings.start_button, width, height);
        }
    }

    #[test]
    fn result_controls_stay_inside_supported_windows() {
        for (width, height) in [(900.0, 620.0), (1440.0, 900.0), (2560.0, 1440.0)] {
            let layout = game_layout(width, height, 9);
            assert_inside(result_panel_rect(&layout), width, height);
            assert_inside(restart_button_rect(&layout), width, height);
        }
    }

    #[test]
    fn cell_hit_test_maps_and_rejects_points() {
        let origin = vec2(100.0, 200.0);
        assert_eq!(
            cell_at_point(origin, 40.0, 5, vec2(100.0, 200.0)),
            Some((0, 0))
        );
        assert_eq!(
            cell_at_point(origin, 40.0, 5, vec2(299.9, 399.9)),
            Some((4, 4))
        );
        assert_eq!(cell_at_point(origin, 40.0, 5, vec2(99.9, 200.0)), None);
        assert_eq!(cell_at_point(origin, 40.0, 5, vec2(300.0, 200.0)), None);
    }

    #[test]
    fn point_in_rect_accepts_center_of_restart_button() {
        let layout = game_layout(1440.0, 900.0, 2);
        let restart = restart_button_rect(&layout);
        assert!(point_in_rect(
            vec2(restart.x + restart.w * 0.5, restart.y + restart.h * 0.5),
            restart,
        ));
    }
}
