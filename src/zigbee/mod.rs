mod clusters;
mod config;
mod runtime;

pub use runtime::start_zigbee;

// todo:
//   - add //! and /// comments once API is narrowed down a bit
//   - extract command/payload parsing logic in the cluster implementations
//   - implement shared lamp-state, so clusters don't have to depend on each other.
//   - implement correct state-changing behavior (minimum leels, with on-off behavior)
