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
        device_id: 0x010d, // Extended color light
        device_version: 1, // Our device revision
        input_clusters: &INPUT_CLUSTERS,
        output_clusters: &OUTPUT_CLUSTERS,
    },
];

// Initially restrict discovery to the known Hue network's channel.
const NETWORK_CHANNEL: u8 = 25;

// Used when no keepalive interval was negotiated with the parent.
const FALLBACK_KEEPALIVE_INTERVAL_MS: u32 = 10_000;

pub(super) fn build_stack_config(network_id: u64) -> StackConfig<'static> {
    // fixme: replace network_id parameter with automatic network join logic
    let network_config = NetworkConfig {
        extended_pan_id: IeeeAddress(network_id),
        channels: (NETWORK_CHANNEL..NETWORK_CHANNEL+1),
        scan_duration: 5,
    };

    let device_config = DeviceConfig {
        logical_type: LogicalType::EndDevice,
        capability_information: CapabilityInformation(
            // Bit '1' remains clear: this device joins as an end device.
            (1 << 2) // Mains-powered
                | (1 << 3) // Receiver enabled while idle
                | (1 << 7), // Request an allocated short address
        ),


        // Address-discovery policy: request a device's IEEE address
        // when its short network address is already known
        discovery_type: DiscoveryType::IEEE,

        // Keep the Trust Center link-key exchange enabled
        tc_link_key_exchange: true,
    };

    let stack_config = StackConfig::new(
        network_config,
        device_config,

        TimingConfig {
            default_keepalive_interval_ms: FALLBACK_KEEPALIVE_INTERVAL_MS,
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
    stack_config
}