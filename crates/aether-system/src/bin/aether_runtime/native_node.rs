//! Linux owner child for one verified native node instance.
use std::{
  collections::{BTreeMap, VecDeque},
  error::Error,
  ffi::{c_void, CString},
  fs,
  io::{Read, Write},
  mem,
  os::fd::AsRawFd,
  os::unix::{
    fs::PermissionsExt,
    net::{UnixListener, UnixStream},
  },
  path::{Path, PathBuf},
  ptr,
  time::{Duration, Instant},
};
use serde_json::{json, Value};
use base64::Engine;
use aether_hal::{DataTypeDescriptor, NodeTypeRegistry};

#[repr(C)]
struct Host {
  abi_version: u32,
  struct_size: u32,
  context: *mut c_void,
  emit: extern "C" fn(*mut c_void, *const u8, usize) -> i32,
  log: extern "C" fn(*mut c_void, u32, *const u8, usize),
}

fn wait_for_request_or_capture(
  listener: &UnixListener,
  until_capture: Option<Duration>,
) -> std::io::Result<()> {
  let timeout_ms = until_capture
    .map(|duration| {
      duration
        .as_millis()
        .saturating_add(u128::from(duration.subsec_nanos() % 1_000_000 != 0))
    })
    .unwrap_or(1_000)
    .min(i32::MAX as u128) as i32;
  let mut socket = libc::pollfd {
    fd: listener.as_raw_fd(),
    events: libc::POLLIN,
    revents: 0,
  };
  let result = unsafe { libc::poll(&mut socket, 1, timeout_ms) };
  if result < 0 {
    let error = std::io::Error::last_os_error();
    if error.kind() != std::io::ErrorKind::Interrupted {
      return Err(error);
    }
  }
  Ok(())
}
#[repr(C)]
struct HostV2 {
  abi_version: u32,
  struct_size: u32,
  context: *mut c_void,
  emit: extern "C" fn(*mut c_void, *const u8, usize) -> i32,
  log: extern "C" fn(*mut c_void, u32, *const u8, usize),
  stop_proof: extern "C" fn(*mut c_void, *const u8, usize) -> i32,
}

struct GraphEdge {
  source: String,
  output: String,
  target: String,
  input: String,
}
struct GraphNode {
  path: String,
  node: NativeNode,
}
struct NativeGraph {
  nodes: Vec<GraphNode>,
  indices: BTreeMap<String, usize>,
  edges: Vec<GraphEdge>,
  output_types: BTreeMap<(String, String), String>,
  registry: NodeTypeRegistry,
  latest: BTreeMap<(String, String), (u64, Value)>,
  sequence: u64,
  sources: Vec<SourceClock>,
  capture_failed: bool,
}
struct SourceClock {
  node: String,
  interval: Duration,
  next: Instant,
}
const MAX_GRAPH_DELIVERY_BYTES: usize = 24 * 1024 * 1024;
const MAX_VISION_SUBSCRIBERS: usize = 4;
const MAX_VISION_JPEG_BYTES: usize = 20 * 1024 * 1024;

struct VisionSubscriber {
  stream: UnixStream,
  source: String,
  sequence: u64,
  pending: Vec<u8>,
  written: usize,
  pending_since: Instant,
}

fn vision_packet(sample: &Value, run_id: &str, revision: &str, source: &str) -> Option<Vec<u8>> {
  let frame = &sample["output"]["payload"];
  if frame["encoding"] != "jpeg" {
    return None;
  }
  let encoded = frame["data"].as_str()?;
  let jpeg = base64::engine::general_purpose::STANDARD
    .decode(encoded)
    .ok()?;
  if jpeg.len() > MAX_VISION_JPEG_BYTES
    || jpeg.len() < 4
    || !jpeg.starts_with(&[0xff, 0xd8])
    || !jpeg.ends_with(&[0xff, 0xd9])
  {
    return None;
  }
  let header = serde_json::to_vec(&json!({"run_id":run_id,"revision":revision,
    "source_path":source,"output_sequence":sample["sequence"],"sample":frame["sample"],
    "width_px":frame["width_px"],"height_px":frame["height_px"],"encoding":"jpeg"}))
  .ok()?;
  if header.len() > 4096 {
    return None;
  }
  let mut packet = Vec::with_capacity(8 + header.len() + jpeg.len());
  packet.extend_from_slice(&(header.len() as u32).to_be_bytes());
  packet.extend_from_slice(&header);
  packet.extend_from_slice(&(jpeg.len() as u32).to_be_bytes());
  packet.extend_from_slice(&jpeg);
  Some(packet)
}

fn publish_vision(
  subscribers: &mut Vec<VisionSubscriber>,
  graph: &NativeGraph,
  run_id: &str,
  revision: &str,
) {
  subscribers.retain_mut(|subscriber| {
    if graph.capture_failed {
      return false;
    }
    if subscriber.pending.is_empty() || subscriber.written == 0 {
      if let Some(sample) = graph.latest_output(&subscriber.source, "frame") {
        if let Some(sequence) = sample["sequence"].as_u64() {
          if sequence > subscriber.sequence {
            let Some(packet) = vision_packet(&sample, run_id, revision, &subscriber.source) else {
              return false;
            };
            subscriber.sequence = sequence;
            subscriber.pending = packet;
            subscriber.written = 0;
            subscriber.pending_since = Instant::now();
          }
        }
      }
    }
    if !subscriber.pending.is_empty() {
      match subscriber
        .stream
        .write(&subscriber.pending[subscriber.written..])
      {
        Ok(0) => return false,
        Ok(count) => subscriber.written += count,
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
        Err(_) => return false,
      }
      if subscriber.written == subscriber.pending.len() {
        subscriber.pending.clear();
        subscriber.written = 0;
      } else if subscriber.pending_since.elapsed() > Duration::from_millis(150) {
        return false;
      }
    }
    true
  });
}

fn account_delivery_bytes(total: &mut usize, bytes: usize) -> Result<(), &'static str> {
  *total = total
    .checked_add(bytes)
    .filter(|sum| *sum <= MAX_GRAPH_DELIVERY_BYTES)
    .ok_or("native graph delivery bytes exceeded")?;
  Ok(())
}
impl NativeGraph {
  fn open(config: &Value) -> Result<Self, Box<dyn Error>> {
    let definitions = config["nodes"]
      .as_array()
      .ok_or("native graph requires nodes")?;
    if definitions.is_empty() || definitions.len() > 64 {
      return Err("native graph node count out of bounds".into());
    }
    let mut nodes: Vec<GraphNode> = Vec::new();
    let mut indices = BTreeMap::new();
    for (index, definition) in definitions.iter().enumerate() {
      let path = definition["path"]
        .as_str()
        .ok_or("native node path missing")?;
      let library = definition["library"]
        .as_str()
        .ok_or("native node library missing")?;
      if library.is_empty() || indices.insert(path.to_owned(), index).is_some() {
        return Err("duplicate path or missing library".into());
      }
    }
    let mut registry = NodeTypeRegistry::builtin();
    for descriptor in config["data_types"]
      .as_array()
      .ok_or("native graph requires data types")?
    {
      let descriptor: DataTypeDescriptor = serde_json::from_value(descriptor.clone())?;
      registry.register_data_type(descriptor)?;
    }
    let mut output_types = BTreeMap::new();
    for output in config["output_types"]
      .as_array()
      .ok_or("native graph requires output types")?
    {
      let node = output["node"].as_str().ok_or("output node missing")?;
      let port = output["port"].as_str().ok_or("output port missing")?;
      let data_type = output["data_type"]
        .as_str()
        .ok_or("output data type missing")?;
      if !indices.contains_key(node)
        || port.is_empty()
        || port == "command_result"
        || registry.data_type(data_type).is_none()
        || output_types
          .insert((node.to_owned(), port.to_owned()), data_type.to_owned())
          .is_some()
      {
        return Err("invalid native graph output declaration".into());
      }
    }
    for definition in definitions {
      let path = definition["path"]
        .as_str()
        .ok_or("native node path missing")?;
      let library = definition["library"]
        .as_str()
        .ok_or("native node library missing")?;
      let node_config =
        json!({"run_id":config["run_id"],"revision":config["revision"],"instance":definition});
      let node = match NativeNode::open(Path::new(library), &serde_json::to_vec(&node_config)?) {
        Ok(node) => node,
        Err(error) => {
          for previous in nodes.iter_mut().rev() {
            let _ = previous.node.stop();
          }
          return Err(error);
        }
      };
      nodes.push(GraphNode {
        path: path.into(),
        node,
      });
    }
    let parsed_edges = (|| -> Result<Vec<GraphEdge>, Box<dyn Error>> {
      let mut edges = Vec::new();
      for edge in config["edges"]
        .as_array()
        .ok_or("native graph requires edges")?
      {
        let source = edge["source"].as_str().ok_or("edge source missing")?;
        let output = edge["output"].as_str().ok_or("edge output missing")?;
        let target = edge["target"].as_str().ok_or("edge target missing")?;
        let input = edge["input"].as_str().ok_or("edge input missing")?;
        if !indices.contains_key(source)
          || !indices.contains_key(target)
          || output.is_empty()
          || output == "command_result"
          || input.is_empty()
        {
          return Err("invalid native graph edge".into());
        }
        edges.push(GraphEdge {
          source: source.into(),
          output: output.into(),
          target: target.into(),
          input: input.into(),
        });
      }
      Ok(edges)
    })();
    let edges = match parsed_edges {
      Ok(edges) => edges,
      Err(error) => {
        for previous in nodes.iter_mut().rev() {
          let _ = previous.node.stop();
        }
        return Err(error);
      }
    };
    let now = Instant::now();
    let mut sources = Vec::new();
    for definition in definitions {
      if let Some(frequency) = definition.pointer("/node/adapter/settings/frequency_hz") {
        let hz = frequency
          .as_f64()
          .ok_or("source frequency_hz must be numeric")?;
        if !hz.is_finite() || hz <= 0.0 || hz > 1000.0 {
          return Err("source frequency_hz out of bounds".into());
        }
        let seconds = 1.0 / hz;
        if !seconds.is_finite() || seconds >= u64::MAX as f64 {
          return Err("source frequency_hz out of bounds".into());
        }
        let node = definition["path"]
          .as_str()
          .ok_or("native node path missing")?;
        if edges.iter().any(|edge| edge.target == node) {
          return Err("frequency_hz is only valid on a source node".into());
        }
        sources.push(SourceClock {
          node: node.into(),
          interval: Duration::from_secs_f64(seconds),
          next: now,
        });
      }
    }
    Ok(Self {
      nodes,
      indices,
      edges,
      output_types,
      registry,
      latest: BTreeMap::new(),
      sequence: 0,
      sources,
      capture_failed: false,
    })
  }
  fn capture_due(&mut self, run_id: &str, revision: &str) {
    let now = Instant::now();
    let due: Vec<String> = self
      .sources
      .iter_mut()
      .filter_map(|source| {
        if now < source.next {
          return None;
        }
        source.next = now + source.interval;
        Some(source.node.clone())
      })
      .collect();
    for node in due {
      if self
        .input(&json!({"op":"observe","node":node}), run_id, revision)
        .is_err()
      {
        self.capture_failed = true;
        self.latest.clear();
      }
    }
  }
  fn latest_output(&self, node: &str, port: &str) -> Option<Value> {
    self
      .latest
      .get(&(node.into(), port.into()))
      .map(|(sequence, output)| json!({"node":node,"output":output,"sequence":sequence}))
  }
  fn health(&self) -> i32 {
    if !self.capture_failed && self.nodes.iter().all(|node| node.node.health() == 1) {
      1
    } else {
      0
    }
  }
  fn stop(&mut self) -> bool {
    let mut proven = true;
    for node in self.nodes.iter_mut().rev() {
      proven = node.node.stop() && proven;
    }
    proven
  }
  fn input(
    &mut self,
    request: &Value,
    run_id: &str,
    revision: &str,
  ) -> Result<Vec<Value>, Box<dyn Error>> {
    if request["op"] == "command" {
      let target = request["node"].as_str().ok_or("command target required")?;
      let index = *self.indices.get(target).ok_or("unknown command target")?;
      let node = &mut self.nodes[index].node;
      if node.api_v2.is_null() || node.actuator.is_none() {
        return Err("native command target is not a v2 actuator".into());
      }
      let outputs = node.input(request)?;
      if outputs.len() != 1
        || outputs[0]["port"] != "command_result"
        || outputs[0]["payload"]["accepted"].as_bool().is_none()
      {
        return Err("invalid native command result".into());
      }
      // This private Dispatcher result is never a graph output or routed edge.
      return Ok(vec![json!({"node":target,"output":outputs[0]})]);
    }
    let target = request["node"]
      .as_str()
      .or_else(|| (self.nodes.len() == 1).then_some(self.nodes[0].path.as_str()))
      .ok_or("input target required")?;
    let index = *self.indices.get(target).ok_or("unknown input target")?;
    let mut pending = VecDeque::from([(index, request.clone())]);
    let mut result = Vec::new();
    let mut deliveries = 0;
    let mut queued_bytes = 0;
    let mut output_bytes = 0;
    while let Some((index, envelope)) = pending.pop_front() {
      deliveries += 1;
      if deliveries > 256 {
        return Err("native graph delivery limit exceeded".into());
      }
      let source = self.nodes[index].path.clone();
      let delivered = if envelope["op"] == "observe" {
        json!({"op":"observe"})
      } else {
        envelope.clone()
      };
      for output in self.nodes[index].node.input(&delivered)? {
        let port = output["port"]
          .as_str()
          .ok_or("native output port invalid")?;
        if port == "command_result" {
          return Err("native command result requires Dispatcher admission".into());
        }
        let data_type = self
          .output_types
          .get(&(source.clone(), port.to_owned()))
          .ok_or("native output port undeclared")?;
        let payload = output
          .get("payload")
          .ok_or("native output payload missing")?;
        self.registry.validate_data_payload(data_type, payload)?;
        self.sequence = self.sequence.wrapping_add(1);
        self.latest.insert(
          (source.clone(), port.into()),
          (self.sequence, output.clone()),
        );
        while self
          .latest
          .values()
          .map(|(_, sample)| serde_json::to_vec(sample).map_or(0, |bytes| bytes.len()))
          .sum::<usize>()
          > MAX_GRAPH_DELIVERY_BYTES
        {
          let oldest = self
            .latest
            .iter()
            .min_by_key(|(_, (sequence, _))| *sequence)
            .map(|(key, _)| key.clone());
          if let Some(oldest) = oldest {
            self.latest.remove(&oldest);
          } else {
            break;
          }
        }
        for edge in self
          .edges
          .iter()
          .filter(|edge| edge.source == source && edge.output == port)
        {
          let next = *self
            .indices
            .get(&edge.target)
            .ok_or("native target disappeared")?;
          let envelope = json!({"run_id":run_id,"revision":revision,"source_node":source,"source_port":port,"target_node":edge.target,"target_port":edge.input,"payload":payload});
          account_delivery_bytes(&mut queued_bytes, serde_json::to_vec(&envelope)?.len())?;
          pending.push_back((next, envelope));
        }
        let emitted = json!({"node":source,"output":output});
        account_delivery_bytes(&mut output_bytes, serde_json::to_vec(&emitted)?.len())?;
        result.push(emitted);
      }
    }
    Ok(result)
  }
}

pub fn serve_graph(
  socket: &Path,
  run_id: &str,
  revision: &str,
  config: &Value,
) -> Result<(), Box<dyn Error>> {
  let mut graph = NativeGraph::open(config)?;
  let admission_token = config["admission_token"].as_str().unwrap_or("");
  let vision_token = config["vision_token"].as_str().unwrap_or("");
  let vision_sources = config["vision_sources"]
    .as_array()
    .cloned()
    .unwrap_or_default();
  let listener = UnixListener::bind(socket)?;
  fs::set_permissions(socket, fs::Permissions::from_mode(0o600))?;
  struct Guard(PathBuf);
  impl Drop for Guard {
    fn drop(&mut self) {
      let _ = fs::remove_file(&self.0);
    }
  }
  let _guard = Guard(socket.to_owned());
  listener.set_nonblocking(true)?;
  let mut stop = false;
  let mut vision_subscribers = Vec::new();
  while !stop {
    graph.capture_due(run_id, revision);
    publish_vision(&mut vision_subscribers, &graph, run_id, revision);
    match listener.accept() {
      Ok((mut stream, _)) => {
        stream.set_read_timeout(Some(Duration::from_secs(2)))?;
        let mut bytes = Vec::new();
        let mut chunk = [0u8; 1024];
        loop {
          let count = stream.read(&mut chunk)?;
          if count == 0 {
            break;
          }
          bytes.extend_from_slice(&chunk[..count]);
          if bytes.len() > 64 * 1024 || bytes == b"status\n" || bytes == b"stop\n" {
            break;
          }
        }
        if let Ok(value) = serde_json::from_slice::<Value>(&bytes) {
          if value["op"] == "vision_subscribe" {
            let source = value["node"].as_str().unwrap_or("");
            let valid = vision_token.len() == 64
              && value["vision_token"] == vision_token
              && value["run_id"] == run_id
              && value["revision"] == revision
              && vision_sources.iter().any(|allowed| allowed == source)
              && graph
                .output_types
                .get(&(source.to_owned(), "frame".to_owned()))
                .map(String::as_str)
                == Some("image_frame@v0.2")
              && !graph.capture_failed
              && vision_subscribers.len() < MAX_VISION_SUBSCRIBERS;
            if valid {
              stream.set_nonblocking(true)?;
              vision_subscribers.push(VisionSubscriber {
                stream,
                source: source.to_owned(),
                sequence: 0,
                pending: Vec::new(),
                written: 0,
                pending_since: Instant::now(),
              });
            } else {
              let _ =
                stream.write_all(b"{\"ok\":false,\"error\":\"vision_subscription_rejected\"}");
            }
            continue;
          }
        }
        let reply = if bytes == b"status\n" {
          json!({"ok":true,"core":"eos","run_id":run_id,"state":if graph.health()==1 {"ready"} else {"degraded"}})
        } else if bytes == b"stop\n" {
          let proof = graph.stop();
          stop = proof;
          json!({"ok":proof,"core":"eos","run_id":run_id,"state":if proof {"stopping"} else {"stop_failed"}})
        } else if let Ok(value) = serde_json::from_slice::<Value>(&bytes) {
          if value["run_id"] != run_id || value["revision"] != revision {
            json!({"ok":false,"error":"stale_run"})
          } else if value["op"] != "input"
            && value["op"] != "observe"
            && value["op"] != "command"
            && value["op"] != "latest_output"
          {
            json!({"ok":false,"error":"invalid_operation"})
          } else if value["op"] == "latest_output" {
            match (value["node"].as_str(), value["port"].as_str()) {
              (Some(node), Some(port))
                if graph.output_types.contains_key(&(node.into(), port.into())) =>
              {
                let after = value["after_sequence"].as_u64().unwrap_or(0);
                if graph.capture_failed {
                  json!({"ok":false,"error":"source_failed"})
                } else {
                  let sample = graph
                    .latest_output(node, port)
                    .filter(|sample| sample["sequence"].as_u64().is_some_and(|seq| seq > after));
                  json!({"ok":true,"run_id":run_id,"revision":revision,"sample":sample})
                }
              }
              _ => json!({"ok":false,"error":"unknown_output"}),
            }
          } else if value["op"] == "command"
            && (admission_token.len() != 64 || value["admission_token"] != admission_token)
          {
            json!({"ok":false,"error":"dispatcher_admission_required"})
          } else {
            match graph.input(&value, run_id, revision) {
              Ok(outputs) => json!({"ok":true,"outputs":outputs}),
              Err(_) => json!({"ok":false,"error":"native_graph_input_failed"}),
            }
          }
        } else {
          json!({"ok":false,"error":"invalid_request"})
        };
        let _ = stream.write_all(serde_json::to_string(&reply)?.as_bytes());
      }
      Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
        let now = Instant::now();
        let until_capture = graph
          .sources
          .iter()
          .map(|source| source.next.saturating_duration_since(now))
          .min();
        wait_for_request_or_capture(
          &listener,
          if vision_subscribers.is_empty() {
            until_capture
          } else {
            Some(
              until_capture
                .unwrap_or(Duration::from_millis(2))
                .min(Duration::from_millis(2)),
            )
          },
        )?;
      }
      Err(error) => return Err(error.into()),
    }
  }
  Ok(())
}
#[repr(C)]
struct Api {
  abi_version: u32,
  struct_size: u32,
  create: Option<unsafe extern "C" fn(*const u8, usize, *const Host, *mut *mut c_void) -> i32>,
  start: Option<unsafe extern "C" fn(*mut c_void) -> i32>,
  input: Option<unsafe extern "C" fn(*mut c_void, *const u8, usize) -> i32>,
  health: Option<unsafe extern "C" fn(*mut c_void) -> i32>,
  stop: Option<unsafe extern "C" fn(*mut c_void) -> i32>,
  destroy: Option<unsafe extern "C" fn(*mut c_void)>,
}
#[repr(C)]
struct ApiV2 {
  abi_version: u32,
  struct_size: u32,
  create: Option<unsafe extern "C" fn(*const u8, usize, *const HostV2, *mut *mut c_void) -> i32>,
  start: Option<unsafe extern "C" fn(*mut c_void) -> i32>,
  observe: Option<unsafe extern "C" fn(*mut c_void, *const u8, usize) -> i32>,
  command: Option<unsafe extern "C" fn(*mut c_void, *const u8, usize) -> i32>,
  health: Option<unsafe extern "C" fn(*mut c_void) -> i32>,
  stop: Option<unsafe extern "C" fn(*mut c_void, *const u8, usize) -> i32>,
  destroy: Option<unsafe extern "C" fn(*mut c_void)>,
}
/// Inspect the exported ABI in a short-lived process before admitting a release.
pub(super) fn probe(library: &Path) -> Result<&'static str, Box<dyn Error>> {
  if !library.is_absolute() || library.extension().is_none_or(|ext| ext != "so") {
    return Err("native adapter requires an absolute .so".into());
  }
  let name = CString::new(library.as_os_str().as_encoded_bytes())?;
  let module = unsafe { libc::dlopen(name.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL) };
  if module.is_null() {
    return Err("native adapter library load failed".into());
  }
  let result = (|| {
    let v2_symbol = unsafe { libc::dlsym(module, c"isos_node_v2".as_ptr()) };
    if !v2_symbol.is_null() {
      let entry: unsafe extern "C" fn() -> *const ApiV2 = unsafe { mem::transmute(v2_symbol) };
      let api = unsafe { entry() };
      if api.is_null()
        || unsafe { (*api).abi_version != 2 || (*api).struct_size < mem::size_of::<ApiV2>() as u32 }
      {
        return Err("native v2 ABI version or table size mismatch");
      }
      if unsafe {
        (*api).create.is_none()
          || (*api).start.is_none()
          || (*api).observe.is_none()
          || (*api).command.is_none()
          || (*api).health.is_none()
          || (*api).stop.is_none()
          || (*api).destroy.is_none()
      } {
        return Err("native v2 ABI callback missing");
      }
      return Ok("isos_node_v2");
    }
    let symbol = unsafe { libc::dlsym(module, c"isos_node_v1".as_ptr()) };
    if symbol.is_null() {
      return Err("native ABI symbol missing");
    }
    let entry: unsafe extern "C" fn() -> *const Api = unsafe { mem::transmute(symbol) };
    let api = unsafe { entry() };
    if api.is_null()
      || unsafe { (*api).abi_version != 1 || (*api).struct_size < mem::size_of::<Api>() as u32 }
    {
      return Err("native ABI version or table size mismatch");
    }
    if unsafe {
      (*api).create.is_none()
        || (*api).start.is_none()
        || (*api).input.is_none()
        || (*api).health.is_none()
        || (*api).stop.is_none()
        || (*api).destroy.is_none()
    } {
      return Err("native ABI callback missing");
    }
    Ok("isos_node_v1")
  })();
  unsafe { libc::dlclose(module) };
  result.map_err(Into::into)
}
extern "C" fn emit(context: *mut c_void, bytes: *const u8, len: usize) -> i32 {
  if context.is_null() || bytes.is_null() || len > 24 * 1024 * 1024 {
    return -1;
  }
  let payload = unsafe { std::slice::from_raw_parts(bytes, len) };
  let Ok(value) = serde_json::from_slice::<Value>(payload) else {
    return -1;
  };
  let outputs = unsafe { &mut *(context as *mut OutputState) };
  if outputs.values.len() >= 256 || outputs.bytes.saturating_add(len) > 24 * 1024 * 1024 {
    return -1;
  }
  outputs.bytes += len;
  outputs.values.push(value);
  0
}
extern "C" fn log(_: *mut c_void, _: u32, _: *const u8, _: usize) {}

struct OutputState {
  values: Vec<Value>,
  bytes: usize,
  stop_proof: Option<Value>,
}

extern "C" fn stop_proof(context: *mut c_void, bytes: *const u8, len: usize) -> i32 {
  if context.is_null() || bytes.is_null() || len > 16 * 1024 {
    return -1;
  }
  let Some(proof) =
    serde_json::from_slice::<Value>(unsafe { std::slice::from_raw_parts(bytes, len) }).ok()
  else {
    return -1;
  };
  let state = unsafe { &mut *(context as *mut OutputState) };
  if state.stop_proof.is_some() {
    return -1;
  }
  state.stop_proof = Some(proof);
  0
}

pub(super) struct NativeNode {
  library: *mut c_void,
  api: *const Api,
  api_v2: *const ApiV2,
  handle: *mut c_void,
  outputs: Box<OutputState>,
  _host: Box<Host>,
  _host_v2: Option<Box<HostV2>>,
  run_id: String,
  revision: String,
  actuator: Option<String>,
  stopped: bool,
}
impl NativeNode {
  pub(super) fn open(library: &Path, config: &[u8]) -> Result<Self, Box<dyn Error>> {
    if !library.is_absolute()
      || library.extension().is_none_or(|ext| ext != "so")
      || config.len() > 64 * 1024
    {
      return Err("invalid native node input".into());
    }
    let config_value: Value = serde_json::from_slice(config)?;
    let name = CString::new(library.as_os_str().as_encoded_bytes())?;
    let module = unsafe { libc::dlopen(name.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL) };
    if module.is_null() {
      return Err("native library load failed".into());
    }
    let v2 =
      config_value["instance"]["node"]["adapter"]["settings"]["service_instance"].is_string();
    let symbol = unsafe {
      libc::dlsym(
        module,
        if v2 {
          c"isos_node_v2".as_ptr()
        } else {
          c"isos_node_v1".as_ptr()
        },
      )
    };
    if symbol.is_null() {
      unsafe {
        libc::dlclose(module);
      }
      return Err("native ABI symbol missing".into());
    }
    let entry: unsafe extern "C" fn() -> *const c_void = unsafe { mem::transmute(symbol) };
    let table = unsafe { entry() };
    let api = if v2 { ptr::null() } else { table.cast::<Api>() };
    let api_v2 = if v2 {
      table.cast::<ApiV2>()
    } else {
      ptr::null()
    };
    if table.is_null()
      || (!v2
        && unsafe { (*api).abi_version != 1 || (*api).struct_size < mem::size_of::<Api>() as u32 })
      || (v2
        && unsafe {
          (*api_v2).abi_version != 2 || (*api_v2).struct_size < mem::size_of::<ApiV2>() as u32
        })
    {
      unsafe {
        libc::dlclose(module);
      }
      return Err("native ABI version or table size mismatch".into());
    }
    if !v2
      && unsafe {
        (*api).create.is_none()
          || (*api).start.is_none()
          || (*api).input.is_none()
          || (*api).health.is_none()
          || (*api).stop.is_none()
          || (*api).destroy.is_none()
      }
      || v2
        && unsafe {
          (*api_v2).create.is_none()
            || (*api_v2).start.is_none()
            || (*api_v2).observe.is_none()
            || (*api_v2).command.is_none()
            || (*api_v2).health.is_none()
            || (*api_v2).stop.is_none()
            || (*api_v2).destroy.is_none()
        }
    {
      unsafe {
        libc::dlclose(module);
      }
      return Err("native ABI callback missing".into());
    }
    let mut outputs = Box::new(OutputState {
      values: Vec::new(),
      bytes: 0,
      stop_proof: None,
    });
    let host = Box::new(Host {
      abi_version: 1,
      struct_size: mem::size_of::<Host>() as u32,
      context: (&mut *outputs as *mut OutputState).cast(),
      emit,
      log,
    });
    let host_v2 = v2.then(|| {
      Box::new(HostV2 {
        abi_version: 2,
        struct_size: mem::size_of::<HostV2>() as u32,
        context: (&mut *outputs as *mut OutputState).cast(),
        emit,
        log,
        stop_proof,
      })
    });
    let mut handle = ptr::null_mut();
    let created = if v2 {
      unsafe {
        ((*api_v2).create.unwrap())(
          config.as_ptr(),
          config.len(),
          &**host_v2.as_ref().unwrap(),
          &mut handle,
        )
      }
    } else {
      unsafe { ((*api).create.unwrap())(config.as_ptr(), config.len(), &*host, &mut handle) }
    };
    if created != 0 || handle.is_null() {
      unsafe {
        libc::dlclose(module);
      }
      return Err("native node create failed".into());
    }
    let started = if v2 {
      unsafe { ((*api_v2).start.unwrap())(handle) }
    } else {
      unsafe { ((*api).start.unwrap())(handle) }
    };
    if started != 0 {
      unsafe {
        if v2 {
          ((*api_v2).destroy.unwrap())(handle);
        } else {
          ((*api).destroy.unwrap())(handle);
        }
        libc::dlclose(module);
      }
      return Err("native node start failed".into());
    }
    Ok(Self {
      library: module,
      api,
      api_v2,
      handle,
      outputs,
      _host: host,
      _host_v2: host_v2,
      run_id: config_value["run_id"].as_str().unwrap_or("").into(),
      revision: config_value["revision"].as_str().unwrap_or("").into(),
      actuator: match config_value["instance"]["node"]["node_type"]
        .as_str()
        .unwrap_or("")
      {
        id if id.starts_with("eos.hardware.base.") => Some("base".into()),
        id if id.starts_with("eos.hardware.manipulator.") => Some("manipulator".into()),
        id if id.starts_with("eos.hardware.gripper.") => Some("gripper".into()),
        id if id.starts_with("eos.hardware.torso.") => Some("torso".into()),
        _ => None,
      },
      stopped: false,
    })
  }
  pub(super) fn health(&self) -> i32 {
    if self.api_v2.is_null() {
      unsafe { ((*self.api).health.unwrap())(self.handle) }
    } else {
      unsafe { ((*self.api_v2).health.unwrap())(self.handle) }
    }
  }
  pub(super) fn input(&mut self, value: &Value) -> Result<Vec<Value>, Box<dyn Error>> {
    let bytes = serde_json::to_vec(value)?;
    if bytes.len() > 24 * 1024 * 1024 {
      return Err("native input too large".into());
    }
    self.outputs.values.clear();
    self.outputs.bytes = 0;
    let status = if self.api_v2.is_null() {
      unsafe { ((*self.api).input.unwrap())(self.handle, bytes.as_ptr(), bytes.len()) }
    } else if value["op"] == "observe" {
      unsafe { ((*self.api_v2).observe.unwrap())(self.handle, bytes.as_ptr(), bytes.len()) }
    } else if value["op"] == "command" && self.actuator.is_some() {
      unsafe { ((*self.api_v2).command.unwrap())(self.handle, bytes.as_ptr(), bytes.len()) }
    } else {
      return Err("v2 command requires an EOS Dispatcher reservation".into());
    };
    if status != 0 {
      return Err("native input failed".into());
    }
    Ok(mem::take(&mut self.outputs.values))
  }
  pub(super) fn stop(&mut self) -> bool {
    if self.stopped {
      return true;
    }
    self.stopped = if self.api_v2.is_null() {
      (unsafe { ((*self.api).stop.unwrap())(self.handle) }) == 0
    } else {
      self.outputs.stop_proof = None;
      let request = json!({"run_id":self.run_id,"revision":self.revision,"stop_epoch":1});
      let Ok(bytes) = serde_json::to_vec(&request) else {
        return false;
      };
      let status =
        unsafe { ((*self.api_v2).stop.unwrap())(self.handle, bytes.as_ptr(), bytes.len()) };
      status == 0
        && self.outputs.stop_proof.as_ref().is_some_and(|proof| {
          proof["run_id"] == self.run_id
            && proof["revision"] == self.revision
            && proof["outcome"] == "stopped"
            && self
              .actuator
              .as_ref()
              .is_none_or(|actuator| proof["actuators"][actuator] == "stopped")
        })
    };
    self.stopped
  }
}
impl Drop for NativeNode {
  fn drop(&mut self) {
    if self.stopped {
      unsafe {
        if self.api_v2.is_null() {
          ((*self.api).destroy.unwrap())(self.handle);
        } else {
          ((*self.api_v2).destroy.unwrap())(self.handle);
        }
        libc::dlclose(self.library);
      }
    }
  }
}
#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn pending_worker_request_wakes_before_next_capture_deadline() {
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("native.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let peer = std::thread::spawn(move || {
      std::thread::sleep(Duration::from_millis(20));
      std::os::unix::net::UnixStream::connect(socket).unwrap()
    });
    let started = Instant::now();
    wait_for_request_or_capture(&listener, Some(Duration::from_millis(500))).unwrap();
    assert!(started.elapsed() < Duration::from_millis(200));
    let _ = listener.accept().unwrap();
    peer.join().unwrap();
  }

  #[test]
  fn native_graph_delivery_budget_rejects_first_byte_over_limit() {
    let mut total = 0;
    assert!(account_delivery_bytes(&mut total, MAX_GRAPH_DELIVERY_BYTES).is_ok());
    assert_eq!(total, MAX_GRAPH_DELIVERY_BYTES);
    assert_eq!(
      account_delivery_bytes(&mut total, 1).unwrap_err(),
      "native graph delivery bytes exceeded"
    );
    assert_eq!(total, MAX_GRAPH_DELIVERY_BYTES);
  }
  #[test]
  fn v2_observes_rejects_unadmitted_commands_and_requires_stop_proof() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("node.c");
    let library = directory.path().join("node.so");
    fs::write(&source, r#"
#include "isos_node.h"
#include <stdlib.h>
struct isos_node_v2_handle { const isos_node_host_v2 *host; };
static int32_t create(const uint8_t *c,size_t l,const isos_node_host_v2 *h,isos_node_v2_handle **o){(void)c;(void)l;*o=malloc(sizeof(**o));(*o)->host=h;return 0;}
static int32_t start(isos_node_v2_handle *n){return n?0:-1;}
static int32_t observe(isos_node_v2_handle *n,const uint8_t *b,size_t l){(void)b;(void)l;const uint8_t o[]="{\"port\":\"state\",\"payload\":{\"value\":1}}";return n->host->emit(n->host->context,o,sizeof(o)-1);}
static int32_t command(isos_node_v2_handle *n,const uint8_t *b,size_t l){(void)n;(void)b;(void)l;return 0;}
static int32_t health(isos_node_v2_handle *n){return n?1:-1;}
static int32_t stop(isos_node_v2_handle *n,const uint8_t *b,size_t l){(void)b;(void)l;const uint8_t p[]="{\"run_id\":\"run_1\",\"revision\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\",\"outcome\":\"stopped\"}";return n->host->stop_proof(n->host->context,p,sizeof(p)-1);}
static void destroy(isos_node_v2_handle *n){free(n);}
static const isos_node_api_v2 api={2,sizeof(isos_node_api_v2),create,start,observe,command,health,stop,destroy};
const isos_node_api_v2 *isos_node_v2(void){return &api;}
"#).unwrap();
    let header = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contract/node");
    let output = std::process::Command::new("cc")
      .args(["-shared", "-fPIC", "-I"])
      .arg(header)
      .arg(&source)
      .arg("-o")
      .arg(&library)
      .output()
      .unwrap();
    assert!(
      output.status.success(),
      "{}",
      String::from_utf8_lossy(&output.stderr)
    );
    probe(&library).unwrap();
    let config = json!({"run_id":"run_1","revision":"a".repeat(64),
      "instance":{"node":{"adapter":{"settings":{"service_instance":"native"}}}}});
    let mut node = NativeNode::open(&library, config.to_string().as_bytes()).unwrap();
    assert_eq!(node.health(), 1);
    assert_eq!(
      node.input(&json!({"op":"observe"})).unwrap(),
      vec![json!({"port":"state","payload":{"value":1}})]
    );
    assert!(node
      .input(&json!({"op":"command","admission":{"dispatcher":true}}))
      .is_err());
    assert!(node.stop());
  }
  #[test]
  fn dispatcher_command_result_is_private_and_exactly_one() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("node.c");
    let library = directory.path().join("node.so");
    fs::write(&source, r#"
#include "isos_node.h"
#include <stdlib.h>
#include <string.h>
struct isos_node_v2_handle { const isos_node_host_v2 *host; };
static int32_t create(const uint8_t *c,size_t l,const isos_node_host_v2 *h,isos_node_v2_handle **o){(void)c;(void)l;*o=malloc(sizeof(**o));(*o)->host=h;return 0;}
static int32_t start(isos_node_v2_handle *n){return n?0:-1;}
static int32_t observe(isos_node_v2_handle *n,const uint8_t *b,size_t l){(void)b;(void)l;const char *o="{\"port\":\"command_result\",\"payload\":{\"accepted\":true}}";return n->host->emit(n->host->context,(const uint8_t *)o,strlen(o));}
static int32_t command(isos_node_v2_handle *n,const uint8_t *b,size_t l){
  char *v=malloc(l+1);memcpy(v,b,l);v[l]=0;
  const char *o=strstr(v,"bad_port") ? "{\"port\":\"state\",\"payload\":{\"accepted\":true}}" :
    strstr(v,"bad_payload") ? "{\"port\":\"command_result\",\"payload\":{\"accepted\":\"yes\"}}" :
    "{\"port\":\"command_result\",\"payload\":{\"accepted\":true}}";
  int32_t result=n->host->emit(n->host->context,(const uint8_t *)o,strlen(o));
  if(strstr(v,"duplicate")) result=n->host->emit(n->host->context,(const uint8_t *)o,strlen(o));
  free(v);return result;
}
static int32_t health(isos_node_v2_handle *n){return n?1:-1;}
static int32_t stop(isos_node_v2_handle *n,const uint8_t *b,size_t l){(void)b;(void)l;const char *p="{\"run_id\":\"run_1\",\"revision\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\",\"outcome\":\"stopped\",\"actuators\":{\"manipulator\":\"stopped\"}}";return n->host->stop_proof(n->host->context,(const uint8_t *)p,strlen(p));}
static void destroy(isos_node_v2_handle *n){free(n);}
static const isos_node_api_v2 api={2,sizeof(isos_node_api_v2),create,start,observe,command,health,stop,destroy};
const isos_node_api_v2 *isos_node_v2(void){return &api;}
"#).unwrap();
    let header = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contract/node");
    let build = std::process::Command::new("cc")
      .args(["-shared", "-fPIC", "-I"])
      .arg(header)
      .arg(&source)
      .arg("-o")
      .arg(&library)
      .output()
      .unwrap();
    assert!(
      build.status.success(),
      "{}",
      String::from_utf8_lossy(&build.stderr)
    );
    let path = "eos/hardware/manipulator/native";
    let config = json!({"run_id":"run_1","revision":"a".repeat(64),"nodes":[
      {"path":path,"library":library,"node":{"node_type":"eos.hardware.manipulator.native@v0.2","adapter":{"settings":{"service_instance":"native"}}}}
    ],"edges":[],"output_types":[],"data_types":[]});
    let mut graph = NativeGraph::open(&config).unwrap();
    let command = |payload: &str| json!({"op":"command","node":path,"payload":payload});
    let outputs = graph
      .input(&command("good"), "run_1", &"a".repeat(64))
      .unwrap();
    assert_eq!(
      outputs,
      vec![json!({"node":path,"output":{"port":"command_result","payload":{"accepted":true}}})]
    );
    for value in ["bad_port", "bad_payload", "duplicate"] {
      assert!(graph
        .input(&command(value), "run_1", &"a".repeat(64))
        .is_err());
    }
    assert!(graph
      .input(
        &json!({"op":"observe","node":path}),
        "run_1",
        &"a".repeat(64)
      )
      .is_err());
    assert!(graph
      .input(&json!({"op":"input","node":path}), "run_1", &"a".repeat(64))
      .is_err());
    assert!(graph.stop());
    let mut reserved_output = config.clone();
    reserved_output["output_types"] =
      json!([{"node":path,"port":"command_result","data_type":"camera_frame"}]);
    assert!(NativeGraph::open(&reserved_output).is_err());
    let mut reserved_edge = config.clone();
    reserved_edge["edges"] =
      json!([{"source":path,"output":"command_result","target":path,"input":"in"}]);
    assert!(NativeGraph::open(&reserved_edge).is_err());
    let socket = directory.path().join("worker.sock");
    let mut served_config = config;
    served_config["admission_token"] = json!("b".repeat(64));
    let worker_socket = socket.clone();
    let worker = std::thread::spawn(move || {
      serve_graph(&worker_socket, "run_1", &"a".repeat(64), &served_config).unwrap();
    });
    for _ in 0..100 {
      if socket.exists() {
        break;
      }
      std::thread::sleep(Duration::from_millis(10));
    }
    let exchange = |request: Value| -> Value {
      let mut stream = std::os::unix::net::UnixStream::connect(&socket).unwrap();
      stream.write_all(request.to_string().as_bytes()).unwrap();
      stream.shutdown(std::net::Shutdown::Write).unwrap();
      let mut reply = Vec::new();
      stream.read_to_end(&mut reply).unwrap();
      serde_json::from_slice(&reply).unwrap()
    };
    let request = json!({"op":"command","node":path,"run_id":"run_1","revision":"a".repeat(64),"payload":"good"});
    assert_eq!(
      exchange(request.clone())["error"],
      "dispatcher_admission_required"
    );
    let mut admitted = request;
    admitted["admission_token"] = json!("b".repeat(64));
    assert_eq!(
      exchange(admitted.clone())["outputs"][0]["output"]["payload"]["accepted"],
      true
    );
    admitted["revision"] = json!("c".repeat(64));
    assert_eq!(exchange(admitted)["error"], "stale_run");
    let mut stream = std::os::unix::net::UnixStream::connect(&socket).unwrap();
    stream.write_all(b"stop\n").unwrap();
    stream.shutdown(std::net::Shutdown::Write).unwrap();
    let mut reply = Vec::new();
    stream.read_to_end(&mut reply).unwrap();
    assert_eq!(serde_json::from_slice::<Value>(&reply).unwrap()["ok"], true);
    worker.join().unwrap();
  }
  #[test]
  fn native_abi_lifecycle_and_input() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("node.c");
    let library = directory.path().join("node.so");
    fs::write(&source, r#"
#include "isos_node_v1.h"
#include <stdlib.h>
struct isos_node_handle { const isos_node_host_v1 *host; };
static int32_t create(const uint8_t *config, size_t length, const isos_node_host_v1 *host, isos_node_handle **out) { (void)config; (void)length; *out=malloc(sizeof(**out)); (*out)->host=host; return 0; }
static int32_t start(isos_node_handle *node) { return node ? 0 : -1; }
static int32_t input(isos_node_handle *node, const uint8_t *bytes, size_t length) { (void)bytes; (void)length; const uint8_t out[]="{\"port\":\"out\",\"payload\":{\"value\":1}}"; return node->host->emit(node->host->context,out,sizeof(out)-1); }
static int32_t health(isos_node_handle *node) { return node ? 1 : -1; }
static int32_t stop(isos_node_handle *node) { return node ? 0 : -1; }
static void destroy(isos_node_handle *node) { free(node); }
static const isos_node_api_v1 api={1,sizeof(isos_node_api_v1),create,start,input,health,stop,destroy};
const isos_node_api_v1 *isos_node_v1(void) { return &api; }
"#).unwrap();
    let header = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contract/node");
    let output = std::process::Command::new("cc")
      .args(["-shared", "-fPIC", "-I"])
      .arg(&header)
      .arg(&source)
      .arg("-o")
      .arg(&library)
      .output()
      .unwrap();
    assert!(
      output.status.success(),
      "{}",
      String::from_utf8_lossy(&output.stderr)
    );
    assert!(probe(&library).is_ok());
    assert!(probe(&source).is_err());
    let mut node = NativeNode::open(&library, br#"{}"#).unwrap();
    assert_eq!(node.health(), 1);
    assert_eq!(
      node.input(&json!({"op":"input"})).unwrap(),
      vec![json!({"port":"out","payload":{"value":1}})]
    );
    assert!(node.stop());
    drop(node);
    let socket = directory.path().join("worker.sock");
    let worker_socket = socket.clone();
    let worker_library = library.clone();
    let worker = std::thread::spawn(move || {
      let config = json!({"run_id":"run_1","revision":"a".repeat(64),"nodes":[
        {"path":"eos/skill/custom/first","library":worker_library,"node":{"adapter":{"settings":{"frequency_hz":20.0}}}}
      ],"edges":[],"output_types":[{"node":"eos/skill/custom/first","port":"out","data_type":"vendor.custom.state@v0.2"}],
      "data_types":[{"id":"vendor.custom.state@v0.2","version":"0.2","purpose":"Test output","media_type":"application/json","codec":"json_v1","schema":{"type":"object","required":["value"],"properties":{"value":{"type":"integer"}}},"max_payload_bytes":1024}]});
      serve_graph(&worker_socket, "run_1", &"a".repeat(64), &config).unwrap();
    });
    for _ in 0..100 {
      if socket.exists() {
        break;
      }
      std::thread::sleep(Duration::from_millis(10));
    }
    let exchange = |request: &[u8]| -> Value {
      let mut stream = std::os::unix::net::UnixStream::connect(&socket).unwrap();
      stream.write_all(request).unwrap();
      stream.shutdown(std::net::Shutdown::Write).unwrap();
      let mut reply = Vec::new();
      stream.read_to_end(&mut reply).unwrap();
      serde_json::from_slice(&reply).unwrap()
    };
    assert_eq!(exchange(b"status\n")["state"], "ready");
    let latest = exchange(
      json!({"op":"latest_output","run_id":"run_1","revision":"a".repeat(64),
      "node":"eos/skill/custom/first","port":"out","wait_ms":200})
      .to_string()
      .as_bytes(),
    );
    assert_eq!(latest["sample"]["output"]["payload"]["value"], 1);
    let sequence = latest["sample"]["sequence"].as_u64().unwrap();
    let start = Instant::now();
    let unchanged = exchange(
      json!({"op":"latest_output","run_id":"run_1","revision":"a".repeat(64),
      "node":"eos/skill/custom/first","port":"out","after_sequence":u64::MAX,"wait_ms":200})
      .to_string()
      .as_bytes(),
    );
    assert!(unchanged["sample"].is_null());
    assert!(start.elapsed() < Duration::from_millis(150));
    std::thread::sleep(Duration::from_millis(60));
    let newer = exchange(
      json!({"op":"latest_output","run_id":"run_1","revision":"a".repeat(64),
      "node":"eos/skill/custom/first","port":"out","after_sequence":sequence})
      .to_string()
      .as_bytes(),
    );
    assert!(newer["sample"]["sequence"].as_u64().unwrap() > sequence);
    assert_eq!(
      exchange(
        json!({"op":"input","run_id":"run_1","revision":"b".repeat(64),"payload":{}})
          .to_string()
          .as_bytes()
      )["error"],
      "stale_run"
    );
    assert_eq!(
      exchange(
        json!({"op":"input","run_id":"run_1","revision":"a".repeat(64),"payload":{}})
          .to_string()
          .as_bytes()
      )["outputs"][0]["output"]["payload"]["value"],
      1
    );
    assert_eq!(
      exchange(
        json!({"op":"observe","run_id":"run_1","revision":"a".repeat(64),
        "node":"eos/skill/custom/first"})
        .to_string()
        .as_bytes()
      )["outputs"][0]["output"]["payload"]["value"],
      1
    );
    assert_eq!(exchange(b"stop\n")["ok"], true);
    worker.join().unwrap();
    let config = json!({"run_id":"run_1","revision":"a".repeat(64),"nodes":[
      {"path":"eos/skill/custom/first","library":library,"node":{}},
      {"path":"eos/skill/custom/second","library":library,"node":{}}
    ],"edges":[{"source":"eos/skill/custom/first","output":"out","target":"eos/skill/custom/second","input":"in"}],
    "output_types":[
      {"node":"eos/skill/custom/first","port":"out","data_type":"vendor.custom.state@v0.2"},
      {"node":"eos/skill/custom/second","port":"out","data_type":"vendor.custom.state@v0.2"}
    ],"data_types":[{"id":"vendor.custom.state@v0.2","version":"0.2","purpose":"Test output","media_type":"application/json","codec":"json_v1","schema":{"type":"object","required":["value"],"properties":{"value":{"type":"integer"}}},"max_payload_bytes":1024}]});
    let mut graph = NativeGraph::open(&config).unwrap();
    assert_eq!(graph.health(), 1);
    let outputs = graph
      .input(
        &json!({"op":"input","node":"eos/skill/custom/first","payload":{}}),
        "run_1",
        &"a".repeat(64),
      )
      .unwrap();
    assert_eq!(outputs.len(), 2);
    assert_eq!(outputs[0]["node"], "eos/skill/custom/first");
    assert_eq!(outputs[1]["node"], "eos/skill/custom/second");
    assert!(graph.stop());
  }
  #[test]
  fn invalid_native_output_never_reaches_downstream_plugin() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("reject.c");
    let library = directory.path().join("reject.so");
    let marker = directory.path().join("downstream_called");
    fs::write(&source, format!(r#"
#include "isos_node_v1.h"
#include <stdlib.h>
#include <string.h>
#include <stdio.h>
struct isos_node_handle {{ const isos_node_host_v1 *host; int downstream; }};
static int32_t create(const uint8_t *config,size_t length,const isos_node_host_v1 *host,isos_node_handle **out) {{
  *out=malloc(sizeof(**out)); (*out)->host=host;
  char *text=malloc(length+1); memcpy(text,config,length); text[length]=0;
  (*out)->downstream=strstr(text,"custom/second")!=NULL; free(text);
  return 0;
}}
static int32_t start(isos_node_handle *node) {{return node?0:-1;}}
static int32_t input(isos_node_handle *node,const uint8_t *bytes,size_t length) {{
  if(node->downstream) {{FILE *f=fopen("{}","w"); if(f) fclose(f); return 0;}}
  char *text=malloc(length+1); memcpy(text,bytes,length); text[length]=0;
  int bad_port=strstr(text,"bad_port")!=NULL; free(text);
  const char *out=bad_port
    ? "{{\"port\":\"unknown\",\"payload\":{{\"value\":1}}}}"
    : "{{\"port\":\"out\",\"payload\":{{\"value\":\"wrong\"}}}}";
  return node->host->emit(node->host->context,(const uint8_t *)out,strlen(out));
}}
static int32_t health(isos_node_handle *node) {{return node?1:-1;}}
static int32_t stop(isos_node_handle *node) {{return node?0:-1;}}
static void destroy(isos_node_handle *node) {{free(node);}}
static const isos_node_api_v1 api={{1,sizeof(isos_node_api_v1),create,start,input,health,stop,destroy}};
const isos_node_api_v1 *isos_node_v1(void) {{return &api;}}
"#, marker.display())).unwrap();
    let header = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contract/node");
    let build = std::process::Command::new("cc")
      .args(["-shared", "-fPIC", "-I"])
      .arg(header)
      .arg(&source)
      .arg("-o")
      .arg(&library)
      .output()
      .unwrap();
    assert!(
      build.status.success(),
      "{}",
      String::from_utf8_lossy(&build.stderr)
    );
    let config = json!({"run_id":"run_1","revision":"a".repeat(64),"nodes":[
      {"path":"eos/skill/custom/first","library":library,"node":{}},
      {"path":"eos/skill/custom/second","library":library,"node":{}}
    ],"edges":[{"source":"eos/skill/custom/first","output":"out","target":"eos/skill/custom/second","input":"in"}],
    "output_types":[{"node":"eos/skill/custom/first","port":"out","data_type":"vendor.custom.state@v0.2"}],
    "data_types":[{"id":"vendor.custom.state@v0.2","version":"0.2","purpose":"Test output","media_type":"application/json","codec":"json_v1","schema":{"type":"object","required":["value"],"properties":{"value":{"type":"integer"}}},"max_payload_bytes":1024}]});
    let mut graph = NativeGraph::open(&config).unwrap();
    for payload in ["bad_port", "bad_payload"] {
      assert!(graph
        .input(
          &json!({"op":"input","node":"eos/skill/custom/first","payload":payload}),
          "run_1",
          &"a".repeat(64)
        )
        .is_err());
      assert!(!marker.exists());
    }
    assert!(graph.stop());
  }
  #[test]
  fn vision_push_is_bounded_and_source_failure_closes_subscriber() {
    let (server, mut client) = UnixStream::pair().unwrap();
    server.set_nonblocking(true).unwrap();
    client
      .set_read_timeout(Some(Duration::from_secs(1)))
      .unwrap();
    let source = "eos/hardware/camera/front";
    let mut graph = NativeGraph {
      nodes: Vec::new(),
      indices: BTreeMap::new(),
      edges: Vec::new(),
      output_types: BTreeMap::new(),
      registry: NodeTypeRegistry::builtin(),
      latest: BTreeMap::new(),
      sequence: 1,
      sources: Vec::new(),
      capture_failed: false,
    };
    graph.latest.insert(
      (source.into(), "frame".into()),
      (
        1,
        json!({"port":"frame","payload":{"sample":{"source_time_ns":1,"clock":"simulation",
        "sequence":1,"frame_id":"front-1"},"width_px":1280,"height_px":720,
        "encoding":"jpeg","data":"/9j/2Q=="}}),
      ),
    );
    let mut subscribers = vec![VisionSubscriber {
      stream: server,
      source: source.into(),
      sequence: 0,
      pending: Vec::new(),
      written: 0,
      pending_since: Instant::now(),
    }];
    publish_vision(&mut subscribers, &graph, "run_1", &"a".repeat(64));
    let mut length = [0u8; 4];
    client.read_exact(&mut length).unwrap();
    let header_len = u32::from_be_bytes(length) as usize;
    assert!(header_len <= 4096);
    let mut header = vec![0; header_len];
    client.read_exact(&mut header).unwrap();
    let header: Value = serde_json::from_slice(&header).unwrap();
    assert_eq!(header["output_sequence"], 1);
    client.read_exact(&mut length).unwrap();
    assert_eq!(u32::from_be_bytes(length), 4);
    let mut jpeg = [0u8; 4];
    client.read_exact(&mut jpeg).unwrap();
    assert_eq!(jpeg, [0xff, 0xd8, 0xff, 0xd9]);
    graph.capture_failed = true;
    publish_vision(&mut subscribers, &graph, "run_1", &"a".repeat(64));
    assert!(subscribers.is_empty());
    assert_eq!(client.read(&mut jpeg).unwrap(), 0);
  }
}
