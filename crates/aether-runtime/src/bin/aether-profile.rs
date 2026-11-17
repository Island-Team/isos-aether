//! Compatibility entry point for the EOS owner.
fn main() -> Result<(), Box<dyn std::error::Error>> {
  aether_system::profile::main()
}
