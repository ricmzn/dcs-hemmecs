use raqote::{AntialiasMode, BlendMode, Color, DrawOptions, SolidSource, Source};

pub const WIDTH: i32 = 1024;
pub const HEIGHT: i32 = 768;
pub const DEFAULT_FONT: &[u8] = include_bytes!("../fonts/Inconsolata-SemiBold.ttf");
pub const FONT_SIZE: f32 = 48.0;
pub const TEXT_OFFSET_Y: f32 = FONT_SIZE * 5.0 / 6.0;
pub const TEXT_COLUMNS: i32 = (WIDTH as f32 / (FONT_SIZE / 2.0)) as i32;

pub const DRAW_OPTIONS: DrawOptions = DrawOptions {
    antialias: AntialiasMode::None,
    blend_mode: BlendMode::Src,
    alpha: 1.0,
};

pub fn background() -> SolidSource {
    Color::new(255, 0, 0, 0).into()
}

pub fn red() -> Source<'static> {
    Color::new(255, 255, 0, 0).into()
}

pub fn green() -> Source<'static> {
    Color::new(255, 0, 255, 0).into()
}