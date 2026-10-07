use embassy_embedded_hal::adapter::BlockingAsync;
use embassy_executor::Spawner;
use embassy_time::{Duration, Ticker};
use embassy_time::Delay as AsyncDelay;
use esp_hal::peripherals::{FLASH, IEEE802154};
use esp_println::println;
use esp_radio::ieee802154::Ieee802154;
use esp_storage::FlashStorage;
use static_cell::StaticCell;
use zigbee::zcl::server::UnsupportedClusterResponder;
use zigbee_mac::esp::EspMlme;
use crate::zigbee::clusters::{Handler, RequestLogger, COLOR_CONTROL, ON_OFF, LEVEL_CONTROL};
use crate::zigbee::clusters::basic::BASIC;
use crate::zigbee::clusters::identify::IDENTIFY;
use crate::zigbee::config::{build_stack_config, INPUT_CLUSTERS};

type ZigbeeFlash = zigbee::storage::FlashStorage<BlockingAsync<FlashStorage<'static>>>;

type ZigbeeStack = zigbee::Stack<'static, EspMlme<'static>, Handler, ZigbeeFlash>;

// Reserve static memory for the stack, initialisation is done later
static STACK: StaticCell<ZigbeeStack> = StaticCell::new();

async fn init_zigbee_storage(flash_peripheral: FLASH<'static>)
    -> zigbee::FlashStorage<BlockingAsync<FlashStorage<'static>>>
{
    // Init flash storage with flash peripheral
    let flash = FlashStorage::new(flash_peripheral);

    // zigbee-rs expects an async flash interface, so we use the BlockingAsync adapter
    // to provide and async interface
    let flash = BlockingAsync::new(flash);

    // Initialize Zigbee's in-memory state and restore saved values
    let storage = zigbee::storage::init_with_flash(
        flash,
        // Addresses are byte offsets from the beginning of flash.
        // The upper bound is exclusive, matching the partition table (partitions.csv)
        0x3f_0000..0x3f_4000,
    ).await;
    storage
}

fn init_radio_mac(ieee802154_peripheral: IEEE802154) -> EspMlme {

    // Move ownership of the radio peripheral into its driver.
    let radio = Ieee802154::new(ieee802154_peripheral);

    // Move the driver into Zigbee's MAC adapter.
    // Network discovery and joining will configure it further later.
    let mac = EspMlme::new(
        radio,
        esp_radio::ieee802154::Config::default(),
    );
    println!("Device IEEE address: {:#018x}", mac.ieee_address());

    mac
}

pub async fn start_zigbee(
    spawner: Spawner,
    ieee802154_peripheral: IEEE802154<'static>, // fixme: why this lifetime annotation?
    flash_peripheral: FLASH<'static>
) {
    // The zigbee library tries tuple handlers from left to right, so the fallback goes last
    let handler = (
        RequestLogger,
        BASIC,
        &IDENTIFY,
        (
            &ON_OFF,
            &LEVEL_CONTROL,
            &COLOR_CONTROL
        ),
        UnsupportedClusterResponder::new(&INPUT_CLUSTERS),
    );

    let mac = init_radio_mac(ieee802154_peripheral);
    let storage = init_zigbee_storage(flash_peripheral).await;
    let stack_config = build_stack_config();

    // Transfer ownership of the MAC, configuration, handlers, and storage
    let stack: &'static ZigbeeStack = STACK.init(
        zigbee::Stack::new(mac, stack_config, handler, storage),
    );

    println!("Zigbee stack constructed for channel {}", stack.config().channel());

    spawner.spawn(
        stack_task(stack).expect("Could not allocate Zigbee task"),
    );

    spawner.spawn(
        maintenance_task(stack).expect("Zigbee maintenance task slot unavailable"),
    );
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
    // fixme: what if the zigbee stack stops? should we implement a check here?
    // Use a ticker for a fixed cadence, instead of just adding a delay
    let mut ticker = Ticker::every(Duration::from_secs(1));
    let mut ticks_since_log = 0_u8;

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
