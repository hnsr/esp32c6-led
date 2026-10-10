use crate::color::Rgbw;
use crate::driver::Driver;
use crate::effect::Effect;
use crate::layout::Layout;
use esp_hal::rng::Rng;
use crate::color;
use crate::lamp::{ColorMode, SharedLamp};

pub struct RenderContext<'a> {
    pub base_color: Rgbw,
    pub brightness: f32,
    pub led_count: usize,
    pub time_s: f32,
    pub rng: &'a Rng,
}

impl<'a> RenderContext<'a> {
    pub fn new(rng: &'a Rng, led_count: usize) -> Self {
        Self {
            base_color: Rgbw {
                red: 0.0,
                green: 0.0,
                blue: 0.0,
                white: 0.0,
            },
            brightness: 0.0,
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
    lamp: &SharedLamp
) {
    driver.begin_frame();

    // Get snapshot of lamp state, and apply it to the render context base_color and brightness
    let lamp = lamp.get();

    // fixme: we are converting Zigbee representations here, which does not belong in the render
    //   loop, we probably need to rethink LampState
    ctx.brightness = f32::from(lamp.brightness) / 254.0;
    ctx.base_color = match lamp.color_mode {
        // If lamp is in Xy color mode, we map to RGB and leave the white channel empty
        ColorMode::Xy => {
            let x: f32 = f32::from(lamp.x) / 65_536.0;
            let y: f32 = f32::from(lamp.y) / 65_536.0;
            // Conversion can fail on invalid values
            color::convert_xy_to_rgbw(x, y).unwrap_or(ctx.base_color)
        },
        // If lamp is in temperature mode, we map primarily to W, but we use RGB to emulate temperature
        ColorMode::Temperature => {
            let mireds: f32 = f32::from(lamp.temperature);
            color::convert_mireds_to_rgbw(mireds)
        }
    };
    if !lamp.on {
        // todo: we should probably also skip rendering, but we need to render at lest one frame
        //       to switch off the LEDs
        ctx.brightness = 0.0;
    }

    for index in 0..ctx.led_count {
        let coord = layout.get_coord(index);
        let mut color = effect.get_color(ctx, index, coord);

        color.red = color.red * ctx.brightness;
        color.green = color.green * ctx.brightness;
        color.blue = color.blue * ctx.brightness;
        color.white = color.white * ctx.brightness;

        driver.write_led(index, color);
    }

    driver.end_frame();
}
