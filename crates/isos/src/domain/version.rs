//! Build-bundled version identities; runtime compatibility does not depend on GitHub.

use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct VersionSupport {
  pub schema: String,
  pub product_version: String,
  pub target_graph_format: String,
  pub isos_http_schema: String,
  pub package_contract: String,
  pub package_schema: String,
}

const BUNDLED: &str = include_str!("../../../../config/version-support.toml");

pub(crate) fn bundled() -> Result<VersionSupport> {
  let map: VersionSupport = toml::from_str(BUNDLED)?;
  ensure!(
    map.schema == "isos.version-support@v2",
    "unsupported version support map"
  );
  ensure!(
    map.product_version == env!("CARGO_PKG_VERSION"),
    "version support product mismatch"
  );
  ensure!(
    map.target_graph_format == "0.2.1",
    "version support graph target mismatch"
  );
  ensure!(
    map.isos_http_schema == "0.1.0",
    "version support HTTP schema mismatch"
  );
  ensure!(
    map.package_contract == "isos.package@v1" && map.package_schema == "0.2.1",
    "version support package contract mismatch"
  );
  Ok(map)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn bundled_map_matches_build_and_known_contracts() {
    let map = bundled().unwrap();
    assert_eq!(map.product_version, "0.2.3");
  }

  #[test]
  fn map_rejects_unknown_fields() {
    let invalid = format!("{BUNDLED}\nunexpected = true\n");
    assert!(toml::from_str::<VersionSupport>(&invalid).is_err());
  }
}
