
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
        physical_temperature_mireds: u16,
    },
    WhiteCct {
        physical_temperature_mireds: TemperatureRange
    },
    Rgb {
        emulated_temperature_mireds: Option<TemperatureRange>,
    },
    Rgbw {
        physical_temperature_mireds: u16,
        emulated_temperature_mireds: Option<TemperatureRange>,
    },
    RgbCct {
        physical_temperature_mireds: TemperatureRange
    },
}

#[derive(Clone, Copy, Debug)]
pub struct TemperatureRange {
    pub min_mireds: u16,
    pub max_mireds: u16,
}