//! UI-independent libmpv playback execution.
//!
//! The module owns libmpv handles and exposes a small control surface. It does
//! not create windows or render contexts; the GPUI host can add rendering later.

mod loader;
mod session;

pub(crate) use loader::{PlayerPaths, bundled_platform_dir, library_candidates};
pub(crate) use session::{PlayerSession, PlayerSessionConfig, PlayerState};

#[cfg(test)]
mod tests;

pub(crate) use session::{mpv_options, mpv_start_value, redact_secrets};
