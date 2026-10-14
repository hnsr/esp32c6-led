use core::cell::Cell;
use crate::hw::{DiodeLayout, HardwareConfig, TemperatureRange};

// Approximate X/Y values for (s)RGB white, which we want to use as the default
const XY_SRGB_WHITE_X: u16 = 0x500D;
const XY_SRGB_WHITE_Y: u16 = 0x5439;

// Temperature (in mireds) for warm white (2700K) and cool white (6500K)
const TEMPERATURE_MIREDS_6500K: u16 = 154; // Cool
const TEMPERATURE_MIREDS_2700K: u16 = 370; // Warm


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorMode {
    Xy,
    Temperature
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LampState {
    pub on: bool,
    pub color_mode: ColorMode,

    pub(super) brightness: u8,

    // Color in XY
    pub(super) x: u16,
    pub(super) y: u16,

    // Temperature in mireds
    pub(super) temperature: u16,

    pub(super) capabilities: LampCapabilities,
}

impl LampState {
    pub fn get_brightness(&self) -> f32 {
        f32::from(self.brightness) / 254.0
    }

    pub fn get_x(&self) -> f32 {
        f32::from(self.x) / 65_536.0
    }

    pub fn get_y(&self) -> f32 {
        f32::from(self.y) / 65_536.0
    }

    pub fn get_temperature(&self) -> f32 {
        f32::from(self.temperature)
    }
}

// fixme: these defaults and the nested capabilities are a bit awkward, maybe rethink this
impl Default for LampState {
    fn default() -> Self {
        Self {
            capabilities: LampCapabilities::new(
                true,
                true,
                TEMPERATURE_MIREDS_6500K,
                500
            ),
            on: true,
            brightness: u8::MAX / 2,
            color_mode: ColorMode::Xy,
            x: XY_SRGB_WHITE_X,
            y: XY_SRGB_WHITE_Y,
            temperature: TEMPERATURE_MIREDS_2700K,
        }
    }
}

pub type SharedLampState = Cell<LampState>;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LampCapabilities {
    pub(super) supports_xy: bool,
    pub(super) supports_temperature: bool,
    pub(super) temperature_min: u16,
    pub(super) temperature_max: u16,

}

impl LampCapabilities {
    pub fn new(
        supports_xy: bool,
        supports_temperature: bool,
        temperature_min: u16,
        temperature_max: u16
    ) -> Self {
        Self {
            supports_xy,
            supports_temperature,
            temperature_min,
            temperature_max,
        }
    }
    pub fn from_hardware(hw: &HardwareConfig) -> Self {
        let supports_xy = match hw.diodes {
            DiodeLayout::White { .. } => false,
            DiodeLayout::WhiteCct { .. } => false,
            DiodeLayout::Rgb { .. } => true,
            DiodeLayout::Rgbw { .. } => true,
            DiodeLayout::RgbCct { .. } => true,
        };

        let temperature_range = match hw.diodes {
            DiodeLayout::White {
                physical_temperature,
            } => {
                // One white emitter so only one native temperature
                Some(TemperatureRange {
                    min_kelvin: physical_temperature,
                    max_kelvin: physical_temperature,
                })
            }

            DiodeLayout::WhiteCct {
                physical_temperature,
            } => Some(physical_temperature),

            DiodeLayout::Rgb {
                emulated_temperature,
            } => {
                // RGB has no native white temperature so this is either the configured emulation
                // range or None
                emulated_temperature
            }

            DiodeLayout::Rgbw {
                physical_temperature,
                emulated_temperature,
            } => {
                let native_range = TemperatureRange {
                    min_kelvin: physical_temperature,
                    max_kelvin: physical_temperature,
                };

                // Use the emulation range when present, otherwise pass on native temperature as a
                // range
                Some(emulated_temperature.unwrap_or(native_range))
            }

            DiodeLayout::RgbCct {
                physical_temperature,
            } => Some(physical_temperature),
        };

        let (supports_temperature, temperature_min, temperature_max) =
            match temperature_range {
                Some(range) => {
                    assert!(range.min_kelvin > 0);
                    assert!(range.min_kelvin <= range.max_kelvin);

                    let min = kelvin_to_mireds(range.max_kelvin);
                    let max = kelvin_to_mireds(range.min_kelvin);

                    // Advertise adjustable temperature only when there's an actual range
                    (min < max, min, max)
                }
                // Temperature control is unavailable
                None => (false, 0, 0),
            };

        Self::new(
            supports_xy,
            supports_temperature,
            temperature_min,
            temperature_max,
        )
    }
}

// fixme: move to color.rs?
fn kelvin_to_mireds(kelvin: u16) -> u16 {
    let kelvin = u32::from(kelvin);
    let mireds = (1_000_000 + kelvin / 2) / kelvin;
    u16::try_from(mireds).expect("temperature exceeds u16 mired range")
}