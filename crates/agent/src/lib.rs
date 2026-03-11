//! `NanoScale` agent backend crate.
//!
//! This crate provides two runtime roles:
//! - orchestrator: control plane API and persistence.
//! - worker: execution plane for deployment and host operations.
//!
//! Shared modules (`db`, `cluster`, `deployment`, `system`) contain logic used by one
//! or both roles.

pub mod cluster;
pub mod config;
pub mod db;
pub mod deployment;
pub mod orchestrator;
mod request_logging;
pub mod system;
pub mod worker;
