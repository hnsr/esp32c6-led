use esp_println::println;
use zigbee::{CurrentPowerMode, CurrentPowerSourceLevel, DeviceConfig, LogicalType, NetworkConfig, PowerSource, StackConfig, TimingConfig};
use zigbee::nwk::nib::CapabilityInformation;
use zigbee::types::IeeeAddress;
use zigbee::zcl::clusters::general::{basic, identify};
use zigbee::zcl::profile;
use zigbee::zdo::config::DiscoveryType;
use zigbee::zdo::descriptor::{DeviceDescriptorConfig, EndpointDescriptor, NodeDescriptorConfig, PowerDescriptorConfig};

const LIGHT_ENDPOINT: u8 = 1;

pub static INPUT_CLUSTERS: [u16; 7] = [
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

pub(super) fn build_stack_config() -> StackConfig<'static> {
    let network_id_text = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/zigbee-network.txt",
    ));
    let extended_pan_id = u64::from_str_radix(network_id_text.trim(), 16)
        .expect("zigbee-network.txt must contain a hexadecimal extended PAN ID");

    let network_config = NetworkConfig {
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

        TimingConfig {
            default_keepalive_interval_ms: 10_000,
            ..TimingConfig::default()
        },

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

    stack_config
}