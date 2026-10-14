
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
    // Ws2812,
    // Ws2812b,
    Sk6812,
}

#[derive(Clone, Copy, Debug)]
pub enum DiodeLayout {
    White {
        physical_temperature: u16,
    },
    WhiteCct {
        physical_temperature: TemperatureRange
    },
    Rgb {
        emulated_temperature: Option<TemperatureRange>,
    },
    Rgbw {
        physical_temperature: u16,
        emulated_temperature: Option<TemperatureRange>,
    },
    RgbCct {
        physical_temperature: TemperatureRange
    },
}

#[derive(Clone, Copy, Debug)]
pub struct TemperatureRange {
    pub min_kelvin: u16,
    pub max_kelvin: u16,
}