use font_kit::font::Font;
use once_cell::unsync::Lazy;
use raqote::{DrawTarget, PathBuilder, Point, StrokeStyle};
use regex::Regex;
use std::cell::Cell;
use winapi::shared::windef::HWND;
use winapi::um::winuser::GetFocus;

use crate::{
    config::Config,
    consts::{
        background, green, red, DRAW_OPTIONS, FONT_SIZE, HEIGHT, TEXT_COLUMNS, TEXT_OFFSET_Y, WIDTH,
    },
    data::FlightData,
};

const WEAPON_CODE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?:\w+[-.])?(\w+)(?:\s.+)?").unwrap());

fn two_columns(left: &str, right: &str) -> String {
    format!(
        "{left}{right:>padding$}",
        left = left,
        right = right,
        padding = TEXT_COLUMNS as usize - left.len()
    )
}

pub fn draw<'a>(
    hwnd: HWND,
    config: &Config,
    data: &FlightData,
    draw_target: &'a mut DrawTarget,
    default_font: &Font,
) -> &'a [u32] {
    draw_target.clear(background());

    let cockpit_params = data.parse_cockpit_params().unwrap_or_default();
    let camera_angles = data.camera_angles();

    // Format text information
    let text = if cockpit_params.ejected {
        String::from("YEET")
    } else if FlightData::is_occluded(camera_angles, &config) {
        String::from("*")
    } else {
        format!(
            "{}\n{}\n\n\n\n\n\n\n\n\n\n\n\n{}\n{}\n{}",
            format!("                   {:0>3.0}", data.yaw.to_degrees()),
            format!(
                "[{:>3.0}]                              [{:>5.0}]",
                data.ias * 1.943844, // m/s -> kn
                data.alt * 3.28084   // m -> ft
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
                    let shells_str = format!("GUN {}", weapon.shells);
                    two_columns(&g_str, &shells_str)
                } else {
                    g_str
                }
            },
            // last line
            {
                let aoa_str = format!("a {:.1}", data.aoa);
                if let Some(engine_data) = &data.engine_data {
                    let fuel_str = format!(
                        "{:.0} lbs",
                        (engine_data.fuel_internal + engine_data.fuel_external) * 2.204622622 // kg -> lb
                    );
                    two_columns(&aoa_str, &fuel_str)
                } else {
                    aoa_str
                }
            }
        )
    };

    // Draw text on the canvas
    draw_target.draw_glyphs(
        &default_font,
        FONT_SIZE,
        &text
            .chars()
            .map(|c| default_font.glyph_for_char(c).unwrap_or_default())
            .collect::<Vec<_>>(),
        &text
            .chars()
            .map({
                let x = Cell::new(FONT_SIZE / 6.0);
                let y = Cell::new(TEXT_OFFSET_Y);
                move |c| {
                    let p = Point::new(x.get(), y.get());
                    if c == '\n' {
                        x.replace(FONT_SIZE / 6.0);
                        y.replace(y.get() + FONT_SIZE);
                    } else {
                        x.replace(x.get() + FONT_SIZE / 2.0);
                    }
                    p
                }
            })
            .collect::<Vec<_>>(),
        &green(),
        &DRAW_OPTIONS,
    );

    // Paint window border in case it's in focus
    if unsafe { GetFocus() } == hwnd {
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
