
#[derive(Clone, Copy, Debug)]
pub struct HardwareConfig {
    pub kind: HardwareKind,
    pub diodes: DiodeLayout,
    pub led_count: usize,
}

#[derive(Clone, Copy, Debug)]
pub enum HardwareKind {
    AddressableStrip {
        chip: LedChip,
    }
}

#[derive(Clone, Copy, Debug)]
pub enum LedChip {
    // todo: Implement correct timings for Ws2812*
    // Ws2812,
    // Ws2812b,
    Sk6812,
}

#[derive(Clone, Copy, Debug)]
pub enum DiodeLayout {
    White {
        physical_temperature_kelvin: u16,
    },
    WhiteCct {
        physical_temperature_kelvin: TemperatureRange
    },
    Rgb {
        emulated_temperature_kelvin: Option<TemperatureRange>,
    },
    Rgbw {
        physical_temperature_kelvin: u16,
        emulated_temperature_kelvin: Option<TemperatureRange>,
    },
    RgbCct {
        physical_temperature_kelvin: TemperatureRange
    },
}

#[derive(Clone, Copy, Debug)]
pub struct TemperatureRange {
    pub min_kelvin: u16,
    pub max_kelvin: u16,
}