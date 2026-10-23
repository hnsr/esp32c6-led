use crate::color::Rgb;
use crate::driver::Driver;
use crate::effect::Effect;
use crate::layout::Layout;
use esp_hal::rng::Rng;
use crate::color;
use crate::zigbee::lamp::{LampMode, SharedLampState};

pub struct RenderContext<'a> {
    pub led_count: usize,
    pub time_s: f32,
    pub rng: &'a Rng,
}

impl<'a> RenderContext<'a> {
    pub fn new(rng: &'a Rng, led_count: usize) -> Self {
        Self {
            led_count,
            time_s: 0.0,
            rng,
        }
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

    // fixme: rework this a little bit, if we don't have a LampState, we need to fallback to some
    //        'default' color/temperature/level. Is HardwareConfig the right place for that?
    if let Some(lamp) = lamp {

        let lamp = lamp.get();

        // fixme: we could just skip rendering if lamp is off, but we need (depending on driver?)
        //        output at least one frame to turn off the LEDs (depending on HardWare/Driver, so
        //        render loop should not really need to know about this and we should abstract that
        //        away somehow)
        let level = if lamp.on { lamp.get_level() } else { 0.0 };

        match lamp.mode {
            LampMode::ColorXy => {
                let rgb = color::convert_xy_to_rgb(lamp.get_x(), lamp.get_y())
                    // fixme: how to handle None here? We could fallback to hardware default color?
                    .unwrap_or(Rgb {
                        red: 0.0,
                        green: 0.0,
                        blue: 0.0,
                    });

                for index in 0..ctx.led_count {
                    let coord = layout.get_coord(index);
                    let mut color = effect.get_color(ctx, rgb, index, coord);

                    color.red = color.red * level;
                    color.green = color.green * level;
                    color.blue = color.blue * level;

                    driver.write_color(index, color);
                }
            }
            LampMode::WhiteTemperature => {
                let temperature = lamp.get_temperature_normalized();

                for index in 0..ctx.led_count {
                    let coord = layout.get_coord(index);
                    let sample = effect.get_temperature(ctx, temperature, index, coord);

                    driver.write_temperature(
                        index,
                        sample.temperature,
                        sample.intensity * level
                    );
                }
            }
            LampMode::WhiteFixed => {
                for index in 0..ctx.led_count {
                    let coord = layout.get_coord(index);
                    let intensity = effect.get_intensity(ctx, index, coord);

                    driver.write_intensity(index, intensity * level);
                }
            }
        }
    }

    driver.end_frame();
}
