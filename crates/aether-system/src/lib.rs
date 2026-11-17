//! EOS owner and worker runtime. Executable names remain in `aether-runtime`.

pub mod io {
  pub mod control;
}

#[path = "bin/common/eos_log.rs"]
mod eos_log;

#[path = "bin/aether-profile.rs"]
pub mod profile;
#[path = "bin/aether-runtime.rs"]
pub mod worker;
