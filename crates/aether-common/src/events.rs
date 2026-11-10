//! Fixed, bounded EOS event constructors for local JSONL logging.

use crate::time::Clock;
use serde_json::{Value, json};
use std::io::{self, Write};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

// Keep legacy event constructors within the local JSONL record bound.
const MAX_LINE: usize = 4096;

/// Compatibility adapter for older EOS event constructors. Production
/// processes write their events into the same local per-core JSONL file.
pub struct LocalCoreLog;

impl LocalCoreLog {
  pub fn new() -> io::Result<Self> {
    Ok(Self)
  }
}

impl Write for LocalCoreLog {
  fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
    let event: Value = serde_json::from_slice(buf.strip_suffix(b"\n").unwrap_or(buf))?;
    let source = event["source"].as_str().unwrap_or("runtime");
    let record = serde_json::json!({
      "timestamp": event["timestamp"],
      "level": event["level"],
      "source": format!("eos.{source}@aether-v{}", env!("CARGO_PKG_VERSION")),
      "message": event["message"],
      "event_code": event["event_code"],
      "run_id": event["run_id"],
      "correlation_id": event["correlation_id"],
      "node_path": event["node_path"],
      "fields": event["fields"],
    });
    crate::diagnostics::append_record(&crate::diagnostics::directory(), &record)?;
    Ok(buf.len())
  }

  fn flush(&mut self) -> io::Result<()> {
    Ok(())
  }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Source {
  Eos,
  Bridge,
  Config,
}

impl Source {
  fn as_str(self) -> &'static str {
    match self {
      Self::Eos => "eos",
      Self::Bridge => "bridge",
      Self::Config => "config",
    }
  }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Stage {
  Options,
  Profile,
  Registry,
  Adapter,
  Journal,
  Backend,
  Listener,
  Stop,
}

impl Stage {
  fn as_str(self) -> &'static str {
    match self {
      Self::Options => "options",
      Self::Profile => "profile",
      Self::Registry => "registry",
      Self::Adapter => "adapter",
      Self::Journal => "journal",
      Self::Backend => "backend",
      Self::Listener => "listener",
      Self::Stop => "stop",
    }
  }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Reason {
  ProfileInvalid,
  RegistryInvalid,
  AdapterUnavailable,
  JournalUnavailable,
  BackendStartFailed,
  ChildExited,
  ListenerBindFailed,
  StopUnconfirmed,
  OtherFailure,
}

impl Reason {
  fn as_str(self) -> &'static str {
    match self {
      Self::ProfileInvalid => "profile_invalid",
      Self::RegistryInvalid => "registry_invalid",
      Self::AdapterUnavailable => "adapter_unavailable",
      Self::JournalUnavailable => "journal_unavailable",
      Self::BackendStartFailed => "backend_start_failed",
      Self::ChildExited => "child_exited",
      Self::ListenerBindFailed => "listener_bind_failed",
      Self::StopUnconfirmed => "stop_unconfirmed",
      Self::OtherFailure => "other_failure",
    }
  }
}

pub fn valid_run_id(value: &str) -> bool {
  (1..=64).contains(&value.len())
    && value
      .bytes()
      .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

pub fn local_run_id() -> String {
  let nanos = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap_or_default()
    .as_nanos();
  format!("local_{}_{}", std::process::id(), nanos)
}

pub fn valid_digest(value: &str) -> bool {
  value.len() == 64
    && value
      .bytes()
      .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Expand an admitted four-segment identity to a saved-graph JSON Pointer.
pub fn graph_pointer(path: &str) -> Option<String> {
  let parts: Vec<_> = path.split('/').collect();
  let segments = match parts.as_slice() {
    ["isos", layer, group, node] => [*layer, *group, *node],
    ["cores", "isos", "layers", layer, "groups", group, "nodes", node] => [*layer, *group, *node],
    _ => return None,
  };
  if !segments.iter().all(|part| {
    !part.is_empty()
      && part
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
  }) {
    return None;
  }
  let escape = |part: &str| part.replace('~', "~0").replace('/', "~1");
  let pointer = format!(
    "/cores/isos/layers/{}/groups/{}/nodes/{}",
    escape(segments[0]),
    escape(segments[1]),
    escape(segments[2])
  );
  (pointer.len() <= 512).then_some(pointer)
}

#[derive(Clone, Copy, Debug)]
pub enum Event<'a> {
  Starting,
  Ready {
    node_count: u32,
  },
  StartFailed {
    stage: Stage,
    reason: Reason,
  },
  NodeMismatch {
    mismatch_code: &'a str,
  },
  Stopping,
  Stopped,
  PackageAdmitted {
    release_sha256: &'a str,
    payload_sha256: &'a str,
  },
  BackendReady,
  BackendLost,
  ListenerLost,
  StopUnconfirmed,
  SkillStopUnconfirmed,
  SkillExecution {
    execution_id: &'a str,
    package_id: &'a str,
    package_version: &'a str,
    stage: &'a str,
    reason_code: &'a str,
    worker_reaped: Option<bool>,
    worker_exit_code: Option<i32>,
  },
  BridgePrimitiveResult {
    status: &'a str,
    phase: &'a str,
  },
}

struct Inner<W> {
  writer: W,
  failed: bool,
}

struct WireEvent<'a> {
  level: &'a str,
  code: &'a str,
  node_path: Option<&'a str>,
  correlation_id: Option<&'a str>,
  message: &'a str,
  fields: Value,
}

/// The only public event sink. Writes exactly one bounded JSON object per line.
pub struct EventSink<W: Write, C: Clock> {
  source: Source,
  run_id: String,
  _clock: C,
  inner: Mutex<Inner<W>>,
}

impl<W: Write, C: Clock> EventSink<W, C> {
  pub fn new(source: Source, run_id: String, writer: W, clock: C) -> Result<Self, &'static str> {
    if !valid_run_id(&run_id) {
      return Err("invalid run ID");
    }
    Ok(Self {
      source,
      run_id,
      _clock: clock,
      inner: Mutex::new(Inner {
        writer,
        failed: false,
      }),
    })
  }

  pub fn emit(&self, event: Event<'_>, node_path: Option<&str>) -> io::Result<()> {
    self.emit_correlated(event, node_path, None)
  }

  pub fn emit_correlated(
    &self,
    event: Event<'_>,
    node_path: Option<&str>,
    correlation_id: Option<&str>,
  ) -> io::Result<()> {
    if correlation_id.is_some_and(|id| !valid_run_id(id)) {
      return Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        "invalid correlation ID",
      ));
    }
    let pointer = node_path
      .map(graph_pointer)
      .transpose_option()
      .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid graph node path"))?;
    let mut inner = self
      .inner
      .lock()
      .map_err(|_| io::Error::other("event lock poisoned"))?;
    if inner.failed {
      return Err(io::Error::new(
        io::ErrorKind::BrokenPipe,
        "event stream disabled",
      ));
    }
    let result = self.write_event(&mut inner.writer, event, pointer.as_deref(), correlation_id);
    inner.failed = result.is_err();
    result
  }

  fn write_event(
    &self,
    writer: &mut W,
    event: Event<'_>,
    node_path: Option<&str>,
    correlation_id: Option<&str>,
  ) -> io::Result<()> {
    let prefix = self.source.as_str();
    if self.source != Source::Eos
      && matches!(
        event,
        Event::PackageAdmitted { .. }
          | Event::BackendReady
          | Event::BackendLost
          | Event::ListenerLost
          | Event::StopUnconfirmed
          | Event::SkillStopUnconfirmed
          | Event::SkillExecution { .. }
          | Event::NodeMismatch { .. }
      )
    {
      return Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        "backend event source differs",
      ));
    }
    let (level, code, message, fields) = match event {
      Event::Starting => (
        "info",
        format!("{prefix}_starting"),
        format!("EOS {prefix} starting"),
        json!({"state":"starting"}),
      ),
      Event::Ready { node_count } => (
        "info",
        format!("{prefix}_ready"),
        if self.source == Source::Eos {
          "EOS runtime ready".into()
        } else {
          format!("EOS {prefix} ready")
        },
        if self.source == Source::Eos {
          json!({"state":"ready","node_count":node_count})
        } else {
          json!({"state":"ready"})
        },
      ),
      Event::StartFailed { stage, reason } => (
        "error",
        format!("{prefix}_start_failed"),
        format!("EOS {prefix} failed to start"),
        json!({"state":"failed","stage":stage.as_str(),"reason_code":reason.as_str()}),
      ),
      Event::NodeMismatch { mismatch_code } => {
        if !matches!(
          mismatch_code,
          "unknown_type" | "version" | "package" | "adapter" | "placement"
        ) {
          return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid node mismatch code",
          ));
        }
        (
          "error",
          "eos_node_mismatch".into(),
          "EOS graph node does not match its registered type".into(),
          json!({"state":"failed","mismatch_code":mismatch_code}),
        )
      }
      Event::Stopping => (
        "info",
        format!("{prefix}_stopping"),
        format!("EOS {prefix} stopping"),
        json!({"state":"stopping"}),
      ),
      Event::Stopped => (
        "info",
        format!("{prefix}_stopped"),
        format!("EOS {prefix} stopped"),
        json!({"state":"stopped"}),
      ),
      Event::PackageAdmitted {
        release_sha256,
        payload_sha256,
      } => {
        if self.source != Source::Eos
          || !valid_digest(release_sha256)
          || !valid_digest(payload_sha256)
        {
          return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid package provenance",
          ));
        }
        (
          "info",
          "eos_package_admitted".into(),
          "EOS backend package admitted".into(),
          json!({"package_id":"island/mujoco-sim","package_version":"0.1.1","release_sha256":release_sha256,"payload_sha256":payload_sha256}),
        )
      }
      Event::BackendReady => (
        "info",
        "eos_backend_ready".into(),
        "EOS backend child ready".into(),
        json!({"state":"ready"}),
      ),
      Event::BackendLost => (
        "error",
        "eos_backend_lost".into(),
        "EOS backend child exited".into(),
        json!({"state":"failed","reason_code":"child_exited"}),
      ),
      Event::ListenerLost => (
        "error",
        "eos_listener_lost".into(),
        "EOS gRPC listener exited unexpectedly".into(),
        json!({"state":"failed","reason_code":"listener_lost"}),
      ),
      Event::StopUnconfirmed => (
        "error",
        "eos_stop_unconfirmed".into(),
        "EOS actuator stop unconfirmed".into(),
        json!({"state":"failed","reason_code":"stop_unconfirmed"}),
      ),
      Event::SkillStopUnconfirmed => (
        "error",
        "eos_skill_stop_unconfirmed".into(),
        "EOS native skill stop unconfirmed".into(),
        json!({"state":"failed","reason_code":"skill_stop_unconfirmed"}),
      ),
      Event::SkillExecution {
        execution_id,
        package_id,
        package_version,
        stage,
        reason_code,
        worker_reaped,
        worker_exit_code,
      } => {
        let safe_atom = |value: &str| {
          !value.is_empty()
            && value.bytes().all(|byte| {
              byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'.' | b'_' | b'-')
            })
        };
        if self.source != Source::Eos
          || execution_id.len() > 80
          || !execution_id.starts_with("ex-")
          || !safe_atom(execution_id)
          || package_id.len() > 96
          || !package_id.split('/').all(safe_atom)
          || package_version.len() > 64
          || !package_version
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_digit())
          || !package_version
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+'))
          || !matches!(stage, "queued" | "running" | "completed" | "failed")
          || reason_code.len() > 128
          || !reason_code.starts_with("worker.")
          || !safe_atom(reason_code)
        {
          return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid skill event",
          ));
        }
        (
          if stage == "failed" { "error" } else { "info" },
          "eos_skill_execution".into(),
          "EOS native skill execution lifecycle".into(),
          json!({"execution_id":execution_id,"package_id":package_id,"package_version":package_version,
            "stage":stage,"reason_code":reason_code,"worker_reaped":worker_reaped,"worker_exit_code":worker_exit_code}),
        )
      }
      Event::BridgePrimitiveResult { status, phase } => {
        if self.source != Source::Bridge
          || node_path.is_some()
          || correlation_id.is_none()
          || !matches!(
            status,
            "Completed" | "Failed" | "Cancelled" | "Rejected" | "InProgress" | "OutcomeUnknown"
          )
          || !matches!(phase, "submit" | "followup")
        {
          return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid bridge result event",
          ));
        }
        (
          if matches!(status, "Failed" | "Rejected" | "OutcomeUnknown") {
            "warn"
          } else {
            "info"
          },
          "bridge_primitive_result".into(),
          "EOS browser primitive result".into(),
          json!({"status":status,"phase":phase}),
        )
      }
    };
    self.write(
      writer,
      WireEvent {
        level,
        code: &code,
        node_path,
        correlation_id,
        message: &message,
        fields,
      },
    )
  }

  fn write(&self, writer: &mut W, event: WireEvent<'_>) -> io::Result<()> {
    let WireEvent {
      level,
      code,
      node_path,
      correlation_id,
      message,
      fields,
    } = event;
    if !(1..=64).contains(&code.len())
      || !code
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte == b'_' || byte.is_ascii_digit())
      || message.len() > 256
    {
      return Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        "invalid event template",
      ));
    }
    let mut line = format!(
      "{{\"timestamp\":{},\"level\":{},\"core\":\"eos\",\"source\":{},\"event_code\":{},\"message\":{},\"run_id\":{},\"correlation_id\":{},\"node_path\":{},\"fields\":{}}}",
      json!(utc_timestamp()), json!(level), json!(self.source.as_str()), json!(code),
      json!(message), json!(self.run_id), json!(correlation_id), json!(node_path), fields
    ).into_bytes();
    line.push(b'\n');
    if line.len() > MAX_LINE {
      return Err(io::Error::new(
        io::ErrorKind::InvalidData,
        "event line exceeds bound",
      ));
    }
    match writer.write(&line) {
      Ok(written) if written == line.len() => Ok(()),
      Ok(_) => Err(io::Error::new(
        io::ErrorKind::WriteZero,
        "partial event write",
      )),
      Err(error) => Err(error),
    }
  }
}

trait OptionTranspose<T> {
  fn transpose_option(self) -> Option<Option<T>>;
}
impl<T> OptionTranspose<T> for Option<Option<T>> {
  fn transpose_option(self) -> Option<Option<T>> {
    match self {
      Some(Some(value)) => Some(Some(value)),
      Some(None) => None,
      None => Some(None),
    }
  }
}

fn utc_timestamp() -> String {
  let now = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap_or_default();
  let seconds = now.as_secs() as libc::time_t;
  let mut tm = unsafe { std::mem::zeroed::<libc::tm>() };
  unsafe {
    libc::gmtime_r(&seconds, &mut tm);
  }
  format!(
    "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
    tm.tm_year + 1900,
    tm.tm_mon + 1,
    tm.tm_mday,
    tm.tm_hour,
    tm.tm_min,
    tm.tm_sec,
    now.subsec_millis()
  )
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::time::ManualClock;

  #[test]
  fn listener_loss_is_a_fixed_eos_error() {
    let sink = EventSink::new(
      Source::Eos,
      "run_7".into(),
      Vec::new(),
      ManualClock::new(1, None),
    )
    .unwrap();
    sink.emit(Event::ListenerLost, None).unwrap();
    let output = sink.inner.lock().unwrap().writer.clone();
    let event: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(event["level"], "error");
    assert_eq!(event["event_code"], "eos_listener_lost");
    assert_eq!(event["message"], "EOS gRPC listener exited unexpectedly");
    assert_eq!(
      event["fields"],
      json!({"state":"failed","reason_code":"listener_lost"})
    );
  }

  #[test]
  fn ids_digests_and_saved_graph_pointer_are_bounded() {
    assert!(valid_run_id("run_7"));
    assert!(valid_run_id(&local_run_id()));
    assert!(!valid_run_id("bad/path"));
    assert!(!valid_run_id(&"x".repeat(65)));
    assert!(valid_digest(&"a".repeat(64)));
    assert!(!valid_digest(&"A".repeat(64)));
    assert_eq!(
      graph_pointer("isos/eos_process/mujoco/sim").as_deref(),
      Some("/cores/isos/layers/eos_process/groups/mujoco/nodes/sim")
    );
    assert!(graph_pointer("isos/eos_process/mujoco/invalid~id").is_none());
  }

  #[test]
  fn skill_lifecycle_and_stop_failure_use_safe_jsonl_fields() {
    let sink = EventSink::new(
      Source::Eos,
      "run_7".into(),
      Vec::new(),
      ManualClock::new(1, None),
    )
    .unwrap();
    sink
      .emit_correlated(
        Event::SkillExecution {
          execution_id: "ex-123",
          package_id: "island/fixture-skills",
          package_version: "0.1.0+Build.1",
          stage: "failed",
          reason_code: "worker.outcome_unknown",
          worker_reaped: Some(true),
          worker_exit_code: Some(1),
        },
        None,
        Some("skill_deadbeef"),
      )
      .unwrap();
    sink.emit(Event::SkillStopUnconfirmed, None).unwrap();
    let inner = sink.inner.lock().unwrap();
    let output = String::from_utf8(inner.writer.clone()).unwrap();
    drop(inner);
    let lines: Vec<Value> = output
      .lines()
      .map(|line| serde_json::from_str(line).unwrap())
      .collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0]["event_code"], "eos_skill_execution");
    assert_eq!(lines[0]["fields"]["package_id"], "island/fixture-skills");
    assert_eq!(lines[0]["fields"]["package_version"], "0.1.0+Build.1");
    assert_eq!(lines[0]["fields"]["reason_code"], "worker.outcome_unknown");
    assert_eq!(lines[1]["event_code"], "eos_skill_stop_unconfirmed");
    assert_eq!(lines[1]["fields"]["reason_code"], "skill_stop_unconfirmed");
    assert!(sink
      .emit(
        Event::SkillExecution {
          execution_id: "ex-123",
          package_id: "island/fixture-skills",
          package_version: "0.1.0",
          stage: "failed",
          reason_code: "guest said /etc/secret",
          worker_reaped: None,
          worker_exit_code: None,
        },
        None
      )
      .is_err());
    assert!(sink
      .emit(
        Event::SkillExecution {
          execution_id: "ex-123",
          package_id: "island/fixture-skills",
          package_version: "0.1.0+Build.1",
          stage: "failed",
          reason_code: "stage_unavailable",
          worker_reaped: None,
          worker_exit_code: None,
        },
        None
      )
      .is_err());
    assert!(sink
      .emit(
        Event::SkillExecution {
          execution_id: "ex-123",
          package_id: "island/fixture-skills",
          package_version: "0.1.0/secret",
          stage: "failed",
          reason_code: "worker.failed",
          worker_reaped: None,
          worker_exit_code: None,
        },
        None
      )
      .is_err());
    let skill_fields = lines[0]["fields"].as_object().unwrap();
    for forbidden in [
      "arguments",
      "result",
      "digest",
      "package_path",
      "grant",
      "token",
    ] {
      assert!(!skill_fields.contains_key(forbidden));
    }
  }

  #[test]
  fn failed_stderr_write_and_invalid_correlation_never_emit_a_partial_event() {
    struct BrokenWriter;
    impl Write for BrokenWriter {
      fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed"))
      }
      fn flush(&mut self) -> io::Result<()> {
        Ok(())
      }
    }
    let sink = EventSink::new(
      Source::Eos,
      "run_7".into(),
      BrokenWriter,
      ManualClock::new(1, None),
    )
    .unwrap();
    assert_eq!(
      sink
        .emit(Event::Ready { node_count: 1 }, None)
        .unwrap_err()
        .kind(),
      io::ErrorKind::BrokenPipe
    );
    assert_eq!(
      sink
        .emit_correlated(Event::Stopped, None, Some("bad/path"))
        .unwrap_err()
        .kind(),
      io::ErrorKind::InvalidInput
    );
  }

  #[test]
  fn partial_writer_disables_stream_before_another_record_can_append() {
    #[derive(Default)]
    struct PartialWriter(Vec<u8>);
    impl Write for PartialWriter {
      fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.push(bytes[0]);
        Ok(1)
      }
      fn flush(&mut self) -> io::Result<()> {
        Ok(())
      }
    }
    let sink = EventSink::new(
      Source::Eos,
      "run_7".into(),
      PartialWriter::default(),
      ManualClock::new(1, None),
    )
    .unwrap();
    assert_eq!(
      sink.emit(Event::Starting, None).unwrap_err().kind(),
      io::ErrorKind::WriteZero
    );
    assert_eq!(
      sink.emit(Event::Stopped, None).unwrap_err().kind(),
      io::ErrorKind::BrokenPipe
    );
    assert_eq!(sink.inner.lock().unwrap().writer.0.len(), 1);
  }
}
