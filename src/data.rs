use font_kit::font::Font;
use raqote::DrawTarget;
use serde::Deserialize;
use std::cell::RefCell;
use std::sync::RwLock;

use crate::config::{Config, ConfigHandle, Occlusion};

trait ToDegrees {
    fn to_degrees(&self) -> Self;
}

impl ToDegrees for (f32, f32, f32) {
    fn to_degrees(&self) -> Self {
        (
            self.0.to_degrees(),
            self.1.to_degrees(),
            self.2.to_degrees(),
        )
    }
}

pub mod dcs {
    use super::*;

    #[serde(default)]
    #[derive(Debug, Clone, Default, Deserialize)]
    pub struct Vec3 {
        pub x: f32,
        pub y: f32,
        pub z: f32,
    }

    impl Vec3 {
        pub fn as_glm_vec3(&self) -> glm::Vec3 {
            glm::Vec3::new(self.x, self.y, self.z)
        }
    }

    impl From<glm::Vec3> for Vec3 {
        fn from(vec: glm::Vec3) -> Self {
            Vec3 {
                x: vec.x,
                y: vec.y,
                z: vec.z,
            }
        }
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
        /// Rotates all three orientation vectors around a given axis
        pub fn rotate(&self, angle: f32, axis: &glm::Vec3) -> Self {
            let x = glm::rotate_vec3(&self.x.as_glm_vec3(), angle, &axis);
            let y = glm::rotate_vec3(&self.y.as_glm_vec3(), angle, &axis);
            let z = glm::rotate_vec3(&self.z.as_glm_vec3(), angle, &axis);
            Position {
                x: x.into(),
                y: y.into(),
                z: z.into(),
                p: self.p.clone(),
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

    impl EngineData {
        pub fn total_fuel(&self) -> f32 {
            self.fuel_internal + self.fuel_external
        }
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

#[derive(Debug, Clone, Default)]
pub struct CockpitParams {
    pub ejected: bool,
}

pub enum UnitSystem {
    Metric,
    Imperial,
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
    pub unit: String,
}

impl FlightData {
    /// Returns the direction where the camera is pointed relative to the plane
    /// in the format (pitch, yaw, roll)
    pub fn camera_angles(&self) -> (f32, f32, f32) {
        let x_yaw = glm::rotate_vec3(&glm::Vec3::x_axis(), -self.yaw, &glm::Vec3::y_axis());
        let z_yaw = glm::rotate_vec3(&glm::Vec3::z_axis(), -self.yaw, &glm::Vec3::y_axis());
        let x_yaw_pitch = glm::rotate_vec3(&x_yaw, self.pitch, &z_yaw);

        // Rotate the camera in all axes
        let cam = &self.cam;
        let cam = cam.rotate(-self.bank, &x_yaw_pitch);
        let cam = cam.rotate(-self.pitch, &z_yaw);
        let cam = cam.rotate(self.yaw, &glm::Vec3::y_axis());

        // X vector is forward, Y is up, and Z is right
        (cam.x.y.asin(), cam.x.z.atan2(cam.x.x), -cam.z.y.asin())
    }

    pub fn is_occluded(camera_angles: (f32, f32, f32), config: &Config) -> bool {
        let (pitch, yaw, _) = camera_angles.to_degrees();
        let Occlusion {
            hide_on_hud,
            hide_in_cockpit,
            hud_horizontal_angle,
            hud_vertical_angle,
        } = config.occlusion;

        // HUD
        (hide_on_hud && pitch < hud_vertical_angle && yaw.abs() < hud_horizontal_angle) ||
        // Front dash
        (hide_in_cockpit && pitch < -20.0 && yaw.abs() / 1.5 + pitch < -10.0) ||
        // Side consoles
        (hide_in_cockpit && pitch < -45.0)
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

    pub fn get_unit_system(&self) -> UnitSystem {
        // WWII modules are not covered because using this in a WWII scenario would be utterly ridiculous
        if self.unit.starts_with("MiG-")
            || self.unit.starts_with("Su-")
            || self.unit.starts_with("Ka-")
            || self.unit.starts_with("Mi-")
            || self.unit == "L-39"
            || self.unit == "AJS37"
        {
            UnitSystem::Metric
        } else {
            UnitSystem::Imperial
        }
    }
}

pub struct ApplicationState {
    pub flight_data: RwLock<Option<FlightData>>,
    pub draw_target: RefCell<DrawTarget>,
    pub font: RefCell<Font>,
    pub config: ConfigHandle,
}
