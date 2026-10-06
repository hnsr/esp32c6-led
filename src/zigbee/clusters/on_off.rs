use core::sync::atomic::{AtomicBool, Ordering};
use esp_println::println;
use zigbee::zcl::frame::Status;
use zigbee::zcl::server::{ClusterCommand, ClusterServer, CommandOutcome};
use zigbee::zcl::types::{AttrInfo, Attribute, AttributeId, Bool, Cluster, ClusterId};
use zigbee::zdo::{ClusterReply, ClusterRequest, ClusterRequestHandler};

const ON_OFF_CLUSTER: Cluster =
    Cluster::new(ClusterId(0x0006), "On/Off");

const ON_OFF_ATTRIBUTE: Attribute<Bool> =
    ON_OFF_CLUSTER.attribute(AttributeId(0x0000), "OnOff");

const ON_OFF_ATTRIBUTES: &[AttrInfo] = &[
    ON_OFF_ATTRIBUTE.attr_info(),
];

pub struct OnOffServer {
    on: AtomicBool,
}

impl OnOffServer {
    pub const fn new() -> Self {
        Self {
            on: AtomicBool::new(false),
        }
    }
    pub fn is_on(&self) -> bool {
        self.on.load(Ordering::Relaxed)
    }

    pub fn set_on(&self, on: bool) {
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


