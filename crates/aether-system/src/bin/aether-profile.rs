//! EOS owner setup and static graph catalog.
use std::{
  env,
  error::Error,
  fs,
  path::{Path, PathBuf},
};
use aether_hal::{NodeTypeRegistry, Profile};
#[path = "aether_profile/setup.rs"]
mod setup;
#[path = "aether_profile/owner.rs"]
mod owner;
#[path = "aether_profile/owner_log.rs"]
mod owner_log;
#[path = "aether_profile/package_runtime.rs"]
mod package_runtime;
#[path = "aether_profile/package_registry.rs"]
mod package_registry;
#[path = "aether_profile/external_dispatcher.rs"]
mod external_dispatcher;
use crate::eos_log;
const PROFILE_FEATURES: [&str; 4] = [
  "catalog",
  "setup-v3",
  "arm-activation-owner",
  "validate-file",
];

fn validate_file(path: &Path, registry: &NodeTypeRegistry) -> Result<(), Box<dyn Error>> {
  if fs::metadata(path)?.len() > 1024 * 1024 {
    return Err("EOS graph exceeds 1 MiB".into());
  }
  let bytes = fs::read(path)?;
  if bytes.len() > 1024 * 1024 {
    return Err("EOS graph exceeds 1 MiB".into());
  }
  let graph: serde_json::Value = serde_json::from_slice(&bytes)?;
  if graph["version"] != aether_hal::GRAPH_VERSION
    || graph["cores"]
      .as_object()
      .is_none_or(|cores| cores.len() != 1 || !cores.contains_key("eos"))
  {
    return Err("expected EOS-only graph 0.2.1 projection".into());
  }
  let profile = Profile::from_json_with_registry(&serde_json::to_string(&graph)?, registry)?;
  setup::execution_mode(&profile, registry)
    .map_err(|reason| format!("unsupported_execution: {reason}"))?;
  Ok(())
}
pub fn main() -> Result<(), Box<dyn Error>> {
  if env::args().nth(1).as_deref() == Some("--capabilities") {
    println!(
      "{}",
      serde_json::json!({"product_version":env!("CARGO_PKG_VERSION"),"graph_version":aether_hal::GRAPH_VERSION,
      "node_type_version":"0.2","supported_graph_versions":["0.2.1"],"supported_node_type_versions":["0.2"],
      "supported_node_types":aether_hal::builtin_reader_node_types(),"binary":"aether-profile","features":PROFILE_FEATURES})
    );
    return Ok(());
  }
  let mut input = Vec::new();
  let mut contributions = Vec::new();
  let mut adapter_arguments = Vec::new();
  let mut process_argument = None;
  let mut args = env::args().skip(1);
  while let Some(argument) = args.next() {
    if argument == "--node-types" {
      contributions.push(PathBuf::from(
        args.next().ok_or("--node-types requires a file")?,
      ));
    } else if argument == "--adapter-library" {
      adapter_arguments.push(
        args
          .next()
          .ok_or("--adapter-library requires family=path@sha256")?,
      );
    } else if argument == "--package-process" {
      if process_argument.is_some() {
        return Err("only one package process handoff is supported".into());
      }
      process_argument = Some(PathBuf::from(
        args.next().ok_or("--package-process requires a file")?,
      ));
    } else {
      input.push(argument);
    }
  }
  let registry = NodeTypeRegistry::with_contribution_files(&contributions)?;
  let mut package_inputs = package_runtime::PackageInputs {
    sidecars: contributions.clone(),
    ..Default::default()
  };
  for argument in adapter_arguments {
    let (family, adapter) = package_runtime::parse_adapter(&argument)?;
    if package_inputs.adapters.insert(family, adapter).is_some() {
      return Err("duplicate adapter library family".into());
    }
  }
  if let Some(path) = process_argument {
    package_inputs.process = Some(package_runtime::parse_process(&path, &contributions)?);
  }
  package_runtime::validate_inputs(&package_inputs, &registry)?;
  if !matches!(input.as_slice(), [verb, flag, _] if verb == "owner" && flag == "--socket")
    && (!package_inputs.adapters.is_empty() || package_inputs.process.is_some())
  {
    return Err("package runtime arguments require owner --socket".into());
  }
  match input.as_slice() {
    [verb, flag, file] if verb == "validate" && flag == "--file" => {
      validate_file(Path::new(file), &registry)
    }
    [verb, flag, socket] if verb == "owner" && flag == "--socket" => {
      owner::serve_with_inputs(&PathBuf::from(socket), &registry, &package_inputs)
    }
    [verb, flag] if verb == "setup" && flag == "--json" => setup::run(&registry),
    [verb] if verb == "catalog" => {
      println!(
        "{}",
        serde_json::json!({"types":registry.all().collect::<Vec<_>>(),
        "data_types":registry.data_types().collect::<Vec<_>>(),
        "adapter_families":registry.catalog_adapter_families(),"primitive_specs":[],
        "port_routes":registry.catalog_port_routes(),"bridge_adapters":registry.catalog_bridge_adapters()})
      );
      Ok(())
    }
    _ => {
      Err("usage: aether-profile catalog | validate --file <graph> | setup --json | owner --socket <absolute-path> [--node-types <sidecar>]... [--adapter-library family=path@sha256] [--package-process <file>]".into())
    }
  }
}
