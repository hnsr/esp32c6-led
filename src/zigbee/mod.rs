mod clusters;
mod config;
mod runtime;

// The operation the firmware is allowed to call.
pub use runtime::start_zigbee;