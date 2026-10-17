use crate::color::Rgbw;
use crate::driver::Driver;
use crate::effect::Effect;
use crate::layout::Layout;
use esp_hal::rng::Rng;
use crate::color;
use crate::zigbee::lamp::{ColorMode, SharedLampState};

pub struct RenderContext<'a> {
    pub base_color: Rgbw,
    pub base_level: f32,
    pub led_count: usize,
    pub time_s: f32,
    pub rng: &'a Rng,
}

impl<'a> RenderContext<'a> {
    pub fn new(rng: &'a Rng, led_count: usize) -> Self {
        Self {
            base_color: Rgbw {
                red: 1.0,
                green: 1.0,
                blue: 1.0,
                white: 0.0,
            },
            base_level: 0.05,
            led_count,
            time_s: 0.0,
            rng,
        }
    }
}

fn apply_lamp_state(ctx: &mut RenderContext, lamp: &SharedLampState) {
    // Get snapshot of lamp state, and apply it to the render context base_color and brightness
    let lamp = lamp.get();

    // Get effective LED Rgbw, fallback to current value if conversion fails.
    ctx.base_color = match lamp.color_mode {
        ColorMode::Xy => {
            color::convert_xy_to_rgbw(lamp.get_x(), lamp.get_y()).unwrap_or(ctx.base_color)
        },
        // fixme: conversion depends on LED HW config, belongs in driver actually. Should render
        //        context be aware of, and implement color/temperature modes?
        ColorMode::Temperature => color::convert_mireds_to_rgbw(lamp.get_temperature())
    };
    ctx.base_level = lamp.get_level();

    if !lamp.on {
        // todo: we should probably also skip rendering, but we need to render at lest one frame
        //       to switch off the LEDs
        ctx.base_level = 0.0;
    }
}

pub fn render(
    ctx: &mut RenderContext,
    layout: &mut dyn Layout,
    effect: &mut dyn Effect,
    driver: &mut dyn Driver,
    lamp: Option<&SharedLampState>
) {
    driver.begin_frame();

    if let Some(lamp) = lamp {
        apply_lamp_state(ctx, lamp);
    }

    for index in 0..ctx.led_count {
        let coord = layout.get_coord(index);
        let mut color = effect.get_color(ctx, index, coord);

        color.red = color.red * ctx.base_level;
        color.green = color.green * ctx.base_level;
        color.blue = color.blue * ctx.base_level;
        color.white = color.white * ctx.base_level;

        driver.write_led(index, color);
    }

    driver.end_frame();
}
