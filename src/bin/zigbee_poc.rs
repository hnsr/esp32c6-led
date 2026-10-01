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
use zigbee::{DeviceConfig, LogicalType, NetworkConfig};
use zigbee::nwk::nib::CapabilityInformation;
use zigbee::types::IeeeAddress;
use zigbee::zdo::config::DiscoveryType;


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

    let network_id_text = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/zigbee-network.txt",
    ));
    let extended_pan_id = u64::from_str_radix(network_id_text.trim(), 16)
        .expect("zigbee-network.txt must contain a hexadecimal extended PAN ID");

    let network_config = NetworkConfig {
        // Hardcoded address of my Hue network
        extended_pan_id: IeeeAddress(extended_pan_id),
        channels: 25..26,
        scan_duration: 5,
    };

    let device_config = DeviceConfig {
        logical_type: LogicalType::EndDevice,

        // The capability byte sent during joining:
        // bit 2: externally powered / mains-powered
        // bit 3: receiver remains enabled when idle
        // bit 7: request an allocated short network address
        // bit 1 stays clear because we are an end device.
        capability_information: CapabilityInformation(
            (1 << 2) | (1 << 3) | (1 << 7),
        ),

        // Address-discovery policy: request a device's IEEE address
        // when its short network address is already known
        discovery_type: DiscoveryType::IEEE,

        // Keep the Trust Center link-key exchange enabled
        tc_link_key_exchange: true,
    };

    println!(
        "Zigbee configuration: extended_PAN={:#018x}, channel={}, capabilities={:#04x}",
        network_config.extended_pan_id.0,
        network_config.channels.start,
        device_config.capability_information.0,
    );



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