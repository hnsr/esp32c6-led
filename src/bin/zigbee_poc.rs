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
use core::sync::atomic::{AtomicBool, Ordering};
use zigbee::zcl::frame::Status;
use zigbee::zcl::server::{
    ClusterCommand, ClusterServer, CommandOutcome,
};
use zigbee::zcl::types::{
    AttrInfo, Attribute, AttributeId, Bool, Cluster, ClusterId,
};
use core::sync::atomic::AtomicU8;
use zigbee::zcl::types::Uint8;
use core::sync::atomic::AtomicU16;
use zigbee::zcl::types::{
    Bitmap8, Bitmap16, Enum8, ReadWrite, TypeId, Uint16,
    ZclBitmap8, ZclBitmap16, ZclEnum8,
};

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

static INPUT_CLUSTERS: [u16; 7] = [
    basic::CLUSTER_ID,    // 0x0000: device identity and basic attributes
    identify::CLUSTER_ID, // 0x0003: identify this physical device
    0x0004,               // Groups: membership in groups of lights
    0x0005,               // Scenes: stored combinations of settings
    0x0006,               // On/Off: switching and the current on/off state
    0x0008,               // Level control (brightness/intensity)
    0x0300,               // Color Control
];

// Our lamp does not declare client-side clusters at this stage.
static OUTPUT_CLUSTERS: [u16; 0] = [];

static ENDPOINTS: [EndpointDescriptor<'static>; 1] = [
    EndpointDescriptor {
        endpoint: LIGHT_ENDPOINT,
        profile_id: profile::HOME_AUTOMATION,
        device_id: 0x010d, // Extended colour light
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
    (
        &'static OnOffServer,
        &'static LevelControlServer,
        &'static ColorControlServer,
    ),
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
        let nlme = stack.device().nlme();
        println!(
            "Main task is running, keepalive={:?}, interval_ms={:?}",
            nlme.keepalive_method(),
            nlme.keepalive_interval_ms(),
        );
        Timer::after_secs(10).await;

        // TODO: Actually implement blinking for visual device identification, once we integrate
        //       into LED diver code
        if IDENTIFY.is_identifying() {
            println!(
                "Identifying: {} seconds remaining",
                IDENTIFY.tick(10),
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
            "Application request: profile={:#06x}, cluster={:#06x}, endpoint={}, unicast={}, bytes={:02x?}",
            request.profile_id,
            request.cluster_id,
            request.dst_endpoint,
            request.unicast,
            request.asdu,
        );

        // No response from this handler, continue through the tuple
        None
    }
}

// On/off cluster
// =================================================================================================
const ON_OFF_CLUSTER: Cluster =
    Cluster::new(ClusterId(0x0006), "On/Off");

const ON_OFF_ATTRIBUTE: Attribute<Bool> =
    ON_OFF_CLUSTER.attribute(AttributeId(0x0000), "OnOff");

const ON_OFF_ATTRIBUTES: &[AttrInfo] = &[
    ON_OFF_ATTRIBUTE.attr_info(),
];

struct OnOffServer {
    on: AtomicBool,
}

impl OnOffServer {
    const fn new() -> Self {
        Self {
            on: AtomicBool::new(false),
        }
    }
    fn is_on(&self) -> bool {
        self.on.load(Ordering::Relaxed)
    }

    fn set_on(&self, on: bool) {
        self.on.store(on, Ordering::Relaxed);

        println!(
            "Light state: {}",
            if on { "ON" } else { "OFF" },
        );
    }
}

impl ClusterServer for OnOffServer {
    fn cluster(&self) -> Cluster {
        ON_OFF_CLUSTER
    }

    fn attributes(&self) -> &'static [AttrInfo] {
        ON_OFF_ATTRIBUTES
    }

    fn encode_value(
        &self,
        id: AttributeId,
        out: &mut [u8],
        offset: &mut usize,
    ) -> Status {
        if id != ON_OFF_ATTRIBUTE.id() {
            return Status::UnsupportedAttribute;
        }

        // Append the ZCL Boolean type identifier and current value.
        // The library builds the surrounding Read Attributes response.
        match ON_OFF_ATTRIBUTE.encode(self.is_on(), out, offset) {
            Ok(()) => Status::Success,
            Err(_) => Status::InsufficientSpace,
        }
    }

    fn command(
        &self,
        command: ClusterCommand<'_>,
        _out: &mut [u8],
    ) -> CommandOutcome {
        let new_state = match (command.id.0, command.data) {
            (0x00, []) => false,         // Off
            (0x01, []) => true,          // On
            (0x02, []) => !self.is_on(), // Toggle

            (0x40, [effect, variant]) => {
                println!(
                    "Off With Effect: effect={:#04x}, variant={:#04x}",
                    effect, variant,
                );

                // TODO: actually implement the effect
                false
            }

            // Recognized command, but incorrect payload length
            (0x00 | 0x01 | 0x02 | 0x40, _) => {
                return CommandOutcome::Status(Status::MalformedCommand);
            }

            // Other On/Off commands are not implemented yet
            _ => {
                return CommandOutcome::Status(Status::UnsupCommand);
            }
        };

        self.set_on(new_state);

        // The library decides whether a Default Response is required.
        CommandOutcome::Status(Status::Success)
    }
}

impl ClusterRequestHandler for OnOffServer {
    fn handle(
        &self,
        request: &ClusterRequest<'_>,
        out: &mut [u8],
    ) -> Option<ClusterReply> {
        self.handle_request(request, out)
    }
}

static ON_OFF: OnOffServer = OnOffServer::new();


// Level control cluster
// =================================================================================================
const LEVEL_CONTROL_CLUSTER: Cluster = Cluster::new(ClusterId(0x0008), "Level Control");

const CURRENT_LEVEL: Attribute<Uint8> =
    LEVEL_CONTROL_CLUSTER.attribute(AttributeId(0x0000), "CurrentLevel", );

const LEVEL_ATTRIBUTES: &[AttrInfo] = &[ CURRENT_LEVEL.attr_info() ];

const MIN_LIGHT_LEVEL: u8 = 1;
const MAX_LIGHT_LEVEL: u8 = 254;

struct LevelControlServer {
    level: AtomicU8,

    // Borrow the existing On/Off server so we don't have to duplicate state.
    on_off: &'static OnOffServer,
}

impl LevelControlServer {
    const fn new(on_off: &'static OnOffServer) -> Self {
        Self {
            level: AtomicU8::new(MAX_LIGHT_LEVEL),
            on_off,
        }
    }

    fn current_level(&self) -> u8 {
        self.level.load(Ordering::Relaxed)
    }
}

impl ClusterServer for LevelControlServer {
    fn cluster(&self) -> Cluster {
        LEVEL_CONTROL_CLUSTER
    }

    fn attributes(&self) -> &'static [AttrInfo] {
        LEVEL_ATTRIBUTES
    }

    fn encode_value(
        &self,
        id: AttributeId,
        out: &mut [u8],
        offset: &mut usize,
    ) -> Status {
        if id != CURRENT_LEVEL.id() {
            return Status::UnsupportedAttribute;
        }
        match CURRENT_LEVEL.encode(
            Uint8(self.current_level()),
            out,
            offset,
        ) {
            Ok(()) => Status::Success,
            Err(_) => Status::InsufficientSpace,
        }
    }

    fn command(
        &self,
        command: ClusterCommand<'_>,
        _out: &mut [u8],
    ) -> CommandOutcome {
        let with_on_off = command.id.0 == 0x04;

        // Basic payload:
        //   level: u8
        //   transition time: little-endian u16, in tenths of a second
        //
        // Move To Level may also carry OptionsMask and OptionsOverride.
        let (requested_level, transition_time, execute_if_off) =
            match (command.id.0, command.data) {
                (0x00 | 0x04, [level, lo, hi]) => (
                    *level,
                    u16::from_le_bytes([*lo, *hi]),
                    false,
                ),

                (0x00, [level, lo, hi, mask, overrides]) => (
                    *level,
                    u16::from_le_bytes([*lo, *hi]),

                    // For now our default Options bitmap is zero.
                    // Bit 0 can be overridden to permit execution while off.
                    (*mask & *overrides & 0x01) != 0,
                ),

                // Recognized command, incorrect payload length.
                (0x00 | 0x04, _) => {
                    return CommandOutcome::Status(
                        Status::MalformedCommand,
                    );
                }

                // Move, Step, Stop, and their variants not implemented for now.
                _ => {
                    return CommandOutcome::Status(
                        Status::UnsupCommand,
                    );
                }
            };

        // Ordinary "Move To Level" does not switch the lamp on, so we ignore it
        if !with_on_off && !self.on_off.is_on() && !execute_if_off {
            println!("Level command ignored: lamp is off");
            return CommandOutcome::Status(Status::Success);
        }

        if requested_level == 0xff {
            return CommandOutcome::Status(Status::InvalidValue);
        }

        // A target below the lighting minimum is clamped to that minimum.
        let level = requested_level.max(MIN_LIGHT_LEVEL);

        self.level.store(level, Ordering::Relaxed);

        // The "With On/Off" variant allows switching the light on/off as needed
        if with_on_off {
            self.on_off.set_on(level > MIN_LIGHT_LEVEL);
        }

        println!(
            "Light level: {}/254, on={}, transition={:#06x} \
             (target applied immediately)",
            level,
            self.on_off.is_on(),
            transition_time,
        );

        CommandOutcome::Status(Status::Success)
    }
}

impl ClusterRequestHandler for LevelControlServer {
    fn handle(
        &self,
        request: &ClusterRequest<'_>,
        out: &mut [u8],
    ) -> Option<ClusterReply> {
        self.handle_request(request, out)
    }
}

static LEVEL_CONTROL: LevelControlServer =
    LevelControlServer::new(&ON_OFF);



// Color control cluster
// =================================================================================================
#[derive(Clone, Copy, Debug)]
#[repr(u8)]
enum ColorMode {
    Xy = 0x01,
    Temperature = 0x02,
}

impl ZclEnum8 for ColorMode {
    fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            0x01 => Some(Self::Xy),
            0x02 => Some(Self::Temperature),
            _ => None,
        }
    }

    fn into_raw(self) -> u8 {
        self as u8
    }
}

#[derive(Clone, Copy)]
struct ColorCapabilities(u16);

impl ZclBitmap16 for ColorCapabilities {
    fn from_bits(bits: u16) -> Self {
        Self(bits)
    }

    fn into_bits(self) -> u16 {
        self.0
    }
}

#[derive(Clone, Copy)]
struct ColorOptions(u8);

impl ZclBitmap8 for ColorOptions {
    fn from_bits(bits: u8) -> Self {
        Self(bits)
    }

    fn into_bits(self) -> u8 {
        self.0
    }
}


const COLOR_CONTROL_CLUSTER: Cluster =
    Cluster::new(ClusterId(0x0300), "Color Control");

const COLOR_REMAINING_TIME: Attribute<Uint16> =
    COLOR_CONTROL_CLUSTER.attribute(AttributeId(0x0002), "RemainingTime");

const COLOR_X: Attribute<Uint16> =
    COLOR_CONTROL_CLUSTER.attribute(AttributeId(0x0003), "CurrentX");

const COLOR_Y: Attribute<Uint16> =
    COLOR_CONTROL_CLUSTER.attribute(AttributeId(0x0004), "CurrentY");

const COLOR_TEMPERATURE: Attribute<Uint16> =
    COLOR_CONTROL_CLUSTER.attribute(
        AttributeId(0x0007), "ColorTemperatureMireds",
    );

const COLOR_MODE: Attribute<Enum8<ColorMode>> =
    COLOR_CONTROL_CLUSTER.attribute(AttributeId(0x0008), "ColorMode");

const COLOR_OPTIONS: Attribute<Bitmap8<ColorOptions>, ReadWrite> =
    COLOR_CONTROL_CLUSTER.attribute(AttributeId(0x000f), "Options");

const COLOR_ENHANCED_MODE: Attribute<Enum8<ColorMode>> =
    COLOR_CONTROL_CLUSTER.attribute(
        AttributeId(0x4001), "EnhancedColorMode",
    );

const COLOR_CAPABILITIES: Attribute<Bitmap16<ColorCapabilities>> =
    COLOR_CONTROL_CLUSTER.attribute(
        AttributeId(0x400a), "ColorCapabilities",
    );

const COLOR_TEMP_MIN: Attribute<Uint16> =
    COLOR_CONTROL_CLUSTER.attribute(
        AttributeId(0x400b), "ColorTempPhysicalMinMireds",
    );

const COLOR_TEMP_MAX: Attribute<Uint16> =
    COLOR_CONTROL_CLUSTER.attribute(
        AttributeId(0x400c), "ColorTempPhysicalMaxMireds",
    );

const COLOR_ATTRIBUTES: &[AttrInfo] = &[
    COLOR_REMAINING_TIME.attr_info(),
    COLOR_X.attr_info(),
    COLOR_Y.attr_info(),
    COLOR_TEMPERATURE.attr_info(),
    COLOR_MODE.attr_info(),
    COLOR_OPTIONS.attr_info(),
    COLOR_ENHANCED_MODE.attr_info(),
    COLOR_CAPABILITIES.attr_info(),
    COLOR_TEMP_MIN.attr_info(),
    COLOR_TEMP_MAX.attr_info(),
];

// Set bits for XY and temperature capabilities
const SUPPORTED_COLOR_CAPABILITIES: ColorCapabilities =
    ColorCapabilities((1 << 3) | (1 << 4));

// TODO: Choose limits depending on hardware
const MIN_COLOR_MIREDS: u16 = 153;
const MAX_COLOR_MIREDS: u16 = 500;


struct ColorControlServer {
    x: AtomicU16,
    y: AtomicU16,
    temperature: AtomicU16,
    mode: AtomicU8,
    options: AtomicU8,
    on_off: &'static OnOffServer,
}

impl ColorControlServer {
    const fn new(on_off: &'static OnOffServer) -> Self {
        Self {
            x: AtomicU16::new(0x616b),
            y: AtomicU16::new(0x607d),

            // 250 mireds = 4000K
            temperature: AtomicU16::new(250),
            mode: AtomicU8::new(ColorMode::Xy as u8),

            options: AtomicU8::new(0),
            on_off,
        }
    }

    fn color_mode(&self) -> ColorMode {
        ColorMode::from_raw(self.mode.load(Ordering::Relaxed))
            .expect("Invalid stored colour mode")
    }
}

impl ClusterServer for ColorControlServer {
    fn cluster(&self) -> Cluster {
        COLOR_CONTROL_CLUSTER
    }

    fn attributes(&self) -> &'static [AttrInfo] {
        COLOR_ATTRIBUTES
    }

    fn encode_value(
        &self,
        id: AttributeId,
        out: &mut [u8],
        offset: &mut usize,
    ) -> Status {
        let result = match id.0 {
            // Targets apply immediately, so no transition remains and we always return 0
            0x0002 => COLOR_REMAINING_TIME.encode(Uint16(0), out, offset),

            0x0003 => COLOR_X.encode(
                Uint16(self.x.load(Ordering::Relaxed)), out, offset,
            ),
            0x0004 => COLOR_Y.encode(
                Uint16(self.y.load(Ordering::Relaxed)), out, offset,
            ),
            0x0007 => COLOR_TEMPERATURE.encode(
                Uint16(self.temperature.load(Ordering::Relaxed)),
                out,
                offset,
            ),
            0x0008 => COLOR_MODE.encode(self.color_mode(), out, offset),
            0x000f => COLOR_OPTIONS.encode(
                ColorOptions(self.options.load(Ordering::Relaxed)),
                out,
                offset,
            ),
            0x4001 => COLOR_ENHANCED_MODE.encode(
                self.color_mode(), out, offset,
            ),
            0x400a => COLOR_CAPABILITIES.encode(
                SUPPORTED_COLOR_CAPABILITIES, out, offset,
            ),
            0x400b => COLOR_TEMP_MIN.encode(
                Uint16(MIN_COLOR_MIREDS), out, offset,
            ),
            0x400c => COLOR_TEMP_MAX.encode(
                Uint16(MAX_COLOR_MIREDS), out, offset,
            ),
            _ => return Status::UnsupportedAttribute,
        };

        match result {
            Ok(()) => Status::Success,
            Err(_) => Status::InsufficientSpace,
        }
    }

    fn decode_value(
        &self,
        id: AttributeId,
        type_id: TypeId,
        bytes: &[u8],
        offset: &mut usize,
    ) -> Status {
        // Options is our only writable attribute
        if id != COLOR_OPTIONS.id() {
            return if COLOR_ATTRIBUTES.iter().any(|attr| attr.id == id) {
                Status::ReadOnly
            } else {
                Status::UnsupportedAttribute
            };
        }

        if type_id != COLOR_OPTIONS.type_id() {
            return Status::InvalidDataType;
        }

        let Ok(options) = COLOR_OPTIONS.decode(type_id, bytes, offset) else {
            return Status::MalformedCommand;
        };

        self.options.store(options.0, Ordering::Relaxed);
        Status::Success
    }

    fn command(
        &self,
        command: ClusterCommand<'_>,
        _out: &mut [u8],
    ) -> CommandOutcome {
        let base_len = match command.id.0 {
            // TODO: Declare consts for the relevant command IDs
            0x07 => 6, // MoveToColor, payload: x: u16, y: u16, transition: u16
            0x0a => 4, // MoveToColorTemperature, payload: temperature: u16, transition: u16
            _ => return CommandOutcome::Status(Status::UnsupCommand),
        };

        // Validate payload length, check for the optional options mask/override
        let (body, mask, overrides) = match command.data.len() {
            n if n == base_len => (
                &command.data[..base_len],
                0u8,
                0u8,
            ),
            n if n == base_len + 2 => (
                &command.data[..base_len],
                command.data[base_len],
                command.data[base_len + 1],
            ),
            _ => {
                return CommandOutcome::Status(Status::MalformedCommand);
            }
        };

        // Load default options, apply per-command overrides
        let defaults = self.options.load(Ordering::Relaxed);
        let effective_options =
            (defaults & !mask) | (overrides & mask);

        let execute_if_off = (effective_options & 0x01) != 0;

        if !self.on_off.is_on() && !execute_if_off {
            println!("Colour command ignored: lamp is off");
            return CommandOutcome::Status(Status::Success);
        }

        match command.id.0 {
            // MoveToColor
            0x07 => {
                let x = u16::from_le_bytes([body[0], body[1]]);
                let y = u16::from_le_bytes([body[2], body[3]]);
                let transition =
                    u16::from_le_bytes([body[4], body[5]]);

                // Ensure valid range, mappping to LED supported ranges wil be done later.
                if x > 0xfeff || y > 0xfeff {
                    return CommandOutcome::Status(Status::InvalidValue);
                }

                self.x.store(x, Ordering::Relaxed);
                self.y.store(y, Ordering::Relaxed);
                self.mode.store(ColorMode::Xy as u8, Ordering::Relaxed);

                println!(
                    "Colour mode: xy, x={:#06x}, y={:#06x}, \
                 transition={:#06x} (target applied immediately)",
                    x, y, transition,
                );
            }
            // MoveToColorTemperature
            0x0a => {
                let requested =
                    u16::from_le_bytes([body[0], body[1]]);
                let transition =
                    u16::from_le_bytes([body[2], body[3]]);

                if requested > 0xfeff {
                    return CommandOutcome::Status(Status::InvalidValue);
                }

                let mireds =
                    requested.clamp(MIN_COLOR_MIREDS, MAX_COLOR_MIREDS);

                self.temperature.store(mireds, Ordering::Relaxed);
                self.mode.store(
                    ColorMode::Temperature as u8,
                    Ordering::Relaxed,
                );

                println!(
                    "Colour mode: temperature, {} mireds (~{} K), \
                 transition={:#06x} (target applied immediately)",
                    mireds,
                    1_000_000u32 / u32::from(mireds),
                    transition,
                );
            }

            _ => unreachable!(),
        }

        CommandOutcome::Status(Status::Success)
    }
}

impl ClusterRequestHandler for ColorControlServer {
    fn handle(
        &self,
        request: &ClusterRequest<'_>,
        out: &mut [u8],
    ) -> Option<ClusterReply> {
        self.handle_request(request, out)
    }
}

static COLOR_CONTROL: ColorControlServer =
    ColorControlServer::new(&ON_OFF);