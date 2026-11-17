//! Read-only catalog routes served beside the runtime bridge.
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::time::Duration;
use aether_lib::io::read_bounded;
use aether_node_profile::{NodeTypeRegistry, Profile, PRIMITIVE_SPECS};
use serde_json::json;
use crate::transport::http::{read_request, reply};

const MAX_PROFILE_BYTES: usize = 1024 * 1024;

pub fn run(
  path: &Path,
  address: SocketAddr,
  node_types: &[PathBuf],
  terminate: &AtomicBool,
  ready: Sender<Result<(), String>>,
) -> Result<(), Box<dyn std::error::Error>> {
  let bytes = read_bounded(path, MAX_PROFILE_BYTES)?;
  let registry = NodeTypeRegistry::with_contribution_files(node_types)?;
  let source = std::str::from_utf8(&bytes)?;
  let (profile, content_type) = if source.trim_start().starts_with('{') {
    (
      Profile::from_json_with_registry(source, &registry)?,
      "application/json",
    )
  } else {
    (
      Profile::from_toml_with_registry(source, &registry)?,
      "application/toml",
    )
  };
  let summary = json!({"ok":true,"version":profile.version,"robot_id":profile.robot.id,"nodes":profile.all_nodes().count()}).to_string();
  let types = serde_json::to_vec(&registry.all().collect::<Vec<_>>())?;
  let data_types = serde_json::to_vec(&registry.data_types().collect::<Vec<_>>())?;
  let adapter_families = serde_json::to_vec(&registry.catalog_adapter_families())?;
  let primitive_specs = serde_json::to_vec(PRIMITIVE_SPECS)?;
  let catalog = CatalogResponses {
    profile: &bytes,
    content_type,
    summary: &summary,
    types: &types,
    data_types: &data_types,
    adapter_families: &adapter_families,
    primitive_specs: &primitive_specs,
  };
  let listener = TcpListener::bind(address)?;
  listener.set_nonblocking(true)?;
  let _ = ready.send(Ok(()));
  while !terminate.load(Ordering::SeqCst) {
    match listener.accept() {
      Ok((mut stream, _)) => {
        if let Err(error) = serve(&mut stream, &catalog) {
          let body = json!({"ok":false,"error":error}).to_string();
          let _ = reply(&mut stream, 400, "application/json", body.as_bytes());
        }
      }
      Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
        std::thread::sleep(Duration::from_millis(20))
      }
      Err(_) => std::thread::sleep(Duration::from_millis(20)),
    }
  }
  Ok(())
}

struct CatalogResponses<'a> {
  profile: &'a [u8],
  content_type: &'a str,
  summary: &'a str,
  types: &'a [u8],
  data_types: &'a [u8],
  adapter_families: &'a [u8],
  primitive_specs: &'a [u8],
}
fn serve(stream: &mut TcpStream, catalog: &CatalogResponses<'_>) -> Result<(), String> {
  let request = read_request(stream)?;
  let (status, mime, body): (u16, &str, &[u8]) =
    match (request.method.as_str(), request.path.as_str()) {
      ("GET", "/api/eos/v2/config") => (200, catalog.content_type, catalog.profile),
      ("GET", "/api/eos/v2/config/validate") => {
        (200, "application/json", catalog.summary.as_bytes())
      }
      ("GET", "/api/eos/v2/types") => (200, "application/json", catalog.types),
      ("GET", "/api/eos/v2/data-types") => (200, "application/json", catalog.data_types),
      ("GET", "/api/eos/v2/adapter-families") => {
        (200, "application/json", catalog.adapter_families)
      }
      ("GET", "/api/eos/v2/primitive-specs") => (200, "application/json", catalog.primitive_specs),
      ("GET", _) => (
        404,
        "application/json",
        b"{\"ok\":false,\"error\":\"not found\"}",
      ),
      _ => (
        405,
        "application/json",
        b"{\"ok\":false,\"error\":\"method not allowed\"}",
      ),
    };
  reply(stream, status, mime, body).map_err(|error| error.to_string())
}
