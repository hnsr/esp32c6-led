use crate::color::Rgbw;
use crate::layout::Coord;
use crate::math::random_unit;
use crate::render::RenderContext;

pub trait Effect {
    // fn update(ctx): updates effect state, returns true if we need to re-render
    // fn get_color(ctd, index, coord): gets color for given led
    fn get_color(&mut self, ctx: &RenderContext, index: usize, coord: Coord) -> Rgbw;
}

pub struct RandomColor {}

impl Effect for RandomColor {
    fn get_color(&mut self, ctx: &RenderContext, _index: usize, _coord: Coord) -> Rgbw {
        Rgbw {
            red: random_unit(ctx.rng),
            green: random_unit(ctx.rng),
            blue: random_unit(ctx.rng),
            white: 0.0
        }
    }
}

pub struct PulsatingColor {}

impl Effect for PulsatingColor {
    fn get_color(&mut self, ctx: &RenderContext, _index: usize, coord: Coord) -> Rgbw {
        let phase = ctx.time_s * core::f32::consts::TAU + coord.x;
        let wave = (libm::sinf(phase) + 1.0) * 0.5;
        let base_color = Rgbw {
            red: 0.0,
            green: 0.0,
            blue: 0.0,
            white: 1.0,
        };
        Rgbw {
            red: base_color.red * wave,
            green: base_color.green * wave,
            blue: base_color.blue * wave,
            white: base_color.white * wave,
        }
    }
}