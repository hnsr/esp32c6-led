use crate::color::Rgbw;
use crate::driver::Driver;
use crate::effect::Effect;
use crate::layout::Layout;
use esp_hal::rng::Rng;

pub struct RenderContext<'a> {
    pub base_color: Rgbw,
    pub brightness: f32,
    pub led_count: usize,
    pub time_s: f32,
    pub rng: &'a Rng,
}

impl<'a> RenderContext<'a> {
    pub fn new(rng: &'a Rng, led_count: usize, base_color: Rgbw, brightness: f32) -> Self {
        Self {
            base_color,
            brightness,
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
) {
    driver.begin_frame();

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
