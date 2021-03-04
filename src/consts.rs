use raqote::{AntialiasMode, BlendMode, Color, DrawOptions, SolidSource, Source};

pub const WIDTH: i32 = 1024;
pub const HEIGHT: i32 = 768;
pub const DEFAULT_FONT: &[u8] = include_bytes!("../fonts/Inconsolata-SemiBold.ttf");
pub const FONT_SIZE: f32 = 48.0;
pub const TEXT_OFFSET_X: f32 = -FONT_SIZE / 3.0;
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

pub fn rgb(rgb: (u8, u8, u8)) -> Source<'static> {
    Color::new(255, rgb.0, rgb.1, rgb.2).into()
}

pub const CONFIG_FILE: &str = "dcs-hemmecs.toml";

pub const COULD_NOT_CREATE_CONFIG: &str = "Oops! DCS Hemmecs tried to open or create a configuration file in this folder, but the permission was denied by Windows.

For this reason, it's not recommended to run DCS Hemmecs from Program Files. However, the application will still run using default settings.";

pub const FIRST_TIME_MESSAGE: &str = "It appears this is the first time you're running DCS Hemmecs!

A configuration file (dcs-hemmecs.toml) has been created in this folder.

You can find more information in the readme of the GitHub repository.";
