//! Compatibility entry point for the EOS worker.
fn main() -> Result<(), Box<dyn std::error::Error>> {
  aether_system::worker::main()
}
