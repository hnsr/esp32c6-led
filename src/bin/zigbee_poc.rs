#![no_std]
#![no_main]

use esp_hal::delay::Delay;
use esp_println::println;
use embassy_executor::Spawner;
use embassy_time::Timer;
use esp_hal::interrupt::software::SoftwareInterruptControl;
use esp_hal::timer::timg::TimerGroup;

esp_bootloader_esp_idf::esp_app_desc!();

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("PANIC: {info}");
    let delay = Delay::new();
    loop {
        delay.delay_millis(1000);
    }
}

#[esp_rtos::main]
async fn main(_spawner: Spawner) -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    // Move ownership of these hardware peripherals into their drivers.
    let software_interrupts =
        SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    let timers = TimerGroup::new(peripherals.TIMG0);

    // Give the scheduler a timer and a software interrupt.
    esp_rtos::start(
        timers.timer0,
        software_interrupts.software_interrupt0,
    );

    // Routes log messages from libraries to the serial output.
    esp_println::logger::init_logger_from_env();

    loop {
        println!("Async runtime is alive");

        Timer::after_secs(1).await;
    }
}