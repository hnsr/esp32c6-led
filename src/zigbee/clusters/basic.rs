use zigbee::zcl::clusters::general::basic::BasicServer;

pub static BASIC: BasicServer<'static> = BasicServer {
    zcl_version: 8,
    application_version: 1,
    stack_version: 0,
    hw_version: 1,
    manufacturer_name: "hnsr",
    model_identifier: "esp32c6-led-poc",
    power_source: 0x01,
};
