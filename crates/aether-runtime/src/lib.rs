//! Compatibility library for the built-in EOS nodes.
//!
//! The runtime loader and existing installations resolve `libaether_runtime.so`.
//! Built-in node implementations live in `aether-node` while this library keeps
//! the installed ABI and filename stable.
use aether_node::Api;

#[no_mangle]
pub extern "C" fn isos_node_v1() -> *const Api {
  aether_node::api()
}
