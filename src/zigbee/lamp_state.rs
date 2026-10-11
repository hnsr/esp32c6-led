use core::cell::Cell;

// Approximate X/Y values for (s)RGB white, which we want to use as the default
const XY_SRGB_WHITE_X: u16 = 0x500D;
const XY_SRGB_WHITE_Y: u16 = 0x5439;

// Temperature (in mireds) for warm white (2700K), to use as a default
const TEMPERATURE_MIREDS_WARM: u16 = 370;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorMode {
    Xy,
    Temperature
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LampState {
    pub on: bool,
    pub color_mode: ColorMode,

    pub(super) brightness: u8,

    // Color in XY
    pub(super) x: u16,
    pub(super) y: u16,

    // Temperature in mireds
    pub(super) temperature: u16
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

impl Default for LampState {
    fn default() -> Self {
        Self {
            on: true,
            brightness: u8::MAX,
            color_mode: ColorMode::Xy,
            x: XY_SRGB_WHITE_X,
            y: XY_SRGB_WHITE_Y,
            temperature: TEMPERATURE_MIREDS_WARM,
        }
    }
}

pub type SharedLampState = Cell<LampState>;