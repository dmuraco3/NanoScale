//! Worker-side deployment primitives.
//!
//! These modules implement source fetch, artifact build/install, service and proxy
//! configuration, TLS provisioning, and teardown.

pub mod build;
pub mod git;
pub mod nginx;
pub mod systemd;
pub mod teardown;
pub mod tls;
