use esp_hal::rng::Rng;

pub fn random_unit(rng: &Rng) -> f32 {
    (rng.random() >> 8) as f32 / 16_777_215.0
}

pub fn unit_f32_to_u8(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0) as u8
}