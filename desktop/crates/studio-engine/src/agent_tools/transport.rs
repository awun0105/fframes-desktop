//! The local transport under the tool broker and its client, one implementation per
//! platform with the same surface:
//!
//! - Unix: a unix domain socket `broker.sock` (0600) in the private runtime directory;
//!   every accepted peer's uid must be ours.
//! - Windows: a named pipe with a per-broker random name whose DACL grants only the
//!   current user and which refuses remote clients; every client token must be the
//!   current user's, unrestricted and outside an AppContainer.
//!
//! Both also own what "private" means for the runtime directory and the capability files.
//! Streams carry newline-delimited JSON with read and write timeouts; [`close`] cuts a
//! stream off from another thread, [`finish`] ends it after the replies were written.

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::*;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::*;

/// What one accept tick produced.
pub enum Accepted {
    Peer(Stream),
    /// A peer connected but is not the current user; it was dropped. A pipe client is
    /// only identifiable after its first line, so Windows refuses in `verify_peer`.
    Refused,
    /// Nothing arrived within the tick.
    Idle,
}
