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
use esp_hal::main;
use esp_hal::time::Instant;
use esp_hal::rng::Rng;
use esp_println::println;
use esp32c6_led::color::Rgbw;
use esp32c6_led::driver::Ws2812RmtDriver;
use esp32c6_led::effect::PulsatingColor;
use esp32c6_led::layout::Linear;
use esp32c6_led::render::{render, RenderContext};

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("PANIC: {info}");
    let delay = Delay::new();
    loop {
        delay.delay_millis(1000);
    }
}

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();



#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[main]
fn main() -> ! {
    // generator version: 1.3.0
    // generator parameters: --chip esp32c6
    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.1.0/examples
    println!("Initialising HAL.");

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    let delay = Delay::new();
    let rng = Rng::new();

    println!("Instantiating layout, effect and driver.");

    let mut layout = Linear {};
    let mut effect = PulsatingColor {};
    let mut driver = Ws2812RmtDriver::new(peripherals);
    let mut ctx = RenderContext::new(&rng, 20, Rgbw {
        red: 1.0,
        green: 0.0,
        blue: 0.0,
        white: 1.0,
    }, 0.1);

    println!("Starting rendering loop.");

    loop {
        // fixme: using f32 might not be stable for long uptimes
        ctx.time_s = Instant::now().duration_since_epoch().as_micros() as f32 / 1000000.0;

        render(&mut ctx, &mut layout, &mut effect, &mut driver);

        delay.delay_millis(25);
    }
}
