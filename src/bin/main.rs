#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use esp_hal::clock::CpuClock;
use esp_hal::delay::Delay;
use esp_hal::gpio::Level;
use esp_hal::main;
use esp_hal::rmt::{PulseCode, Rmt, TxChannelConfig, TxChannelCreator};
use esp_hal::time::Rate;
use esp_hal::time::Instant;
use esp_hal::rng::Rng;
use esp_println::println;

const WS2812_ZERO: PulseCode = PulseCode::new(Level::High, 24, Level::Low, 76);
const WS2812_ONE: PulseCode = PulseCode::new(Level::High, 48, Level::Low, 52);

const MAX_LEDS: usize = 100;

fn led_frame(red: u8, green: u8, blue: u8, white: u8) -> [PulseCode; 32] {

    let color = ((green as u32) << 24) | ((red as u32) << 16) | ((blue as u32) << 8) | (white as u32);

    let mut pulses = [PulseCode::end_marker(); 32];

    for (bit, pulse) in pulses.iter_mut().enumerate() {
        let mask = 1 << (31 - bit);
        *pulse = if color & mask == 0 {
            WS2812_ZERO
        } else {
            WS2812_ONE
        };
    }

    pulses
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    let delay = Delay::new();

    loop {
        println!("oops!");
        delay.delay_millis(1000);
    }
}

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[derive(Copy, Clone)]
struct Coord {
    x: f32,
    y: f32,
    z: f32
}

#[derive(Copy, Clone)]
struct Rgbw {
    red: f32,
    green: f32,
    blue: f32,
    white: f32
}

struct RenderContext<'a> {
    brightness: f32,
    led_count: usize,
    time_s: f32,
    rng: &'a Rng,
}


fn unit_f32_to_u8(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0) as u8
}

fn random_unit(rng: &Rng) -> f32 {
    (rng.random() >> 8) as f32 / 16_777_215.0
}

fn render<Layout, Shader>(
    ctx: &mut RenderContext,
    buffer: &mut [PulseCode],
    mut layout: Layout,
    mut shader: Shader
)
where
    Layout: FnMut(&mut RenderContext, usize) -> Coord,
    Shader: FnMut(&mut RenderContext, Coord) -> Rgbw,
{
    let led_count = ctx.led_count;

    for index in 0..led_count {
        let coord = layout(ctx, index);
        let rgbw = shader(ctx, coord);

        let pulses = led_frame(
            unit_f32_to_u8(rgbw.red * ctx.brightness),
            unit_f32_to_u8(rgbw.green * ctx.brightness),
            unit_f32_to_u8(rgbw.blue * ctx.brightness),
            unit_f32_to_u8(rgbw.white * ctx.brightness),
        );

        let target = &mut buffer[(index * 32)..((index+1) * 32)];

        target.copy_from_slice(&pulses);
    }

    // Write end marker to buffer
    let marker = PulseCode::end_marker();
    let buffer_end = &mut buffer[(led_count * 32)..(led_count * 32)+1];
    buffer_end[0] = marker;
}

fn coord_identity(ctx: &mut RenderContext, index: usize) -> Coord {
    Coord {
        x: index as f32,
        y: 0.0,
        z: 0.0
    }
}

fn shader_random(ctx: &mut RenderContext, coord: Coord) -> Rgbw {
    Rgbw {
        red: random_unit(ctx.rng),
        green: random_unit(ctx.rng),
        blue: random_unit(ctx.rng),
        white: 0.0
    }
}

fn shader_pulsate(ctx: &mut RenderContext, coord: Coord) -> Rgbw {
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

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[main]
fn main() -> ! {
    // generator version: 1.3.0
    // generator parameters: --chip esp32c6
    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.1.0/examples

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    // The integrated WS2812B-compatible RGB LED is connected to GPIO 8. At an
    // 80 MHz RMT clock, one tick is 12.5 ns; each encoded bit totals 100 ticks.
    let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80)).unwrap();
    let tx_config = TxChannelConfig::default()
        .with_clk_divider(1)
        .with_idle_output_level(Level::Low)
        .with_idle_output(true);
    let mut channel = rmt
        .channel0
        .configure_tx(&tx_config)
        .unwrap()
        .with_pin(peripherals.GPIO8);
    let delay = Delay::new();

    // Buffer holding 32 pulsecodes for each encoded LED color, plus end marker
    let mut buffer = [PulseCode::end_marker(); MAX_LEDS * 32 + 1];

    let rng = Rng::new();

    let mut ctx = RenderContext {
        brightness: 0.1,
        led_count: 90,
        time_s: 0.0,
        rng: &rng,
    };

    loop {
        // fixme: using f32 might not be stable for long uptimes
        ctx.time_s = Instant::now().duration_since_epoch().as_micros() as f32 / 1000000.0;

        // fixme: check buffer bounds
        render(&mut ctx, &mut buffer, coord_identity, shader_pulsate);

        channel = channel.transmit(&buffer[0..ctx.led_count * 32 + 1]).unwrap().wait().unwrap();

        delay.delay_millis(25);
    }
}
