#![no_std]
#![no_main]

use esp_hal::delay::Delay;
use esp_println::println;
use embassy_executor::Spawner;
use embassy_time::Timer;
use esp_hal::interrupt::software::SoftwareInterruptControl;
use esp_hal::timer::timg::TimerGroup;
use esp_radio::ieee802154::Ieee802154;
use zigbee_mac::esp::EspMlme;
use zigbee_mac::mlme::{Mlme, ScanType};
use embassy_embedded_hal::adapter::BlockingAsync;
use esp_storage::FlashStorage;

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

    // Reserve 24 KiB of RAM and register it with the global allocator.
    esp_alloc::heap_allocator!(size: 24 * 1024);

    // Init flash storage with flash peripheral
    let flash = FlashStorage::new(peripherals.FLASH);

    // zigbee-rs expects an async flash interface, so we use the BlockingAsync adapter
    // to provide and async interface
    let flash = BlockingAsync::new(flash);

    // Initialize Zigbee's in-memory state and restore saved values
    let _storage = zigbee::storage::init_with_flash(
        flash,
        // Addresses are byte offsets from the beginning of flash.
        // The upper bound is exclusive, matching the partition table (partitions.csv)
        0x3f_0000..0x3f_4000,
    ).await;

    println!("Zigbee storage initialized");

    // Move ownership of the radio peripheral into its driver.
    let radio = Ieee802154::new(peripherals.IEEE802154);

    // Move the driver into Zigbee's MAC adapter.
    // Network discovery and joining will configure it further later.
    let mac = EspMlme::new(
        radio,
        esp_radio::ieee802154::Config::default(),
    );
    println!("Device IEEE address: {:#018x}", mac.ieee_address());

    println!("Scanning Zigbee channels 11–26...");

    // Scan channels 11-26 (inclusive), listen for duration 5, on each channel.
    match mac.scan_network(ScanType::Active, 11..27, 5).await {
        Ok(result) => {
            println!("Received {} network beacons", result.pan_descriptor.len());

            for network in &result.pan_descriptor {
                println!(
                    "channel={} PAN={:#06x} extended_PAN={:#018x} sender={:?} LQI={} join_open={}",
                    network.channel,
                    network.coord_pan_id.0,
                    network.zigbee_beacon.extended_pan_id.0,
                    network.coord_address,
                    network.link_quality,
                    network.superframe_spec.association_permit,
                );
            }
        }
        Err(error) => {
            println!("Scan failed: {error:?}");
        }
    }

    loop {
        println!("Async runtime is alive");

        Timer::after_secs(1).await;
    }
}