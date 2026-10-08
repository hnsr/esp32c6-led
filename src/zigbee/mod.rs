mod clusters;
mod config;
mod runtime;

// The operation the firmware is allowed to call.
pub use runtime::start_zigbee;

// todo:
//   - user levelled logging instead if prinln!
//   - add //! and /// comments once API is narrowed down a bit
//   - extract command/payload parsing logic in the cluster implementations
//   - implement shared lamp-state, so clusters don't have to depend on each other.
//   - implement correct state-changing behavior (minimum leels, with on-off behavior)
