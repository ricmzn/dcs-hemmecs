use font_kit::font::Font;
use raqote::{DrawTarget, PathBuilder, Point, StrokeStyle};
use std::cell::Cell;
use winapi::shared::windef::HWND;
use winapi::um::winuser::GetFocus;

use crate::{
    consts::{background, green, red, DRAW_OPTIONS, FONT_SIZE, HEIGHT, TEXT_OFFSET_Y, WIDTH},
    data::FlightData,
};

pub fn draw<'a>(
    hwnd: HWND,
    data: &FlightData,
    draw_target: &'a mut DrawTarget,
    default_font: &Font,
) -> &'a [u32] {
    draw_target.clear(background());

    let cp = data.parse_cockpit_params().unwrap_or_default();

    // Format text information
    let text = if !cp.ejected {
        format!(
            "{}\n{}\n\n\n\n\n\n\n\n\n\n\n\n{}\n{}\n{}",
            format!("                   {:0>3.0}", data.yaw.to_degrees()),
            format!(
                "[{:>3.0}]                              [{:>5.0}]",
                data.ias * 1.943844, // m/s -> kn
                data.alt * 3.28084   // m -> ft
            ),
            format!("M {:.2}", data.mach),
            format!("G {:.1}", data.g.y),
            {
                let aoa_str = format!("a {:.1}", data.aoa);
                if let Some(engine_data) = &data.engine_data {
                    let fuel_str = format!(
                        "{:.0} lbs",
                        (engine_data.fuel_internal + engine_data.fuel_external) * 2.204622622 // kg -> lb
                    );
                    format!("{0}{1:>2$}", aoa_str, fuel_str, 42 - aoa_str.len())
                } else {
                    aoa_str
                }
            }
        )
    } else {
        format!("EJECTED")
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
