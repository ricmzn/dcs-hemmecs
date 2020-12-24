use font_kit::font::Font;
use once_cell::unsync::Lazy;
use raqote::{DrawTarget, PathBuilder, Point, Source, StrokeStyle};
use regex::Regex;
use winapi::shared::windef::HWND;

use crate::{
    config::Config,
    consts::{
        background, red, rgb, DRAW_OPTIONS, FONT_SIZE, HEIGHT, TEXT_COLUMNS, TEXT_OFFSET_X,
        TEXT_OFFSET_Y, WIDTH,
    },
    data::{FlightData, UnitSystem},
    windows::is_focused,
};

const WEAPON_CODE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?:\w+[-.])?(\w+)(?:\s.+)?").unwrap());

/// Formats a line as left and right aligned columns of text
fn two_columns(left: &str, right: &str) -> String {
    format!(
        "{left}{right:>padding$}",
        left = left,
        right = right,
        padding = TEXT_COLUMNS as usize - left.len()
    )
}

/// Draws text with the default font size
fn draw_text(draw_target: &mut DrawTarget, font: &Font, color: &Source, text: &str) {
    let char_ids = text
        .chars()
        .map(|c| font.glyph_for_char(c).unwrap_or_default())
        .collect::<Vec<_>>();

    let char_positions = text
        .chars()
        .map({
            // Init
            let mut x = TEXT_OFFSET_X;
            let mut y = TEXT_OFFSET_Y;
            // Loop
            move |c| {
                if c == '\n' {
                    x = FONT_SIZE / 6.0;
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

pub fn draw<'a>(
    hwnd: HWND,
    config: &Config,
    data: &Option<FlightData>,
    draw_target: &'a mut DrawTarget,
    default_font: &Font,
) -> &'a [u32] {
    draw_target.clear(background());

    let color = rgb(config.appearance.color);

    if let Some(data) = data {
        let cockpit_params = data.parse_cockpit_params().unwrap_or_default();
        let camera_angles = data.camera_angles();

        let text = if cockpit_params.ejected {
            String::from("YEET")
        } else if FlightData::is_occluded(camera_angles, &config) {
            String::from("*")
        } else {
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
                                format!("{:.0} lbs", engine_data.total_fuel() * 2.204622622)
                            }
                        };
                        two_columns(&aoa_str, &fuel_str)
                    } else {
                        // Just AoA
                        aoa_str
                    }
                }
            )
        };
        draw_text(draw_target, &default_font, &color, &text);
    } else {
        draw_text(draw_target, &default_font, &color, "Not Connected");
    }

    // Paint window border in case it's in focus
    if is_focused(hwnd) {
        let mut pb = PathBuilder::new();
        pb.rect(0.0, 0.0, WIDTH as f32, HEIGHT as f32);
        draw_target.stroke(
            &pb.finish(),
            &red(),
            &StrokeStyle {
                width: 4.0,
                ..Default::default()
            },
            &DRAW_OPTIONS,
        );
    }

    draw_target.get_data()
}
