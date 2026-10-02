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
//use zigbee_mac::mlme::{Mlme, ScanType};
use embassy_embedded_hal::adapter::BlockingAsync;
use esp_storage::FlashStorage;
use zigbee::{DeviceConfig, LogicalType, NetworkConfig};
use zigbee::nwk::nib::CapabilityInformation;
use zigbee::types::IeeeAddress;
use zigbee::zdo::config::DiscoveryType;
use zigbee::{
    CurrentPowerMode, CurrentPowerSourceLevel, PowerSource,
    StackConfig, TimingConfig,
};
use zigbee::zcl::{profile, clusters::general::{basic, identify}};
use zigbee::zdo::descriptor::{
    DeviceDescriptorConfig, EndpointDescriptor,
    NodeDescriptorConfig, PowerDescriptorConfig,
};
use static_cell::StaticCell;
use zigbee::zcl::clusters::general::basic::BasicServer;
use zigbee::zcl::clusters::general::identify::IdentifyServer;
use zigbee::zcl::server::UnsupportedClusterResponder;
use embassy_time::Delay as AsyncDelay;

esp_bootloader_esp_idf::esp_app_desc!();

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("PANIC: {info}");
    let delay = Delay::new();
    loop {
        delay.delay_millis(1000);
    }
}

const LIGHT_ENDPOINT: u8 = 1;

static INPUT_CLUSTERS: [u16; 5] = [
    basic::CLUSTER_ID,    // 0x0000: device identity and basic attributes
    identify::CLUSTER_ID, // 0x0003: identify this physical device
    0x0004,               // Groups: membership in groups of lights
    0x0005,               // Scenes: stored combinations of settings
    0x0006,               // On/Off: switching and the current on/off state
];

// Our lamp does not declare client-side clusters at this stage.
static OUTPUT_CLUSTERS: [u16; 0] = [];

static ENDPOINTS: [EndpointDescriptor<'static>; 1] = [
    EndpointDescriptor {
        endpoint: LIGHT_ENDPOINT,
        profile_id: profile::HOME_AUTOMATION,
        device_id: 0x0100, // On/Off light
        device_version: 1, // Our device revision
        input_clusters: &INPUT_CLUSTERS,
        output_clusters: &OUTPUT_CLUSTERS,
    },
];

static BASIC: BasicServer<'static> = BasicServer {
    zcl_version: 8,
    application_version: 1,
    stack_version: 0,
    hw_version: 1,
    manufacturer_name: "hnsr",
    model_identifier: "esp32c6-led-poc",
    power_source: 0x01,
};

static IDENTIFY: IdentifyServer = IdentifyServer::new();

type ZigbeeFlash = zigbee::storage::FlashStorage<BlockingAsync<FlashStorage<'static>>>;

type Handler = (
    RequestLogger,
    BasicServer<'static>,
    &'static IdentifyServer,
    UnsupportedClusterResponder<'static>,
);

type ZigbeeStack = zigbee::Stack<'static, EspMlme<'static>, Handler, ZigbeeFlash>;

// Reserve static memory for the stack, initialisation is done later
static STACK: StaticCell<ZigbeeStack> = StaticCell::new();

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
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
    let storage = zigbee::storage::init_with_flash(
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

    let stack_config = StackConfig::new(
        network_config,
        device_config,

        TimingConfig::default(),

        DeviceDescriptorConfig {
            node: NodeDescriptorConfig {
                // Bit 3 of this field identifies the 2.4 GHz band
                frequency_band: 0x08,

                manufacturer_code: 0x0000, // placeholder value for testing

                maximum_buffer_size: 80,
                maximum_incoming_transfer_size: 128,
                maximum_outgoing_transfer_size: 128,

                ..NodeDescriptorConfig::default()
            },

            power: PowerDescriptorConfig {
                // Match the receiver-on-when-idle capability already configured
                current_power_mode: CurrentPowerMode::Synchronized,

                // Continuously externally powered device
                available_power_sources: &[PowerSource::ConstantMainPower],
                current_power_source: PowerSource::ConstantMainPower,
                current_power_source_level: CurrentPowerSourceLevel::Full,
            },

            endpoints: &ENDPOINTS,
        },
    );

    println!(
        "Zigbee descriptors configured: {} application endpoint(s)",
        stack_config.descriptors().endpoints.len(),
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

    // println!("Scanning Zigbee channels 11–26...");
    //
    // // Scan channels 11-26 (inclusive), listen for duration 5, on each channel.
    // match mac.scan_network(ScanType::Active, 11..27, 5).await {
    //     Ok(result) => {
    //         println!("Received {} network beacons", result.pan_descriptor.len());
    //
    //         for network in &result.pan_descriptor {
    //             println!(
    //                 "channel={} PAN={:#06x} extended_PAN={:#018x} sender={:?} LQI={} join_open={}",
    //                 network.channel,
    //                 network.coord_pan_id.0,
    //                 network.zigbee_beacon.extended_pan_id.0,
    //                 network.coord_address,
    //                 network.link_quality,
    //                 network.superframe_spec.association_permit,
    //             );
    //         }
    //     }
    //     Err(error) => {
    //         println!("Scan failed: {error:?}");
    //     }
    // }

    // The zigbee library tries tuple handlers from left to right, so the fallback goes last
    let handler = (
        RequestLogger,
        BASIC,
        &IDENTIFY,
        UnsupportedClusterResponder::new(&INPUT_CLUSTERS),
    );

    // Transfer ownership of the MAC, configuration, handlers, and storage
    let stack: &'static ZigbeeStack = STACK.init(
        zigbee::Stack::new(mac, stack_config, handler, storage),
    );

    println!(
        "Zigbee stack constructed for channel {}",
        stack.config().channel(),
    );

    spawner.spawn(
        stack_task(stack).expect("Could not allocate Zigbee task"),
    );

    loop {
        Timer::after_secs(1).await;

        println!(
            "Main task is alive!"
        );

        // Advance Identify's countdown. Later we'll blink an LED here.
        if IDENTIFY.is_identifying() {
            println!(
                "Identifying: {} seconds remaining",
                IDENTIFY.tick(1),
            );
        }
    }
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

use zigbee::zdo::{
    ClusterReply, ClusterRequest, ClusterRequestHandler,
};

// A handler that logs requests and lets subsequent handlers run.
struct RequestLogger;

impl ClusterRequestHandler for RequestLogger {
    fn handle(
        &self,
        request: &ClusterRequest<'_>,
        _out: &mut [u8],
    ) -> Option<ClusterReply> {
        println!(
            "Application request: profile={:#06x}, cluster={:#06x}, endpoint={}, bytes={:02x?}",
            request.profile_id,
            request.cluster_id,
            request.dst_endpoint,
            request.asdu,
        );

        // No response from this handler, continue through the tuple
        None
    }
}