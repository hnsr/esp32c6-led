use zigbee::zcl::frame::Status;
use zigbee::zcl::server::{ClusterCommand, ClusterServer, CommandOutcome};
use zigbee::zcl::types::{AttrInfo, Attribute, AttributeId, Cluster, ClusterId, Uint8};
use zigbee::zdo::{ClusterReply, ClusterRequest, ClusterRequestHandler};
use crate::lamp::SharedLamp;

const LEVEL_CONTROL_CLUSTER: Cluster = Cluster::new(ClusterId(0x0008), "Level Control");

const CURRENT_LEVEL: Attribute<Uint8> =
    LEVEL_CONTROL_CLUSTER.attribute(AttributeId(0x0000), "CurrentLevel");

const LEVEL_ATTRIBUTES: &[AttrInfo] = &[CURRENT_LEVEL.attr_info()];

// fixme: should be defined on lamp state
const MIN_LIGHT_LEVEL: u8 = 1;
const MAX_LIGHT_LEVEL: u8 = 254;

pub(in crate::zigbee) struct LevelControlServer {
    lamp: &'static SharedLamp
}

impl LevelControlServer {
    pub const fn new(lamp: &'static SharedLamp) -> Self {
        Self {
            lamp
        }
    }

    fn current_level(&self) -> u8 {
        self.lamp.get().brightness
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

        let mut lamp = self.lamp.get();

        // Ordinary "Move To Level" does not switch the lamp on, so we ignore it
        if !with_on_off && !lamp.on && !execute_if_off {
            log::debug!("Level command ignored: lamp is off");
            return CommandOutcome::Status(Status::Success);
        }

        if requested_level == 0xff {
            return CommandOutcome::Status(Status::InvalidValue);
        }

        // Clamp requested level to lamp limits
        let level = requested_level.clamp(MIN_LIGHT_LEVEL, MAX_LIGHT_LEVEL);

        lamp.brightness = level;

        // The "With On/Off" variant allows switching the light on/off as needed
        if with_on_off {
            lamp.on = level > MIN_LIGHT_LEVEL;
        }
        self.lamp.set(lamp);

        log::debug!(
            "Light level: {}/254, on={}, transition={:#06x} \
             (target applied immediately)",
            level,
            lamp.on,
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
