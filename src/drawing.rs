use font_kit::font::Font;
use once_cell::sync::Lazy;
use raqote::{DrawTarget, PathBuilder, Point, Source, StrokeStyle};
use regex::Regex;
use winapi::shared::windef::HWND;

use crate::{
    config::Config,
    consts::{
        background, red, rgb, DRAW_OPTIONS, FONT_SIZE, HUD_HEIGHT, HUD_WIDTH, TEXT_COLUMNS,
        TEXT_OFFSET_X, TEXT_OFFSET_Y,
    },
    data::{FlightData, UnitSystem},
};

static WEAPON_CODE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?:\w+[-.])?(\w+)(?:\s.+)?").unwrap());

/// Formats a line as left and right aligned columns of text
fn two_columns(left: &str, right: &str) -> String {
    format!(
        "{left}{right:>padding$}",
        left = left,
        right = right,
        padding = TEXT_COLUMNS as usize - left.len() - 1
    )
}

/// Draws text with the default font size
fn draw_text(
    draw_target: &mut DrawTarget,
    font: &Font,
    color: &Source,
    text: &str,
    offsets: (f32, f32),
) {
    let char_ids = text
        .chars()
        .map(|c| font.glyph_for_char(c).unwrap_or_default())
        .collect::<Vec<_>>();

    let char_positions = text
        .chars()
        .map({
            // Init
            let mut x = TEXT_OFFSET_X + offsets.0;
            let mut y = TEXT_OFFSET_Y + offsets.1;
            // Loop
            move |c| {
                if c == '\n' {
                    x = TEXT_OFFSET_X + offsets.0;
                    y += FONT_SIZE;
                } else {
                    x += FONT_SIZE / 2.0;
                }
                Point::new(x, y)
            }
        })
        .collect::<Vec<_>>();

    draw_target.draw_glyphs(
        &font,
        FONT_SIZE,
        &char_ids,
        &char_positions,
        color,
        &DRAW_OPTIONS,
    );
}

fn render_data(data: &FlightData) -> String {
    // Convert units as necessary
    let unit_system = data.get_unit_system();
    let (ias, ias_digits, alt, alt_digits) = match unit_system {
        // m/s -> km/h and m
        UnitSystem::Metric => (data.ias * 3.6, 4, data.alt, 5),
        // m/s -> kn and m -> ft
        UnitSystem::Imperial => (data.ias * 1.943844, 3, data.alt * 3.28084, 5),
    };
    // Generate the output text
    format!(
        "{}\n{}\n\n\n\n\n\n\n\n\n\n\n\n{}\n{}\n{}",
        format!("                   {:0>3.0}", data.yaw.to_degrees()),
        two_columns(
            &format!("[{0:>1$.0}]", ias, ias_digits),
            &format!("[{0:>1$.0}]", alt, alt_digits)
        ),
        // 3rd line from bottom
        {
            let mach_str = format!("M {:.2}", data.mach);
            if let Some(weapon) = &data.weapons {
                let weapon_str = if let Some(current) = &weapon.current {
                    // Remove the prefix and suffix for brevity
                    let short_name = WEAPON_CODE
                        .captures(&current.name)
                        .and_then(|captures| captures.get(1))
                        .map(|m| m.as_str())
                        .unwrap_or(&current.name);

                    format!("{} {}", short_name, current.count)
                } else {
                    String::new()
                };
                two_columns(&mach_str, &weapon_str)
            } else {
                mach_str
            }
        },
        // 2nd line from bottom
        {
            let g_str = format!("G {:.1}", data.g.y);
            if let Some(weapon) = &data.weapons {
                // G-force and cannon ammo
                let shells_str = format!("GUN {}", weapon.shells);
                two_columns(&g_str, &shells_str)
            } else {
                // Just g-force
                g_str
            }
        },
        // last line
        {
            let aoa_str = format!("a {:.1}", data.aoa);
            if let Some(engine_data) = &data.engine_data {
                // AoA and total fuel (FC3 only)
                let fuel_str = match unit_system {
                    // kg
                    UnitSystem::Metric => format!("{:.0} kg", engine_data.total_fuel()),
                    // kg -> lb
                    UnitSystem::Imperial => {
                        format!("{:.0} lbs", engine_data.total_fuel() * 2.2046225)
                    }
                };
                two_columns(&aoa_str, &fuel_str)
            } else {
                // Just AoA
                aoa_str
            }
        }
    )
}

pub fn draw<'a>(
    hwnd: HWND,
    config: &Config,
    data: &Option<FlightData>,
    draw_target: &'a mut DrawTarget,
    screen_dimensions: (i32, i32),
    default_font: &Font,
) -> &'a [u32] {
    draw_target.clear(background());

    // Nicely place the HUD area of the HMD in the center and a little bit high
    let offsets = (
        screen_dimensions.0 as f32 / 2.0 - HUD_WIDTH as f32 / 2.0,
        screen_dimensions.1 as f32 / 2.0
            - HUD_HEIGHT as f32 / 2.0
            - screen_dimensions.1 as f32 / 10.0,
    );
    let color = rgb(config.appearance.color);

    if config.show_sample_data {
        let sample_data = render_data(&FlightData::sample());
        draw_text(draw_target, &default_font, &color, &sample_data, offsets);
    } else if let Some(data) = data {
        let cockpit_params = data.parse_cockpit_params().unwrap_or_default();
        let camera_angles = data.camera_angles();

        // Radar targets
        for target in &data.targets {
            let point_size = 24.0;
            let position = target.position.p.project(screen_dimensions, &data.cam);
            let dist = (target.distance / 1852.0).round().to_string();
            if let Some((x, y)) = position {
                draw_target.draw_text(
                    default_font,
                    point_size,
                    "<   >",
                    Point::new(x - point_size * (3.0 / 4.0), y + point_size * (2.0 / 4.0)),
                    &color,
                    &DRAW_OPTIONS,
                );
                draw_target.draw_text(
                    default_font,
                    point_size,
                    &dist,
                    Point::new(x - point_size * (1.0 / 4.0), y + point_size * (4.0 / 4.0)),
                    &color,
                    &DRAW_OPTIONS,
                );
            }
        }

        let text = if cockpit_params.ejected {
            String::from("YEET")
        } else if FlightData::is_occluded(camera_angles, &config) {
            String::from("*")
        } else {
            render_data(&data)
        };

        draw_text(draw_target, &default_font, &color, &text, offsets);
    } else {
        draw_text(draw_target, &default_font, &color, "Not Connected", offsets);
    }

    draw_target.get_data()
}
