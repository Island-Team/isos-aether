use serde_json::{json, Value};

pub(super) fn output() -> Value {
  json!({"port":"delta","payload":{"joint_positions_rad":[0.05,-0.04,0.03,-0.02,0.01,-0.01]}})
}
