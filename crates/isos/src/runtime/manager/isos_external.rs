//! Direct IsOS children for the narrow `isos.ui.external.v1` Unix-socket interface.
//! The receipt survives manager restarts; Stop requires both a fenced reply and child exit.

use crate::{
  app::config::{self, Paths},
  domain::packages::{self, VerifiedPackage},
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
  fs,
  io::{BufRead, BufReader, Read, Write},
  os::unix::{
    fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    net::UnixStream,
  },
  path::PathBuf,
  process::{Command, Stdio},
  thread,
  time::{Duration, Instant},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Child {
  node_path: String,
  node_type: String,
  pid: u32,
  start_ticks: u64,
  socket: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
struct Receipt {
  run_id: String,
  revision: String,
  children: Vec<Child>,
}

fn receipt_path(paths: &Paths, name: &str) -> PathBuf {
  paths.runtime.join(format!("{name}.isos-external.json"))
}

fn child_identity(pid: u32) -> Result<(char, u64)> {
  let stat = fs::read_to_string(format!("/proc/{pid}/stat"))?;
  let fields: Vec<_> = stat
    .rsplit_once(") ")
    .context("invalid child process identity")?
    .1
    .split_whitespace()
    .collect();
  Ok((
    fields
      .first()
      .context("missing child process state")?
      .chars()
      .next()
      .context("empty child process state")?,
    fields
      .get(19)
      .context("missing child process start time")?
      .parse()?,
  ))
}

fn alive(child: &Child) -> bool {
  child_identity(child.pid).is_ok_and(|(state, ticks)| state != 'Z' && ticks == child.start_ticks)
}

fn read_receipt(paths: &Paths, name: &str) -> Result<Option<Receipt>> {
  let path = receipt_path(paths, name);
  let metadata = match fs::symlink_metadata(&path) {
    Ok(metadata) => metadata,
    Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
    Err(error) => return Err(error.into()),
  };
  ensure!(
    metadata.file_type().is_file()
      && metadata.uid() == unsafe { libc::geteuid() }
      && metadata.nlink() == 1
      && metadata.permissions().mode() & 0o7777 == 0o600
      && metadata.len() <= 65536,
    "unsafe IsOS external receipt"
  );
  let file = fs::OpenOptions::new()
    .read(true)
    .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
    .open(path)?;
  let opened = file.metadata()?;
  ensure!(
    metadata.ino() == opened.ino() && metadata.dev() == opened.dev(),
    "IsOS external receipt changed while opening"
  );
  let mut bytes = Vec::new();
  file.take(65537).read_to_end(&mut bytes)?;
  ensure!(bytes.len() <= 65536, "IsOS external receipt is too large");
  Ok(Some(serde_json::from_slice(&bytes)?))
}

fn rpc(child: &Child, run_id: &str, revision: &str, op: &str, payload: Value) -> Result<Value> {
  let mut stream = UnixStream::connect(&child.socket)?;
  stream.set_read_timeout(Some(Duration::from_secs(2)))?;
  stream.set_write_timeout(Some(Duration::from_secs(2)))?;
  let request = json!({"schema_version":1,"op":op,"run_id":run_id,"revision":revision,"node_path":child.node_path,"payload":payload});
  let bytes = serde_json::to_vec(&request)?;
  ensure!(bytes.len() <= 65536, "external node request exceeds 64 KiB");
  stream.write_all(&bytes)?;
  stream.write_all(b"\n")?;
  let mut line = Vec::new();
  BufReader::new(stream)
    .take(65537)
    .read_until(b'\n', &mut line)?;
  ensure!(
    !line.is_empty() && line.len() <= 65536 && line.last() == Some(&b'\n'),
    "external node reply is unbounded or incomplete"
  );
  let reply: Value = serde_json::from_slice(&line)?;
  ensure!(
    reply["schema_version"] == 1
      && reply["run_id"] == run_id
      && reply["revision"] == revision
      && reply["node_path"] == child.node_path
      && reply["ok"] == true,
    "external node rejected {op} or returned a stale fence"
  );
  Ok(reply)
}

fn declarations(
  graph: &Value,
  packages: &[VerifiedPackage],
) -> Result<Vec<(String, String, PathBuf)>> {
  let mut result = Vec::new();
  let Some(layers) = graph
    .pointer("/cores/isos/layers")
    .and_then(Value::as_object)
  else {
    return Ok(result);
  };
  for (layer, layer_value) in layers {
    for (group, group_value) in layer_value["groups"]
      .as_object()
      .into_iter()
      .flat_map(|groups| groups.iter())
    {
      for (id, node) in group_value["nodes"]
        .as_object()
        .into_iter()
        .flat_map(|nodes| nodes.iter())
      {
        let Some(package_id) = node.pointer("/package/id").and_then(Value::as_str) else {
          continue;
        };
        let version = node["package"]["version"]
          .as_str()
          .context("external node lacks package version")?;
        let node_type = node["node_type"]
          .as_str()
          .context("external node lacks type")?;
        let package = packages
          .iter()
          .find(|package| package.release.id == package_id && package.release.version == version)
          .context("external node package is not verified")?;
        let contributions = package
          .release
          .node_contributions
          .as_ref()
          .context("external node lacks contributions")?;
        let definition = contributions
          .node_types
          .iter()
          .find(|definition| definition["id"] == node_type)
          .context("external node type is not contributed by its exact package")?;
        if definition["implementation"]["kind"] != "external" {
          continue;
        }
        ensure!(
          definition["implementation"]
            == json!({"kind":"external","interface":"isos.ui.external.v1","version":"1","lifecycle_adapter":"unix-socket-v1"}),
          "IsOS external node interface is unsupported"
        );
        let declaration = contributions
          .processes
          .iter()
          .find(|process| process.node_type == node_type)
          .context("IsOS external node lacks a process declaration")?;
        ensure!(
          declaration.lifecycle_adapter == "unix-socket-v1",
          "IsOS external process adapter is unsupported"
        );
        let entrypoint = packages::checked_file(&package.root, &declaration.entrypoint, true)?;
        result.push((
          format!("isos/{layer}/{group}/{id}"),
          node_type.to_owned(),
          entrypoint,
        ));
      }
    }
  }
  Ok(result)
}

pub(crate) fn supports(graph: &Value, packages: &[VerifiedPackage]) -> Result<()> {
  let _ = declarations(graph, packages)?;
  Ok(())
}

pub(crate) fn start(
  paths: &Paths,
  name: &str,
  graph: &Value,
  run_id: &str,
  revision: &str,
  packages: &[VerifiedPackage],
) -> Result<()> {
  ensure!(
    read_receipt(paths, name)?.is_none(),
    "IsOS external child receipt is already present"
  );
  let nodes = declarations(graph, packages)?;
  if nodes.is_empty() {
    return Ok(());
  }
  let mut receipt = Receipt {
    run_id: run_id.to_owned(),
    revision: revision.to_owned(),
    children: Vec::new(),
  };
  config::atomic_write(&receipt_path(paths, name), &serde_json::to_vec(&receipt)?)?;
  for (node_path, node_type, entrypoint) in nodes {
    let hash = hex::encode(Sha256::digest(node_path.as_bytes()));
    let socket = paths
      .runtime
      .join(format!("{name}.ui-{}.sock", &hash[..16]));
    ensure!(!socket.exists(), "external node socket already exists");
    let mut process = Command::new(entrypoint)
      .env("ISOS_NODE_SOCKET", &socket)
      .env("ISOS_NODE_PATH", &node_path)
      .env("ISOS_RUN_ID", run_id)
      .env("ISOS_GRAPH_REVISION", revision)
      .stdin(Stdio::null())
      .stdout(Stdio::null())
      .stderr(Stdio::null())
      .spawn()?;
    let start_ticks = match child_identity(process.id()) {
      Ok((_, ticks)) => ticks,
      Err(error) => {
        let _ = process.kill();
        let _ = process.wait();
        let _ = fs::remove_file(&socket);
        return Err(error);
      }
    };
    let child = Child {
      node_path,
      node_type,
      pid: process.id(),
      start_ticks,
      socket,
    };
    receipt.children.push(child.clone());
    if let Err(error) =
      config::atomic_write(&receipt_path(paths, name), &serde_json::to_vec(&receipt)?)
    {
      let _ = process.kill();
      let _ = process.wait();
      let _ = fs::remove_file(&child.socket);
      return Err(error);
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    let readiness = loop {
      if !alive(&child) {
        break Err(anyhow::anyhow!("external node exited before readiness"));
      }
      if rpc(&child, run_id, revision, "health", Value::Null).is_ok() {
        break Ok(());
      }
      if Instant::now() >= deadline {
        break Err(anyhow::anyhow!("external node did not become ready"));
      }
      thread::sleep(Duration::from_millis(50));
    };
    if let Err(error) = readiness {
      let _ = process.kill();
      let _ = process.wait();
      return Err(error);
    }
  }
  Ok(())
}

pub(crate) fn stop(paths: &Paths, name: &str, run_id: &str, revision: &str) -> Result<()> {
  let Some(receipt) = read_receipt(paths, name)? else {
    return Ok(());
  };
  ensure!(
    receipt.run_id == run_id && receipt.revision == revision,
    "IsOS external child receipt has a stale fence"
  );
  for child in receipt.children.iter().rev() {
    if alive(child) {
      rpc(child, run_id, revision, "stop", Value::Null)?;
      let deadline = Instant::now() + Duration::from_secs(5);
      while alive(child) && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(50));
      }
      ensure!(
        !alive(child),
        "external node Stop was acknowledged but child remains alive"
      );
    }
    let _ = fs::remove_file(&child.socket);
  }
  fs::remove_file(receipt_path(paths, name))?;
  Ok(())
}

pub(crate) fn status(paths: &Paths, name: &str, run_id: &str, revision: &str) -> Result<Value> {
  let Some(receipt) = read_receipt(paths, name)? else {
    return Ok(json!({"ready":true,"children":[]}));
  };
  ensure!(
    receipt.run_id == run_id && receipt.revision == revision,
    "IsOS external child receipt has a stale fence"
  );
  let children: Vec<_> = receipt
    .children
    .iter()
    .map(|child| {
      let ready = alive(child) && rpc(child, run_id, revision, "health", Value::Null).is_ok();
    json!({"id":child.node_path,"core":"isos","node_path":child.node_path,"node_type":child.node_type,"ready":ready,"state":if ready {"running"} else {"unavailable"},"restarts":0,"pid":child.pid})
    })
    .collect();
  Ok(json!({"ready":children.iter().all(|child| child["ready"] == true),"children":children}))
}

pub(crate) fn request(
  paths: &Paths,
  name: &str,
  run_id: &str,
  revision: &str,
  node_path: &str,
  op: &str,
  payload: Value,
) -> Result<Value> {
  ensure!(
    matches!(op, "input" | "read"),
    "unsupported external node operation"
  );
  let receipt = read_receipt(paths, name)?.context("IsOS external node is not running")?;
  ensure!(
    receipt.run_id == run_id && receipt.revision == revision,
    "IsOS external node request has a stale fence"
  );
  let child = receipt
    .children
    .iter()
    .find(|child| child.node_path == node_path)
    .context("IsOS external node was not started")?;
  ensure!(alive(child), "IsOS external node exited");
  rpc(child, run_id, revision, op, payload)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::domain::{packages::Receipt as PackageReceipt, registry::release::Release};

  #[test]
  fn verified_external_child_runs_inputs_reads_and_stops() {
    let temp = tempfile::tempdir().unwrap();
    let paths = Paths {
      config: temp.path().join("config"),
      data: temp.path().join("data"),
      runtime: temp.path().join("run"),
    };
    paths.prepare().unwrap();
    let root = temp.path().join("package");
    fs::create_dir_all(root.join("bin")).unwrap();
    let entrypoint = root.join("bin/node");
    fs::write(&entrypoint, r#"#!/usr/bin/env python3
import json, os, socket
s = socket.socket(socket.AF_UNIX)
s.bind(os.environ['ISOS_NODE_SOCKET'])
s.listen(8)
value = {'count': 0}
while True:
    c, _ = s.accept()
    with c:
        request = json.loads(c.makefile().readline())
        if request['op'] == 'input': value = request['payload']['value']
        reply = {'schema_version': 1, 'ok': True, 'run_id': request['run_id'], 'revision': request['revision'], 'node_path': request['node_path'], 'payload': value}
        c.sendall((json.dumps(reply) + '\n').encode())
        if request['op'] == 'stop': break
s.close()
"#).unwrap();
    let mut permissions = fs::metadata(&entrypoint).unwrap().permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&entrypoint, permissions).unwrap();
    let release: Release = serde_json::from_value(json!({
      "contract":"island.package.release","schema_version":"0.2.0","id":"community/panel","kind":"program","version":"1.0.0","source_commit":"0123456789012345678901234567890123456789","assets":[],
      "node_contributions":{"node_types":[{"id":"isos.ui.community.panel@v0.2","implementation":{"kind":"external","interface":"isos.ui.external.v1","version":"1","lifecycle_adapter":"unix-socket-v1"}}],
      "processes":[{"node_type":"isos.ui.community.panel@v0.2","entrypoint":"bin/node","lifecycle_adapter":"unix-socket-v1","resources":[]}]}
    })).unwrap();
    let package = VerifiedPackage {
      root: root.clone(),
      store: root,
      release,
      receipt: PackageReceipt {
        schema_version: 2,
        id: "community/panel".into(),
        version: "1.0.0".into(),
        source_commit: None,
        asset_sha256: None,
        local_link: None,
        source_checkout: None,
        source_kind: None,
        release_sha256: None,
        payload_sha256: None,
        sidecar_sha256: None,
      },
      sidecar: None,
    };
    let graph: Value = serde_json::from_str(r#"{"version":"0.2.1","cores":{"isos":{"layers":{"ui":{"groups":{"community":{"nodes":{"panel":{"id":"panel","node_type":"isos.ui.community.panel@v0.2","package":{"id":"community/panel","version":"1.0.0"}}}}}}}}}}"#).unwrap();
    supports(&graph, &[package]).unwrap();
    let packages = [VerifiedPackage { root: temp.path().join("package"), store: temp.path().join("package"), release: serde_json::from_value(json!({
      "contract":"island.package.release","schema_version":"0.2.0","id":"community/panel","kind":"program","version":"1.0.0","source_commit":"0123456789012345678901234567890123456789","assets":[],
      "node_contributions":{"node_types":[{"id":"isos.ui.community.panel@v0.2","implementation":{"kind":"external","interface":"isos.ui.external.v1","version":"1","lifecycle_adapter":"unix-socket-v1"}}],
      "processes":[{"node_type":"isos.ui.community.panel@v0.2","entrypoint":"bin/node","lifecycle_adapter":"unix-socket-v1","resources":[]}]}
    })).unwrap(), receipt: PackageReceipt { schema_version: 2, id: "community/panel".into(), version: "1.0.0".into(), source_commit: None, asset_sha256: None, local_link: None, source_checkout: None, source_kind: None, release_sha256: None, payload_sha256: None, sidecar_sha256: None }, sidecar: None }];
    start(&paths, "default", &graph, "r1", &"a".repeat(64), &packages).unwrap();
    assert_eq!(
      status(&paths, "default", "r1", &"a".repeat(64)).unwrap()["ready"],
      true
    );
    let node_path = "isos/ui/community/panel";
    request(
      &paths,
      "default",
      "r1",
      &"a".repeat(64),
      node_path,
      "input",
      json!({"port":"command","value":{"count":7}}),
    )
    .unwrap();
    assert_eq!(
      request(
        &paths,
        "default",
        "r1",
        &"a".repeat(64),
        node_path,
        "read",
        json!({"port":"state","value":null})
      )
      .unwrap()["payload"]["count"],
      7
    );
    assert!(
      request(
        &paths,
        "default",
        "old",
        &"a".repeat(64),
        node_path,
        "read",
        Value::Null
      )
      .is_err()
    );
    stop(&paths, "default", "r1", &"a".repeat(64)).unwrap();
    assert!(!receipt_path(&paths, "default").exists());

    // A child can exit before Stop. Its owned socket must still be removed.
    let stale_socket = paths.runtime.join("stale-external.sock");
    let listener = std::os::unix::net::UnixListener::bind(&stale_socket).unwrap();
    let stale = Receipt {
      run_id: "r2".into(),
      revision: "b".repeat(64),
      children: vec![Child {
        node_path: node_path.into(),
        node_type: "isos.ui.community.panel@v0.2".into(),
        pid: 0,
        start_ticks: 0,
        socket: stale_socket.clone(),
      }],
    };
    config::atomic_write(
      &receipt_path(&paths, "default"),
      &serde_json::to_vec(&stale).unwrap(),
    )
    .unwrap();
    stop(&paths, "default", "r2", &"b".repeat(64)).unwrap();
    drop(listener);
    assert!(!stale_socket.exists());

    // An immediately exiting child cannot leave a live process or a stuck socket.
    fs::write(&entrypoint, b"#!/bin/sh\nexit 0\n").unwrap();
    assert!(start(&paths, "default", &graph, "r3", &"c".repeat(64), &packages).is_err());
    let receipt = read_receipt(&paths, "default").unwrap().unwrap();
    assert!(receipt.children.iter().all(|child| !alive(child)));
    stop(&paths, "default", "r3", &"c".repeat(64)).unwrap();
    assert!(!receipt_path(&paths, "default").exists());
  }
}
