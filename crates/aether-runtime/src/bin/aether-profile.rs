//! EOS owner setup and static graph catalog.
use std::{env, error::Error, path::PathBuf};
use aether_hal::NodeTypeRegistry;
#[path = "aether_profile/setup.rs"]
mod setup;
#[path = "aether_profile/owner.rs"]
mod owner;
#[path = "common/eos_log.rs"]
mod eos_log;
const PROFILE_FEATURES: [&str; 3] = ["catalog", "setup-v2", "arm-activation-owner"];
fn main() -> Result<(), Box<dyn Error>> {
  if env::args().nth(1).as_deref() == Some("--capabilities") {
    println!(
      "{}",
      serde_json::json!({"product_version":env!("CARGO_PKG_VERSION"),"graph_version":aether_hal::GRAPH_VERSION,
      "node_type_version":"0.2","supported_graph_versions":["0.2.0"],"supported_node_type_versions":["0.2"],
      "supported_node_types":aether_hal::builtin_reader_node_types(),"binary":"aether-profile","features":PROFILE_FEATURES})
    );
    return Ok(());
  }
  let input: Vec<String> = env::args().skip(1).collect();
  let registry = NodeTypeRegistry::builtin();
  match input.as_slice() {
    [verb, flag, socket] if verb == "owner" && flag == "--socket" => {
      owner::serve(&PathBuf::from(socket), &registry)
    }
    [verb, flag] if verb == "setup" && flag == "--json" => setup::run(&registry),
    [verb] if verb == "catalog" => {
      println!(
        "{}",
        serde_json::json!({"types":registry.all().collect::<Vec<_>>(),
        "data_types":registry.data_types().collect::<Vec<_>>(),
        "adapter_families":[],"primitive_specs":[]})
      );
      Ok(())
    }
    _ => {
      Err("usage: aether-profile catalog | setup --json | owner --socket <absolute-path>".into())
    }
  }
}
