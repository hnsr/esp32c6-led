#![no_std]
#![no_main]

use esp_hal::delay::Delay;
use esp_println::println;
use embassy_executor::Spawner;
// use embassy_time::Timer;
use esp_hal::interrupt::software::SoftwareInterruptControl;
use esp_hal::timer::timg::TimerGroup;
// use esp_radio::ieee802154::Ieee802154;
// use zigbee_mac::esp::EspMlme;
//use zigbee_mac::mlme::{Mlme, ScanType};
// use embassy_embedded_hal::adapter::BlockingAsync;
// use esp_storage::FlashStorage;
// use zigbee::{DeviceConfig, LogicalType, NetworkConfig};
// use zigbee::nwk::nib::CapabilityInformation;
// use zigbee::types::IeeeAddress;
// use zigbee::zdo::config::DiscoveryType;
// use zigbee::{
//     CurrentPowerMode, CurrentPowerSourceLevel, PowerSource,
//     StackConfig, TimingConfig,
// };
// use zigbee::zcl::{profile, clusters::general::{basic, identify}};
// use zigbee::zdo::descriptor::{
//     DeviceDescriptorConfig, EndpointDescriptor,
//     NodeDescriptorConfig, PowerDescriptorConfig,
// };
// use static_cell::StaticCell;
// use zigbee::zcl::clusters::general::basic::BasicServer;
// use zigbee::zcl::clusters::general::identify::IdentifyServer;
// use zigbee::zcl::server::UnsupportedClusterResponder;
// use embassy_time::Delay as AsyncDelay;
// use core::sync::atomic::{AtomicBool, Ordering};
// use zigbee::zcl::frame::Status;
// use zigbee::zcl::server::{
//     ClusterCommand, ClusterServer, CommandOutcome,
// };
// use zigbee::zcl::types::{
//     AttrInfo, Attribute, AttributeId, Bool, Cluster, ClusterId,
// };
// use core::sync::atomic::AtomicU8;
// use zigbee::zcl::types::Uint8;
// use core::sync::atomic::AtomicU16;
// use zigbee::zcl::types::{
//     Bitmap8, Bitmap16, Enum8, ReadWrite, TypeId, Uint16,
//     ZclBitmap8, ZclBitmap16, ZclEnum8,
// };
use esp32c6_led::zigbee::runtime::start_zigbee;

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
async fn main(spawner: Spawner) {
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

    start_zigbee(spawner, peripherals.IEEE802154, peripherals.FLASH).await
}
