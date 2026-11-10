
[中文](../../../../docs_cn/ei/design/runtime/fleet.md)


## Configuration

```toml
[[modes.fleet.backends]]
id = "robot-a"
name = "west robot"
intro = "mobile manipulator in the west cell"
gateway = { host = "127.0.0.1", grpc_port = 16003 }

[[modes.fleet.backends]]
id = "robot-b"
name = "east robot"
intro = "inspection robot in the east cell"
gateway = { host = "127.0.0.1", grpc_port = 18110 }
```


## Discovery and addressing


- the current EOS runtime identity and version/start/health;
- hardware profile, model, vendor, and description;
- every generated device ID, class, part, backend metadata, and health;
- runtime-scoped world resources and versioned primitive descriptors with per-device numeric limits.

Resources and primitives are keyed by alias plus device ID. Two robots, or two devices of the same class on one robot, therefore remain distinct. An action target is always explicit:

```json
{
  "robot_id": "robot-a",
  "device_id": "base-1",
  "primitive": "base.velocity",
  "arguments": {
    "linear_x_m_s": 0.2,
    "linear_y_m_s": 0.0,
    "angular_z_rad_s": 0.0,
    "valid_for_ms": 250
  }
}
```


## Coordination and safety

Coordination uses the current discovery generation and device health to select candidates. The safety gate revalidates the exact capability, limits, and current state before every action; one backend disconnect cannot turn another backend's result into success. Emergency stop erects a fleet barrier and attempts every discovered device and endpoint, recording each failure independently.

## Current limits

- The first endpoint remains the source of the aggregate scene projection.
- Zone/station reachability cannot be derived from EOS discovery today.
