use core::cell::Cell;
use crate::hw::{DiodeLayout, HardwareConfig, TemperatureRange};

// Approximate X/Y values for (s)RGB white, which we want to use as the default
const XY_SRGB_WHITE_X: u16 = 0x500D;
const XY_SRGB_WHITE_Y: u16 = 0x5439;

// Temperature (in mireds) for warm white (2700K) and cool white (6500K)
const TEMPERATURE_MIREDS_2700K: u16 = 370; // Warm

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LampMode {
    ColorXy,
    WhiteTemperature,
    WhiteFixed
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LampState {
    pub on: bool,
    pub mode: LampMode,

    pub(super) level: u8,

    // Color in XY
    pub(super) x: u16,
    pub(super) y: u16,

    // Temperature in mireds
    pub(super) temperature: u16,

    pub(super) capabilities: LampCapabilities,
}

impl LampState {
    // Create initial state using some defaults, but constrained to lamp capabilities
    // todo: at some point we want to load lamp state from flash, but it must still be constrained
    //       to capabilities.
    pub fn new(capabilities: LampCapabilities) -> Self {
        let temperature = match capabilities.supports_white {
            WhiteCapability::None => 0,
            WhiteCapability::Fixed { temperature_mireds } => temperature_mireds,
            WhiteCapability::Range { temperature_min_mireds, temperature_max_mireds } => {
                TEMPERATURE_MIREDS_2700K.clamp(
                    temperature_min_mireds,
                    temperature_max_mireds,
                )
            }
        };

        let mode = if capabilities.supports_xy {
            LampMode::ColorXy
        } else {
            match capabilities.supports_white {
                // fixme: a lamp that does not support_xy but also no whites should not exist, how
                //        can we represent that more sensibly? Maybe just panic here and validate
                //        in config?
                WhiteCapability::None => LampMode::ColorXy,
                WhiteCapability::Fixed { .. } => LampMode::WhiteFixed,
                WhiteCapability::Range { .. } => LampMode::WhiteTemperature
            }
        };

        Self {
            capabilities,
            on: true,
            level: u8::MAX / 2,
            mode,
            x: XY_SRGB_WHITE_X,
            y: XY_SRGB_WHITE_Y,
            temperature,
        }
    }

    pub fn get_level(&self) -> f32 {
        f32::from(self.level) / 254.0
    }

    pub fn get_x(&self) -> f32 {
        f32::from(self.x) / 65_536.0
    }

    pub fn get_y(&self) -> f32 {
        f32::from(self.y) / 65_536.0
    }

    /// Returns current lamp temperature in mireds
    pub fn get_temperature_mireds(&self) -> f32 {
        f32::from(self.temperature)
    }

    /// Returns current lamp temperature, normalized (0 for coldest, 1 for warmest)
    pub fn get_temperature_normalized(&self) -> f32 {
        match self.capabilities.supports_white {
            WhiteCapability::None =>
                unreachable!("temperature not supported"),
            WhiteCapability::Fixed { .. } =>
                unreachable!("temperature not supported"),
            WhiteCapability::Range {
                temperature_min_mireds,
                temperature_max_mireds
            } => {
                let mireds = f32::from(self.temperature);
                let min = f32::from(temperature_min_mireds);
                let max = f32::from(temperature_max_mireds);
                (mireds - min) / (max - min)
            }
        }
    }
}

pub type SharedLampState = Cell<LampState>;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LampCapabilities {
    pub(super) supports_xy: bool,
    pub(super) supports_white: WhiteCapability,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WhiteCapability {
    None,

    // Emits white light at a fixed temperature
    Fixed {
        temperature_mireds: u16,
    },

    // Emits white light and allows temperature selection
    Range {
        temperature_min_mireds: u16,
        temperature_max_mireds: u16,
    },
}

impl LampCapabilities {
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
                physical_temperature_mireds: physical_temperature,
            } => {
                // One white emitter so only one native temperature
                Some(TemperatureRange {
                    min_mireds: physical_temperature,
                    max_mireds: physical_temperature,
                })
            }

            DiodeLayout::WhiteCct {
                physical_temperature_mireds: physical_temperature,
            } => Some(physical_temperature),

            DiodeLayout::Rgb {
                emulated_temperature_mireds: emulated_temperature,
            } => {
                // RGB has no native white temperature so this is either the configured emulation
                // range or None
                emulated_temperature
            }

            DiodeLayout::Rgbw {
                physical_temperature_mireds: physical_temperature,
                emulated_temperature_mireds: emulated_temperature,
            } => {
                let native_range = TemperatureRange {
                    min_mireds: physical_temperature,
                    max_mireds: physical_temperature,
                };

                // Use the emulation range when present, otherwise pass on native temperature as a
                // range
                Some(emulated_temperature.unwrap_or(native_range))
            }

            DiodeLayout::RgbCct {
                physical_temperature_mireds: physical_temperature,
            } => Some(physical_temperature),
        };

        let white_capability =
            match temperature_range {
                Some(range) => {
                    assert!(range.min_mireds > 0);
                    assert!(range.min_mireds <= range.max_mireds);

                    let min = range.min_mireds;
                    let max = range.max_mireds;
                    if min < max {
                        WhiteCapability::Range {
                            temperature_min_mireds: min,
                            temperature_max_mireds: max
                        }
                    } else {
                        WhiteCapability::Fixed { temperature_mireds: min }
                    }
                }
                // Temperature control is unavailable
                None => WhiteCapability::None,
            };

        Self {
            supports_xy,
            supports_white: white_capability
        }
    }
}
