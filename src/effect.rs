use crate::color::Rgb;
use crate::layout::Coord;
use crate::math::random_unit;
use crate::render::RenderContext;

pub struct TemperatureSample {
    /// 0.0 = coldest supported white, 1.0 = warmest
    pub temperature: f32,
    pub intensity: f32,
}

pub trait Effect {
    // fn update(ctx): updates effect state, returns true if we need to re-render
    // fn get_color(ctd, index, coord): gets color for given led
    fn get_color(
        &mut self,
        ctx: &RenderContext,
        base_color: Rgb,
        index: usize,
        coord: Coord
    ) -> Rgb;

    fn get_temperature(
        &mut self,
        ctx: &RenderContext,
        base_temperature: f32,
        index: usize,
        coord: Coord,
    ) -> TemperatureSample;

    fn get_intensity(
        &mut self,
        ctx: &RenderContext,
        index: usize,
        coord: Coord,
    ) -> f32;
}

// todo: move to separate per-effect modules

pub struct RandomColor {}

impl Effect for RandomColor {
    fn get_color(
        &mut self,
        ctx: &RenderContext,
        _base_color: Rgb,
        _index: usize,
        _coord: Coord
    ) -> Rgb {
        Rgb {
            red: random_unit(ctx.rng),
            green: random_unit(ctx.rng),
            blue: random_unit(ctx.rng),
        }
    }

    fn get_temperature(
        &mut self,
        ctx: &RenderContext,
        _base_temperature: f32,
        _index: usize,
        _coord: Coord
    ) -> TemperatureSample {
        TemperatureSample {
            temperature: random_unit(ctx.rng),
            intensity: random_unit(ctx.rng),
        }
    }

    fn get_intensity(&mut self, ctx: &RenderContext, _index: usize, _coord: Coord) -> f32 {
        random_unit(ctx.rng)
    }
}

pub struct PulsatingColor {}

impl Effect for PulsatingColor {
    fn get_color(
        &mut self,
        ctx: &RenderContext,
        base_color: Rgb,
        _index: usize,
        coord: Coord
    ) -> Rgb {
        let phase = ctx.time_s * core::f32::consts::TAU + coord.x;
        let wave = (libm::sinf(phase) + 1.0) * 0.5;
        Rgb {
            red: base_color.red * wave,
            green: base_color.green * wave,
            blue: base_color.blue * wave
        }
    }

    fn get_temperature(
        &mut self,
        ctx: &RenderContext,
        _base_temperature: f32,
        _index: usize,
        coord: Coord
    ) -> TemperatureSample {
        let phase = ctx.time_s * core::f32::consts::TAU + coord.x;
        let wave = (libm::sinf(phase) + 1.0) * 0.5;
        TemperatureSample {
            temperature: wave,
            intensity: 0.5 + wave / 2.0,
        }
    }

    fn get_intensity(
        &mut self,
        ctx: &RenderContext,
        _index: usize,
        coord: Coord
    ) -> f32 {
        let phase = ctx.time_s * core::f32::consts::TAU + coord.x;
        let wave = (libm::sinf(phase) + 1.0) * 0.5;
        wave
    }
}
