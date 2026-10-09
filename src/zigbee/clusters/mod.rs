use crate::zigbee::clusters::color::ColorControlServer;
use crate::zigbee::clusters::level::LevelControlServer;
use crate::zigbee::clusters::on_off::OnOffServer;
use crate::zigbee::config::INPUT_CLUSTERS;
use zigbee::zcl::clusters::general::basic::BasicServer;
use zigbee::zcl::clusters::general::identify::IdentifyServer;
use zigbee::zcl::server::UnsupportedClusterResponder;
use zigbee::zdo::{ClusterReply, ClusterRequest, ClusterRequestHandler};

pub mod color;
pub mod level;
pub mod on_off;

// The zigbee library tries tuple handlers from left to right, so the fallback goes last
pub(super) type Handler = (
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

pub(super) fn build_handler() -> Handler {
    (
        RequestLogger,
        BASIC,
        &IDENTIFY,
        (&ON_OFF, &LEVEL_CONTROL, &COLOR_CONTROL),
        UnsupportedClusterResponder::new(&INPUT_CLUSTERS),
    )
}

pub(super) struct RequestLogger;

impl ClusterRequestHandler for RequestLogger {
    fn handle(&self, request: &ClusterRequest<'_>, _out: &mut [u8]) -> Option<ClusterReply> {
        log::info!(
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

pub static BASIC: BasicServer<'static> = BasicServer {
    // fixme: remove hardcoded values
    zcl_version: 8,
    application_version: 1,
    stack_version: 0,
    hw_version: 1,
    manufacturer_name: "hnsr",
    model_identifier: "esp32c6-led-poc",
    power_source: 0x01,
};
pub static IDENTIFY: IdentifyServer = IdentifyServer::new();
pub static ON_OFF: OnOffServer = OnOffServer::new();
pub static LEVEL_CONTROL: LevelControlServer = LevelControlServer::new(&ON_OFF);
pub static COLOR_CONTROL: ColorControlServer = ColorControlServer::new(&ON_OFF);
