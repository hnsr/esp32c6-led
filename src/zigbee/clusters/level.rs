use crate::zigbee::clusters::on_off::OnOffServer;
use core::sync::atomic::{AtomicU8, Ordering};
use esp_println::println;
use zigbee::zcl::frame::Status;
use zigbee::zcl::server::{ClusterCommand, ClusterServer, CommandOutcome};
use zigbee::zcl::types::{AttrInfo, Attribute, AttributeId, Cluster, ClusterId, Uint8};
use zigbee::zdo::{ClusterReply, ClusterRequest, ClusterRequestHandler};

const LEVEL_CONTROL_CLUSTER: Cluster = Cluster::new(ClusterId(0x0008), "Level Control");

const CURRENT_LEVEL: Attribute<Uint8> =
    LEVEL_CONTROL_CLUSTER.attribute(AttributeId(0x0000), "CurrentLevel");

const LEVEL_ATTRIBUTES: &[AttrInfo] = &[CURRENT_LEVEL.attr_info()];

const MIN_LIGHT_LEVEL: u8 = 1;
const MAX_LIGHT_LEVEL: u8 = 254;

pub(in crate::zigbee) struct LevelControlServer {
    level: AtomicU8,

    // Borrow the existing On/Off server so we don't have to duplicate state.
    on_off: &'static OnOffServer,
}

impl LevelControlServer {
    pub const fn new(on_off: &'static OnOffServer) -> Self {
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

    fn encode_value(&self, id: AttributeId, out: &mut [u8], offset: &mut usize) -> Status {
        if id != CURRENT_LEVEL.id() {
            return Status::UnsupportedAttribute;
        }
        match CURRENT_LEVEL.encode(Uint8(self.current_level()), out, offset) {
            Ok(()) => Status::Success,
            Err(_) => Status::InsufficientSpace,
        }
    }

    fn command(&self, command: ClusterCommand<'_>, _out: &mut [u8]) -> CommandOutcome {
        let with_on_off = command.id.0 == 0x04;

        // Basic payload:
        //   level: u8
        //   transition time: little-endian u16, in tenths of a second
        //
        // Move To Level may also carry OptionsMask and OptionsOverride.
        let (requested_level, transition_time, execute_if_off) = match (command.id.0, command.data)
        {
            (0x00 | 0x04, [level, lo, hi]) => (*level, u16::from_le_bytes([*lo, *hi]), false),

            (0x00, [level, lo, hi, mask, overrides]) => (
                *level,
                u16::from_le_bytes([*lo, *hi]),
                // For now our default Options bitmap is zero.
                // Bit 0 can be overridden to permit execution while off.
                (*mask & *overrides & 0x01) != 0,
            ),

            // Recognized command, incorrect payload length.
            (0x00 | 0x04, _) => {
                return CommandOutcome::Status(Status::MalformedCommand);
            }

            // Move, Step, Stop, and their variants not implemented for now.
            _ => {
                return CommandOutcome::Status(Status::UnsupCommand);
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
    fn handle(&self, request: &ClusterRequest<'_>, out: &mut [u8]) -> Option<ClusterReply> {
        self.handle_request(request, out)
    }
}
