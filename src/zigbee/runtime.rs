use crate::zigbee::clusters::{Handler, IDENTIFY, build_handler};
use crate::zigbee::config::build_stack_config;
use core::ops::Range;
use embassy_embedded_hal::adapter::BlockingAsync;
use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::Delay as AsyncDelay;
use embassy_time::{Duration, Ticker};
use esp_hal::peripherals::{FLASH, IEEE802154};
use esp_radio::ieee802154::Ieee802154;
use esp_storage::FlashStorage;
use static_cell::StaticCell;
use zigbee_mac::esp::EspMlme;
use crate::zigbee::lamp::{LampCapabilities, LampState, SharedLampState};

// Matches the `zigbee` partition in partitions.csv:
// offset 0x3f0000, size 0x4000 (16 KiB)
const ZIGBEE_FLASH_RANGE: Range<u32> = 0x3f_0000..0x3f_4000;

type ZigbeeFlash = zigbee::storage::FlashStorage<BlockingAsync<FlashStorage<'static>>>;

type ZigbeeStack = zigbee::Stack<'static, EspMlme<'static>, Handler, ZigbeeFlash>;

// Reserve static memory for the stack, initialization is done later
static STACK: StaticCell<ZigbeeStack> = StaticCell::new();

static LAMP: StaticCell<SharedLampState> = StaticCell::new();

async fn init_storage(flash: FLASH<'static>) -> ZigbeeFlash {
    // Init flash storage with flash peripheral
    let storage = FlashStorage::new(flash);

    // zigbee-rs expects an async flash interface, so we use the BlockingAsync adapter
    // to provide and async interface
    let flash = BlockingAsync::new(storage);

    // Initialize and return Zigbee's in-memory state and restore saved values
    zigbee::storage::FlashStorage::new(flash, ZIGBEE_FLASH_RANGE).await
}

fn init_radio_mac(ieee802154: IEEE802154<'_>) -> EspMlme<'_> {
    // Move ownership of the radio peripheral into its driver.
    let radio = Ieee802154::new(ieee802154);

    // Move the driver into Zigbee's MAC adapter.
    // Network discovery and joining will configure it further later.
    let mac = EspMlme::new(radio, esp_radio::ieee802154::Config::default());
    log::info!("Device IEEE address: {:#018x}", mac.ieee_address());

    mac
}

static FORGET_NETWORK: Signal<CriticalSectionRawMutex, ()> = Signal::new();

pub fn request_network_reset() {
    FORGET_NETWORK.signal(());
}

pub async fn start_zigbee(
    spawner: Spawner,
    ieee802154: IEEE802154<'static>,
    flash: FLASH<'static>,
    lamp_capabilities: LampCapabilities
) -> &'static SharedLampState {
    let lamp_state: &SharedLampState =
        LAMP.init(SharedLampState::new(LampState::new(lamp_capabilities)));

    lamp_state.update(|mut state| {
        state.capabilities = lamp_capabilities;
        state
    });

    let handler = build_handler(lamp_state);
    let mac = init_radio_mac(ieee802154);
    let storage = init_storage(flash).await;

    log::info!(
        "Restored Zigbee channel: {}",
        zigbee::nwk::nib::get_ref().operating_channel(),
    );

    let stack_config = build_stack_config();

    log::info!("Zigbee configuration: extended_PAN={:#018x}, channel={}, capabilities={:#04x}",
        stack_config.network().extended_pan_id.0,
        stack_config.network().channels.start,
        stack_config.device().capability_information.0,);
    log::info!(
        "Zigbee descriptors configured: {} application endpoint(s)",
        stack_config.descriptors().endpoints.len(),
    );

    // Transfer ownership of the MAC, configuration, handlers, and storage
    let stack: &'static ZigbeeStack =
        STACK.init(zigbee::Stack::new(mac, stack_config, handler, storage));

    spawner.spawn(stack_task(stack).expect("Zigbee task slot unavailable"));
    spawner.spawn(maintenance_task(stack).expect("Zigbee maintenance task slot unavailable"));

    lamp_state
}

#[embassy_executor::task]
async fn stack_task(stack: &'static ZigbeeStack) {
    log::info!("Starting Zigbee stack");

    // We run the stack and await a possible 'reset' signal and let these race. If the stack stops,
    // we just sit there and wait for a possible reset press (by again awaiting FORGET_NETWORK), if
    // the signal comes in first, we forget the network and reset the ESP.

    let outcome = select(
        FORGET_NETWORK.wait(),
        stack.run(AsyncDelay),
    ).await;

    if let Either::Second(reason) = outcome {
        log::error!("Zigbee stack stopped: {reason:?}");

        // Keep the reset button usable even if joining failed
        FORGET_NETWORK.wait().await;
    }

    // The run future has now been dropped, stopping its network activity. Awaiting forget_network
    // ensures its flash flush finishes first.
    log::info!("Forgetting Zigbee network");
    stack.forget_network().await;
    esp_hal::system::software_reset();
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

            log::debug!(
                "Zigbee keepalive={:?}, interval_ms={:?}, identify_s={}",
                nlme.keepalive_method(),
                nlme.keepalive_interval_ms(),
                IDENTIFY.identify_time(),
            );
        }
    }
}
