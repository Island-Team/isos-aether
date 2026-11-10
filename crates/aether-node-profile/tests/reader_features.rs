use aether_node_profile::builtin_reader_node_types;

#[test]
fn built_in_reader_ids_are_current_and_sorted() {
  let ids = builtin_reader_node_types();
  assert!(!ids.is_empty());
  assert!(ids
    .iter()
    .all(|id| id.starts_with("eos.") && id.ends_with("@v0.2")));
  assert!(ids.windows(2).all(|pair| pair[0] < pair[1]));
  assert!(ids.iter().any(|id| id == "eos.hardware.mock.arm@v0.2"));
  assert!(ids.iter().any(|id| id == "eos.dispatcher.arm.delta@v0.2"));
}
