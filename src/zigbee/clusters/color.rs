use super::on_off::OnOffServer;
use core::sync::atomic::{AtomicU8, AtomicU16, Ordering};
use zigbee::zcl::frame::Status;
use zigbee::zcl::server::{ClusterCommand, ClusterServer, CommandOutcome};
use zigbee::zcl::types::{
    AttrInfo, Attribute, AttributeId, Bitmap8, Bitmap16, Cluster, ClusterId, Enum8, ReadWrite,
    TypeId, Uint16, ZclBitmap8, ZclBitmap16, ZclEnum8,
};
use zigbee::zdo::{ClusterReply, ClusterRequest, ClusterRequestHandler};

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

const COLOR_CONTROL_CLUSTER: Cluster = Cluster::new(ClusterId(0x0300), "Color Control");

const COLOR_REMAINING_TIME: Attribute<Uint16> =
    COLOR_CONTROL_CLUSTER.attribute(AttributeId(0x0002), "RemainingTime");

const COLOR_X: Attribute<Uint16> = COLOR_CONTROL_CLUSTER.attribute(AttributeId(0x0003), "CurrentX");

const COLOR_Y: Attribute<Uint16> = COLOR_CONTROL_CLUSTER.attribute(AttributeId(0x0004), "CurrentY");

const COLOR_TEMPERATURE: Attribute<Uint16> =
    COLOR_CONTROL_CLUSTER.attribute(AttributeId(0x0007), "ColorTemperatureMireds");

const COLOR_MODE: Attribute<Enum8<ColorMode>> =
    COLOR_CONTROL_CLUSTER.attribute(AttributeId(0x0008), "ColorMode");

const COLOR_OPTIONS: Attribute<Bitmap8<ColorOptions>, ReadWrite> =
    COLOR_CONTROL_CLUSTER.attribute(AttributeId(0x000f), "Options");

const COLOR_ENHANCED_MODE: Attribute<Enum8<ColorMode>> =
    COLOR_CONTROL_CLUSTER.attribute(AttributeId(0x4001), "EnhancedColorMode");

const COLOR_CAPABILITIES: Attribute<Bitmap16<ColorCapabilities>> =
    COLOR_CONTROL_CLUSTER.attribute(AttributeId(0x400a), "ColorCapabilities");

const COLOR_TEMP_MIN: Attribute<Uint16> =
    COLOR_CONTROL_CLUSTER.attribute(AttributeId(0x400b), "ColorTempPhysicalMinMireds");

const COLOR_TEMP_MAX: Attribute<Uint16> =
    COLOR_CONTROL_CLUSTER.attribute(AttributeId(0x400c), "ColorTempPhysicalMaxMireds");

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
const SUPPORTED_COLOR_CAPABILITIES: ColorCapabilities = ColorCapabilities((1 << 3) | (1 << 4));

const COMMAND_MOVE_TO_COLOR: u8 = 0x07;
const COMMAND_MOVE_TO_COLOR_TEMPERATURE: u8 = 0x0a;

// TODO: Choose limits depending on hardware
const MIN_COLOR_MIREDS: u16 = 153;
const MAX_COLOR_MIREDS: u16 = 500;

pub(in crate::zigbee) struct ColorControlServer {
    x: AtomicU16,
    y: AtomicU16,
    temperature: AtomicU16,
    mode: AtomicU8,
    options: AtomicU8,
    on_off: &'static OnOffServer,
}

impl ColorControlServer {
    pub(super) const fn new(on_off: &'static OnOffServer) -> Self {
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
        ColorMode::from_raw(self.mode.load(Ordering::Relaxed)).expect("Invalid stored colour mode")
    }
}

impl ClusterServer for ColorControlServer {
    fn cluster(&self) -> Cluster {
        COLOR_CONTROL_CLUSTER
    }

    fn attributes(&self) -> &'static [AttrInfo] {
        COLOR_ATTRIBUTES
    }

    fn encode_value(&self, id: AttributeId, out: &mut [u8], offset: &mut usize) -> Status {
        let result = match id.0 {
            // Targets apply immediately, so no transition remains and we always return 0
            0x0002 => COLOR_REMAINING_TIME.encode(Uint16(0), out, offset),

            0x0003 => COLOR_X.encode(Uint16(self.x.load(Ordering::Relaxed)), out, offset),
            0x0004 => COLOR_Y.encode(Uint16(self.y.load(Ordering::Relaxed)), out, offset),
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
            0x4001 => COLOR_ENHANCED_MODE.encode(self.color_mode(), out, offset),
            0x400a => COLOR_CAPABILITIES.encode(SUPPORTED_COLOR_CAPABILITIES, out, offset),
            0x400b => COLOR_TEMP_MIN.encode(Uint16(MIN_COLOR_MIREDS), out, offset),
            0x400c => COLOR_TEMP_MAX.encode(Uint16(MAX_COLOR_MIREDS), out, offset),
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

    fn command(&self, command: ClusterCommand<'_>, _out: &mut [u8]) -> CommandOutcome {
        let base_len = match command.id.0 {
            COMMAND_MOVE_TO_COLOR => 6, // payload: x: u16, y: u16, transition: u16
            COMMAND_MOVE_TO_COLOR_TEMPERATURE => 4, // payload: temperature: u16, transition: u16
            _ => return CommandOutcome::Status(Status::UnsupCommand),
        };

        // Validate payload length, check for the optional options mask/override
        let (body, mask, overrides) = match command.data.len() {
            n if n == base_len => (&command.data[..base_len], 0u8, 0u8),
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
        let effective_options = (defaults & !mask) | (overrides & mask);

        let execute_if_off = (effective_options & 0x01) != 0;

        if !self.on_off.is_on() && !execute_if_off {
            log::debug!("Colour command ignored: lamp is off");
            return CommandOutcome::Status(Status::Success);
        }

        match command.id.0 {
            COMMAND_MOVE_TO_COLOR => {
                let x = u16::from_le_bytes([body[0], body[1]]);
                let y = u16::from_le_bytes([body[2], body[3]]);
                let transition = u16::from_le_bytes([body[4], body[5]]);

                // Ensure valid range, mappping to LED supported ranges wil be done later.
                if x > 0xfeff || y > 0xfeff {
                    return CommandOutcome::Status(Status::InvalidValue);
                }

                self.x.store(x, Ordering::Relaxed);
                self.y.store(y, Ordering::Relaxed);
                self.mode.store(ColorMode::Xy as u8, Ordering::Relaxed);

                log::debug!(
                    "Colour mode: xy, x={:#06x}, y={:#06x}, \
                 transition={:#06x} (target applied immediately)",
                    x, y, transition,
                );
            }
            COMMAND_MOVE_TO_COLOR_TEMPERATURE => {
                let requested = u16::from_le_bytes([body[0], body[1]]);
                let transition = u16::from_le_bytes([body[2], body[3]]);

                if requested > 0xfeff {
                    return CommandOutcome::Status(Status::InvalidValue);
                }

                let mireds = requested.clamp(MIN_COLOR_MIREDS, MAX_COLOR_MIREDS);

                self.temperature.store(mireds, Ordering::Relaxed);
                self.mode
                    .store(ColorMode::Temperature as u8, Ordering::Relaxed);

                log::debug!(
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
    fn handle(&self, request: &ClusterRequest<'_>, out: &mut [u8]) -> Option<ClusterReply> {
        self.handle_request(request, out)
    }
}
