use font_kit::font::Font;
use raqote::DrawTarget;
use serde::Deserialize;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Default)]
pub struct CockpitParams {
    pub ejected: bool,
}

#[serde(default)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[serde(default)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct EngineStat {
    pub left: f32,
    pub right: f32,
}

#[serde(default)]
#[derive(Debug, Clone, Default, Deserialize)]
#[allow(non_snake_case)]
pub struct EngineData {
    pub RPM: EngineStat,
    pub fuel_internal: f32,
    pub fuel_external: f32,
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
    pub g: Position,
    pub engine_data: Option<EngineData>,
}

impl FlightData {
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
