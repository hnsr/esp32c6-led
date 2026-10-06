use esp_println::println;
use zigbee::zcl::clusters::general::basic::BasicServer;
use zigbee::zcl::clusters::general::identify::IdentifyServer;
use zigbee::zcl::server::UnsupportedClusterResponder;
use zigbee::zdo::{ClusterReply, ClusterRequest, ClusterRequestHandler};
use crate::zigbee::clusters::color::ColorControlServer;
use crate::zigbee::clusters::level::LevelControlServer;
use crate::zigbee::clusters::on_off::OnOffServer;

pub mod basic;
pub mod identify;
pub mod on_off;
pub mod level;
pub mod color;

pub type Handler = (
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

pub struct RequestLogger;

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

// FIXME: also re-export BASIC/IDENTIFY here somehow for consistency?
//        Or just instantiate them here since they're trivial?
pub static ON_OFF: OnOffServer = OnOffServer::new();
pub static LEVEL_CONTROL: LevelControlServer = LevelControlServer::new(&ON_OFF);
pub static COLOR_CONTROL: ColorControlServer = ColorControlServer::new(&ON_OFF);