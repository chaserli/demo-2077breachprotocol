use macroquad::audio::{Sound, load_sound_from_bytes};
use macroquad::prelude::*;

const ASSET_ICON_MATRIX_BYTES: &[u8] = include_bytes!("../../assets/img/icon-code-matrix.png");
const ASSET_ICON_SEQUENCE_BYTES: &[u8] = include_bytes!("../../assets/img/icon-code-sequnce.png");
const FONT_CYBERPUNK_BYTES: &[u8] = include_bytes!("../../assets/font/Cyberpunk.ttf");
const FONT_MEDIUM_BYTES: &[u8] = include_bytes!("../../assets/font/Rajdhani-Medium.ttf");
const FONT_BOLD_BYTES: &[u8] = include_bytes!("../../assets/font/Rajdhani-Bold.ttf");
const AUDIO_CLICK_BYTES: &[u8] = include_bytes!("../../assets/audio/Active_FX_Button.wav");
const AUDIO_BGM_BYTES: &[u8] =
    include_bytes!("../../assets/audio/Cyberpunk 2077  Breach_Protocol.wav");

pub(crate) struct Assets {
    pub(super) icon_matrix: Texture2D,
    pub(super) icon_sequence: Texture2D,
    pub(super) font_display: Font,
    pub(super) font_medium: Font,
    pub(super) font_bold: Font,
    pub(super) audio: AudioAssets,
}

pub(super) struct AudioAssets {
    pub(super) click: Sound,
    pub(super) bgm: Sound,
}

pub(crate) async fn load_assets() -> Assets {
    let icon_matrix =
        Texture2D::from_file_with_format(ASSET_ICON_MATRIX_BYTES, Some(ImageFormat::Png));
    let icon_sequence =
        Texture2D::from_file_with_format(ASSET_ICON_SEQUENCE_BYTES, Some(ImageFormat::Png));

    let font_display = load_ttf_font_from_bytes(FONT_CYBERPUNK_BYTES)
        .expect("assets/font/Cyberpunk.ttf must be a valid TTF font");
    let font_medium = load_ttf_font_from_bytes(FONT_MEDIUM_BYTES)
        .expect("assets/font/Rajdhani-Medium.ttf must be a valid TTF font");
    let font_bold = load_ttf_font_from_bytes(FONT_BOLD_BYTES)
        .expect("assets/font/Rajdhani-Bold.ttf must be a valid TTF font");
    let click = load_sound_from_bytes(AUDIO_CLICK_BYTES)
        .await
        .expect("assets/audio/Active_FX_Button.wav must be a valid audio file");
    let bgm = load_sound_from_bytes(AUDIO_BGM_BYTES)
        .await
        .expect("assets/audio/Cyberpunk 2077  Breach_Protocol.wav must be a valid audio file");

    icon_matrix.set_filter(FilterMode::Linear);
    icon_sequence.set_filter(FilterMode::Linear);

    Assets {
        icon_matrix,
        icon_sequence,
        font_display,
        font_medium,
        font_bold,
        audio: AudioAssets { click, bgm },
    }
}
