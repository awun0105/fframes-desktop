//! Parity tests: every behavior on the studio's parity list, driven through the studio window's
//! actions and observed through its telemetry, the engine and files on disk. They name no
//! view type, so the views can be replaced without rewriting them; only `harness.rs` knows
//! the root view.

#[path = "../../../studio-engine/tests/support/build_fixture.rs"]
mod build_fixture;
mod harness;

mod agent;
mod audio;
mod canvas;
mod checkpoints;
mod composer;
mod keyboard;
mod presets;
mod services;
mod settings;
mod setup;
mod timeline;
mod transport;
mod window;
