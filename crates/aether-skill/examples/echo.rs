use aether_skill::{Completion, Descriptor, Invocation, Skill};
use serde_json::json;

struct Echo;

impl Skill for Echo {
  fn descriptor(&self) -> Descriptor {
    Descriptor {
      name: "echo".into(),
      version: "1.0.0".into(),
      description: "Return the supplied message".into(),
      parameters_schema: json!({
        "type": "object",
        "properties": {"message": {"type": "string"}},
        "required": ["message"],
        "additionalProperties": false
      }),
      returns_schema: json!({
        "type": "object",
        "properties": {"message": {"type": "string"}},
        "required": ["message"],
        "additionalProperties": false
      }),
      primitives: vec![],
      reentrant: true,
    }
  }

  fn execute(&self, invocation: Invocation<'_>) -> Result<Completion, aether_skill::SkillError> {
    Ok(Completion {
      accepted: true,
      reason_code: "ok".into(),
      result: json!({"message": invocation.arguments["message"]}),
      message: String::new(),
    })
  }
}

fn main() {
  if let Err(error) = aether_skill::serve(&[Box::new(Echo)]) {
    eprintln!("{error}");
    std::process::exit(1);
  }
}
