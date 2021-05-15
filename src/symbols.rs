use raqote::{Color, DrawTarget, PathBuilder, StrokeStyle};
use std::f32::consts::PI;

use crate::consts::ANTI_ALIASED;

#[derive(Debug)]
pub enum Identification {
    Hostile,
    Friendly,
    Unknown,
}

#[derive(Debug)]
pub enum Donor {
    Ownship,
    Datalink,
}

const SYMBOL_RADIUS: f32 = 16.0;
const STROKE_WIDTH: f32 = 2.0;

pub fn draw_symbol(
    draw_target: &mut DrawTarget,
    x: f32,
    y: f32,
    iff: &Identification,
    src: &Donor,
    locked: bool,
) {
    let shape = match iff {
        Identification::Hostile => {
            let mut pb = PathBuilder::new();
            pb.move_to(x - SYMBOL_RADIUS, y);
            pb.line_to(x, y - SYMBOL_RADIUS);
            pb.line_to(x + SYMBOL_RADIUS, y);
            if locked {
                pb.line_to(x, y + SYMBOL_RADIUS);
                pb.line_to(x - SYMBOL_RADIUS, y);
            }
            pb.finish()
        }
        Identification::Friendly => {
            let mut pb = PathBuilder::new();
            if locked {
                pb.arc(x, y, SYMBOL_RADIUS, -PI, PI * 2.0);
            } else {
                pb.arc(x, y, SYMBOL_RADIUS, -PI, PI);
            }
            pb.finish()
        }
        Identification::Unknown => {
            let mut pb = PathBuilder::new();
            if locked {
                pb.rect(
                    x - SYMBOL_RADIUS,
                    y - SYMBOL_RADIUS,
                    SYMBOL_RADIUS * 2.0,
                    SYMBOL_RADIUS * 2.0,
                );
            } else {
                pb.move_to(x - SYMBOL_RADIUS, y);
                pb.line_to(x - SYMBOL_RADIUS, y - SYMBOL_RADIUS);
                pb.line_to(x + SYMBOL_RADIUS, y - SYMBOL_RADIUS);
                pb.line_to(x + SYMBOL_RADIUS, y);
            }
            pb.finish()
        }
    };

    let color = match iff {
        Identification::Hostile => Color::new(255, 192, 0, 0).into(),
        Identification::Friendly => Color::new(255, 0, 0, 192).into(),
        Identification::Unknown => Color::new(255, 192, 192, 0).into(),
    };

    let stroke = match src {
        Donor::Ownship => StrokeStyle {
            width: STROKE_WIDTH,
            ..Default::default()
        },
        Donor::Datalink => StrokeStyle {
            width: STROKE_WIDTH,
            dash_array: vec![4.0, 4.0],
            dash_offset: if let Identification::Unknown = iff {
                2.0
            } else {
                0.0
            },
            ..Default::default()
        },
    };

    draw_target.stroke(&shape, &color, &stroke, &ANTI_ALIASED);
}

#[allow(dead_code)]
pub fn display_gallery(draw_target: &mut DrawTarget, x: f32, y: f32) {
    for (i, iff) in [
        Identification::Hostile,
        Identification::Friendly,
        Identification::Unknown,
    ]
    .iter()
    .enumerate()
    {
        for (j, src) in [Donor::Ownship, Donor::Datalink].iter().enumerate() {
            for (k, &locked) in [false, true].iter().enumerate() {
                draw_symbol(
                    draw_target,
                    x + i as f32 * SYMBOL_RADIUS * 4.0,
                    y + j as f32 * SYMBOL_RADIUS * 4.0 + k as f32 * SYMBOL_RADIUS * 8.0,
                    iff,
                    src,
                    locked,
                );
            }
        }
    }
}
