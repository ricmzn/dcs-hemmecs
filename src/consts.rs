use raqote::{AntialiasMode, BlendMode, Color, DrawOptions, SolidSource, Source};

pub const HUD_WIDTH: i32 = 1024;
pub const HUD_HEIGHT: i32 = 768;
pub const FONT_SIZE: f32 = 48.0;
pub const TEXT_OFFSET_X: f32 = -FONT_SIZE / 3.0;
pub const TEXT_OFFSET_Y: f32 = FONT_SIZE * 5.0 / 6.0;
pub const TEXT_COLUMNS: i32 = (HUD_WIDTH as f32 / (FONT_SIZE / 2.0)) as i32;
pub static DEFAULT_FONT: &[u8] = include_bytes!("../fonts/Inconsolata-SemiBold.ttf");

pub static ANTI_ALIASED: DrawOptions = DrawOptions {
    antialias: AntialiasMode::Gray,
    blend_mode: BlendMode::SrcOver,
    alpha: 1.0,
};

pub fn background() -> SolidSource {
    Color::new(0, 0, 0, 0).into()
}

pub fn rgb(rgb: (u8, u8, u8)) -> Source<'static> {
    Color::new(255, rgb.0, rgb.1, rgb.2).into()
}

pub static CONFIG_FILE: &str = "dcs-hemmecs.toml";
