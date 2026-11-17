use serde_json::{json, Value};

pub(super) fn output(envelope: &Value) -> Value {
  json!({"port":"observation","payload":envelope["payload"]})
}
