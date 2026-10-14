//! Parity tests: every behavior on the M1 parity list, driven through the studio window's
//! actions and observed through its telemetry, the engine and files on disk. They name no
//! view type, so the views can be replaced without rewriting them; only `harness.rs` knows
//! the root view.

#[path = "../support/build_fixture.rs"]
mod build_fixture;
mod harness;

mod services;
