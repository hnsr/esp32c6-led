use crate::zigbee::clusters::{Handler, IDENTIFY, build_handler};
use crate::zigbee::config::build_stack_config;
use core::ops::Range;
use embassy_embedded_hal::adapter::BlockingAsync;
use embassy_executor::Spawner;
use embassy_time::Delay as AsyncDelay;
use embassy_time::{Duration, Ticker};
use esp_hal::peripherals::{FLASH, IEEE802154};
use esp_println::println;
use esp_radio::ieee802154::Ieee802154;
use esp_storage::FlashStorage;
use static_cell::StaticCell;
use zigbee_mac::esp::EspMlme;

// Matches the `zigbee` partition in partitions.csv:
// offset 0x3f0000, size 0x4000 (16 KiB)
const ZIGBEE_FLASH_RANGE: Range<u32> = 0x3f_0000..0x3f_4000;

type ZigbeeFlash = zigbee::storage::FlashStorage<BlockingAsync<FlashStorage<'static>>>;

type ZigbeeStack = zigbee::Stack<'static, EspMlme<'static>, Handler, ZigbeeFlash>;

// Reserve static memory for the stack, initialization is done later
static STACK: StaticCell<ZigbeeStack> = StaticCell::new();

async fn init_zigbee_storage(flash_peripheral: FLASH<'static>) -> ZigbeeFlash {
    // Init flash storage with flash peripheral
    let flash = FlashStorage::new(flash_peripheral);

    // zigbee-rs expects an async flash interface, so we use the BlockingAsync adapter
    // to provide and async interface
    let flash = BlockingAsync::new(flash);

    // Initialize and return Zigbee's in-memory state and restore saved values
    zigbee::storage::init_with_flash(flash, ZIGBEE_FLASH_RANGE).await
}

fn init_radio_mac(ieee802154_peripheral: IEEE802154<'_>) -> EspMlme<'_> {
    // Move ownership of the radio peripheral into its driver.
    let radio = Ieee802154::new(ieee802154_peripheral);

    // Move the driver into Zigbee's MAC adapter.
    // Network discovery and joining will configure it further later.
    let mac = EspMlme::new(radio, esp_radio::ieee802154::Config::default());
    println!("Device IEEE address: {:#018x}", mac.ieee_address());

    mac
}

pub async fn start_zigbee(
    spawner: Spawner,
    ieee802154_peripheral: IEEE802154<'static>,
    flash_peripheral: FLASH<'static>,
) {
    // fixme: replace fixed network ID with proper reset + join network logic
    let network_id_text = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/zigbee-network.txt",));
    let extended_pan_id = u64::from_str_radix(network_id_text.trim(), 16)
        .expect("zigbee-network.txt must contain a hexadecimal extended PAN ID");

    let handler = build_handler();
    let mac = init_radio_mac(ieee802154_peripheral);
    let storage = init_zigbee_storage(flash_peripheral).await;
    let stack_config = build_stack_config(extended_pan_id);

    println!(
        "Zigbee configuration: extended_PAN={:#018x}, channel={}, capabilities={:#04x}",
        stack_config.network().extended_pan_id.0,
        stack_config.network().channels.start,
        stack_config.device().capability_information.0,
    );
    println!(
        "Zigbee descriptors configured: {} application endpoint(s)",
        stack_config.descriptors().endpoints.len(),
    );

    // Transfer ownership of the MAC, configuration, handlers, and storage
    let stack: &'static ZigbeeStack =
        STACK.init(zigbee::Stack::new(mac, stack_config, handler, storage));

    println!(
        "Zigbee stack constructed for channel {}",
        stack.config().channel()
    );

    spawner.spawn(stack_task(stack).expect("Zigbee task slot unavailable"));
    spawner.spawn(maintenance_task(stack).expect("Zigbee maintenance task slot unavailable"));
}

#[embassy_executor::task]
async fn stack_task(stack: &'static ZigbeeStack) {
    println!("Starting Zigbee commissioning...");

    // Drive commissioning, incoming frames, persistence, and keepalive.
    // During normal operation this remains running indefinitely.
    let outcome = stack.run(AsyncDelay).await;

    // If it returns, print the reason and retain it for diagnosis.
    println!("Zigbee stack stopped: {outcome:?}");
}

#[embassy_executor::task]
async fn maintenance_task(stack: &'static ZigbeeStack) {
    // Use a ticker for a fixed cadence, instead of just adding a delay
    let mut ticker = Ticker::every(Duration::from_secs(1));
    let mut ticks_since_log = 0_u8;

    // fixme: what if the zigbee stack stops? should we implement a check here to break out of
    //        the loop?
    loop {
        ticker.next().await;

        IDENTIFY.tick(1);

        ticks_since_log += 1;

        if ticks_since_log == 10 {
            ticks_since_log = 0;

            let nlme = stack.device().nlme();

            println!(
                "Zigbee keepalive={:?}, interval_ms={:?}, identify_s={}",
                nlme.keepalive_method(),
                nlme.keepalive_interval_ms(),
                IDENTIFY.identify_time(),
            );
        }
    }
}
