use core::cell::Cell;

// X/Y values for sRGB white, which we want to use as the default
const XY_SRGB_WHITE_X: u16 = 0x500D;
const XY_SRGB_WHITE_Y: u16 = 0x5439;

// Temperature (in mireds) for warm white (2700K)
const TEMPERATURE_MIREDS_WARM: u16 = 370;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorMode {
    Xy,
    Temperature
}

// The lamp state matches the Zigbee capabilities and representation (XY, temperature in mireds),
// but not encoding which stays in the Zigbee cluster implementation
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LampState {
    pub on: bool,

    pub brightness: u8,

    pub color_mode: ColorMode,

    // Color in XY
    pub x: u16,
    pub y: u16,

    // Temperature in mireds
    pub temperature: u16
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

pub type SharedLamp = Cell<LampState>;