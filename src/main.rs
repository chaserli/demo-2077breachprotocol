mod gui;

use macroquad::prelude::*;

fn window_conf() -> Conf {
    Conf {
        window_title: "Breach Protocol".to_string(),
        window_width: 1440,
        window_height: 900,
        window_resizable: true,
        high_dpi: true,
        icon: None,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let assets = gui::load_assets().await;
    let mut app = gui::GuiApp::new();

    loop {
        let width = screen_width();
        let height = screen_height();
        clear_background(gui::background_color());
        let mouse_pos = mouse_vec();

        app.update(
            &assets,
            mouse_pos,
            is_mouse_button_pressed(MouseButton::Left),
            width,
            height,
        );
        app.draw(&assets, mouse_pos, width, height);
        next_frame().await;
    }
}

fn mouse_vec() -> Vec2 {
    let (x, y) = mouse_position();
    vec2(x, y)
}
