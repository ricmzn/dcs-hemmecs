use font_kit::font::Font;
use nalgebra::{Matrix3, Rotation3};
use raqote::DrawTarget;
use serde::Deserialize;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

pub mod dcs {
    use super::*;

    #[serde(default)]
    #[derive(Debug, Clone, Default, Deserialize)]
    pub struct Vec3 {
        pub x: f32,
        pub y: f32,
        pub z: f32,
    }

    #[serde(default)]
    #[derive(Debug, Clone, Default, Deserialize)]
    pub struct Position {
        /// Orientation x-vector
        pub x: Vec3,
        /// Orientation y-vector
        pub y: Vec3,
        /// Orientation z-vector
        pub z: Vec3,
        /// Position in world
        pub p: Vec3,
    }

    impl Position {
        #[rustfmt::skip]
        pub fn rotation(&self) -> Rotation3<f32> {
            Rotation3::<f32>::from_matrix_unchecked(Matrix3::new(
                self.x.x, self.z.x, self.y.x,
                self.x.z, self.z.z, self.y.z,
                self.x.y, self.z.y, self.y.y,
            ))
        }

        pub fn get_relative_vector(&self, other: &Rotation3<f32>) -> Vec3 {
            let self_rotation = self.rotation();
            let difference = self_rotation.matrix() - other.matrix();
            Vec3 {
                x: difference.m11,
                y: difference.m21,
                z: difference.m31,
            }
        }
    }

    #[serde(default)]
    #[derive(Debug, Clone, Default, Deserialize)]
    pub struct EngineDetails {
        pub left: f32,
        pub right: f32,
    }

    #[serde(default)]
    #[derive(Debug, Clone, Default, Deserialize)]
    #[allow(non_snake_case)]
    pub struct EngineData {
        pub RPM: EngineDetails,
        pub fuel_internal: f32,
        pub fuel_external: f32,
    }

    #[serde(default)]
    #[derive(Debug, Clone, Default, Deserialize)]
    pub struct WeaponDetails {
        pub name: String,
        pub count: i32,
    }

    #[serde(default)]
    #[derive(Debug, Clone, Default, Deserialize)]
    pub struct WeaponData {
        pub current: Option<WeaponDetails>,
        pub shells: i32,
    }
}

#[serde(default)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct FlightData {
    pub cp_params: Option<String>,
    pub time: f32,
    pub ias: f32,
    pub mach: f32,
    pub alt: f32,
    pub rad_alt: f32,
    pub pitch: f32,
    pub bank: f32,
    pub yaw: f32,
    pub aoa: f32,
    pub g: dcs::Vec3,
    pub cam: dcs::Position,
    pub engine_data: Option<dcs::EngineData>,
    pub weapons: Option<dcs::WeaponData>,
}

#[derive(Debug, Clone, Default)]
pub struct CockpitParams {
    pub ejected: bool,
}

impl FlightData {
    pub fn camera_relative_vector(&self) -> dcs::Vec3 {
        self.cam.get_relative_vector(&Rotation3::from_euler_angles(
            -self.bank,
            -self.pitch,
            self.yaw,
        ))
    }

    pub fn parse_cockpit_params(&self) -> Option<CockpitParams> {
        self.cp_params.as_ref().map(|params_raw| {
            let mut params = CockpitParams::default();
            // DCS undocumented cockpit param format
            // Each parameter is separated by a line break,
            // and is presented in the format Key:Value
            for param in params_raw.split("\n") {
                let mut key_value = param.split(":");
                if let Some(key) = key_value.next() {
                    if let Some(value) = key_value.next() {
                        match key {
                            "EJECTION_INITIATED_0" => {
                                // (Undocumented) values:
                                // -1: not ejected
                                // >1: ejecting
                                // 0: pilot absent or dead
                                params.ejected = value.parse::<f32>().unwrap_or(-1.0) >= 0.0;
                            }
                            _ => (),
                        }
                    }
                }
            }
            params
        })
    }
}

pub struct WindowData {
    pub flight_data: Arc<Mutex<FlightData>>,
    pub draw_target: RefCell<DrawTarget>,
    pub font: RefCell<Font>,
}
