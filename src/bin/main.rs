#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use embassy_executor::Spawner;
use embassy_time::{Duration, Ticker};
use esp_hal::clock::CpuClock;
use esp_hal::delay::Delay;
use esp_hal::interrupt::software::SoftwareInterruptControl;
use esp_hal::rng::Rng;
use esp_hal::time::Instant;
use esp_hal::timer::timg::TimerGroup;
use esp_println::println;
use esp32c6_led::driver::AddressableLedRmtDriver;
use esp32c6_led::effect::PulsatingColor;
use esp32c6_led::hw::{DiodeLayout, HardwareConfig, HardwareKind, LedChip, TemperatureRange};
use esp32c6_led::layout::Linear;
use esp32c6_led::render::{RenderContext, render};
use esp32c6_led::zigbee::lamp::LampCapabilities;
use esp32c6_led::zigbee::start_zigbee;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("PANIC: {info}");
    let delay = Delay::new();
    loop {
        delay.delay_millis(10000);
    }
}

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    // Setup timer and interrupts, and start the scheduler
    let software_interrupts = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    let timers = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timers.timer0, software_interrupts.software_interrupt0);

    // Routes log messages from libraries to the serial output.
    esp_println::logger::init_logger_from_env();

    // Reserve 24 KiB of RAM and register it with the global allocator.
    esp_alloc::heap_allocator!(size: 24 * 1024);

    let rng = Rng::new();

    println!("Instantiating layout, effect and driver");

    let mut layout = Linear {};
    let mut effect = PulsatingColor {};

    let config = HardwareConfig {
        kind: HardwareKind::AddressableStrip {
            chip: LedChip::Sk6812,
        },
        diodes: DiodeLayout::Rgbw {
            physical_temperature: 2700,
            emulated_temperature: Option::from(TemperatureRange {
                min_kelvin: 2200,
                max_kelvin: 6500,
            }),
        },
        led_count: 10,
    };

    // fixme: pick driver based on config
    // fixme: make GPIO8 variable depending on LED HW config
    let mut driver = AddressableLedRmtDriver::new(peripherals.RMT, peripherals.GPIO8);
    let mut ctx = RenderContext::new(&rng, config.led_count);

    // todo: apply LED HW limits and capabilities based on diodes and emulataion config.
    let lamp_capabilities = LampCapabilities::new(
        true,
        true,
        153,
        500
    );

    let lamp_state = start_zigbee(
        spawner,
        peripherals.IEEE802154,
        peripherals.FLASH,
        lamp_capabilities
    ).await;

    println!("Starting rendering loop");

    // Set up a ticker for 40 hz
    let mut ticker = Ticker::every(Duration::from_millis(25));

    // Uncomment this and the other timing lines to measure render loop performance
    // let mut timing = LoopTimings::<400>::new("renderer");

    loop {
        // timing.begin();

        // fixme: using f32 might not be stable for long uptimes
        ctx.time_s = Instant::now().duration_since_epoch().as_micros() as f32 / 1_000_000.0;

        render(&mut ctx, &mut layout, &mut effect, &mut driver, Some(lamp_state));

        // timing.end_work();
        // if timing.report_if_ready() {
        //     ticker.reset();
        // }

        // Suspend render loop until next tick, allowing zigbee task to run
        ticker.next().await;

        // fixme: detect when rendering takes too long, it needs to allow zigbee task to handle
        //   requests in a reasonable time and before the mac receive queue fills (mac
        //   frame acknowledgements are handled through interruption)
    }
}
